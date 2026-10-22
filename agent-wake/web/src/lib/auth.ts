export interface SessionInfo {
  authenticated: boolean;
  setupRequired: boolean;
  username: string | null;
}

export class AuthRequestError extends Error {
  constructor(
    message: string,
    readonly status: number
  ) {
    super(message);
    this.name = 'AuthRequestError';
  }
}

interface JsonObject {
  [key: string]: unknown;
}

function isObject(value: unknown): value is JsonObject {
  return typeof value === 'object' && value !== null;
}

function optionalBoolean(value: JsonObject, names: string[]): boolean | undefined {
  for (const name of names) {
    const candidate = value[name];
    if (typeof candidate === 'boolean') return candidate;
  }
  return undefined;
}

function usernameField(value: JsonObject): string | null {
  for (const name of ['username', 'name', 'user']) {
    const candidate = value[name];
    if (typeof candidate === 'string' && candidate.trim()) return candidate.trim();
    if (isObject(candidate)) {
      const nested = candidate.username ?? candidate.name;
      if (typeof nested === 'string' && nested.trim()) return nested.trim();
    }
  }
  return null;
}

function configuredField(value: JsonObject): boolean | undefined {
  const configured = optionalBoolean(value, ['configured', 'initialized']);
  return configured === undefined ? undefined : !configured;
}

async function responseJson(response: Response): Promise<JsonObject> {
  try {
    const value: unknown = await response.json();
    return isObject(value) ? value : {};
  } catch {
    return {};
  }
}

function responseMessage(value: JsonObject, fallback: string): string {
  for (const key of ['message', 'error']) {
    const candidate = value[key];
    if (typeof candidate === 'string' && candidate.trim()) return candidate.trim().slice(0, 300);
  }
  return fallback;
}

export async function fetchSession(): Promise<SessionInfo> {
  const response = await fetch('/api/session', {
    method: 'GET',
    credentials: 'same-origin',
    cache: 'no-store',
    headers: { Accept: 'application/json' }
  });

  if (response.status === 401 || response.status === 403) {
    return { authenticated: false, setupRequired: false, username: null };
  }
  if (!response.ok) {
    throw new AuthRequestError(`Session check failed (${response.status}).`, response.status);
  }

  const value = await responseJson(response);
  const username = usernameField(value);
  const authenticated = optionalBoolean(value, ['authenticated', 'isAuthenticated', 'loggedIn']) ?? username !== null;
  const setupRequired = optionalBoolean(value, ['setupRequired', 'needsSetup']) ?? configuredField(value) ?? !authenticated;
  return { authenticated, setupRequired, username };
}

export async function login(username: string, password: string): Promise<SessionInfo> {
  const response = await fetch('/api/login', {
    method: 'POST',
    credentials: 'same-origin',
    headers: { Accept: 'application/json', 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, password })
  });
  const value = await responseJson(response);

  if (!response.ok) {
    const fallback = response.status === 401 ? 'The username or password was not accepted.' : `Sign in failed (${response.status}).`;
    throw new AuthRequestError(responseMessage(value, fallback), response.status);
  }

  if (optionalBoolean(value, ['authenticated', 'ok']) ?? true) {
    return fetchSession();
  }
  throw new AuthRequestError(responseMessage(value, 'The host did not establish a session.'), response.status);
}

export async function logout(): Promise<void> {
  const response = await fetch('/api/logout', {
    method: 'POST',
    credentials: 'same-origin',
    headers: { Accept: 'application/json' }
  });
  if (!response.ok) throw new AuthRequestError(`Sign out failed (${response.status}).`, response.status);
}
