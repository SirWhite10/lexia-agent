import { mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import type { RequestEvent } from '@sveltejs/kit';
import { Database } from 'bun:sqlite';

const databasePath = resolve(process.cwd(), 'app-data/state/lexia.sqlite');
mkdirSync(dirname(databasePath), { recursive: true });

const database = new Database(databasePath);
database.run(`
	CREATE TABLE IF NOT EXISTS users (
		id TEXT PRIMARY KEY,
		username TEXT NOT NULL UNIQUE,
		password_hash TEXT NOT NULL,
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
