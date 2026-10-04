import { afterAll, beforeEach, describe, expect, test } from 'bun:test';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Database } from 'bun:sqlite';

// The recovery table lives in the same state boundary as the rest of the host,
// so pointing LEXIA_STATE_DIR at a temp directory keeps tokens out of the real
// installation. Dynamic import exception: the env var must be set before the
// module reads it at load.
const workspace = mkdtempSync(join(tmpdir(), 'lexosa-auth-test-'));
process.env.LEXIA_STATE_DIR = workspace;
const auth = await import('../src/lib/server/auth.js');

const databaseFile = join(workspace, 'lexia.sqlite');
const recoveryFile = join(workspace, 'auth-recovery.txt');
const password = 'correct-horse-battery';
const replacement = 'a-brand-new-password';

afterAll(() => {
	rmSync(workspace, { recursive: true, force: true });
});

/** Reads the recovery table directly: a test that only trusts the module's own
 * API would not notice the code being stored in the clear. */
function storedToken() {
	const db = new Database(databaseFile, { readonly: true });
	const row = db.query('SELECT token_hash, expires_at, used_at FROM recovery_tokens ORDER BY created_at DESC LIMIT 1').get() as
		| { token_hash: string; expires_at: string; used_at: string | null }
		| null;
	db.close();
	return row;
}

beforeEach(async () => {
	const db = new Database(databaseFile);
	db.run('DELETE FROM recovery_tokens');
	db.run('DELETE FROM users');
	db.close();
	rmSync(recoveryFile, { force: true });
	await auth.createUser('operator', password);
});

describe('recovery codes', () => {
	test('are written to the state file and stored only as a hash', async () => {
		const recovery = await auth.createRecoveryCode();

		expect(recovery?.username).toBe('operator');
		expect(readFileSync(recoveryFile, 'utf8').trim()).toBe(recovery?.code);
		expect(storedToken()?.token_hash).not.toBe(recovery?.code);
		expect(storedToken()?.token_hash).toMatch(/^[0-9a-f]{64}$/);
		expect(storedToken()?.used_at).toBeNull();
	});

	test('are refused when the workspace has no account', async () => {
		const db = new Database(databaseFile);
		db.run('DELETE FROM users');
		db.close();

		expect(await auth.createRecoveryCode()).toBeNull();
		expect(existsSync(recoveryFile)).toBe(false);
	});

	test('set a new password that signs in, and retire the old one', async () => {
		const { code } = (await auth.createRecoveryCode())!;

		expect(await auth.resetPassword(code, replacement)).toEqual({ ok: true, username: 'operator' });
		expect(await auth.authenticate('operator', replacement)).toEqual({ id: expect.any(String), username: 'operator' });
		expect(await auth.authenticate('operator', password)).toBeNull();
	});

	test('leave no plaintext code on disk once spent', async () => {
		const { code } = (await auth.createRecoveryCode())!;

		await auth.resetPassword(code, replacement);

		expect(existsSync(recoveryFile)).toBe(false);
		expect(storedToken()?.used_at).not.toBeNull();
	});

	test('work once only', async () => {
		const { code } = (await auth.createRecoveryCode())!;
		await auth.resetPassword(code, replacement);

		const replay = await auth.resetPassword(code, 'yet-another-password');

		expect(replay.ok).toBe(false);
		expect(await auth.authenticate('operator', replacement)).not.toBeNull();
	});

	test('are rejected when expired or unknown, leaving the password alone', async () => {
		const expired = await auth.createRecoveryCode({ ttlMs: 0 });

		expect(await auth.resetPassword(expired!.code, replacement)).toEqual({
			ok: false,
			error: 'That recovery code has expired. Request a new one.',
		});

		const current = (await auth.createRecoveryCode())!;

		expect(await auth.resetPassword('not-a-real-code', replacement)).toEqual({
			ok: false,
			error: 'That recovery code is not valid. Request a new one.',
		});
		expect(current.code.length).toBeGreaterThanOrEqual(43);
		expect(await auth.authenticate('operator', password)).not.toBeNull();
	});

	test('are retired when a newer code is requested', async () => {
		const first = (await auth.createRecoveryCode())!;
		const second = (await auth.createRecoveryCode())!;

		expect(first.code).not.toBe(second.code);
		expect(await auth.resetPassword(first.code, replacement)).toEqual({
			ok: false,
			error: 'That recovery code is not valid. Request a new one.',
		});
		expect(await auth.resetPassword(second.code, replacement)).toEqual({ ok: true, username: 'operator' });
	});
});