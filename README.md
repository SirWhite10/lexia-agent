# Lexia-Agent

Lexia is a server-side SvelteKit agent harness for chat, coding, workflows, and tool-assisted work. Jev routes requests, EVE executes them, and OpenRouter provides the initial model pool.

## Development

Install dependencies and start the SvelteKit development server with Bun:

```sh
bun install
bun run dev
```

Run validation and build the production frontend:

```sh
bun run check
bun run build
bun run preview
```

The application uses Svelte 5 runes mode, shadcn-svelte primitives, Quicksand for interface text, and IA Writer Duo for code and operational output.

## Repository orientation

Read [AGENTS.md](./AGENTS.md) first. The workspace routers provide focused entry points for product architecture, content, and operations:

- [PRODUCT.md](./PRODUCT.md)
- [CONTENT.md](./CONTENT.md)
- [OPERATIONS.md](./OPERATIONS.md)
- [CONTEXT.md](./CONTEXT.md)
- [DESIGN.md](./DESIGN.md)
