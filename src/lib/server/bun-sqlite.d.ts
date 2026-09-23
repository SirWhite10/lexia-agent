declare module 'bun:sqlite' {
	export class Database {
		constructor(filename: string);
		run(query: string): void;
		query<T = unknown>(query: string): {
			get(...params: unknown[]): T;
			run(...params: unknown[]): void;
		};
	}
}

declare module 'node:fs' {
	export function mkdirSync(path: string, options?: { recursive?: boolean }): void;
}

declare module 'node:path' {
	export function dirname(path: string): string;
	export function resolve(...paths: string[]): string;
}

declare const process: { cwd(): string };

declare const Bun: {
	password: {
		hash(password: string): Promise<string>;
		verify(password: string, hash: string): Promise<boolean>;
	};
};
