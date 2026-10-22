/// <reference lib="webworker" />
declare class AudioWorkletProcessor {
  readonly port: MessagePort;
  constructor(options?: AudioWorkletNodeOptions);
  process(inputs: Float32Array[][], outputs: Float32Array[][]): boolean;
}

declare function registerProcessor(name: string, processor: new (options?: AudioWorkletNodeOptions) => AudioWorkletProcessor): void;
declare const sampleRate: number;

class CaptureProcessor extends AudioWorkletProcessor {
  private readonly frameSamples: number;
  private readonly frameDurationMs: number;
  private readonly monoFrame: Float32Array<ArrayBuffer>;
  private readonly freeBuffers: Float32Array<ArrayBuffer>[] = [];
  private frameOffset = 0;
  private sumSquares = 0;
  private framePeak = 0;
  private overflowed = false;
  private active = true;

  constructor(options?: AudioWorkletNodeOptions) {
    super(options);
    const rawOptions: unknown = options?.processorOptions;
    const requested =
      typeof rawOptions === 'object' &&
      rawOptions !== null &&
      'frameSamples' in rawOptions &&
      typeof rawOptions.frameSamples === 'number'
        ? rawOptions.frameSamples
        : undefined;
    this.frameSamples = Math.max(1, Math.round(requested ?? sampleRate * 0.02));
    this.frameDurationMs = (this.frameSamples / sampleRate) * 1000;
    this.monoFrame = new Float32Array(this.frameSamples);
    for (let index = 0; index < 3; index += 1) this.freeBuffers.push(new Float32Array(this.frameSamples));
    this.port.onmessage = (event: MessageEvent) => {
      const message: unknown = event.data;
      if (typeof message !== 'object' || message === null || !('type' in message)) return;
      if (message.type === 'stop') this.active = false;
      if (message.type === 'return' && 'buffer' in message && message.buffer instanceof ArrayBuffer && this.freeBuffers.length < 3) {
        this.freeBuffers.push(new Float32Array(message.buffer));
      }
    };
  }

  private pushFrame(): void {
    const output = this.freeBuffers.pop();
    if (!output) {
      if (!this.overflowed) {
        this.overflowed = true;
        this.port.postMessage({ type: 'overflow', durationMs: this.frameDurationMs });
      }
      this.resetFrame();
      return;
    }

    output.set(this.monoFrame);
    const rms = Math.sqrt(this.sumSquares / this.frameSamples);
    const peak = this.framePeak;
    this.port.postMessage(
      { type: 'frame', buffer: output.buffer, rms, peak, durationMs: this.frameDurationMs },
      [output.buffer]
    );
    this.resetFrame();
  }

  private resetFrame(): void {
    this.frameOffset = 0;
    this.sumSquares = 0;
    this.framePeak = 0;
    this.monoFrame.fill(0);
  }

  override process(inputs: Float32Array[][], outputs: Float32Array[][]): boolean {
    for (const output of outputs) {
      for (const channel of output) channel.fill(0);
    }
    if (!this.active) return false;

    const input = inputs[0];
    if (!input || input.length === 0 || input[0].length === 0) return true;

    const first = input[0];
    const channelCount = input.length;
    for (let sampleIndex = 0; sampleIndex < first.length; sampleIndex += 1) {
      let mono = 0;
      for (let channelIndex = 0; channelIndex < channelCount; channelIndex += 1) mono += input[channelIndex][sampleIndex];
      mono /= channelCount;
      if (!Number.isFinite(mono)) mono = 0;
      this.monoFrame[this.frameOffset] = mono;
      this.sumSquares += mono * mono;
      const absolute = Math.abs(mono);
      if (absolute > this.framePeak) this.framePeak = absolute;
      this.frameOffset += 1;
      if (this.frameOffset === this.frameSamples) this.pushFrame();
    }
    return true;
  }
}

registerProcessor('agent-wake-capture', CaptureProcessor);
