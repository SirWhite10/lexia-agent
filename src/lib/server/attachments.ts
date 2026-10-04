import { mkdirSync, writeFileSync } from 'node:fs';
import { extname, join, resolve } from 'node:path';
import { Database } from 'bun:sqlite';

// Files and voice notes the operator attaches to a turn. Bytes live under the
// state directory rather than in SQLite: transcripts are derived from them
// later, so the original recording has to survive being large and the database
// stays a projection the chat can query cheaply.
const stateDir = resolve(process.env.LEXIA_STATE_DIR ?? join(process.cwd(), 'app-data/state'));
const attachmentDir = join(stateDir, 'attachments');
mkdirSync(attachmentDir, { recursive: true });

const database = new Database(join(stateDir, 'lexia.sqlite'));
database.run(`
	CREATE TABLE IF NOT EXISTS attachments (
		id TEXT PRIMARY KEY,
		message_id TEXT,
		name TEXT NOT NULL,
		mime_type TEXT NOT NULL,
		size INTEGER NOT NULL,
		kind TEXT NOT NULL CHECK (kind IN ('file', 'audio')),
		path TEXT NOT NULL,
		transcript TEXT,
		created_at TEXT NOT NULL
	)
`);

export type AttachmentKind = 'file' | 'audio';

export type Attachment = {
	id: string;
	name: string;
	mimeType: string;
	size: number;
	kind: AttachmentKind;
};

type AttachmentRow = {
	id: string;
	message_id: string;
	name: string;
	mime_type: string;
	size: number;
	kind: AttachmentKind;
};

const MAX_ATTACHMENT_BYTES = 25 * 1024 * 1024;

/** Keeps a caller-supplied filename from escaping the attachment directory: the
 * stored name is a generated id plus a sanitised extension, never the original
 * path. */
function storedName(id: string, name: string): string {
	const extension = extname(name).slice(0, 16).replace(/[^a-zA-Z0-9.]/g, '');
	return `${id}${extension}`;
}

export function saveAttachment(input: {
	name: string;
	mimeType: string;
	bytes: ArrayBuffer;
	kind: AttachmentKind;
}): Attachment {
	if (input.bytes.byteLength > MAX_ATTACHMENT_BYTES) {
		throw new Error(`Attachments are limited to ${Math.round(MAX_ATTACHMENT_BYTES / 1024 / 1024)} MB.`);
	}

	const id = crypto.randomUUID();
	const path = join(attachmentDir, storedName(id, input.name));
	writeFileSync(path, Buffer.from(input.bytes));

	database
		.query(
			'INSERT INTO attachments (id, message_id, name, mime_type, size, kind, path, transcript, created_at) VALUES (?, NULL, ?, ?, ?, ?, ?, NULL, ?)'
		)
		.run(
			id,
			input.name.slice(0, 255),
			input.mimeType || 'application/octet-stream',
			input.bytes.byteLength,
			input.kind,
			path,
			new Date().toISOString()
		);

	return { id, name: input.name, mimeType: input.mimeType, size: input.bytes.byteLength, kind: input.kind };
}

/** Attaches uploaded files to the message they were sent with. An id that was
 * never uploaded is ignored rather than fatal: a stale composer chip should not
 * cost the operator their turn. */
export function linkAttachmentsToMessage(ids: string[], messageId: string): void {
	const statement = database.query('UPDATE attachments SET message_id = ? WHERE id = ?');
	for (const id of ids) statement.run(messageId, id);
}

/** Attachments grouped by the message they travelled with, which is how the
 * chat renders them: as chips under the message, not as a separate feed. */
export function listAttachments(messageIds: string[]): Record<string, Attachment[]> {
	if (messageIds.length === 0) return {};

	const placeholders = messageIds.map(() => '?').join(',');
	const rows = database
		.query(
			`SELECT id, message_id, name, mime_type, size, kind FROM attachments WHERE message_id IN (${placeholders}) ORDER BY created_at`
		)
		.all(...messageIds) as AttachmentRow[];

	const grouped: Record<string, Attachment[]> = {};
	for (const row of rows) {
		(grouped[row.message_id] ??= []).push({
			id: row.id,
			name: row.name,
			mimeType: row.mime_type,
			size: row.size,
			kind: row.kind
		});
	}
	return grouped;
}