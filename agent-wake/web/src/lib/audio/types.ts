export const STABLE_FAILURE_CODES = [
  'permission',
  'device',
  'network',
  'protocol',
  'backpressure',
  'model',
  'idle'
] as const;

export type StableFailureCode = (typeof STABLE_FAILURE_CODES)[number];

export type CaptureState =
  | 'idle'
  | 'requesting-permission'
  | 'ready'
  | 'connecting'
  | 'streaming'
  | 'stopping'
  | 'error-permission'
  | 'error-device'
  | 'error-network'
  | 'error-protocol'
  | 'error-backpressure'
  | 'error-model'
  | 'error-idle';

export interface AudioStreamConfig {
  type: 'start';
  protocolVersion: 1;
  sampleRate: number;
  channels: 1;
  encoding: 'pcm_f32le';
}

export interface ReadyEvent {
  type: 'ready';
  sampleRate: number;
  encoding: string;
  frame?: { minMs?: number; maxMs?: number; maxBytes?: number };
  queue?: { maxFrames?: number; maxBytes?: number };
}

export interface StatusEvent {
  type: 'status';
  state?: string;
  message?: string;
  model?: string;
  threshold?: number;
  sampleRate?: number;
}

export interface StatsEvent {
  type: 'stats';
  framesReceived?: number;
  bytesReceived?: number;
  durationMs?: number;
  queueDepth?: number;
  queueCapacity?: number;
  droppedFrames?: number;
  rms?: number;
  peak?: number;
}

export interface WakeEvent {
  type: 'wake';
  keyword?: string;
  score?: number;
  threshold?: number;
  timestamp?: string | number;
}

export interface ServerErrorEvent {
  type: 'error';
  code: string;
  message?: string;
  retryable?: boolean;
}

export interface StoppedEvent {
  type: 'stopped';
  reason?: string;
  stats?: StatsEvent;
}

export type ServerAudioEvent =
  | ReadyEvent
  | StatusEvent
  | StatsEvent
  | WakeEvent
  | ServerErrorEvent
  | StoppedEvent;

export type TransportEvent = ServerAudioEvent | { type: 'protocol-error'; message: string };

export interface AudioTransport {
  connect(): Promise<void>;
  start(config: AudioStreamConfig): void;
  send(frame: ArrayBuffer): boolean;
  stop(): void;
  subscribe(listener: (event: TransportEvent) => void): () => void;
}

export interface CaptureInfo {
  sampleRate: number;
  deviceId: string;
  label: string;
}

export interface CaptureFrame {
  buffer: ArrayBuffer;
  rms: number;
  peak: number;
  durationMs: number;
}
