import { fail, redirect } from '@sveltejs/kit';
import { bootstrapAgent, bootstrapStatus } from '#lib/server/bootstrap.js';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = () => {
	const status = bootstrapStatus();
	// A finished install has nothing left to bootstrap; the files are editable
	// by hand and the agent grows its own memory from here.
	if (status.bootstrapped) redirect(303, '/');
	return status;
};

export const actions: Actions = {
	create: async ({ request }) => {
		const form = await request.formData();
		const name = String(form.get('name') ?? '').trim();
		const purpose = String(form.get('purpose') ?? '').trim();
		const tone = String(form.get('tone') ?? '').trim();
		const traits = form
			.getAll('traits')
			.map((trait) => String(trait).trim())
			.filter((trait) => trait.length > 0);

		if (!name) return fail(400, { error: 'Give your agent a name.' });
		if (!purpose) return fail(400, { error: 'Say what it is for, even roughly.' });

		bootstrapAgent({ name, purpose, tone, traits });
		redirect(303, '/');
	}
};
