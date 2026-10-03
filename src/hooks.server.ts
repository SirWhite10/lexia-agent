import { dev } from '$app/env';
import type { Handle } from '@sveltejs/kit/hooks';
import { getUser } from './lib/server/auth';
import { startEveSupervisor } from './lib/server/eve-supervisor';
import { sidebarObserverScript } from './lib/dev/sidebar-observer';

// The host supervises EVE on every boot (ADR 0003): the supervisor spawns
// `eve dev` from the authored source tree and restarts it if it dies. The
// gate that used to be `dev` only existed because a production build expected
// an already-promoted release to be running; there is no release path, so
// there is nothing to distinguish.
startEveSupervisor();

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
