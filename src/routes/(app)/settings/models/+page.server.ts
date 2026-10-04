import { fail } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import { listOpenRouterModels, type CatalogModel } from '#lib/server/model-catalog.js';
import { assignModel, createUseCase, listUseCases, modelForUseCase, removeUseCase } from '#lib/server/model-routes.js';
import { OpenRouterError, testOpenRouterModel } from '#lib/server/openrouter.js';
import { openRouterKeySource } from '#lib/server/provider-key.js';

export const load: PageServerLoad = async () => {
	const openrouterConfigured = openRouterKeySource() !== 'none';

	// A missing credential is a normal state, not a load failure: the page says
	// where to add a key instead of rendering an empty picker or a server error.
	let catalogue: CatalogModel[] = [];
	let catalogueError: string | null = null;
	if (openrouterConfigured) {
		try {
			catalogue = await listOpenRouterModels();
		} catch (error) {
			catalogueError =
				error instanceof OpenRouterError ? error.message : 'The OpenRouter model catalogue could not be loaded.';
		}
	}

	return {
		openrouterConfigured,
		// The test panel previews whatever the router would actually run, so the
		// Always assignment wins; OPENROUTER_MODEL stays the fallback for an
		// operator who configures models only through the environment.
		defaultModelId: modelForUseCase('always') ?? process.env.OPENROUTER_MODEL ?? '',
		useCases: listUseCases(),
		catalogue,
		catalogueError
	};
};

export const actions: Actions = {
	assign: async ({ request }) => {
		const formData = await request.formData();
		const useCase = String(formData.get('useCase') ?? '').trim();
		const modelId = String(formData.get('modelId') ?? '').trim();

		if (!useCase || modelId.length > 200) {
			return fail(400, { assignError: 'Choose a model from the catalogue.' });
		}

		try {
			// The assignment is echoed back so the row updates from this action's
			// own result rather than re-running the load and the catalogue fetch.
			return { assigned: true, assignment: assignModel(useCase, modelId) };
		} catch (error) {
			return fail(400, {
				useCase,
				assignError: error instanceof Error ? error.message : 'That model could not be assigned.'
			});
		}
	},

	createUseCase: async ({ request }) => {
		const label = String((await request.formData()).get('label') ?? '');

		try {
			return { created: true, useCase: createUseCase(label) };
		} catch (error) {
			return fail(400, {
				label,
				createError: error instanceof Error ? error.message : 'That use-case could not be created.'
			});
		}
	},

	removeUseCase: async ({ request }) => {
		const useCase = String((await request.formData()).get('useCase') ?? '').trim();

		try {
			removeUseCase(useCase);
			return { removed: true, useCase };
		} catch (error) {
			return fail(400, {
				removeError: error instanceof Error ? error.message : 'That use-case could not be removed.'
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