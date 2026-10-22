# Browser microphone streaming handoff

## Goal

Build a browser-based test UI for the sherpa-onnx wake-word prototype. The UI runs in Svelte in the user's browser. The browser captures audio from the user's Arch or Bazzite machine and streams it to the more powerful host, where the existing Rust sherpa-onnx detector runs.

This is a test harness, not the final always-listening implementation. A browser tab must remain open and active, and the user must grant microphone permission. Later, Lexia can use a native local audio service for reliable background listening while reusing the Svelte UI and detector core.

## Why WSS first

Use a secure WebSocket connection for the first remote-audio prototype. This is one-way mic audio from one browser to one detector. WebRTC is a good fit for calls and peer media, but adds signaling and ICE/STUN/TURN setup that this test does not need. Cloudflare supports proxied WebSockets, so the existing domain or Tunnel can carry the secure connection.

Keep the frontend transport behind a small client interface so a later native Tauri transport can replace WSS without rebuilding the UI. Revisit WebRTC if the project grows into live two-way voice or WSS proves insufficient.

## Process boundaries

- The browser owns microphone permission, device selection, capture, and an immediate local input-level meter.
- The web host owns the WSS endpoint and serves the Svelte app.
- The Rust detector owns model loading, audio resampling, keyword spotting, and detection events. Do not reimplement sherpa-onnx in browser JavaScript.
- The current Rust prototype opens CPAL's default host microphone. Replace or separate that capture path so the detector can also accept audio frames received from the browser. Keep CPAL available for a future local-native mode if practical.
- Keep the detector on the host. Do not forward audio to an LLM or persist it.

The implementation agent should choose the smallest reliable Bun-to-Rust boundary. The current Bun server can own HTTP/WSS and supervise a Rust worker, or a small Rust service can own the streaming endpoint. Keep the detector core in Rust either way, and document the chosen process boundary before adding another runtime or transport dependency.

## Browser capture and audio framing

1. The user explicitly clicks **Start microphone**. Only then request `getUserMedia` permission.
2. List available audio inputs with `enumerateDevices`, and let the user choose one. Labels may be unavailable until permission is granted. Show the selected label and refresh the list on device changes.
3. Capture through an `AudioWorklet` or equivalent bounded audio callback. Downmix to mono in the browser if needed. Send little-endian float32 PCM chunks, with the actual capture sample rate and encoding declared at session start. Do not assume the browser runs at 16 kHz; the detector already accepts other rates and resamples to 16 kHz.
4. Use bounded frames around 20–40 ms and bounded queues. If the connection falls behind, report it and avoid unbounded buffering.
5. The browser may calculate a local RMS/peak meter for responsive UI feedback. The server should also report received frame counts and level statistics so the user can tell whether audio reaches the detector.
6. On stop, device loss, page shutdown, or socket failure, stop all media tracks and release the audio context. Do not keep the microphone open after the user presses Stop.

Suggested initial WebSocket protocol:

- Connect to `wss://<test-host>/ws/audio` after the page has loaded.
- First client message: JSON start metadata, including sample rate, mono channel count, `pcm_f32le`, and a protocol version.
- Subsequent client messages: bounded binary PCM frames.
- Client control message: JSON stop.
- Server messages: JSON status, aggregate audio statistics, wake detections, and structured errors. Never include raw audio in logs or status messages.

The exact frame envelope can change during implementation, but it must identify the sample rate and encoding once per stream and preserve message order.

## UI requirements

Keep the agent dark-first visual style and provide a light-mode toggle. The page should show:

- Microphone permission and selected input device.
- Start/stop controls and a clear listening state.
- A live input meter, capture sample rate, and whether audio frames are reaching the host.
- Detector state: model loading/ready, current threshold, last wake phrase and time.
- A readable event/error log, including connection failures, missing devices, permission denial, and detector startup errors.
- A WAV-fixture test action so model/detection failures can be separated from browser microphone or network failures.
- A clear notice that audio is streamed to the configured host and is not saved.

Make failures diagnostic. For example, distinguish browser permission denial, no input device, WebSocket unavailable, no frames received, model load failure, and stream ended. Do not present the sherpa resampler's sample-rate message as proof that the selected physical microphone is correct.

## Network and security

- Browser microphone APIs require a secure context. Use the HTTPS hostname on the proxied Cloudflare domain or a Cloudflare Tunnel hostname. Plain HTTP to a remote host will not work for microphone access.
- Use WSS for audio. Cloudflare documents support for proxied WebSockets.
- Put a public test hostname behind Cloudflare Access or equivalent authentication before routing it to the app. Do not expose an unauthenticated audio-ingest endpoint.
- Validate the expected `Origin`, authenticate the connection, limit concurrent streams and frame sizes, and close idle or malformed sessions.
- The browser-to-host audio stream passes through Cloudflare when using its proxy or Tunnel. The host receives mic audio. Tell the tester this plainly; do not claim end-to-end or device-local audio processing for this remote test.
- Do not write audio frames to disk, application logs, analytics, or crash reports. Keep only short-lived in-memory buffers required for inference.
- If routing directly to the origin through Cloudflare, use a proxied DNS record and Full (strict) TLS with a valid origin certificate. A Cloudflare Origin CA certificate is for Cloudflare-to-origin TLS; it is not a browser-trusted certificate when proxying is disabled.

## Acceptance checks

1. From a current browser on Arch or Bazzite, open the HTTPS test hostname and grant microphone permission.
2. The page lists inputs after permission, lets the user select one, and identifies that selection in the UI.
3. Speaking or making a test sound moves the local meter. The host's received-frame counter also advances.
4. The server runs the bundled WAV fixture through the same detector core and reports its result independently of the microphone path.
5. Speaking the configured wake phrase produces a visible wake event with a timestamp. Silence does not produce a wake event during a reasonable check.
6. Stop closes the mic track and WSS connection. Disconnects, permission failures, and missing devices produce useful messages rather than a hung spinner.
7. The app stores no captured audio. The static browser client contains no model credentials or agent execution logic.

## Existing prototype facts

- `src/main.rs` currently uses CPAL's default input device. It does not enumerate or select devices.
- The detector logs a resampler when it receives audio at a rate other than 16 kHz. A 44.1 kHz to 16 kHz message means the detector is adapting the supplied rate; it does not identify the physical mic or guarantee that the wake phrase was heard.
- The current Linux x86-64 binary and model are in the project ZIP for local tests. The browser/WSS mode will need a host-side service or updated executable; it is a separate path from the current CPAL microphone mode.
- The Bun server currently serves the ZIP landing page on port 8787. The implementation agent should preserve that utility or replace it intentionally with the Svelte test UI.

## References

- [MediaDevices.getUserMedia](https://developer.mozilla.org/en-US/docs/Web/API/MediaDevices/getUserMedia)
- [MediaDevices.enumerateDevices](https://developer.mozilla.org/en-US/docs/Web/API/MediaDevices/enumerateDevices)
- [Cloudflare WebSockets](https://developers.cloudflare.com/network/websockets/)
- [Cloudflare SSL modes](https://developers.cloudflare.com/ssl/origin-configuration/ssl-modes/)
- [Cloudflare Origin CA](https://developers.cloudflare.com/ssl/origin-configuration/origin-ca/)
- [W3C WebRTC specification](https://www.w3.org/TR/webrtc/)
