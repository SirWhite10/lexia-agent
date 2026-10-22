import { createHmac, timingSafeEqual } from 'node:crypto';

/**
 * The host's session, presented as a token.
 *
 * The wake service is a separate process on a separate origin, so the host's
 * session cookie never reaches it. The host mints a short-lived signed token
 * instead, and this is the other half of that exchange.
 *
 * This file is a deliberate duplicate of the host's `src/lib/server/wake-token.ts`.
 * The two run in different runtimes — Bun and SvelteKit's server — and cannot
 * share a module, so the format is specified in both places rather than
 * imported from one. The rule the two must agree on: the same secret, the
 * same separator, the same signature over the same fields, and a lifetime of
 * one minute.
 */

const MINIMUM_SECRET_LENGTH = 32;
/** Not a dot, a dash or a colon: a user id is a UUID and must not split the token. */
const SEPARATOR = '~';

export type HostUser = {
	id: string;
	/** The user id the host signed. The username is not carried: the wake
	 * service does not need it, and a token should carry no more than it must. */
};

export function wakeTokenSecret(): string | null {
	const secret = process.env.AGENT_WAKE_TOKEN_SECRET?.trim();
	if (!secret || secret.length < MINIMUM_SECRET_LENGTH) return null;
	return secret;
}

/**
 * Verify a token the host minted. Returns null for anything that is not a
 * live, correctly signed token, including the case where the wake service has
 * no secret configured: an unconfigured service refuses rather than trusting
 * an unsigned token.
 */
export function verifyHostToken(token: string | null): HostUser | null {
	const secret = wakeTokenSecret();
	if (!secret || !token) return null;
	const parts = token.split(SEPARATOR);
	if (parts.length !== 3) return null;
	const [id, expiresAtRaw, signature] = parts;
	const expiresAt = Number(expiresAtRaw);
	if (!Number.isFinite(expiresAt) || Date.now() > expiresAt) return null;
	const expected = createHmac('sha256', secret)
		.update(`${id}${SEPARATOR}${expiresAtRaw}`)
		.digest('base64url');
	const given = Buffer.from(signature);
	const wanted = Buffer.from(expected);
	if (given.length !== wanted.length || !timingSafeEqual(given, wanted)) return null;
	return { id };
}
