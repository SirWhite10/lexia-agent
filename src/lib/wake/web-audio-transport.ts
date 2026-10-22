import type {
  AudioStreamConfig,
  AudioTransport,
  ReadyEvent,
  ServerAudioEvent,
  TransportEvent
} from './types.js';

const MAX_FRAME_BYTES = 64 * 1024;
const MAX_QUEUE_MS = 250;
const FLUSH_INTERVAL_MS = 20;
const ALLOWED_EVENTS: Record<string, true> = {
  ready: true,
  status: true,
  stats: true,
  wake: true,
  error: true,
  stopped: true
};

interface QueuedFrame {
  data: ArrayBuffer;
  durationMs: number;
}


function parseAudioEvent(value: unknown): ServerAudioEvent | null {
  if (typeof value !== 'object' || value === null || !('type' in value) || typeof value.type !== 'string') return null;
  if (!Object.hasOwn(ALLOWED_EVENTS, value.type)) return null;

  if (value.type === 'ready') {
    return {
      type: 'ready',
      sampleRate: 'sampleRate' in value && typeof value.sampleRate === 'number' ? value.sampleRate : 0,
      encoding: 'encoding' in value && typeof value.encoding === 'string' ? value.encoding : ''
    };
  }
  if (value.type === 'status') {
    return {
      type: 'status',
      state: 'state' in value && typeof value.state === 'string' ? value.state : undefined,
      message: 'message' in value && typeof value.message === 'string' ? value.message : undefined,
      model: 'model' in value && typeof value.model === 'string' ? value.model : undefined,
      threshold: 'threshold' in value && typeof value.threshold === 'number' ? value.threshold : undefined,
      sampleRate: 'sampleRate' in value && typeof value.sampleRate === 'number' ? value.sampleRate : undefined
    };
  }
  if (value.type === 'stats') {
    return {
      type: 'stats',
      framesReceived: 'framesReceived' in value && typeof value.framesReceived === 'number' ? value.framesReceived : undefined,
      bytesReceived: 'bytesReceived' in value && typeof value.bytesReceived === 'number' ? value.bytesReceived : undefined,
      durationMs: 'durationMs' in value && typeof value.durationMs === 'number' ? value.durationMs : undefined,
      queueDepth: 'queueDepth' in value && typeof value.queueDepth === 'number' ? value.queueDepth : undefined,
      queueCapacity: 'queueCapacity' in value && typeof value.queueCapacity === 'number' ? value.queueCapacity : undefined,
      droppedFrames: 'droppedFrames' in value && typeof value.droppedFrames === 'number' ? value.droppedFrames : undefined,
      rms: 'rms' in value && typeof value.rms === 'number' ? value.rms : undefined,
      peak: 'peak' in value && typeof value.peak === 'number' ? value.peak : undefined
    };
  }
  if (value.type === 'wake') {
    return {
      type: 'wake',
      keyword: 'keyword' in value && typeof value.keyword === 'string' ? value.keyword : undefined,
      // The command the agent's phrase declares. The service sends null when
      // the phrase is not one the agent answers to, and that null is
      // information: a detection nobody mapped still has to be reported.
      command:
        'command' in value && (typeof value.command === 'string' || value.command === null)
          ? value.command
          : undefined,
      score: 'score' in value && typeof value.score === 'number' ? value.score : undefined,
      threshold: 'threshold' in value && typeof value.threshold === 'number' ? value.threshold : undefined,
      timestamp:
        'timestamp' in value && (typeof value.timestamp === 'string' || typeof value.timestamp === 'number')
          ? value.timestamp
          : undefined
    };
  }
  if (value.type === 'error') {
    return {
      type: 'error',
      code: 'code' in value && typeof value.code === 'string' ? value.code : 'network',
      message: 'message' in value && typeof value.message === 'string' ? value.message : undefined,
      retryable: 'retryable' in value && typeof value.retryable === 'boolean' ? value.retryable : undefined
    };
  }
  return {
    type: 'stopped',
    reason: 'reason' in value && typeof value.reason === 'string' ? value.reason : undefined
  };
}

