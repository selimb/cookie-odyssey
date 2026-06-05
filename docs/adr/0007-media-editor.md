# Media Editor

## Context

Two problems pushed on the same spot.
The new-entry page had no way to attach Media at all -- you had to create an Entry first, then add Media on the edit page.
And the edit page's Media management was a pile of HTMX round-trips: up/down reorder buttons, a tiny form per caption, each one re-rendering the `fragment_media_list`.
I wanted Media upload on the new-entry page, and I wanted both pages to behave the same.

The catch: by design, the new-entry page has no `entry_id` until you hit Create, so you can fat-finger your way in and just back out with nothing created.
But the existing commit flow attaches Media to an `entry_id`.
So "add Media before the Entry exists" is a real chicken-and-egg, and it's the thing that has to be cracked first.

## Decision

Media editing becomes a single client-owned Stimulus component used on both the new-entry and edit pages.
It owns an in-memory ordered list of Media items and renders them itself (as thumbnails), instead of the server re-rendering a fragment after each mutation.

It is initialized with the usual `href_*` URLs and an optional `entryId`, and that one value selects the persistence mode:

- **With an `entryId` (edit):** every mutation -- add, caption, reorder, delete -- fires its server call immediately.
  Media stays instantly persisted, exactly as before.
- **Without an `entryId` (new-entry):** mutations only touch the in-memory list, which is serialized into a hidden form field so the normal HTMX form submit carries it.
  The server creates the Entry and its Media in one transaction on Create.

Because the client now owns the DOM, the mutation endpoints (`commit`, `caption`, `reorder`, `delete`) return JSON, not an HTML fragment.
`/api/media-upload-url` is unchanged.

This deliberately deviates from the HTMX-first stance in [0001](0001-htmx.md): reorder, caption, and delete move from declarative HTMX into JS.
The deviation follows a deliberate boundary -- auto-save is fine for Media (uploading, reordering, deleting), but the prose fields (title, address, body, ...) keep an explicit Save, because I don't want half-written text published by an autosave.
So the page's Save button still governs only the text fields; Media is independent of it.

## Considered Options

- **Lazily create a draft Entry on the new-entry page**, then reuse the whole edit-page machinery.
  Rejected: it leaves stray draft Entries when you back out, and "back out cleanly" was a hard requirement.
  (Orphaned `File` rows from background uploads are fine -- `StorageCleanup` already sweeps those.)
- **A minimal new-entry preview** (thumbnail + remove only), leaving the edit page on HTMX.
  Rejected: it doesn't unify the two pages, and it still needs its own pre-Entry handling anyway.
- **Stage all Media edits until Save**, like the text fields.
  Rejected: I want Media changes to persist instantly when an Entry exists; only prose needs the explicit-Save gate.

## Consequences

- More JS, and the new-entry Media list is client-rendered with no server-side fallback -- the unavoidable price of having no Entry to render against.
- The editor renders thumbnails, not full Media, so we don't have to port the `common/media.html` macros into JS.
  The published day/gallery view stays server-rendered and untouched.
- On the new-entry page, a file's `file_id` is minted before its blob finishes uploading, so Create must wait for in-flight uploads to land.
  This is gated with HTMX's `htmx:confirm` event: if uploads are still settling, preventDefault, await them, then `issueRequest()` (or abort and toast on failure).
- Reorder stays as up/down buttons for now; pointer-based drag is deferred, and the component leaves a seam for it.
- Drag-and-drop-to-create-an-Entry (drop Media anywhere in a Journal and land on a pre-filled new-entry page) is future work that builds directly on this component.
