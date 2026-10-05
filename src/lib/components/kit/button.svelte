<script lang="ts" module>
	import { cn, type WithElementRef } from '#lib/utils.js';
	import type { Snippet } from 'svelte';
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';
	export type ButtonVariant =
		| 'default'
		| 'primary'
		| 'secondary'
		| 'ghost'
		| 'link'
		| 'text'
		| 'danger'
		| 'warning'
		| 'success'
		| 'info';

	export type ButtonSize = 'xsmall' | 'small' | 'medium' | 'large';

	/** `gpui-component/src/button/button.rs` → `ButtonRounded`: radius * 0.5, radius, radius * 2, 0. */
	export type ButtonRounded = 'none' | 'small' | 'medium' | 'large';

	/**
	 * Variant surfaces, transcribed from `ButtonVariant::{normal,hovered,active}` in
	 * button.rs. Status variants keep their own colour and dim on hover instead of
	 * mixing a new surface, because the theme has no separate hover token for them.
	 */
	const VARIANT: Record<ButtonVariant, { bg: string; hover: string; active: string; fg: string; border: string }> = {
		default: {
			bg: 'bg-(--kit-button)',
			hover: 'hover:bg-(--kit-button-hover)',
			active: 'active:bg-(--kit-button-active)',
			fg: 'text-[color:var(--kit-button-foreground)]',
			border: 'border-(--kit-input-border)'
		},
		primary: {
			bg: 'bg-(--kit-primary)',
			hover: 'hover:bg-(--kit-primary-hover)',
			active: 'active:bg-(--kit-primary-active)',
			fg: 'text-[color:var(--kit-primary-foreground)]',
			border: 'border-(--kit-primary)'
		},
		secondary: {
			bg: 'bg-(--kit-secondary)',
			hover: 'hover:bg-(--kit-secondary-hover)',
			active: 'active:bg-(--kit-secondary-active)',
			fg: 'text-[color:var(--kit-secondary-foreground)]',
			border: 'border-(--kit-border)'
		},
		danger: {
			bg: 'bg-(--kit-danger)',
			hover: 'hover:opacity-90',
			active: 'active:opacity-80',
			fg: 'text-[color:var(--kit-status-foreground)]',
			border: 'border-(--kit-danger)'
		},
		warning: {
			bg: 'bg-(--kit-warning)',
			hover: 'hover:opacity-90',
			active: 'active:opacity-80',
			fg: 'text-[color:var(--kit-status-foreground)]',
			border: 'border-(--kit-warning)'
		},
		success: {
			bg: 'bg-(--kit-success)',
			hover: 'hover:opacity-90',
			active: 'active:opacity-80',
			fg: 'text-[color:var(--kit-status-foreground)]',
			border: 'border-(--kit-success)'
		},
		info: {
			bg: 'bg-(--kit-info)',
			hover: 'hover:opacity-90',
			active: 'active:opacity-80',
			fg: 'text-[color:var(--kit-status-foreground)]',
			border: 'border-(--kit-info)'
		},
		ghost: {
			bg: 'bg-transparent',
			hover: 'hover:bg-(--kit-muted)',
			active: 'active:bg-(--kit-accent)',
			fg: 'text-[color:var(--kit-secondary-foreground)]',
			border: 'border-transparent'
		},
		link: {
			bg: 'bg-transparent',
			hover: 'hover:bg-(--kit-muted)',
			active: 'active:bg-(--kit-accent)',
			fg: 'text-[color:var(--kit-link)]',
			border: 'border-transparent'
		},
		text: {
			bg: 'bg-transparent',
			hover: 'hover:bg-(--kit-muted)',
			active: 'active:bg-(--kit-accent)',
			fg: 'text-[color:var(--kit-foreground)]/90',
			border: 'border-transparent'
		}
	};

	/**
	 * Heights and padding from button.rs: xsmall `h_5 px_1`, small `h_6 px_2`,
	 * medium `h_8 px_2p5`, large `h_8 px_3`; gap 1 for the two small sizes and gap 2
	 * above. Icon-only buttons are square at the same height.
	 */
	const SIZE: Record<ButtonSize, { height: string; padding: string; gap: string; text: string; icon: string }> = {
		xsmall: { height: 'h-[20px]', padding: 'px-(--kit-space-xs)', gap: 'gap-(--kit-space-xs)', text: 'text-[length:var(--kit-text-xs)]', icon: '[&_svg]:size-[14px]' },
		small: { height: 'h-[24px]', padding: 'px-(--kit-space-sm)', gap: 'gap-(--kit-space-xs)', text: 'text-[length:var(--kit-text-sm)]', icon: '[&_svg]:size-[16px]' },
		medium: { height: 'h-[32px]', padding: 'px-[10px]', gap: 'gap-(--kit-space-sm)', text: 'text-[length:var(--kit-text-md)]', icon: '[&_svg]:size-[18px]' },
		large: { height: 'h-[32px]', padding: 'px-(--kit-space-md)', gap: 'gap-(--kit-space-sm)', text: 'text-[length:var(--kit-text-md)]', icon: '[&_svg]:size-[18px]' }
	};

	const ROUNDED: Record<ButtonRounded, string> = {
		none: 'rounded-(--kit-radius-none)',
		small: 'rounded-[4px]',
		medium: 'rounded-(--kit-radius-lg)',
		large: 'rounded-[16px]'
	};

	/**
	 * An intersection, not a union: a button carries anchor attributes when it renders
	 * an anchor, and a union leaves the spread untypeable at the call site.
	 */
	export type ButtonProps = WithElementRef<HTMLButtonAttributes> &
		WithElementRef<HTMLAnchorAttributes> & {
			variant?: ButtonVariant;
			size?: ButtonSize;
			rounded?: ButtonRounded;
			outline?: boolean;
			iconOnly?: boolean;
			loading?: boolean;
			children?: Snippet;
		};

	/**
	 * Class recipe shared by the button and the route's own controls. Exported so a
	 * caller can style a link that must read as a button without rendering one.
	 */
	export function buttonClass(options: {
		variant?: ButtonVariant;
		size?: ButtonSize;
		rounded?: ButtonRounded;
		/** `outline` is a modifier upstream: it keeps the variant's text and border and swaps the surface. */
		outline?: boolean;
		/** Icon-only buttons are square and drop their horizontal padding. */
		iconOnly?: boolean;
		class?: string | undefined;
	}): string {
		const { variant = 'secondary', size = 'medium', rounded = 'medium', outline = false, iconOnly = false, class: className } = options;
		const surface = VARIANT[variant];
		const metrics = SIZE[size];
		return cn(
			'inline-flex shrink-0 items-center justify-center whitespace-nowrap border leading-(--kit-leading-sm) font-medium select-none',
			// Desktop convention: the arrow cursor on actions. Only the link variants
			// read as destinations, so only they get the pointing hand.
			variant === 'link' || variant === 'text' ? 'cursor-pointer' : 'cursor-default',
			metrics.height,
			metrics.gap,
			metrics.text,
			metrics.icon,
			iconOnly ? 'aspect-square px-0' : metrics.padding,
			ROUNDED[rounded],
			outline ? 'bg-(--kit-background)' : surface.bg,
			outline ? 'hover:bg-(--kit-muted)' : surface.hover,
			outline ? 'active:bg-(--kit-accent)' : surface.active,
			surface.fg,
			surface.border,
			'disabled:pointer-events-none disabled:opacity-50',
			// A loading button keeps its colours and stops reacting to the pointer: it is
			// not waiting for another click.
			'aria-busy:pointer-events-none',
			className
		);
	}
