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

Media editing becomes a single client-owned Stimulus component used identically on both the new-entry and edit pages.
It owns an in-memory ordered list of Media items and renders them itself (as thumbnails), instead of the server re-rendering a fragment after each mutation.

The component is **mode-less and stages everything until submit.**
Adding a file uploads its blob to storage in the background (minting a `file_id`), but reorder, caption, and delete only mutate the in-memory list.
That list is serialized into a hidden form field, so the page's normal form submit carries it:

- **New-entry page:** Create submits the Entry's fields plus the Media list; the server creates the Entry and its Media in one transaction.
- **Edit page:** Save submits the prose fields plus the Media list; the server updates the prose and **reconciles** the Media list against the existing rows in one transaction.

The reconcile is keyed on `file_id_original` (unique per upload): rows absent from the submitted list are deleted, surviving rows have their order and caption updated, and new file ids are inserted.
Keying on `file_id_original` makes re-saving idempotent without the client tracking row ids, so the client carries no `JournalEntryMedia` id at all.

Because nothing persists per-mutation, there are **no** `commit` / `caption` / `reorder` / `delete` endpoints -- only `/api/media-upload-url` (unchanged) plus the two existing form-submit handlers.
This means Media now commits on Save alongside prose, rather than auto-persisting on the edit page.

Two cross-cutting concerns ride on the form submit:

- A file's `file_id` is minted before its blob finishes uploading, so submit must wait for in-flight uploads to land.
  This is gated with HTMX's `htmx:confirm` event (on both forms now): if uploads are still settling, preventDefault, await them, then `issueRequest()` (or abort and toast on failure).
- Because Media is no longer auto-saved, a `beforeunload` guard warns before discarding unsaved edits or in-flight uploads.

This still deviates from the HTMX-first stance in [0001](0001-htmx.md): reorder, caption, and delete are JS-driven, in-memory operations rather than declarative HTMX round-trips.

## Considered Options

- **Per-mutation auto-persist on the edit page** (the original plan for this ADR): with an `entryId`, every add/caption/reorder/delete fires its own JSON endpoint immediately; without one, the new-entry page stages in memory.
  Rejected after a first implementation: the two persistence modes diverged, the new-entry page has no server state to be authoritative against, and the optimistic edit-page client never reconciled -- a lost or failed mutation response (realistic on mobile LTE) silently desynced the displayed order from the database, because reorder swapped rows by a client-sent `order` index.
  Staging-until-submit collapses both pages to one path and removes the drift entirely.
- **Lazily create a draft Entry on the new-entry page**, then reuse the whole edit-page machinery.
  Rejected: it leaves stray draft Entries when you back out, and "back out cleanly" was a hard requirement.
  (Orphaned `File` rows from background uploads are fine -- `StorageCleanup` already sweeps those.)
- **A minimal new-entry preview** (thumbnail + remove only), leaving the edit page on HTMX.
  Rejected: it doesn't unify the two pages, and it still needs its own pre-Entry handling anyway.

## Consequences

- Media on the edit page no longer takes effect the instant you touch it; it commits when you Save, together with the prose.
  This reverses the auto-persist intent in the original PRD, traded for one uniform, drift-free path.
  Save is an explicit button, not an autosave, so nothing half-written gets published without intent -- and "back out cleanly leaves only orphaned blobs" now holds on the edit page too, not just new-entry.
- More JS, and the Media list is client-rendered with no server-side fallback -- the unavoidable price of having no Entry to render against on new-entry.
- The editor renders thumbnails, not full Media, so we don't have to port the `common/media.html` macros into JS.
  The published day/gallery view stays server-rendered and untouched.
- The `beforeunload` guard only fires on real browser unloads (tab close, refresh, external navigation); HTMX-boosted in-app navigation doesn't trigger it.
- Reorder stays as up/down buttons for now; pointer-based drag is deferred, and the component leaves a seam for it.
- Drag-and-drop-to-create-an-Entry (drop Media anywhere in a Journal and land on a pre-filled new-entry page) is future work that builds directly on this component.
