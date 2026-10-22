// `?worker&url` bundles the processor through Vite so the TypeScript is transpiled and
// emitted as a real .js asset. A bare `?url` import ships the raw .ts source under a
// `data:video/mp2t` MIME type, which audioWorklet.addModule() always rejects.
import workletUrl from './capture-worklet.ts?worker&url';
import type { CaptureFrame, CaptureInfo } from './types.js';

interface CaptureCallbacks {
  onFrame(frame: CaptureFrame): void;
  onOverflow(): void;
  onEnded(): void;
}

type BrowserWindow = Window &
  typeof globalThis & {
    webkitAudioContext?: typeof AudioContext;
  };

interface WorkletMessage {
  type: 'frame' | 'overflow';
  buffer?: ArrayBuffer;
  rms?: number;
  peak?: number;
  durationMs?: number;
}

export class MicrophoneCapture {
  private stream: MediaStream | null = null;
  private context: AudioContext | null = null;
  private source: MediaStreamAudioSourceNode | null = null;
  private node: AudioWorkletNode | null = null;
  private track: MediaStreamTrack | null = null;

  constructor(private readonly callbacks: CaptureCallbacks) {}

  async start(requestedDeviceId: string): Promise<CaptureInfo> {
    if (this.stream) throw new Error('Microphone capture is already active.');
    if (!window.isSecureContext) {
      throw new DOMException('Microphone capture requires HTTPS or localhost.', 'NotAllowedError');
    }
    if (!navigator.mediaDevices?.getUserMedia || !navigator.mediaDevices.enumerateDevices) {
      throw new DOMException('Microphone capture is unavailable in this browser context.', 'NotSupportedError');
    }

    const browserWindow: BrowserWindow = window;
    const AudioContextConstructor = browserWindow.AudioContext ?? browserWindow.webkitAudioContext;
    if (!AudioContextConstructor) throw new DOMException('Web Audio is unavailable in this browser.', 'NotSupportedError');

    const audio: MediaTrackConstraints = {
      channelCount: 1,
      echoCancellation: false,
      noiseSuppression: false,
      autoGainControl: false,
      ...(requestedDeviceId ? { deviceId: { exact: requestedDeviceId } } : {})
    };

    this.stream = await navigator.mediaDevices.getUserMedia({ audio, video: false });
    const track = this.stream.getAudioTracks()[0];
    if (!track) {
      await this.stop();
      throw new DOMException('The selected stream did not contain an audio track.', 'NotFoundError');
    }
    this.track = track;

    track.addEventListener('ended', this.handleEnded);
    try {
      this.context = new AudioContextConstructor({ latencyHint: 'interactive' });
      if (!this.context.audioWorklet) throw new DOMException('AudioWorklet is unavailable.', 'NotSupportedError');
      await this.context.audioWorklet.addModule(workletUrl);
      this.source = this.context.createMediaStreamSource(this.stream);
      this.node = new AudioWorkletNode(this.context, 'agent-wake-capture', {
        numberOfInputs: 1,
        numberOfOutputs: 1,
        outputChannelCount: [1],
        processorOptions: { frameSamples: Math.round(this.context.sampleRate * 0.02) }
      });
      this.node.port.onmessage = (event: MessageEvent<WorkletMessage>) => this.handleWorkletMessage(event.data);
      this.source.connect(this.node);
      this.node.connect(this.context.destination);
      await this.context.resume();

      const settings = track.getSettings();
      return {
        sampleRate: this.context.sampleRate,
        deviceId: settings.deviceId ?? requestedDeviceId,
        label: track.label || 'Default microphone'
      };
    } catch (error) {
      await this.stop();
      throw error;
    }
  }

  async stop(): Promise<void> {
    this.node?.port.postMessage({ type: 'stop' });
    if (this.node) this.node.port.onmessage = null;
    this.source?.disconnect();
    this.node?.disconnect();
    if (this.track) this.track.removeEventListener('ended', this.handleEnded);
    this.track?.stop();

    const context = this.context;
    this.node = null;
    this.source = null;
    this.track = null;
    this.context = null;
    this.stream = null;
    if (context && context.state !== 'closed') {
      try {
        await context.close();
      } catch {
        // The browser may close the context during page shutdown.
      }
    }
  }

  private handleWorkletMessage(message: WorkletMessage): void {
    if (message.type === 'overflow') {
      this.callbacks.onOverflow();
      return;
    }
    if (message.type !== 'frame' || !message.buffer) return;
    const buffer = message.buffer;
    const rms = typeof message.rms === 'number' && Number.isFinite(message.rms) ? message.rms : 0;
    const peak = typeof message.peak === 'number' && Number.isFinite(message.peak) ? message.peak : 0;
    const durationMs = typeof message.durationMs === 'number' && Number.isFinite(message.durationMs) ? message.durationMs : 20;
    try {
      this.callbacks.onFrame({ buffer, rms, peak, durationMs });
    } finally {
      this.node?.port.postMessage({ type: 'return', buffer }, [buffer]);
    }
  }

  private readonly handleEnded = (): void => {
    this.callbacks.onEnded();
  };
}
