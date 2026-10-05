<script lang="ts">
	import { cn } from '#lib/utils.js';
	import type { ArtifactLanguage } from './types.js';

	/**
	 * A deliberately small tokenizer. It only has to be good enough to read
	 * code by, and it must never change the text: the concatenation of every
	 * token's text is the source, unchanged. Anything it cannot classify is
	 * emitted as `plain`, so an unrecognised construct renders verbatim rather
	 * than being swallowed by a greedy pattern.
	 */
	type TokenKind = 'plain' | 'comment' | 'string' | 'keyword' | 'number' | 'function' | 'tag';

	type Token = { text: string; kind: TokenKind };

	const KEYWORDS = new Set([
		'as', 'async', 'await', 'break', 'case', 'catch', 'class', 'const', 'continue', 'debugger',
		'default', 'delete', 'do', 'else', 'enum', 'export', 'extends', 'finally', 'fn', 'for',
		'from', 'function', 'if', 'impl', 'import', 'in', 'instanceof', 'interface', 'let', 'match',
		'mod', 'mut', 'new', 'of', 'private', 'pub', 'public', 'return', 'static', 'struct', 'super',
		'switch', 'this', 'throw', 'trait', 'try', 'type', 'typeof', 'use', 'var', 'void', 'while',
		'yield'
	]);

	const LITERALS = new Set(['true', 'false', 'null', 'undefined', 'None', 'Some', 'Ok', 'Err']);

	// One pass, ordered: earlier alternatives win, so `//` inside a string that
	// started earlier cannot re-open as a comment.
	const PATTERN =
		/\/\/[^\n]*|\/\*[\s\S]*?\*\/|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`|<[A-Za-z/][^>\s]*|\b\d+(?:\.\d+)?\b|\b[A-Za-z_$][\w$]*/g;

	function classify(word: string, source: string, end: number): TokenKind {
		if (KEYWORDS.has(word)) return 'keyword';
		if (LITERALS.has(word)) return 'keyword';
		if (source[end] === '(' || source[end] === '<') return 'function';
		if (/^[A-Z]/.test(word)) return 'function';
		return 'plain';
	}

	function tokenize(text: string): Token[] {
		const tokens: Token[] = [];
		let cursor = 0;
		PATTERN.lastIndex = 0;
		let match: RegExpExecArray | null;
		while ((match = PATTERN.exec(text)) !== null) {
			if (match.index > cursor) {
				tokens.push({ text: text.slice(cursor, match.index), kind: 'plain' });
			}
			const word = match[0];
			let kind: TokenKind;
			if (word.startsWith('//') || word.startsWith('/*')) kind = 'comment';
			else if (word.startsWith('<') && /<\/?[A-Za-z]/.test(word)) kind = 'tag';
			else if (/^\d/.test(word)) kind = 'number';
			else if (/^["'`]/.test(word)) kind = 'string';
			else kind = classify(word, text, match.index + word.length);
			tokens.push({ text: word, kind });
			cursor = match.index + word.length;
		}
		if (cursor < text.length) tokens.push({ text: text.slice(cursor), kind: 'plain' });
		return tokens;
	}

	const KIND_CLASS: Record<TokenKind, string> = {
		plain: '',
		comment: 'text-[color:var(--kit-code-comment)] italic',
		string: 'text-[color:var(--kit-code-string)]',
		keyword: 'text-[color:var(--kit-code-keyword)]',
		number: 'text-[color:var(--kit-code-number)]',
		function: 'text-[color:var(--kit-code-function)]',
		tag: 'text-[color:var(--kit-code-tag)]'
	};

	interface Props {
		source: string;
		language?: ArtifactLanguage;
		filename?: string;
		class?: string;
		/**
		 * Height of the scrolling body. The default bounds a standalone view; a
		 * host that already gives the source a bounded region passes `max-h-none`
		 * so the two scroll regions do not fight.
		 */
		bodyClass?: string;
	}

	let { source, language, filename, class: className, bodyClass = 'max-h-96' }: Props = $props();

	const label = $derived(filename ?? language ?? 'source');
	const lines = $derived.by(() => source.replace(/\n$/, '').split('\n').map(tokenize));

	let copied = $state(false);
	let copyTimer: ReturnType<typeof setTimeout> | undefined;

	async function copy() {
		try {
			await navigator.clipboard.writeText(source);
			copied = true;
			clearTimeout(copyTimer);
			copyTimer = setTimeout(() => (copied = false), 1500);
		} catch {
			copied = false;
		}
	}

	$effect(() => () => clearTimeout(copyTimer));
</script>

<div data-kit="code-view" class={cn('overflow-hidden rounded-(--kit-radius-md) border border-(--kit-border) bg-(--kit-code-background)', className)}>
	<div class="flex items-center justify-between gap-(--kit-space-sm) border-b border-(--kit-border) px-(--kit-space-sm) py-(--kit-space-xs)">
		<span class="truncate font-mono text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">{label}</span>
		<button
			type="button"
			onclick={copy}
			aria-label={copied ? 'Source copied to the clipboard' : 'Copy source to the clipboard'}
			class="rounded-(--kit-radius-sm) border border-(--kit-border) px-(--kit-space-sm) py-(--kit-space-xxs) text-[length:var(--kit-text-xs)] text-[color:var(--kit-foreground)] transition-colors duration-(--kit-duration-fast) hover:bg-(--kit-secondary) active:bg-(--kit-secondary-active) disabled:opacity-50"
		>
			{copied ? 'Copied' : 'Copy'}
		</button>
	</div>
	<!-- A scrollable region has to be focusable, or a keyboard reader cannot scroll it. -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div
		role="region"
		aria-label="Source, scrollable"
		tabindex="0"
		class={cn('kit-scroll overflow-auto', bodyClass)}
	>
		<pre class="w-max min-w-full py-(--kit-space-sm) font-mono text-[length:var(--kit-text-mono)] leading-(--kit-leading-mono) text-[color:var(--kit-code-foreground)]"><code
				>{#each lines as line, index (index)}<span class="flex w-max min-w-full"
					><span
						aria-hidden="true"
						class="sticky left-0 w-(--kit-space-xxl) shrink-0 select-none bg-(--kit-code-background) pr-(--kit-space-sm) text-right text-[color:var(--kit-muted-foreground)]"
						>{index + 1}</span
					><span class="whitespace-pre pr-(--kit-space-lg)"
						>{#each line as token, tokenIndex (tokenIndex)}<span class={KIND_CLASS[token.kind]}>{token.text}</span
							>{/each}</span
					></span
				>{/each}</code
			></pre
		>
	</div>
</div>