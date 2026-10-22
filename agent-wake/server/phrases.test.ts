import { describe, expect, test } from 'bun:test';
import { DEFAULT_PHRASE, commandFor, defaultPhrases, validatePhrases } from './phrases';

describe('validatePhrases', () => {
	test('an agent with no phrase is refused, because nothing could wake it', () => {
		expect(() => validatePhrases([])).toThrow('at least one wake phrase');
	});

	test('one agent holds several phrases, each mapped to its own command', () => {
		const phrases = validatePhrases([
			{ phrase: 'agent', command: 'chat.reply' },
			{ phrase: 'hey agent', command: 'note.write' }
		]);
		expect(commandFor(phrases, 'AGENT')).toBe('chat.reply');
		expect(commandFor(phrases, 'HEY AGENT')).toBe('note.write');
	});

	test('an unregistered keyword maps to no command, rather than a default one', () => {
		const phrases = validatePhrases([{ phrase: 'agent', command: 'chat.reply' }]);
		expect(commandFor(phrases, 'LEXIA')).toBeNull();
	});

	test('the same phrase cannot map to two commands', () => {
		expect(() =>
			validatePhrases([
				{ phrase: 'agent', command: 'chat.reply' },
				{ phrase: 'AGENT', command: 'note.write' }
			])
		).toThrow('listed more than once');
	});

	test('a phrase the detector cannot spell is refused by name', () => {
		expect(() => validatePhrases([{ phrase: 'H!ENROY', command: 'chat.reply' }])).toThrow('letters, digits');
	});

	test('a command must be a plain command name, not a free-form string', () => {
		expect(() => validatePhrases([{ phrase: 'agent', command: 'rm -rf /' }])).toThrow('lowercase');
		expect(() => validatePhrases([{ phrase: 'agent', command: 'Chat.Reply' }])).toThrow('lowercase');
	});

	test('a skipped setup starts with the default phrase', () => {
		expect(defaultPhrases()).toEqual([{ phrase: DEFAULT_PHRASE, command: 'chat.reply' }]);
		expect(DEFAULT_PHRASE).toBe('AGENT');
	});
});
