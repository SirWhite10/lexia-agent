/** How the sidebar sections open when a page loads. Persisted per browser
 * alongside the appearance preference on Settings → General: this is a device
 * preference about how the shell looks, not workspace state. */
export type SidebarSectionStart = 'remember' | 'collapsed' | 'expanded';

export const SIDEBAR_SECTION_STARTS = [
	{
		value: 'remember',
		label: 'Remember my choices',
		description: 'Keeps the sections you opened open and the rest closed. A new browser starts fully collapsed.'
	},
	{
		value: 'collapsed',
		label: 'Always start collapsed',
		description: 'Every sidebar section starts closed each time you load the app.'
	},
	{
		value: 'expanded',
		label: 'Always start expanded',
		description: 'Every sidebar section starts open each time you load the app.'
	}
] as const;

export const SIDEBAR_SECTION_IDS = ['sub-agents', 'settings'] as const;
export type SidebarSectionId = (typeof SIDEBAR_SECTION_IDS)[number];

const START_KEY = 'lexosa.sidebar.sectionStart';
const OPEN_KEY = 'lexosa.sidebar.openSections';

// Storage is asked of the environment rather than through `$app/env`: this
// module is instantiated from both the client and the server graph, and "is
// storage available here" is the only question it actually needs answered.
function readStorage(key: string): string | null {
	try {
		return globalThis.localStorage?.getItem(key) ?? null;
	} catch {
		return null;
	}
}

function writeStorage(key: string, value: string): void {
	try {
		globalThis.localStorage?.setItem(key, value);
	} catch {
		// Storage can be unavailable (disabled, private mode); the preference then
		// lasts for as long as this page does.
	}
}

function readSectionStart(): SidebarSectionStart {
	const stored = readStorage(START_KEY);
	return SIDEBAR_SECTION_STARTS.find((option) => option.value === stored)?.value ?? 'remember';
}

/** Only boolean flags are accepted; anything else in storage is dropped rather
 * than trusted, so a hand-edited or stale value cannot break the shell. */
function readRememberedSections(): Partial<Record<SidebarSectionId, boolean>> {
	try {
		const parsed: unknown = JSON.parse(readStorage(OPEN_KEY) ?? '{}');
		if (typeof parsed !== 'object' || parsed === null) return {};
		return Object.fromEntries(
			Object.entries(parsed).filter((entry): entry is [SidebarSectionId, boolean] => typeof entry[1] === 'boolean')
		);
	} catch {
		return {};
	}
}

/** Reactive module state lives on an instance rather than in exported bindings:
 * Svelte forbids reassigning exported `$state`, and the shell needs both a mode
 * and a per-section record that survive client-side navigation. */
class SidebarPreferences {
	sectionStart = $state<SidebarSectionStart>(readSectionStart());

	/** What the user last had open, carried across loads. */
	remembered = $state<Partial<Record<SidebarSectionId, boolean>>>(readRememberedSections());

	/** What is on screen right now. Follows the mode until the user toggles. */
	open = $state<Record<SidebarSectionId, boolean>>({
		'sub-agents': this.startsOpen('sub-agents'),
		settings: this.startsOpen('settings')
	});

	/** A browser with nothing remembered starts with every section closed,
	 * which is what a first-time visitor sees. */
	startsOpen(id: SidebarSectionId): boolean {
		if (this.sectionStart === 'collapsed') return false;
		if (this.sectionStart === 'expanded') return true;
		return this.remembered[id] ?? false;
	}

	toggle(id: SidebarSectionId): void {
		this.remembered[id] = !this.open[id];
		this.open[id] = !this.open[id];
		writeStorage(OPEN_KEY, JSON.stringify(this.remembered));
	}

	/** Switches modes. Sections already on screen follow the new mode at once
	 * rather than at the next load; what the user had opened stays remembered,
	 * so returning to "Remember my choices" restores it. */
	setSectionStart(value: SidebarSectionStart): void {
		this.sectionStart = value;
		writeStorage(START_KEY, value);
		for (const id of SIDEBAR_SECTION_IDS) this.open[id] = this.startsOpen(id);
	}
}

export const sidebarPreferences = new SidebarPreferences();