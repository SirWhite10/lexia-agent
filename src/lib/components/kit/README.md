# GPUI Kit port (`src/lib/components/kit`)

A browser port of the **GPUI Kit** component layer, used by the `/kit` test page
to answer one question: *how does the GPUI Kit UI hold up in a browser, on the
device you actually hold?*

## Why a port and not the real thing

`gpui-kit` is a Rust crate. It renders through GPUI, which draws to a native
window surface; the `wasm32` build is acknowledged upstream (see the comment in
`gpui-component/src/theme/mod.rs::init`) but there is no browser backend to host
it, so a SvelteKit route cannot embed it. This directory therefore reproduces
GPUI Kit's *component layer and visual language* in Svelte 5 so the interface can
be judged in the browser. It is a preview surface, not a second design system:
nothing outside `/kit` imports from here, and `kit.css` scopes every token to
`.kit` so the Lexia design tokens in `src/routes/layout.css` are untouched.


## Status

**Web is alpha** ([WF-DEC-003](../../docs/planning/tickets/WF-DEC-003-gpui-kit-forefront-web-alpha.md)).
The theme transcription and the component layer below are real and load-bearing;
the browser renderer for the actual crate is not yet established, and
[WF-INV-002](../../docs/planning/tickets/WF-INV-002-gpui-kit-web-renderer-reality.md)
exists to settle that before anything here is treated as a shipping surface.

This directory stays in the tree as the reference implementation and the parity
oracle for the GPUI Kit port. Do not delete it while the migration is in flight.

## Source of truth for every value

Values are transcribed from `gpui-component 0.7.0` / `gpui-base 0.7.0` in the
local cargo registry, not invented. When a number below changes upstream, this
directory changes with it:

| What | Where it came from |
| --- | --- |
| Colour tokens | `gpui-component-0.7.0/src/theme/default-theme.json` + `default-colors.json` |
| Radius tokens | `gpui-base-0.7.0/src/theme_tokens.rs` → `RadiusTokens::default` |
| Spacing tokens | `gpui-base-0.7.0/src/theme_tokens.rs` → `SpacingTokens::default` |
| Typography tokens | `gpui-base-0.7.0/src/theme_tokens.rs` → `TypographyTokens::default` |
| Motion tokens | `gpui-component-0.7.0/src/theme/motion.rs` → `MotionTokens::default` |
| Component anatomy | the component's own `RenderOnce::render` in `gpui-component-0.7.0/src/` |

`kit.css` carries the transcription with the source path in a comment at each
block, so a reader can check a value without leaving the file.

## Token contract

Tokens live on `.kit` and are read with the Tailwind arbitrary-value shorthand:

```svelte
<button class="bg-(--kit-primary) text-[color:var(--kit-primary-foreground)] rounded-(--kit-radius-md)">Save</button>
```

`text-*` is the one ambiguous Tailwind utility here, so its variables always carry
an explicit data type: `text-[length:var(--kit-text-sm)]` for the type scale and
`text-[color:var(--kit-muted-foreground)]` for colour. Written as
`text-(--kit-text-sm)` Tailwind resolves it as a colour, which silently drops both
the size and the colour.

Never write a raw hex, `rgb()`, or `oklch()` in a component. Every colour comes
from a `--kit-*` variable so light/dark and any future theme change propagate.
Radii come from `--kit-radius-*` (`sm` 3px, `md` 6px, `lg` 8px, `xl` 12px,
`2xl` 20px = `radius * 2.5`, `3xl` 24px, `full` 9999px); spacing from
`--kit-space-*` (`xxs` 2, `xs` 4, `sm` 8, `md` 12, `lg` 16, `xl` 24, `xxl` 32);
type from `--kit-text-*` / `--kit-leading-*` (xs 12/16, sm 14/20, md 16/24,
lg 18/28, xl 20/28, mono 13/20); motion from `--kit-duration-*` and
`--kit-ease-*` (fast 120ms, normal 180ms, slow 280ms).

