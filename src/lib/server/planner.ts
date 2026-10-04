/**
 * Jev's first cut (WF-IMP-001 slice): the planner that reads a request and
 * decides the run graph behind it.
 *
 * Today it recognises exactly one intent — delegate this to a sub-agent — from
 * the wording, and hands back the role card and task to run. That is a heuristic
 * on purpose and it is not the final shape: the router that replaces it will read
 * the thought tree in `/thinking` instead of a list of patterns. What matters
 * now is that the decision is made in one place and produces a real run graph,
 * so swapping the decision out later changes nothing downstream.
 */

export type SubAgentPlan = {
	/** Role-card name, also the sidebar label. */
	name: string;
	/** One line of scope, shown under the name in the sub-agent manager. */
	focus: string;
	/** Version 1 of the sub-agent's prompt: who it is and what it answers. */
	systemPrompt: string;
	/** The single job this sub-agent was spawned for. */
	task: string;
};

export type TurnPlan = { subagent: SubAgentPlan | null };

/** The phrasings that mean "this is not my job". "spin up" alone is not one of
 * them — people say it about kettles. */
const DELEGATION_PHRASES = [
	'sub[-\\s]?agents?',
	'delegate',
	'hand (?:this|it|that) (?:off|over)',
	'(?:have|get) (?:someone|another one|it|them) (?:to )?(?:do|look|research|check|find|dig)',
	'ask (?:someone|another agent|a sub[-\\s]?agent) to',
	'spin up an? (?:agent|helper|researcher)'
];

/** The hand-off clause runs to the first break after it: a comma, or the
 * conjunction that starts the actual work. */
const DELEGATION_CLAUSE = new RegExp(
	// Lazy on both sides: greedy would run to the end of the sentence and treat
	// the whole request as the hand-off clause.
	`^[^,.\\n]*?\\b(?:${DELEGATION_PHRASES.join('|')})\\b[^,.\\n]*?(?:[,.;\\n]|\\band then\\b|\\bthen\\b|\\band\\b|$)`,
	'i'
);

/** Lead-ins that describe the hand-off rather than the work inside it. */
const HANDOFF_LEAD_IN =
	/^(?:please\s+|go ahead and\s+|can you\s+|could you\s+|i(?:'| a)?d like you to\s+)*(?:make|have|get|ask) (?:it|em|him|her|them|someone) (?:to )?(?:do|look|research|check|find|dig|run)\s*/i;

/** "ask a subagent to look into X" has no break after the hand-off: the work
 * starts at the verb, so the clause is a prefix rather than a span. */
const DELEGATION_LEAD_CLAUSE =
	/^(?:please\s+|can you\s+|could you\s+|go ahead and\s+|i(?:'| a)?d like you to\s+)*(?:ask|have|get|tell|spin up)\s+(?:a\s+|an\s+|another\s+)?(?:sub[-\s]?agent|agent|helper|researcher|someone|them)\s+(?:to\s+)?/i;
/** Roles the request itself named; used when there is no topic to name the
 * sub-agent after. */
const NAMED_ROLES = ['researcher', 'analyst', 'writer', 'reviewer', 'engineer', 'specialist'];

export function planTurn(input: { body: string }): TurnPlan {
	const body = input.body.trim();
	if (!body) return { subagent: null };
	if (!DELEGATION_PHRASES.some((phrase) => new RegExp(`\\b(?:${phrase})\\b`, 'i').test(body))) return { subagent: null };

	const task = extractTask(body);

	return {
		subagent: {
			name: subAgentName(task, body),
			focus: task,
			systemPrompt: [
				`You are ${subAgentName(task, body)}, a sub-agent Lexosa spawned for one job.`,
				'Answer that job and nothing else.',
				'Report what you found as plain prose, and say plainly what you could not establish.'
			].join(' '),
			task
		}
	};
}

function extractTask(body: string): string {
	const clause = DELEGATION_CLAUSE.exec(body);
	const afterClause = clause ? body.slice(clause[0].length).trim() : '';
	const task = stripLeadIn(afterClause.length > 0 ? afterClause : body).replace(/\s+/g, ' ').trim();
	// A request that is nothing but the hand-off still needs something to do.
	return task.length > 0 ? task : body;
}

function stripLeadIn(value: string): string {
	return value.replace(HANDOFF_LEAD_IN, '').replace(DELEGATION_LEAD_CLAUSE, '').trim();
}

function subAgentName(task: string, body: string): string {
	// A proper noun in the task is the topic: "search the stock of SpaceX".
	const topic = task.match(/\b[A-Z][A-Za-z0-9.-]{2,}\b/);
	if (topic) return `${topic[0]} researcher`;

	// Otherwise keep the role the request asked for rather than inventing one
	// from the first long word in the sentence.
	const role = NAMED_ROLES.find((candidate) => new RegExp(`\\b${candidate}\\b`, 'i').test(body));
	return role ? role[0].toUpperCase() + role.slice(1) : 'Researcher';
}