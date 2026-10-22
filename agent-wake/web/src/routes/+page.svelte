<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchSession, login, logout, type SessionInfo } from '$lib/auth';
  import { MicrophoneCapture } from '$lib/audio/microphone';
  import type {
    CaptureState,
    AudioTransport,
    StableFailureCode,
    TransportEvent
  } from '$lib/audio/types';
  import { WebSocketAudioTransport } from '$lib/audio/web-audio-transport';
  import {
    browserAudioFailure,
    failureLabel,
    networkError,
    safeErrorMessage,
    stableFailureCode
  } from '$lib/errors';

  type SessionView = 'loading' | 'authenticated' | 'anonymous' | 'unavailable';
  type EventTone = 'info' | 'success' | 'warning' | 'error';
  type FixtureStatus = 'idle' | 'running' | 'passed' | 'clear' | 'error';

  interface DiagnosticEvent {
    id: number;
    time: string;
    iso: string;
    tone: EventTone;
    title: string;
    detail: string;
  }

  interface HostStats {
    framesReceived: number;
    bytesReceived: number;
    durationMs: number;
    queueDepth: number | null;
    queueCapacity: number | null;
    droppedFrames: number;
    rms: number;
    peak: number;
  }

  interface DetectorState {
    label: string;
    model: string | null;
    threshold: number | null;
    message: string | null;
  }

  interface WakeResult {
    keyword: string;
    score: number | null;
    threshold: number | null;
    at: number;
  }

  interface FixtureResult {
    status: FixtureStatus;
    detected: boolean | null;
    keyword: string | null;
    score: number | null;
    threshold: number | null;
    durationMs: number | null;
    timestamp: string | null;
    message: string;
  }

  interface TerminalFailure {
    state: CaptureState;
    code: StableFailureCode;
    message: string;
  }

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
  const EMPTY_FIXTURE: FixtureResult = {
    status: 'idle',
    detected: null,
    keyword: null,
    score: null,
    threshold: null,
    durationMs: null,
    timestamp: null,
    message: 'Run the bundled WAV without opening the microphone.'
  };
  const STATE_COPY: Record<CaptureState, { label: string; detail: string }> = {
    idle: { label: 'Microphone idle', detail: 'Choose an input, then start capture explicitly.' },
    'requesting-permission': { label: 'Requesting microphone', detail: 'Waiting for browser permission and opening the selected input.' },
    ready: { label: 'Microphone ready', detail: 'Permission is remembered. Start again whenever you are ready.' },
    connecting: { label: 'Negotiating audio', detail: 'Local frames are bounded while the host completes protocol setup.' },
    streaming: { label: 'Listening', detail: 'Audio is streaming to the configured detector host.' },
    stopping: { label: 'Stopping safely', detail: 'Closing the socket, worklet, media track, and audio context.' },
    'error-permission': { label: 'Permission needed', detail: 'Allow microphone access in the browser site settings.' },
    'error-device': { label: 'Input unavailable', detail: 'Refresh inputs or choose a different microphone.' },
    'error-network': { label: 'Host connection failed', detail: 'The Bun host or WSS route could not be reached.' },
    'error-protocol': { label: 'Protocol mismatch', detail: 'The host did not negotiate the expected mono f32 stream.' },
    'error-backpressure': { label: 'Send queue full', detail: 'Capture stopped before audio could buffer without bound.' },
    'error-model': { label: 'Detector unavailable', detail: 'The host model or detector worker did not become ready.' },
    'error-idle': { label: 'Stream idle', detail: 'The host closed the stream because frames stopped arriving.' }
  };
  const TIME_FORMAT = new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  const NUMBER_FORMAT = new Intl.NumberFormat();

  let sessionView = $state<SessionView>('loading');
  let session = $state<SessionInfo>({ authenticated: false, setupRequired: false, username: null });
  let authError = $state('');
  let authSubmitting = $state(false);
  let username = $state('');
  let password = $state('');
  let confirmPassword = $state('');

  let lightMode = $state(false);
  let captureState = $state<CaptureState>('idle');
  let permissionGranted = $state(false);
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
  let detector = $state<DetectorState>({ label: 'Not connected', model: null, threshold: null, message: null });
  let lastWake = $state<WakeResult | null>(null);
  let lastError = $state<{ code: StableFailureCode; message: string } | null>(null);
  let diagnostics = $state<DiagnosticEvent[]>([]);
  let fixture = $state<FixtureResult>({ ...EMPTY_FIXTURE });

  let capture: MicrophoneCapture | null = null;
  let transport: AudioTransport | null = null;
  let unsubscribeTransport: (() => void) | null = null;
  let teardownPromise: Promise<void> | null = null;
  let fixtureAbort: AbortController | null = null;
  let captureGeneration = 0;
  let eventSequence = 0;
  let disposed = false;

  let authenticated = $derived(sessionView === 'authenticated' && session.authenticated);
  let captureActive = $derived(
    captureState === 'requesting-permission' ||
      captureState === 'connecting' ||
      captureState === 'streaming' ||
      captureState === 'stopping'
  );
  let captureBusy = $derived(captureState === 'requesting-permission' || captureState === 'connecting' || captureState === 'stopping');
  let deviceSelectionEnabled = $derived(!captureActive);
  let canRunFixture = $derived(authenticated && !captureActive && fixture.status !== 'running');
  let currentState = $derived(STATE_COPY[captureState]);
  let selectedDevice = $derived(devices.find((device) => device.deviceId === selectedDeviceId));

  onMount(() => {
    lightMode = document.documentElement.dataset.theme === 'light';
    void initializeSession();
    const handleDeviceChange = () => void handleDevicesChanged();
    const handlePageHide = () => void terminateCapture();
    navigator.mediaDevices?.addEventListener('devicechange', handleDeviceChange);
    window.addEventListener('pagehide', handlePageHide);
    return () => {
      disposed = true;
      fixtureAbort?.abort();
      navigator.mediaDevices?.removeEventListener('devicechange', handleDeviceChange);
      window.removeEventListener('pagehide', handlePageHide);
      void terminateCapture();
    };
  });

  async function initializeSession(): Promise<void> {
    sessionView = 'loading';
    authError = '';
    try {
      const current = await fetchSession();
      if (disposed) return;
      session = current;
      sessionView = current.authenticated ? 'authenticated' : 'anonymous';
      addEvent('info', 'Session checked', current.authenticated ? `Signed in as ${current.username ?? 'tester'}.` : 'Authentication is required.');
      if (current.authenticated) await handleDevicesChanged();
    } catch (error) {
      if (disposed) return;
      sessionView = 'unavailable';
      authError = networkError(error, 'The session endpoint could not be reached.');
    }
  }

  async function handleLogin(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (authSubmitting) return;
    authError = '';
    if (session.setupRequired && password !== confirmPassword) {
      authError = 'The setup passwords do not match.';
      return;
    }
    authSubmitting = true;
    try {
      const current = await login(username.trim(), password);
      if (disposed) return;
      session = current;
      sessionView = current.authenticated ? 'authenticated' : 'anonymous';
      password = '';
      confirmPassword = '';
      captureState = 'idle';
      addEvent('success', 'Session established', `Signed in as ${current.username ?? username.trim()}.`);
      await handleDevicesChanged();
    } catch (error) {
      authError = error instanceof TypeError ? networkError(error, 'Sign in could not reach the host.') : safeErrorMessage(error, 'Sign in failed.');
    } finally {
      authSubmitting = false;
    }
  }

  async function handleLogout(): Promise<void> {
    if (captureActive) await terminateCapture();
    fixtureAbort?.abort();
    fixture = { ...EMPTY_FIXTURE };
    try {
      await logout();
      if (disposed) return;
      session = { authenticated: false, setupRequired: false, username: null };
      sessionView = 'anonymous';
      captureState = permissionGranted ? 'ready' : 'idle';
      hostStats = { ...EMPTY_STATS };
      detector = { label: 'Not connected', model: null, threshold: null, message: null };
      addEvent('info', 'Signed out', 'The host session was closed.');
    } catch (error) {
      addEvent('error', 'Sign out failed', safeErrorMessage(error, 'The host could not close the session.'));
    }
  }

  function applyTheme(): void {
    const theme = lightMode ? 'light' : 'dark';
    document.documentElement.dataset.theme = theme;
    document.querySelector('meta[name="theme-color"]')?.setAttribute('content', lightMode ? '#f7f7f7' : '#090909');
    try {
      localStorage.setItem('agent-wake-theme', theme);
    } catch {
      // A blocked storage API must not prevent the explicit theme change.
    }
  }

  async function refreshDevices(): Promise<void> {
    if (!navigator.mediaDevices?.enumerateDevices) return;
    const available = await navigator.mediaDevices.enumerateDevices();
    const inputs = available.filter((device) => device.kind === 'audioinput');
    devices = inputs;
    const selectedStillPresent = inputs.some((device) => device.deviceId === selectedDeviceId);
    if (selectedDeviceId && !selectedStillPresent) {
      if (captureActive) {
        await failCapture('device', 'The selected microphone disconnected. Choose another input and start again.');
      }
      selectedDeviceId = '';
      selectedDeviceLabel = 'Browser default microphone';
      return;
    }
  }

  async function handleDevicesChanged(): Promise<void> {
    try {
      await refreshDevices();
    } catch (error) {
      const message = safeErrorMessage(error, 'The browser could not refresh its input list.');
      if (captureActive) await failCapture('device', message);
      else addEvent('warning', 'Device refresh failed', message);
    }
  }

  async function startMicrophone(): Promise<void> {
    if (!authenticated || captureBusy || captureActive) return;
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
    detector = { label: 'Waiting for host', model: null, threshold: null, message: null };
    lastWake = null;
    let phase: 'permission' | 'network' = 'permission';

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
        void failCapture('backpressure', 'The browser capture callback fell behind. Audio buffering was stopped at the safety limit.');
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
      permissionGranted = true;
      selectedDeviceLabel = info.label;
      if (info.deviceId) selectedDeviceId = info.deviceId;
      captureSampleRate = info.sampleRate;
      await refreshDevices();
      const nextTransport = new WebSocketAudioTransport();
      transport = nextTransport;
      if (generation !== captureGeneration || capture !== nextCapture) return;
      unsubscribeTransport = nextTransport.subscribe(handleTransportEvent);
      await nextTransport.connect();
      nextTransport.start({
        type: 'start',
        protocolVersion: 1,
        sampleRate: info.sampleRate,
        channels: 1,
        encoding: 'pcm_f32le'
      });
      addEvent('info', 'Host connected', 'WSS opened; waiting for the detector ready event.');
    } catch (error) {
      if (generation !== captureGeneration || capture !== nextCapture) return;
      if (disposed) return;
      const mapped = phase === 'permission'
        ? browserAudioFailure(error)
        : { code: 'network' as const, message: networkError(error, 'The secure audio connection could not start.') };
      await failCapture(mapped.code, mapped.message);
    }
  }

  async function stopMicrophone(): Promise<void> {
    if (!captureActive) return;
    captureState = 'stopping';
    await terminateCapture();
    addEvent('info', 'Microphone stopped', 'The media track, AudioWorklet context, and WebSocket were closed.');
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
      detector = { ...detector, label: 'Ready', message: event.encoding };
      addEvent('success', 'Detector ready', `Protocol v1 negotiated at ${NUMBER_FORMAT.format(event.sampleRate)} Hz.`);
      return;
    }
    if (event.type === 'status') {
      detector = {
        label: event.state ?? detector.label,
        model: event.model ?? detector.model,
        threshold: event.threshold ?? detector.threshold,
        message: event.message ?? detector.message
      };
      if (event.message) addEvent('info', `Detector: ${event.state ?? 'status'}`, event.message);
      return;
    }
    if (event.type === 'stats') {
      hostStats = {
        framesReceived: event.framesReceived ?? hostStats.framesReceived,
        bytesReceived: event.bytesReceived ?? hostStats.bytesReceived,
        durationMs: event.durationMs ?? hostStats.durationMs,
        queueDepth: event.queueDepth ?? null,
        queueCapacity: event.queueCapacity ?? null,
        droppedFrames: event.droppedFrames ?? hostStats.droppedFrames,
        rms: event.rms ?? hostStats.rms,
        peak: event.peak ?? hostStats.peak
      };
      return;
    }
    if (event.type === 'wake') {
      const at = parseEventTime(event.timestamp);
      const keyword = (event.keyword?.trim() || 'wake word').slice(0, 200);
      lastWake = {
        keyword,
        score: event.score ?? null,
        threshold: event.threshold ?? detector.threshold,
        at
      };
      detector = { ...detector, label: 'Wake detected' };
      addEvent('success', 'Wake event', `${keyword} at ${TIME_FORMAT.format(at)}.`);
      return;
    }
    if (event.type === 'error') {
      const code = stableFailureCode(event.code);
      void failCapture(code, safeErrorMessage(event.message, `${failureLabel(code)} failed on the host.`));
      return;
    }
    if (event.type === 'stopped') {
      addEvent('info', 'Host stream stopped', event.reason ?? 'The detector closed the audio stream.');
      void terminateCapture();
    }
  }

  async function failCapture(code: StableFailureCode, message: string): Promise<void> {
    await terminateCapture({ state: `error-${code}`, code, message });
  }

  async function terminateCapture(failure?: TerminalFailure): Promise<void> {
    if (teardownPromise) {
      await teardownPromise;
      return;
    }

    captureGeneration += 1;
    const currentTransport = transport;
    const currentCapture = capture;
    const currentUnsubscribe = unsubscribeTransport;
    transport = null;
    capture = null;
    unsubscribeTransport = null;
    transportReady = false;
    if (currentTransport || currentCapture) captureState = 'stopping';

    const work = (async () => {
      currentUnsubscribe?.();
      currentTransport?.stop();
      await currentCapture?.stop();
    })();
    teardownPromise = work;
    await work;
    if (teardownPromise === work) teardownPromise = null;
    if (disposed) return;

    localRms = 0;
    localPeak = 0;
    detector = { label: 'Not connected', model: detector.model, threshold: detector.threshold, message: null };
    if (failure) {
      lastError = { code: failure.code, message: failure.message };
      captureState = failure.state;
      addEvent('error', failureLabel(failure.code), failure.message);
    } else {
      lastError = null;
      captureState = permissionGranted ? 'ready' : 'idle';
    }
  }

  async function runFixture(): Promise<void> {
    if (!canRunFixture) return;
    const controller = new AbortController();
    fixtureAbort = controller;
    fixture = { ...EMPTY_FIXTURE, status: 'running', message: 'Running the bundled WAV through the host detector…' };
    addEvent('info', 'Fixture started', 'The microphone path is not involved in this test.');

    try {
      const response = await fetch('/api/fixture', {
        method: 'POST',
        credentials: 'same-origin',
        cache: 'no-store',
        headers: { Accept: 'application/json' },
        signal: controller.signal
      });
      const value = await readJson(response);
      if (!response.ok) {
        const message = jsonMessage(value, `Fixture request failed (${response.status}).`);
        fixture = { ...EMPTY_FIXTURE, status: 'error', message };
        addEvent('error', 'Fixture failed', message);
        return;
      }
      fixture = parseFixtureResult(value);
      addEvent(
        fixture.status === 'error' ? 'error' : 'success',
        fixture.status === 'error' ? 'Fixture error' : 'Fixture complete',
        fixture.message
      );
    } catch (error) {
      if (controller.signal.aborted) return;
      const message = networkError(error, 'The fixture could not be run.');
      fixture = { ...EMPTY_FIXTURE, status: 'error', message };
      addEvent('error', 'Fixture failed', message);
    } finally {
      if (fixtureAbort === controller) fixtureAbort = null;
    }
  }

  async function readJson(response: Response): Promise<Record<string, unknown>> {
    try {
      const value: unknown = await response.json();
      return isRecord(value) ? value : {};
    } catch {
      return {};
    }
  }

  function parseFixtureResult(value: Record<string, unknown>): FixtureResult {
    const result = isRecord(value.result) ? value.result : value;
    const detected = booleanValue(result.detected ?? result.wakeDetected ?? result.wake_detected);
    const statusText = stringValue(result.status);
    const accepted = booleanValue(result.ok ?? result.success);
    let status: FixtureStatus = accepted === false ? 'error' : detected === true ? 'passed' : detected === false ? 'clear' : 'idle';
    if (statusText === 'error' || statusText === 'failed') status = 'error';
    const keyword = stringValue(result.keyword ?? result.text);
    const score = numberValue(result.score);
    const threshold = numberValue(result.threshold);
    const durationMs = numberValue(result.durationMs ?? result.duration_ms);
    const timestamp = stringValue(result.timestamp);
    const fallback = detected === true
      ? `${keyword || 'Wake word'} detected in the fixture.`
      : detected === false
        ? 'No wake word was detected in the fixture.'
        : statusText === 'accepted' || statusText === 'queued'
          ? 'The host accepted the fixture request.'
          : 'The host returned no detection result.';
    return {
      status,
      detected,
      keyword,
      score,
      threshold,
      durationMs,
      timestamp,
      message: jsonMessage(result, fallback)
    };
  }

  function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null;
  }

  function booleanValue(value: unknown): boolean | null {
    if (typeof value === 'boolean') return value;
    if (value === 0 || value === 1) return value === 1;
    return null;
  }

  function stringValue(value: unknown): string | null {
    return typeof value === 'string' && value.trim() ? value.trim().slice(0, 300) : null;
  }

  function numberValue(value: unknown): number | null {
    return typeof value === 'number' && Number.isFinite(value) ? value : null;
  }

  function jsonMessage(value: Record<string, unknown>, fallback: string): string {
    return stringValue(value.message ?? value.error) ?? fallback;
  }

  function parseEventTime(value: string | number | undefined): number {
    if (typeof value === 'number' && Number.isFinite(value)) {
      return value < 1_000_000_000_000 ? value * 1000 : value;
    }
    if (typeof value === 'string') {
      const parsed = Date.parse(value);
      if (Number.isFinite(parsed)) return parsed;
    }
    return Date.now();
  }

  function addEvent(tone: EventTone, title: string, detail: string): void {
    eventSequence += 1;
    const at = Date.now();
    diagnostics = [{ id: eventSequence, time: TIME_FORMAT.format(at), iso: new Date(at).toISOString(), tone, title, detail }, ...diagnostics].slice(0, 40);
  }

  function meterPercent(amplitude: number): number {
    if (!Number.isFinite(amplitude) || amplitude <= 0) return 0;
    return Math.min(100, Math.round(Math.sqrt(Math.min(1, amplitude)) * 100));
  }

  function decibels(amplitude: number): string {
    if (!Number.isFinite(amplitude) || amplitude <= 0) return '−∞ dB';
    return `${(20 * Math.log10(amplitude)).toFixed(1)} dB`;
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${NUMBER_FORMAT.format(bytes)} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
  }

  function formatDuration(milliseconds: number): string {
    if (milliseconds < 1000) return `${NUMBER_FORMAT.format(Math.round(milliseconds))} ms`;
    return `${(milliseconds / 1000).toFixed(1)} s`;
  }

  function formatMetric(value: number | null, digits = 3): string {
    return value === null ? '—' : value.toFixed(digits);
  }
