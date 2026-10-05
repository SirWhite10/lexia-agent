import { fail, redirect } from '@sveltejs/kit';
import { authenticate, createRecoveryCode, createUser, hasUsers, resetPassword } from '../../../lib/server/auth';
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

/** Shape every failure in this route shares, so the page can switch steps off
 * one field instead of guessing from the error text. `needsSignIn` marks the
 * refusal that must land on the sign-in form: setup is not offered once an
 * account exists, whatever the page believed when it was rendered. */
type AuthForm = {
	mode: 'login' | 'setup' | 'recover' | 'reset';
	username?: string;
	error: string;
	needsSignIn?: boolean;
};

/** Two flags carry the successful outcomes of the recovery flow through the
 * redirect, because a completed action has no `form` payload to render from.
 * The browser only ever learns that a code was minted, never the code. */
export const load: PageServerLoad = ({ url }) => ({
	initialized: hasUsers(),
	returnTo: safeReturnTo(url.searchParams.get('returnTo')),
	recoveryRequested: url.searchParams.get('recover') === '1',
	passwordReset: url.searchParams.get('reset') === '1',
});

export const actions: Actions = {
	login: async ({ request, cookies, url }) => {
		const { username, password } = readCredentials(await request.formData());
		const user = await authenticate(username, password);

		if (!user) return fail<AuthForm>(400, { mode: 'login', username, error: 'That username or password is not correct.' });
		cookies.set('lexia_session', user.id, sessionCookie);
		redirect(303, safeReturnTo(url.searchParams.get('returnTo')));
	},

	setup: async ({ request, cookies, url }) => {
		const { username, password, confirmPassword } = readCredentials(await request.formData());

		// Credentials are read first so the refusal can prefill the sign-in form.
		if (hasUsers()) {
			return fail<AuthForm>(409, {
				mode: 'setup',
				username,
				needsSignIn: true,
				error: 'This workspace already has an account. Sign in instead.',
			});
		}

		if (username.length < 3) return fail<AuthForm>(400, { mode: 'setup', username, error: 'Choose a username with at least 3 characters.' });
		if (password.length < 8) return fail<AuthForm>(400, { mode: 'setup', username, error: 'Choose a password with at least 8 characters.' });
		if (password !== confirmPassword) return fail<AuthForm>(400, { mode: 'setup', username, error: 'The passwords do not match.' });

		try {
			const user = await createUser(username, password);
			cookies.set('lexia_session', user.id, sessionCookie);
		} catch {
			return fail<AuthForm>(409, { mode: 'setup', username, error: 'That username is already in use.' });
		}

		redirect(303, safeReturnTo(url.searchParams.get('returnTo')));
	},

	recover: async ({ url }) => {
		const recovery = await createRecoveryCode();
		if (!recovery) return fail<AuthForm>(400, { mode: 'recover', error: 'There is no account on this workspace to recover.' });

		// The code goes to the operator's terminal and state directory, never to
		// the requester: this app answers on the LAN, and minting a code is the
	// proof of shell access that separates the operator from a passer-by.
		console.log(
			`[auth] recovery code for ${recovery.username}, valid until ${recovery.expiresAt}:\n  ${recovery.code}\n  also written to ${recovery.path}`,
		);

		redirect(303, `/login?recover=1&returnTo=${encodeURIComponent(safeReturnTo(url.searchParams.get('returnTo')))}`);
	},

	reset: async ({ request, url }) => {
		const formData = await request.formData();
		const code = String(formData.get('code') ?? '').trim();
		const password = String(formData.get('password') ?? '');
		const confirmPassword = String(formData.get('confirmPassword') ?? '');
		const returnTo = encodeURIComponent(safeReturnTo(url.searchParams.get('returnTo')));

		if (password.length < 8) return fail<AuthForm>(400, { mode: 'reset', error: 'Choose a password with at least 8 characters.' });
		if (password !== confirmPassword) return fail<AuthForm>(400, { mode: 'reset', error: 'The passwords do not match.' });

		const result = await resetPassword(code, password);
		if (!result.ok) return fail<AuthForm>(400, { mode: 'reset', error: result.error });

		console.log(`[auth] password reset for ${result.username}`);
		redirect(303, `/login?reset=1&returnTo=${returnTo}`);
	},
};
