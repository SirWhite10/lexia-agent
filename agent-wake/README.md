# Agent sherpa-onnx wake-word service

A Rust microphone service that checks whether sherpa-onnx can detect a wake phrase locally. It does not modify or depend on the agent host. The probe only reports a wake event; it does not record, transcribe, or send audio anywhere.

**Which phrase it listens for.** The phrases an agent answers to are the host's to declare: the service reads them from the host's `GET /api/agent` and spells each one into the detector's keyword file at startup. The platform default is `AGENT`. The bundled fixture says "Hey agent", because that is the word the committed WAV contains, and the fixture checks the detection path rather than the phrase.

**The prebuilt release is not carried in this repository.** The zip, the compiled binary and the model are fetched or built locally; see [Preparing the model](#preparing-the-model) and [Building](#building).

## Requirements

- Rust/Cargo (stable)
- A microphone and its OS permissions for live listening
- Linux: ALSA development files for CPAL (`pkg-config` and `libasound2-dev` on Debian/Ubuntu)
- Python with `sentencepiece`, which is what spells a phrase the way the model's own vocabulary spells it

## Build from source

The release binary is built here rather than carried: it is dynamically linked, needs glibc 2.34 or newer plus `libasound.so.2` (provided by ALSA compatibility/runtime packages on Bazzite and Arch), and was built on x86-64 Linux, so a binary from another machine is a guess about the user's system.

## Prepare the model

```sh
bash scripts/prepare-model.sh
cargo run --release
```

On Windows PowerShell, run `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\prepare-model.ps1` instead of the Bash setup script.

The setup command downloads the English Zipformer keyword-spotting model (about 19 MB) if it is not already present. The Rust crate downloads sherpa-onnx's matching native runtime during the first build.

The keyword list holds the pre-tokenized phrase the detector matches. The service fills it from the host's declared phrases at startup; the committed file carries the fixture's phrase, “AGENT”, because that is the word the committed WAV contains.

## Serve the lab over HTTP

Start the Bun server from this directory:

```sh
bun run serve
```

It listens on `0.0.0.0:8787` by default. Open `http://localhost:8787/` on this machine, or use its reachable address from another device. Set `PORT` or `HOST` to override the bind address. The browser interface for the wake lab now lives in the agent host, which is where the signed-in user is.

To test a mono, 16 kHz WAV file instead of the microphone:

```sh
cargo run --release -- --wav path/to/mono.wav
```

To make detection stricter (fewer accidental wakes) or more sensitive (fewer missed wakes):

```sh
cargo run --release -- --threshold 0.35
```

The threshold is model-specific; test it with speech and ordinary background audio before treating the result as reliable.

## What this validates

- The selected pretrained KWS model accepts a custom keyword list without training a new model.
- Audio is processed locally and a successful match emits a `WAKE { ... }` JSON line.
- It does **not** yet retain the words after the wake phrase, run speech-to-text, integrate with Tauri, or send an intent to Jev. Those are later integration tests if this detector performs well.

## Browser/WSS test harness

The service is the detector's half: a supervised Rust worker, the WSS endpoint, and the fixture endpoint. The browser interface is the agent host's, at `/wake`. The default `bun run serve` command binds the API and the WSS endpoint to port **8787**. Build the worker, configure the service's own account, and start it from the repository root:

```sh
bun run build:worker
AGENT_WAKE_USERNAME=tester AGENT_WAKE_PASSWORD='use-a-test-password' bun run serve
```

The agent host reaches it over `AGENT_WAKE_WS_URL` and `AGENT_WAKE_URL`, which default to the local service. The two processes also need the same `AGENT_WAKE_TOKEN_SECRET`, at least 32 characters: the host signs a short-lived token for the signed-in user and the service verifies it, so the host's own page can open the socket without presenting its session cookie to another origin. Without the secret the feature is off and both sides say so.

The browser sends bounded mono `pcm_f32le` frames to the Rust worker. The service reports aggregate frame statistics and wake events without logging or persisting audio.

The checked-in `assets/fixtures/agent-wake.wav` is a synthetic, provenance-documented “Hey agent” WAV fixture. Run it from the wake lab to test the model independently of microphone permission, device selection, and network capture. The existing model-package WAVs are not positive Hey agent fixtures.

### Assigning an agent's wake word

The detector listens for pre-tokenized keywords, so a new wake word has to be spelled with the model's own SentencePiece vocabulary. One authenticated call does the whole job:

```sh
curl -X POST http://localhost:8787/api/keywords \
  -H 'content-type: application/json' -b cookies.txt \
  -d '{"name":"Henroy"}'
```

It spells the name with `scripts/tokenize-keywords.py`, appends the token line to `assets/keywords.txt`, restarts the detector worker so it reloads that file, and re-runs the WAV fixture as a regression check. The response carries the exact token line, whether the file changed, the full keyword list, and the fixture result. Re-assigning the same name is a no-op that still re-verifies.

Name assignment shells out to `python3` with the `sentencepiece` module, because a wake word spelled slightly differently is a wake word that never fires. Install it once on the host:

```sh
python3 -m pip install sentencepiece
```

Set `AGENT_WAKE_PYTHON` if `python3` is not the interpreter that has it. The host refuses the change with `409` while a microphone stream is running, because the reload drops the detector session.

What this cannot do is prove the name fires: it verifies the spelling and that the model still works, not that it hears that voice say that name. Record the name, drop the mono 16 kHz WAV in `assets/fixtures`, and add it to `server/fixtures.ts` to test it acoustically.

For a deployment check, verify `/healthz`, application login, unauthenticated WSS rejection, Origin enforcement, frame/session limits, fixture execution, microphone cleanup, and the no-audio-retention requirement. Do not expose the host without application credentials; the hostname alone is not authentication.

The application-login variables are intentionally required for a public deployment. For local evaluation, use a disposable password and keep it in the process environment. The session cookie becomes `Secure` when the request is HTTPS or carries `X-Forwarded-Proto: https`; do not treat plain HTTP on a remote hostname as a supported microphone deployment.

## Persistent Linux deployment

Copy `deploy/agent-wake.service` to `/etc/systemd/system/agent-wake.service`, create `/etc/agent-wake.env` from `deploy/agent-wake.env.example` with real credentials, then run:

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now agent-wake.service
sudo systemctl status agent-wake.service
```

The service binds dual-stack `::`:8787, so both `127.0.0.1:8787` and the Cloudflare Tunnel's `localhost:8787` origin can reach it. The tunnel itself is already configured for `lexia.dlx.studio → http://localhost:8787`; keep that route and do not point it at the public IP.

To create or replace the single host account without putting a password in shell history, run:

```sh
sudo ./scripts/provision-user.sh tester
```

The command prompts for the password, stores only an Argon2id hash in `/etc/agent-wake.env`, restarts the service, and never prints the password. For automation, pipe the password through stdin:

```sh
printf '%s' 'use-a-long-random-password' | sudo ./scripts/provision-user.sh --stdin tester
```

## API reference

One Bun process (`bun run serve`) serves the UI, the JSON API, and the audio WebSocket, and supervises the Rust detector worker as a child process. Everything below is same-origin: the browser, the API, and the worker all live on the same port.

| Method | Path | Auth | What it does |
| --- | --- | --- | --- |
| `GET` | `/healthz` | none | Detector readiness, for load balancers, tunnels and smoke tests |
| `GET` | `/api/session` | none | Who the cookie belongs to, and whether logins are configured at all |
| `POST` | `/api/login` | none | Exchange credentials for the session cookie |
| `POST` | `/api/logout` | none | Invalidate the session cookie |
| `POST` | `/api/fixture` | session | Run a bundled WAV through the detector without a microphone |
| `POST` | `/api/keywords` | session | Spell a new wake word, load it, re-verify the model |
| `GET` | `/ws/audio` | session + Origin | Stream microphone audio, receive wake events |
| `GET`/`HEAD` | `/agent-wake.zip` | none | Download the prebuilt project archive |
| `GET` | `/` and static paths | none | The Svelte test UI |

### Authentication

There is one application account, configured by environment variables. `POST /api/login` accepts JSON or form-encoded `username`/`password` and answers with the session cookie:

```sh
curl -i -X POST http://localhost:8787/api/login \
  -H 'content-type: application/json' \
  -d '{"username":"tester","password":"use-a-test-password"}'
```

```json
{ "authenticated": true, "user": { "name": "tester" } }
```

Set-Cookie is `agent_wake_session`, `HttpOnly`, `SameSite=Lax`, `Path=/`, valid for 8 hours, and marked `Secure` whenever the request is HTTPS or carries `X-Forwarded-Proto: https`. Sessions live in process memory, so restarting the host signs everyone out. When `AGENT_WAKE_PASSWORD_HASH` is set the password is verified as an Argon2id hash; otherwise `AGENT_WAKE_PASSWORD` is compared directly.

| Status | When |
| --- | --- |
| `401` | `invalid_credentials` - unknown user or wrong password |
| `503` | `auth_not_configured` - no username/password pair in the environment |
| `405` | Any method other than `POST` |

`GET /api/session` never fails; it reports state. `POST /api/logout` invalidates the token and clears the cookie (any method does this; the UI sends `POST`). Use `GET /api/session` on page load to decide whether to show the login form, and `/healthz` to wait for the model before pointing a browser at the page.

### `POST /api/fixture`

Runs a bundled WAV through the same detector core the microphone path uses, with no browser and no microphone involved. That separation is the point: if this fails, the problem is the model or the worker, not audio capture.

```sh
curl -X POST http://localhost:8787/api/fixture -b cookies.txt -H 'content-type: application/json' -d '{"fixtureId":"agent-wake"}'
```

```json
{ "ok": true, "fixtureId": "agent-wake",
  "result": { "detected": true, "keyword": "AGENT", "timestamp": 1791109713732, "durationMs": 1484,
              "stats": { "duration_ms": 1484, "frame_count": 2, "peak": 0.53, "rms": 0.07,
                         "sample_count": 23744, "sample_rate": 16000 } } }
```

The body is optional; an empty body runs the default fixture. Register more fixtures in `server/fixtures.ts` - only allowlisted paths are accepted, so the worker cannot be pointed at arbitrary files. `401` without a session, `503` when the model is not ready, `500` on `fixture_failed`.

### `POST /api/keywords`

Assigns a wake word to an agent in one call: spell the name with the model's SentencePiece vocabulary, append it to the keyword file, restart the detector so it reloads, and re-run the reference fixture as a regression check. See [Assigning an agent's wake word](#assigning-an-agents-wake-word) for the setup it needs.

```sh
curl -X POST http://localhost:8787/api/keywords -b cookies.txt \
  -H 'content-type: application/json' -d '{"name":"Henroy"}'
```

```json
{ "ok": true, "name": "HENROY", "tokens": "▁HE N RO Y", "added": true,
  "keywords": ["▁A G ENT", "▁HE N RO Y"],
  "verification": { "fixtureId": "agent-wake", "detected": true, "ok": true },
  "message": "HENROY is loaded. Speak \"HENROY\" into the microphone to confirm it fires." }
```

`tokens` is the exact line appended to `assets/keywords.txt`, `added` is `false` when that line was already present (re-assigning still re-verifies), and `keywords` is the whole list the detector will listen for. Names are uppercased and must be 1-64 characters of letters, digits, spaces, hyphens or apostrophes.

| Status | When |
| --- | --- |
| `401` | `unauthorized` - no session |
| `503` | `model_unavailable` - the detector is not ready to be reloaded |
| `409` | `stream_active` - a microphone stream is running; the reload would drop it |
| `400` | `invalid_body` - not JSON |
| `422` | `invalid_name` - unusable name, or this model cannot spell every sound in it |
| `500` | `reload_failed` / `fixture_missing` - the detector did not come back, or the reference WAV is unregistered |

This proves the spelling is real and that the model still works. It cannot prove the name fires - only speaking it into the microphone does that.

### `GET /ws/audio`

The microphone path. One browser streams mono float32 PCM to the host, which forwards it to the Rust detector and streams statistics and wake events back. The upgrade is refused unless a session cookie is present and the `Origin` is allowlisted (`403` `origin_forbidden`, `401` `unauthorized`).

Client to server, in order:

| Frame | Payload |
| --- | --- |
| Text | `{"type":"start","protocolVersion":1,"sampleRate":48000,"channels":1,"encoding":"pcm_f32le"}` |
| Binary | Bounded frames of 20-40 ms of mono `pcm_f32le` at the declared rate |
| Text | `{"type":"stop"}` |

Server to client:

| Type | Meaning |
| --- | --- |
| `status` | Connection opened, or the detector session opened/closed |
| `ready` | Protocol negotiated; carries `sampleRate`, `threshold`, `maxFrameBytes`, `maxQueueMs` |
| `stats` | Aggregates only: `framesReceived`, `bytesReceived`, `durationMs`, `queueDepth`, `rms`, `peak` |
| `wake` | A wake word fired, with `keyword`, `timestamp` and `threshold` |
| `stopped` | Final aggregates for the finished session |
| `error` | `{ code, message, retryable }` - always followed by a close |

Audio leaves the browser and reaches the host for inference. Nothing is written to disk, and no raw audio appears in any status message or log. Because the stream crosses Cloudflare when proxied, say so plainly to the tester; this is not device-local processing.

Limits the host enforces: 64 KiB per frame, 20-40 ms per frame, sample rate 8-192 kHz, a 10 s idle timeout, and one concurrent stream (`session_busy`). The detector worker holds a single session (`MAX_SESSIONS = 1`), so a dropped connection that is not cleaned up leaves the microphone unusable until the host restarts - which is why the host closes every session it opened.

```js
// In a browser the session cookie and Origin are sent automatically.
const socket = new WebSocket(`wss://${location.host}/ws/audio`);
socket.binaryType = 'arraybuffer';
socket.onopen = () => socket.send(JSON.stringify({ type: 'start', protocolVersion: 1, sampleRate: 48000, channels: 1, encoding: 'pcm_f32le' }));
socket.onmessage = (event) => {
  const message = JSON.parse(event.data);
  if (message.type === 'ready') worklet.port.postMessage({ start: true });
  if (message.type === 'wake') console.log('heard', message.keyword, message.timestamp);
};
```

Scripted clients run outside a browser need to carry the session themselves. Bun's WebSocket accepts headers, so pass the cookie and an allowlisted `Origin` explicitly:

```js
const socket = new WebSocket('ws://localhost:8787/ws/audio', { headers: { cookie, origin: 'http://localhost:8787' } });
```

Error codes: `unauthorized`, `origin_forbidden`, `upgrade_failed`, `invalid_state`, `invalid_control`, `invalid_frame`, `frame_too_large` (close 1009), `backpressure`, `session_busy` and `model_unavailable` (close 1013), `session_start_failed`, `idle_timeout`, `session_close_failed`. A clean stop closes with 1000.

### No browser interface

The service serves no HTML. The wake lab is a route in the agent host, at `/wake`, beside the other signed-in surfaces, and the host's session is what gates it. The host mints a short-lived token for the signed-in user and signs it; the service verifies it with the same secret and refuses anything unsigned. The token travels in the socket's query string and in the `Authorization` header of a fixture request.

The origin allowlist still guards the service's own session cookie, which is ambient and so presentable by any page the user visits. A token-carrying request is not gated on it: a cross-site page cannot mint a token, so refusing one on its origin would keep the host's own page out without closing anything.

### Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `AGENT_WAKE_USERNAME` | - | The single account name |
| `AGENT_WAKE_PASSWORD` | - | Password for local evaluation |
| `AGENT_WAKE_PASSWORD_HASH` | - | Argon2id hash, preferred over the plaintext password |
| `AGENT_WAKE_MODEL_DIR` | `models/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01` | Model directory |
| `AGENT_WAKE_KEYWORDS_FILE` | `assets/keywords.txt` | Pre-tokenized wake-word list |
| `AGENT_WAKE_WORKER_BIN` | `target/release/agent-wake-worker` | Detector worker executable |
| `AGENT_WAKE_THRESHOLD` | `0.25` | Detector threshold; higher rejects more, misses more |
| `AGENT_WAKE_PYTHON` | `python3` | Interpreter used to spell new wake words |
| `AGENT_WAKE_TOKEN_SECRET` | - | Secret the agent host signs its session token with; at least 32 characters, and the same value on both sides |
| `AGENT_HOST_URL` | `http://127.0.0.1:8787` | The agent host, which declares the phrases this service listens for |
| `ALLOWED_ORIGINS` | `https://lexia.dlx.studio`, `http://localhost:8787`, `http://127.0.0.1:8787` | Extra origins allowed to open the audio socket with this service's own session cookie |
| `HOST` / `PORT` | `::` / `8787` | Bind address |
| `PUBLIC_ORIGIN` | `https://lexia.dlx.studio` | Printed at startup only |

### Local CLI

The Rust binary runs the detector on its own, with no host, no browser and no server. Use it when you want to hear a wake word happen on this machine, or when the browser is not part of the question.

```sh
cargo run --release                          # live microphone, terminal dashboard
cargo run --release -- --wav path/to/mono.wav   # test a recording and exit
cargo run --release -- --threshold 0.35      # fewer false wakes, more missed ones
cargo run --release -- --log-interval 2      # status line every 2s instead of 5s
cargo run --release -- --no-live             # log lines only, no live dashboard
cargo run --release -- --threads 2 --model-dir /path/to/model
cargo run --release -- --keywords-file /path/to/keywords.txt
```

Flags: `--model-dir`, `--keywords-file`, `--wav`, `--threads`, `--threshold`, `--log-interval` (seconds, `0` disables), `--no-live`.

In a terminal it keeps one line current, so silence, signal and detection are distinguishable at a glance:

```
[11:26:53] model loaded, listening for: ▁A G ENT
[11:26:53] listening on <input device> at 44100 Hz (2 channel(s)). Press Ctrl+C to stop.
● listening 6.0s · 44100 Hz x2 · frames 140 · peak -9.6 dBFS · rms -27.5 dBFS · threshold 0.25 · last wake none
```

`●` is a green filled dot while the input carries signal and `○` when it is effectively silent, which is the difference between a quiet room and a capture path that is not working. A detection replaces the line for a few seconds:

```
★ WAKE WORD DETECTED: AGENT at 6.4s
```

and then the line settles into `last wake AGENT 1.2s ago` so you can tell a recent hit from an old one.

Every detection also prints the machine-readable record on stdout, so `cargo run --release | jq` still works, and a matching timestamped line on stderr:

```
WAKE {"start_time":6.35, "keyword": "AGENT", "timestamps": [6.35, 6.87, 7.07], "tokens":[" LE","X","IA"]}
[11:26:59] wake word detected: AGENT at 6.4s
```

Piped or run under systemd there is no live line, so the same numbers arrive on a timer as one `key=value` line per field, with no spaces inside values:

```
[status] up=6.1s rate=44100 ch=2 frames=142 samples=267159 peak=-9.6_dBFS rms=-27.6_dBFS threshold=0.25 keyword=- last_wake=none
```

`--wav` mode processes the file as fast as it can and exits, reporting what it heard and what the audio looked like:

```
$ cargo run --release -- --wav assets/fixtures/agent-wake.wav
[11:27:11] reading assets/fixtures/agent-wake.wav
WAKE {"start_time":0.00, "keyword": "AGENT", "timestamps": [0.00, 0.52, 0.72], "tokens":[" LE", "X", "IA"]}
[11:27:11] wake word detected: AGENT 0.00s into the clip
[11:27:11] finished: 1.48s, 2 frames, 23744 samples at 16000 Hz, peak -5.5 dBFS, wake word detected
```

With no match it prints `NO WAKE` and the same summary with `no wake word`, which is how you tell a quiet room from a broken microphone or a threshold that is too high.

### Troubleshooting

- **`Detector worker binary is missing`** - run `bun run build:worker`. The host only speaks its length-prefixed protocol to `agent-wake-worker`; the plain binary is a microphone CLI.
- **`sentencepiece is not installed`** on `POST /api/keywords` - install it for the interpreter named by `AGENT_WAKE_PYTHON`.
- **`origin_forbidden`** - the page was served from an origin outside the allowlist; add it to `ALLOWED_ORIGINS`.
- **`model_unavailable`** - the worker failed to load the model; check `AGENT_WAKE_MODEL_DIR` and that `models/` was populated by `scripts/prepare-model.sh`.
- **Everything is `401`** on a fresh deployment - no credentials were configured; see [the systemd section](#persistent-linux-deployment).

## Model licensing

The sherpa-onnx runtime is Apache-2.0. The selected model package identifies itself as Apache-2.0 as well; retain its notices and recheck the package terms before redistributing it with the agent platform. This prototype downloads the model locally rather than committing or bundling it.