</script>

<script lang="ts">
	import Spinner from './spinner.svelte';

	let {
		variant = 'secondary',
		size = 'medium',
		rounded = 'medium',
		outline = false,
		iconOnly = false,
		loading = false,
		disabled = false,
		href,
		type = 'button',
		class: className,
		children,
		...restProps
	}: ButtonProps = $props();

	const resolvedClass = $derived(
		buttonClass({ variant, size, rounded, outline, iconOnly, class: cn(className, loading && 'pointer-events-none') })
	);
</script>

{#if href}
	<a
		data-kit="button"
		class={resolvedClass}
		{href}
		aria-disabled={disabled || loading ? 'true' : undefined}
		aria-busy={loading ? 'true' : undefined}
		tabindex={disabled || loading ? -1 : undefined}
		{...restProps}
	>
		{#if loading}<Spinner size={size === 'xsmall' ? 'sm' : 'md'} />{/if}
		{@render children?.()}
	</a>
{:else}
	<button
		data-kit="button"
		class={resolvedClass}
		{type}
		disabled={disabled}
		aria-busy={loading ? 'true' : undefined}
		{...restProps}
	>
		{#if loading}<Spinner size={size === 'xsmall' ? 'sm' : 'md'} />{/if}
		{@render children?.()}
	</button>
{/if}