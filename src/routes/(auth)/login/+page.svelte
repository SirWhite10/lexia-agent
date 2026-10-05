<script lang="ts">
	import { invalidateAll } from '$app/navigation';

	let { data, form } = $props();

	type Step = 'welcome' | 'login' | 'reset' | 'setup-account' | 'setup-password';
	let step = $state<Step>('welcome');
	let username = $state('');
	let password = $state('');
	let confirmPassword = $state('');
	let initialUsername = $derived(form?.username ?? '');

	/** The server owns the truth about the workspace; client state only walks
	 * the onboarding wizard. A setup refusal (`needsSignIn`) and a completed
	 * password reset both land on sign-in, a minted or rejected recovery code
	 * lands on the code form, and an initialized workspace never opens the
	 * wizard — so a stale tab cannot reopen it over an existing account. */
	function resolveStep(current: Step): Step {
		if (form?.needsSignIn || data.passwordReset) return 'login';
		if (data.recoveryRequested || form?.mode === 'recover' || form?.mode === 'reset') return 'reset';
		if (form?.mode === 'setup') return 'setup-password';
		if (current !== 'welcome') return current;
		return data.initialized ? 'login' : 'welcome';
	}

	let visibleStep = $derived(resolveStep(step));
	// Recovery is off the onboarding path, so it carries no progress bar.
	let progressStep = $derived(visibleStep === 'welcome' ? 0 : visibleStep === 'setup-password' ? 2 : 1);
	let pageTitle = $derived(
		visibleStep === 'welcome'
			? 'Welcome to Lexosa'
			: visibleStep === 'login'
				? 'Sign in — Lexosa'
				: visibleStep === 'reset'
					? 'Recover your account — Lexosa'
					: 'Get started — Lexosa',
	);

	const setUsername = (event: Event) => {
		username = (event.currentTarget as HTMLInputElement).value;
	};

	const goBack = () => {
		step = visibleStep === 'setup-password' ? 'setup-account' : visibleStep === 'reset' ? 'login' : 'welcome';
	};

	const start = () => {
		step = data.initialized ? 'login' : 'setup-account';
	};

	const continueSetup = () => {
		if (username.trim().length >= 3) step = 'setup-password';
	};
</script>

<svelte:head>
	<title>{pageTitle}</title>
	<meta name="description" content="Set up your Lexosa agent workspace" />
</svelte:head>

<svelte:window
	onpageshow={(event) => {
		// A tab restored from the back/forward cache keeps its hydrated state, so
		// `initialized` can be stale and the wizard would reopen on an account
		// that already exists. Re-run the load before anything renders.
		if (event.persisted) invalidateAll();
	}}
/>

