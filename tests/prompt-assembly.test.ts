import { beforeEach, describe, expect, test } from 'bun:test';
import { assemblePrompt, resetPrefixTracking } from '../src/lib/server/prompt.js';

const SKILLS = ['# Skill: lights\nOperate the smart home.', '# Skill: calendar\nRead and write events.'];
const PERSONA = '# Personality\n\n- [2026-01-01T00:00:00.000Z] answers concisely';

beforeEach(() => {
	resetPrefixTracking();
});

describe('cache-stable prompt assembly', () => {
	test('two turns differing only in memory share the prefix digest', () => {
		const first = assemblePrompt({ personality: PERSONA, skills: SKILLS, memory: ['fact A'], tools: ['lights.off'] });
		expect(first.prefixStable).toBe(true);

		const second = assemblePrompt({ personality: PERSONA, skills: SKILLS, memory: ['fact B', 'fact C'], tools: ['lights.off'] });
		expect(second.prefixStable).toBe(true);
		expect(second.prefixDigest).toBe(first.prefixDigest);
		expect(second.suffixDigest).not.toBe(first.suffixDigest);
	});

	test('a personality rewrite changes the prefix once, at the boundary', () => {
		assemblePrompt({ personality: PERSONA, skills: SKILLS, memory: [], tools: [] });
		const grown = `${PERSONA}\n- [2026-01-02T00:00:00.000Z] asks before acting`;
		const rewritten = assemblePrompt({ personality: grown, skills: SKILLS, memory: [], tools: [] });
		expect(rewritten.prefixStable).toBe(false);
		const after = assemblePrompt({ personality: grown, skills: SKILLS, memory: ['fact'], tools: [] });
		expect(after.prefixStable).toBe(true);
		expect(after.prefixDigest).toBe(rewritten.prefixDigest);
	});

	test('slot order is personality, skills, memory, tools', () => {
		const { system } = assemblePrompt({
			personality: PERSONA,
			skills: SKILLS,
			memory: ['remembered fact'],
			tools: ['resolved capability'],
		});
		const order = ['# Personality', '# Skills', '# Memory', '# Tools'].map((heading) => system.indexOf(heading));
		expect(order).toEqual([...order].sort((a, b) => a - b));
		expect(order.every((index) => index >= 0)).toBe(true);
	});

	test('empty slots are omitted, not rendered as empty headings', () => {
		const { system } = assemblePrompt({ skills: [], memory: [], tools: [] });
		expect(system).toBe('');
		const withMemory = assemblePrompt({ skills: [], memory: ['fact'], tools: [] });
		expect(withMemory.system).toContain('# Memory');
		expect(withMemory.system).not.toContain('# Tools');
	});
});
