<script lang="ts">
	let { data, form } = $props();

	type Step = 'welcome' | 'choice' | 'login' | 'setup-account' | 'setup-password';
	let step = $state<Step>('welcome');
	let username = $state('');
	let password = $state('');
	let confirmPassword = $state('');
	let initialUsername = $derived(form?.username ?? '');
	let visibleStep = $derived(
		step === 'welcome' && form?.mode === 'login'
			? 'login'
			: step === 'welcome' && form?.mode === 'setup'
				? 'setup-password'
				: step,
	);

	const setUsername = (event: Event) => {
		username = (event.currentTarget as HTMLInputElement).value;
	};

	const goBack = () => {
		step = visibleStep === 'setup-password' ? 'setup-account' : visibleStep === 'setup-account' || visibleStep === 'login' ? 'choice' : 'welcome';
	};

	const start = () => {
		step = data.initialized ? 'choice' : 'setup-account';
	};

	const continueSetup = () => {
		if (username.trim().length >= 3) step = 'setup-password';
	};
</script>

<svelte:head>
	<title>{visibleStep === 'welcome' ? 'Welcome to Lexia' : 'Get started — Lexia'}</title>
	<meta name="description" content="Set up your Lexia agent workspace" />
</svelte:head>

<main class="onboarding-shell">
	<section class="onboarding-card" aria-labelledby="onboarding-title">
		<div class="progress" aria-label="Onboarding progress">
			{#each ['welcome', 'choice', 'finish'] as item, index}
				<span class:active={index <= (visibleStep === 'welcome' ? 0 : visibleStep === 'choice' ? 1 : 2)}></span>
			{/each}
		</div>

		{#if visibleStep === 'welcome'}
			<div class="hero-icon" aria-hidden="true">✦</div>
			<p class="eyebrow">LEXIA</p>
			<h1 id="onboarding-title">Welcome to your new agent.</h1>
			<p class="intro">A thoughtful workspace for turning ideas, questions, and plans into momentum.</p>
			<button class="primary-button" type="button" onclick={start}>Get started <span aria-hidden="true">→</span></button>
		{:else if visibleStep === 'choice'}
			<button class="back-button" type="button" onclick={goBack}>← Back</button>
			<p class="eyebrow">READY WHEN YOU ARE</p>
			<h1 id="onboarding-title">How would you like to begin?</h1>
			<p class="intro">Your Lexia workspace is initialized. Choose how you want to enter it.</p>
			<div class="choice-list">
				<button class="choice" type="button" onclick={() => (step = 'login')}>
					<span class="choice-icon">↗</span><span><strong>Sign in</strong><small>I already have an account</small></span>
				</button>
			</div>
		{:else if visibleStep === 'login'}
			<button class="back-button" type="button" onclick={goBack}>← Back</button>
			<p class="eyebrow">WELCOME BACK</p>
			<h1 id="onboarding-title">Sign in to Lexia.</h1>
			<p class="intro">Pick up where you left off.</p>
			{#if form?.mode === 'login'}<p class="error" role="alert">{form.error}</p>{/if}
			<form method="POST" action="?/login">
				<label>Username <input name="username" autocomplete="username" required value={username || initialUsername} oninput={setUsername} /></label>
				<label>Password <input name="password" type="password" autocomplete="current-password" required /></label>
				<button class="primary-button" type="submit">Enter workspace <span aria-hidden="true">→</span></button>
			</form>
		{:else if visibleStep === 'setup-account'}
			<button class="back-button" type="button" onclick={goBack}>← Back</button>
			<p class="eyebrow">STEP 1 OF 2</p>
			<h1 id="onboarding-title">Name your workspace.</h1>
			<p class="intro">Choose the username you’ll use to return to Lexia.</p>
			<label>Username <input autocomplete="username" minlength="3" placeholder="your name" value={username || initialUsername} oninput={setUsername} /></label>
			<button class="primary-button" type="button" onclick={continueSetup}>Continue <span aria-hidden="true">→</span></button>
		{:else}
			<button class="back-button" type="button" onclick={goBack}>← Back</button>
			<p class="eyebrow">STEP 2 OF 2</p>
			<h1 id="onboarding-title">Secure your agent.</h1>
			<p class="intro">Create a password, then confirm it so your workspace stays yours.</p>
			{#if form?.mode === 'setup'}<p class="error" role="alert">{form.error}</p>{/if}
			<form method="POST" action="?/setup">
				<input type="hidden" name="username" value={username || initialUsername} />
				<label>Password <input bind:value={password} name="password" type="password" autocomplete="new-password" minlength="8" required /></label>
				<label>Confirm password <input bind:value={confirmPassword} name="confirmPassword" type="password" autocomplete="new-password" minlength="8" required /></label>
				<button class="primary-button" type="submit">Create my agent <span aria-hidden="true">→</span></button>
			</form>
		{/if}

		<p class="footer-note">Private by design · Built for your way of thinking</p>
	</section>
</main>

<style>
	.onboarding-shell { display: grid; min-height: 100vh; place-items: center; padding: 1.5rem; background: radial-gradient(circle at 20% 15%, oklch(0.72 0.12 10 / 24%), transparent 38%), var(--background); }
	.onboarding-card { width: min(100%, 31rem); min-height: 36rem; padding: clamp(2rem, 7vw, 4rem); border: 1px solid var(--border); border-radius: 1.25rem; background: color-mix(in oklch, var(--card) 94%, transparent); box-shadow: 0 2rem 5rem oklch(0.04 0 0 / 22%); }
	.progress { display: flex; gap: 0.4rem; margin-bottom: 3.5rem; }
	.progress span { height: 0.2rem; flex: 1; border-radius: 1rem; background: var(--border); transition: background 180ms ease; }
	.progress span.active { background: var(--primary); }
	.hero-icon { display: grid; width: 3.5rem; height: 3.5rem; margin-bottom: 2rem; place-items: center; border-radius: 1rem; background: var(--primary); color: var(--primary-foreground); font-size: 1.7rem; }
	.eyebrow { margin: 0 0 1rem; color: var(--secondary); font-size: 0.72rem; font-weight: 700; letter-spacing: 0.16em; }
	h1 { margin: 0 0 1rem; font-size: clamp(2rem, 6vw, 3.1rem); line-height: 1.05; }
	.intro { margin: 0 0 2rem; color: var(--muted-foreground); line-height: 1.6; }
	.primary-button, .choice { border: 0; border-radius: 0.6rem; font: inherit; font-weight: 700; cursor: pointer; }
	.primary-button { display: flex; width: 100%; justify-content: space-between; padding: 0.9rem 1rem; background: var(--primary); color: var(--primary-foreground); }
	.primary-button:hover { filter: brightness(0.96); }
	.back-button { margin: -1rem 0 2rem; padding: 0; border: 0; background: transparent; color: var(--muted-foreground); font: inherit; cursor: pointer; }
	.choice-list { display: grid; gap: 0.75rem; }
	.choice { display: flex; align-items: center; gap: 1rem; width: 100%; padding: 1rem; border: 1px solid var(--border); background: var(--background); color: var(--foreground); text-align: left; }
	.choice:hover { border-color: var(--primary); }
	.choice-icon { display: grid; width: 2.2rem; height: 2.2rem; place-items: center; border-radius: 0.5rem; background: var(--accent); }
	.choice span:last-child { display: grid; gap: 0.2rem; }
	.choice small { color: var(--muted-foreground); font-weight: 400; }
	form { display: grid; gap: 1rem; }
	label { display: grid; gap: 0.45rem; color: var(--muted-foreground); font-size: 0.82rem; font-weight: 700; }
	input { width: 100%; box-sizing: border-box; padding: 0.8rem 0.85rem; border: 1px solid var(--input); border-radius: 0.5rem; background: var(--background); color: var(--foreground); font: inherit; }
	input:focus { border-color: var(--primary); outline: 2px solid color-mix(in oklch, var(--primary) 35%, transparent); }
	form .primary-button { margin-top: 0.5rem; }
	.error { margin: -0.75rem 0 1rem; color: var(--destructive); font-size: 0.85rem; }
	.footer-note { margin: 3rem 0 0; color: var(--muted-foreground); font-size: 0.72rem; text-align: center; }
</style>