<main class="onboarding-shell">
	<section class="onboarding-card" aria-labelledby="onboarding-title">
		{#if visibleStep !== 'reset'}
			<div class="progress" aria-label="Onboarding progress">
				{#each [0, 1, 2] as index}
					<span class:active={index <= progressStep}></span>
				{/each}
			</div>
		{/if}

		{#if visibleStep === 'welcome'}
			<div class="hero-icon" aria-hidden="true">✦</div>
			<p class="eyebrow">LEXOSA</p>
			<h1 id="onboarding-title">Welcome to your new agent.</h1>
			<p class="intro">A thoughtful workspace for turning ideas, questions, and plans into momentum.</p>
			<button class="primary-button" type="button" onclick={start}>Get started <span aria-hidden="true">→</span></button>
		{:else if visibleStep === 'login'}
			{#if !data.initialized}
				<button class="back-button" type="button" onclick={goBack}>← Back</button>
			{/if}
			<p class="eyebrow">WELCOME BACK</p>
			<h1 id="onboarding-title">Sign in to Lexosa.</h1>
			<p class="intro">Pick up where you left off.</p>
			{#if data.passwordReset}<p class="notice" role="status">Password updated. Sign in with your new password.</p>{/if}
			{#if form?.error}<p class="error" role="alert">{form.error}</p>{/if}
			<form method="POST" action="?/login">
				<label>Username <input name="username" autocomplete="username" required value={username || initialUsername} oninput={setUsername} /></label>
				<label>Password <input name="password" type="password" autocomplete="current-password" required /></label>
				<button class="primary-button" type="submit">Enter workspace <span aria-hidden="true">→</span></button>
			</form>

			<form method="POST" action="?/recover">
				<button class="link-button" type="submit">Forgot password?</button>
			</form>
		{:else if visibleStep === 'setup-account'}
			<button class="back-button" type="button" onclick={goBack}>← Back</button>
			<p class="eyebrow">STEP 1 OF 2</p>
			<h1 id="onboarding-title">Name your workspace.</h1>
			<p class="intro">Choose the username you’ll use to return to Lexosa.</p>
			<div class="stack">
				<label>Username <input autocomplete="username" minlength="3" placeholder="your name" value={username || initialUsername} oninput={setUsername} /></label>
				<button class="primary-button" type="button" onclick={continueSetup}>Continue <span aria-hidden="true">→</span></button>
			</div>
		{:else if visibleStep === 'reset'}
			<button class="back-button" type="button" onclick={goBack}>← Back</button>
			<p class="eyebrow">ACCOUNT RECOVERY</p>
			<h1 id="onboarding-title">Set a new password.</h1>
			<p class="intro">
				A one-time recovery code was printed to the host terminal and written to
				<code>app-data/state/auth-recovery.txt</code>. Paste it with your new password — it expires in 30
				minutes and works once.
			</p>
			{#if form?.error}<p class="error" role="alert">{form.error}</p>{/if}
			<form method="POST" action="?/reset">
				<label>Recovery code <input name="code" autocomplete="one-time-code" spellcheck="false" required /></label>
				<label>New password <input name="password" type="password" autocomplete="new-password" minlength="8" required /></label>
				<label>Confirm password <input name="confirmPassword" type="password" autocomplete="new-password" minlength="8" required /></label>
				<button class="primary-button" type="submit">Set new password <span aria-hidden="true">→</span></button>
			</form>
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
	.intro code { padding: 0.1rem 0.3rem; border-radius: 0.3rem; background: var(--accent); color: var(--foreground); font-size: 0.9em; }
	.primary-button { display: flex; width: 100%; justify-content: space-between; padding: 0.9rem 1rem; border: 0; border-radius: 0.6rem; background: var(--primary); color: var(--primary-foreground); font: inherit; font-weight: 700; cursor: pointer; }
	.primary-button:hover { filter: brightness(0.96); }
	.back-button { margin: -1rem 0 2rem; padding: 0; border: 0; background: transparent; color: var(--muted-foreground); font: inherit; cursor: pointer; }
	form { display: grid; gap: 1rem; }
	.link-button { padding: 0; border: 0; background: transparent; color: var(--secondary); font: inherit; text-decoration: underline; text-underline-offset: 0.2rem; cursor: pointer; }
	.link-button:hover { color: var(--foreground); }
	label { display: grid; gap: 0.45rem; color: var(--muted-foreground); font-size: 0.82rem; font-weight: 700; }
	input { width: 100%; box-sizing: border-box; padding: 0.8rem 0.85rem; border: 1px solid var(--input); border-radius: 0.5rem; background: var(--background); color: var(--foreground); font: inherit; }
	input:focus { border-color: var(--primary); outline: 2px solid color-mix(in oklch, var(--primary) 35%, transparent); }
	.stack { display: grid; gap: 1rem; }
	.stack .primary-button { margin-top: 0.5rem; }
	form .primary-button { margin-top: 0.5rem; }
	.error { margin: -0.75rem 0 1rem; color: var(--destructive); font-size: 0.85rem; }
	.notice { margin: -0.75rem 0 1rem; color: var(--secondary); font-size: 0.85rem; }
	.footer-note { margin: 3rem 0 0; color: var(--muted-foreground); font-size: 0.72rem; text-align: center; }
</style>
