import { openRouterFetch } from './openrouter.js';
import { providerById, type Provider } from './provider-registry.js';
import { providerKey } from './provider-key.js';

// The model catalogue behind the Settings → Models picker. It is fetched
// server-side and narrowed to display-safe fields: the browser receives names,
// ids and context sizes, never the raw provider payload and never a credential.
// Narrowing here means a new provider field cannot leak by accident when
// someone spreads the raw object somewhere else later.
export type CatalogModel = {
	id: string;
	name: string;
	contextLength: number | null;
	description: string | null;
};

/** Raised for every provider, not just OpenRouter, so the page can show one
 * sentence about whichever service failed rather than a class name. */
export class ModelCatalogError extends Error {}

const CATALOGUE_TIMEOUT_MS = 15_000;

/**
 * Every model a provider lists, sorted by display name. Read fresh on each page
 * load rather than cached in the state database: a stale picker would offer
 * models the account can no longer use.
 *
 * A provider with no catalogue we have verified returns an empty list, and the
 * picker asks for a typed model id instead.
 */
export async function listProviderModels(providerId: string): Promise<CatalogModel[]> {
	const provider = providerById(providerId);
	if (!provider) throw new ModelCatalogError(`Unknown provider “${providerId}”.`);
	if (!provider.catalogue) return [];

	const response =
		provider.catalogue.shape === 'openrouter'
			? await openRouterFetch(provider.catalogue.url, { method: 'GET' })
			: await plainCatalogueFetch(provider.catalogue.url, provider);

	let payload: { data?: unknown };
	try {
		payload = (await response.json()) as { data?: unknown };
	} catch {
		throw new ModelCatalogError(`${provider.label} returned an unreadable model list.`);
	}

	if (!Array.isArray(payload.data)) {
		throw new ModelCatalogError(`${provider.label} returned no model list.`);
	}

	const models: CatalogModel[] = [];
	for (const entry of payload.data) {
		if (!entry || typeof entry !== 'object') continue;
		const record = entry as { id?: unknown; name?: unknown; context_length?: unknown; description?: unknown };
		// A row without a usable id cannot be assigned or requested, so it is
		// dropped rather than offered as an option that would fail on selection.
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

/** Kept as the named OpenRouter path: the dispatch above is what the settings
 * pages use, and this is what a caller that only wants OpenRouter reaches for. */
export async function listOpenRouterModels(): Promise<CatalogModel[]> {
	return listProviderModels('openrouter');
}

/** A plain authenticated GET for providers whose model list is not routed
 * through the OpenRouter helper. Kept local rather than widened into that
 * module, which is about one specific provider. */
async function plainCatalogueFetch(url: string, provider: Provider): Promise<Response> {
	const key = providerKey(provider.id);
	if (!key) throw new ModelCatalogError(`${provider.label} has no key on this host.`);

	try {
		return await fetch(url, {
			method: 'GET',
			cache: 'no-store',
			signal: AbortSignal.timeout(CATALOGUE_TIMEOUT_MS),
			headers: { Authorization: `Bearer ${key}` }
		});
	} catch (error) {
		if (error instanceof DOMException && error.name === 'TimeoutError') {
			throw new ModelCatalogError(`${provider.label} did not answer within 15 seconds.`);
		}
		throw new ModelCatalogError(`Could not reach ${provider.label}. Check host network access and try again.`);
	}
}