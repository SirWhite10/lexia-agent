import { fail } from '@sveltejs/kit';
import { listAgentTasks } from '#lib/server/runs.js';
import { archiveSubAgent, listStandingSubAgents, renameSubAgent } from '#lib/server/agents.js';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = () => ({
	subAgents: listStandingSubAgents(),
	agentTasks: listAgentTasks(10)
});


export const actions: Actions = {
	rename: async ({ request }) => {
		const form = await request.formData();
		const id = String(form.get('id') ?? '');
		const name = String(form.get('name') ?? '').trim();
		if (!name) return fail(400, { id, error: 'Name cannot be empty.' });
		renameSubAgent(id, name);
	},
	archive: async ({ request }) => {
		const form = await request.formData();
		archiveSubAgent(String(form.get('id') ?? ''));
	},
};
