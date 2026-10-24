# Getaways group Entries explicitly

A Getaway is a named group that Entries belong to, not a date range on the Scrapbook.
It holds nothing but a name.
Its dates are derived from its Entries, and in a Scrapbook a Day only shows Loose Entries.

## Considered Options

A date range ("Feb 6 - Feb 8 is Quebec City") would have been simpler for the data model: no new column on Entries, and Days would keep working as they do.
I rejected it for three reasons:

- The main one is UI.
  A Getaway with dates needs its own screens: create, edit dates, delete, and a way to find them.
  A Getaway that is only a name needs none.
  It is created by typing its name on an Entry, deleted automatically when its last Entry leaves, and renamed from a popover on its own page.
- A Loose Entry on a Getaway date (e.g. something back home on the Saturday) cannot be expressed.
- The stored range and the actual Entries can disagree, e.g. an Entry dated one day after the Getaway ends.
  This would need validation, and overlapping ranges would need rules too.

## Consequences

Anything that used to locate an Entry by Journal and date can now be wrong for Entries in a Getaway:

- Marking Notifications as read must match Loose Entries on a Day page, and the Getaway's Entries on a Getaway page.
- Comments on a Getaway belong to the Getaway, not to its dates.
- Links stored outside the app (emails) point to the Entry and are resolved when followed, since an Entry can move into or out of a Getaway after the link was sent.

What a name-only Getaway costs, compared to a date range:

- Grouping is manual and per Entry.
  Putting a week of existing Entries into a Getaway means editing each one, where a date range would claim them all at once.
  There is no bulk move.
- Typos create a second Getaway.
  The Entry form suggests existing names and uniqueness is case-insensitive, but "Quebec Cty" is still a new Getaway.
- A Getaway cannot exist before its first Entry.
  There is nowhere to plan a trip, and no room for a description or cover photo without adding fields later.
- Dates move with the Entries.
  Editing an Entry's date changes the Getaway's range and its position in the Scrapbook.
- Uniqueness is case-insensitive only for ASCII letters (SQLite `NOCASE`), so names that differ in the case of an accented letter are treated as different.
- Entries and Comments both gained a Getaway reference, so every query that grouped by date had to be revisited (see above).
