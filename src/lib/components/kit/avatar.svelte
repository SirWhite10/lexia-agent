<script lang="ts" module>
	/** `avatar/mod.rs::avatar_size` is 16/24/48/80px; the port offers a denser
	 * ladder (24/32/40/56) so an avatar can carry chat rows without becoming
	 * the largest object on the page. */
	export type KitAvatarSize = 'sm' | 'md' | 'lg' | 'xl';

	export type KitAvatarProps = {
		src?: string;
		/** Used for the initials fallback and as the image's `alt`. */
		name?: string;
		size?: KitAvatarSize;
		class?: string;
	};
</script>

<script lang="ts">
	import { cn } from '#lib/utils.js';

	let { src, name, size = 'md', class: className }: KitAvatarProps = $props();

	const SIZE_CLASS: Record<KitAvatarSize, string> = {
		sm: 'size-6 text-[length:var(--kit-text-xs)]',
		md: 'size-8 text-[length:var(--kit-text-sm)]',
		lg: 'size-10 text-[length:var(--kit-text-md)]',
		xl: 'size-14 text-[length:var(--kit-text-lg)]',
	};

	/** `avatar.rs::extract_text_initials`: the first letter of the first two
	 * words, uppercased; a single-letter result falls back to the first two
	 * characters of the whole name so a one-word name still reads. */
	function initials(name: string | undefined): string {
		if (!name) return '';

		const letters = name
			.split(' ')
			.filter(Boolean)
			.map((word) => word.charAt(0))
			.slice(0, 2)
			.join('');

		return (letters.length === 1 ? name.slice(0, 2) : letters).toUpperCase();
	}

	let failed = $state(false);

	// A new source deserves a fresh attempt: the previous one errored out.
	$effect(() => {
		void src;
		failed = false;
	});

	const showImage = $derived(Boolean(src) && !failed);
</script>

<span
	data-kit="avatar"
	class={cn(
		'inline-flex shrink-0 items-center justify-center overflow-hidden rounded-(--kit-radius-full) bg-(--kit-muted) font-medium text-[color:var(--kit-foreground)]',
		SIZE_CLASS[size],
		className,
	)}
>
	{#if showImage}
		<img
			src={src}
			alt={name ?? ''}
			class="size-full object-cover"
			onerror={() => (failed = true)}
		/>
	{:else}
		<span aria-hidden="true" class="leading-none select-none">{initials(name)}</span>
		{#if name}
			<span class="sr-only">{name}</span>
		{/if}
	{/if}
</span>
