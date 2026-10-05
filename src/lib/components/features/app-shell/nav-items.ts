import MessageSquareIcon from '@lucide/svelte/icons/message-square';
import SettingsIcon from '@lucide/svelte/icons/settings';
import WorkflowIcon from '@lucide/svelte/icons/workflow';
import type { Component } from 'svelte';
import type { SubAgent } from '#lib/server/agents.js';

export type NavItem = {
	title: string;
	href: string;
	icon: Component;
	/** Path prefix for an entry that stays lit across a whole section, rather
	 * than only on its own page. The Settings tab needs it because it and the
	 * General sub-page share an href. */
	section?: string;
};

export type SectionItem = {
	title: string;
	href: string;
};

/** Top-level destinations; mirrored in the mobile tab bar. */
export const chatItem: NavItem = { title: 'Chat', href: '/', icon: MessageSquareIcon };
export const workflowsItem: NavItem = { title: 'Workflows', href: '/workflows', icon: WorkflowIcon };
export const settingsItem: NavItem = {
	title: 'Settings',
	href: '/settings/general',
	icon: SettingsIcon,
	section: '/settings'
};
/** Settings accordion sub-sections; one stack page per item. Thinking closes the
 * list: it configures how the agent routes, which is where that belongs. */
export const settingsItems: SectionItem[] = [
	{ title: 'General', href: '/settings/general' },
	{ title: 'Accounts', href: '/settings/accounts' },
	{ title: 'Providers', href: '/settings/providers' },
	{ title: 'Models', href: '/settings/models' },
	{ title: 'Thinking', href: '/thinking' }
];

export const primaryNav: NavItem[] = [chatItem, workflowsItem, settingsItem];

/** Which entry is highlighted. The whole item is passed rather than a bare
 * href: the Settings tab and the General sub-page have the same href, so only
 * the entry itself can say which of the two is meant. */
export function isNavActive(item: NavItem | SectionItem, pathname: string): boolean {
	const section = 'section' in item ? item.section : undefined;
	if (section) return pathname.startsWith(section);
	if (item.href === '/') return pathname === '/' || pathname.startsWith('/agents');
	return pathname === item.href;
}

/** Header title: static for known routes, sub-agent name for /agents/[id]. */
export function titleForPath(pathname: string, subAgents: SubAgent[]): string {
	if (pathname === '/') return 'Chat';
	if (pathname.startsWith('/workflows')) return 'Workflows';
	if (pathname.startsWith('/thinking')) return 'Thinking';
	if (pathname === '/agents') return 'Sub-agents';
	if (pathname.startsWith('/agents/')) {
		const id = pathname.slice('/agents/'.length);
		return subAgents.find((sub) => sub.id === id)?.name ?? 'Sub-agents';
	}
	if (pathname.startsWith('/settings')) {
		return settingsItems.find((item) => pathname === item.href)?.title ?? 'Settings';
	}
	return 'Lexosa';
}
