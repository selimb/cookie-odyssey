# Media Editor and New-Entry Media Upload

## Problem Statement

As an Admin, attaching Media to a new Entry is a two-step slog.
The new-entry page has no way to upload Media at all, so I have to create the Entry first (with no Media), land on the edit page, and only then start adding photos and videos.
On top of that, the Media controls on the edit page feel clunky: reordering is up/down buttons, captions are tiny per-item forms, and every change is a server round-trip that re-renders the whole list.

I want to attach Media at the moment I create an Entry, without committing to an Entry I might back out of, and without waiting on slow video uploads before I can keep working.

## Solution

A single Media editor, used on both the new-entry page and the edit page, that owns the Media list on the client.

On the new-entry page I can add Media while I fill in the Entry's fields.
Each file starts uploading in the background immediately, showing a thumbnail right away.
No Entry exists yet, so if I change my mind I just navigate away and nothing is created.
When I press Create, the Entry and its Media are created together; if an upload is still in flight, Create waits for it to finish before submitting.

On the edit page the same editor manages the Entry's existing Media.
It behaves identically to the new-entry page: changes are staged in the in-memory list, and pressing Save persists the prose fields and the Media list together in one step.
The server reconciles the submitted list against what it already has.

Save is the single commit point for an Entry: it governs the prose fields (title, location, date, time, body) and the Media list together.
This is a deliberate change from the first cut of this feature, which auto-persisted each Media change on the edit page (see ADR 0007): one uniform stage-until-Save path is simpler and avoids the client's view of the Media drifting out of sync with the server.
Save is an explicit button, never an autosave, so nothing half-written is published without intent.
Because Media is no longer auto-saved, leaving the page with unsaved edits (or uploads still in flight) prompts a browser "leave site?" warning.

## User Stories

1. As an Admin, I want to add Media on the new-entry page, so that I can attach photos and videos at the moment I create an Entry instead of in a separate second step.
2. As an Admin, I want each file I add to start uploading in the background immediately, so that I am not blocked while a slow video uploads.
3. As an Admin, I want a thumbnail to appear as soon as I add a file, so that I get immediate confirmation the file was accepted.
4. As an Admin, I want a visible pending indicator on a thumbnail while its upload is in flight, so that I know which uploads have not finished yet.
5. As an Admin, I want to add multiple files at once, so that I can attach a batch of Media in one action.
6. As an Admin, I want to add both images and videos, so that all my Media types are supported.
7. As an Admin, I want to remove a file I added by mistake before creating the Entry, so that it is not attached to the Entry.
8. As an Admin, I want the Media to keep the order I added them in on the new-entry page, so that the ordering is predictable when I land on the edit page.
9. As an Admin, I want to press Create while an upload is still in progress and have the submit wait for the upload to finish, so that I do not have to babysit the page.
10. As an Admin, I want Create to show a spinner while it waits on in-flight uploads, so that I understand why the submit is delayed.
11. As an Admin, I want to be told when an upload fails and have Create not proceed, so that I never create an Entry that is silently missing Media I thought I added.
12. As an Admin, I want to navigate away from the new-entry page without creating anything, so that a mis-started Entry leaves no trace.
13. As an Admin, I want the Entry and its Media to be created together when I press Create, so that I never end up with a half-created Entry.
14. As an Admin, I want to be redirected to the Entry's edit page after Create, so that I can immediately fine-tune the Entry.
15. As an Admin, I want the new-entry date to be pre-filled when one is provided, so that I do not have to retype a date I already implied.
16. As an Admin, I want to add Media on the edit page, so that I can attach more Media to an existing Entry.
17. As an Admin, I want to edit a Media item's caption on the edit page, so that I can label my Media.
18. As an Admin, I want to reorder Media on the edit page with up/down controls, so that the gallery reads the way I want.
19. As an Admin, I want to delete a Media item on the edit page, so that unwanted Media can be removed.
20. As an Admin, I want pressing Save on the edit page to persist my prose and my Media changes together, so that there is one obvious commit point for the Entry.
21. As an Admin, I want my in-progress prose and Media to never be auto-published, so that typos and half-finished edits stay private until I choose to Save.
22. As an Admin, I want a warning if I try to leave the page with unsaved edits or uploads still in flight, so that I do not lose work by navigating away.
23. As an Admin, I want the new-entry and edit Media editors to look and behave the same, so that I do not have to relearn the interaction between the two pages.
24. As an Admin, I want the editor to show lightweight thumbnails rather than full-size Media, so that the editing screen stays fast even on mobile LTE.
25. As an Admin, I want the editor to work on my phone, so that I can manage Media from a mobile device.
26. As an Admin, I want a clear error message if requesting upload URLs or uploading to storage fails, so that I know an action did not succeed.
27. As a regular User, I want Media editing to remain unavailable to me, so that only Admins can change Journal content.
28. As an Admin, I want orphaned uploads from abandoned new-entry attempts to be cleaned up automatically, so that backing out does not litter storage.

## Implementation Decisions

### Architecture (see ADR 0007)

