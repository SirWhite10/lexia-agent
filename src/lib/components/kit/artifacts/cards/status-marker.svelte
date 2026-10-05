<script lang="ts">
	import { cn } from '#lib/utils.js';
	import type { TimelinePayload } from '../types.js';

	interface Props {
		status: TimelinePayload['steps'][number]['status'];
		class?: string;
	}

	let { status, class: className }: Props = $props();

	/**
	 * Each status has its own glyph and its own word, so the run reads without
	 * relying on colour: the queue is an empty circle, a failure is a cross, a
	 * cancellation is a barred circle, and the running state breathes.
	 */
	const STATUS: Record<Props['status'], { glyph: string; label: string; class: string }> = {
		queued: { glyph: '○', label: 'Queued', class: 'text-[color:var(--kit-muted-foreground)]' },
		running: { glyph: '◐', label: 'Running', class: 'kit-breathe text-[color:var(--kit-info)]' },
		succeeded: { glyph: '✓', label: 'Succeeded', class: 'text-[color:var(--kit-success)]' },
		failed: { glyph: '✕', label: 'Failed', class: 'text-[color:var(--kit-danger)]' },
		cancelled: { glyph: '⊘', label: 'Cancelled', class: 'text-[color:var(--kit-muted-foreground)]' }
	};
	const marker = $derived(STATUS[status]);
</script>

<span class={cn('inline-flex items-center gap-(--kit-space-xs) text-[length:var(--kit-text-xs)]', marker.class, className)}>
	<span aria-hidden="true" class="font-mono leading-none">{marker.glyph}</span>
	<span>{marker.label}</span>
</span>