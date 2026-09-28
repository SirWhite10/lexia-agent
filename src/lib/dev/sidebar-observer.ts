// MUTED diagnostic for the sidebar trigger. Re-arm by flipping
// `injectSidebarObserver` in src/hooks.server.ts (technique documented in
// AGENTS.md, "Dev-only page instrumentation").
//
// === Method notes for future testing concepts ===
//
// Injection: `transformPageChunk` in hooks.server.ts splices this string into
// <head> while HTML streams out, so the observer runs at parse time — before
// hydration, before the first paint settles. A console snippet pasted by hand
// arms seconds too late: the interesting window has already passed and the
// behavior under test appears "fixed". Same recipe works for any pre-hydration
// instrumentation (event tripwires, long-task timers, network probes).
//
// Constraints that made this work:
// - Plain ES5 body, no template literals: the string is inlined verbatim into an
//   inline <script>, nothing to compile.
// - Gated on `dev` from `$app/env` (Kit 3 renamed `$app/environment`):
//   dead-branch elimination strips the payload from production output entirely
//   (verified against `bun run build` artifacts).
// - Every line logs wall clock `HH:MM:SS.mmm` (correlate with devtools
//   Network/Performance) plus parse-relative `+Nms` (measures the window).
// - Capture-phase trigger listeners fire even when Svelte handlers are not
//   attached yet, separating "click swallowed" from "click never landed".
// - Node-replacement observer catches hydration/HMR rebuilding the subtree,
//   which orphans listeners bound to the discarded node.
// - Resource probe lists svelte/bits-ui module copies: dev can serve optimized
//   deps and source side by side, and duplicated runtimes split runes
//   reactivity (state flips in one copy, view reads the other).
//
// === Findings (2026-09-23) ===
//
// "Sidebar trigger needs several presses in dev, works first try in preview":
// pre-hydration dead clicks. SSR renders the trigger looking interactive, but
// handlers attach only when Svelte hydrates. Dev ships the module graph
// un-bundled (2.7k+ files) — worse over remote access (WAN round trip per
// module) — so hydration completes seconds in; preview ships pre-bundled
// chunks and hydrates in milliseconds. No app bug. Reading guide: a logged
// `trigger click` with no following `data-state ->` = swallowed click; no
// `trigger pointerdown` at all = press landed outside the trigger subtree
// (element not parsed yet); `data-state ->` with no visual change = reactivity
// split (duplicate runtime).

export const sidebarObserverScript = `
(() => {
	var t0 = performance.now();
	var ms = function () { return (performance.now() - t0).toFixed(0) + 'ms'; };
	var clock = function () {
		var d = new Date();
		var pad = function (n, w) { return ('000' + n).slice(-w); };
		return pad(d.getHours(), 2) + ':' + pad(d.getMinutes(), 2) + ':' + pad(d.getSeconds(), 2) + '.' + pad(d.getMilliseconds(), 3);
	};
	var stateOf = function () {
		var el = document.querySelector('[data-slot="sidebar"]');
		return el ? el.getAttribute('data-state') : 'NO SIDEBAR NODE';
	};
	var log = function (msg) { console.log('[sidebar-dev ' + clock() + '] ' + msg + ' | +' + ms()); };

	log('parsed, initial data-state = ' + stateOf() + ', readyState = ' + document.readyState);

	var firstArm = true;
	var arm = function (el) {
		if (el.__sidebarDevArmed) return;
		el.__sidebarDevArmed = true;
		new MutationObserver(function (muts) {
			muts.forEach(function (m) {
				log(m.attributeName + ' -> ' + m.target.getAttribute(m.attributeName));
			});
		}).observe(el, {
			attributes: true,
			attributeFilter: ['data-state', 'data-collapsible', 'data-variant']
		});
		log(firstArm ? 'observer armed' : 'SIDEBAR NODE REPLACED, observer re-armed');
		firstArm = false;
	};

	var tryArm = function () {
		var el = document.querySelector('[data-slot="sidebar"]');
		if (el) arm(el);
	};
	tryArm();

	// Catches hydration/HMR tearing the sidebar subtree down and rebuilding it,
	// which silently orphans clicks bound to the discarded node.
	new MutationObserver(tryArm).observe(document.documentElement, {
		childList: true,
		subtree: true
	});

	// Capture phase: fires even when no Svelte handler is attached yet, so a
	// logged click with no following state change = swallowed click.
	['pointerdown', 'click'].forEach(function (type) {
		document.addEventListener(type, function (e) {
			var t = e.target && e.target.closest ? e.target.closest('[data-slot="sidebar-trigger"]') : null;
			if (t) log('trigger ' + type + ', state now = ' + stateOf());
		}, true);
	});

	// Dev serves optimized deps and source side by side; duplicated runtime
	// copies break runes reactivity across the boundary.
	window.addEventListener('load', function () {
		setTimeout(function () {
			var copies = performance.getEntriesByType('resource')
				.map(function (r) { return r.name; })
				.filter(function (n) { return /bits-ui|\\.vite\\/deps\\/svelte/.test(n); });
			log('runtime module copies in play: ' + JSON.stringify(copies));
		}, 3000);
	});
})();
`;
