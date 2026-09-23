import { fail, redirect } from '@sveltejs/kit';
import { authenticate, createUser, hasUsers } from '../../../lib/server/auth';
import type { Actions, PageServerLoad } from './$types';

const sessionCookie = {
	httpOnly: true,
	path: '/',
	sameSite: 'lax' as const,
	secure: false,
	maxAge: 60 * 60 * 24 * 30,
};

function safeReturnTo(value: string | null): string {
	return value?.startsWith('/') && !value.startsWith('//') ? value : '/';
}

function readCredentials(formData: FormData) {
	return {
		username: String(formData.get('username') ?? '').trim(),
		password: String(formData.get('password') ?? ''),
		confirmPassword: String(formData.get('confirmPassword') ?? ''),
	};
}

export const load: PageServerLoad = ({ url }) => ({
	initialized: hasUsers(),
	returnTo: safeReturnTo(url.searchParams.get('returnTo')),
});

export const actions: Actions = {
	login: async ({ request, cookies, url }) => {
		const { username, password } = readCredentials(await request.formData());
		const user = await authenticate(username, password);

		if (!user) return fail(400, { mode: 'login', username, error: 'That username or password is not correct.' });
		cookies.set('lexia_session', user.id, sessionCookie);
		redirect(303, safeReturnTo(url.searchParams.get('returnTo')));
	},

	setup: async ({ request, cookies, url }) => {
		if (hasUsers()) return fail(409, { mode: 'setup', error: 'Lexia is already initialized. Sign in instead.' });

		const { username, password, confirmPassword } = readCredentials(await request.formData());
		if (username.length < 3) return fail(400, { mode: 'setup', username, error: 'Choose a username with at least 3 characters.' });
		if (password.length < 8) return fail(400, { mode: 'setup', username, error: 'Choose a password with at least 8 characters.' });
		if (password !== confirmPassword) return fail(400, { mode: 'setup', username, error: 'The passwords do not match.' });

		try {
			const user = await createUser(username, password);
			cookies.set('lexia_session', user.id, sessionCookie);
		} catch {
			return fail(409, { mode: 'setup', username, error: 'That username is already in use.' });
		}

		redirect(303, safeReturnTo(url.searchParams.get('returnTo')));
	},
};
