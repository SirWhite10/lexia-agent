import { redirect } from '@sveltejs/kit';
import type { Handle } from '@sveltejs/kit/hooks';
import { getUser } from './lib/server/auth';

const LOGIN_PATH = '/login';

export const handle: Handle = async ({ event, resolve }) => {
	event.locals.user = getUser(event);

	// The agent owns the root URL. Keep the auth surface in its own route group
	// so the login layout can evolve independently from the authenticated shell.
	if (event.url.pathname === '/' && !event.locals.user) {
		const returnTo = encodeURIComponent(`${event.url.pathname}${event.url.search}`);
		redirect(303, `${LOGIN_PATH}?returnTo=${returnTo}`);
	}

	return resolve(event);
};
