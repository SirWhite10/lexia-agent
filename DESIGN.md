<!-- Initial Lexia baseline. Once spec.mdx, spec-config.ts, and spec:gen exist, edit those sources and regenerate this file. -->
---
version: alpha
name: Lexia
description: A dark-first, friendly technical interface for a powerful server-side AI agent harness.
colors:
  primary: "oklch(0.7850 0.1281 10.75)"
  primary-hex: "#FF95A5"
  secondary: "oklch(0.4822 0.1061 266.19)"
  secondary-hex: "#425B9A"
  accent: "oklch(0.7760 0.0971 236.01)"
  accent-hex: "#76C0EC"
  light-surface: "oklch(0.9734 0.0353 91.12)"
  light-surface-hex: "#FFF6DC"
  dark-background: "#090909"
  dark-foreground: "#FFFFFF"
  light-background: "oklch(0.985 0 0)"
  light-foreground: "#090909"
  dark-card: "oklch(0.21 0.006 285.885)"
  dark-muted: "oklch(0.274 0.006 286.033)"
  dark-muted-foreground: "oklch(0.705 0.015 286.067)"
  light-muted: "oklch(0.967 0.001 286.375)"
  light-muted-foreground: "oklch(0.552 0.016 285.938)"
  light-ring: "oklch(0.9734 0.0353 91.12)"
  dark-border: "oklch(1 0 0 / 10%)"
  dark-input: "oklch(1 0 0 / 15%)"
  destructive: "oklch(0.704 0.191 22.216)"
typography:
  display:
    fontFamily: Quicksand
    fontSize: 3rem
    fontWeight: 700
    lineHeight: 1.05
    letterSpacing: -0.03em
  headline-lg:
    fontFamily: Quicksand
    fontSize: 2rem
    fontWeight: 700
    lineHeight: 1.15
    letterSpacing: -0.02em
  headline-md:
    fontFamily: Quicksand
    fontSize: 1.5rem
    fontWeight: 650
    lineHeight: 1.2
    letterSpacing: -0.015em
  body-lg:
    fontFamily: Quicksand
    fontSize: 1.125rem
    fontWeight: 450
    lineHeight: 1.55
  body-md:
    fontFamily: Quicksand
    fontSize: 1rem
    fontWeight: 450
    lineHeight: 1.55
  body-sm:
    fontFamily: Quicksand
    fontSize: 0.875rem
    fontWeight: 450
    lineHeight: 1.45
  label-md:
    fontFamily: Quicksand
    fontSize: 0.875rem
    fontWeight: 650
    lineHeight: 1.2
    letterSpacing: 0.01em
  label-sm:
    fontFamily: Quicksand
    fontSize: 0.75rem
    fontWeight: 650
    lineHeight: 1.2
    letterSpacing: 0.04em
  code-md:
    fontFamily: IA Writer Duo
    fontSize: 0.875rem
    fontWeight: 400
    lineHeight: 1.6
rounded:
  none: 0px
  sm: 0.25rem
  md: 0.45rem
  lg: 0.75rem
  full: 9999px
spacing:
  xs: 0.25rem
  sm: 0.5rem
  md: 1rem
  lg: 1.5rem
  xl: 2rem
  2xl: 3rem
  gutter: 1.5rem
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.light-foreground}"
    rounded: "{rounded.md}"
    padding: "{spacing.sm} {spacing.md}"
  button-secondary:
    backgroundColor: "{colors.secondary}"
    textColor: "{colors.dark-foreground}"
    rounded: "{rounded.md}"
    padding: "{spacing.sm} {spacing.md}"
  button-accent:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.light-foreground}"
    rounded: "{rounded.md}"
    padding: "{spacing.sm} {spacing.md}"
  focus-ring:
    backgroundColor: "{colors.light-ring}"
    rounded: "{rounded.sm}"
  code-block:
    backgroundColor: "{colors.dark-card}"
    textColor: "{colors.dark-foreground}"
    typography: "{typography.code-md}"
    rounded: "{rounded.md}"
---

# Lexia Design System

## Overview

Lexia is dark-first, warm, friendly, and technical. The interface should feel capable without feeling severe: Quicksand supplies approachable human shapes, while IA Writer Duo gives code, model output, and operational data a precise workspace voice.

