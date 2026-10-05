/**
 * React artifact runtime.
 *
 * The contract with the agent, in full:
 *
 *  - The artifact source is a module body with **no imports**. `React` and
 *    `html` are free identifiers supplied by this runtime, and the component is
 *    handed back with `export default function ...` (or `export default <expr>`),
 *    which is rewritten to a `return` before evaluation.
 *  - `html` is `htm` bound to `React.createElement`, so a template literal is a
 *    real element tree: html`<button onClick=${fn}>Text</button>`.
 *  - The source is evaluated with `new Function('React', 'html', code)` and the
 *    returned value is mounted with `createRoot` from `react-dom/client`.
 *  - Nothing else is in scope: no host state, no host imports. A component that
 *    throws reports through the same inline error panel the Svelte renderer
 *    uses, rather than taking the chat down with it.
 *
 * React, `react-dom` and `htm` are loaded with dynamic `import()` on the first
 * React artifact that renders and reused afterwards, so a session that never
 * shows a React artifact never downloads them: none of the three appears in a
 * static import anywhere in this module.
 */

import type * as ReactNamespace from 'react';
import type * as ReactDomClient from 'react-dom/client';
import type { ComponentType, createElement as ReactCreateElement } from 'react';

export type ReactArtifactError = { phase: 'load' | 'evaluate' | 'render'; message: string };

export type ReactMount = { unmount: () => void };
type Runtime = {
	React: typeof ReactNamespace;
	createRoot: typeof ReactDomClient.createRoot;
	createElement: typeof ReactCreateElement;
	/** `htm` bound to `React.createElement`; see the contract above. */
	html: (strings: TemplateStringsArray, ...values: unknown[]) => unknown;
};

let runtime: Promise<Runtime> | undefined;

async function loadRuntime(): Promise<Runtime> {
	runtime ??= (async () => {
		const [reactModule, reactDomModule, htmModule] = await Promise.all([
			import('react'),
			import('react-dom/client'),
			import('htm')
		]);
		// Unchecked casts, both for the same reason: `unwrapDefault` returns
		// `unknown` because the module shape is chosen by the bundler, and the
		// namespaces it returns are exactly the two typed modules imported below.
		const React = unwrapDefault(reactModule) as typeof ReactNamespace;
		const reactDom = unwrapDefault(reactDomModule) as typeof ReactDomClient;
		return {
			React,
			createRoot: reactDom.createRoot,
			createElement: React.createElement,
			html: htmModule.default.bind(React.createElement)
		};
	})().catch((cause) => {
		// A failed load must not poison the module: the next artifact retries.
		runtime = undefined;
		throw cause;
	});
	return runtime;
}

/**
 * React and `react-dom` ship CommonJS, so a bundler's interop can hand back a
 * namespace whose exports live under `default` rather than on the namespace
 * itself. Both shapes occur in practice, so the runtime accepts either rather
 * than assuming the ESM shape and failing with "React.useState is not a function".
 */
function unwrapDefault(namespace: unknown): unknown {
	return namespace !== null && typeof namespace === 'object' && 'default' in namespace ? namespace.default : namespace;
}

function describe(cause: unknown): string {
	return cause instanceof Error ? cause.message : String(cause);
}

/** Rewrites the artifact module's single `export default` into a `return`. */
function toFactoryBody(source: string): string {
	return source.replace(/\bexport\s+default\s+/, 'return ');
}

/**
 * Evaluates and mounts a React artifact into `target`, returning its teardown.
 *
 * `onRuntimeError` exists because a React artifact that throws while rendering does not
 * throw out of `root.render`: React catches it, unmounts the tree, and logs it. Without
 * this callback an agent artifact that fails simply renders nothing, which reads as an
 * empty artifact rather than as a failure.
 *
 * Throws a `ReactArtifactError` for the failures that happen before rendering starts.
 */
export async function mountReactArtifact(
	source: string,
	target: HTMLElement,
	onRuntimeError?: (message: string) => void
): Promise<ReactMount> {
	let loaded: Runtime;
	try {
		loaded = await loadRuntime();
	} catch (cause) {
		throw { phase: 'load', message: describe(cause) } satisfies ReactArtifactError;
	}

	let Component: ComponentType;
	try {
		const factory = new Function('React', 'html', toFactoryBody(source)) as (
			React: unknown,
			html: unknown
		) => unknown;
		const exported = factory(loaded.React, loaded.html);
		if (typeof exported !== 'function') {
			throw new Error('The artifact did not export a component as its default.');
		}
		Component = exported as ComponentType;
	} catch (cause) {
		throw { phase: 'evaluate', message: describe(cause) } satisfies ReactArtifactError;
	}

	try {
		const root = loaded.createRoot(target, {
			onUncaughtError: (cause: unknown) => onRuntimeError?.(describe(cause))
		});
		root.render(loaded.createElement(Component));
		return { unmount: () => root.unmount() };
	} catch (cause) {
		throw { phase: 'render', message: describe(cause) } satisfies ReactArtifactError;
	}
}