import { dev } from '$app/env';
import type { Handle } from '@sveltejs/kit/hooks';
import { getUser } from './lib/server/auth';
import { startEveSupervisor } from './lib/server/eve-supervisor';
import { sidebarObserverScript } from './lib/dev/sidebar-observer';

// The host supervises EVE (ADR 0001). In development that means a single
// `bun run dev` brings both processes up: the supervisor spawns `eve dev` and
// restarts it if it dies, so the chat never has to ask the operator to babysit
// a second terminal. Production supervises `eve start` against a promoted
// release through the same seam.
if (dev) startEveSupervisor();

// MUTED — set to true to re-arm the sidebar trigger diagnostic.
//
// Injection technique (documented in AGENTS.md, "Dev-only page instrumentation"):
// `transformPageChunk` rewrites the rendered HTML as it streams out, so a script
// lands in <head> at parse time — before hydration. Hand-pasted console snippets
// arm too late to observe pre-hydration behavior at all.
const injectSidebarObserver = false;

// Session identity only. Access control lives with the surface that owns it:
// the (app) route group guards its authenticated shell in +layout.server.ts.
export const handle: Handle = async ({ event, resolve }) => {
	event.locals.user = getUser(event);
	return resolve(event, {
		transformPageChunk: ({ html }) =>
			dev && injectSidebarObserver
				? html.replace('</head>', `<script>${sidebarObserverScript}</script>\n</head>`)
				: html,
	});
};
