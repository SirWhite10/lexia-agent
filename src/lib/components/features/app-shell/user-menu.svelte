<script lang="ts">
	import LogOutIcon from '@lucide/svelte/icons/log-out';
	import MoonIcon from '@lucide/svelte/icons/moon';
	import SunIcon from '@lucide/svelte/icons/sun';
	import { mode, toggleMode } from 'mode-watcher';
	import * as Avatar from '#lib/components/ui/avatar/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';

	let { userName }: { userName: string } = $props();

	// Sign-out is a plain server action; the hidden form keeps the menu item a
	// menu item instead of nesting interactive elements.
	let logoutForm = $state<HTMLFormElement>();
	const initial = $derived(userName.charAt(0).toUpperCase());
</script>

<form method="POST" action="/logout" bind:this={logoutForm} class="hidden"></form>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			<Button variant="ghost" size="sm" class="max-w-40 gap-2 pl-1.5 pr-2" aria-label="Account menu" {...props}>
				<Avatar.Root class="size-6">
					<Avatar.Fallback class="bg-primary text-[0.7rem] font-bold text-primary-foreground">{initial}</Avatar.Fallback>
				</Avatar.Root>
				<span class="truncate text-sm font-semibold">{userName}</span>
			</Button>
		{/snippet}
	</DropdownMenu.Trigger>
	<DropdownMenu.Content align="end" sideOffset={6}>
		<DropdownMenu.Label class="font-normal">
			<span class="block text-xs text-muted-foreground">Signed in as</span>
			<span class="block truncate font-semibold">{userName}</span>
		</DropdownMenu.Label>
		<DropdownMenu.Separator />
		<DropdownMenu.Item onSelect={() => toggleMode()}>
			{#if mode.current === 'dark'}
				<SunIcon /> Light mode
			{:else}
				<MoonIcon /> Dark mode
			{/if}
		</DropdownMenu.Item>
		<DropdownMenu.Separator />
		<DropdownMenu.Item variant="destructive" onSelect={() => logoutForm?.requestSubmit()}>
			<LogOutIcon /> Sign out
		</DropdownMenu.Item>
	</DropdownMenu.Content>
</DropdownMenu.Root>
