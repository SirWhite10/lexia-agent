/**
 * Where the wake service lives.
 *
 * The detector is a separate process from this host, so the browser cannot
 * use this origin for it. Both addresses are configuration: the socket the
 * page opens and the service's HTTP endpoints. The defaults are the local
 * service the way `agent-wake/serve.ts` binds it.
 */

const DEFAULT_WS_URL = 'ws://127.0.0.1:8787/ws/audio';
const DEFAULT_SERVICE_URL = 'http://127.0.0.1:8787';

/** The socket the page opens. A relative value is resolved against the page's origin. */
export function wakeWsUrl(): string {
	return process.env.AGENT_WAKE_WS_URL?.trim() || DEFAULT_WS_URL;
}

/** The service's HTTP endpoints, without a trailing slash. */
export function wakeServiceUrl(): string {
	return (process.env.AGENT_WAKE_URL?.trim() || DEFAULT_SERVICE_URL).replace(/\/+$/, '');
}
