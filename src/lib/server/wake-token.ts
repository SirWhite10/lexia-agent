import { createHmac } from 'node:crypto';

/**
 * A short-lived token the host mints for the wake service.
 *
 * The host's session cookie is scoped to the host's own origin, so the browser
 * cannot present it to the wake service on another port. A token is the way
 * across: the host signs it, the wake service verifies it with the same
 * secret, and it is worth nothing a minute later.
 *
 * The secret is configuration, not a value in this repository. Both processes
 * need it, and neither can work without it: the host refuses to mint, and the
 * wake service refuses the connection. A feature that is off says so rather
 * than failing later in a way nobody can read.
 */

const TOKEN_LIFETIME_MS = 60_000;
const MINIMUM_SECRET_LENGTH = 32;
/** Not a dot, a dash or a colon: a user id is a UUID and must not split the token. */
const SEPARATOR = '~';

export function wakeTokenSecret(): string | null {
	const secret = process.env.AGENT_WAKE_TOKEN_SECRET?.trim();
	if (!secret || secret.length < MINIMUM_SECRET_LENGTH) return null;
	return secret;
}

/** Mint a token for a signed-in user, or null when the wake service is not configured. */
export function mintWakeToken(userId: string): { token: string; expiresAt: number } | null {
	const secret = wakeTokenSecret();
	if (!secret) return null;
	const expiresAt = Date.now() + TOKEN_LIFETIME_MS;
	const signature = createHmac('sha256', secret)
		.update(`${userId}${SEPARATOR}${expiresAt}`)
		.digest('base64url');
	return { token: `${userId}${SEPARATOR}${expiresAt}${SEPARATOR}${signature}`, expiresAt };
}
