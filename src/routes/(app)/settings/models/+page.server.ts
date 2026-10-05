import { fail } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import { listProviderModels, ModelCatalogError, type CatalogModel } from '#lib/server/model-catalog.js';
import { assignModel, createUseCase, listUseCases, modelForUseCase, removeUseCase } from '#lib/server/model-routes.js';
import { MODALITY_LABELS, MODALITY_ORDER, providersFor, type Modality } from '#lib/server/provider-registry.js';
import { OpenRouterError, testOpenRouterModel } from '#lib/server/openrouter.js';
import { providerKeySource } from '#lib/server/provider-key.js';

type ProviderCard = {
	id: string;
	label: string;
	modalities: string[];
	/** False when the provider publishes no list we have verified: its model id is
	 * then typed by hand rather than picked from a list. */
	hasCatalogue: boolean;
	/** Whether that catalogue actually lists models for the modality of this row.
	 * OpenRouter serves transcription models, but its /models list is text-only. */
	coversModality: boolean;
	configured: boolean;
};

async function describeProviders(modality: Modality): Promise<ProviderCard[]> {
	// Every provider that serves this modality is offered, configured or not: the
	// operator may be about to add a key in Settings → Providers.
	return await Promise.all(
		providersFor(modality).map(async (provider) => ({
			id: provider.id,
			label: provider.label,
			modalities: provider.modalities.map((served) => MODALITY_LABELS[served]),
			hasCatalogue: provider.catalogue !== null,
			coversModality: provider.catalogueModalities?.includes(modality) ?? false,
			configured: providerKeySource(provider.id) !== 'none'
		}))
	);
}

export const load: PageServerLoad = async () => {
	// Catalogues are fetched per provider, and a provider is only asked when it
	// has both a key and a list to give: a missing credential is a normal state,
	// not a load failure, and the page says where to add one.
	const catalogues: Record<string, CatalogModel[]> = {};
	const catalogueErrors: Record<string, string> = {};
	const providers: Record<string, ProviderCard[]> = {};

	for (const modality of MODALITY_ORDER) {
		const cards = await describeProviders(modality);
		providers[modality] = cards;

		for (const card of cards) {
			if (catalogues[card.id] || !card.configured || !card.hasCatalogue) continue;
			try {
				catalogues[card.id] = await listProviderModels(card.id);
			} catch (error) {
				catalogueErrors[card.id] =
					error instanceof OpenRouterError || error instanceof ModelCatalogError
						? error.message
						: `${card.label} returned no model list.`;
			}
		}
	}

	return {
		modalityLabels: MODALITY_LABELS,
		modalityOrder: MODALITY_ORDER,
		providers,
		catalogues,
		catalogueErrors,
		// The test panel previews whatever the router would actually run, so the
		// Always assignment wins; OPENROUTER_MODEL stays the fallback for an
		// operator who configures models only through the environment.
		defaultModelId: modelForUseCase('always') ?? process.env.OPENROUTER_MODEL ?? '',
		useCases: listUseCases()
	};
};

export const actions: Actions = {
	assign: async ({ request }) => {
		const formData = await request.formData();
		const useCase = String(formData.get('useCase') ?? '').trim();
		const modelId = String(formData.get('modelId') ?? '').trim();
		const providerId = String(formData.get('providerId') ?? '').trim();

		if (!useCase || modelId.length > 200) {
			return fail(400, { assignError: 'Choose a model for this use-case.' });
		}

		try {
			return { assigned: true, assignment: assignModel(useCase, modelId, providerId) };
		} catch (error) {
			return fail(400, {
				useCase,
				assignError: error instanceof Error ? error.message : 'That model could not be assigned.'
			});
		}
	},

	createUseCase: async ({ request }) => {
		const formData = await request.formData();
		const label = String(formData.get('label') ?? '');
		const requested = String(formData.get('modality') ?? 'text');
		const modality = MODALITY_ORDER.find((entry) => entry === requested) ?? 'text';

		try {
			return { created: true, assignment: createUseCase(label, modality) };
		} catch (error) {
			return fail(400, {
				label,
				createError: error instanceof Error ? error.message : 'That use-case could not be created.'
			});
		}
	},

	removeUseCase: async ({ request }) => {
		const useCase = String((await request.formData()).get('useCase') ?? '');
		try {
			removeUseCase(useCase);
			return { removed: true };
		} catch (error) {
			return fail(400, {
				error: error instanceof Error ? error.message : 'That use-case could not be removed.'
			});
		}
	},

	test: async ({ request }) => {
		const formData = await request.formData();
		const modelId = String(formData.get('modelId') ?? '').trim();

		if (!modelId || modelId.length > 160) {
			return fail(400, { modelId, error: 'Enter a model ID of 1–160 characters.' });
		}

		try {
			return { success: true, result: await testOpenRouterModel(modelId) };
		} catch (error) {
			return fail(502, {
				modelId,
				error:
					error instanceof OpenRouterError
						? error.message
						: 'The OpenRouter test failed unexpectedly.'
			});
		}
	}
};