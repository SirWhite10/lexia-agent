#!/usr/bin/env bun
/**
 * Agent directory tooling.
 *
 *   bun scripts/agent.ts new <name> [--parent <dir>]   create an agent directory from the template
 *   bun scripts/agent.ts pack                          rebuild templates/agent.zip from templates/agent/
 *   bun scripts/agent.ts check <dir>                   validate a directory against the template
 *
 * An agent directory is data, not source: nothing in it is compiled, and the
 * host watches it so changes take effect without a rebuild. See
 * agents/default/README.md for the structure and the watch contract, and ADR 0005
 * for why the tree exists separately from the binary.
 *
 * The template is committed as a reviewable tree at templates/agent/ rather than
 * as an opaque archive, so a change to the agent's shape is a diff a reviewer
 * can read. The zip is built from it and committed alongside so that creating an
 * agent does not require the tree.
 *
 * The zip is written here rather than shelled out to `zip`, because a build step
 * that depends on an external binary is a build step that fails on a machine
 * without it. Entries are stored deflated with a fixed timestamp so the same
 * tree always produces the same bytes.
 */

import { deflateRawSync } from 'node:zlib';
import { readdirSync, readFileSync, writeFileSync, mkdirSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';

const ROOT = resolve(import.meta.dir, '..');
const TEMPLATE_DIR = join(ROOT, 'templates/agent');
const TEMPLATE_ZIP = join(ROOT, 'templates/agent.zip');

const TOKENS = ['{{AGENT_NAME}}', '{{AGENT_TITLE}}', '{{AGENT_DESCRIPTION}}'] as const;
type Token = (typeof TOKENS)[number];

// ---------------------------------------------------------------- zip format

/** Fixed MS-DOS timestamp (2026-01-01 00:00:00) so identical trees zip identically. */
const DOS_TIME = 0;
const DOS_DATE = ((2026 - 1980) << 9) | (1 << 5) | 1;

const CRC_TABLE = (() => {
	const table = new Uint32Array(256);
	for (let i = 0; i < 256; i++) {
		let c = i;
		for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
		table[i] = c >>> 0;
	}
	return table;
})();

function crc32(bytes: Uint8Array): number {
	let c = 0xffffffff;
	for (let i = 0; i < bytes.length; i++) c = CRC_TABLE[(c ^ bytes[i]) & 0xff] ^ (c >>> 8);
	return (c ^ 0xffffffff) >>> 0;
}

type Entry = { name: string; data: Uint8Array };

/**
 * Build a ZIP archive from `entries`, in the order given, with directory
 * records for every path component so the archive extracts into the right
 * shape even where the extractor does not infer directories.
 */
function buildZip(entries: Entry[]): Uint8Array {
	const chunks: Uint8Array[] = [];
	const central: Uint8Array[] = [];
	let offset = 0;

	const push = (parts: Uint8Array[]) => {
		const blob = Buffer.concat(parts.map((p) => Buffer.from(p)));
		chunks.push(new Uint8Array(blob));
		return blob.length;
	};

	const seenDirs = new Set<string>();
	const dirRecord = (name: string) => {
		if (seenDirs.has(name)) return;
		seenDirs.add(name);
		const nameBytes = Buffer.from(name, 'utf8');
		const local = Buffer.alloc(30 + nameBytes.length);
		local.writeUInt32LE(0x04034b50, 0);
		local.writeUInt16LE(20, 4); // version needed
		local.writeUInt16LE(0, 6); // flags
		local.writeUInt16LE(0, 8); // method: stored
		local.writeUInt16LE(DOS_TIME, 10);
		local.writeUInt16LE(DOS_DATE, 12);
		local.writeUInt32LE(0, 14); // crc of an empty directory entry
		local.writeUInt32LE(0, 18); // compressed size
		local.writeUInt32LE(0, 22); // uncompressed size
		local.writeUInt16LE(nameBytes.length, 26);
		local.writeUInt16LE(0, 28); // extra length
		nameBytes.copy(local, 30);
		const size = push([local]);
		const cd = Buffer.alloc(46 + nameBytes.length);
		cd.writeUInt32LE(0x02014b50, 0);
		cd.writeUInt16LE(20, 4); // version made by
		cd.writeUInt16LE(20, 6); // version needed
		cd.writeUInt16LE(0, 8);
		cd.writeUInt16LE(0, 10);
		cd.writeUInt16LE(DOS_TIME, 12);
		cd.writeUInt16LE(DOS_DATE, 14);
		cd.writeUInt32LE(0, 16);
		cd.writeUInt32LE(0, 20);
		cd.writeUInt32LE(0, 24);
		cd.writeUInt16LE(nameBytes.length, 28);
		cd.writeUInt16LE(0, 30); // extra
		cd.writeUInt16LE(0, 32); // comment
		cd.writeUInt16LE(0, 34); // disk
		cd.writeUInt16LE(0, 36); // internal attrs
		cd.writeUInt32LE(((0o040755 << 16) >>> 0), 38); // external attrs: directory
		cd.writeUInt32LE(offset, 42);
		nameBytes.copy(cd, 46);
		central.push(new Uint8Array(cd));
		offset += size;
	};

	for (const entry of entries) {
		// Every prefix that names a directory gets a record; the last component
		// is the file itself and must not become one, or every offset after it
		// desyncs.
		const parts = entry.name.split('/');
		const parents = parts.slice(0, -1);
		for (let i = 1; i <= parents.length; i++) dirRecord(`${parents.slice(0, i).join('/')}/`);

		const nameBytes = Buffer.from(entry.name, 'utf8');
		const crc = crc32(entry.data);
		const deflated = new Uint8Array(deflateRawSync(Buffer.from(entry.data), { level: 9 }));
		// Fall back to stored when deflate does not actually shrink the entry.
		const useDeflate = deflated.length < entry.data.length;
		const body = useDeflate ? deflated : entry.data;
		const method = useDeflate ? 8 : 0;

		const local = Buffer.alloc(30 + nameBytes.length);
		local.writeUInt32LE(0x04034b50, 0);
		local.writeUInt16LE(20, 4);
		local.writeUInt16LE(0, 6);
		local.writeUInt16LE(method, 8);
		local.writeUInt16LE(DOS_TIME, 10);
		local.writeUInt16LE(DOS_DATE, 12);
		local.writeUInt32LE(crc, 14);
		local.writeUInt32LE(body.length, 18);
		local.writeUInt32LE(entry.data.length, 22);
		local.writeUInt16LE(nameBytes.length, 26);
		local.writeUInt16LE(0, 28);
		nameBytes.copy(local, 30);
		const size = push([local, body]);

		const cd = Buffer.alloc(46 + nameBytes.length);
		cd.writeUInt32LE(0x02014b50, 0);
		cd.writeUInt16LE(20, 4);
		cd.writeUInt16LE(20, 6);
		cd.writeUInt16LE(0, 8);
		cd.writeUInt16LE(method, 10);
		cd.writeUInt16LE(DOS_TIME, 12);
		cd.writeUInt16LE(DOS_DATE, 14);
		cd.writeUInt32LE(crc, 16);
		cd.writeUInt32LE(body.length, 20);
		cd.writeUInt32LE(entry.data.length, 24);
		cd.writeUInt16LE(nameBytes.length, 28);
		cd.writeUInt16LE(0, 30);
		cd.writeUInt16LE(0, 32);
		cd.writeUInt16LE(0, 34);
		cd.writeUInt16LE(0, 36);
		cd.writeUInt32LE(((0o100644 << 16) >>> 0), 38); // external attrs: regular file
		cd.writeUInt32LE(offset, 42);
		nameBytes.copy(cd, 46);
		central.push(new Uint8Array(cd));
		offset += size;
	}

	const cdBlob = Buffer.concat(central.map((c) => Buffer.from(c)));
	const eocd = Buffer.alloc(22);
	eocd.writeUInt32LE(0x06054b50, 0);
	eocd.writeUInt16LE(0, 4);
	eocd.writeUInt16LE(0, 6);
	eocd.writeUInt16LE(central.length, 8);
	eocd.writeUInt16LE(central.length, 10);
	eocd.writeUInt32LE(cdBlob.length, 12);
	eocd.writeUInt32LE(offset, 16);
	eocd.writeUInt16LE(0, 20);

	return new Uint8Array(Buffer.concat([...chunks.map((c) => Buffer.from(c)), cdBlob, eocd]));
}

// -------------------------------------------------------------------- shared

/** Every file under `dir`, relative and sorted, so two trees compare stably. */
function walk(dir: string): string[] {
	const out: string[] = [];
	for (const entry of readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
		const full = join(dir, entry.name);
		if (entry.isDirectory()) out.push(...walk(full).map((p) => `${entry.name}/${p}`));
		else out.push(entry.name);
	}
	return out;
}

function fail(message: string): never {
	console.error(`agent: ${message}`);
	process.exit(1);
}

function readTemplateFiles(): Map<string, Uint8Array> {
	const files = new Map<string, Uint8Array>();
	for (const rel of walk(TEMPLATE_DIR)) {
		files.set(rel, new Uint8Array(readFileSync(join(TEMPLATE_DIR, rel))));
	}
	return files;
}

function substitute(data: Uint8Array, values: Record<Token, string>): Uint8Array {
	let text = new TextDecoder().decode(data);
	for (const token of TOKENS) text = text.split(token).join(values[token]);
	return new TextEncoder().encode(text);
}

/** Slug rules: lowercase, alphanumeric with single dashes, no leading or trailing dash. */
function assertSlug(name: string): void {
	if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(name)) {
		fail(`"${name}" is not a valid agent name — use lowercase words joined by single dashes, e.g. "code-reviewer"`);
	}
}

