import { readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import type { PageServerLoad } from './$types';

const releasesRoot = resolve(process.cwd(), 'app-data', 'releases');

export const load: PageServerLoad = () => {
	let releases: { name: string }[] = [];
	try {
		releases = readdirSync(releasesRoot, { withFileTypes: true })
			.map((entry) => ({ name: entry.name }))
			.sort((a, b) => a.name.localeCompare(b.name));
	} catch {
		// No releases root on a fresh installation; the empty state explains it.
	}

	return { releases };
};
