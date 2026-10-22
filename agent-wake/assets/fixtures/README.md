# Wake-word WAV fixture

`agent-wake.wav` is a generated, mono 16 kHz PCM WAV containing the single test phrase “Lexia”. It was produced locally with eSpeak NG 1.52.0 using the `en-us` voice, then converted to the detector's native WAV input format with a local audio conversion utility. It is a test-only synthetic recording, not a recording of a person. The generated fixture is committed for reproducible detector verification; do not substitute an unverified sample.

- Fixture ID: `agent-wake`
- Expected detector keyword: `LEXIA`
- Expected detector result: one wake event
- Source/tool: eSpeak NG 1.52.0, generated locally for this repository
- Audio format: mono, 16 kHz, PCM WAV
- Runtime use: server-side detector fixture only; never copied to an upload directory or logged as samples

The fixture is intentionally separate from browser microphone capture. The UI runs it through the same Rust detector session/WAV path so model failures can be distinguished from permission, device, and WSS failures.
