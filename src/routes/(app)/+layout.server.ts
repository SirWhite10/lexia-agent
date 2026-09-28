import { redirect } from '@sveltejs/kit';
import { titleForPath } from '#lib/components/features/app-shell/nav-items.js';
import { listStandingSubAgents } from '#lib/server/agents.js';
import type { LayoutServerLoad } from './$types';

// The authenticated shell is the only surface for signed-in work. Every route
// in this group requires a session; unauthenticated visitors return here after
// login via the same returnTo contract the login actions already enforce.
// The shell itself needs the standing sub-agent list (sidebar + sub-agent area)
// and the header title for the current route on every navigation.
export const load: LayoutServerLoad = ({ locals, url }) => {
	if (!locals.user) {
		const returnTo = encodeURIComponent(`${url.pathname}${url.search}`);
		redirect(303, `/login?returnTo=${returnTo}`);
	}

	const subAgents = listStandingSubAgents();
	return { user: locals.user, subAgents, title: titleForPath(url.pathname, subAgents) };
};
