import { error, fail } from '@sveltejs/kit';
import {
	archiveSubAgent,
	getSubAgent,
	listSubAgentPromptVersions,
	retireSubAgent,
	reviseSubAgentPrompt,
	updateSubAgentRoleCard,
} from '#lib/server/agents.js';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = ({ params }) => {
	const subAgent = getSubAgent(params.id);
	if (!subAgent) error(404, 'Not found');
	return { subAgent, promptVersions: listSubAgentPromptVersions(params.id) };
};

async function readForm(request: Request): Promise<{ id: string; form: FormData }> {
	const form = await request.formData();
	return { id: String(form.get('id') ?? ''), form };
}

export const actions: Actions = {
	role: async ({ request }) => {
		const { id, form } = await readForm(request);
		const focus = String(form.get('focus') ?? '').trim();
		const memoryScope = String(form.get('memoryScope') ?? '').trim();
		const capabilities = String(form.get('capabilities') ?? '').trim();
		if (!getSubAgent(id)) return fail(404, { error: 'That sub-agent no longer exists.' });
		updateSubAgentRoleCard(id, { focus, memoryScope, capabilities });
		return { success: true };
	},

	prompt: async ({ request }) => {
		const { id, form } = await readForm(request);
		const body = String(form.get('body') ?? '').trim();
		if (!body) return fail(400, { error: 'The system prompt cannot be empty.' });
		if (!getSubAgent(id)) return fail(404, { error: 'That sub-agent no longer exists.' });
		const version = reviseSubAgentPrompt(id, body);
		return { success: true, version };
	},

	retire: async ({ request }) => {
		const { id } = await readForm(request);
		if (!getSubAgent(id)) return fail(404, { error: 'That sub-agent no longer exists.' });
		retireSubAgent(id);
		return { success: true };
	},

	archive: async ({ request }) => {
		const { id } = await readForm(request);
		if (!getSubAgent(id)) return fail(404, { error: 'That sub-agent no longer exists.' });
		archiveSubAgent(id);
		return { success: true };
	}
};
