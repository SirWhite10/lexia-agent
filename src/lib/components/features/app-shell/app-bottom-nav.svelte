<script lang="ts">
	import { page } from '$app/state';
	import { isNavActive, primaryNav } from './nav-items.js';
</script>

<!-- Thumb-first primary navigation for phones; the desktop rail supersedes it. -->
<nav
	class="fixed inset-x-0 bottom-0 z-10 border-t border-border bg-background/95 pb-[env(safe-area-inset-bottom)] backdrop-blur md:hidden"
	aria-label="Primary"
>
	<ul class="grid grid-cols-3">
		{#each primaryNav as item (item.href)}
			{@const active = isNavActive(item, page.url.pathname)}
			<li>
				<a
					href={item.href}
					aria-current={active ? 'page' : undefined}
					class="flex min-h-12 flex-col items-center justify-center gap-0.5 px-2 py-1.5 text-[0.7rem] font-semibold transition-colors {active
						? 'text-primary'
						: 'text-muted-foreground hover:text-foreground'}"
				>
					<item.icon class="size-5" />
					<span>{item.title}</span>
				</a>
			</li>
		{/each}
	</ul>
</nav>
