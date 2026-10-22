use std::{
    io::{IsTerminal, Write},
    path::PathBuf,
    sync::mpsc::{self, SyncSender},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use clap::Parser;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream, StreamConfig,
};
use agent_wake::detector::{
    Detector, DetectorConfig, DetectorEvent, DetectorSessionStats,
};

/// Frames below this peak are treated as "no microphone signal", which is the
/// difference between a quiet room and a capture path that is broken.
const SILENCE_PEAK: f32 = 1.0e-4;
const LIVE_REFRESH: Duration = Duration::from_millis(200);
const WAKE_FLASH: Duration = Duration::from_secs(3);

#[derive(Debug, Parser)]
#[command(about = "Local microphone/WAV test for sherpa-onnx keyword spotting")]
struct Args {
    #[arg(long, default_value = agent_wake::detector::MODEL_DIR)]
    model_dir: PathBuf,

    #[arg(long, default_value = agent_wake::detector::KEYWORDS_FILE)]
    keywords_file: PathBuf,

    #[arg(long)]
    wav: Option<PathBuf>,

    #[arg(long, default_value_t = 1)]
    threads: i32,

    #[arg(long, default_value_t = 0.25)]
    threshold: f32,

    /// Seconds between status lines on stderr; 0 turns them off.
    #[arg(long, default_value_t = 5.0)]
    log_interval: f64,

    /// Suppress the live terminal status line.
    #[arg(long)]
    no_live: bool,
}

/// What the terminal needs to remember about the most recent detection.
struct LastWake {
    keyword: String,
    at: Duration,
}

/// Terminal-facing view of a running detection session.
struct Status {
    started: Instant,
    threshold: f32,
    sample_rate: u32,
    channels: u16,
    keywords: String,
    last_wake: Option<LastWake>,
    live: bool,
    last_render: Duration,
    last_log: Duration,
    log_interval: Duration,
}

impl Status {
    fn new(threshold: f32, sample_rate: u32, channels: u16, live: bool, log_interval: f64) -> Self {
        let started = Instant::now();
        Self {
            started,
            threshold,
            sample_rate,
            channels,
            keywords: "(unknown)".to_string(),
            last_wake: None,
            // Paint and log on the first frame rather than waiting for a tick.
            live,
            last_render: LIVE_REFRESH,
            last_log: Duration::ZERO,
            log_interval: Duration::from_secs_f64(log_interval.max(0.0)),
        }
    }

    fn record(&mut self, events: &[DetectorEvent]) {
        for event in events {
            match event {
                DetectorEvent::Wake(wake) => {
                    let at = self.started.elapsed();
                    println!("WAKE {}", wake.raw_json());
                    eprintln!(
                        "[{}] wake word detected: {} at {:.1}s",
                        clock(),
                        wake.keyword,
                        at.as_secs_f64()
                    );
                    self.last_wake = Some(LastWake {
                        keyword: wake.keyword.clone(),
                        at,
                    });
                }
                DetectorEvent::Error {
                    code,
                    message,
                    retryable,
                } => {
                    eprintln!(
                        "[{}] detector {code}: {message} (retryable: {retryable})",
                        clock()
                    );
                }
            }
        }
    }

    /// Paint the live line and/or emit a periodic log line. Cheap to call per chunk.
    fn tick(&mut self, stats: &DetectorSessionStats) {
        let elapsed = self.started.elapsed();
        if self.live && elapsed.saturating_sub(self.last_render) >= LIVE_REFRESH {
            let line = live_line(self, stats, elapsed);
            eprint!("\r\x1b[2K{line}");
            let _ = std::io::stderr().flush();
            self.last_render = elapsed;
        }
        if !self.log_interval.is_zero() && elapsed.saturating_sub(self.last_log) >= self.log_interval {
            // A log line would land on top of the live line and be wiped by the
            // next repaint, so close the row, log, and repaint straight after.
            if self.live {
                eprintln!();
            }
            eprintln!("{}", log_line(self, stats, elapsed));
            if self.live {
                self.last_render = Duration::ZERO;
            }
            self.last_log = elapsed;
        }
    }

