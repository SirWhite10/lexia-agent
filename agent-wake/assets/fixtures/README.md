# Wake phrase WAV fixture

`agent-wake.wav` is a generated, mono 16 kHz PCM WAV containing the phrase "Hey agent". It was produced locally with eSpeak NG 1.52.0 using the `en-us` voice, then converted to the detector's native WAV input format with a local audio conversion utility. It is a test-only synthetic recording, not a recording of a person. The generated fixture is committed for reproducible detector verification; do not substitute an unverified sample.

- Fixture ID: `agent-wake`
- Expected detector keyword: `AGENT`
- Expected detector result: one wake event
- Source/tool: eSpeak NG 1.52.0, generated locally for this repository
- Audio format: mono, 16 kHz, PCM WAV
- Runtime use: server-side detector fixture only; never copied to an upload directory or logged as samples

The phrase carries a word before it on purpose. The model does not fire on an isolated "Agent" at any threshold tried — 0.25, 0.10 and 0.05 — but it fires on the same keyword inside a phrase, which is what a person saying "Agent, do this" sounds like. The keyword the detector reports is `AGENT` either way, so the phrase the agent answers to is unchanged.

The fixture is intentionally separate from browser microphone capture. The page runs it through the same Rust detector session/WAV path, so model failures can be distinguished from permission, device and connection failures.