function titleFromSlug(name: string): string {
	return name
		.split('-')
		.map((word) => word[0].toUpperCase() + word.slice(1))
		.join(' ');
}

// ----------------------------------------------------------------- commands

/** Repo-relative when the path is inside the repo, absolute when it is not. */
function displayPath(path: string): string {
	const rel = relative(ROOT, path);
	return rel.startsWith('..') ? path : rel;
}

function cmdNew(args: string[]): void {
	const name = args[0];
	if (!name) fail('new requires a name: bun scripts/agent.ts new <name> [--parent <dir>]');
	assertSlug(name);

	const parentIndex = args.indexOf('--parent');
	const parent =
		parentIndex === -1 ? join(ROOT, 'agents/default/subagents') : resolve(ROOT, args[parentIndex + 1] ?? '');
	const target = join(parent, name);

	if (statSync(target, { throwIfNoEntry: false })) {
		fail(`${displayPath(target)} already exists — refusing to overwrite an agent directory`);
	}

	const values: Record<Token, string> = {
		'{{AGENT_NAME}}': name,
		'{{AGENT_TITLE}}': titleFromSlug(name),
		'{{AGENT_DESCRIPTION}}': `Describe what ${titleFromSlug(name)} is for.`
	};

	for (const [rel, data] of readTemplateFiles()) {
		const out = join(target, rel);
		mkdirSync(dirname(out), { recursive: true });
		writeFileSync(out, substitute(data, values));
	}

	console.log(`created ${displayPath(target)}`);
	console.log(`  edit agent.toml and instructions/core.md next — both are read on change, no restart needed`);
}