    fn summary(&self, stats: &DetectorSessionStats) {
        if self.live {
            eprintln!();
        }
        eprintln!(
            "[{}] stopped after {:.1}s: {} frames, {} samples, peak {}, {}",
            clock(),
            self.started.elapsed().as_secs_f64(),
            stats.frame_count,
            stats.sample_count,
            decibels(stats.peak),
            match &self.last_wake {
                Some(wake) => format!("last wake {}", wake.keyword),
                None => "no wake word".to_string(),
            }
        );
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let keywords = std::fs::read_to_string(&args.keywords_file).unwrap_or_else(|_| "(unknown)".to_string());
    let detector = Detector::load(DetectorConfig {
        model_dir: args.model_dir,
        keywords_file: args.keywords_file,
        threads: args.threads,
        threshold: args.threshold,
    })?;

    if let Some(wav_path) = args.wav {
        test_wav(&detector, &wav_path)
    } else {
        listen(&detector, keywords, args.threshold, args.log_interval, args.no_live)
    }
}

/// A peak of zero is silence, not minus infinity; anything above it becomes dBFS.
fn decibels(amplitude: f32) -> String {
    if amplitude.abs() < SILENCE_PEAK {
        return "silent".to_string();
    }
    format!("{:.1} dBFS", 20.0 * amplitude.abs().log10())
}

fn clock() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let seconds = now.as_secs() % 86_400;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds % 3600) / 60,
        seconds % 60
    )
}

/// The single-line dashboard a human watches while the microphone is open.
fn live_line(status: &Status, stats: &DetectorSessionStats, elapsed: Duration) -> String {
    if let Some(wake) = status
        .last_wake
        .as_ref()
        .filter(|wake| elapsed.saturating_sub(wake.at) < WAKE_FLASH)
    {
        return format!(
            "\x1b[1;33m★ WAKE WORD DETECTED: {} at {:.1}s\x1b[0m",
            wake.keyword,
            wake.at.as_secs_f64()
        );
    }
    let signal = if stats.peak >= SILENCE_PEAK {
        "\x1b[32m●\x1b[0m"
    } else {
        "\x1b[2m○\x1b[0m"
    };
    let last_wake = match &status.last_wake {
        Some(wake) => format!(
            "last wake {} {:.1}s ago",
            wake.keyword,
            elapsed.saturating_sub(wake.at).as_secs_f64()
        ),
        None => "last wake none".to_string(),
    };
    format!(
        "{signal} listening {:.1}s · {} Hz x{} · frames {} · peak {} · rms {} · threshold {:.2} · {last_wake}",
        elapsed.as_secs_f64(),
        status.sample_rate,
        status.channels,
        stats.frame_count,
        decibels(stats.peak),
        decibels(stats.rms as f32),
        status.threshold
    )
}

/// The same information as one parseable line, for pipes, systemd and journald.
fn log_line(status: &Status, stats: &DetectorSessionStats, elapsed: Duration) -> String {
    let last_wake = match &status.last_wake {
        Some(wake) => format!("{:.1}s", elapsed.saturating_sub(wake.at).as_secs_f64()),
        None => "none".to_string(),
    };
    format!(
        "[status] up={:.1}s rate={} ch={} frames={} samples={} peak={} rms={} threshold={:.2} keyword={} last_wake={last_wake}",
        elapsed.as_secs_f64(),
        status.sample_rate,
        status.channels,
        stats.frame_count,
        stats.sample_count,
        decibels(stats.peak).replace(' ', "_"),
        decibels(stats.rms as f32).replace(' ', "_"),
        status.threshold,
        status.last_wake.as_ref().map_or("-", |wake| wake.keyword.as_str())
    )
}

