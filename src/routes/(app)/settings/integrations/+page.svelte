<script lang="ts">
	import type { PageData } from './$types';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import * as Card from '#lib/components/ui/card/index.js';

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>Integrations — Lexia</title>
	<meta name="description" content="Model providers and connected services." />
</svelte:head>

<section class="mx-auto w-full max-w-2xl flex-1 space-y-4 px-5 py-6" aria-label="Integrations">
	<p class="mb-5 text-sm text-muted-foreground">
		Model providers and connected services. Credentials stay server-side and never reach the client.
	</p>

	<Card.Root>
		<Card.Header>
			<div class="flex items-center justify-between gap-4">
				<Card.Title>OpenRouter</Card.Title>
				<Badge variant={data.openrouterConfigured ? 'secondary' : 'outline'}>
					{data.openrouterConfigured ? 'Configured' : 'Not configured'}
				</Badge>
			</div>
			<Card.Description>Initial model provider for EVE model routes.</Card.Description>
		</Card.Header>
		<Card.Content>
			<p class="text-sm text-muted-foreground">
				API keys are set via environment variables on the host.
			</p>
		</Card.Content>
	</Card.Root>
</section>