Derived radii: GPUI Kit computes surface tiers from the theme radius
(`radius_2xl = radius * 2.5`, `radius_3xl = radius * 3`, `radius_4xl = radius * 3.5`),
so `--kit-radius-2xl` is 20px, not the app's `--radius-2xl`.

## Verified component specs

Read from the crate; use these rather than inventing metrics.

### Button (`button/button.rs`)

- Sizes (`Size`): `xsmall` = `size_5` 20px, `small` = `size_6` 24px,
  `medium` = `size_8` 32px, `large` = `size_8` 32px, or a custom `Size(px)`.
- Icon-only (no label and no children): square at the size above.
- Labelled padding: `xsmall` `h_5 px_1`, `small` `h_6 px_2`, `medium` `h_8 px_2p5`,
  `large` `h_8 px_3`. `compact` adds a matching `min_w_*`.
- Gap: `gap_1` for xsmall/small, `gap_2` otherwise.
- Rounding (`ButtonRounded`): `small` = `radius * 0.5`, `medium` = `radius` (8px),
  `large` = `radius * 2` (16px), `none`, or a custom value.
- Variants and their colours:
  - `primary`: bg `primary`, fg `primary_foreground`, border `primary`,
    hover `primary_hover`, active `primary_active`.
  - `secondary`: bg `secondary`, fg `secondary_foreground`, border `border`,
    hover `secondary_hover`, active `secondary_active`.
  - `default`: bg is `input` mixed 30% toward transparent in dark, plain
    `background` in light; fg `foreground`; border `input`; hover `input` mixed
    50% toward transparent; active darkens 20% in dark, 10% in light.
  - `ghost` / `link` / `text`: transparent background, transparent border.
    `link` takes the `link` colour and underlines; `text` uses
    `foreground` at 90% opacity.
  - `danger`, `warning`, `success`, `info`: bg is the matching status colour,
    fg is its `*_foreground`.
  - `outline` is a modifier, not a variant: it keeps the variant's foreground and
    border and swaps the background for the surface colour.
- Cursor: default arrow. Only `link` and `text` show the pointing hand.
- A loading button keeps its normal colours and must not react to the pointer.
- Focus ring: 2px in `ring`, 2px offset, `:focus-visible` only.

### Chat trio (`message.rs`, `bubble.rs`)

- `Message` row: vertical stack, `w_full`, `gap` 10px, items aligned start or end
  by `MessageAlignment`. The inner row is `items_end` with `gap_2` (8px) and is
  reversed for `End`. The footer sits *outside* that row so the avatar stays
  flush with the bottom of the content, and is offset 40px when an avatar is present.
- `MessageAvatar`: `min_w_8` (32px), full radius, `self_end`, bg `muted`.
- `MessageHeader` / `MessageFooter`: `gap_1` (4px), `text_xs`, line height 1.25,
  medium weight, `muted_foreground`, inset `px_3` unless the content contains a
  ghost bubble (a ghost bubble has no surface, so the inset would double up).
- `MessageContent`: vertical stack, `gap` 10px, aligned start or end.
- `BubbleContent`: radius `radius_2xl` (20px), 1px transparent border,
  `px_3 py_2`, `text_sm`, line height 1.625. Variants:
  - `filled` = bg `primary`, fg `primary_foreground`
  - `secondary` = bg `muted`, fg `secondary_foreground`
  - `muted` = bg `muted`, fg `foreground`
  - `tinted` = `primary` mixed 24% toward `background` in dark, 12% in light
  - `outline` = bg `background`, border `border`, fg `foreground`
  - `ghost` = transparent, no border, no padding, no radius
  - `destructive` = `danger` at 20% in dark, 10% in light, fg `danger`
- `BubbleGroup`: a vertical stack of consecutive bubbles from one sender.

## Component map

