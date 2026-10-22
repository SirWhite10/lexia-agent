import { error, json } from '@sveltejs/kit';
import { mintWakeToken } from '#lib/server/wake-token.js';
import { wakeServiceUrl } from '#lib/server/wake-service.js';
import type { RequestHandler } from './$types';

/**
 * Run the bundled WAV through the detector, on the service's behalf.
 *
 * The page cannot call the service's `/api/fixture` itself: the service is on
 * another origin, and a request carrying an `Authorization` header is
 * preflighted, which the service does not answer. Proxying through the host
 * keeps the browser on one origin and keeps the token on this side of it.
 */
export const POST: RequestHandler = async ({ locals }) => {
	if (!locals.user) error(401, 'Sign in to use the wake service.');
	const minted = mintWakeToken(locals.user.id);
	if (!minted) {
		error(503, 'The wake service is not configured on this host.');
	}

	let response: Response;
	try {
		response = await fetch(`${wakeServiceUrl()}/api/fixture`, {
			method: 'POST',
			headers: { 'content-type': 'application/json', authorization: `Bearer ${minted.token}` },
			body: JSON.stringify({ fixtureId: 'agent-wake' })
		});
	} catch {
		error(502, 'The wake service could not be reached.');
	}

	const body: unknown = await response.json().catch(() => ({}));
	return json(body, { status: response.status });
};
