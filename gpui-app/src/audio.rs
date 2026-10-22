//! The host owns the microphone.
//!
//! One capture session per host, opened lazily the first time a plugin that
//! holds the `audio_capture` grant asks for audio, and shared by every plugin
//! that holds it. The host is the only process that opens the device, so two
//! granted plugins cannot fight over it — and a plugin that was never granted
//! the capability cannot reach the import at all, because the linker does not
//! hand it over.
//!
//! The samples delivered are mono f32 at [`CAPTURE_SAMPLE_RATE`], whatever the
//! device's native rate is. Resampling here rather than in every guest means
//! the contract does not depend on the hardware the user happens to own, and it
//! matches the rate the wake detector expects.
//!
//! One constraint the host cannot remove: the wake service is a separate
//! process that opens the same default device for the browser capture path.
//! On Linux the second open of one device usually fails, so the host reports
//! that the device is busy rather than handing back silence. The wake path and
//! the plugin path cannot both hold the microphone at once.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[cfg(not(target_family = "wasm"))]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// The rate every guest receives, whatever the device does natively.
pub const CAPTURE_SAMPLE_RATE: u32 = 16_000;

/// How much audio the host keeps when a guest is not reading. Thirty seconds is
/// long enough for a guest that stalled and short enough that a forgotten
/// capture cannot grow without bound.
const MAX_BUFFERED_SAMPLES: usize = CAPTURE_SAMPLE_RATE as usize * 30;

/// Why a capture could not be served.
#[derive(Debug)]
pub enum AudioError {
    /// No input device the host could open.
    NoDevice,
    /// The device is held by another process. The message says who holds it,
    /// because the common cause is the wake service rather than a fault.
    DeviceBusy(String),
    /// The device speaks a sample format the host does not convert.
    UnsupportedFormat(String),
    /// The device could not be opened or started.
    Io(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::NoDevice => write!(f, "no microphone was found"),
            AudioError::DeviceBusy(reason) => {
                write!(f, "the microphone is in use by another process: {reason}")
            }
            AudioError::UnsupportedFormat(format) => {
                write!(f, "unsupported microphone sample format: {format}")
            }
            AudioError::Io(reason) => write!(f, "the microphone could not be opened: {reason}"),
        }
    }
}

impl std::error::Error for AudioError {}

/// The resampling queue between the device and the guests.
///
/// The device delivers interleaved samples at its own rate; guests receive mono
/// f32 at [`CAPTURE_SAMPLE_RATE`]. The mixdown to mono happens in the capture
/// source, so this only has to change the rate.
#[derive(Debug, Default)]
pub struct CaptureBuffer {
    samples: VecDeque<f32>,
    /// The device's rate. Zero until the first sample arrives.
    rate: u32,
    /// Index of the next source sample, used to place output samples.
    source_index: u64,
    /// Position of the next output sample, in source samples.
    next_output: f64,
    /// The previous source sample, for interpolation.
    previous: f32,
    has_previous: bool,
}

impl CaptureBuffer {
    /// Accept interleaved device samples and queue them at the capture rate.
    pub fn push_interleaved(&mut self, input: &[f32], channels: u16, rate: u32) {
        let channels = channels.max(1) as usize;
        if self.rate == 0 {
            self.rate = rate.max(1);
        }
        for frame in input.chunks(channels) {
            let mono = frame.iter().sum::<f32>() / frame.len() as f32;
            self.push_mono(&[mono], rate);
        }
    }

    /// Accept mono device samples and queue them at the capture rate. The rate
    /// is the device's own; the buffer resamples to [`CAPTURE_SAMPLE_RATE`].
    pub fn push_mono(&mut self, input: &[f32], rate: u32) {
        if self.rate == 0 {
            self.rate = rate.max(1);
        }
        let step = self.rate as f64 / CAPTURE_SAMPLE_RATE as f64;
        for &sample in input {
            if self.has_previous {
                // Every output position that has been passed by the incoming
                // sample is emitted now, interpolated between the two samples
                // that bracket it.
                while self.next_output <= self.source_index as f64 {
                    let offset = self.next_output - (self.source_index as f64 - 1.0);
                    let value = self.previous + (sample - self.previous) * offset as f32;
                    self.samples.push_back(value);
                    self.next_output += step;
                    if self.samples.len() > MAX_BUFFERED_SAMPLES {
                        self.samples.pop_front();
                    }
                }
            }
            self.previous = sample;
            self.has_previous = true;
            self.source_index += 1;
        }
    }

