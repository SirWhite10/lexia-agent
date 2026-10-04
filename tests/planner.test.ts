import { describe, expect, test } from 'bun:test';
import { planTurn } from '../src/lib/server/planner.js';

describe('the turn planner', () => {
	test('plans no delegation for ordinary conversation', () => {
		for (const body of [
			'',
			'   ',
			'hello',
			'what model are you',
			'summarise the notes I attached',
			'I will spin up the kettle afterwards'
		]) {
			expect(planTurn({ body }).subagent).toBeNull();
		}
	});

	test('plans a delegation when a sub-agent is named', () => {
		const plan = planTurn({ body: 'run a sub agent, make em do a search on the stock of SpaceX' });

		expect(plan.subagent).not.toBeNull();
		expect(plan.subagent?.name).toBe('SpaceX researcher');
		expect(plan.subagent?.task.toLowerCase()).toContain('spacex');
	});

	test('plans a delegation when the wording hands the work off', () => {
		const plan = planTurn({ body: 'delegate this to a researcher and have them find the Q3 figures' });

		expect(plan.subagent?.name).toBe('Researcher');
		expect(plan.subagent?.focus.length).toBeGreaterThan(0);
		expect(plan.subagent?.systemPrompt).toContain('sub-agent');
	});

	test('never plans an empty task', () => {
		const plan = planTurn({ body: 'please spin up an agent' });

		expect(plan.subagent?.task.length).toBeGreaterThan(0);
		expect(plan.subagent?.focus).toBe(plan.subagent?.task);
	});

	test('names the sub-agent after the topic it was given', () => {
		expect(planTurn({ body: 'have someone do a deep dive on Anthropic pricing' }).subagent?.name).toBe(
			'Anthropic researcher'
		);
	});
});