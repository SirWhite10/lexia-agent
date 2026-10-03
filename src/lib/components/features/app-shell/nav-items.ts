import MessageSquareIcon from '@lucide/svelte/icons/message-square';
import SettingsIcon from '@lucide/svelte/icons/settings';
import WorkflowIcon from '@lucide/svelte/icons/workflow';
import type { Component } from 'svelte';
import type { SubAgent } from '#lib/server/agents.js';

export type NavItem = {
	title: string;
	href: string;
	icon: Component;
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
};

export const primaryNav: NavItem[] = [chatItem, workflowsItem, settingsItem];

/** Settings accordion sub-sections; one stack page per item under /settings. */
export const settingsItems: SectionItem[] = [
	{ title: 'General', href: '/settings/general' },
	{ title: 'Integrations', href: '/settings/integrations' },
	{ title: 'Accounts', href: '/settings/accounts' },
	{ title: 'Models', href: '/settings/models' },
];

/** Exact match at the root; sub-agent routes light up the Chat tab. */
export function isNavActive(href: string, pathname: string): boolean {
	if (href === '/') return pathname === '/' || pathname.startsWith('/agents');
	if (href.startsWith('/settings')) return pathname.startsWith('/settings');
	return pathname === href || pathname.startsWith(`${href}/`);
}

/** Header title: static for known routes, sub-agent name for /agents/[id]. */
export function titleForPath(pathname: string, subAgents: SubAgent[]): string {
	if (pathname === '/') return 'Chat';
	if (pathname.startsWith('/workflows')) return 'Workflows';
	if (pathname === '/agents') return 'Sub-agents';
	if (pathname.startsWith('/agents/')) {
		const id = pathname.slice('/agents/'.length);
		return subAgents.find((sub) => sub.id === id)?.name ?? 'Sub-agents';
	}
	if (pathname.startsWith('/settings')) {
		return settingsItems.find((item) => pathname.startsWith(item.href))?.title ?? 'Settings';
	}
	return 'Lexia';
}
