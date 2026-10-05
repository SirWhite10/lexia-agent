---
kind: implementation
status: planned
---

# WF-IMP-011: Composer context model in gpui-kit, and the voice affordance

## Goal

Port the composer's context machinery — trigger tokens, inline badges, the context tray and document-level undo — into gpui-kit, and place the voice control the composer has been leaving room for.

## Depends on

- [WF-IMP-009](WF-IMP-009-port-chat-surface-to-gpui-kit.md)
- The `origin/lexosa` branch, which already carries a chat composer and speech providers

## Context model

The contract is in `src/lib/components/kit/chat/composer-model.ts` and is deliberately small:

- **The input is the source of truth.** Context items live in a catalog keyed by their token, and every view is derived by re-scanning the text. Deleting a character inside a token removes the item; pasting re-derives it. No stored caret ranges, so nothing can point at text that no longer exists.
- **Tokens are one word.** `/` tools, `$` skills, `#` files, `^` memory, `@` agents, and bare `https://…` as a link. A token counts only at the start of the text or after whitespace.
- **An unresolved token is plain text.** It drives the typeahead. Choosing a suggestion registers it, and the same text then renders as a badge.
- **Removal deletes the token's own range**, plus enough whitespace that the surrounding words do not fuse, and the caret returns to where the token started.
- **Undo is document-level** over text plus catalog, with consecutive typing coalesced into one step and structural changes kept separate. It covers item removal and text edits, because the browser's own undo would restore text without the catalog and leave a badge's text as plain text.

## The hard part, and it is not the parser

The reference implementation draws inline badges by mirroring the text: a transparent real textarea holds the value, caret and selection, and a layer above it paints the glyphs and chips in the same positions. That is a browser technique. In gpui-kit the field is a real input element with its own layout, so the inline badges need a construction of their own. Investigate before building:

- whether gpui-kit's `Input` supports inline decorated ranges or a child element layer;
- otherwise, a composed `RenderOnce` element: the input plus a badge strip above it, which is the tray, with the inline representation moving to that strip.

That second option is a real behaviour change and must be decided deliberately, not discovered late. It affects the "input is the source of truth" rule: the tokens still live in the text, they are just drawn in a strip rather than inside the field.

## Voice

- Add the microphone to the right of the field, where the layout already reserves the position and where Send currently sits alone.
- Wire it to the speech providers already present on `origin/lexosa`. Do not add a second transcription path.
- Behaviour: press to dictate, press to stop; the transcript enters the composer as ordinary text, so it composes with context tokens and undo like everything else; a failure says what failed and leaves the text intact.
- Voice is alpha on web, matching [WF-DEC-003](WF-DEC-003-gpui-kit-forefront-web-alpha.md).

## Out of scope

- Removing the Svelte composer.
- New context kinds beyond the five triggers and links.
- Wake words, or any behaviour that requires the app to listen without being asked.

## Acceptance criteria

- Typing a trigger and choosing a suggestion produces a badge and a tray entry; removing either removes it from both; undo and redo restore both.
- Removing a token never fuses the surrounding words, and the caret lands where the token was.
- The tray and the inline representation can never disagree, because they read one parse.
- Voice dictation inserts text the user can then edit, extend with context tokens, and undo.
- The microphone is reachable by keyboard and states its state without relying on colour.

## Progress

Not started.