</script>

<svelte:head>
  <title>Wake word lab · Agent</title>
</svelte:head>

<div class="app-shell">
  <header class="topbar">
    <a class="brand" href="/" aria-label="Agent wake word lab home">
      <span class="brand-mark" aria-hidden="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
          <path d="M12 3v3m0 12v3M3 12h3m12 0h3M5.6 5.6l2.1 2.1m8.6 8.6 2.1 2.1m0-12.8-2.1 2.1m-8.6 8.6-2.1 2.1" />
          <circle cx="12" cy="12" r="4" />
        </svg>
      </span>
      <span>Agent</span>
      <span class="brand-divider">/</span>
      <span class="brand-section">Wake word lab</span>
    </a>
    <div class="header-actions">
      {#if authenticated}
        <span class="account-name">{session.username ?? 'Tester'}</span>
        <button class="quiet-button" type="button" onclick={() => void handleLogout()}>Sign out</button>
      {/if}
      <button
        class="theme-toggle"
        type="button"
        aria-label={lightMode ? 'Switch to dark mode' : 'Switch to light mode'}
        aria-pressed={lightMode}
        onclick={() => {
          lightMode = !lightMode;
          applyTheme();
        }}
      >
        {#if lightMode}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
            <circle cx="12" cy="12" r="4" /><path d="M12 2v2m0 16v2M4.9 4.9l1.4 1.4m11.4 11.4 1.4 1.4M2 12h2m16 0h2M4.9 19.1l1.4-1.4M19.1 4.9l-1.4 1.4" />
          </svg>
          Dark
        {:else}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
            <path d="M20.9 13A9 9 0 0 1 11 3.1 9 9 0 1 0 20.9 13Z" />
          </svg>
          Light
        {/if}
      </button>
    </div>
  </header>

  <main>
    {#if sessionView === 'loading'}
      <section class="auth-card loading-card" aria-live="polite">
        <span class="spinner" aria-hidden="true"></span>
        <p class="eyebrow">CHECKING SESSION</p>
        <h1>Opening the lab…</h1>
        <p class="muted">Contacting the same-origin Bun host.</p>
      </section>
    {:else if !authenticated}
      <section class="auth-wrap" aria-labelledby="auth-title">
        <div class="auth-intro">
          <p class="eyebrow"><span></span> PRIVATE TEST HARNESS</p>
          <h1 id="auth-title">Hear the signal.<br /><em>Trust the path.</em></h1>
          <p>Stream one microphone to the sherpa-onnx detector, inspect every handoff, and keep the browser side deliberately small.</p>
          <div class="promise-list">
            <span><i aria-hidden="true"></i> Explicit microphone permission</span>
            <span><i aria-hidden="true"></i> No captured-audio storage</span>
            <span><i aria-hidden="true"></i> Same-origin authenticated WSS</span>
          </div>
        </div>

        <div class="auth-card">
          <span class="auth-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
              <rect x="5" y="10" width="14" height="10" rx="3" /><path d="M8 10V7a4 4 0 0 1 8 0v3M12 14v2" />
            </svg>
          </span>
          <p class="eyebrow">{session.setupRequired ? 'FIRST-RUN SETUP' : 'AUTHORIZED TESTERS'}</p>
          <h2>{session.setupRequired ? 'Secure this lab.' : 'Welcome back.'}</h2>
          <p class="muted auth-copy">
            {session.setupRequired
              ? 'Create the local host account. Credentials are sent only to this origin and are never embedded in the static app.'
              : 'Sign in with the application session created by the Bun host.'}
          </p>

          {#if authError}
            <div class="inline-error" role="alert">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M12 8v5m0 3h.01" /></svg>
              <span>{authError}</span>
            </div>
          {/if}

          {#if sessionView === 'unavailable'}
            <button class="primary-button" type="button" onclick={() => void initializeSession()}>Try session check again</button>
          {:else}
            <form onsubmit={handleLogin}>
              <label>
                <span>Username</span>
                <input bind:value={username} autocomplete="username" minlength="3" required disabled={authSubmitting} />
              </label>
              <label>
                <span>Password</span>
                <input bind:value={password} type="password" autocomplete={session.setupRequired ? 'new-password' : 'current-password'} minlength="8" required disabled={authSubmitting} />
              </label>
              {#if session.setupRequired}
                <label>
                  <span>Confirm password</span>
                  <input bind:value={confirmPassword} type="password" autocomplete="new-password" minlength="8" required disabled={authSubmitting} />
                </label>
              {/if}
              <button class="primary-button" type="submit" disabled={authSubmitting}>
                {authSubmitting ? 'Contacting host…' : session.setupRequired ? 'Create secure session' : 'Enter the lab'}
                <span aria-hidden="true">→</span>
              </button>
            </form>
          {/if}
          <p class="security-note">The browser never receives model paths, host secrets, or a WebSocket credential.</p>
        </div>
      </section>
    {:else}
      <section class="hero">
        <div>
          <p class="eyebrow"><span></span> SHERPA-ONNX DIAGNOSTICS</p>
          <h1>Every frame has<br /><em>a visible story.</em></h1>
        </div>
        <p>Test the real microphone path and the bundled fixture independently. Audio reaches the configured host for inference and is never saved by this UI.</p>
      </section>

      {#if lastError}
        <section class="failure-banner" role="alert" aria-labelledby="failure-title">
          <span class="failure-icon" aria-hidden="true">!</span>
          <div>
            <p class="failure-code">{failureLabel(lastError.code)}</p>
            <h2 id="failure-title">{currentState.label}</h2>
            <p>{lastError.message}</p>
          </div>
          {#if captureState === 'error-permission' || captureState === 'error-device'}
            <button class="secondary-button" type="button" onclick={() => void handleDevicesChanged()}>Refresh inputs</button>
          {/if}
        </section>
      {/if}

      <section class="pipeline" aria-label="Audio pipeline state">
        <div class:active={permissionGranted} class="pipeline-step">
          <span>{permissionGranted ? '✓' : '1'}</span><div><strong>Browser permission</strong><small>{permissionGranted ? 'Granted' : 'Waiting'}</small></div>
        </div>
        <i aria-hidden="true"></i>
        <div class:active={captureState === 'connecting' || captureState === 'streaming'} class="pipeline-step">
          <span>{captureState === 'connecting' || captureState === 'streaming' ? '✓' : '2'}</span><div><strong>Local capture</strong><small>{localFrames ? `${NUMBER_FORMAT.format(localFrames)} frames` : 'Not started'}</small></div>
        </div>
        <i aria-hidden="true"></i>
        <div class:active={transportReady} class="pipeline-step">
          <span>{transportReady ? '✓' : '3'}</span><div><strong>Host transport</strong><small>{transportReady ? 'Receiving' : 'Not ready'}</small></div>
        </div>
        <i aria-hidden="true"></i>
        <div class:active={detector.label === 'Ready' || detector.label === 'Wake detected'} class="pipeline-step">
          <span>{detector.label === 'Ready' || detector.label === 'Wake detected' ? '✓' : '4'}</span><div><strong>Detector</strong><small>{detector.label}</small></div>
        </div>
      </section>

      <div class="dashboard-grid">
        <section class="panel capture-panel" aria-labelledby="capture-title">
          <div class="panel-heading">
            <div>
              <p class="eyebrow">01 · MICROPHONE</p>
              <h2 id="capture-title">Local capture</h2>
            </div>
            <span class:listening={captureState === 'streaming'} class="state-pill">
              <i aria-hidden="true"></i>{currentState.label}
            </span>
          </div>
          <p class="panel-description">{currentState.detail}</p>

          <label class="device-field">
            <span>Input device</span>
            <div class="select-row">
              <select bind:value={selectedDeviceId} disabled={!deviceSelectionEnabled} aria-describedby="device-help">
                <option value="">Browser default microphone</option>
                {#each devices as device}
                  <option value={device.deviceId}>{device.label || 'Unnamed audio input'}</option>
                {/each}
              </select>
              <button class="icon-button" type="button" aria-label="Refresh microphone inputs" title="Refresh inputs" disabled={!deviceSelectionEnabled} onclick={() => void handleDevicesChanged()}>
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true"><path d="M20 6v5h-5M4 18v-5h5M18.5 9A7 7 0 0 0 6 6.5L4 9m16 6-2 2.5A7 7 0 0 1 5.5 15" /></svg>
              </button>
            </div>
            <small id="device-help">
              {selectedDevice?.label || (selectedDeviceId ? selectedDeviceLabel : 'Use the browser-selected default input')}
              {devices.length ? ` · ${devices.length} input${devices.length === 1 ? '' : 's'} found` : ' · labels appear after permission'}
            </small>
          </label>

          <div class="control-row">
            {#if captureActive}
              <button class="stop-button" type="button" disabled={captureState === 'stopping'} onclick={() => void stopMicrophone()}>
                <span class="stop-square" aria-hidden="true"></span>{captureState === 'stopping' ? 'Stopping…' : 'Stop microphone'}
              </button>
            {:else}
              <button class="primary-button" type="button" disabled={captureBusy} onclick={() => void startMicrophone()}>
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="m8 5 11 7-11 7V5Z" /></svg>
                Start microphone
              </button>
            {/if}
            <div class="sample-rate">
              <span>Capture rate</span>
              <strong>{captureSampleRate ? `${NUMBER_FORMAT.format(captureSampleRate)} Hz` : 'Not negotiated'}</strong>
            </div>
          </div>

          <div class="privacy-strip">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="M12 3 5 6v5c0 4.6 2.9 8.8 7 10 4.1-1.2 7-5.4 7-10V6l-7-3Z" /><path d="m9.5 12 1.7 1.7 3.5-3.7" /></svg>
            <span><strong>Remote test, explicit capture.</strong> Audio travels through the configured host to the detector. This UI does not record or persist it.</span>
          </div>
        </section>

        <section class="panel meters-panel" aria-labelledby="meters-title">
          <div class="panel-heading compact">
            <div>
              <p class="eyebrow">02 · SIGNAL PATH</p>
              <h2 id="meters-title">Local → host</h2>
            </div>
            <span class="mono-label">PCM_F32LE · MONO</span>
          </div>

          <div class="meter-block">
            <div class="meter-heading">
              <span><i class="local-dot" aria-hidden="true"></i>Browser input</span>
              <strong>{decibels(localRms)}</strong>
            </div>
            <div class="meter-track" role="meter" aria-label="Local microphone level" aria-valuemin="0" aria-valuemax="100" aria-valuenow={meterPercent(localRms)}>
              <span class="local-fill" style={`width: ${meterPercent(localRms)}%`}></span>
            </div>
            <div class="metric-grid three">
              <div><span>Local frames</span><strong>{NUMBER_FORMAT.format(localFrames)}</strong></div>
              <div><span>Local peak</span><strong>{decibels(localPeak)}</strong></div>
              <div><span>Local bytes</span><strong>{formatBytes(localBytes)}</strong></div>
            </div>
          </div>

          <div class="path-arrow" aria-hidden="true"><span></span><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m9 18 6-6-6-6" /></svg></div>

          <div class="meter-block remote-meter">
            <div class="meter-heading">
              <span><i class="host-dot" aria-hidden="true"></i>Host received</span>
              <strong>{decibels(hostStats.rms)}</strong>
            </div>
            <div class="meter-track" role="meter" aria-label="Host detector level" aria-valuemin="0" aria-valuemax="100" aria-valuenow={meterPercent(hostStats.rms)}>
              <span class="host-fill" style={`width: ${meterPercent(hostStats.rms)}%`}></span>
            </div>
            <div class="metric-grid four">
              <div><span>Host frames</span><strong>{NUMBER_FORMAT.format(hostStats.framesReceived)}</strong></div>
              <div><span>Host bytes</span><strong>{formatBytes(hostStats.bytesReceived)}</strong></div>
              <div><span>Queue</span><strong>{hostStats.queueDepth === null ? '—' : `${hostStats.queueDepth}${hostStats.queueCapacity ? ` / ${hostStats.queueCapacity}` : ''}`}</strong></div>
              <div><span>Dropped</span><strong>{NUMBER_FORMAT.format(hostStats.droppedFrames)}</strong></div>
            </div>
          </div>
        </section>

        <section class="panel detector-panel" aria-labelledby="detector-title">
          <div class="panel-heading compact">
            <div>
              <p class="eyebrow">03 · DETECTOR</p>
              <h2 id="detector-title">Wake state</h2>
            </div>
            <span class:ready={detector.label === 'Ready' || detector.label === 'Wake detected'} class="detector-badge">{detector.label}</span>
          </div>

          <div class="detector-visual" class:awake={captureState === 'streaming' && lastWake !== null}>
            <div class="wave-rings" aria-hidden="true"><i></i><i></i><i></i><span></span></div>
            <p>{captureState !== 'streaming' ? 'DETECTOR NOT LISTENING' : lastWake ? 'WAKE WORD DETECTED' : 'LISTENING FOR KEYWORD'}</p>
            <strong>{detector.threshold === null ? '—' : detector.threshold.toFixed(3)}</strong>
            <small>threshold</small>
          </div>

          <dl class="detector-details">
            <div><dt>Model</dt><dd>{detector.model ?? 'Not reported'}</dd></div>
            <div><dt>Status detail</dt><dd>{detector.message ?? 'Start the microphone or run a fixture.'}</dd></div>
            <div><dt>Last wake</dt><dd>{lastWake ? `${lastWake.keyword} · ${TIME_FORMAT.format(lastWake.at)}` : 'No wake event yet'}</dd></div>
            <div><dt>Wake score</dt><dd>{lastWake?.score === null || lastWake === null ? '—' : formatMetric(lastWake.score, 4)}</dd></div>
          </dl>
        </section>

        <section class="panel fixture-panel" aria-labelledby="fixture-title">
          <div class="fixture-copy">
            <p class="eyebrow">04 · FIXTURE CONTROL</p>
            <h2 id="fixture-title">Test without the mic</h2>
            <p>Run the curated WAV through the same host detector. This isolates model and inference failures from browser permission, device, and network problems.</p>
            <button class="secondary-button fixture-button" type="button" disabled={!canRunFixture} onclick={() => void runFixture()}>
              {#if fixture.status === 'running'}<span class="spinner small" aria-hidden="true"></span>Running fixture…{:else}<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" aria-hidden="true"><path d="m8 5 11 7-11 7V5Z" /></svg>Run WAV fixture{/if}
            </button>
            {#if captureActive}<small class="disabled-note">Stop the active microphone stream before running the fixture.</small>{/if}
          </div>
          <div class:success={fixture.status === 'passed'} class:clear={fixture.status === 'clear'} class="fixture-result">
            <span class="fixture-result-icon" aria-hidden="true">
              {#if fixture.status === 'passed'}✓{:else if fixture.status === 'error'}!{:else if fixture.status === 'running'}…{:else}○{/if}
            </span>
            <div>
              <strong>{fixture.status === 'passed' ? 'Wake detected' : fixture.status === 'clear' ? 'No detection' : fixture.status === 'error' ? 'Fixture error' : fixture.status === 'running' ? 'Processing' : 'Fixture ready'}</strong>
              <p>{fixture.message}</p>
              {#if fixture.keyword || fixture.durationMs !== null || fixture.score !== null}
                <small>{fixture.keyword ?? 'Bundled WAV'} · {fixture.durationMs === null ? 'duration —' : formatDuration(fixture.durationMs)} · score {formatMetric(fixture.score, 4)}</small>
              {/if}
            </div>
          </div>
        </section>

        <section class="panel diagnostics-panel" aria-labelledby="diagnostics-title">
          <div class="panel-heading compact">
            <div>
              <p class="eyebrow">05 · DIAGNOSTICS</p>
              <h2 id="diagnostics-title">Event trail</h2>
            </div>
            <span class="mono-label">LAST {diagnostics.length} / 40</span>
          </div>
          {#if diagnostics.length === 0}
            <div class="empty-log"><span aria-hidden="true">—</span><p>Session and capture events will appear here.</p></div>
          {:else}
            <ol class="event-list">
              {#each diagnostics as event (event.id)}
                <li class={event.tone}>
                  <span class="event-dot" aria-hidden="true"></span>
                  <div><strong>{event.title}</strong><p>{event.detail}</p></div>
                  <time datetime={event.iso}>{event.time}</time>
                </li>
              {/each}
            </ol>
          {/if}
          <div class="diagnostic-foot">
            <span>PCM only in memory</span><span>Queue cap 250 ms</span><span>Worklet pool 3 frames</span><span>Client overflows {localOverflows}</span>
          </div>
        </section>
      </div>
    {/if}
  </main>

  <footer>
    <span>LEXIA · WAKE WORD LAB</span>
    <span>Same-origin host · No audio persistence</span>
    <a href="/agent-wake.zip" download>Project ZIP</a>
  </footer>
</div>

<style>
  .app-shell {
    width: min(calc(100% - 2rem), 86rem);
    min-height: 100vh;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
  }

  .topbar {
    min-height: 5.25rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    border-bottom: 1px solid var(--border);
  }

  .brand {
    display: inline-flex;
    align-items: center;
    gap: 0.65rem;
    color: var(--foreground);
    font-weight: 700;
    letter-spacing: -0.025em;
    text-decoration: none;
  }

  .brand-mark {
    width: 2.25rem;
    height: 2.25rem;
    display: grid;
    place-items: center;
    border-radius: 0.7rem;
    background: var(--primary);
    color: var(--primary-foreground);
    box-shadow: 0 0 1.8rem color-mix(in oklch, var(--primary) 25%, transparent);
  }

  .brand-mark svg { width: 1.25rem; height: 1.25rem; }
  .brand-divider { color: var(--muted-foreground); font-weight: 450; }
  .brand-section { color: var(--muted-foreground); font-weight: 500; }
  .header-actions { display: flex; align-items: center; gap: 0.65rem; }
  .account-name { max-width: 10rem; overflow: hidden; color: var(--muted-foreground); font-size: 0.8rem; text-overflow: ellipsis; white-space: nowrap; }

  button {
    border: 0;
  }

  .theme-toggle,
  .quiet-button,
  .icon-button,
  .secondary-button,
  .stop-button,
  .primary-button {
    min-height: 2.65rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.55rem;
    border-radius: var(--radius);
    font-weight: 700;
    transition: transform 150ms ease, filter 150ms ease, border-color 150ms ease, background 150ms ease;
  }

  .theme-toggle,
  .quiet-button,
  .icon-button,
  .secondary-button {
    border: 1px solid var(--border);
    background: var(--surface-soft);
    color: var(--foreground);
  }

  .theme-toggle { padding: 0 0.8rem; font-size: 0.78rem; }
  .theme-toggle svg { width: 1rem; height: 1rem; }
  .quiet-button { padding: 0 0.8rem; color: var(--muted-foreground); font-size: 0.78rem; }
  .theme-toggle:hover,
  .quiet-button:hover,
  .icon-button:hover,
  .secondary-button:hover { border-color: color-mix(in oklch, var(--primary) 50%, var(--border)); background: color-mix(in oklch, var(--primary) 8%, var(--surface-soft)); }
  .theme-toggle:active,
  .secondary-button:active,
  .primary-button:active,
  .stop-button:active { transform: translateY(1px); }
  button:disabled { opacity: 0.48; filter: saturate(0.45); }

  main { flex: 1; padding: clamp(3.5rem, 8vw, 7rem) 0 4rem; }
  h1, h2, p { margin-top: 0; }
  h1 { margin-bottom: 1.4rem; font-size: clamp(3rem, 7.5vw, 6.6rem); font-weight: 700; line-height: 0.94; letter-spacing: -0.065em; }
  h1 em { color: var(--primary); font-style: normal; }
  h2 { margin-bottom: 0; font-size: 1.2rem; letter-spacing: -0.025em; }

  .eyebrow {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    margin-bottom: 1rem;
    color: var(--accent);
    font: 400 0.69rem 'iA Writer Duo', monospace;
    letter-spacing: 0.12em;
  }

  .eyebrow > span { width: 1.4rem; height: 1px; background: currentColor; }
  .muted { color: var(--muted-foreground); line-height: 1.6; }

  .auth-wrap {
    min-height: 35rem;
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(21rem, 29rem);
    align-items: center;
    gap: clamp(3rem, 9vw, 8rem);
  }

  .auth-intro > p:not(.eyebrow) { max-width: 42rem; color: var(--muted-foreground); font-size: 1.1rem; line-height: 1.7; }
  .promise-list { display: flex; flex-wrap: wrap; gap: 0.7rem 1.2rem; margin-top: 2rem; color: var(--muted-foreground); font-size: 0.78rem; }
  .promise-list span { display: inline-flex; align-items: center; gap: 0.45rem; }
  .promise-list i { width: 0.42rem; height: 0.42rem; border-radius: 50%; background: var(--success); box-shadow: 0 0 0.65rem color-mix(in oklch, var(--success) 45%, transparent); }

  .auth-card {
    padding: clamp(1.6rem, 4vw, 2.5rem);
    border: 1px solid var(--border);
    border-radius: 1.1rem;
    background: color-mix(in oklch, var(--card) 94%, transparent);
    box-shadow: var(--shadow);
  }

  .auth-card h2 { margin-bottom: 0.75rem; font-size: 2rem; }
  .auth-copy { font-size: 0.88rem; }
  .auth-icon { width: 3rem; height: 3rem; display: grid; place-items: center; margin-bottom: 2rem; border-radius: 0.85rem; background: var(--accent); color: #090909; }
  .auth-icon svg { width: 1.4rem; height: 1.4rem; }
  .auth-card form { display: grid; gap: 1rem; margin-top: 1.5rem; }
  .auth-card label { display: grid; gap: 0.4rem; color: var(--muted-foreground); font-size: 0.75rem; font-weight: 700; }
  .auth-card input { width: 100%; min-height: 2.9rem; padding: 0.7rem 0.8rem; border: 1px solid var(--input); border-radius: 0.55rem; background: var(--background); color: var(--foreground); }
  .auth-card input:focus { border-color: var(--primary); }
  .security-note { margin: 1.4rem 0 0; color: var(--muted-foreground); font-size: 0.68rem; line-height: 1.55; text-align: center; }
  .inline-error { display: flex; align-items: flex-start; gap: 0.6rem; margin-top: 1rem; padding: 0.75rem; border: 1px solid color-mix(in oklch, var(--destructive) 40%, transparent); border-radius: 0.55rem; background: color-mix(in oklch, var(--destructive) 10%, transparent); color: var(--destructive); font-size: 0.78rem; line-height: 1.45; }
  .inline-error svg { width: 1rem; flex: none; margin-top: 0.1rem; }
  .loading-card { width: min(100%, 32rem); margin: 8vh auto 0; text-align: center; }
  .loading-card .eyebrow { justify-content: center; }
  .loading-card h1 { font-size: 2.5rem; }

  .primary-button {
    min-height: 3rem;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    background: var(--primary);
    color: var(--primary-foreground);
  }
  .primary-button svg { width: 1.1rem; }
  .auth-card .primary-button { width: 100%; margin-top: 0.35rem; }
  .primary-button:hover:not(:disabled) { filter: brightness(1.05); transform: translateY(-1px); }
  .secondary-button { padding: 0.65rem 0.9rem; font-size: 0.8rem; }
  .secondary-button svg { width: 1rem; }

  .spinner { width: 1.6rem; height: 1.6rem; display: inline-block; border: 2px solid var(--border); border-top-color: var(--primary); border-radius: 50%; animation: spin 700ms linear infinite; }
  .spinner.small { width: 1rem; height: 1rem; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .hero { display: grid; grid-template-columns: minmax(0, 1.4fr) minmax(18rem, 0.6fr); align-items: end; gap: 3rem; margin-bottom: 3rem; }
  .hero h1 { margin-bottom: 0; }
  .hero > p { margin-bottom: 0.4rem; color: var(--muted-foreground); font-size: 1rem; line-height: 1.7; }

  .failure-banner { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; align-items: center; gap: 1rem; margin-bottom: 1rem; padding: 1rem 1.2rem; border: 1px solid color-mix(in oklch, var(--destructive) 38%, var(--border)); border-radius: var(--radius); background: color-mix(in oklch, var(--destructive) 9%, var(--card)); }
  .failure-icon { width: 2rem; height: 2rem; display: grid; place-items: center; border-radius: 50%; background: var(--destructive); color: white; font-weight: 700; }
  .failure-banner h2 { margin-bottom: 0.2rem; font-size: 1rem; }
  .failure-banner p:last-child { margin-bottom: 0; color: var(--muted-foreground); font-size: 0.8rem; }
  .failure-code { margin-bottom: 0.15rem; color: var(--destructive); font: 400 0.65rem 'iA Writer Duo', monospace; text-transform: uppercase; }

  .pipeline { display: flex; align-items: center; margin-bottom: 1rem; padding: 0.9rem 1rem; border: 1px solid var(--border); border-radius: var(--radius); background: var(--surface-soft); }
  .pipeline-step { min-width: 0; display: flex; align-items: center; gap: 0.55rem; }
  .pipeline-step > span { width: 1.55rem; height: 1.55rem; display: grid; flex: none; place-items: center; border: 1px solid var(--border); border-radius: 50%; color: var(--muted-foreground); font: 400 0.65rem 'iA Writer Duo', monospace; }
  .pipeline-step.active > span { border-color: color-mix(in oklch, var(--success) 70%, transparent); background: color-mix(in oklch, var(--success) 18%, transparent); color: var(--success); }
  .pipeline-step div { min-width: 0; display: grid; }
  .pipeline-step strong { overflow: hidden; font-size: 0.72rem; text-overflow: ellipsis; white-space: nowrap; }
  .pipeline-step small { overflow: hidden; color: var(--muted-foreground); font-size: 0.62rem; text-overflow: ellipsis; white-space: nowrap; }
  .pipeline > i { height: 1px; flex: 1; min-width: 1rem; margin: 0 0.7rem; background: var(--border); }

  .dashboard-grid { display: grid; grid-template-columns: minmax(0, 1.05fr) minmax(0, 0.95fr); gap: 1rem; }
  .panel { min-width: 0; padding: clamp(1.15rem, 2.5vw, 1.6rem); border: 1px solid var(--border); border-radius: var(--radius); background: color-mix(in oklch, var(--card) 92%, transparent); box-shadow: 0 0.6rem 2rem oklch(0 0 0 / 8%); }
  .panel-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; }
  .panel-heading.compact { align-items: center; }
  .panel-heading .eyebrow { margin-bottom: 0.45rem; }
  .panel-description { min-height: 2.7rem; margin: 0.7rem 0 1.3rem; color: var(--muted-foreground); font-size: 0.8rem; line-height: 1.55; }
  .state-pill, .detector-badge, .mono-label { display: inline-flex; align-items: center; gap: 0.45rem; padding: 0.38rem 0.58rem; border-radius: 999px; background: var(--muted); color: var(--muted-foreground); font-size: 0.66rem; font-weight: 700; white-space: nowrap; }
  .state-pill i { width: 0.42rem; height: 0.42rem; border-radius: 50%; background: currentColor; }
  .state-pill.listening { background: color-mix(in oklch, var(--success) 15%, transparent); color: var(--success); }
  .state-pill.listening i { box-shadow: 0 0 0.7rem var(--success); animation: pulse 1.4s ease-in-out infinite; }
  @keyframes pulse { 50% { opacity: 0.35; } }
  .mono-label { border-radius: 0.35rem; font: 400 0.62rem 'iA Writer Duo', monospace; letter-spacing: 0.04em; }

  .device-field { display: grid; gap: 0.45rem; }
  .device-field > span { color: var(--muted-foreground); font-size: 0.72rem; font-weight: 700; }
  .device-field small { color: var(--muted-foreground); font-size: 0.68rem; line-height: 1.45; }
  .select-row { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 0.5rem; }
  select, .auth-card input { border: 1px solid var(--input); }
  select { width: 100%; min-height: 2.8rem; padding: 0 2rem 0 0.75rem; border-radius: 0.55rem; appearance: none; background: var(--background) linear-gradient(45deg, transparent 50%, var(--muted-foreground) 50%) calc(100% - 14px) 50% / 5px 5px no-repeat, var(--background); color: var(--foreground); font-size: 0.78rem; }
  .icon-button { width: 2.8rem; min-height: 2.8rem; padding: 0; }
  .icon-button svg { width: 1rem; }
  .control-row { display: flex; align-items: center; gap: 1rem; margin-top: 1.2rem; }
  .control-row .primary-button, .stop-button { min-width: 12rem; }
  .stop-button { padding: 0.75rem 1rem; border: 1px solid color-mix(in oklch, var(--destructive) 55%, var(--border)); background: color-mix(in oklch, var(--destructive) 12%, transparent); color: var(--destructive); }
  .stop-square { width: 0.65rem; height: 0.65rem; border-radius: 0.12rem; background: currentColor; }
  .sample-rate { display: grid; gap: 0.1rem; }
  .sample-rate span { color: var(--muted-foreground); font-size: 0.62rem; text-transform: uppercase; }
  .sample-rate strong { font: 400 0.75rem 'iA Writer Duo', monospace; }
  .privacy-strip { display: flex; align-items: flex-start; gap: 0.65rem; margin-top: 1.2rem; padding: 0.75rem; border-radius: 0.55rem; background: color-mix(in oklch, var(--accent) 8%, transparent); color: var(--muted-foreground); font-size: 0.68rem; line-height: 1.5; }
  .privacy-strip svg { width: 1rem; flex: none; color: var(--accent); }
  .privacy-strip strong { color: var(--foreground); }

  .meter-block { padding: 1rem; border: 1px solid var(--border); border-radius: 0.6rem; background: var(--surface-soft); }
  .meter-heading { display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin-bottom: 0.65rem; font-size: 0.74rem; }
  .meter-heading > span { display: inline-flex; align-items: center; gap: 0.5rem; font-weight: 700; }
  .meter-heading strong { font: 400 0.7rem 'iA Writer Duo', monospace; }
  .local-dot, .host-dot { width: 0.45rem; height: 0.45rem; border-radius: 50%; }
  .local-dot { background: var(--primary); }
  .host-dot { background: var(--accent); }
  .meter-track { position: relative; height: 0.6rem; overflow: hidden; border-radius: 999px; background: var(--muted); }
  .meter-track span { position: absolute; inset: 0 auto 0 0; min-width: 0; border-radius: inherit; transition: width 80ms linear; }
  .local-fill { background: linear-gradient(90deg, var(--secondary), var(--primary)); }
  .host-fill { background: linear-gradient(90deg, var(--secondary), var(--accent)); }
  .metric-grid { display: grid; gap: 0.4rem; margin-top: 0.85rem; }
  .metric-grid.three { grid-template-columns: repeat(3, 1fr); }
  .metric-grid.four { grid-template-columns: repeat(4, 1fr); }
  .metric-grid div { min-width: 0; display: grid; gap: 0.18rem; }
  .metric-grid span { color: var(--muted-foreground); font-size: 0.57rem; }
  .metric-grid strong { overflow: hidden; font: 400 0.68rem 'iA Writer Duo', monospace; text-overflow: ellipsis; white-space: nowrap; }
  .path-arrow { height: 2.2rem; display: flex; align-items: center; justify-content: center; color: var(--muted-foreground); transform: rotate(90deg); }
  .path-arrow span { width: 2.5rem; height: 1px; background: var(--border); }
  .path-arrow svg { width: 0.8rem; margin-left: -0.2rem; }

  .detector-badge.ready { background: color-mix(in oklch, var(--success) 15%, transparent); color: var(--success); }
  .detector-visual { min-height: 13rem; display: flex; flex-direction: column; align-items: center; justify-content: center; margin-top: 1rem; overflow: hidden; border: 1px solid var(--border); border-radius: 0.65rem; background: radial-gradient(circle, color-mix(in oklch, var(--secondary) 15%, transparent), transparent 62%), var(--background); }
  .wave-rings { position: relative; width: 5.2rem; height: 5.2rem; display: grid; place-items: center; margin-bottom: 0.8rem; }
  .wave-rings i { position: absolute; border: 1px solid color-mix(in oklch, var(--accent) 32%, transparent); border-radius: 50%; }
  .wave-rings i:nth-child(1) { width: 2.4rem; height: 2.4rem; }
  .wave-rings i:nth-child(2) { width: 3.7rem; height: 3.7rem; }
  .wave-rings i:nth-child(3) { width: 5.1rem; height: 5.1rem; }
  .wave-rings span { width: 0.8rem; height: 0.8rem; border-radius: 50%; background: var(--accent); box-shadow: 0 0 1.5rem var(--accent); }
  .detector-visual.awake .wave-rings span { background: var(--primary); box-shadow: 0 0 1.8rem var(--primary); }
  .detector-visual p { margin-bottom: 0.6rem; color: var(--muted-foreground); font: 400 0.6rem 'iA Writer Duo', monospace; letter-spacing: 0.08em; }
  .detector-visual strong { font: 400 1.35rem 'iA Writer Duo', monospace; }
  .detector-visual small { color: var(--muted-foreground); font-size: 0.6rem; }
  .detector-details { display: grid; margin: 0.8rem 0 0; }
  .detector-details div { display: grid; grid-template-columns: 6.5rem minmax(0, 1fr); gap: 0.7rem; padding: 0.55rem 0; border-top: 1px solid var(--border); }
  .detector-details dt { color: var(--muted-foreground); font-size: 0.64rem; }
  .detector-details dd { min-width: 0; margin: 0; overflow: hidden; font: 400 0.67rem 'iA Writer Duo', monospace; text-overflow: ellipsis; white-space: nowrap; }

  .fixture-panel { grid-column: 1 / -1; display: grid; grid-template-columns: minmax(0, 0.85fr) minmax(18rem, 1.15fr); align-items: center; gap: 2rem; }
  .fixture-copy > p:not(.eyebrow) { max-width: 36rem; margin-bottom: 1.1rem; color: var(--muted-foreground); font-size: 0.8rem; line-height: 1.6; }
  .fixture-button { min-width: 10.5rem; }
  .disabled-note { display: block; margin-top: 0.55rem; color: var(--muted-foreground); font-size: 0.64rem; }
  .fixture-result { min-height: 7.5rem; display: flex; align-items: flex-start; gap: 0.8rem; padding: 1rem; border: 1px solid var(--border); border-radius: 0.65rem; background: var(--surface-soft); }
  .fixture-result-icon { width: 2rem; height: 2rem; display: grid; flex: none; place-items: center; border: 1px solid var(--border); border-radius: 50%; color: var(--muted-foreground); font-weight: 700; }
  .fixture-result strong { font-size: 0.84rem; }
  .fixture-result p { margin: 0.3rem 0 0.5rem; color: var(--muted-foreground); font-size: 0.72rem; line-height: 1.5; }
  .fixture-result small { color: var(--muted-foreground); font: 400 0.62rem 'iA Writer Duo', monospace; }
  .fixture-result.success { border-color: color-mix(in oklch, var(--success) 35%, var(--border)); background: color-mix(in oklch, var(--success) 8%, transparent); }
  .fixture-result.success .fixture-result-icon { border-color: var(--success); color: var(--success); }
  .fixture-result.clear { border-color: color-mix(in oklch, var(--accent) 35%, var(--border)); }

  .diagnostics-panel { grid-column: 1 / -1; }
  .event-list { max-height: 20rem; margin: 0.8rem 0 0; padding: 0; overflow-y: auto; list-style: none; }
  .event-list li { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; align-items: start; gap: 0.7rem; padding: 0.65rem 0; border-top: 1px solid var(--border); }
  .event-dot { width: 0.42rem; height: 0.42rem; margin-top: 0.28rem; border-radius: 50%; background: var(--accent); }
  .event-list li.success .event-dot { background: var(--success); }
  .event-list li.warning .event-dot { background: var(--warning); }
  .event-list li.error .event-dot { background: var(--destructive); }
  .event-list strong { display: block; font-size: 0.72rem; }
  .event-list p { margin: 0.15rem 0 0; color: var(--muted-foreground); font-size: 0.67rem; line-height: 1.45; }
  .event-list time { color: var(--muted-foreground); font: 400 0.6rem 'iA Writer Duo', monospace; }
  .empty-log { min-height: 8rem; display: grid; place-items: center; align-content: center; gap: 0.5rem; color: var(--muted-foreground); font-size: 0.75rem; }
  .diagnostic-foot { display: flex; flex-wrap: wrap; gap: 0.4rem; margin-top: 0.7rem; }
  .diagnostic-foot span { padding: 0.32rem 0.48rem; border-radius: 0.3rem; background: var(--muted); color: var(--muted-foreground); font: 400 0.56rem 'iA Writer Duo', monospace; }

  footer { min-height: 5rem; display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin-top: auto; border-top: 1px solid var(--border); color: var(--muted-foreground); font: 400 0.6rem 'iA Writer Duo', monospace; letter-spacing: 0.05em; }
  footer a { color: var(--accent); text-underline-offset: 0.2em; }

  @media (max-width: 860px) {
    .auth-wrap, .hero, .dashboard-grid { grid-template-columns: 1fr; }
    .auth-wrap { gap: 3rem; }
    .auth-card { width: min(100%, 32rem); }
    .hero { gap: 1.5rem; }
    .pipeline { display: grid; grid-template-columns: 1fr 1fr; gap: 0.8rem; }
    .pipeline > i { display: none; }
    .fixture-panel { grid-column: auto; grid-template-columns: 1fr; }
    .diagnostics-panel { grid-column: auto; }
  }

  @media (max-width: 580px) {
    .app-shell { width: min(calc(100% - 1.2rem), 86rem); }
    .topbar { min-height: 4.4rem; }
    .brand-section, .brand-divider, .account-name, .quiet-button { display: none; }
    .theme-toggle { min-height: 2.4rem; }
    main { padding-top: 3.5rem; }
    h1 { font-size: clamp(2.8rem, 16vw, 4.5rem); }
    .auth-wrap { min-height: auto; }
    .auth-intro { display: none; }
    .auth-card { width: 100%; }
    .failure-banner { grid-template-columns: auto 1fr; }
    .failure-banner button { grid-column: 1 / -1; }
    .pipeline { grid-template-columns: 1fr; }
    .control-row { align-items: stretch; flex-direction: column; }
    .control-row .primary-button, .stop-button { width: 100%; }
    .sample-rate { display: flex; justify-content: space-between; }
    .metric-grid.three, .metric-grid.four { grid-template-columns: repeat(2, 1fr); }
    .detector-details div { grid-template-columns: 5.5rem minmax(0, 1fr); }
    footer { align-items: flex-start; flex-direction: column; justify-content: center; padding: 1rem 0; }
  }
</style>
