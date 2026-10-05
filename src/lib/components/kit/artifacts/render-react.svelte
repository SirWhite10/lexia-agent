<script lang="ts">
	import { mountReactArtifact, type ReactArtifactError, type ReactMount } from './react-runtime.js';

	/**
	 * React artifacts run for real. `react`, `react-dom` and `htm` are loaded by
	 * `react-runtime.ts` on first use, so they arrive as a separate chunk that a
	 * session that never shows a React artifact never downloads. The authoring
	 * contract is documented there.
	 */

	interface Props {
		source: string;
		class?: string;
	}

	let { source, class: className }: Props = $props();

	let host = $state<HTMLDivElement | null>(null);
	let error = $state<ReactArtifactError | null>(null);
	let loading = $state(true);

	const PHASE_COPY: Record<ReactArtifactError['phase'], string> = {
		load: 'React could not be loaded, so the artifact did not run.',
		evaluate: 'The artifact did not evaluate to a component.',
		render: 'The artifact threw while rendering.'
	};

	$effect(() => {
		const target = host;
		const text = source;
		if (!target) return;

		let disposed = false;
		let app: ReactMount | undefined;
		error = null;
		loading = true;

		(async () => {
			try {
				const mounted = await mountReactArtifact(text, target, (message) => {
					if (!disposed) error = { phase: 'render', message };
				});
				if (disposed) {
					mounted.unmount();
					return;
				}
				app = mounted;
				loading = false;
			} catch (cause) {
				if (disposed) return;
				error = cause as ReactArtifactError;
				loading = false;
			}
		})();

		return () => {
			disposed = true;
			app?.unmount();
		};
	});
</script>

<div data-kit="render-react" class={className}>
	{#if error}
		<div
			role="alert"
			class="rounded-(--kit-radius-md) border border-(--kit-danger) bg-(--kit-code-background) p-(--kit-space-sm) text-[length:var(--kit-text-xs)]"
		>
			<p class="font-medium text-[color:var(--kit-danger)]">{PHASE_COPY[error.phase]}</p>
			<p class="mt-(--kit-space-xs) font-mono text-[color:var(--kit-code-foreground)] break-words">{error.message}</p>
		</div>
	{:else if loading}
		<p class="px-(--kit-space-xs) py-(--kit-space-lg) text-[length:var(--kit-text-sm)] text-[color:var(--kit-muted-foreground)]">
			Loading React…
		</p>
	{/if}
	<div bind:this={host} class="min-h-(--kit-space-sm)"></div>
</div>