<script lang="ts">
	import type { Component } from 'svelte';

	/**
	 * Svelte artifacts are real Svelte 5 components: the source is compiled in
	 * the browser with `svelte/compiler` and mounted into a wrapper element.
	 *
	 * The contract with the agent is a normal runes-mode component file. It is
	 * compiled for the client with `runes: true` and `css: 'injected'`, then
	 * the generated module is evaluated with `new Function`. The generated code
	 * imports `svelte/internal/client` (under the name `$`) and part of the
	 * `svelte` runtime, so those imports are stripped and the bindings are
	 * passed in as parameters instead — the artifact then runs on exactly the
	 * internals this page already ships.
	 *
	 * Nothing from the host page is in scope: an artifact that reaches for app
	 * state finds nothing, and an artifact that fails to compile or throws at
	 * runtime reports into an inline panel instead of unmounting the chat.
	 *
	 * Compilation is memoised by source text, so re-rendering the same source
	 * never recompiles.
	 */

	type ArtifactError = {
		phase: 'compile' | 'runtime';
		message: string;
		line?: number;
		source?: string;
	};

	type Compiled = { component: Component } | { error: ArtifactError };

	interface Props {
		source: string;
		class?: string;
	}

	let { source, class: className }: Props = $props();

	/**
	 * Names bound in the evaluated module. `'$'` is the identifier the compiler
	 * gives its `svelte/internal/client` import, so it has to be bound under
	 * that exact name; the rest are the runtime functions an artifact may
	 * reference by name.
	 */
	const INJECTED = [
		'$',
		'untrack',
		'flushSync',
		'tick',
		'mount',
		'unmount',
		'onMount',
		'onDestroy',
		'beforeUpdate',
		'afterUpdate',
		'createEventDispatcher',
		'getContext',
		'setContext',
		'hasContext',
		'getAllContexts'
	] as const;

	const cache = new Map<string, Compiled>();

	let host = $state<HTMLDivElement | null>(null);
	let error = $state<ArtifactError | null>(null);
	let compiling = $state(true);

	async function compileArtifact(text: string): Promise<Compiled> {
		const cached = cache.get(text);
		if (cached) return cached;

		let result: Compiled;
		try {
			const [{ compile }, internal, svelte] = await Promise.all([
				import('svelte/compiler'),
				import('svelte/internal/client'),
				import('svelte')
			]);

			const compiled = compile(text, {
				generate: 'client',
				runes: true,
				css: 'injected',
				filename: 'artifact.svelte'
			});

			const runtime = svelte as unknown as Record<string, unknown>;
			const module = new Function(
				...INJECTED,
				compiled.js.code
					// `import x from 'y'`, `import { a, b } from 'y'` and `import 'y'`.
					.replace(/^import\s+[\s\S]*?\sfrom\s+['"][^'"]*['"];?/gm, '')
					.replace(/^import\s+['"][^'"]*['"];?/gm, '')
					// The compiled module is one `export default function`; the
					// factory returns the component instead.
					.replace(/export\s+default\s*/, 'return ')
			);

			result = { component: module(internal, ...INJECTED.slice(1).map((name) => runtime[name])) as Component };
		} catch (cause) {
			const failure = cause as { message?: string; start?: { line: number } };
			const line = failure.start?.line;
			result = {
				error: {
					phase: 'compile',
					message: failure.message ?? String(cause),
					line,
					source: line ? text.split('\n')[line - 1]?.trim() : undefined
				}
			};
		}

		cache.set(text, result);
		return result;
	}

	$effect(() => {
		const target = host;
		const text = source;
		if (!target) return;

		let disposed = false;
		let teardown: (() => void) | undefined;
		error = null;
		compiling = true;

		(async () => {
			const result = await compileArtifact(text);
			if (disposed) return;

			if ('error' in result) {
				error = result.error;
				compiling = false;
				return;
			}

			try {
				const { mount, unmount } = await import('svelte');
				target.replaceChildren();
				const app = mount(result.component, { target });
				teardown = () => unmount(app);
				compiling = false;
			} catch (cause) {
				const failure = cause as Error;
				const stackLine = /:(\d+):\d+/.exec(failure.stack ?? '');
				error = {
					phase: 'runtime',
					message: failure.message ?? String(cause),
					line: stackLine ? Number(stackLine[1]) : undefined
				};
				compiling = false;
			}
		})();

		return () => {
			disposed = true;
			try {
				teardown?.();
			} catch {
				// A teardown that throws must not take the chat down with it.
			}
		};
	});
</script>

<div data-kit="render-svelte" class={className}>
	{#if error}
		<div
			role="alert"
			class="rounded-(--kit-radius-md) border border-(--kit-danger) bg-(--kit-code-background) p-(--kit-space-sm) text-[length:var(--kit-text-xs)]"
		>
			<p class="font-medium text-[color:var(--kit-danger)]">
				{error.phase === 'compile' ? 'The artifact did not compile.' : 'The artifact threw while running.'}
			</p>
			<p class="mt-(--kit-space-xs) font-mono text-[color:var(--kit-code-foreground)] break-words">{error.message}</p>
			{#if error.line}
				<p class="mt-(--kit-space-xs) text-[color:var(--kit-muted-foreground)]">
					Line {error.line}{#if error.source}<span class="ml-(--kit-space-xs) block font-mono text-[color:var(--kit-code-foreground)] break-all"
							>{error.source}</span
						>{/if}
				</p>
			{/if}
		</div>
	{:else if compiling}
		<p class="px-(--kit-space-xs) py-(--kit-space-lg) text-[length:var(--kit-text-sm)] text-[color:var(--kit-muted-foreground)]">
			Compiling the component…
		</p>
	{/if}
	<div bind:this={host} class="min-h-(--kit-space-sm)"></div>
</div>