function cmdPack(): void {
	const entries: Entry[] = [];
	for (const [rel, data] of readTemplateFiles()) entries.push({ name: `agent/${rel}`, data });

	const zip = buildZip(entries);
	writeFileSync(TEMPLATE_ZIP, zip);
	const raw = statSync(TEMPLATE_ZIP).size;
	const deflated = new Uint8Array(deflateRawSync(zip, { level: 9 })).length;
	console.log(`packed ${entries.length} entries -> ${relative(ROOT, TEMPLATE_ZIP)} (${raw} B raw, ${deflated} B deflated)`);
}

function cmdCheck(args: string[]): void {
	const dir = args[0];
	if (!dir) fail('check requires a directory: bun scripts/agent.ts check <dir>');
	const root = resolve(ROOT, dir);
	if (!statSync(root, { throwIfNoEntry: false })?.isDirectory()) fail(`${dir} is not a directory`);

	const expected = [...readTemplateFiles().keys()].sort();
	// A parent agent legitimately contains nested agent directories, and each of
	// those is validated by running `check` on it directly. Counting them here
	// would make every agent with a sub-agent look like template drift.
	const actual = walk(root)
		.filter((p) => !p.endsWith('.DS_Store') && !/^subagents\/[^/]+\//.test(p))
		.sort();

	const problems: string[] = [];
	for (const file of expected) {
		if (!actual.includes(file)) problems.push(`missing ${file}`);
	}
	for (const file of actual) {
		if (!expected.includes(file)) problems.push(`unexpected ${file}`);
	}

	for (const [rel] of readTemplateFiles()) {
		const path = join(root, rel);
		if (!statSync(path, { throwIfNoEntry: false })?.isFile()) continue;
		// A token inside backticks documents the placeholder rather than being one:
		// an agent's own README explains what the tokens are, and flagging that
		// would make a correct directory look unfinished.
		const withoutCode = readFileSync(path, 'utf8').replace(/`[^`]*`/g, '');
		for (const token of TOKENS) {
			if (withoutCode.includes(token)) problems.push(`unsubstituted ${token} in ${rel}`);
		}
	}

	// The manifest is the role card, so the two fields that must exist are
	// checked rather than assumed.
	const manifest = readFileSync(join(root, 'agent.toml'), 'utf8');
	if (!/^name\s*=\s*"[^"]+"/m.test(manifest)) problems.push('agent.toml has no name');
	if (!/^scope\s*=\s*"[^"]+"/m.test(manifest)) problems.push('agent.toml has no memory scope');

	if (problems.length) {
		for (const p of problems) console.error(`  ${p}`);
		fail(`${dir} does not match the template`);
	}
	console.log(`${dir} matches the template (nested agents under subagents/ are checked separately)`);
}

// --------------------------------------------------------------------- entry

const [command, ...args] = process.argv.slice(2);
switch (command) {
	case 'new':
		cmdNew(args);
		break;
	case 'pack':
		cmdPack();
		break;
	case 'check':
		cmdCheck(args);
		break;
	default:
		console.error(
			['usage:', '  bun scripts/agent.ts new <name> [--parent <dir>]', '  bun scripts/agent.ts pack', '  bun scripts/agent.ts check <dir>'].join('\n')
		);
		process.exit(1);
}