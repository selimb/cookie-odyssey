# Cookie Odyssey

A personal travel journal application where Admins publish Journals and Entries, and authenticated Users read and comment.

## Language

**Journal**:
A named collection of travel Entries, identified by a URL Slug.
Has a start and end date and an optional cover image.
_Avoid_: Trip, album, book

**Entry**:
A record within a Journal representing a single event on a given day.
A Journal may have multiple Entries on the same day.
Has a title, date, time, address, body text, and attached Media.
Can be in Draft or published state.
_Avoid_: Post, article, log

**Day**:
A view of all Entries within a Journal on a given date.
A Day may contain multiple Entries.
_Avoid_: Date view, daily log, day page

**Media**:
An image or video attached to a Journal Entry, with an optional caption and display order.
Each Media item references one original File and one Thumbnail File.
_Avoid_: Attachment, asset, photo, image (when referring to the general concept)

**Draft**:
The unpublished state of an Entry.
Draft Entries are visible to Admins only.
Publishing an Entry is a one-way operation.
_Avoid_: Unpublished, hidden, private

**File**:
A record representing a stored object in Azure Blob Storage.
Holds the bucket, key, and dimensions (width/height).
Files are created before upload and referenced by Media items.
_Avoid_: Asset, object, blob, upload

**Thumbnail**:
A reduced-size preview image associated with a Media item.
Every Media item has exactly one Thumbnail, stored as a separate File.
_Avoid_: Preview, small image, resized image

**Transcode Task**:
A background task that converts an uploaded video File into a web-compatible format.
Created automatically when a video Media item is committed.
_Avoid_: Video job, encoding task, conversion

**Comment**:
A text note left by any authenticated User on a Journal, optionally associated with a specific date.
Comments are not attached to individual Entries.
_Avoid_: Note, reply, message

**User**:
A registered person who can view Journals, read Entries, and leave Comments.
A User must be approved by an Admin before they can log in.
_Avoid_: Member, account, viewer

**Admin**:
A User with elevated permissions.
Admins can create and edit Journals and Entries, upload Media, and manage other Users.
All other authenticated Users are read-only.
_Avoid_: Superuser, editor, owner

**Notification**:
A record that something new is available for readers, shared by all of its Recipients.
Currently the only kind is "Entry published".
Appears under the bell icon, and is marked read for a Recipient when they view the Day containing its Entry.
_Avoid_: Alert, update, message

**Pending Notification**:
A Notification that has not been flushed yet, so nobody has been told about it.
Created when an Entry is published.
_Avoid_: Queued notification, draft notification

**Recipient**:
A User who a Notification was delivered to, with their own read and email state.
Determined at Flush time: all approved Users, Admins included.
_Avoid_: Subscriber, target

**Flush**:
The Admin action that delivers every Pending Notification to all Recipients at once, and triggers at most one summary email per Recipient.
Cannot be undone.
_Avoid_: Send, publish, broadcast, announce

**Slug**:
A URL-safe string that uniquely identifies a Journal, used as its path parameter in page routes.
_Avoid_: ID, handle, identifier, permalink

**Toast**:
A transient UI notification shown to the user after an action, with a success or error variant.
Delivered via the `HX-Trigger` response header.
_Avoid_: Alert, snackbar, banner
