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
		transcript_error TEXT,
		transcribed_at TEXT,
		created_at TEXT NOT NULL
	)
`);

// The transcript columns arrived after the table did. `CREATE TABLE IF NOT
// EXISTS` cannot add them to a table that already exists, so each missing one is
// added here; existing rows land on NULL, which reads as "not transcribed yet".
const attachmentColumns = new Set(
	(database.query('PRAGMA table_info(attachments)').all() as { name: string }[]).map((column) => column.name)
);
if (!attachmentColumns.has('transcript_error')) {
	database.run('ALTER TABLE attachments ADD COLUMN transcript_error TEXT');
}
if (!attachmentColumns.has('transcribed_at')) {
	database.run('ALTER TABLE attachments ADD COLUMN transcribed_at TEXT');
}

export type AttachmentKind = 'file' | 'audio';

export type Attachment = {
	id: string;
	name: string;
	mimeType: string;
	size: number;
	kind: AttachmentKind;
	/** Filled in once a voice note has been run through a provider. Null while it
	 * has not, which is not an error. */
	transcript: string | null;
	/** Why a voice note has no transcript, when something tried and failed. */
	transcriptError: string | null;
};

/** One voice note waiting to be transcribed, with its bytes already on disk. */
export type UntranscribedAudio = { id: string; path: string; mimeType: string };

type AttachmentRow = {
	id: string;
	message_id: string;
	name: string;
	mime_type: string;
	size: number;
	kind: AttachmentKind;
	transcript: string | null;
	transcript_error: string | null;
};

const AUDIO_EXTENSIONS = ['.webm', '.m4a', '.mp3', '.ogg', '.oga', '.wav', '.flac', '.aac', '.opus'];

/** Audio is decided by MIME type *and* extension. A recorded blob can reach the
 * server with an empty type — the browser decides, not us — and a voice note
 * filed as a plain file would sit there forever without being transcribed. */
export function attachmentKind(name: string, mimeType: string): AttachmentKind {
	if (mimeType.startsWith('audio/')) return 'audio';
	const extension = extname(name).toLowerCase();
	return AUDIO_EXTENSIONS.includes(extension) ? 'audio' : 'file';
}

const SELECT_COLUMNS = 'id, message_id, name, mime_type, size, kind, transcript, transcript_error';

function toAttachment(row: AttachmentRow): Attachment {
	return {
		id: row.id,
		name: row.name,
		mimeType: row.mime_type,
		size: row.size,
		kind: row.kind,
		transcript: row.transcript,
		transcriptError: row.transcript_error
	};
}

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

	const mimeType = input.mimeType || 'application/octet-stream';
	database
		.query(
			'INSERT INTO attachments (id, message_id, name, mime_type, size, kind, path, transcript, transcript_error, transcribed_at, created_at) VALUES (?, NULL, ?, ?, ?, ?, ?, NULL, NULL, NULL, ?)'
		)
		.run(id, input.name.slice(0, 255), mimeType, input.bytes.byteLength, input.kind, path, new Date().toISOString());

	return {
		id,
		name: input.name,
		mimeType,
		size: input.bytes.byteLength,
		kind: input.kind,
		transcript: null,
		transcriptError: null
	};
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
		.query(`SELECT ${SELECT_COLUMNS} FROM attachments WHERE message_id IN (${placeholders}) ORDER BY created_at`)
		.all(...messageIds) as AttachmentRow[];

	const grouped: Record<string, Attachment[]> = {};
	for (const row of rows) {
		(grouped[row.message_id] ??= []).push(toAttachment(row));
	}
	return grouped;
}

/** Voice notes that have never been run through a provider. Failed attempts are
 * left out: retrying a rejected key on every turn would spend a request to be
 * told the same thing. */
export function listUntranscribedAudio(limit = 10): UntranscribedAudio[] {
	const rows = database
		.query(
			`SELECT id, path, mime_type FROM attachments
			 WHERE kind = 'audio' AND transcript IS NULL AND transcript_error IS NULL
			 ORDER BY created_at LIMIT ?`
		)
		.all(limit) as { id: string; path: string; mime_type: string }[];

	return rows.map((row) => ({ id: row.id, path: row.path, mimeType: row.mime_type }));
}

export function recordTranscript(id: string, transcript: string): void {
	database
		.query("UPDATE attachments SET transcript = ?, transcript_error = NULL, transcribed_at = ? WHERE id = ?")
		.run(transcript, new Date().toISOString(), id);
}

/** The failure is kept, not thrown away: the chat shows it under the voice note
 * so "why is there no transcript" has an answer without opening a log. */
export function recordTranscriptError(id: string, error: string): void {
	database
		.query("UPDATE attachments SET transcript_error = ? WHERE id = ?")
		.run(error.slice(0, 300), id);
}