fn test_wav(detector: &Detector, wav_path: &PathBuf) -> Result<()> {
    eprintln!("[{}] reading {}", clock(), wav_path.display());
    let result = detector.run_wav(wav_path)?;
    for wake in &result.wakes {
        println!("WAKE {}", wake.raw_json());
        eprintln!(
            "[{}] wake word detected: {} {}",
            clock(),
            wake.keyword,
            match wake.json.get("start_time").and_then(|value| value.as_f64()) {
                Some(seconds) => format!("{seconds:.2}s into the clip"),
                None => format!("at {}ms", wake.timestamp_ms)
            }
        );
    }
    if !result.detected {
        println!("NO WAKE");
    }
    let stats = &result.stats;
    eprintln!(
        "[{}] finished: {:.2}s, {} frames, {} samples at {} Hz, peak {}, {}",
        clock(),
        stats.duration_ms as f64 / 1000.0,
        stats.frame_count,
        stats.sample_count,
        stats.sample_rate,
        decibels(stats.peak),
        if result.detected {
            "wake word detected"
        } else {
            "no wake word"
        }
    );
    Ok(())
}

fn build_input_stream<T, Convert>(
    device: &cpal::Device,
    config: &StreamConfig,
    sender: SyncSender<Vec<f32>>,
    convert: Convert,
) -> Result<Stream>
where
    T: cpal::SizedSample + Copy,
    Convert: Fn(T) -> f32 + Send + 'static,
{
    let error_sender = sender.clone();
    device
        .build_input_stream(
            config,
            move |samples: &[T], _| {
                let chunk = samples.iter().copied().map(&convert).collect();
                // A bounded channel prevents the audio callback from building an unbounded backlog.
                let _ = sender.try_send(chunk);
            },
            move |error| {
                let _ = error_sender.try_send(Vec::new());
                eprintln!("[{}] capture error: {error}", clock());
            },
            None
        )
        .context("Could not open the microphone input stream")
}

