<script lang="ts">
	import type { PageData } from './$types';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Card from '#lib/components/ui/card/index.js';

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>Accounts — Lexia</title>
	<meta name="description" content="This device is signed in to the local Lexia workspace." />
</svelte:head>

<section class="mx-auto w-full max-w-2xl flex-1 space-y-4 px-5 py-6" aria-label="Accounts">
	<p class="mb-5 text-sm text-muted-foreground">
		This device is signed in to the local Lexia workspace.
	</p>

	<Card.Root>
		<Card.Header>
			<Card.Title>Account</Card.Title>
		</Card.Header>
		<Card.Content class="flex items-center justify-between gap-4">
			<div class="min-w-0">
				<p class="truncate font-semibold">{data.user.name ?? data.user.id}</p>
				<p class="font-mono text-xs text-muted-foreground">local session</p>
			</div>
			<form method="POST" action="/logout">
				<Button type="submit" variant="destructive">Sign out</Button>
			</form>
		</Card.Content>
	</Card.Root>
</section>
