const CHAT_COMPLETIONS_URL = 'https://openrouter.ai/api/v1/chat/completions';
const TEST_PROMPT = 'Reply with exactly: OpenRouter connection successful.';

export type OpenRouterTestResult = {
	model: string;
	reply: string;
	usage: {
		promptTokens: number | null;
		completionTokens: number | null;
		totalTokens: number | null;
	};
};

export class OpenRouterError extends Error {
	constructor(message: string) {
		super(message);
		this.name = 'OpenRouterError';
	}
}

/**
 * The one authenticated request path: resolves the credential, bounds the wait,
 * and translates every provider failure into the OpenRouterError wording the
 * settings pages show. Callers only decode the body, so no caller can invent a
 * different message for the same outage or leak a raw provider error.
 */
export async function openRouterFetch(url: string, init: RequestInit = {}, timeoutMs = 25_000): Promise<Response> {
	const apiKey = process.env.OPENROUTER_API_KEY;
	if (!apiKey) {
		throw new OpenRouterError('OpenRouter is not configured on the Lexosa host.');
	}

	let response: Response;
	try {
		response = await fetch(url, {
			...init,
			cache: 'no-store',
			signal: AbortSignal.timeout(timeoutMs),
			headers: { Authorization: `Bearer ${apiKey}`, ...init.headers }
		});
	} catch (error) {
		if (error instanceof DOMException && error.name === 'TimeoutError') {
			throw new OpenRouterError(`OpenRouter did not respond within ${Math.round(timeoutMs / 1000)} seconds.`);
		}
		throw new OpenRouterError('Could not connect to OpenRouter. Check host network access and try again.');
	}

	if (!response.ok) {
		if (response.status === 401 || response.status === 403) {
			throw new OpenRouterError('OpenRouter rejected the API key. Check the host configuration.');
		}
		if (response.status === 402) {
			throw new OpenRouterError('OpenRouter reports that this account has insufficient credits.');
		}
		if (response.status === 429) {
			throw new OpenRouterError('OpenRouter rate-limited this request. Wait a moment and try again.');
		}
		throw new OpenRouterError(`OpenRouter returned an error (HTTP ${response.status}). Check the model ID and account.`);
	}

	return response;
}


/**
 * Makes one small, non-streaming provider request for the Models settings panel.
 * The server owns the credential and deliberately returns only display-safe result data.
 */
