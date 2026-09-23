<svelte:head>
	<title>Lexia — Agent</title>
	<meta name="description" content="Your Lexia agent workspace" />
</svelte:head>

<main class="agent-shell">
	<header class="agent-header">
		<div>
			<p class="eyebrow">LEXIA</p>
			<h1>Your agent workspace</h1>
		</div>
		<span class="status"><span class="status-dot"></span>Ready</span>
	</header>

	<section class="agent-card" aria-label="Agent conversation">
		<div class="agent-mark" aria-hidden="true">✦</div>
		<div>
			<p class="eyebrow">EVE ONLINE</p>
			<h2>What can I help you build?</h2>
			<p class="muted">Your authenticated Lexia agent will appear here.</p>
		</div>
	</section>
</main>

<style>
	.agent-shell {
		min-height: 100vh;
		padding: 2rem clamp(1.25rem, 5vw, 5rem);
		background: var(--background);
	}

	.agent-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		max-width: 72rem;
		margin: 0 auto;
	}

	.eyebrow {
		margin: 0 0 0.45rem;
		font-size: 0.72rem;
		font-weight: 700;
		letter-spacing: 0.16em;
		color: var(--secondary);
	}

	h1,
	h2,
	p {
		margin-top: 0;
	}

	h1 {
		margin-bottom: 0;
		font-size: clamp(1.7rem, 3vw, 2.5rem);
	}

	.status {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.55rem 0.8rem;
		border: 1px solid var(--border);
		border-radius: 999px;
		font-size: 0.8rem;
		color: var(--muted-foreground);
	}

	.status-dot {
		width: 0.45rem;
		height: 0.45rem;
		border-radius: 50%;
		background: #57b894;
	}

	.agent-card {
		display: flex;
		align-items: center;
		gap: 1.25rem;
		max-width: 42rem;
		margin: 20vh auto 0;
		padding: 2rem;
		border: 1px solid var(--border);
		border-radius: 1rem;
		background: var(--card);
		box-shadow: 0 1.5rem 4rem oklch(0.3 0.02 90 / 8%);
	}

	.agent-mark {
		display: grid;
		width: 3rem;
		height: 3rem;
		flex: 0 0 auto;
		place-items: center;
		border-radius: 0.8rem;
		background: var(--primary);
		font-size: 1.5rem;
	}

	h2 {
		margin-bottom: 0.5rem;
		font-size: clamp(1.3rem, 3vw, 2rem);
	}

	.muted {
		margin-bottom: 0;
		color: var(--muted-foreground);
	}
</style>
