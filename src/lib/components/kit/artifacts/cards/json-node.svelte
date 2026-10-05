<script lang="ts">
	import { cn } from '#lib/utils.js';
	import JsonNode from './json-node.svelte';

	/**
	 * One node of the JSON tree. Objects and arrays are collapsible, and each
	 * expansion reveals exactly one more level: a nested object starts closed,
	 * so a deep payload never dumps a wall of text on the reader and nothing is
	 * hidden without a control that says it is hidden.
	 */

	interface Props {
		value: unknown;
		/** Key rendered before the value when the node sits inside an object. */
		name?: string;
		/** Array entries carry a trailing comma; object entries do not. */
		inArray?: boolean;
		/** The artifact opened the tree closed, or this node is nested one level down. */
		initiallyCollapsed?: boolean;
		class?: string;
	}

	let { value, name, inArray = false, initiallyCollapsed = false, class: className }: Props = $props();

	const isObject = $derived(value !== null && typeof value === 'object');
	const entries = $derived(
		isObject
			? Object.entries(value as Record<string, unknown>).map(([key, entry]) => ({ key, value: entry }))
			: []
	);

	// `open` follows the props until the reader touches the toggle, then it is
	// theirs: a re-render with a new payload does not slam a tree they opened.
	let expanded = $state<boolean | null>(null);
	const open = $derived(expanded ?? (isObject && !initiallyCollapsed));

	const PRIMITIVE_CLASS: Record<string, string> = {
		string: 'text-[color:var(--kit-code-string)]',
		number: 'text-[color:var(--kit-code-number)]',
		boolean: 'text-[color:var(--kit-code-keyword)]',
		undefined: 'text-[color:var(--kit-code-comment)]'
	};

	const primitiveClass = $derived(
		value === null
			? 'text-[color:var(--kit-code-comment)]'
			: (PRIMITIVE_CLASS[typeof value] ?? 'text-[color:var(--kit-code-foreground)]')
	);

	const primitiveText = $derived(
		typeof value === 'string' ? JSON.stringify(value) : value === undefined ? 'undefined' : String(value)
	);

	const summary = $derived(
		Array.isArray(value)
			? `${entries.length} ${entries.length === 1 ? 'item' : 'items'}`
			: `${entries.length} ${entries.length === 1 ? 'key' : 'keys'}`
	);
</script>

{#if isObject}
	<div class={cn('min-w-0', className)}>
		<button
			type="button"
			aria-expanded={open}
			onclick={() => (expanded = !open)}
			class="flex w-full items-center gap-(--kit-space-xs) rounded-(--kit-radius-sm) py-(--kit-space-xxs) text-left transition-colors duration-(--kit-duration-fast) hover:bg-(--kit-secondary)"
		>
			<span aria-hidden="true" class="w-3 shrink-0 text-[color:var(--kit-muted-foreground)]">{open ? '▾' : '▸'}</span>
			{#if name !== undefined}
				<span class="text-[color:var(--kit-code-tag)]">{name}</span>
				<span class="text-[color:var(--kit-muted-foreground)]">:</span>
			{/if}
			<span class="text-[color:var(--kit-code-foreground)]">{Array.isArray(value) ? '[' : '{'}</span>
			{#if !open}
				<span class="truncate text-[color:var(--kit-muted-foreground)]">{summary}</span>
				<span class="text-[color:var(--kit-code-foreground)]">{Array.isArray(value) ? ']' : '}'}</span>
				{#if inArray}<span class="text-[color:var(--kit-code-foreground)]">,</span>{/if}
			{/if}
			<span class="sr-only">{open ? 'Collapse' : 'Expand'}</span>
		</button>

		{#if open}
			<div class="ml-(--kit-space-xs) border-l border-(--kit-border) pl-(--kit-space-sm)">
				{#each entries as entry (entry.key)}
					<JsonNode
						value={entry.value}
						name={entry.key}
						inArray={Array.isArray(value)}
						initiallyCollapsed
					/>
				{/each}
			</div>
			<p
				class="m-0 ml-(--kit-space-sm) text-[length:var(--kit-text-mono)] leading-(--kit-leading-mono) text-[color:var(--kit-code-foreground)]"
			>
				{Array.isArray(value) ? ']' : '}'}{#if inArray},{/if}
			</p>
		{/if}
	</div>
{:else}
	<p class={cn('m-0 truncate pl-(--kit-space-sm) text-[length:var(--kit-text-mono)] leading-(--kit-leading-mono)', className)}>
		{#if name !== undefined}
			<span class="text-[color:var(--kit-code-tag)]">{name}</span><span class="text-[color:var(--kit-muted-foreground)]">:</span>
		{/if}
		<span class={primitiveClass}>{primitiveText}</span>{#if inArray}<span
				class="text-[color:var(--kit-code-foreground)]"
				>,</span
			>{/if}
	</p>
{/if}