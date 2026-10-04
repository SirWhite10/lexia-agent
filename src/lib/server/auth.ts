import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join, resolve } from 'node:path';
import type { RequestEvent } from '@sveltejs/kit';
import { Database } from 'bun:sqlite';

// Same state boundary as the other data modules (runs, memory, agents): an
// explicit LEXIA_STATE_DIR wins, otherwise the default sits under the app root.
const stateDir = resolve(process.env.LEXIA_STATE_DIR ?? join(process.cwd(), 'app-data/state'));
const databasePath = join(stateDir, 'lexia.sqlite');
mkdirSync(stateDir, { recursive: true });

const database = new Database(databasePath);

database.run(`
	CREATE TABLE IF NOT EXISTS users (
		id TEXT PRIMARY KEY,
		username TEXT NOT NULL UNIQUE,
		password_hash TEXT NOT NULL,
		created_at TEXT NOT NULL
	)
`);

/** Where a freshly minted recovery code is left for the operator. The browser
 * never receives the code, so minting one requires shell access to the host —
 * the app is served on the LAN, and loading /login is not that proof. */
const recoveryFilePath = join(stateDir, 'auth-recovery.txt');

/** Recovery codes are short-lived by design: 30 minutes is enough to read a
 * terminal and type a password, and short enough that a leaked code is not a
 * standing credential. */
const RECOVERY_TTL_MS = 30 * 60 * 1000;

database.run(`
	CREATE TABLE IF NOT EXISTS recovery_tokens (
		id TEXT PRIMARY KEY,
		user_id TEXT NOT NULL,
		token_hash TEXT NOT NULL,
		expires_at TEXT NOT NULL,
		used_at TEXT,
		created_at TEXT NOT NULL
	)
`);

type UserRow = { id: string; username: string };

export function hasUsers(): boolean {
	return (database.query('SELECT COUNT(*) AS count FROM users').get() as { count: number }).count > 0;
}

export async function createUser(username: string, password: string): Promise<UserRow> {
	const user = {
		id: crypto.randomUUID(),
		username,
		passwordHash: await Bun.password.hash(password),
		createdAt: new Date().toISOString(),
	};

	database
		.query('INSERT INTO users (id, username, password_hash, created_at) VALUES (?, ?, ?, ?)')
		.run(user.id, user.username, user.passwordHash, user.createdAt);

	return { id: user.id, username: user.username };
}

export async function authenticate(username: string, password: string): Promise<UserRow | null> {
	const row = database
		.query('SELECT id, username, password_hash FROM users WHERE username = ?')
		.get(username) as (UserRow & { password_hash: string }) | null;

	if (!row || !(await Bun.password.verify(password, row.password_hash))) return null;
	return { id: row.id, username: row.username };
}

/** Reads the server-issued session cookie without exposing auth state to the client. */
export function getUser(event: Pick<RequestEvent, 'cookies'>): App.Locals['user'] {
	const session = event.cookies.get('lexia_session');
	if (!session) return null;

	const user = database.query('SELECT id, username FROM users WHERE id = ?').get(session) as UserRow | null;
	return user ? { id: user.id, name: user.username } : null;
}

type RecoveryTokenRow = {
	id: string;
	user_id: string;
	token_hash: string;
	expires_at: string;
	used_at: string | null;
};

export type RecoveryCode = {
	username: string;
	code: string;
	expiresAt: string;
	path: string;
};

/** The code is 256 bits of CSPRNG output, so a fast digest is the right trade:
 * there is nothing to brute force, and the reset path stays instant. */
function tokenHash(code: string): string {
	return createHash('sha256').update(code).digest('hex');
}

/** Mints a single-use recovery code and leaves it where the operator can read
 * it. Returns null when the workspace has no account to recover. Only the
 * oldest account is recoverable: the setup action refuses to run once any user
 * exists, so this installation has at most one. */
export async function createRecoveryCode(options: { ttlMs?: number } = {}): Promise<RecoveryCode | null> {
	const user = database.query('SELECT id, username FROM users ORDER BY created_at LIMIT 1').get() as UserRow | null;
	if (!user) return null;

	// One live code at a time: a new request retires the previous code so it
	// cannot outlive the moment it was superseded.
	database.run('DELETE FROM recovery_tokens WHERE used_at IS NULL');

	const code = Buffer.from(crypto.getRandomValues(new Uint8Array(32))).toString('base64url');
	const expiresAt = new Date(Date.now() + (options.ttlMs ?? RECOVERY_TTL_MS)).toISOString();
	database
	.query(
		'INSERT INTO recovery_tokens (id, user_id, token_hash, expires_at, used_at, created_at) VALUES (?, ?, ?, ?, NULL, ?)',
	)
	.run(crypto.randomUUID(), user.id, tokenHash(code), expiresAt, new Date().toISOString());

	rmSync(recoveryFilePath, { force: true });
	writeFileSync(recoveryFilePath, `${code}\n`, { mode: 0o600 });

	return { username: user.username, code, expiresAt, path: recoveryFilePath };
}

export type PasswordReset = { ok: true; username: string } | { ok: false; error: string };

/** Consumes a recovery code and sets a new password. A code is spent whether or
 * not it was the right one for the attempt in flight, and the file holding the
 * plaintext is removed with it so no spent code stays on disk. */
export async function resetPassword(code: string, password: string): Promise<PasswordReset> {
	const row = database
	.query('SELECT id, user_id, token_hash, expires_at, used_at FROM recovery_tokens WHERE token_hash = ?')
	.get(tokenHash(code.trim())) as RecoveryTokenRow | undefined;
	if (!row || row.used_at) return { ok: false, error: 'That recovery code is not valid. Request a new one.' };
	if (Date.parse(row.expires_at) <= Date.now()) return { ok: false, error: 'That recovery code has expired. Request a new one.' };

	const user = database.query('SELECT id, username FROM users WHERE id = ?').get(row.user_id) as UserRow | null;
	if (!user) return { ok: false, error: 'That account no longer exists on this workspace.' };

	database
		.query('UPDATE users SET password_hash = ? WHERE id = ?')
		.run(await Bun.password.hash(password), user.id);
	database.query('UPDATE recovery_tokens SET used_at = ? WHERE id = ?').run(new Date().toISOString(), row.id);
rmSync(recoveryFilePath, { force: true });

	return { ok: true, username: user.username };
}