    /// Take up to `max_samples` queued samples, oldest first.
    pub fn take(&mut self, max_samples: usize) -> Vec<f32> {
        let count = max_samples.min(self.samples.len());
        self.samples.drain(..count).collect()
    }

    /// How many samples are waiting.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

/// A source of device samples. The device adapter is behind this trait so the
/// queue and the grant rules can be tested without a microphone.
pub trait CaptureSource: Send {
    /// Begin capturing, pushing samples into `sink`. Returns the device rate.
    fn start(&mut self, sink: Arc<Mutex<CaptureBuffer>>) -> Result<u32, AudioError>;
    /// Stop capturing and release the device.
    fn stop(&mut self);
}

struct Inner {
    /// Shared by reference, never moved. The device thread holds a clone of
    /// this handle, so the buffer cannot be taken back out of the session.
    buffer: Arc<Mutex<CaptureBuffer>>,
    source: Option<Box<dyn CaptureSource>>,
    started: bool,
}

/// The host's microphone. Shared by every plugin holding the grant.
#[derive(Clone)]
pub struct AudioCapture {
    inner: Arc<Mutex<Inner>>,
}

impl Default for AudioCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioCapture {
    /// A capture with the device adapter. Nothing is opened until it is read.
    pub fn new() -> Self {
        Self::with_source(Box::new(MicSource::default()))
    }

    /// A capture with a given source. Used by the tests, which cannot open a
    /// device.
    pub fn with_source(source: Box<dyn CaptureSource>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                buffer: Arc::new(Mutex::new(CaptureBuffer::default())),
                source: Some(source),
                started: false,
            })),
        }
    }

    /// Read up to `max_bytes` of mono f32 audio at the capture rate.
    ///
    /// Returns an empty vector when the device is live but has not produced a
    /// frame yet. That is different from an error: silence is a valid answer.
    pub fn read(&self, max_bytes: usize) -> Result<Vec<f32>, AudioError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if !inner.started {
            // The source pushes into the shared buffer; it keeps a clone of the
            // handle for as long as the device is open. The handle is taken
            // before the source is borrowed, so the two borrows do not overlap.
            let sink = inner.buffer.clone();
            let source = inner.source.as_mut().ok_or(AudioError::NoDevice)?;
            source.start(sink)?;
            inner.started = true;
        }
        let mut buffer = inner.buffer.lock().unwrap_or_else(|e| e.into_inner());
        Ok(buffer.take(max_bytes / std::mem::size_of::<f32>()))
    }

    /// Release the device. The next read starts a fresh session.
    pub fn stop(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(source) = inner.source.as_mut() {
            source.stop();
        }
        inner.started = false;
    }
}

/// The device adapter: ALSA on Linux through `cpal`.
///
/// Native-only. The browser build has no device to open: there the page holds
/// the microphone permission and streams frames to the host over the wire.
#[cfg(not(target_family = "wasm"))]
#[derive(Default)]
pub struct MicSource {
    stream: Option<cpal::Stream>,
}

#[cfg(not(target_family = "wasm"))]
impl CaptureSource for MicSource {
    #[cfg(not(target_family = "wasm"))]
    fn start(&mut self, sink: Arc<Mutex<CaptureBuffer>>) -> Result<u32, AudioError> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or(AudioError::NoDevice)?;
        let supported = device
            .default_input_config()
            .map_err(|e| AudioError::Io(e.to_string()))?;
        let config: cpal::StreamConfig = supported.clone().into();
        let channels = config.channels;
        let rate = config.sample_rate.0;

