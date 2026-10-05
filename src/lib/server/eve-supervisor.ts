import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
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

type SupervisorState = {
	supervisor: Supervisor | null;
	healthTimer: NodeJS.Timeout | null;
	evePid: number | null;
	exitHookInstalled: boolean;
};
// Vite replaces this module's instance on every server-side edit. With
// module-level state a reload orphans the running child, leaks a health timer,
// and leaves `restartEve` reporting a restart that never happened — so the
// state lives on globalThis and a hot reload adopts the process it started.
const state = ((globalThis as typeof globalThis & { __lexosaEve?: SupervisorState }).__lexosaEve ??= {
	supervisor: null,
	healthTimer: null,
	evePid: null,
	exitHookInstalled: false
});

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

/** EVE daemonizes: the process this host spawns forks a listener that puts
 * itself in its own process group, so neither a signal to the spawned pid nor
 * to its group reaches the process that actually holds the port. Termination
 * therefore has two parts — signal the group this host started, then signal
 * whoever owns the port. Without the second part the next start finds the old
 * agent still answering, concludes someone else owns the port, and quietly
 * does nothing while the caller reports a restart. */
function portOwnerPid(port: number): number | null {
	try {
		const listing = execFileSync('ss', ['-ltnpH', `sport = :${port}`], { encoding: 'utf8' });
		return Number(/pid=(\d+)/.exec(listing)?.[1] ?? NaN) || null;
	} catch {
		return null; // No `ss`, or nothing is listening.
	}
}

function terminateEve(): void {
	if (state.evePid !== null) {
		try {
			process.kill(-state.evePid, 'SIGTERM');
		} catch {
			// The group is already gone, or was never created.
		}
		state.evePid = null;
	}

	const owner = portOwnerPid(EVE_PORT);
	if (owner === null) return;
	try {
		process.kill(owner, 'SIGTERM');
	} catch {
		// It exited between the lookup and the signal.
	}
}

function delay(ms: number): Promise<void> {
	const { promise, resolve } = Promise.withResolvers<void>();
	setTimeout(resolve, ms);
	return promise;
}

function spawnEve(): ChildProcess {
	// `--no-ui` keeps EVE a headless server; the host is the only front end.
	// EVE binds $PORT and then 2000, never this host's own idea of where EVE
	// lives, so the documented LEXIA_EVE_PORT has to be handed over explicitly:
	// without it two hosts on one machine contend for the same agent port.
	const args = ['dev', '--no-ui', '--port', String(EVE_PORT)];
	// Prefer the eve installed in the agent project so the version this
	// project locked is the one that runs; fall back to bunx so a tree that
	// has not been installed still comes up.
	const useLocal = existsSync(join(EVE_ROOT, 'node_modules', '.bin', 'eve'));
	const command = useLocal ? join(EVE_ROOT, 'node_modules', '.bin', 'eve') : 'bunx';
	const argv = useLocal ? args : ['eve@latest', ...args];
	return spawn(command, argv, {
		cwd: EVE_ROOT,
		env: childEnv(),
		// Its own process group, so a terminal interrupt cannot half-kill the
		// tree and `terminateEve` can signal the agent this host started.
		detached: true,
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
	if (state.supervisor) return;
	state.supervisor = { process: null, stopping: false, restarts: 0 };

	const ensureRunning = async () => {
		if (!state.supervisor || state.supervisor.stopping) return;
		if (state.supervisor.process) return;
		if (await isUp()) return; // Someone else owns the port; leave it alone.

		state.supervisor.process = spawnEve();
		state.evePid = state.supervisor.process.pid ?? null;
		state.supervisor.process.stdout?.on('data', () => {});
		state.supervisor.process.stderr?.on('data', (chunk: Buffer) => {
			const line = chunk.toString().trim();
			if (line) console.error(`[eve] ${line}`);
		});
		state.supervisor.process.on('exit', () => {
			if (!state.supervisor || state.supervisor.stopping) return;
			state.supervisor.process = null;
			state.supervisor.restarts += 1;
			setTimeout(ensureRunning, RESTART_BACKOFF_MS);
		});
	};

	void ensureRunning();
	state.healthTimer = setInterval(() => void ensureRunning(), HEALTH_INTERVAL_MS);
	state.healthTimer.unref?.();
}

/** Brings EVE up under this host and keeps it alive for the process lifetime. */
export function startEveSupervisor(): void {
	startSupervisor();

	// The agent sits in its own process group, so Ctrl-C in the terminal never
	// reaches it: the host takes it down as it exits instead.
	if (!state.exitHookInstalled) {
		state.exitHookInstalled = true;
		process.once('exit', () => {
			terminateEve();
		});
	}
}

/** Restarts the supervised EVE process so it picks up configuration it captured
 * at module load, such as the OpenRouter credential. This interrupts whatever
 * EVE was running: it belongs to a deliberate operator action, never to a
 * background write. */
export async function restartEve(): Promise<void> {
	stopEveSupervisor();
	// A new agent refuses to start while anything still answers on the port, so
	// wait for the port to free instead of reporting a restart that quietly did
	// not happen.
	for (let attempt = 0; attempt < 40 && (await isUp()); attempt += 1) {
		await delay(250);
	}
	startSupervisor();
}

/** Stops the supervised EVE process; used on host shutdown. */
export function stopEveSupervisor(): void {
	if (!state.supervisor) return;
	state.supervisor.stopping = true;
	terminateEve();
	state.supervisor.process = null;
	state.supervisor = null;

	if (state.healthTimer) {
		clearInterval(state.healthTimer);
		state.healthTimer = null;
	}
}