The default scheme is dark. Light mode is an explicit alternative, not the source of truth. Use the pink primary sparingly for the most important action or state on a surface; blue-violet secondary and sky-blue accent provide structure, navigation, information, and supporting actions.

All color calculations and derived shades should use OKLCH where supported. Preserve the listed hex values as brand anchors when exporting or interoperating with tools that do not support OKLCH.

## Colors

- **Primary:** `#FF95A5` / `oklch(0.7850 0.1281 10.75)`. The main Lexia action and highest-priority emphasis.
- **Secondary:** `#425B9A` / `oklch(0.4822 0.1061 266.19)`. Navigation, secondary actions, and trusted structural emphasis.
- **Accent / information:** `#76C0EC` / `oklch(0.7760 0.0971 236.01)`. Informational states, active context, and supporting highlights.
- **Light surface:** `#FFF6DC` / `oklch(0.9734 0.0353 91.12)`. Warm light-mode surface and focus-ring anchor.
- **Dark background:** `#090909`. The default application canvas.
- **Dark text:** `#FFFFFF`. Primary text in dark mode.
- **Light text:** `#090909`. Primary text in light mode.

Derived tints and shades are allowed and should remain in OKLCH. Semantic status colors may be derived from the palette but must preserve readable contrast and must not compete with the primary action.

The shadcn-svelte semantic variables should map to these roles. `primary`, `accent`, `chart-*`, and `sidebar-ring` are not copied from the starter palette: primary uses Lexia pink, accent uses Lexia sky blue, charts use the Lexia palette, and the ring uses the warm light-surface color.

## Typography

Quicksand is the primary interface and narrative family, provided by `@fontsource-variable/quicksand`. Use it for headings, labels, body copy, controls, and status text.

IA Writer Duo is the monospace family, provided by `@fontsource/ia-writer-duo`. Use it for code, tool input/output, model output where exact formatting matters, identifiers, timestamps, and operational metadata.

Use no more than two adjacent weights in one component. Headings may use 650–700; body copy uses 450; monospace content uses 400 unless emphasis is necessary.

## Layout

Use a compact, readable workspace layout with a 4px base rhythm and generous separation between major agent surfaces. Preserve clear containment between conversation, tool activity, workflows, and approval controls. Prefer responsive grids and fluid widths over fixed desktop assumptions so the same server-side application can serve web, desktop, and mobile clients.

## Elevation & Depth

Use soft shadows and tonal surfaces rather than heavy borders or dramatic glass effects. In dark mode, separate cards and panels with the shadcn tonal layers and restrained borders; in light mode, use the warm surface and soft shadow to distinguish content from the neutral background.

Streaming, tool execution, and approval states should be communicated with color, motion, and a concise status label. Respect reduced-motion preferences and never make motion the only state signal.

## Shapes

The default radius is `0.45rem`, matching the shadcn-svelte starting point. Use small radii for controls, the default radius for cards and inputs, larger radii only for deliberate grouping, and full rounding for pills, badges, and status indicators. Do not mix sharp and highly rounded visual languages in one surface.

## Components

Build the component system on shadcn-svelte primitives in `src/lib/components/ui/`. Lexia-specific components belong in `src/lib/components/ai/` and compose those primitives rather than copying React component APIs.

The initial AI component vocabulary includes conversation messages, composer/input, streaming response, code block, tool call, tool result, reasoning disclosure, workflow progress, model/tier badge, approval prompt, error state, and release/update status.

Each interactive component must define default, hover, focus, pressed, disabled, loading, success, and error behavior where applicable. Focus rings use the warm light-surface token and must remain visible in both themes.

## Do's and Don'ts

- Do use the pink primary for the single most important action or emphasis in a view.
- Do use OKLCH for derived colors and test contrast in both themes.
- Do keep model output and operational data visually distinct through IA Writer Duo and restrained surfaces.
- Do preserve visible keyboard focus and provide text or icons in addition to color for state.
- Don't use white text on the pink primary; use the dark foreground for readable contrast.
- Don't let charts, badges, or secondary controls compete with the primary action.
- Don't place model keys, tool execution, or agent orchestration in client code.
- Don't edit this generated artifact directly after the design-source generator is established.
