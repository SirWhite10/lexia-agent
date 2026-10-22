use std::{
    collections::HashMap,
    fmt,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};

use crate::detector::{
    Detector, DetectorEvent, DetectorSession, DetectorSessionStats, FixtureResult, WakeDetection,
    MAX_SAMPLE_RATE, MIN_SAMPLE_RATE,
};

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
pub const MAX_SESSIONS: usize = 1;
pub const MAX_QUEUE_DURATION_MS: u32 = 250;
const CONTROL_KIND: u8 = 0x01;
const PCM_KIND: u8 = 0x02;
const FIXTURE_ROOT: &str = "assets";

pub fn run<R: Read, W: Write>(
    input: R,
    output: W,
    startup: Result<Detector, String>,
) -> Result<()> {
    let output = output;
    let mut worker = Worker::new(output);
    let detector = match startup {
        Ok(detector) => detector,
        Err(message) => {
            let request_id = worker.read_hello(input)?;
            worker.send(
                json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "type": "ready",
                    "requestId": request_id,
                    "ready": false,
                    "modelReady": false,
                    "error": {
                        "code": "model_load",
                        "message": message,
                        "retryable": false
                    }
                }),
                CONTROL_KIND,
            )?;
            return Ok(());
        }
    };

    worker.serve(input, &detector)
}

struct Worker<W: Write> {
    output: W,
    hello_received: bool,
}

#[derive(Debug)]
struct Frame {
    kind: u8,
    payload: Vec<u8>,
}

struct WorkerSession<'a> {
    detector: DetectorSession<'a>,
    next_sequence: u64,
}

#[derive(Debug)]
struct WorkerReadError(String);

impl fmt::Display for WorkerReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for WorkerReadError {}

impl<W: Write> Worker<W> {
    fn new(output: W) -> Self {
        Self {
            output,
            hello_received: false,
        }
    }

    fn serve<R: Read>(mut self, mut input: R, detector: &Detector) -> Result<()> {
        let mut sessions = HashMap::new();
        loop {
            let frame = match read_frame(&mut input).context("read worker frame") {
                Ok(Some(frame)) => frame,
                Ok(None) => return Ok(()),
                Err(error) => {
                    self.send_error(None, None, "malformed_frame", &error.to_string(), false)?;
                    return Err(error);
                }
            };

            match frame.kind {
                CONTROL_KIND => {
                    if self.handle_control(&frame.payload, detector, &mut sessions)? {
                        return Ok(());
                    }
                }
                PCM_KIND => match self.handle_pcm(&frame.payload, &mut sessions) {
                    Ok((session_id, events)) => {
                        let stats = sessions
                            .get(&session_id)
                            .expect("accepted PCM keeps its session open")
                            .detector
                            .stats();
                        self.send_events(None, &session_id, events)?;
                        self.send_stats(session_id, stats)?;
                    }
                    Err(error) => {
                        self.send_error(
                            None,
                            error.session_id.as_ref(),
                            error.code,
                            &error.message,
                            error.retryable,
                        )?;
                        if error.close_session {
                            if let Some(session_id) = error.session_id {
                                sessions.remove(&session_id);
                            }
                        }
                    }
                },
                other => {
                    self.send_error(
                        None,
                        None,
                        "unknown_frame_kind",
                        &format!("unsupported frame kind: {other}"),
                        false,
                    )?;
                }
            }
        }
    }

    fn read_hello<R: Read>(&mut self, mut input: R) -> Result<Option<Value>> {
        let Some(frame) = read_frame(&mut input).context("read hello frame")? else {
            return Ok(None);
        };
        if frame.kind != CONTROL_KIND {
            self.send_error(
                None,
                None,
                "expected_control",
                "hello must be a JSON control frame",
                false,
            )?;
            return Ok(None);
        }
        let request_id = serde_json::from_slice::<Value>(&frame.payload)
            .ok()
            .and_then(|value| value.get("requestId").cloned())
            .filter(|value| !value.is_null());
        match parse_command(&frame.payload) {
            Ok(WorkerCommand::Hello {
                protocol_version, ..
            }) if protocol_version == PROTOCOL_VERSION => Ok(request_id),
            _ => {
                self.send_error(
                    request_id,
                    None,
                    "invalid_hello",
                    "first command must be a version 1 hello",
                    false,
                )?;
                Ok(None)
            }
        }
    }