fn listen(
    detector: &Detector,
    keywords: String,
    threshold: f32,
    log_interval: f64,
    no_live: bool,
) -> Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No default microphone was found."))?;
    let supported = device
        .default_input_config()
        .context("Could not query the microphone's default audio format")?;
    let config: StreamConfig = supported.clone().into();
    let channels = config.channels;
    let sample_rate = config.sample_rate.0;

    let mut status = Status::new(
        threshold,
        sample_rate,
        channels,
        !no_live && std::io::stderr().is_terminal(),
        log_interval,
    );
    status.keywords = keywords.trim().replace('\n', " / ");
    eprintln!(
        "[{}] model loaded, listening for: {}",
        clock(),
        status.keywords
    );

    let mut session = detector.open_session("cpal", sample_rate)?;
    let (sender, receiver) = mpsc::sync_channel::<Vec<f32>>(32);
    let input = match supported.sample_format() {
        SampleFormat::F32 => {
            build_input_stream::<f32, _>(&device, &config, sender, |sample| sample)?
        }
        SampleFormat::I16 => build_input_stream::<i16, _>(&device, &config, sender, |sample| {
            sample as f32 / i16::MAX as f32
        })?,
        SampleFormat::U16 => build_input_stream::<u16, _>(&device, &config, sender, |sample| {
            (sample as f32 - 32768.0) / 32768.0
        })?,
        format => bail!("Unsupported microphone sample format: {format:?}"),
    };
    input.play().context("Could not start microphone capture")?;
    eprintln!(
        "[{}] listening on {} at {} Hz ({} channel(s)). Press Ctrl+C to stop.",
        clock(),
        device
            .name()
            .unwrap_or_else(|_| "default input".to_string()),
        sample_rate,
        channels
    );

    while let Ok(interleaved) = receiver.recv() {
        if interleaved.is_empty() {
            continue;
        }
        let mono: Vec<f32> = interleaved
            .chunks(channels as usize)
            .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
            .collect();
        let events = session.accept_pcm(&mono)?;
        status.record(&events);
        status.tick(session.stats());
    }

    status.summary(session.stats());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(peak: f32, rms: f64, frames: u64) -> DetectorSessionStats {
        DetectorSessionStats {
            sample_rate: 48_000,
            frame_count: frames,
            sample_count: frames * 960,
            duration_ms: frames * 20,
            rms,
            peak,
        }
    }

    fn status() -> Status {
        Status::new(0.25, 48_000, 2, false, 5.0)
    }

    #[test]
    fn quiet_input_reads_as_silence_not_minus_infinity() {
        assert_eq!(decibels(0.0), "silent");
        assert_eq!(decibels(1.0e-9), "silent");
        assert_eq!(decibels(1.0), "0.0 dBFS");
        assert_eq!(decibels(0.5), "-6.0 dBFS");
        assert_eq!(decibels(-0.25), "-12.0 dBFS");
    }

    #[test]
    fn live_line_shows_capture_rate_frames_and_level() {
        let status = status();
        let line = live_line(&status, &stats(0.5, 0.1, 100), Duration::from_secs(3));
        assert!(line.contains("listening 3.0s"), "{line}");
        assert!(line.contains("48000 Hz x2"), "{line}");
        assert!(line.contains("frames 100"), "{line}");
        assert!(line.contains("peak -6.0 dBFS"), "{line}");
        assert!(line.contains("rms -20.0 dBFS"), "{line}");
        assert!(line.contains("threshold 0.25"), "{line}");
        assert!(line.contains("last wake none"), "{line}");
    }

    #[test]
    fn live_line_distinguishes_silence_from_signal() {
        let status = status();
        let silent = live_line(&status, &stats(0.0, 0.0, 4), Duration::from_secs(1));
        assert!(silent.contains("peak silent"), "{silent}");
        assert!(silent.contains("rms silent"), "{silent}");
        // A broken capture path and a quiet room look the same; signal does not.
        assert!(silent.contains('\u{25cb}'), "{silent}");

        let signal = live_line(&status, &stats(0.01, 0.001, 5), Duration::from_secs(1));
        assert!(signal.contains('\u{25cf}'), "{signal}");
        assert!(signal.contains("peak -40.0 dBFS"), "{signal}");
    }

    #[test]
    fn live_line_flashes_only_just_after_a_wake() {
        let mut status = status();
        status.last_wake = Some(LastWake {
            keyword: "AGENT".to_string(),
            at: Duration::from_secs(5),
        });
        let stats = stats(0.2, 0.05, 10);

        let flashed = live_line(&status, &stats, Duration::from_secs(6));
        assert!(flashed.contains("WAKE WORD DETECTED: AGENT at 5.0s"), "{flashed}");

        let settled = live_line(&status, &stats, Duration::from_secs(20));
        assert!(settled.contains("last wake AGENT 15.0s ago"), "{settled}");
    }

    #[test]
    fn log_line_is_one_parsable_line() {
        let status = status();
        let line = log_line(&status, &stats(0.0, 0.0, 7), Duration::from_millis(12_500));
        assert!(line.starts_with("[status] up=12.5s rate=48000 ch=2 frames=7 samples=6720 "), "{line}");
        assert!(line.contains("peak=silent rms=silent"), "{line}");
        assert!(line.ends_with("keyword=- last_wake=none"), "{line}");
        assert!(
            line.split(' ').skip(1).all(|field| field.contains('=')),
            "every field must be key=value so the line can be split on spaces: {line}"
        );
    }

    #[test]
    fn log_line_reports_levels_and_the_last_wake_without_spaces() {
        let mut status = status();
        status.last_wake = Some(LastWake {
            keyword: "AGENT".to_string(),
            at: Duration::from_secs(9),
        });
        let line = log_line(&status, &stats(0.5, 0.25, 10), Duration::from_secs(10));
        assert!(line.contains("peak=-6.0_dBFS rms=-12.0_dBFS"), "{line}");
        assert!(line.ends_with("keyword=AGENT last_wake=1.0s"), "{line}");
    }

    #[test]
    fn clock_uses_a_24_hour_format() {
        let stamp = clock();
        assert_eq!(stamp.len(), 8, "{stamp}");
        assert!(
            stamp.chars().enumerate().all(|(index, c)| {
                if index == 2 || index == 5 {
                    c == ':'
                } else {
                    c.is_ascii_digit()
                }
            }),
            "{stamp}"
        );
    }
}
