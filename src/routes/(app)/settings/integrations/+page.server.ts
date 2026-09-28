import type { PageServerLoad } from './$types';

export const load: PageServerLoad = () => {
	return { openrouterConfigured: Boolean(process.env.OPENROUTER_API_KEY) };
};
