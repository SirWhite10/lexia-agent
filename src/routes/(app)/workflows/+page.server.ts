import { readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import type { PageServerLoad } from './$types';

// OPERATIONS.md: workflows are documented directories under workflow/. The page
// only reads that contract surface; workflow scripts are never imported here.
const workflowsRoot = resolve(process.cwd(), 'workflow');

export const load: PageServerLoad = () => {
	let workflows: { name: string }[] = [];
	try {
		workflows = readdirSync(workflowsRoot, { withFileTypes: true })
			.filter((entry) => entry.isDirectory())
			.map((entry) => ({ name: entry.name }))
			.sort((a, b) => a.name.localeCompare(b.name));
	} catch {
		// No workflow root on a fresh installation; the empty state explains it.
	}

	return { workflows };
};
