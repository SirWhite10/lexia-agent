const sessionCookieName = 'agent_wake_session';
const sessionLifetimeMs = 8 * 60 * 60 * 1000;

type Session = {
	username: string;
	expiresAt: number;
};

const sessions = new Map<string, Session>();

function configuredUsername(): string {
	return Bun.env.AGENT_WAKE_USERNAME?.trim() ?? '';
}

function configuredPasswordHash(): string {
	return Bun.env.AGENT_WAKE_PASSWORD_HASH?.trim() ?? '';
}

function configuredPassword(): string {
	return Bun.env.AGENT_WAKE_PASSWORD ?? '';
}

export function authConfigured(): boolean {
	return configuredUsername().length > 0 && (configuredPasswordHash().length > 0 || configuredPassword().length > 0);
}

function parseCookies(header: string | null): Map<string, string> {
	const cookies = new Map<string, string>();
	if (!header) return cookies;
	for (const part of header.split(';')) {
		const separator = part.indexOf('=');
		if (separator < 0) continue;
		const name = part.slice(0, separator).trim();
		const value = part.slice(separator + 1).trim();
		if (!name) continue;
		try {
			cookies.set(name, decodeURIComponent(value));
		} catch {
			cookies.set(name, value);
		}
	}
	return cookies;
}

function isSecureRequest(request: Request): boolean {
	const forwarded = request.headers.get('x-forwarded-proto')?.split(',')[0]?.trim().toLowerCase();
	return forwarded === 'https' || new URL(request.url).protocol === 'https:';
}

function cookie(token: string, request: Request, maxAgeSeconds: number): string {
	const secure = isSecureRequest(request) ? '; Secure' : '';
	return `${sessionCookieName}=${encodeURIComponent(token)}; Path=/; HttpOnly; SameSite=Lax; Max-Age=${maxAgeSeconds}${secure}`;
}

function pruneExpiredSessions(): void {
	const now = Date.now();
	for (const [token, session] of sessions) {
		if (session.expiresAt <= now) sessions.delete(token);
	}
}

export function getSession(request: Request): Session | null {
	pruneExpiredSessions();
	const token = parseCookies(request.headers.get('cookie')).get(sessionCookieName);
	if (!token) return null;
	const session = sessions.get(token);
	if (!session || session.expiresAt <= Date.now()) {
		if (token) sessions.delete(token);
		return null;
	}
	return session;
}

export function requireSession(request: Request): Session | null {
	return getSession(request);
}

async function readCredentials(request: Request): Promise<{ username: string; password: string } | null> {
	try {
		const body = (await request.json()) as { username?: unknown; password?: unknown };
		return {
			username: typeof body.username === 'string' ? body.username.trim() : '',
			password: typeof body.password === 'string' ? body.password : ''
		};
	} catch {
		try {
			const form = await request.formData();
			return {
				username: String(form.get('username') ?? '').trim(),
				password: String(form.get('password') ?? '')
			};
		} catch {
			return null;
		}
	}
}

async function passwordMatches(password: string): Promise<boolean> {
	const hash = configuredPasswordHash();
	if (hash) return Bun.password.verify(password, hash);
	return password.length > 0 && password === configuredPassword();
}

export async function login(request: Request): Promise<Response> {
	if (request.method !== 'POST') return new Response('Method not allowed', { status: 405, headers: { Allow: 'POST' } });
	if (!authConfigured()) {
		return Response.json(
			{ error: 'auth_not_configured', message: 'Set AGENT_WAKE_USERNAME and AGENT_WAKE_PASSWORD (or AGENT_WAKE_PASSWORD_HASH) before using the test host.' },
			{ status: 503 }
		);
	}
	const credentials = await readCredentials(request);
	if (!credentials || credentials.username !== configuredUsername() || !(await passwordMatches(credentials.password))) {
		return Response.json({ error: 'invalid_credentials', message: 'That username or password is not correct.' }, { status: 401 });
	}
	const token = crypto.randomUUID();
	sessions.set(token, { username: credentials.username, expiresAt: Date.now() + sessionLifetimeMs });
	return Response.json(
		{ authenticated: true, user: { name: credentials.username } },
		{ headers: { 'Set-Cookie': cookie(token, request, sessionLifetimeMs / 1000) } }
	);
}

export function session(request: Request): Response {
	const current = getSession(request);
	return Response.json({
		authenticated: Boolean(current),
		configured: authConfigured(),
		user: current ? { name: current.username } : null
	});
}

export function logout(request: Request): Response {
	const token = parseCookies(request.headers.get('cookie')).get(sessionCookieName);
	if (token) sessions.delete(token);
	return Response.json(
		{ authenticated: false },
		{ headers: { 'Set-Cookie': cookie('', request, 0) } }
	);
}

export { sessionCookieName };
