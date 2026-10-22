import { join, normalize, relative, resolve } from 'node:path';

const buildDirectory = resolve(process.cwd(), 'web/build');

export async function staticWebResponse(request: Request): Promise<Response | null> {
	const url = new URL(request.url);
	if (url.pathname === '/' || url.pathname === '/login' || url.pathname === '/logout') return indexResponse(request);
	const requested = decodeURIComponent(url.pathname).replace(/^\/+/, '');
	if (!requested || requested.includes('\0')) return new Response('Bad request', { status: 400 });
	const candidate = resolve(join(buildDirectory, normalize(requested)));
	if (relative(buildDirectory, candidate).startsWith('..')) return new Response('Not found', { status: 404 });
	const file = Bun.file(candidate);
	if (await file.exists()) return new Response(request.method === 'HEAD' ? null : file, { headers: { 'content-type': file.type || 'application/octet-stream' } });
	return indexResponse(request);
}

function indexResponse(request: Request): Response {
	const file = Bun.file(join(buildDirectory, 'index.html'));
	return new Response(request.method === 'HEAD' ? null : file, { headers: { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-cache' } });
}
