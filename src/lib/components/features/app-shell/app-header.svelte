<script lang="ts">
	import * as Separator from '#lib/components/ui/separator/index.js';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import UserMenu from './user-menu.svelte';

	let { title, userName }: { title: string; userName: string } = $props();
</script>

<header
	class="sticky top-0 z-10 flex min-h-14 items-center gap-2 border-b border-border bg-background/95 px-3 pt-[env(safe-area-inset-top)] backdrop-blur md:px-4"
>
	<Sidebar.Trigger class="hidden md:inline-flex" />
	<Separator.Separator orientation="vertical" class="hidden h-5 md:block" />
	<span
		class="grid size-7 shrink-0 place-items-center rounded-[min(var(--radius-md),10px)] bg-primary text-sm text-primary-foreground"
		aria-hidden="true">✦</span
	>
	<h1 class="min-w-0 flex-1 truncate font-bold tracking-tight">{title}</h1>
	<div class="shrink-0">
		<UserMenu {userName} />
	</div>
</header>
