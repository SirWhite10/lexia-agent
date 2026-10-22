import { error, json } from '@sveltejs/kit';
import { mintWakeToken } from '#lib/server/wake-token.js';
import type { RequestHandler } from './$types';

/**
 * A fresh token for the wake service.
 *
 * The page asks for one when the user starts a capture, rather than carrying
 * a token from page load: a capture can start minutes after the page does,
 * and a token is only worth a minute.
 */
export const POST: RequestHandler = ({ locals }) => {
	if (!locals.user) error(401, 'Sign in to use the wake service.');
	const minted = mintWakeToken(locals.user.id);
	if (!minted) {
		error(503, 'The wake service is not configured on this host.');
	}
	return json(minted, { headers: { 'cache-control': 'no-store' } });
};
