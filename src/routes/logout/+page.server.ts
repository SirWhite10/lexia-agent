import { redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

// Session teardown endpoint shared by every surface. This SvelteKit version
// dispatches form actions only from leaf +page.server.ts nodes, so logout lives
// on its own route instead of the (app) layout. GET just lands on login.
export const load: PageServerLoad = () => {
	redirect(303, '/login');
};

export const actions: Actions = {
	default: ({ cookies }) => {
		cookies.delete('lexia_session', { path: '/' });
		redirect(303, '/login');
	},
};
