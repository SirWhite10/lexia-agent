<script lang="ts">
	/**
	 * HTML artifacts run in an iframe with `sandbox="allow-scripts"` and
	 * deliberately *without* `allow-same-origin`: without it the frame gets an
	 * opaque origin, so it cannot read this page, its DOM, its storage or its
	 * cookies, and a document that tries fails inside the frame rather than
	 * against the host.
	 *
	 * Because the origin is opaque the parent cannot measure the document
	 * through the DOM, so the document measures itself and reports the height
	 * back through `postMessage`.
	 */

	interface Props {
		source: string;
		title?: string;
		class?: string;
	}

	let { source, title = 'HTML artifact', class: className }: Props = $props();

	const MIN_HEIGHT = 96;
	const MEASURE_GRACE_MS = 400;
	const MAX_HEIGHT = 4000;
	const MEASURE_TIMEOUT_MS = 8000;

	/** Unique per frame, so two artifacts on one page never measure each other. */
	const channel = `kit-html-${Math.random().toString(36).slice(2)}`;

	let frame = $state<HTMLIFrameElement | null>(null);
	let height = $state<number | null>(null);
	let failed = $state(false);

	/**
	 * The only code this host adds to artifact source. It measures the document
	 * and posts the height up; it reads nothing and sends nothing anywhere but
	 * this frame's parent.
	 */
	const srcdoc = $derived(
		`${source}
<script>(function(){var c=${JSON.stringify(channel)};
function report(){parent.postMessage({kit:c,height:document.documentElement.scrollHeight},'*')}
if(typeof ResizeObserver==='function'){new ResizeObserver(report).observe(document.documentElement)}
addEventListener('message',function(e){if(e.data&&e.data.kitReq===c)report()});
addEventListener('load',report);report();})();<\/script>`
	);

	function onMessage(event: MessageEvent) {
		if (event.source !== frame?.contentWindow) return;
		const data = event.data;
		if (!data || typeof data !== 'object' || data.kit !== channel) return;
		const reported = Number(data.height);
		if (!Number.isFinite(reported)) return;
		height = Math.min(MAX_HEIGHT, Math.max(MIN_HEIGHT, Math.ceil(reported)));
	}


	let timer: ReturnType<typeof setTimeout> | undefined;

	function onLoad() {
		// The document posts its height as it parses, which can land before this
		// page finished hydrating and attached the listener, and a document that
		// never resizes again posts nothing further. So the host asks for the height
		// once the frame has loaded and judges the frame a moment later, rather than
		// on whether the first unsolicited message happened to arrive in time.
		frame?.contentWindow?.postMessage({ kitReq: channel }, '*');
		setTimeout(() => (failed = height === null), MEASURE_GRACE_MS);
	}

	$effect(() => {
		const text = source;
		height = null;
		failed = false;
		timer = setTimeout(() => (failed = true), MEASURE_TIMEOUT_MS);
		return () => {
			clearTimeout(timer);
			void text;
		};
	});
</script>

<svelte:window onmessage={onMessage} />

<div data-kit="render-html" class={className}>
	{#if failed}
		<div class="mb-(--kit-space-sm) rounded-(--kit-radius-md) border border-(--kit-border) bg-(--kit-muted) px-(--kit-space-sm) py-(--kit-space-xs) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">
			<span class="font-medium text-(--kit-foreground)">This artifact did not report its height.</span>
			The frame is left in place below so the document stays inspectable.
		</div>
	{/if}
	<iframe
		bind:this={frame}
		{srcdoc}
		sandbox="allow-scripts"
		referrerpolicy="no-referrer"
		{title}
		onload={onLoad}
		class="w-full rounded-(--kit-radius-md) border border-(--kit-border) bg-(--kit-background)"
		style:height={`${height ?? MIN_HEIGHT}px`}
	></iframe>
	<p class="mt-(--kit-space-sm) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">
		Sandboxed preview: scripts run, everything else is blocked. The frame has an opaque origin, so it cannot
		read this page, its styles, its storage or its cookies.
	</p>
</div>