    fn handle_control<'a>(
        &mut self,
        payload: &[u8],
        detector: &'a Detector,
        sessions: &mut HashMap<SessionId, WorkerSession<'a>>,
    ) -> Result<bool> {
        let command = match parse_command(payload) {
            Ok(command) => command,
            Err(message) => {
                self.send_error(None, None, "invalid_json", &message, true)?;
                return Ok(false);
            }
        };
        if matches!(&command, WorkerCommand::Hello { .. }) {
            if self.hello_received {
                self.send_error(
                    command.request_id(),
                    None,
                    "already_initialized",
                    "hello has already been accepted",
                    false,
                )?;
                return Ok(false);
            }
            if command.protocol_version() != PROTOCOL_VERSION {
                self.send_error(
                    command.request_id(),
                    None,
                    "unsupported_protocol",
                    "unsupported protocol version",
                    false,
                )?;
                return Ok(false);
            }
            self.hello_received = true;
            self.send(
                json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "type": "ready",
                    "requestId": command.request_id(),
                    "ready": true,
                    "modelReady": true,
                    "threshold": detector.threshold(),
                    "limits": {
                        "maxFrameBytes": MAX_FRAME_BYTES,
                        "maxSessions": MAX_SESSIONS,
                        "minSampleRate": MIN_SAMPLE_RATE,
                        "maxSampleRate": MAX_SAMPLE_RATE,
                        "maxQueueDurationMs": MAX_QUEUE_DURATION_MS
                    }
                }),
                CONTROL_KIND,
            )?;
            return Ok(false);
        }
        if !self.hello_received {
            self.send_error(
                command.request_id(),
                None,
                "not_initialized",
                "hello must be the first command",
                false,
            )?;
            return Ok(false);
        }

        match command {
            WorkerCommand::Hello { .. } => unreachable!(),
            WorkerCommand::OpenSession {
                request_id,
                session_id,
                sample_rate,
            } => {
                if sessions.contains_key(&session_id) {
                    self.send_error(
                        request_id,
                        Some(&session_id),
                        "session_exists",
                        "session is already open",
                        false,
                    )?;
                } else if sessions.len() >= MAX_SESSIONS {
                    self.send_error(
                        request_id,
                        Some(&session_id),
                        "backpressure",
                        "the worker session limit has been reached",
                        true,
                    )?;
                } else {
                    match detector.open_session(session_id.to_string(), sample_rate) {
                        Ok(session) => {
                            sessions.insert(
                                session_id,
                                WorkerSession {
                                    detector: session,
                                    next_sequence: 0,
                                },
                            );
                            self.send(
                                json!({
                                    "protocolVersion": PROTOCOL_VERSION,
                                    "type": "status",
                                    "requestId": request_id,
                                    "sessionId": session_id,
                                    "state": "open",
                                    "sampleRate": sample_rate
                                }),
                                CONTROL_KIND,
                            )?;
                        }
                        Err(error) => self.send_error(
                            request_id,
                            Some(&session_id),
                            "invalid_session",
                            &error.to_string(),
                            false,
                        )?,
                    }
                }
            }
            WorkerCommand::CloseSession {
                request_id,
                session_id,
            } => {
                if let Some(session) = sessions.remove(&session_id) {
                    self.finish_session(request_id, &session_id, session)?;
                } else {
                    self.send_error(
                        request_id,
                        Some(&session_id),
                        "session_not_found",
                        "session is not open",
                        false,
                    )?;
                }
            }
            WorkerCommand::RunFixture { request_id, path } => {
                match allowlisted_fixture(Path::new(&path)) {
                    Ok(path) => match detector.run_wav(&path) {
                        Ok(result) => {
                            self.send_fixture_result(request_id, &path, result)?;
                        }
                        Err(error) => self.send_error(
                            request_id,
                            None,
                            "fixture_failed",
                            &error.to_string(),
                            false,
                        )?,
                    },
                    Err(message) => {
                        self.send_error(request_id, None, "fixture_not_allowed", &message, false)?
                    }
                }
            }
            WorkerCommand::Shutdown { request_id } => {
                let session_ids = sessions.keys().copied().collect::<Vec<_>>();
                for session_id in session_ids {
                    if let Some(session) = sessions.remove(&session_id) {
                        self.finish_session(None, &session_id, session)?;
                    }
                }
                self.send(
                    json!({
                        "protocolVersion": PROTOCOL_VERSION,
                        "type": "stopped",
                        "requestId": request_id
                    }),
                    CONTROL_KIND,
                )?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn handle_pcm(
        &mut self,
        payload: &[u8],
        sessions: &mut HashMap<SessionId, WorkerSession<'_>>,
    ) -> std::result::Result<(SessionId, Vec<DetectorEvent>), PcmCommandError> {
        if !self.hello_received {
            return Err(PcmCommandError::new(
                "not_initialized",
                "hello must be the first command",
                false,
            ));
        }
        let pcm = PcmPayload::parse(payload)
            .map_err(|message| PcmCommandError::new("malformed_pcm", &message, false))?;
        let session = sessions.get_mut(&pcm.session_id).ok_or_else(|| {
            PcmCommandError::session(
                "session_not_found",
                "session is not open",
                &pcm.session_id,
                false,
            )
        })?;
        if pcm.sequence != session.next_sequence {
            let message = format!(
                "expected PCM sequence {}, received {}",
                session.next_sequence, pcm.sequence
            );
            return Err(PcmCommandError::session(
                "sequence_gap",
                &message,
                &pcm.session_id,
                true,
            ));
        }
        let events = session.detector.accept_pcm(&pcm.samples).map_err(|error| {
            PcmCommandError::session("invalid_pcm", &error.to_string(), &pcm.session_id, true)
        })?;
        session.next_sequence += 1;
        Ok((pcm.session_id, events))
    }

    fn finish_session(
        &mut self,
        request_id: Option<Value>,
        session_id: &SessionId,
        session: WorkerSession<'_>,
    ) -> Result<()> {
        let result = session
            .detector
            .finish()
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        self.send_events(request_id.clone(), session_id, result.events)?;
        self.send(
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "type": "status",
                "requestId": request_id,
                "sessionId": session_id,
                "state": "closed",
                "stats": result.stats
            }),
            CONTROL_KIND,
        )
    }

    fn send_events(
        &mut self,
        request_id: Option<Value>,
        session_id: &SessionId,
        events: Vec<DetectorEvent>,
    ) -> Result<()> {
        for event in events {
            match event {
                DetectorEvent::Wake(detection) => {
                    self.send_wake(request_id.clone(), session_id, detection)?
                }
                DetectorEvent::Error {
                    code,
                    message,
                    retryable,
                } => self.send_error(
                    request_id.clone(),
                    Some(session_id),
                    code,
                    &message,
                    retryable,
                )?,
            }
        }
        Ok(())
    }

    fn send_wake(
        &mut self,
        request_id: Option<Value>,
        session_id: &SessionId,
        detection: WakeDetection,
    ) -> Result<()> {
        self.send(
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "type": "wake",
                "requestId": request_id,
                "sessionId": session_id,
                "keyword": detection.keyword,
                "timestamp": detection.timestamp_ms,
                "start_time": detection.start_time,
                "timestamps": detection.timestamps,
                "json": detection.json
            }),
            CONTROL_KIND,
        )
    }

    fn send_stats(&mut self, session_id: SessionId, stats: &DetectorSessionStats) -> Result<()> {
        self.send(
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "type": "stats",
                "sessionId": session_id,
                "stats": stats
            }),
            CONTROL_KIND,
        )
    }

    fn send_fixture_result(
        &mut self,
        request_id: Option<Value>,
        path: &Path,
        result: FixtureResult,
    ) -> Result<()> {
        self.send(
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "type": "fixture_result",
                "requestId": request_id,
                "fixture": path.file_name().and_then(|name| name.to_str()).unwrap_or("fixture.wav"),
                "detected": result.detected,
                "wakes": result.wakes,
                "stats": result.stats
            }),
            CONTROL_KIND,
        )
    }

    fn send_error(
        &mut self,
        request_id: Option<Value>,
        session_id: Option<&SessionId>,
        code: &str,
        message: &str,
        retryable: bool,
    ) -> Result<()> {
        self.send(
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "type": "error",
                "requestId": request_id,
                "sessionId": session_id,
                "error": {
                    "code": code,
                    "message": message,
                    "retryable": retryable
                }
            }),
            CONTROL_KIND,
        )
    }

    fn send(&mut self, message: Value, kind: u8) -> Result<()> {
        write_frame(&mut self.output, kind, &serde_json::to_vec(&message)?)
    }
}

