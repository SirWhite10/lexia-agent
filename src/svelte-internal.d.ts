/**
 * `svelte/internal/client` is imported by the artifact Svelte runtime
 * (`src/lib/components/kit/artifacts/render-svelte.svelte`), which evaluates
 * compiled artifact modules against the same client internals the compiler
 * generated them for. The package does not ship a type declaration for that
 * subpath, so it is declared here rather than papered over at the call site.
 */
declare module 'svelte/internal/client';