export class WebSocketAudioTransport implements AudioTransport {
  /**
   * @param url The wake service's socket endpoint. It is not the page's own
   *   origin: the detector runs in a separate service, and the host only
   *   serves the page. A relative URL is resolved against the page's origin.
   */
  constructor(private readonly url: string) {}

  private socket: WebSocket | null = null;
  private config: AudioStreamConfig | null = null;
  private ready = false;
  private stopping = false;
  private stopSent = false;
  private flushTimer: ReturnType<typeof setTimeout> | null = null;
  private queue: QueuedFrame[] = [];
  private connectTimer: ReturnType<typeof setTimeout> | null = null;
  private negotiationTimer: ReturnType<typeof setTimeout> | null = null;
  private queuedDurationMs = 0;
  private readonly listeners = new Set<(event: TransportEvent) => void>();

  connect(): Promise<void> {
    if (this.socket) return Promise.reject(new Error('Audio transport is already connected.'));
    if (typeof WebSocket === 'undefined') return Promise.reject(new Error('This browser does not provide WebSocket.'));

    const scheme = location.protocol === 'https:' ? 'wss:' : 'ws:';
    const endpoint = this.url.startsWith('/') ? `${scheme}//${location.host}${this.url}` : this.url;
    const socket = new WebSocket(endpoint);
    socket.binaryType = 'arraybuffer';
    this.socket = socket;
    this.stopping = false;
    this.stopSent = false;

    const { promise, resolve, reject } = Promise.withResolvers<void>();
    let settled = false;
    this.connectTimer = setTimeout(() => {
      if (settled) return;
      settled = true;
      this.stopping = true;
      socket.close();
      this.resetSocket();
      reject(new Error('The secure audio connection timed out after 10 seconds.'));
    }, 10_000);
    socket.addEventListener('open', () => {
      settled = true;
      this.clearConnectTimer();
      resolve();
    });
    socket.addEventListener('error', () => {
      if (!settled) {
        this.stopping = true;
        this.clearConnectTimer();
        settled = true;
        reject(new Error('The secure audio connection could not be opened.'));
      }
    });
    socket.addEventListener('close', (event) => {
      this.clearConnectTimer();
      const wasStopping = this.stopping;
      this.resetSocket();
      if (!settled) {
        settled = true;
        reject(new Error(event.reason || 'The audio connection closed during setup.'));
      } else if (!wasStopping) {
        this.emit({ type: 'error', code: 'network', message: 'The host audio connection closed unexpectedly.', retryable: true });
      }
    });
    socket.addEventListener('message', (event) => this.handleMessage(event.data));
    return promise;
  }

