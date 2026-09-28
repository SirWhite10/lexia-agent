import { spawn, type ChildProcess } from 'node:child_process';
import { existsSync } from 'node:fs';
import { join, resolve } from 'node:path';

// The host supervises EVE (ADR 0001): EVE runs as a separate process from an
// external, versioned release folder, and the stable Lexia host owns starting,
// health-checking, restarting, and stopping it. In development the host runs
// `eve dev` under this same supervisor so a single `bun run dev` brings the
// whole system up; production points LEXIA_EVE_MODE at `start` against a
// promoted release instead.
//
// The supervisor is a process-level concern, not a model concern: the host
// still never calls a model itself (eve.ts owns that), it only keeps the EVE
// process alive so eve.ts can reach it over HTTP.

const EVE_PORT = Number(process.env.LEXIA_EVE_PORT ?? 2000);
const EVE_URL = (process.env.LEXIA_EVE_URL ?? `http://127.0.0.1:${EVE_PORT}`).replace(/\/$/, '');
const EVE_MODE = process.env.LEXIA_EVE_MODE ?? 'dev';
const EVE_ROOT = resolve(process.env.LEXIA_EVE_ROOT ?? join(process.cwd(), 'my-agent'));
const HEALTH_INTERVAL_MS = 5_000;
const RESTART_BACKOFF_MS = 2_000;

type Supervisor = {
	process: ChildProcess | null;
	stopping: boolean;
	restarts: number;
};

let supervisor: Supervisor | null = null;

function childEnv(): NodeJS.ProcessEnv {
	// The credential belongs to EVE's own process. If the host has it in the
	// environment, pass it through; otherwise EVE inherits whatever it was
	// started with.
	return { ...process.env };
}

function spawnEve(): ChildProcess {
	const args = EVE_MODE === 'start' ? ['start'] : ['dev', '--no-ui'];
	// Prefer a locally installed eve so a release folder is used as-is; fall
	// back to bunx so development works without a project install.
	const useLocal = existsSync(join(EVE_ROOT, 'node_modules', '.bin', 'eve'));
	const command = useLocal ? join(EVE_ROOT, 'node_modules', '.bin', 'eve') : 'bunx';
	const argv = useLocal ? args : ['eve@latest', ...args];

	return spawn(command, argv, {
		cwd: EVE_ROOT,
		env: childEnv(),
		stdio: ['ignore', 'pipe', 'pipe']
	});
}

async function isUp(): Promise<boolean> {
	try {
		const response = await fetch(`${EVE_URL}/`, { signal: AbortSignal.timeout(1_500) });
		return response.status > 0;
	} catch {
		return false;
	}
}

function startSupervisor(): void {
	if (supervisor) return;
	supervisor = { process: null, stopping: false, restarts: 0 };

	const ensureRunning = async () => {
		if (!supervisor || supervisor.stopping) return;
		if (supervisor.process) return;
		if (await isUp()) return; // Someone else owns the port; leave it alone.

		supervisor.process = spawnEve();
		supervisor.process.stdout?.on('data', () => {});
		supervisor.process.stderr?.on('data', (chunk: Buffer) => {
			const line = chunk.toString().trim();
			if (line) console.error(`[eve] ${line}`);
		});
		supervisor.process.on('exit', () => {
			if (!supervisor || supervisor.stopping) return;
			supervisor.process = null;
			supervisor.restarts += 1;
			setTimeout(ensureRunning, RESTART_BACKOFF_MS);
		});
	};

	void ensureRunning();
	setInterval(() => void ensureRunning(), HEALTH_INTERVAL_MS).unref?.();
}

/** Brings EVE up under this host and keeps it alive for the process lifetime. */
export function startEveSupervisor(): void {
	startSupervisor();
}

/** Stops the supervised EVE process; used on host shutdown. */
export function stopEveSupervisor(): void {
	if (!supervisor) return;
	supervisor.stopping = true;
	supervisor.process?.kill('SIGTERM');
	supervisor.process = null;
	supervisor = null;
}