| File | GPUI Kit counterpart |
| --- | --- |
| `button.svelte` | `button::Button` (`ButtonVariant`, `Size`, `ButtonRounded`, `outline`) |
| `input.svelte`, `textarea.svelte` | `input::Input`, `input::Textarea` |
| `badge.svelte`, `kbd.svelte`, `avatar.svelte`, `marker.svelte` | `badge::Badge`, `kbd::Kbd`, `avatar::Avatar`, `marker::Marker` |
| `alert.svelte`, `spinner.svelte`, `skeleton.svelte`, `shimmer.svelte`, `progress.svelte` | `alert::Alert`, `spinner::Spinner`, `skeleton::Skeleton`, `shimmer::ShimmerText`, `progress::{Progress,ProgressCircle}` |
| `switch.svelte`, `checkbox.svelte` | `switch::Switch`, `checkbox::Checkbox` |
| `tooltip.svelte`, `separator.svelte`, `empty.svelte` | `tooltip::Tooltip`, `separator::Separator`, `empty::Empty` |
| `tabs.svelte`, `accordion.svelte` | `tab::{Tabs,TabBar}`, `accordion::Accordion` |
| `dialog.svelte`, `sheet.svelte` | `dialog::Dialog`, `sheet::Sheet` via `window.open_dialog` / `open_sheet` |
| `data-table.svelte`, `list.svelte`, `card.svelte`, `description-list.svelte`, `status-bar.svelte` | `table::DataTable`, `list::List`, `group_box::GroupBox`, `description_list::DescriptionList`, `status_bar::StatusBar` |
| `message.svelte`, `bubble.svelte`, `message-scroller.svelte`, `attachment.svelte` | `message::{Message,MessageAvatar,MessageHeader,MessageContent,MessageFooter}`, `bubble::BubbleContent`, `message_scroller::MessageScroller`, `attachment::Attachment` |
| `artifacts/` | no upstream counterpart: inline agent artifacts are a product concept, not a gpui-kit component |
| `chat/` | no upstream counterpart: the chat surface this port exists to evaluate |

## Deviations from the crate

Recorded so a later reader can tell an intentional browser translation from drift:

- **Dialog and sheet** use the platform `<dialog>` element instead of a window
  overlay, because the browser already supplies what `FocusTrapElement` and
  `WindowExt` supply: modal focus trapping, inertness behind the surface, Escape
  dismissal, and focus restoration to the trigger.
- **Badge and avatar ladders** are sized for a browser page rather than a native
  window: badges are 16/20/24px where the crate's smallest is 10px, and avatars
  run 24/32/40/56px. `MessageAvatar` reserves 32px upstream, which is `md` here.
- **Alert and bubble tints** mix toward `--kit-background` rather than transparent
  white, because a translucent status surface disappears against the dark theme.
- **Reduced motion** stops the loops rather than shortening them: a spinner
  becomes a static ring and a shimmer a filled block, so the state stays legible.
- **Filled status buttons** take their label colour from `--kit-status-foreground`
  rather than the theme's `*_foreground` tokens. Upstream pairs the light dark-mode
  status fills (green-400, yellow-300, red-400, cyan-400) with their dark
  `*_foreground` values, which is correct for status *text* on the page surface but
  unreadable as a label on the fill itself. The status tokens are transcribed
  unchanged; only the label colour moved.

## Conventions

- Svelte 5 runes only (`$props`, `$state`, `$derived`, `$effect`). No legacy
  reactivity, no `on:click`, no slots.
- Tabs for indentation, single quotes, semicolons, trailing commas in
  multi-line literals — the repo's existing style.
- `cn` from `#lib/utils.js` for class merging.
- `data-kit="<component>"` marks a component root for the harness and for
  inspection; it is not a styling hook.
- Every interactive component is keyboard reachable and shows hover, focus,
  active, disabled and (where relevant) loading states distinctly.