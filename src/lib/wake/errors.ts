import type { StableFailureCode } from './types.js';

const labels: Record<StableFailureCode, string> = {
  permission: 'Microphone permission',
  device: 'Input device',
  network: 'Network',
  protocol: 'Audio protocol',
  backpressure: 'Backpressure',
  model: 'Detector model',
  idle: 'Idle timeout'
};

export function stableFailureCode(value: string): StableFailureCode {
  const normalized = value.toLowerCase();
  if (normalized.includes('permission') || normalized.includes('notallowed')) return 'permission';
  if (normalized.includes('device') || normalized.includes('notfound') || normalized.includes('audioinput')) return 'device';
  if (normalized.includes('backpressure') || normalized.includes('queue')) return 'backpressure';
  if (normalized.includes('model') || normalized.includes('worker')) return 'model';
  if (normalized.includes('idle') || normalized.includes('timeout')) return 'idle';
  if (normalized.includes('protocol') || normalized.includes('encoding') || normalized.includes('format')) return 'protocol';
  return 'network';
}

export function failureLabel(code: StableFailureCode): string {
  return labels[code];
}

export function browserAudioFailure(error: unknown): { code: StableFailureCode; message: string } {
  if (error instanceof DOMException) {
    if (error.name === 'NotAllowedError' || error.name === 'SecurityError') {
      return { code: 'permission', message: 'Microphone access was denied. Allow it in the browser site settings, then try again.' };
    }
    if (error.name === 'NotFoundError' || error.name === 'DevicesNotFoundError') {
      return { code: 'device', message: 'No microphone input was found. Connect an input, then refresh the device list.' };
    }
    if (error.name === 'NotReadableError' || error.name === 'TrackStartError') {
      return { code: 'device', message: 'The selected microphone could not be opened. It may be in use by another application.' };
    }
    if (error.name === 'OverconstrainedError' || error.name === 'ConstraintNotSatisfiedError') {
      return { code: 'device', message: 'The selected microphone no longer supports the requested mono capture settings.' };
    }
    if (error.name === 'AbortError') {
      return { code: 'device', message: 'The browser stopped the microphone request before it completed.' };
    }
  }
  return { code: 'device', message: 'The browser could not start microphone capture. Check the input and permissions, then try again.' };
}

export function safeErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message) return error.message.slice(0, 300);
  if (typeof error === 'string' && error.trim()) return error.trim().slice(0, 300);
  return fallback;
}

export function networkError(error: unknown, action: string): string {
  return `${action} Could not reach the Bun host. Check the network and try again. ${safeErrorMessage(error, '')}`.trim();
}
