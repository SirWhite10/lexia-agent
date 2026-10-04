import type { Actions, PageServerLoad } from './$types';
import { restartEve } from '#lib/server/eve-supervisor.js';
import { openRouterKeySource, saveOpenRouterKey } from '#lib/server/provider-key.js';

export const load: PageServerLoad = () => {
	const source = openRouterKeySource();
	return {
		openrouterConfigured: source !== 'none',
		// Only ever the source, never any part of the key: the page promises the
		// credential stays server-side.
		openRouterKeySource: source
	};
};

export const actions: Actions = {
	saveKey: async ({ request }) => {
		const apiKey = String((await request.formData()).get('apiKey') ?? '');

		// EVE read the credential once, at module load, and inherited this
		// process's environment. A stored key — added or removed — is only live
		// once EVE has restarted; an environment-provided key already reached
		// EVE at spawn and must not disturb a running turn.
		const eveRestarted = saveOpenRouterKey(apiKey);
		if (eveRestarted) await restartEve();

		return { saved: true, source: openRouterKeySource(), eveRestarted };
	}
};