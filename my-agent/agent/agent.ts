import { createOpenRouter } from '@openrouter/ai-sdk-provider';
import { defineAgent } from 'eve';

// Direct provider rather than a Vercel AI Gateway model id: this model is
// OpenRouter-hosted and the gateway catalog does not carry it. EVE accepts any
// AI SDK LanguageModel here, so the provider is constructed in code and the
// credential stays on the host, never reaching the browser.
const openrouter = createOpenRouter({
	apiKey: process.env.OPENROUTER_API_KEY,
	baseURL: 'https://openrouter.ai/api/v1'
});

export default defineAgent({
	model: openrouter('inclusionai/ling-3.0-flash-sante:free'),
	// EVE looks up context-window size in its AI Gateway catalog, which does not
	// carry this provider-hosted model, and refuses to compile compaction without
	// one. Declaring it here is the documented escape hatch: eve uses the value
	// verbatim and skips the lookup. Ling 3.0 Flash advertises a 128k window.
	modelContextWindowTokens: 128_000
});