- A single client-owned Stimulus Media editor controller replaces the current `media--form` controller and is used identically on both the new-entry and edit pages.
- The controller owns an in-memory ordered list of Media items and renders them itself, rather than the server re-rendering a fragment after each mutation.
- The controller is mode-less. It is initialized with `href_upload_url` and an initial items array, lives inside the page's form, and serializes its list into a hidden `media_items` field.

### Persistence: stage until submit

- The editor never mutates the server per change. Adding a file uploads its blob to storage in the background (minting a `file_id`); reorder, caption, and delete only touch the in-memory list.
- The form submit carries the serialized list:
  - **New-entry page:** Create submits the Entry's fields plus the Media list; the server creates the `Entry` and its `JournalEntryMedia` rows in one transaction.
  - **Edit page:** Save submits the prose fields plus the Media list; the server updates the prose and reconciles the Media list in the same transaction.
- The reconcile is keyed on `file_id_original` (unique per upload): rows absent from the submitted list are deleted, surviving rows have their order and caption updated, and new file ids are inserted. Keying on `file_id_original` makes re-saving idempotent, so the client carries no `JournalEntryMedia` id.

### Backend

- There are **no** `commit` / `caption` / `reorder` / `delete` endpoints. The only Media endpoint is `/api/media-upload-url` (unchanged): it mints `File` records and pre-signed SAS URLs, and the original + Thumbnail are uploaded directly to storage by the client (the server is never in the upload path).
- A single `sync_journal_entry_media` query performs the reconcile and is shared by both the new-entry create handler and the edit Save handler. It is generic over the connection so it runs inside each handler's transaction.
- The edit page handler passes the Entry's existing Media to the page as an initial items array so the controller can render from it.
- A `// SYNC` type (`MediaEditorItem`) defines the Media item shape shared between Rust and TypeScript.

### Rendering

- The editor renders Thumbnails (a simple image from the Thumbnail `File`), not full Media, so the `common/media.html` macros do not need to be reimplemented in JS.
- The published Day / gallery view stays server-rendered and is untouched.

### Submit-while-uploading (wait-then-submit)

- A file's `file_id` is minted by `/api/media-upload-url` before its blob finishes uploading, so the form submit must wait for in-flight uploads to actually land, not merely for ids to exist.
- The wait is implemented with HTMX's `htmx:confirm` event, which fires before the request and exposes `issueRequest()`:
  - If uploads are still settling, `preventDefault()`, show the submit spinner, await the in-flight upload batches, then release the original submit via `issueRequest()`.
  - If any upload failed, do not issue the request; surface a Toast and let the Admin remove the failed item and retry.
- The listener is attached to the form on both pages. It only gates the form's own submit (not, e.g., the edit page's Publish button).

### Unsaved-changes guard

- Because Media is no longer auto-saved, a `beforeunload` listener warns before leaving the page with unsaved edits or in-flight uploads. It fires on real browser unloads (tab close, refresh, external navigation); HTMX-boosted in-app navigation does not trigger it.

### Reorder

- Reorder stays as up/down controls for now. Pointer-based drag reorder is deferred, and the component leaves a seam to drop it in later.

### Orphan cleanup

- Background uploads that are never committed (e.g. a new-entry attempt that is abandoned) leave orphaned `File` rows. This is an accepted state; the existing `StorageCleanup` job already deletes `File` rows not referenced by any `JournalEntryMedia` along with their blobs.

## Out of Scope

- **Location / GPS.** Pre-filling an Entry's address or coordinates from Media metadata is deferred -- there is no field wired up to store it yet, and it will be its own feature.
- **Drag-and-drop to create an Entry.** Dropping Media anywhere in a Journal and landing on a pre-filled new-entry page is future work that builds on this editor. It is not included here.
- **Pointer-based drag reorder.** Up/down controls only for now.
- **EXIF / container metadata parsing in production.** Not needed; see Further Notes.
- **Full caption/reorder of full-size Media in the editor.** The editor works with Thumbnails; the published gallery rendering is unchanged.
- **Non-Admin access.** All Media and Entry mutation routes remain Admin-only at the router level; this feature adds no new permission levels.

## Further Notes

- **Entry date source.** When this editor is later fed by drag-and-drop, the new Entry's date will come from `file.lastModified` (the earliest across the dropped files), not from EXIF. A diagnostic on the thumbnail demo page confirmed `file.lastModified` is accurate for the iPhone file picker. The macOS "drag straight out of Apple Photos" path -- the primary drag-and-drop case -- still needs a spot check; if it reports export-time instead of capture-time, the JPEG EXIF fallback (already written as a demo-only probe) would be revisited.
- **Demo-only parser.** A dependency-free metadata probe (JPEG EXIF `DateTimeOriginal`, ISO BMFF `mvhd` creation time) lives on the thumbnail demo page purely as a diagnostic. Production code does not import it.
- **HTMX-first deviation.** This feature deliberately moves Media reorder/caption/delete from declarative HTMX into a JS-heavy client component, a conscious deviation from ADR 0001 that is recorded in ADR 0007. Owning a stage-until-submit in-memory list with no Entry to render against on new-entry is the rationale.
