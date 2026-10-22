import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { createHmac } from 'node:crypto';
import { verifyHostToken } from './host-token';

/**
 * The token is the only thing that lets the agent host's page reach this
 * service: the host's session cookie is scoped to the host's origin and never
 * arrives here. So the checks that matter are that a live, correctly signed
 * token is accepted and that everything else is refused.
 */
const SECRET = 'a-secret-long-enough-to-be-used-in-a-test';
const SEPARATOR = '~';

function sign(userId: string, expiresAt: number, secret = SECRET): string {
	const signature = createHmac('sha256', secret)
		.update(`${userId}${SEPARATOR}${expiresAt}`)
		.digest('base64url');
	return `${userId}${SEPARATOR}${expiresAt}${SEPARATOR}${signature}`;
}

beforeEach(() => {
	process.env.AGENT_WAKE_TOKEN_SECRET = SECRET;
});

afterEach(() => {
	delete process.env.AGENT_WAKE_TOKEN_SECRET;
});

describe('verifyHostToken', () => {
	test('accepts a live token signed with the shared secret', () => {
		const userId = '3f1c1a4e-0f2a-4a3e-9a2f-1d5f6c7b8a90';
		const token = sign(userId, Date.now() + 60_000);
		expect(verifyHostToken(token)).toEqual({ id: userId });
	});

	test('refuses a token signed with a different secret', () => {
		const token = sign('someone', Date.now() + 60_000, 'a-different-secret-that-is-long-enough');
		expect(verifyHostToken(token)).toBeNull();
	});

	test('refuses a token whose signature was tampered with', () => {
		const token = sign('someone', Date.now() + 60_000);
		const parts = token.split(SEPARATOR);
		parts[2] = 'not-the-signature';
		expect(verifyHostToken(parts.join(SEPARATOR))).toBeNull();
	});

	test('refuses an expired token', () => {
		expect(verifyHostToken(sign('someone', Date.now() - 1))).toBeNull();
	});

	test('refuses a malformed token', () => {
		expect(verifyHostToken('not-a-token')).toBeNull();
		expect(verifyHostToken('')).toBeNull();
		expect(verifyHostToken(null)).toBeNull();
	});

	test('refuses every token when no secret is configured', () => {
		delete process.env.AGENT_WAKE_TOKEN_SECRET;
		// An unconfigured service refuses rather than trusting an unsigned
		// token: a feature that is off says so instead of accepting anything.
		expect(verifyHostToken(sign('someone', Date.now() + 60_000))).toBeNull();
	});
});