#[derive(Debug)]
struct PcmCommandError {
    code: &'static str,
    message: String,
    session_id: Option<SessionId>,
    close_session: bool,
    retryable: bool,
}

impl PcmCommandError {
    fn new(code: &'static str, message: &str, retryable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            session_id: None,
            close_session: false,
            retryable,
        }
    }

    fn session(
        code: &'static str,
        message: &str,
        session_id: &SessionId,
        close_session: bool,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            session_id: Some(*session_id),
            close_session,
            retryable: false,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommandEnvelope {
    protocol_version: u32,
    #[serde(default)]
    request_id: Option<Value>,
    #[serde(rename = "type")]
    command_type: String,
}

#[derive(Debug)]
enum WorkerCommand {
    Hello {
        protocol_version: u32,
        request_id: Option<Value>,
    },
    OpenSession {
        request_id: Option<Value>,
        session_id: SessionId,
        sample_rate: u32,
    },
    CloseSession {
        request_id: Option<Value>,
        session_id: SessionId,
    },
    RunFixture {
        request_id: Option<Value>,
        path: String,
    },
    Shutdown {
        request_id: Option<Value>,
    },
}

impl WorkerCommand {
    fn request_id(&self) -> Option<Value> {
        match self {
            Self::Hello { request_id, .. }
            | Self::OpenSession { request_id, .. }
            | Self::CloseSession { request_id, .. }
            | Self::RunFixture { request_id, .. }
            | Self::Shutdown { request_id } => request_id.clone(),
        }
    }

    fn protocol_version(&self) -> u32 {
        match self {
            Self::Hello {
                protocol_version, ..
            } => *protocol_version,
            _ => 0,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenSessionPayload {
    session_id: SessionId,
    sample_rate: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionPayload {
    session_id: SessionId,
}

#[derive(Deserialize)]
struct FixturePayload {
    path: String,
}

fn parse_command(payload: &[u8]) -> std::result::Result<WorkerCommand, String> {
    let envelope: CommandEnvelope =
        serde_json::from_slice(payload).map_err(|error| error.to_string())?;
    if envelope.protocol_version != PROTOCOL_VERSION {
        return Err(format!(
            "unsupported protocol version: {}",
            envelope.protocol_version
        ));
    }
    let request_id = envelope.request_id.filter(|value| !value.is_null());
    match envelope.command_type.as_str() {
        "hello" => Ok(WorkerCommand::Hello {
            protocol_version: envelope.protocol_version,
            request_id,
        }),
        "open_session" => {
            let body: OpenSessionPayload =
                serde_json::from_slice(payload).map_err(|error| error.to_string())?;
            Ok(WorkerCommand::OpenSession {
                request_id,
                session_id: body.session_id,
                sample_rate: body.sample_rate,
            })
        }
        "close_session" => {
            let body: SessionPayload =
                serde_json::from_slice(payload).map_err(|error| error.to_string())?;
            Ok(WorkerCommand::CloseSession {
                request_id,
                session_id: body.session_id,
            })
        }
        "run_fixture" => {
            let body: FixturePayload =
                serde_json::from_slice(payload).map_err(|error| error.to_string())?;
            Ok(WorkerCommand::RunFixture {
                request_id,
                path: body.path,
            })
        }
        "shutdown" => Ok(WorkerCommand::Shutdown { request_id }),
        other => Err(format!("unknown command type: {other}")),
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct SessionId([u8; 16]);

impl fmt::Display for SessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, byte) in self.0.iter().enumerate() {
            if matches!(index, 4 | 6 | 8 | 10) {
                formatter.write_str("-")?;
            }
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl Serialize for SessionId {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SessionId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        parse_session_id(&value).map_err(D::Error::custom)
    }
}

fn parse_session_id(value: &str) -> std::result::Result<SessionId, String> {
    if value.len() != 36 {
        return Err("session id must be a canonical UUID".into());
    }
    let bytes = value.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            if *byte != b'-' {
                return Err("session id must be a canonical UUID".into());
            }
        } else if !byte.is_ascii_hexdigit() {
            return Err("session id must be a canonical UUID".into());
        }
    }
    let mut hex = [0_u8; 32];
    let mut hex_index = 0;
    for byte in bytes.iter().copied().filter(|byte| *byte != b'-') {
        hex[hex_index] = byte;
        hex_index += 1;
    }
    let mut id = [0_u8; 16];
    for index in 0..16 {
        let high = hex_value(hex[index * 2])?;
        let low = hex_value(hex[index * 2 + 1])?;
        id[index] = (high << 4) | low;
    }
    Ok(SessionId(id))
}

fn hex_value(byte: u8) -> std::result::Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err("session id must be a canonical UUID".into()),
    }
}

struct PcmPayload {
    session_id: SessionId,
    sequence: u64,
    samples: Vec<f32>,
}

impl PcmPayload {
    fn parse(payload: &[u8]) -> std::result::Result<Self, String> {
        const HEADER_BYTES: usize = 16 + 8;
        if payload.len() <= HEADER_BYTES || (payload.len() - HEADER_BYTES) % 4 != 0 {
            return Err("PCM payload length is invalid".into());
        }
        let session_id = SessionId(
            payload[..16]
                .try_into()
                .map_err(|_| "PCM session id is invalid".to_string())?,
        );
        let sequence = u64::from_be_bytes(
            payload[16..24]
                .try_into()
                .map_err(|_| "PCM sequence is invalid".to_string())?,
        );
        let samples = payload[HEADER_BYTES..]
            .chunks_exact(4)
            .map(|bytes| f32::from_le_bytes(bytes.try_into().expect("chunks_exact has four bytes")))
            .collect();
        Ok(Self {
            session_id,
            sequence,
            samples,
        })
    }
}

fn read_frame<R: Read>(input: &mut R) -> std::result::Result<Option<Frame>, WorkerReadError> {
    let mut first_length_byte = [0_u8; 1];
    match input.read(&mut first_length_byte) {
        Ok(0) => return Ok(None),
        Ok(1) => {}
        Ok(_) => return Err(WorkerReadError("frame length prefix is invalid".into())),
        Err(error) => return Err(WorkerReadError(error.to_string())),
    }
    let mut remaining_length = [0_u8; 3];
    input
        .read_exact(&mut remaining_length)
        .map_err(|error| WorkerReadError(error.to_string()))?;
    let length_bytes = [
        first_length_byte[0],
        remaining_length[0],
        remaining_length[1],
        remaining_length[2],
    ];
    let length = u32::from_be_bytes(length_bytes) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(WorkerReadError(format!(
            "frame payload is {length} bytes; maximum is {MAX_FRAME_BYTES}"
        )));
    }
    let mut header = [0_u8; 1];
    input
        .read_exact(&mut header)
        .map_err(|error| WorkerReadError(error.to_string()))?;
    let mut payload = vec![0_u8; length];
    input
        .read_exact(&mut payload)
        .map_err(|error| WorkerReadError(error.to_string()))?;
    Ok(Some(Frame {
        kind: header[0],
        payload,
    }))
}

fn write_frame<W: Write>(output: &mut W, kind: u8, payload: &[u8]) -> Result<()> {
    if payload.len() > MAX_FRAME_BYTES {
        anyhow::bail!("outgoing frame exceeds maximum payload size");
    }
    output.write_all(&(payload.len() as u32).to_be_bytes())?;
    output.write_all(&[kind])?;
    output.write_all(payload)?;
    output.flush()?;
    Ok(())
}

fn allowlisted_fixture(path: &Path) -> std::result::Result<PathBuf, String> {
    if path.extension().and_then(|extension| extension.to_str()) != Some("wav") {
        return Err("fixture must be a .wav file under assets".into());
    }
    let root = std::fs::canonicalize(FIXTURE_ROOT)
        .map_err(|_| "fixture assets directory is unavailable".to_string())?;
    let fixture = std::fs::canonicalize(path).map_err(|_| "fixture does not exist".to_string())?;
    if !fixture.starts_with(&root) || fixture == root {
        return Err("fixture must be a .wav file under assets".into());
    }
    Ok(fixture)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn frame_round_trip_preserves_kind_and_payload() {
        let mut bytes = Vec::new();
        write_frame(&mut bytes, CONTROL_KIND, br#"{"type":"hello"}"#).unwrap();
        let mut input = Cursor::new(bytes);
        let frame = read_frame(&mut input).unwrap().unwrap();
        assert_eq!(frame.kind, CONTROL_KIND);
        assert_eq!(frame.payload, br#"{"type":"hello"}"#);
        assert!(read_frame(&mut input).unwrap().is_none());
    }

    #[test]
    fn rejects_oversized_and_truncated_frames() {
        let mut oversized = Cursor::new(((MAX_FRAME_BYTES + 1) as u32).to_be_bytes().to_vec());
        assert!(matches!(
            read_frame(&mut oversized),
            Err(WorkerReadError(_))
        ));

        let mut truncated = Cursor::new(vec![0, 0, 0, 4, 1, 0]);
        assert!(matches!(
            read_frame(&mut truncated),
            Err(WorkerReadError(_))
        ));
    }

    #[test]
    fn pcm_header_validation_handles_malformed_samples() {
        assert!(PcmPayload::parse(&[0; 23]).is_err());
        assert!(PcmPayload::parse(&[0; 25]).is_err());
    }

    #[test]
    fn pcm_envelope_decodes_session_sequence_and_f32_samples() {
        let mut payload = vec![0_u8; 16];
        payload.extend_from_slice(&7_u64.to_be_bytes());
        payload.extend_from_slice(&(-0.5_f32).to_le_bytes());
        let decoded = PcmPayload::parse(&payload).unwrap();
        assert_eq!(decoded.sequence, 7);
        assert_eq!(decoded.samples, [-0.5]);
    }

    #[test]
    fn session_ids_require_canonical_uuid() {
        assert!(parse_session_id("00000000-0000-4000-8000-000000000001").is_ok());
        assert!(parse_session_id("not-a-uuid").is_err());
        assert!(parse_session_id("00000000000040008000000000000001").is_err());
    }

    #[test]
    fn parses_typed_control_commands() {
        let command = parse_command(
            br#"{"protocolVersion":1,"type":"open_session","requestId":"a","sessionId":"00000000-0000-4000-8000-000000000001","sampleRate":48000}"#,
        )
        .unwrap();
        assert!(matches!(
            command,
            WorkerCommand::OpenSession {
                sample_rate: 48_000,
                ..
            }
        ));
        assert!(parse_command(br#"{"protocolVersion":1,"type":"mystery"}"#).is_err());
    }
}
