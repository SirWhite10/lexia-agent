export type FixtureDefinition = {
	id: string;
	path: string;
	label: string;
};

const fixtureRoot = 'assets/fixtures';

export const fixtures: Record<string, FixtureDefinition> = {
	'agent-wake': {
		id: 'agent-wake',
		path: `${fixtureRoot}/agent-wake.wav`,
		label: 'Wake phrase'
	}
};

export function fixtureFor(id: string): FixtureDefinition | null {
	return fixtures[id] ?? null;
}
