import { fail } from '@sveltejs/kit';
import { restartEve } from '#lib/server/eve-supervisor.js';
import { MODALITY_LABELS, PROVIDERS, providerBaseUrl, providerById } from '#lib/server/provider-registry.js';
import { providerKeySource, saveProviderKey } from '#lib/server/provider-key.js';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = () => ({
	// Only the source is ever returned. No part of a credential reaches the client,
	// which is the promise this page and the key store both rest on.
	providers: PROVIDERS.map((provider) => ({
		id: provider.id,
		label: provider.label,
		envVar: provider.envVar,
		summary: provider.summary,
		modalities: provider.modalities.map((modality) => MODALITY_LABELS[modality]),
		hasCatalogue: provider.catalogue !== null,
		// Only for self-hosted providers: where this host will actually look.
		baseUrlEnv: provider.baseUrlEnv ?? null,
		baseUrl: providerBaseUrl(provider),
		keySource: providerKeySource(provider.id)
	}))
});

export const actions: Actions = {
	saveKey: async ({ request }) => {
		const form = await request.formData();
		const provider = providerById(String(form.get('provider') ?? ''));
		if (!provider) return fail(400, { provider: '', error: 'That provider is not one Lexosa knows.' });

		const apiKey = String(form.get('apiKey') ?? '');
		let changed = false;
		try {
			changed = saveProviderKey(provider.id, apiKey);
		} catch (error) {
			return fail(400, {
				provider: provider.id,
				error: error instanceof Error ? error.message : 'That key could not be saved.'
			});
		}

		// Only OpenRouter's credential reaches EVE, and only a live change is worth
		// a restart: EVE captured the old one when it loaded.
		const restarted = provider.id === 'openrouter' && changed;
		if (restarted) await restartEve();

		return {
			provider: provider.id,
			keySource: providerKeySource(provider.id),
			restarted
		};
	}
};