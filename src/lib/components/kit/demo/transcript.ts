/**
 * Fixtures for the kit test page.
 *
 * These are the seeded conversation, the artifacts it carries, and the script
 * the harness replays to exercise run progress. They are deliberately real
 * content rather than lorem ipsum: a UI test page is only useful if the strings
 * it renders have realistic length, punctuation, and shape. Every artifact kind
 * in `artifacts/types.ts` appears here at least once, so the page doubles as the
 * artifact gallery.
 */

import type { Artifact } from '../artifacts/types.js';
import type { ChatMessage, RunAction } from '../chat/model.js';

const minute = 60_000;

/**
 * Deterministic product thumbnails as inline SVG data URIs. External image
 * hosts make the page fail offline and shift layout while loading, so the demo
 * carries its own artwork; `hue` is derived from the name so each product is
 * visually distinct without shipping binary assets.
 */
function productImage(name: string, hue: number): string {
	const initials = name
		.split(' ')
		.slice(0, 2)
		.map((word) => word[0])
		.join('');
	const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 120"><rect width="160" height="120" fill="hsl(${hue} 32% 22%)"/><rect x="10" y="10" width="140" height="100" rx="8" fill="hsl(${hue} 34% 30%)"/><text x="80" y="70" font-family="system-ui,sans-serif" font-size="34" font-weight="700" fill="hsl(${hue} 60% 88%)" text-anchor="middle">${initials}</text></svg>`;
	return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

/** A locally drawn chart thumbnail, for the same offline reason. */
const usagePreview =
	'data:image/svg+xml;utf8,' +
	encodeURIComponent(
		`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 160"><rect width="320" height="160" fill="#0a0a0a"/><g fill="#2563eb"><rect x="16" y="96" width="34" height="48" rx="4"/><rect x="62" y="72" width="34" height="72" rx="4"/><rect x="108" y="88" width="34" height="56" rx="4"/><rect x="154" y="48" width="34" height="96" rx="4"/><rect x="200" y="60" width="34" height="84" rx="4"/><rect x="246" y="28" width="34" height="116" rx="4"/></g><g fill="#525252"><rect x="16" y="148" width="264" height="2" rx="1"/></g></svg>`
	);

/** Self-contained by construction: the HTML artifact runs in a sandboxed frame with no network. */
const tokenWidgetHtml = `<!doctype html>
<html>
	<head>
		<meta charset="utf-8" />
		<style>
			body { margin: 0; padding: 16px; font: 14px/20px system-ui, sans-serif; background: #0a0a0a; color: #fafafa; }
			h1 { margin: 0 0 2px; font-size: 18px; line-height: 28px; }
			p { margin: 0 0 12px; color: #a3a3a3; font-size: 12px; line-height: 16px; }
			ul { margin: 0; padding: 0; list-style: none; display: grid; gap: 8px; }
			li { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 8px 12px; border: 1px solid #262626; border-radius: 6px; background: #171717; }
			span { color: #a3a3a3; }
			strong { font-variant-numeric: tabular-nums; }
			em { color: #4ade80; font-style: normal; }
		</style>
	</head>
	<body>
		<h1>Token use, last 6 turns</h1>
		<p>Generated artifact. Sandboxed frame, no network, no host styles.</p>
		<ul>
			<li><span>Prompt</span><strong>1,284</strong></li>
			<li><span>Completion</span><strong>3,910</strong></li>
			<li><span>Cached</span><em>72%</em></li>
		</ul>
	</body>
</html>`;

/**
 * Svelte artifacts are compiled in the browser by `artifacts/render-svelte.svelte`
 * at render time, so the source here is the source an agent would emit.
 */
const counterSvelteSource = `<script>
	let count = $state(0);
	let step = $state(1);
</script>

<div class="wrap">
	<p class="label">Sub-agent turns</p>
	<strong class="value">{count}</strong>
	<button onclick={() => (count += step)}>+{step} dispatch</button>
</div>

<style>
	.wrap { display: grid; gap: 8px; justify-items: start; }
	.label { margin: 0; font: 12px/16px system-ui; color: #a3a3a3; }
	.value { font: 600 28px/36px system-ui; font-variant-numeric: tabular-nums; }
	button { padding: 6px 12px; border: 1px solid #262626; border-radius: 6px; background: #fafafa; color: #0a0a0a; font: 500 13px/20px system-ui; cursor: pointer; }
	button:hover { background: #e5e5e5; }
</style>`;

/**
 * React artifacts are authored as function source with no imports: the runtime
 * binds `React`, `htm` and `default` before evaluating, the same contract
 * documented on `artifacts/react-runtime.ts`.
 */
const counterReactSource = `export default function Turns(props) {
	const [total, setTotal] = React.useState(props.start ?? 0);
	const label = { margin: 0, font: '12px/16px system-ui', color: 'inherit', opacity: 0.65 };

	return html\`<div style=\${{ display: 'grid', gap: '8px', justifyItems: 'start' }}>
		<p style=\${label}>React artifact</p>
		<strong style=\${{ font: '600 28px/36px system-ui', fontVariantNumeric: 'tabular-nums' }}>\${total}</strong>
		<button
			onClick=\${() => setTotal((n) => n + 1)}
			style=\${{
				padding: '6px 12px',
				borderRadius: '6px',
				border: '1px solid currentColor',
				background: 'transparent',
				color: 'inherit',
				font: '500 13px/20px system-ui',
				cursor: 'pointer'
			}}
		>+1 turn</button>
	</div>\`;
}`;

const rustButtonCode = `// gpui-kit: the same button the kit page renders, in Rust.
use gpui_kit::*;

fn primary_action() -> impl IntoElement {
    Button::new("save")
        .primary()
        .size(ButtonSize::Large)
        .icon("check")
        .label("Save the run")
        .on_click(|_, _, _| { /* durable: the run record is the source of truth */ })
}

fn secondary_action() -> impl IntoElement {
    Button::new("inspect")
        .outline()
        .label("Inspect actions")
        .tooltip("Keyboard- and pointer-reachable, never hover-only")
}`;

const runEventsRust = `// One user turn becomes one run (ADR 0002). Progress is streamed as
// action.* events; the database stays authoritative, so a refresh replays
// from the last event id instead of losing anything.
pub async fn stream_run(run_id: &RunId) -> Result<()> {
    let mut events = EventStream::attach(run_id).await?;
    while let Some(event) = events.next().await? {
        match event.kind {
            EventKind::ActionRunning { progress, .. } => {
                actions.apply(run_id, progress).await?;
            }
            EventKind::RunTerminal { .. } => break,
            _ => {}
        }
    }
    Ok(())
}`;

/**
 * Sample command output for the terminal artifact. It is written to look like a real
 * failing type-check, including the exit code, because a terminal artifact that only
 * *succeeded* would never be reviewed. The path and symbol are invented for the demo so
 * nothing here can be mistaken for a report about this repository.
 */
const checkTerminalLines = [
	{ kind: 'stdout' as const, text: '$ bunx tsc --noEmit src/routes/kit/example-runner.ts' },
	{ kind: 'dim' as const, text: 'type-checking the generated runner' },
	{ kind: 'stdout' as const, text: '' },
	{ kind: 'stdout' as const, text: 'src/routes/kit/example-runner.ts(41,7): error TS2304: Cannot find name \'scheduleRetry\'.' },
	{ kind: 'dim' as const, text: '  39 |  const result = await runner.run(request);' },
	{ kind: 'dim' as const, text: '  40 |  if (!result.ok) {' },
	{ kind: 'dim' as const, text: '  41 |    scheduleRetry(result.attempt);' },
	{ kind: 'dim' as const, text: '  42 |  }' },
	{ kind: 'dim' as const, text: '  43 |  return result;' },
	{ kind: 'stdout' as const, text: '' },
	{ kind: 'stdout' as const, text: 'Found 1 error in 1 file.' },
	{ kind: 'dim' as const, text: '============================================================' }
];

export const seededArtifacts: Record<string, Artifact> = {
	plan: {
		id: 'a-plan',
		kind: 'checklist',
		title: 'GPUI Kit browser port',
		summary: 'Plan the run is executing, kept inline so the turn is auditable.',
		data: {
			total: 4,
			items: [
				{ label: 'Transcribe the theme tokens', done: true, detail: 'radius, spacing, typography, motion' },
				{ label: 'Port the chat primitives', done: true, detail: 'Message, Bubble, Attachment, MessageScroller' },
				{ label: 'Port the overlay layer', done: true, detail: 'Sheet, Dialog, Tooltip, Tabs' },
				{ label: 'Prove it at three screen sizes', done: false, detail: 'phone, tablet, desktop' }
			]
		}
	},
	tokenWidget: {
		id: 'a-token-widget',
		kind: 'html',
		title: 'Token use card',
		summary: 'HTML artifact. Sandboxed frame: no network, no host styles, no access to the page.',
		source: tokenWidgetHtml,
		height: 'auto'
	},
	counterSvelte: {
		id: 'a-counter-svelte',
		kind: 'svelte',
		title: 'Turn counter',
		summary: 'Svelte artifact. Compiled in the browser when it renders.',
		source: counterSvelteSource,
		height: 'auto'
	},
	counterReact: {
		id: 'a-counter-react',
		kind: 'react',
		title: 'Turn counter',
		summary: 'React artifact. React and htm load on demand, only when a React artifact renders.',
		source: counterReactSource,
		height: 'auto'
	},
	rustButton: {
		id: 'a-rust-button',
		kind: 'code',
		title: 'rust-gpui-kit-buttons.rs',
		summary: 'The Rust source behind the kit page buttons, for comparison.',
		source: rustButtonCode,
		language: 'rust',
		height: 'auto'
	},
	runEvents: {
		id: 'a-run-events',
		kind: 'code',
		title: 'src/lib/server/runs/stream.rs',
		summary: 'Run streaming, the behaviour the progress card mirrors.',
		source: runEventsRust,
		language: 'rust',
		height: 'auto'
	},
	actionTable: {
		id: 'a-action-table',
		kind: 'table',
		title: 'Run actions',
		summary: 'Action-level ledger. Numeric columns right-align; long cells truncate with the full value on hover.',
		data: {
			footnote: '5 actions · 1 delegated to a sub-agent · 4,182 ms wall clock',
			columns: [
				{ key: 'outcome', label: 'Action', truncate: true },
				{ key: 'capability', label: 'Capability' },
				{ key: 'agent', label: 'Agent' },
				{ key: 'duration', label: 'Duration', numeric: true },
				{ key: 'status', label: 'Status' }
			],
			rows: [
				{ outcome: 'Reply to the request', capability: 'chat.reply', agent: 'EVE', duration: '1,204 ms', status: 'succeeded' },
				{ outcome: 'Record the request as a note', capability: 'note.write', agent: 'EVE', duration: '38 ms', status: 'succeeded' },
				{ outcome: 'Spawn the research sub-agent', capability: 'subagent.spawn', agent: 'scout', duration: '412 ms', status: 'succeeded' },
				{ outcome: 'Read the crate theme', capability: 'file.read', agent: 'scout', duration: '96 ms', status: 'succeeded' },
				{ outcome: 'Summarise findings', capability: 'memory.write', agent: 'scout', duration: '—', status: 'failed' }
			]
		}
	},
	eveStatus: {
		id: 'a-eve-status',
		kind: 'record',
		title: 'EVE runtime',
		data: {
			caption: 'Live supervisor state for this installation.',
			items: [
				{ label: 'Release', value: '2026.9.3', hint: 'Promoted from app-data/releases' },
				{ label: 'Transport', value: 'http://127.0.0.1:8787' },
				{ label: 'Model route', value: 'openrouter/stealth/space-bunny-alpha' },
				{ label: 'Sub-agents', value: '3 standing', hint: 'scout, reviewer, sonic' },
				{ label: 'Memory', value: '1,284 notes', hint: 'capture → retrieve → inject' }
			]
		}
	},
	products: {
		id: 'a-products',
		kind: 'products',
		title: 'Standing desks under £400',
		summary: 'Live retailer results. Prices and stock move; the card never presents itself as a quote.',
		data: {
			currency: 'GBP',
			products: [
				{
					name: 'Oakridge 140 cm electric desk',
					vendor: 'Currys',
					price: '£379.99',
					priceNote: '£120 cheaper than the median',
					rating: '4.6',
					reviews: 1284,
					imageUrl: productImage('Oakridge desk', 24),
					url: 'https://www.currys.co.uk/',
					badges: ['Best seller'],
					stock: 'in'
				},
				{
					name: 'Linnea sit-stand frame',
					vendor: 'IKEA',
					price: '£349.00',
					priceNote: 'Delivery in 4 weeks',
					rating: '4.4',
					reviews: 612,
					imageUrl: productImage('Linnea frame', 200),
					url: 'https://www.ikea.com/gb/en/',
					badges: ['Long delivery'],
					stock: 'low'
				},
				{
					name: 'Atlas Pro dual-motor desk',
					vendor: 'Amazon',
					price: '£399.99',
					priceNote: 'Prime delivery tomorrow',
					rating: '4.8',
					reviews: 3971,
					imageUrl: productImage('Atlas pro desk', 150),
					url: 'https://www.amazon.co.uk/',
					badges: ['Fastest'],
					stock: 'in'
				},
				{
					name: 'Compact Foldaway desk',
					vendor: 'Argos',
					price: '£229.99',
					priceNote: 'Out of stock',
					rating: '3.9',
					reviews: 208,
					imageUrl: productImage('Foldaway desk', 300),
					url: 'https://www.argos.co.uk/',
					stock: 'out'
				}
			]
		}
	},
	docsLink: {
		id: 'a-docs-link',
		kind: 'link',
		title: 'gpui-kit docs',
		data: {
			url: 'https://gpui-kit.com/docs/design-guides',
			site: 'GPUI Kit',
			host: 'gpui-kit.com',
			title: 'Design Guides',
			description: 'The normative guides for GPUI Kit applications: hierarchy, density, overlays, motion and interface copy.',
			imageUrl: usagePreview
		}
	},
	metrics: {
		id: 'a-metrics',
		kind: 'metrics',
		title: 'Turn metrics',
		summary: 'Cost and latency for the turn that produced these artifacts.',
		data: {
			period: 'This turn',
			metrics: [
				{ label: 'Wall clock', value: '4.2 s', delta: '1.1 s faster', direction: 'up', spark: [8, 7, 7, 6, 6, 5, 4] },
				{ label: 'Sub-agents', value: '2', delta: '+1', direction: 'down' },
				{ label: 'Artifacts', value: '16', delta: 'all rendered', direction: 'flat' },
				{ label: 'Errors', value: '0', delta: 'clean run', direction: 'flat' }
			]
		}
	},
	turnChart: {
		id: 'a-turn-chart',
		kind: 'chart',
		title: 'Tokens per turn',
		summary: 'Prompt against completion across the last eight turns.',
		data: {
			kind: 'bar',
			unit: 'tokens',
			labels: ['t1', 't2', 't3', 't4', 't5', 't6', 't7', 't8'],
			series: [
				{ label: 'Prompt', color: 4, points: [820, 940, 1100, 980, 1240, 1310, 1180, 1284] },
				{ label: 'Completion', color: 3, points: [1420, 1880, 2240, 2610, 2980, 3320, 3640, 3910] }
			]
		}
	},
	themeDiff: {
		id: 'a-theme-diff',
		kind: 'diff',
		title: 'src/lib/components/kit/kit.css',
		summary: 'The token layer added for this page, transcribed from the crate.',
		data: {
			path: 'src/lib/components/kit/kit.css',
			language: 'css',
			added: 4,
			removed: 1,
			lines: [
				{ kind: 'context', text: '.kit {' },
				{ kind: 'add', text: '\t--kit-background: #0a0a0a; /* neutral-950 */' },
				{ kind: 'add', text: '\t--kit-foreground: #fafafa; /* neutral-50 */' },
				{ kind: 'remove', text: '\t--background: oklch(0.985 0 0);' },
				{ kind: 'add', text: '\t--kit-radius-lg: 8px;' },
				{ kind: 'context', text: '}' }
			]
		}
	},
	files: {
		id: 'a-files',
		kind: 'files',
		title: 'Files this run touched',
		data: {
			root: 'src/lib/components/kit',
			files: [
				{ path: 'chat/model.ts', size: '2.5 kB', created: true },
				{ path: 'artifacts/types.ts', size: '6.5 kB', created: true },
				{ path: 'artifacts/cards/*.svelte', size: '14 files', created: true },
				{ path: 'demo/transcript.ts', size: '11.2 kB', created: true },
				{ path: 'kit.css', size: '11.6 kB', created: true },
				{ path: '../ui/button/button.svelte', size: '3.1 kB', modified: true }
			]
		}
	},
	subagentTimeline: {
		id: 'a-timeline',
		kind: 'timeline',
		title: 'Sub-agent fan-out',
		summary: 'One request, two sub-agents, the actions each performed.',
		data: {
			title: 'run_7fq2 · "how does gpui-kit hold up in a browser?"',
			startedAt: '09:41:02',
			durationMs: 4182,
			steps: [
				{
					id: 's1',
					label: 'Reply to the request',
					kind: 'request',
					status: 'succeeded',
					durationMs: 1204,
					children: [
						{ label: 'Assemble prompt from memory', status: 'succeeded', detail: '4 notes retrieved' },
						{ label: 'Stream to OpenRouter', status: 'succeeded', detail: '3,910 completion tokens' }
					]
				},
				{
					id: 's2',
					label: 'scout',
					kind: 'subagent',
					status: 'succeeded',
					durationMs: 2610,
					detail: 'crate source sweep',
					children: [
						{ label: 'Read theme tokens', status: 'succeeded', detail: 'theme_tokens.rs' },
						{ label: 'Read chat components', status: 'succeeded', detail: 'message, bubble, attachment' },
						{ label: 'Confirm wasm build support', status: 'succeeded', detail: 'init() comment names WASM' }
					]
				},
				{
					id: 's3',
					label: 'reviewer',
					kind: 'subagent',
					status: 'failed',
					durationMs: 368,
					detail: 'no such capability: review.ui'
				}
			]
		}
	},
	toolPayload: {
		id: 'a-json',
		kind: 'json',
		title: 'subagent.spawn payload',
		data: {
			value: {
				capability: 'subagent.spawn',
				agent: 'scout',
				budget: { maxTurns: 6, maxTokens: 24_000 },
				scope: ['gpui-component/src/theme', 'gpui-base/src/theme_tokens.rs'],
				returnAs: { kind: 'record', title: 'Findings' },
				onFailure: 'report, do not retry'
			}
		}
	},
	checkOutput: {
		id: 'a-terminal',
		kind: 'terminal',
		title: 'Type-check output',
		summary: 'A failing command, because a terminal artifact that only ever succeeds is never reviewed.',
		data: {
			command: 'bunx tsc --noEmit src/routes/kit/example-runner.ts',
			cwd: 'alexia-bot-Lexosa-Test-GPUI',
			exitCode: 1,
			lines: checkTerminalLines
		}
	},
	portrait: {
		id: 'a-media',
		kind: 'media',
		title: 'Generated chart',
		data: {
			imageUrl: usagePreview,
			alt: 'Bar chart of token use rising across six turns',
			caption: 'Sub-agent output, rendered as an image rather than as markup.',
			ratio: '2 / 1'
		}
	}
};

/**
 * Seeded conversation. Timestamps are offsets from page load so the page reads
 * like a session that has been going for a few minutes rather than one that
 * started a second ago.
 */
export function seededConversation(now: number): ChatMessage[] {
	return [
		{
			id: 'm1',
			authorKind: 'user',
			body: 'Were we going to use gpui-kit? I want to see how the UI holds up in a browser before we commit.',
			createdAt: now - 9 * minute
		},
		{
			id: 'm2',
			authorKind: 'lexia',
			body: "Not yet. The app is SvelteKit; GPUI is Rust, so nothing in this repo renders GPUI directly. This page ports the component layer and the default theme so the look is judged on the device you actually hold.",
			createdAt: now - 8 * minute,
			artifacts: [seededArtifacts.docsLink]
		},
		{
			id: 'm3',
			authorKind: 'user',
			body: 'Show me what a run looks like when it fans out to sub-agents, and what the agent can hand back.',
			createdAt: now - 5 * minute
		},
		{
			id: 'm4',
			authorKind: 'lexia',
			body: 'Here is the shape of that run, and every artifact shape the agent can emit. The fan-out is the parent run plus the sub-agents it spawned; the artifacts render inline under the message that produced them.',
			createdAt: now - 4 * minute,
			runId: 'run_7fq2',
			artifacts: [
				seededArtifacts.subagentTimeline,
				seededArtifacts.plan,
				seededArtifacts.actionTable,
				seededArtifacts.eveStatus,
				seededArtifacts.turnChart,
				seededArtifacts.metrics,
				seededArtifacts.products,
				seededArtifacts.files,
				seededArtifacts.themeDiff,
				seededArtifacts.toolPayload,
				seededArtifacts.checkOutput,
				seededArtifacts.tokenWidget,
				seededArtifacts.counterSvelte,
				seededArtifacts.counterReact,
				seededArtifacts.rustButton,
				seededArtifacts.portrait
			]
		}
	];
}

/** A patch the harness applies to a run, mirroring the server's `action.*` and `run.terminal` events. */
type RunPatch =
	| { type: 'add'; action: RunAction }
	| { type: 'status'; id: string; status: RunAction['status']; progress?: string | null }
	| { type: 'finish'; outcome: 'succeeded' | 'failed' | 'cancelled' };

/**
 * The harness replays this to exercise a live run. Each step is applied at the
 * listed delay against the run start, in the same order the server would emit
 * the equivalent events, so the progress card, the fan-out and the artifacts all
 * move the way they do in production.
 */
export const scriptedRun: Array<{ at: number; action: RunPatch }> = [];

const plan: Array<[number, RunPatch]> = [
	[120, { type: 'add', action: { id: 'a1', outcome: 'Reply to the request', capability: 'chat.reply', status: 'running', progress: 'streaming', error: null, agent: 'EVE' } }],
	[520, { type: 'add', action: { id: 'a2', outcome: 'Spin up the scout sub-agent', capability: 'subagent.spawn', status: 'running', progress: null, error: null, agent: 'scout' } }],
	[900, { type: 'add', action: { id: 'a2.1', outcome: 'Read gpui-base theme tokens', capability: 'file.read', status: 'running', progress: null, error: null, agent: 'scout' } }],
	[1500, { type: 'status', id: 'a2.1', status: 'succeeded', progress: '1 file' }],
	[1700, { type: 'add', action: { id: 'a3', outcome: 'Build the artifact set', capability: 'artifact.write', status: 'queued', progress: null, error: null } }],
	[2400, { type: 'status', id: 'a1', status: 'succeeded', progress: 'done' }],
	[2600, { type: 'add', action: { id: 'a4', outcome: 'Record the turn as a note', capability: 'note.write', status: 'queued', progress: null, error: null, agent: 'EVE' } }],
	[3200, { type: 'status', id: 'a2', status: 'succeeded', progress: '3 findings' }],
	[3600, { type: 'status', id: 'a3', status: 'running', progress: '4/16' }],
	[4200, { type: 'status', id: 'a3', status: 'succeeded', progress: '16 rendered' }],
	[4600, { type: 'status', id: 'a4', status: 'succeeded', progress: 'done' }],
	[5000, { type: 'finish', outcome: 'succeeded' }]
];

for (const [at, action] of plan) scriptedRun.push({ at, action });