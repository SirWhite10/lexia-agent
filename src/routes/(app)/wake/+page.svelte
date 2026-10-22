<script lang="ts">
	import { onMount } from 'svelte';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Card from '#lib/components/ui/card/index.js';
	import { Label } from '#lib/components/ui/label/index.js';
	import { NativeSelect } from '#lib/components/ui/native-select/index.js';
	import { MicrophoneCapture } from '#lib/wake/microphone.js';
	import type { AudioTransport, CaptureState, StableFailureCode, TransportEvent } from '#lib/wake/types.js';
	import { WebSocketAudioTransport } from '#lib/wake/web-audio-transport.js';
	import { browserAudioFailure, networkError, safeErrorMessage } from '#lib/wake/errors.js';

	let { data }: { data: { wsUrl: string } } = $props();

	type EventTone = 'info' | 'success' | 'warning' | 'error';
	type DiagnosticEvent = { id: number; time: string; tone: EventTone; title: string; detail: string };
	type HostStats = {
		framesReceived: number;
		bytesReceived: number;
		durationMs: number;
		queueDepth: number | null;
		queueCapacity: number | null;
		droppedFrames: number;
		rms: number;
		peak: number;
	};
	type WakeResult = { keyword: string; command: string | null; at: number };

	const EMPTY_STATS: HostStats = {
		framesReceived: 0,
		bytesReceived: 0,
		durationMs: 0,
		queueDepth: null,
		queueCapacity: null,
		droppedFrames: 0,
		rms: 0,
		peak: 0
	};
	const STATE_COPY: Record<CaptureState, { label: string; detail: string }> = {
		idle: { label: 'Microphone idle', detail: 'Choose an input, then start capture explicitly.' },
		'requesting-permission': { label: 'Requesting microphone', detail: 'Waiting for browser permission.' },
		ready: { label: 'Microphone ready', detail: 'Permission is remembered. Start again whenever you are ready.' },
		connecting: { label: 'Negotiating audio', detail: 'Local frames are bounded while the service completes setup.' },
		streaming: { label: 'Listening', detail: 'Audio is streaming to the detector.' },
		stopping: { label: 'Stopping safely', detail: 'Closing the socket, worklet, media track and audio context.' },
		'error-permission': { label: 'Permission needed', detail: 'Allow microphone access in the browser site settings.' },
		'error-device': { label: 'Input unavailable', detail: 'Refresh inputs or choose a different microphone.' },
		'error-network': { label: 'Connection failed', detail: 'The wake service could not be reached.' },
		'error-protocol': { label: 'Protocol mismatch', detail: 'The service did not negotiate the expected mono f32 stream.' },
		'error-backpressure': { label: 'Send queue full', detail: 'Capture stopped before audio could buffer without bound.' },
		'error-model': { label: 'Detector unavailable', detail: 'The service model or detector worker did not become ready.' },
		'error-idle': { label: 'Stream idle', detail: 'The service closed the stream because frames stopped arriving.' }
	};
	const NUMBER_FORMAT = new Intl.NumberFormat();
	const TIME_FORMAT = new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' });

	let captureState = $state<CaptureState>('idle');
	let devices = $state<MediaDeviceInfo[]>([]);
	let selectedDeviceId = $state('');
	let selectedDeviceLabel = $state('Browser default microphone');
	let captureSampleRate = $state<number | null>(null);
	let transportReady = $state(false);
	let localFrames = $state(0);
	let localBytes = $state(0);
	let localRms = $state(0);
	let localPeak = $state(0);
	let localOverflows = $state(0);
	let hostStats = $state<HostStats>({ ...EMPTY_STATS });
	let detectorMessage = $state<string | null>(null);
	let detectorModel = $state<string | null>(null);
	let detectorThreshold = $state<number | null>(null);
	let lastWake = $state<WakeResult | null>(null);
	let lastError = $state<{ code: StableFailureCode; message: string } | null>(null);
	let diagnostics = $state<DiagnosticEvent[]>([]);
	let fixtureRunning = $state(false);
	let fixtureMessage = $state<string | null>(null);

	let capture: MicrophoneCapture | null = null;
	let transport: AudioTransport | null = null;
	let unsubscribeTransport: (() => void) | null = null;
	let captureGeneration = 0;
	let eventSequence = 0;
	let disposed = false;

	let captureActive = $derived(
		captureState === 'requesting-permission' ||
			captureState === 'connecting' ||
			captureState === 'streaming' ||
			captureState === 'stopping'
	);
	let captureBusy = $derived(
		captureState === 'requesting-permission' || captureState === 'connecting' || captureState === 'stopping'
	);
	let currentState = $derived(STATE_COPY[captureState]);
	let meterPercent = $derived(Math.min(100, Math.round(localRms * 400)));

	onMount(() => {
		const handleDeviceChange = () => void refreshDevices();
		const handlePageHide = () => void terminateCapture();
		navigator.mediaDevices?.addEventListener('devicechange', handleDeviceChange);
		window.addEventListener('pagehide', handlePageHide);
		void refreshDevices();
		return () => {
			disposed = true;
			navigator.mediaDevices?.removeEventListener('devicechange', handleDeviceChange);
			window.removeEventListener('pagehide', handlePageHide);
			void terminateCapture();
		};
	});

	/**
	 * A token the wake service accepts, minted by this host for the signed-in
	 * user. The host's session cookie is scoped to this origin, so it never
	 * reaches the service; the token is how the two speak.
	 */
	async function mintToken(): Promise<string> {
		const response = await fetch('/api/wake/token', { method: 'POST' });
		if (!response.ok) {
			const body = await response.json().catch(() => ({ message: response.statusText }));
			throw new Error(body.message ?? 'The wake service is not configured on this host.');
		}
		const body = (await response.json()) as { token: string };
		return body.token;
	}

	async function refreshDevices(): Promise<void> {
		if (!navigator.mediaDevices?.enumerateDevices) return;
		const available = await navigator.mediaDevices.enumerateDevices();
		const inputs = available.filter((device) => device.kind === 'audioinput');
		devices = inputs;
		if (selectedDeviceId && !inputs.some((device) => device.deviceId === selectedDeviceId)) {
			selectedDeviceId = '';
			selectedDeviceLabel = 'Browser default microphone';
		}
	}

	function addEvent(tone: EventTone, title: string, detail: string): void {
		const entry: DiagnosticEvent = {
			id: ++eventSequence,
			time: TIME_FORMAT.format(new Date()),
			tone,
			title,
			detail
		};
		diagnostics = [entry, ...diagnostics].slice(0, 60);
	}

	async function failCapture(code: StableFailureCode, message: string): Promise<void> {
		lastError = { code, message };
		captureState = `error-${code}` as CaptureState;
		addEvent('error', 'Capture failed', message);
		await terminateCapture();
	}

	async function terminateCapture(): Promise<void> {
		captureGeneration += 1;
		transportReady = false;
		unsubscribeTransport?.();
		unsubscribeTransport = null;
		transport?.stop();
		transport = null;
		await capture?.stop();
		capture = null;
		if (!disposed && captureState !== 'idle') captureState = 'idle';
	}

	async function startMicrophone(): Promise<void> {
		if (captureBusy || captureActive) return;
		const generation = ++captureGeneration;
		captureState = 'requesting-permission';
		lastError = null;
		localFrames = 0;
		localBytes = 0;
		localRms = 0;
		localPeak = 0;
		localOverflows = 0;
		captureSampleRate = null;
		hostStats = { ...EMPTY_STATS };
		detectorMessage = 'Waiting for the service';
		detectorModel = null;
		detectorThreshold = null;
		lastWake = null;

		const nextCapture = new MicrophoneCapture({
			onFrame: (frame) => {
				localFrames += 1;
				localBytes += frame.buffer.byteLength;
				localRms = frame.rms;
				localPeak = frame.peak;
				if (transportReady && transport) transport.send(frame.buffer);
			},
			onOverflow: () => {
				localOverflows += 1;
				void failCapture(
					'backpressure',
					'The browser capture callback fell behind. Audio buffering was stopped at the safety limit.'
				);
			},
			onEnded: () => {
				if (captureActive) void failCapture('device', 'The browser ended the selected microphone track.');
			}
		});
		capture = nextCapture;

		try {
			const info = await nextCapture.start(selectedDeviceId);
			if (generation !== captureGeneration || capture !== nextCapture) {
				await nextCapture.stop();
				return;
			}
			selectedDeviceLabel = info.label;
			if (info.deviceId) selectedDeviceId = info.deviceId;
			captureSampleRate = info.sampleRate;
			await refreshDevices();

			const token = await mintToken();
			if (generation !== captureGeneration || capture !== nextCapture) return;
			const separator = data.wsUrl.includes('?') ? '&' : '?';
			const nextTransport = new WebSocketAudioTransport(`${data.wsUrl}${separator}token=${encodeURIComponent(token)}`);
			transport = nextTransport;
			unsubscribeTransport = nextTransport.subscribe(handleTransportEvent);
			captureState = 'connecting';
			await nextTransport.connect();
			nextTransport.start({
				type: 'start',
				protocolVersion: 1,
				sampleRate: info.sampleRate,
				channels: 1,
				encoding: 'pcm_f32le'
			});
			addEvent('info', 'Connected', 'The socket is open; waiting for the detector ready event.');
		} catch (error) {
			if (generation !== captureGeneration || capture !== nextCapture) return;
			if (disposed) return;
			const mapped = isPermissionFailure(error)
				? browserAudioFailure(error)
				: { code: 'network' as const, message: networkError(error, 'The audio connection could not start.') };
			await failCapture(mapped.code, mapped.message);
		}
	}

	/** The microphone and the socket are different failures, and the user needs
	 * to be told which one happened. */
	function isPermissionFailure(error: unknown): boolean {
		return error instanceof DOMException && error.name === 'NotAllowedError';
	}

	async function stopMicrophone(): Promise<void> {
		if (!captureActive) return;
		captureState = 'stopping';
		await terminateCapture();
		addEvent('info', 'Microphone stopped', 'The media track, worklet context and socket were closed.');
	}

	function handleTransportEvent(event: TransportEvent): void {
		if (disposed) return;
		if (event.type === 'protocol-error') {
			void failCapture('protocol', event.message);
			return;
		}
		if (event.type === 'ready') {
			transportReady = true;
			captureState = 'streaming';
			detectorMessage = 'Listening';
			addEvent('success', 'Detector ready', 'The service accepted the stream and is listening.');
			return;
		}
		if (event.type === 'status') {
			detectorMessage = event.message ?? event.state ?? detectorMessage;
			if (event.model) detectorModel = event.model;
			if (event.threshold !== undefined) detectorThreshold = event.threshold;
			return;
		}
		if (event.type === 'stats') {
			hostStats = {
				framesReceived: event.framesReceived ?? 0,
				bytesReceived: event.bytesReceived ?? 0,
				durationMs: event.durationMs ?? 0,
				queueDepth: event.queueDepth ?? null,
				queueCapacity: event.queueCapacity ?? null,
				droppedFrames: event.droppedFrames ?? 0,
				rms: event.rms ?? 0,
				peak: event.peak ?? 0
			};
			return;
		}
		if (event.type === 'wake') {
			const keyword = event.keyword ?? 'wake word';
			const command = event.command ?? null;
			lastWake = { keyword, command, at: Date.now() };
			addEvent(
				'success',
				`Woke on "${keyword}"`,
				command ? `Ran ${command}.` : 'The agent does not answer to that phrase, so nothing ran.'
			);
			return;
		}
		if (event.type === 'error') {
			void failCapture('model', event.message ?? 'The detector reported an error.');
			return;
		}
		if (event.type === 'stopped') {
			if (captureActive) void failCapture('idle', 'The service closed the stream.');
		}
	}

	/**
	 * Run the bundled WAV through the detector. It proves the model and the
	 * worker are alive without opening the microphone, which is the difference
	 * between a broken model and a quiet room.
	 */
	async function runFixture(): Promise<void> {
		if (fixtureRunning) return;
		fixtureRunning = true;
		fixtureMessage = null;
		try {
			// The host proxies this to the service: the service is on another
			// origin, and a request carrying an Authorization header is
			// preflighted, which the service does not answer.
			const response = await fetch('/api/wake/fixture', { method: 'POST' });
			const body = await response.json().catch(() => ({}));
			if (!response.ok) {
				const message = body.message ?? `The service answered ${response.status}.`;
				fixtureMessage = message;
				addEvent('warning', 'Fixture failed', message);
				return;
			}
			const result = (body.result ?? {}) as {
				detected?: boolean;
				keyword?: string | null;
				durationMs?: number | null;
			};
			const keyword = result.keyword ?? 'wake word';
			const duration = result.durationMs;
			fixtureMessage = result.detected
				? `Detected "${keyword}"${duration === null || duration === undefined ? '.' : ` in ${formatDuration(duration)}.`}`
				: 'The fixture did not fire. The model is loaded but did not match.';
			addEvent(result.detected ? 'success' : 'warning', 'Fixture finished', fixtureMessage);
		} catch (error) {
			fixtureMessage = safeErrorMessage(error, 'The wake service could not be reached.');
			addEvent('warning', 'Fixture failed', fixtureMessage);
		} finally {
			fixtureRunning = false;
		}
	}

	function formatBytes(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
	}

	function formatDuration(milliseconds: number): string {
		return `${(milliseconds / 1000).toFixed(1)} s`;
	}
