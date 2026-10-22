import { getSession } from './auth';
import { fixtureFor } from './fixtures';
import { DetectorWorker } from './worker';

export async function fixtureEndpoint(request: Request, worker: DetectorWorker): Promise<Response> {
	if (request.method !== 'POST') return new Response('Method not allowed', { status: 405, headers: { Allow: 'POST' } });
	if (!getSession(request)) return Response.json({ error: 'unauthorized', message: 'Sign in before running the WAV fixture.' }, { status: 401 });
	if (!worker.isReady()) return Response.json({ error: 'model_unavailable', message: 'The detector model is not ready.' }, { status: 503 });
	let fixtureId = 'agent-wake';
	try {
		const body = (await request.json()) as { fixtureId?: unknown };
		if (typeof body.fixtureId === 'string') fixtureId = body.fixtureId;
	} catch {
		// The default allowlisted fixture is used for an empty body.
	}
	const fixture = fixtureFor(fixtureId);
	try {
		const result = await worker.runFixture(fixture.id, fixture.path);
		return Response.json({ ok: true, fixtureId: fixture.id, result: normalizeFixtureResult(result) });
	} catch (error) {
		return Response.json({ error: 'fixture_failed', message: error instanceof Error ? error.message : 'The WAV fixture failed.' }, { status: 500 });
	}
}

function normalizeFixtureResult(result: Record<string, unknown>): Record<string, unknown> {
	const wakes = Array.isArray(result.wakes) ? result.wakes : [];
	const firstWake = wakes[0];
	const wake = firstWake && typeof firstWake === 'object' ? firstWake as Record<string, unknown> : {};
	const stats = result.stats && typeof result.stats === 'object' ? result.stats as Record<string, unknown> : {};
	return {
		detected: result.detected === true,
		keyword: typeof wake.keyword === 'string' ? wake.keyword : null,
		timestamp: typeof wake.timestamp === 'number' ? wake.timestamp : null,
		durationMs: typeof stats.duration_ms === 'number' ? stats.duration_ms : null,
		stats
	};
}
