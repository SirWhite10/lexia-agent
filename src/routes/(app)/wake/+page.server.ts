import { wakeWsUrl } from '#lib/server/wake-service.js';
import type { PageServerLoad } from './$types';

/**
 * Where the wake service lives.
 *
 * The detector is a separate process, so the page cannot use its own origin
 * for it. The socket address is configuration: `AGENT_WAKE_WS_URL`, defaulting
 * to the local service the way `agent-wake/serve.ts` binds it.
 */
export const load: PageServerLoad = () => ({ wsUrl: wakeWsUrl() });
