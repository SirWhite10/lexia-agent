import { OpenRouterError, openRouterFetch } from './openrouter.js';

// The OpenRouter model catalogue behind the Settings → Models picker. It is
// fetched server-side and narrowed to display-safe fields: the browser receives
// names, ids, and context sizes, never the raw provider payload, and never a
// credential. Narrowing here means a new provider field cannot leak by accident
// when someone spreads the raw object somewhere else later.
const MODELS_URL = 'https://openrouter.ai/api/v1/models';

export type CatalogModel = {
	id: string;
	name: string;
	contextLength: number | null;
	description: string | null;
};

/**
 * Every model OpenRouter lists, sorted by display name. The catalogue is a
 * single unauthenticated-cost GET, so it is read fresh on each page load rather
 * than cached in the state database: a stale picker would silently offer models
 * the account can no longer use.
 */
export async function listOpenRouterModels(): Promise<CatalogModel[]> {
	const response = await openRouterFetch(MODELS_URL, { method: 'GET' });

	let payload: { data?: unknown };
	try {
		payload = await response.json();
	} catch {
		throw new OpenRouterError('OpenRouter returned an unreadable model catalogue.');
	}

	if (!Array.isArray(payload.data)) {
		throw new OpenRouterError('OpenRouter returned no model catalogue.');
	}

	const models: CatalogModel[] = [];
	for (const entry of payload.data) {
		if (!entry || typeof entry !== 'object') continue;
		const record = entry as { id?: unknown; name?: unknown; context_length?: unknown; description?: unknown };
		// A row without a usable id cannot be assigned or requested, so it is
		// dropped rather than shown as an option that would fail on selection.
		if (typeof record.id !== 'string' || record.id.length === 0) continue;
		models.push({
			id: record.id,
			name: typeof record.name === 'string' && record.name.length > 0 ? record.name : record.id,
			contextLength: typeof record.context_length === 'number' ? record.context_length : null,
			description: typeof record.description === 'string' ? record.description : null
		});
	}

	models.sort((left, right) => left.name.localeCompare(right.name));
	return models;
}