  start(config: AudioStreamConfig): void {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) throw new Error('Audio transport is not open.');
    this.config = config;
    this.ready = false;
    this.clearQueue();
    this.socket.send(JSON.stringify(config));
    this.flushTimer = setInterval(() => this.flush(), FLUSH_INTERVAL_MS);
    this.negotiationTimer = setTimeout(() => {
      if (this.ready) return;
      this.emit({ type: 'error', code: 'network', message: 'The host did not complete audio negotiation within 10 seconds.', retryable: true });
    }, 10_000);
  }

  send(frame: ArrayBuffer): boolean {
    const socket = this.socket;
    if (!socket || socket.readyState !== WebSocket.OPEN || !this.ready || !this.config) {
      this.protocolFailure('Audio arrived before the host completed protocol negotiation.');
      return false;
    }
    if (frame.byteLength === 0 || frame.byteLength % Float32Array.BYTES_PER_ELEMENT !== 0 || frame.byteLength > MAX_FRAME_BYTES) {
      this.protocolFailure('The browser produced an invalid PCM frame size.');
      return false;
    }

    const durationMs = (frame.byteLength / Float32Array.BYTES_PER_ELEMENT / this.config.sampleRate) * 1000;
    this.queue.push({ data: frame, durationMs });
    this.queuedDurationMs += durationMs;
    if (this.queuedDurationMs > MAX_QUEUE_MS) {
      this.backpressureFailure(`The host send queue exceeded the ${MAX_QUEUE_MS} ms safety limit.`);
      return false;
    }
    this.flush();
    return true;
  }

  stop(): void {
    const socket = this.socket;
    this.stopping = true;
    this.ready = false;
    this.clearQueue();
    if (this.flushTimer !== null) {
      clearInterval(this.flushTimer);
      this.flushTimer = null;
    }
    if (this.negotiationTimer !== null) {
      clearTimeout(this.negotiationTimer);
      this.negotiationTimer = null;
    }
    if (socket?.readyState === WebSocket.OPEN && !this.stopSent) {
      this.stopSent = true;
      try {
        socket.send(JSON.stringify({ type: 'stop' }));
      } catch {
        // Closing below is the safe fallback when the connection is already failing.
      }
    }
    socket?.close(1000, 'client stop');
    this.resetSocket();
  }


  subscribe(listener: (event: TransportEvent) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  private handleMessage(data: unknown): void {
    if (typeof data !== 'string') {
      this.protocolFailure('The host sent an unexpected binary control message.');
      return;
    }
    let value: unknown;
    try {
      value = JSON.parse(data);
    } catch {
      this.protocolFailure('The host sent malformed JSON.');
      return;
    }
    const event = parseAudioEvent(value);
    if (!event) {
      this.protocolFailure('The host sent an invalid or unsupported audio event.');
      return;
    }
    if (event.type === 'ready' && !this.validateReady(event)) return;
    this.emit(event);
  }

  private validateReady(event: ReadyEvent): boolean {
    if (
      !Number.isInteger(event.sampleRate) ||
      event.sampleRate <= 0 ||
      event.encoding !== 'pcm_f32le' ||
      !this.config ||
      event.sampleRate !== this.config.sampleRate
    ) {
      this.protocolFailure('The host negotiated an incompatible sample rate or PCM encoding.');
      return false;
    }
    this.ready = true;
    if (this.negotiationTimer !== null) {
      clearTimeout(this.negotiationTimer);
      this.negotiationTimer = null;
    }
    return true;
  }

  private flush(): void {
    const socket = this.socket;
    if (!socket || socket.readyState !== WebSocket.OPEN || !this.ready) return;
    const maxQueuedBytes = (this.config?.sampleRate ?? 0) * Float32Array.BYTES_PER_ELEMENT * (MAX_QUEUE_MS / 1000);

    while (this.queue.length > 0) {
      const frame = this.queue[0];
      if (socket.bufferedAmount + frame.data.byteLength > maxQueuedBytes) return;
      this.queue.shift();
      this.queuedDurationMs = Math.max(0, this.queuedDurationMs - frame.durationMs);
      try {
        socket.send(frame.data);
      } catch {
        this.emit({ type: 'error', code: 'network', message: 'The browser could not write an audio frame to the host.', retryable: true });
        return;
      }
    }
  }

  private protocolFailure(message: string): void {
    this.ready = false;
    this.emit({ type: 'protocol-error', message });
  }

  private backpressureFailure(message: string): void {
    this.clearQueue();
    this.emit({ type: 'error', code: 'backpressure', message, retryable: false });
  }

  private clearQueue(): void {
    this.queue = [];
    this.queuedDurationMs = 0;
  }

  private resetSocket(): void {
    this.socket = null;
    this.config = null;
    this.ready = false;
    this.clearQueue();
    this.clearConnectTimer();
    if (this.negotiationTimer !== null) {
      clearTimeout(this.negotiationTimer);
      this.negotiationTimer = null;
    }
  }

  private clearConnectTimer(): void {
    if (this.connectTimer === null) return;
    clearTimeout(this.connectTimer);
    this.connectTimer = null;
  }

  private emit(event: TransportEvent): void {
    for (const listener of this.listeners) listener(event);
  }
}
