use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use sherpa_onnx::{KeywordSpotter, KeywordSpotterConfig, Wave};

pub const MODEL_DIR: &str = "models/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01";
pub const KEYWORDS_FILE: &str = "assets/keywords.txt";
pub const MIN_SAMPLE_RATE: u32 = 8_000;
pub const MAX_SAMPLE_RATE: u32 = 192_000;

#[derive(Clone, Debug)]
pub struct DetectorConfig {
    pub model_dir: PathBuf,
    pub keywords_file: PathBuf,
    pub threads: i32,
    pub threshold: f32,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            model_dir: MODEL_DIR.into(),
            keywords_file: KEYWORDS_FILE.into(),
            threads: 1,
            threshold: 0.25,
        }
    }
}

impl DetectorConfig {
    pub fn validate(&self) -> Result<(), DetectorError> {
        if self.model_dir.as_os_str().is_empty() {
            return Err(DetectorError::InvalidConfig(
                "model directory must not be empty".into(),
            ));
        }
        if self.keywords_file.as_os_str().is_empty() {
            return Err(DetectorError::InvalidConfig(
                "keywords file must not be empty".into(),
            ));
        }
        if self.threads <= 0 {
            return Err(DetectorError::InvalidConfig(
                "thread count must be greater than zero".into(),
            ));
        }
        if !self.threshold.is_finite() || !(0.0..=1.0).contains(&self.threshold) {
            return Err(DetectorError::InvalidConfig(
                "keyword threshold must be between 0 and 1".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum DetectorError {
    InvalidConfig(String),
    InvalidSampleRate(u32),
    InvalidPcm(String),
    ModelLoad,
    WaveRead(String),
    InvalidResultJson,
}

impl fmt::Display for DetectorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(message) => write!(formatter, "invalid detector config: {message}"),
            Self::InvalidSampleRate(rate) => write!(
                formatter,
                "sample rate must be between {MIN_SAMPLE_RATE} and {MAX_SAMPLE_RATE} Hz: {rate}"
            ),
            Self::InvalidPcm(message) => write!(formatter, "invalid PCM input: {message}"),
            Self::ModelLoad => write!(
                formatter,
                "Could not create keyword spotter. Check that the model files and keyword file exist."
            ),
            Self::WaveRead(path) => write!(formatter, "Could not read WAV file: {path}"),
            Self::InvalidResultJson => write!(formatter, "keyword spotter returned invalid JSON"),
        }
    }
}

impl Error for DetectorError {}

#[derive(Clone, Debug, Serialize)]
pub struct WakeDetection {
    pub keyword: String,
    #[serde(rename = "timestamp")]
    pub timestamp_ms: u64,
    pub start_time: Option<serde_json::Value>,
    pub timestamps: Option<serde_json::Value>,
    pub json: serde_json::Value,
    #[serde(skip)]
    raw_json: String,
}

impl WakeDetection {
    pub fn raw_json(&self) -> &str {
        &self.raw_json
    }
}

#[derive(Debug)]
pub enum DetectorEvent {
    Wake(WakeDetection),
    Error {
        code: &'static str,
        message: String,
        retryable: bool,
    },
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct DetectorSessionStats {
    pub sample_rate: u32,
    pub frame_count: u64,
    pub sample_count: u64,
    pub duration_ms: u64,
    pub rms: f64,
    pub peak: f32,
}

#[derive(Debug)]
pub struct SessionResult {
    pub events: Vec<DetectorEvent>,
    pub stats: DetectorSessionStats,
}

#[derive(Debug)]
pub struct FixtureResult {
    pub detected: bool,
    pub wakes: Vec<WakeDetection>,
    pub stats: DetectorSessionStats,
}

pub struct Detector {
    spotter: KeywordSpotter,
    threshold: f32,
}

impl Detector {
    pub fn load(config: DetectorConfig) -> Result<Self, DetectorError> {
        config.validate()?;
        let mut sherpa_config = KeywordSpotterConfig::default();
        sherpa_config.model_config.transducer.encoder = Some(
            config
                .model_dir
                .join("encoder-epoch-12-avg-2-chunk-16-left-64.int8.onnx")
                .to_string_lossy()
                .into_owned(),
        );
        sherpa_config.model_config.transducer.decoder = Some(
            config
                .model_dir
                .join("decoder-epoch-12-avg-2-chunk-16-left-64.int8.onnx")
                .to_string_lossy()
                .into_owned(),
        );
        sherpa_config.model_config.transducer.joiner = Some(
            config
                .model_dir
                .join("joiner-epoch-12-avg-2-chunk-16-left-64.int8.onnx")
                .to_string_lossy()
                .into_owned(),
        );
        sherpa_config.model_config.tokens = Some(
            config
                .model_dir
                .join("tokens.txt")
                .to_string_lossy()
                .into_owned(),
        );
        sherpa_config.model_config.provider = Some("cpu".to_string());
        sherpa_config.model_config.num_threads = config.threads;
        sherpa_config.keywords_file = Some(config.keywords_file.to_string_lossy().into_owned());
        sherpa_config.keywords_threshold = config.threshold;
        sherpa_config.keywords_score = 1.0;

        let spotter = KeywordSpotter::create(&sherpa_config).ok_or(DetectorError::ModelLoad)?;
        Ok(Self {
            spotter,
            threshold: config.threshold,
        })
    }

    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    pub fn open_session<S: Into<String>>(
        &self,
        session_id: S,
        sample_rate: u32,
    ) -> Result<DetectorSession<'_>, DetectorError> {
        validate_sample_rate(sample_rate)?;
        let session_id = session_id.into();
        if session_id.is_empty() {
            return Err(DetectorError::InvalidConfig(
                "session id must not be empty".into(),
            ));
        }
        Ok(DetectorSession {
            detector: self,
            session_id,
            sample_rate,
            stream: self.spotter.create_stream(),
            stats: DetectorSessionStats {
                sample_rate,
                ..DetectorSessionStats::default()
            },
            sum_squares: 0.0,
        })
    }

    pub fn run_wav(&self, path: &Path) -> Result<FixtureResult, DetectorError> {
        let wave = Wave::read(path.to_string_lossy().as_ref())
            .ok_or_else(|| DetectorError::WaveRead(path.to_string_lossy().into_owned()))?;
        let mut session = self.open_session("fixture", wave.sample_rate() as u32)?;
        session.accept_pcm_buffered(wave.samples())?;
        let tail = vec![0.0_f32; wave.sample_rate() as usize / 2];
        session.accept_pcm_buffered(&tail)?;
        let result = session.finish()?;
        let wakes = result
            .events
            .into_iter()
            .filter_map(|event| match event {
                DetectorEvent::Wake(detection) => Some(detection),
                DetectorEvent::Error { .. } => None,
            })
            .collect::<Vec<_>>();
        Ok(FixtureResult {
            detected: !wakes.is_empty(),
            wakes,
            stats: result.stats,
        })
    }

    fn drain_ready(
        &self,
        stream: &sherpa_onnx::OnlineStream,
    ) -> Result<Vec<DetectorEvent>, DetectorError> {
        let mut events = Vec::new();
        while self.spotter.is_ready(stream) {
            self.spotter.decode(stream);
            if let Some(result) = self.spotter.get_result(stream) {
                if !result.keyword.is_empty() {
                    let json: serde_json::Value = serde_json::from_str(&result.json)
                        .map_err(|_| DetectorError::InvalidResultJson)?;
                    events.push(DetectorEvent::Wake(WakeDetection {
                        keyword: result.keyword,
                        timestamp_ms: unix_timestamp_ms(),
                        start_time: json.get("start_time").cloned(),
                        timestamps: json.get("timestamps").cloned(),
                        json,
                        raw_json: result.json,
                    }));
                    self.spotter.reset(stream);
                }
            }
        }
        Ok(events)
    }
}

pub struct DetectorSession<'a> {
    detector: &'a Detector,
    session_id: String,
    sample_rate: u32,
    stream: sherpa_onnx::OnlineStream,
    stats: DetectorSessionStats,
    sum_squares: f64,
}

impl DetectorSession<'_> {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn stats(&self) -> &DetectorSessionStats {
        &self.stats
    }

    pub fn accept_pcm(&mut self, samples: &[f32]) -> Result<Vec<DetectorEvent>, DetectorError> {
        self.accept_pcm_buffered(samples)?;
        self.detector.drain_ready(&self.stream)
    }

    fn accept_pcm_buffered(&mut self, samples: &[f32]) -> Result<(), DetectorError> {
        if samples.iter().any(|sample| !sample.is_finite()) {
            return Err(DetectorError::InvalidPcm(
                "samples must all be finite".into(),
            ));
        }
        if samples.is_empty() {
            return Ok(());
        }

        self.stats.frame_count += 1;
        self.stats.sample_count += samples.len() as u64;
        self.stats.duration_ms =
            self.stats.sample_count.saturating_mul(1_000) / self.sample_rate as u64;
        for sample in samples {
            self.sum_squares += f64::from(*sample) * f64::from(*sample);
            self.stats.peak = self.stats.peak.max(sample.abs());
        }
        self.stats.rms = (self.sum_squares / self.stats.sample_count as f64).sqrt();
        self.stream
            .accept_waveform(self.sample_rate as i32, samples);
        Ok(())
    }

    pub fn finish(self) -> Result<SessionResult, DetectorError> {
        self.stream.input_finished();
        let events = self.detector.drain_ready(&self.stream)?;
        Ok(SessionResult {
            events,
            stats: self.stats,
        })
    }
}

fn validate_sample_rate(sample_rate: u32) -> Result<(), DetectorError> {
    if (MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&sample_rate) {
        Ok(())
    } else {
        Err(DetectorError::InvalidSampleRate(sample_rate))
    }
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_config() {
        let mut config = DetectorConfig::default();
        config.threads = 0;
        assert!(matches!(
            config.validate(),
            Err(DetectorError::InvalidConfig(_))
        ));

        config.threads = 1;
        config.threshold = 1.1;
        assert!(matches!(
            config.validate(),
            Err(DetectorError::InvalidConfig(_))
        ));
    }

    #[test]
    fn validates_sample_rate_without_loading_a_model() {
        assert!(validate_sample_rate(48_000).is_ok());
        assert!(matches!(
            validate_sample_rate(0),
            Err(DetectorError::InvalidSampleRate(0))
        ));
    }
}
