<script lang="ts">
	import { cn } from '#lib/utils.js';
	import JsonNode from './json-node.svelte';
	import type { JsonPayload } from '../types.js';

	interface Props {
		data: JsonPayload;
		class?: string;
	}

	let { data, class: className }: Props = $props();
</script>

<div
	data-kit="json-card"
	class={cn(
		'kit-scroll overflow-x-auto rounded-(--kit-radius-md) border border-(--kit-border) bg-(--kit-code-background) p-(--kit-space-sm)',
		className
	)}
>
	<JsonNode value={data.value} initiallyCollapsed={data.defaultCollapsed ?? false} />
</div>