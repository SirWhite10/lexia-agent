import { spawn, type ChildProcess } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';

// The host owns the EVE process: it starts `eve dev` against the authored
// source tree in the working tree, health-checks it, and restarts it if it
// dies, so one command brings the whole system up and the chat never asks the
// operator to babysit a second terminal (ADR 0003).
//
// There is no release mode. `eve build`, `eve start`, and release folders are
// out of scope for V1; changing the agent is an edit under LEXIA_EVE_ROOT and
// EVE reloads it. Do not reintroduce a mode switch here without a new ADR.
//
// The supervisor is a process-level concern, not a model concern: the host
// still never calls a model itself (eve.ts owns that), it only keeps the EVE
// process alive so eve.ts can reach it over HTTP.

const EVE_PORT = Number(process.env.LEXIA_EVE_PORT ?? 2000);
const EVE_URL = (process.env.LEXIA_EVE_URL ?? `http://127.0.0.1:${EVE_PORT}`).replace(/\/$/, '');
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
	const env = { ...process.env };

	// `bun --bun vite dev` runs this host under Bun's Node-compatibility shim,
	// and Bun injects that shim at the front of PATH while pointing NODE at
	// it. EVE is a Node program, and the shim cannot evaluate one of EVE's own
	// internal modules — EVE dies during startup, prints "Failed to evaluate
	// authored module", and the supervisor would restart it forever without
	// ever binding its port. Hand the child a PATH and NODE without the shim
	// so it resolves the real node binary. Do not "simplify" this away.
	if (process.versions.bun && env.NODE) {
		// Bun points NODE at the shim itself, which is also the PATH entry it
		// prepends, so match both that value and its parent directory.
		const shim = resolve(env.NODE);
		const parent = dirname(shim);
		env.PATH = env.PATH?.split(':')
			.filter((entry) => {
				const resolved = resolve(entry || '.');
				return resolved !== shim && resolved !== parent;
			})
			.join(':');
		delete env.NODE;
	}

	// The credential belongs to EVE's own process. If the host has it in the
	// environment, pass it through; otherwise EVE inherits whatever it was
	// started with.
	return env;
}

function spawnEve(): ChildProcess {
	// `--no-ui` keeps EVE a headless server; the host is the only front end.
	const args = ['dev', '--no-ui'];
	// Prefer the eve installed in the agent project so the version this
	// project locked is the one that runs; fall back to bunx so a tree that
	// has not been installed still comes up.
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