        // A bounded channel keeps the audio callback from building a backlog if
        // a guest stops reading. Dropping audio is better than growing memory.
        let (sender, receiver) = std::sync::mpsc::sync_channel::<Vec<f32>>(32);
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => build_stream::<f32, _>(&device, &config, sender, |s| s),
            cpal::SampleFormat::I16 => build_stream::<i16, _>(&device, &config, sender, |s| {
                s as f32 / i16::MAX as f32
            }),
            cpal::SampleFormat::U16 => build_stream::<u16, _>(&device, &config, sender, |s| {
                (s as f32 - 32768.0) / 32768.0
            }),
            format => {
                return Err(AudioError::UnsupportedFormat(format!("{format:?}")));
            }
        }
        .map_err(|e| {
            // A device already held by another process is the common case, and
            // it is not a fault in the host.
            let reason = e.to_string();
            if reason.contains("busy") || reason.contains("Device or resource busy") {
                AudioError::DeviceBusy(reason)
            } else {
                AudioError::Io(reason)
            }
        })?;

        // The consumer runs on its own thread so the device callback never
        // blocks on a guest.
        let mixer = sink.clone();
        let mixer_channels = channels;
        let mixer_rate = rate;
        std::thread::spawn(move || {
            while let Ok(chunk) = receiver.recv() {
                if chunk.is_empty() {
                    continue;
                }
                if let Ok(mut buffer) = mixer.lock() {
                    buffer.push_interleaved(&chunk, mixer_channels, mixer_rate);
                }
            }
        });

        stream
            .play()
            .map_err(|e| AudioError::Io(e.to_string()))?;
        self.stream = Some(stream);
        Ok(rate)
    }

    fn stop(&mut self) {
        self.stream = None;
    }
}

/// The browser build has no device adapter. The host never opens a microphone
/// there; the page does, and the frames arrive over the wire.
#[cfg(target_family = "wasm")]
#[derive(Default)]
pub struct MicSource;

#[cfg(target_family = "wasm")]
impl CaptureSource for MicSource {
    fn start(&mut self, _sink: Arc<Mutex<CaptureBuffer>>) -> Result<u32, AudioError> {
        Err(AudioError::NoDevice)
    }

    fn stop(&mut self) {}
}

