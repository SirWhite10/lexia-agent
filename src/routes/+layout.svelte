<script lang="ts">
	import { ModeWatcher } from 'mode-watcher';
	import './layout.css';
	import favicon from '#lib/assets/favicon.svg';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<!-- DESIGN.md: dark is the default scheme; light is an explicit alternative. -->
<ModeWatcher defaultMode="dark" />

{@render children()}
