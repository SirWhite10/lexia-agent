import { getSession } from './auth';
import { fixtureFor } from './fixtures';
import { appendKeywordLine, normalizeName, readKeywordLines, tokenizeName } from './keywords';
import type { DetectorWorker } from './worker';

/**
 * Assign a wake word to an agent: spell the name with the model's vocabulary,
 * append it to the keyword file, restart the detector so it reloads, and prove
 * the model still works on the bundled fixture.
 */
export async function keywordEndpoint(request: Request, worker: DetectorWorker): Promise<Response> {
	if (request.method !== 'POST') return new Response('Method not allowed', { status: 405, headers: { Allow: 'POST' } });
	if (!getSession(request)) return Response.json({ error: 'unauthorized', message: 'Sign in before changing the wake word.' }, { status: 401 });
	if (!worker.isReady()) return Response.json({ error: 'model_unavailable', message: 'The detector model is not ready.' }, { status: 503 });
	if (worker.hasActiveSessions()) {
		return Response.json({ error: 'stream_active', message: 'Stop the microphone stream before changing the wake word.' }, { status: 409 });
	}

	let body: { name?: unknown };
	try {
		body = (await request.json()) as { name?: unknown };
	} catch {
		return Response.json({ error: 'invalid_body', message: 'Send a JSON body with a name.' }, { status: 400 });
	}

	let name: string;
	let tokens: string;
	try {
		name = normalizeName(body.name);
		tokens = await tokenizeName(name);
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		return Response.json({ error: 'invalid_name', message }, { status: 422 });
	}

	const added = await appendKeywordLine(tokens);
	await worker.restart();
	const fixture = fixtureFor('agent-wake');
	if (!fixture) return Response.json({ error: 'fixture_missing', message: 'The reference WAV fixture is not registered.' }, { status: 500 });
	if (!worker.isReady()) {
		return Response.json(
			{ error: 'reload_failed', message: worker.health().message ?? 'The detector did not come back after reloading.' },
			{ status: 500 }
		);
	}

	let verification: Record<string, unknown>;
	try {
		const result = await worker.runFixture(fixture.id, fixture.path);
		verification = { fixtureId: fixture.id, detected: result.detected === true, ok: result.detected === true };
	} catch (error) {
		verification = { fixtureId: fixture.id, detected: false, ok: false, message: error instanceof Error ? error.message : String(error) };
	}

	return Response.json({
		ok: verification.ok === true,
		name,
		tokens,
		added,
		keywords: await readKeywordLines(),
		verification,
		message: verification.ok === true
			? `${name} is loaded. Speak "${name}" into the microphone to confirm it fires.`
			: 'The keyword was added, but the detector no longer detects the reference fixture. Check the model before relying on it.'
	});
}