export async function testOpenRouterModel(modelId: string): Promise<OpenRouterTestResult> {
	const apiKey = process.env.OPENROUTER_API_KEY;
	if (!apiKey) {
		throw new OpenRouterError('OpenRouter is not configured on the Lexosa host.');
	}

	let response: Response;
	try {
		response = await fetch(CHAT_COMPLETIONS_URL, {
			method: 'POST',
			cache: 'no-store',
			signal: AbortSignal.timeout(25_000),
			headers: {
				Authorization: `Bearer ${apiKey}`,
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({
				model: modelId,
				messages: [{ role: 'user', content: TEST_PROMPT }],
				max_tokens: 48,
				temperature: 0
			})
		});
	} catch (error) {
		if (error instanceof DOMException && error.name === 'TimeoutError') {
			throw new OpenRouterError('OpenRouter did not respond within 25 seconds.');
		}
		throw new OpenRouterError('Could not connect to OpenRouter. Check host network access and try again.');
	}

	if (!response.ok) {
		if (response.status === 401 || response.status === 403) {
			throw new OpenRouterError('OpenRouter rejected the API key. Check the host configuration.');
		}
		if (response.status === 402) {
			throw new OpenRouterError('OpenRouter reports that this account has insufficient credits.');
		}
		if (response.status === 429) {
			throw new OpenRouterError('OpenRouter rate-limited this request. Wait a moment and try again.');
		}
		throw new OpenRouterError(`OpenRouter returned an error (HTTP ${response.status}). Check the model ID and account.`);
	}

	let payload: {
		model?: unknown;
		choices?: Array<{ message?: { content?: unknown } }>;
		usage?: { prompt_tokens?: unknown; completion_tokens?: unknown; total_tokens?: unknown };
	};
	try {
		payload = await response.json();
	} catch {
		throw new OpenRouterError('OpenRouter returned an unreadable response.');
	}

	const reply = payload.choices?.[0]?.message?.content;
	if (typeof reply !== 'string') {
		throw new OpenRouterError('OpenRouter returned no text reply for this model.');
	}
	const tokenCount = (value: unknown) => (typeof value === 'number' ? value : null);
	return {
		model: typeof payload.model === 'string' ? payload.model : modelId,
		reply,
		usage: {
			promptTokens: tokenCount(payload.usage?.prompt_tokens),
			completionTokens: tokenCount(payload.usage?.completion_tokens),
			totalTokens: tokenCount(payload.usage?.total_tokens)
		}
	};
}


const EMBEDDINGS_URL = 'https://openrouter.ai/api/v1/embeddings';

/**
 * One server-side embedding request. The credential never leaves the host and
 * only the vector crosses the boundary; callers store the model id beside the
 * vector so a model change can re-index instead of mixing vector spaces.
 */
export async function embedOpenRouterText(model: string, text: string): Promise<number[]> {
	const apiKey = process.env.OPENROUTER_API_KEY;
	if (!apiKey) {
		throw new OpenRouterError('OpenRouter is not configured on the Lexosa host.');
	}

	let response: Response;
	try {
		response = await fetch(EMBEDDINGS_URL, {
			method: 'POST',
			cache: 'no-store',
			signal: AbortSignal.timeout(25_000),
			headers: {
				Authorization: `Bearer ${apiKey}`,
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({ model, input: text })
		});
	} catch (error) {
		if (error instanceof DOMException && error.name === 'TimeoutError') {
			throw new OpenRouterError('OpenRouter did not respond to the embedding request within 25 seconds.');
		}
		throw new OpenRouterError('Could not connect to OpenRouter for embeddings. Check host network access and try again.');
	}

	if (!response.ok) {
		if (response.status === 401 || response.status === 403) {
			throw new OpenRouterError('OpenRouter rejected the API key for embeddings. Check the host configuration.');
		}
		if (response.status === 402) {
			throw new OpenRouterError('OpenRouter reports that this account has insufficient credits for embeddings.');
		}
		if (response.status === 429) {
			throw new OpenRouterError('OpenRouter rate-limited the embedding request. Wait a moment and try again.');
		}
		throw new OpenRouterError(`OpenRouter embedding request failed (HTTP ${response.status}).`);
	}

	let payload: { data?: Array<{ embedding?: unknown }> };
	try {
		payload = await response.json();
	} catch {
		throw new OpenRouterError('OpenRouter returned an unreadable embedding response.');
	}

	const embedding = payload.data?.[0]?.embedding;
	if (!Array.isArray(embedding) || embedding.some((value) => typeof value !== 'number')) {
		throw new OpenRouterError('OpenRouter returned no usable embedding vector.');
	}
	return embedding as number[];
}

/**
 * One non-streaming chat completion returning the assistant's text. The
 * credential stays on the host and only the reply crosses the boundary; used
 * by reflection to distill facts and traits, and by any future model-backed
 * capability. Callers own prompt safety and output validation.
 */
export async function completeOpenRouter(
	model: string,
	system: string,
	user: string,
	maxTokens = 800
): Promise<string> {
	const apiKey = process.env.OPENROUTER_API_KEY;
	if (!apiKey) {
		throw new OpenRouterError('OpenRouter is not configured on the Lexosa host.');
	}

	let response: Response;
	try {
		response = await fetch(CHAT_COMPLETIONS_URL, {
			method: 'POST',
			cache: 'no-store',
			signal: AbortSignal.timeout(40_000),
			headers: {
				Authorization: `Bearer ${apiKey}`,
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({
				model,
				messages: [
					{ role: 'system', content: system },
					{ role: 'user', content: user }
				],
				max_tokens: maxTokens,
				temperature: 0.2
			})
		});
	} catch (error) {
		if (error instanceof DOMException && error.name === 'TimeoutError') {
			throw new OpenRouterError('OpenRouter did not respond to the completion within 40 seconds.');
		}
		throw new OpenRouterError('Could not connect to OpenRouter. Check host network access and try again.');
	}

	if (!response.ok) {
		if (response.status === 401 || response.status === 403) {
			throw new OpenRouterError('OpenRouter rejected the API key. Check the host configuration.');
		}
		if (response.status === 402) {
			throw new OpenRouterError('OpenRouter reports that this account has insufficient credits.');
		}
		if (response.status === 429) {
			throw new OpenRouterError('OpenRouter rate-limited this request. Wait a moment and try again.');
		}
		throw new OpenRouterError(`OpenRouter returned an error (HTTP ${response.status}).`);
	}

	let payload: { choices?: Array<{ message?: { content?: unknown } }> };
	try {
		payload = await response.json();
	} catch {
		throw new OpenRouterError('OpenRouter returned an unreadable response.');
	}

	const content = payload.choices?.[0]?.message?.content;
	if (typeof content !== 'string') {
		throw new OpenRouterError('OpenRouter returned no text for this completion.');
	}
	return content;
}