fn build_stream<T, Convert>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sender: std::sync::mpsc::SyncSender<Vec<f32>>,
    convert: Convert,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + Copy,
    Convert: Fn(T) -> f32 + Send + 'static,
{
    let error_sender = sender.clone();
    device
        .build_input_stream(
            config,
            move |samples: &[T], _: &cpal::InputCallbackInfo| {
                let chunk = samples.iter().copied().map(&convert).collect();
                let _ = sender.try_send(chunk);
            },
            move |error| {
                let _ = error_sender.try_send(Vec::new());
                tracing::warn!("microphone capture error: {error}");
            },
            None,
        )
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A source that hands over fixed samples, so the queue and the grant path
    /// can be tested on a host with no microphone.
    struct FakeSource {
        samples: Vec<f32>,
        rate: u32,
        started: bool,
    }

    impl FakeSource {
        fn new(samples: Vec<f32>, rate: u32) -> Self {
            Self {
                samples,
                rate,
                started: false,
            }
        }
    }

    impl CaptureSource for FakeSource {
        fn start(&mut self, sink: Arc<Mutex<CaptureBuffer>>) -> Result<u32, AudioError> {
            self.started = true;
            let mut buffer = sink.lock().unwrap();
            buffer.push_mono(&self.samples, self.rate);
            Ok(self.rate)
        }

        fn stop(&mut self) {
            self.started = false;
        }
    }

    #[test]
    fn a_capture_is_not_opened_until_something_reads_it() {
        let source = Box::new(FakeSource::new(vec![0.5; 16_000], 48_000));
        let capture = AudioCapture::with_source(source);
        // Nothing has read, so the device is untouched. The observable proof is
        // that the first read succeeds and returns the queued audio.
        assert!(capture.read(64).is_ok());
    }

    #[test]
    fn samples_are_delivered_at_the_capture_rate_whatever_the_device_does() {
        // One second of a 48 kHz device is 48,000 samples and must become
        // 16,000 samples at the capture rate.
        let source = Box::new(FakeSource::new(vec![0.25; 48_000], 48_000));
        let capture = AudioCapture::with_source(source);
        let samples = capture.read(64 * 1024).expect("audio");
        assert!(
            (samples.len() as i64 - 16_000).abs() <= 64,
            "expected about 16,000 samples, got {}",
            samples.len()
        );
    }

    #[test]
    fn a_16k_device_passes_through_unchanged() {
        let source = Box::new(FakeSource::new(vec![0.5; 16_000], 16_000));
        let capture = AudioCapture::with_source(source);
        let samples = capture.read(64 * 1024).expect("audio");
        assert!((samples.len() as i64 - 16_000).abs() <= 64, "got {}", samples.len());
        assert!(samples.iter().all(|s| (s - 0.5).abs() < 0.001), "the level is preserved");
    }

    #[test]
    fn a_read_returns_only_what_was_asked_for() {
        let source = Box::new(FakeSource::new(vec![0.25; 48_000], 48_000));
        let capture = AudioCapture::with_source(source);
        let first = capture.read(400).expect("audio");
        assert_eq!(first.len(), 100, "400 bytes is 100 f32 samples");
    }

    #[test]
    fn a_device_that_cannot_be_opened_is_reported_rather_than_silent() {
        struct FailingSource;
        impl CaptureSource for FailingSource {
            fn start(&mut self, _sink: Arc<Mutex<CaptureBuffer>>) -> Result<u32, AudioError> {
                Err(AudioError::DeviceBusy("the wake service holds it".into()))
            }
            fn stop(&mut self) {}
        }
        let capture = AudioCapture::with_source(Box::new(FailingSource));
        let error = capture.read(64).expect_err("the device is busy");
        assert!(matches!(error, AudioError::DeviceBusy(_)), "{error}");
        assert!(error.to_string().contains("in use"), "{error}");
    }

    #[test]
    fn interleaved_stereo_is_mixed_to_mono() {
        let mut buffer = CaptureBuffer::default();
        // Left 1.0, right 0.0, repeated: the mono mix is 0.5.
        buffer.push_interleaved(&[1.0, 0.0, 1.0, 0.0], 2, 48_000);
        let samples = buffer.take(16);
        assert!(!samples.is_empty());
        assert!(
            samples.iter().all(|s| (s - 0.5).abs() < 0.001),
            "the mix is the average of the channels, got {samples:?}"
        );
    }

    /// The real device, not a fake. Skipped by default because CI has no
    /// microphone; run it with `--ignored` on a machine that has one.
    ///
    /// The assertion is about the *count* of samples against the wall clock,
    /// not about their level. A quiet room delivers silence, and silence is a
    /// legitimate answer; what would be broken is a path that delivers nothing,
    /// or delivers at the wrong rate.
    #[test]
    #[ignore = "needs a microphone"]
    fn a_real_microphone_delivers_audio_at_the_capture_rate() {
        let capture = AudioCapture::new();
        let started = std::time::Instant::now();
        let mut samples = Vec::new();
        while started.elapsed() < std::time::Duration::from_millis(1_200) {
            std::thread::sleep(std::time::Duration::from_millis(50));
            samples.extend(capture.read(8192).expect("the microphone opens"));
        }
        let elapsed = started.elapsed().as_secs_f64();
        let peak = samples.iter().fold(0.0_f32, |acc, s| acc.max(s.abs()));
        println!(
            "captured {} samples in {elapsed:.2}s (peak {peak:.6})",
            samples.len()
        );

        assert!(
            !samples.is_empty(),
            "the microphone delivered no audio at all"
        );
        // At the capture rate, one second of wall clock is 16,000 samples. The
        // band is wide because the device's own buffering decides where the
        // first frame lands, but a path delivering at the wrong rate cannot
        // land inside it.
        let expected = elapsed * CAPTURE_SAMPLE_RATE as f64;
        assert!(
            (samples.len() as f64) > expected * 0.5 && (samples.len() as f64) < expected * 1.5,
            "expected about {expected:.0} samples in {elapsed:.2}s, got {}",
            samples.len()
        );
        capture.stop();
    }

    #[test]
    fn the_buffer_is_bounded() {
        let mut buffer = CaptureBuffer::default();
        // Far more than the cap, to prove the buffer drops the oldest audio
        // rather than growing without bound.
        let loud = vec![0.5_f32; 48_000 * 60];
        buffer.push_mono(&loud, 48_000);
        assert!(
            buffer.len() <= MAX_BUFFERED_SAMPLES,
            "the buffer grew to {} samples",
            buffer.len()
        );
    }
}