</script>

<svelte:head>
	<title>Wake word — Agent</title>
	<meta name="description" content="Microphone capture and wake detection for this agent." />
</svelte:head>

<section class="mx-auto w-full max-w-3xl flex-1 space-y-4 px-5 py-6" aria-label="Wake word">
	<p class="mb-5 text-sm text-muted-foreground">
		Listen for this agent's wake phrase. The detector runs in a separate service; this page is the
		control surface for it.
	</p>

	<Card.Root>
		<Card.Header>
			<Card.Title>Microphone</Card.Title>
			<Card.Description>Capture is explicit. Nothing is recorded, and audio is not stored.</Card.Description>
		</Card.Header>
		<Card.Content class="space-y-4">
			<div class="space-y-2">
				<Label for="wake-device">Input</Label>
				<NativeSelect id="wake-device" bind:value={selectedDeviceId} disabled={captureActive} class="w-full">
					<option value="">Browser default microphone</option>
					{#each devices as device (device.deviceId)}
						<option value={device.deviceId}>{device.label || `Microphone ${device.deviceId.slice(0, 6)}`}</option>
					{/each}
				</NativeSelect>
			</div>

			<div class="flex flex-wrap items-center gap-2">
				<Button onclick={() => void startMicrophone()} disabled={captureBusy || captureActive}>
					Start capture
				</Button>
				<Button variant="outline" onclick={() => void stopMicrophone()} disabled={!captureActive}>
					Stop
				</Button>
				<Badge variant={captureState === 'streaming' ? 'default' : 'secondary'}>{currentState.label}</Badge>
			</div>

			<p class="text-sm text-muted-foreground">{currentState.detail}</p>

			<div class="space-y-1">
				<div class="flex items-center justify-between text-xs text-muted-foreground">
					<span>Input level</span>
					<span>{meterPercent}%</span>
				</div>
				<div class="h-2 w-full overflow-hidden rounded-full bg-muted" role="presentation">
					<div class="h-full rounded-full bg-primary transition-[width]" style={`width: ${meterPercent}%`}></div>
				</div>
			</div>

			<dl class="grid grid-cols-2 gap-x-4 gap-y-1 text-sm sm:grid-cols-3">
				<dt class="text-muted-foreground">Device</dt>
				<dd class="truncate">{selectedDeviceLabel}</dd>
				<dt class="text-muted-foreground">Sample rate</dt>
				<dd>{captureSampleRate ? `${captureSampleRate} Hz` : '—'}</dd>
				<dt class="text-muted-foreground">Local frames</dt>
				<dd>{NUMBER_FORMAT.format(localFrames)}</dd>
				<dt class="text-muted-foreground">Local bytes</dt>
				<dd>{formatBytes(localBytes)}</dd>
				<dt class="text-muted-foreground">Overflows</dt>
				<dd>{localOverflows}</dd>
			</dl>

			{#if lastError}
				<p class="text-sm text-destructive" role="alert">{lastError.message}</p>
			{/if}
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>Detector</Card.Title>
			<Card.Description>What the service reports about itself and the stream.</Card.Description>
		</Card.Header>
		<Card.Content>
			<dl class="grid grid-cols-2 gap-x-4 gap-y-1 text-sm sm:grid-cols-3">
				<dt class="text-muted-foreground">State</dt>
				<dd>{detectorMessage ?? 'Not connected'}</dd>
				<dt class="text-muted-foreground">Model</dt>
				<dd class="truncate">{detectorModel ?? '—'}</dd>
				<dt class="text-muted-foreground">Threshold</dt>
				<dd>{detectorThreshold ?? '—'}</dd>
				<dt class="text-muted-foreground">Frames received</dt>
				<dd>{NUMBER_FORMAT.format(hostStats.framesReceived)}</dd>
				<dt class="text-muted-foreground">Bytes received</dt>
				<dd>{formatBytes(hostStats.bytesReceived)}</dd>
				<dt class="text-muted-foreground">Dropped</dt>
				<dd>{hostStats.droppedFrames}</dd>
			</dl>
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>Last wake</Card.Title>
			<Card.Description>The phrase that fired, and the command it ran.</Card.Description>
		</Card.Header>
		<Card.Content>
			{#if lastWake}
				<dl class="grid grid-cols-2 gap-x-4 gap-y-1 text-sm">
					<dt class="text-muted-foreground">Phrase</dt>
					<dd>{lastWake.keyword}</dd>
					<dt class="text-muted-foreground">Command</dt>
					<dd>{lastWake.command ?? 'None — the agent does not answer to that phrase'}</dd>
					<dt class="text-muted-foreground">At</dt>
					<dd>{TIME_FORMAT.format(new Date(lastWake.at))}</dd>
				</dl>
			{:else}
				<p class="text-sm text-muted-foreground">Nothing has woken it yet.</p>
			{/if}
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>Fixture</Card.Title>
			<Card.Description>
				Run the bundled WAV through the detector. It separates a broken model from a quiet room.
			</Card.Description>
		</Card.Header>
		<Card.Content class="space-y-3">
			<Button variant="outline" onclick={() => void runFixture()} disabled={fixtureRunning}>
				{fixtureRunning ? 'Running…' : 'Run the bundled WAV'}
			</Button>
			{#if fixtureMessage}
				<p class="text-sm text-muted-foreground">{fixtureMessage}</p>
			{/if}
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>Diagnostics</Card.Title>
			<Card.Description>What happened, newest first.</Card.Description>
		</Card.Header>
		<Card.Content>
			{#if diagnostics.length === 0}
				<p class="text-sm text-muted-foreground">Nothing has happened yet.</p>
			{:else}
				<ul class="space-y-2 text-sm">
					{#each diagnostics as entry (entry.id)}
						<li class="flex flex-wrap items-baseline gap-x-2">
							<span class="font-mono text-xs text-muted-foreground">{entry.time}</span>
							<span class={entry.tone === 'error' ? 'text-destructive' : entry.tone === 'success' ? 'text-foreground' : 'text-muted-foreground'}>
								{entry.title}
							</span>
							<span class="text-muted-foreground">{entry.detail}</span>
						</li>
					{/each}
				</ul>
			{/if}
		</Card.Content>
	</Card.Root>
</section>
