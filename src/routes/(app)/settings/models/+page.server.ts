import { fail } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import { OpenRouterError, testOpenRouterModel } from '#lib/server/openrouter.js';

export const load: PageServerLoad = () => ({
	openrouterConfigured: Boolean(process.env.OPENROUTER_API_KEY),
	defaultModelId: process.env.OPENROUTER_MODEL ?? ''
});

export const actions: Actions = {
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
