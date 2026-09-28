<script lang="ts">
	import AppBottomNav from '#lib/components/features/app-shell/app-bottom-nav.svelte';
	import AppHeader from '#lib/components/features/app-shell/app-header.svelte';
	import AppSidebar from '#lib/components/features/app-shell/app-sidebar.svelte';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';

	let { data, children } = $props();
</script>

<!-- Authenticated shell: desktop rail + sticky header + mobile tab bar. The
     header carries the current page title (layout load resolves it); the
     bottom padding clears the tab bar so content is never trapped under it. -->
<Sidebar.Provider>
	<AppSidebar subAgents={data.subAgents} />
	<Sidebar.Inset class="min-w-0">
		<AppHeader title={data.title} userName={data.user.name ?? data.user.id} />
		<main class="flex flex-1 flex-col pb-[calc(3.5rem+env(safe-area-inset-bottom))] md:pb-0">
			{@render children()}
		</main>
		<AppBottomNav />
	</Sidebar.Inset>
</Sidebar.Provider>
