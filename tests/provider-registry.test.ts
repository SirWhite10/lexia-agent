import { describe, expect, test } from 'bun:test';
import { MODALITY_LABELS, MODALITY_ORDER, PROVIDERS, providerById, providersFor } from '../src/lib/server/provider-registry.js';

describe('the provider registry', () => {
	test('has no duplicate ids or environment variables', () => {
		expect(new Set(PROVIDERS.map((provider) => provider.id)).size).toBe(PROVIDERS.length);
		expect(new Set(PROVIDERS.map((provider) => provider.envVar)).size).toBe(PROVIDERS.length);
	});

	test('names every environment variable after its provider id', () => {
		for (const provider of PROVIDERS) {
			const expected = `${provider.id.toUpperCase().replace(/-/g, '_')}_API_KEY`;
			expect(provider.envVar).toBe(expected);
		}
	});

	test('covers text, speech, transcription, image and video', () => {
		const served = new Set(PROVIDERS.flatMap((provider) => provider.modalities));
		expect([...served].sort()).toEqual([...MODALITY_ORDER].sort());
	});

	test('only claims modalities it can label', () => {
		for (const provider of PROVIDERS) {
			expect(provider.modalities.length).toBeGreaterThan(0);
			for (const modality of provider.modalities) {
				expect(MODALITY_LABELS[modality]).toBeTruthy();
			}
		}
	});

	test('covers the modalities the user asked for with more than one provider each', () => {
		// A modality served by one provider would make the picker pointless: there
		// would be nothing to choose between.
		for (const modality of MODALITY_ORDER) {
			expect(providersFor(modality).length).toBeGreaterThan(1);
		}
	});

	test('looks providers up by id', () => {
		expect(providerById('higgsfield')?.modalities).toEqual(['image', 'video']);
		expect(providerById('nope')).toBeUndefined();
	});
});