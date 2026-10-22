import { fetchAgentPhrases, setCurrentPhrases, spellPhrases } from './server/agent-phrases';
import { authConfigured, getSession, login, logout, requireSession, session } from './server/auth';
import { fixtureEndpoint } from './server/fixture';
import { keywordEndpoint } from './server/keyword-endpoint';
import { createWebSocketHandler } from './server/wss';
import { DetectorWorker } from './server/worker';

const host = Bun.env.HOST ?? '::';
const port = Number(Bun.env.PORT ?? 8787);
const worker = new DetectorWorker();
const websocket = createWebSocketHandler({ worker });

const server = Bun.serve({
	hostname: host,
	port,
	async fetch(request, bunServer) {
		const url = new URL(request.url);
		if (url.pathname === '/healthz') {
			const health = worker.health();
			return Response.json({ ok: health.ready, worker: health }, { status: health.ready ? 200 : 503 });
		}
		if (url.pathname === '/api/session') {
			if (request.method !== 'GET') return methodNotAllowed('GET');
			return session(request);
		}
		if (url.pathname === '/api/login') return login(request);
		if (url.pathname === '/api/logout') return logout(request);
		if (url.pathname === '/api/fixture') return fixtureEndpoint(request, worker);
		if (url.pathname === '/api/keywords') return keywordEndpoint(request, worker);
		if (url.pathname === '/ws/audio') return websocket.fetch(request, bunServer) ?? new Response('WebSocket upgrade failed', { status: 400 });
		// The project ZIP is not carried in this repository, so there is no
		// route for it. A route that answers 404 for a file that is deliberately
		// absent is a route that looks like it works and does not.
		// The browser interface for the wake lab lives in the agent host now,
		// which is where the signed-in user is. This service answers the
		// detector's endpoints and nothing else.
		if (url.pathname === '/') {
			return Response.json({ ok: true, service: 'agent-wake', websocket: '/ws/audio' });
		}
		return new Response('Not found', { status: 404 });
	},
	websocket
});

function methodNotAllowed(allow: string): Response {
	return new Response('Method not allowed', { status: 405, headers: { Allow: allow } });
}

void worker
	.start()
	.then(async () => {
		// The agent's phrases are the host's to declare. They are spelled into
		// the detector's keyword file here, because the detector matches
		// pre-spelled keywords and the host does not know the model's
		// vocabulary.
		const { phrases, unavailable } = await fetchAgentPhrases();
		if (unavailable) {
			// The detector still runs. Detections will carry no command, and
			// this is why.
			console.warn(`[host] ${unavailable}`);
			return;
		}
		setCurrentPhrases(phrases);
		const { added, refused } = await spellPhrases(phrases);
		for (const entry of refused) {
			console.warn(`[host] the detector cannot spell "${entry.phrase}": ${entry.reason}`);
		}
		if (added.length > 0) {
			console.log(`[host] spelled ${added.length} wake phrase(s) into the keyword file: ${added.join(', ')}`);
			await worker.restart();
		}
	})
	.catch((error: unknown) => {
		console.error(`[host] detector worker unavailable: ${error instanceof Error ? error.message : String(error)}`);
	});

for (const signal of ['SIGINT', 'SIGTERM'] as const) {
	process.on(signal, () => {
		void worker.stop().finally(() => server.stop(true));
	});
}

console.log(`Serving the agent wake-word test at http://${host}:${server.port}/`);
console.log(`Public hostname: ${Bun.env.PUBLIC_ORIGIN ?? 'https://lexia.dlx.studio'}`);
console.log(`Authentication configured: ${authConfigured()}`);

export { getSession, requireSession };
