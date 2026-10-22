import type { PageServerLoad } from './$types';

/**
 * Where the wake service lives.
 *
 * The detector is a separate process, so the page cannot use its own origin.
 * Both addresses are configuration: `AGENT_WAKE_WS_URL` for the socket and
 * `AGENT_WAKE_URL` for the service's HTTP endpoints, each defaulting to the
 * local service.
 */
export const load: PageServerLoad = () => ({
	wsUrl: process.env.AGENT_WAKE_WS_URL ?? 'ws://127.0.0.1:8787/ws/audio',
	serviceUrl: (process.env.AGENT_WAKE_URL ?? 'http://127.0.0.1:8787').replace(/\/+$/, '')
});
