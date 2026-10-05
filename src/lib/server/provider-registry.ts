/**
 * Every outside service Lexosa can be configured to call, and what each one is
 * good for. This is the single source of truth: the Providers page renders from
 * it, the key store files credentials by its ids, and the model picker offers
 * only providers that serve the modality of the row being assigned.
 *
 * A provider with `catalogue: null` publishes no model list we have verified, so
 * its model id is typed by hand. That is deliberate: an unverified endpoint that
 * answers with a 404 is worse than no list at all, because it looks like the
 * provider is broken rather than the app being unsure.
 */

export type Modality = 'text' | 'speech' | 'transcription' | 'image' | 'video';

export type ProviderCatalogue = {
	url: string;
	/** `openrouter` returns context length and descriptions; `plain` returns the
	 * `{ id, name }` list every OpenAI-compatible service exposes. */
	shape: 'openrouter' | 'plain';
};

export type Provider = {
	/** Also the credential file name and the id used in form posts. */
	id: string;
	label: string;
	/** Read at host boot and preferred over the stored key for the process life.
	 * A self-hosted provider often needs none; it is still declared so the card
	 * can say where a key would go. */
	envVar: string;
	/** One line for the provider card: what this key buys. */
	summary: string;
	modalities: Modality[];
	catalogue: ProviderCatalogue | null;
	/** Which modalities the catalogue actually lists. OpenRouter's /models is a
	 * text catalogue: the audio models it serves for transcription cannot be picked
	 * from it, so those rows take a typed model id instead of pretending. */
	catalogueModalities?: Modality[];
	/** Environment variable holding a base URL, for self-hosted providers. */
	baseUrlEnv?: string;
};


export const MODALITY_LABELS: Record<Modality, string> = {
	text: 'Text',
	speech: 'Spoken replies',
	transcription: 'Transcriptions',
	image: 'Images',
	video: 'Video'
};
/** The order modality headings appear in Settings → Models. */
export const MODALITY_ORDER: Modality[] = ['text', 'speech', 'transcription', 'image', 'video'];

export const PROVIDERS: readonly Provider[] = [
	{
		id: 'openrouter',
		label: 'OpenRouter',
		envVar: 'OPENROUTER_API_KEY',
		summary: 'Models from many labs behind one key: text, images, and transcription over one endpoint.',
		modalities: ['text', 'image', 'transcription'],
		// Its /models list is text-only, so a transcription model is typed by hand
		// (openai/whisper-large-v3 and friends) rather than picked from the list.
		catalogueModalities: ['text'],
		catalogue: { url: 'https://openrouter.ai/api/v1/models', shape: 'openrouter' }
	},
	{
		id: 'openai',
		label: 'OpenAI',
		envVar: 'OPENAI_API_KEY',
		summary: 'Text, spoken replies, transcriptions, images and video under one key.',
		modalities: ['text', 'speech', 'transcription', 'image', 'video'],
		catalogueModalities: ['text', 'speech', 'transcription', 'image', 'video'],
		catalogue: { url: 'https://api.openai.com/v1/models', shape: 'plain' }
	},
	{
		id: 'elevenlabs',
		label: 'ElevenLabs',
		envVar: 'ELEVENLABS_API_KEY',
		summary: 'Speech synthesis and transcription.',
		modalities: ['speech', 'transcription'],
		catalogueModalities: ['speech', 'transcription'],
		catalogue: { url: 'https://api.elevenlabs.io/v1/models', shape: 'plain' }
	},
	{
		id: 'deepgram',
		label: 'Deepgram',
		envVar: 'DEEPGRAM_API_KEY',
		summary: 'Speech to text. Publishes no model list, so its model id is typed by hand.',
		modalities: ['transcription'],
		catalogue: null
	},
	{
		id: 'local',
		label: 'Local model (self-hosted)',
		envVar: 'LOCAL_API_KEY',
		summary:
			'A transcription model running on this machine or LAN: whisper.cpp, Faster-Whisper or Parakeet, served over the OpenAI transcription shape. Nothing leaves the box.',
		modalities: ['transcription'],
		// The model is whichever one that server was started with, so there is
		// nothing to list; the row takes a typed id and may leave it empty.
		catalogue: null,
		baseUrlEnv: 'LOCAL_TRANSCRIPTION_URL'
	},
	{
		id: 'stability',
		label: 'Stability AI',
		envVar: 'STABILITY_API_KEY',
		summary: 'Image generation. Model ids are typed by hand.',
		modalities: ['image'],
		catalogue: null
	},
	{
		id: 'higgsfield',
		label: 'Higgsfield',
		envVar: 'HIGGSFIELD_API_KEY',
		summary: 'Image and video generation. Model ids are typed by hand.',
		modalities: ['image', 'video'],
		catalogue: null
	},
	{
		id: 'runway',
		label: 'Runway',
		envVar: 'RUNWAY_API_KEY',
		summary: 'Video generation. Model ids are typed by hand.',
		modalities: ['video'],
		catalogue: null
	}
];

export function providerById(id: string): Provider | undefined {
	return PROVIDERS.find((provider) => provider.id === id);
}

export function providersFor(modality: Modality): Provider[] {
	return PROVIDERS.filter((provider) => provider.modalities.includes(modality));
}

const DEFAULT_LOCAL_BASE_URL = 'http://127.0.0.1:8080';

/** Where a self-hosted provider is reached. Read from the environment on every
 * call rather than captured at boot, so pointing Lexosa at another box is a
 * restart away and not a code change. */
export function providerBaseUrl(provider: Provider): string | null {
	if (!provider.baseUrlEnv) return null;
	return (process.env[provider.baseUrlEnv] ?? DEFAULT_LOCAL_BASE_URL).replace(/\/+$/, '');
}