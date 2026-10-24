use crate::{
    journal::routes::{JournalDayGetPath, JournalGetawayGetPath},
    Route,
};

/// Returns the current page of an Entry, scrolled to the Entry.
/// That is the Getaway page when the Entry has a Getaway, and the Day page
/// otherwise.
/// Shared by [entry-link] and by pages that list Entries, which build the link
/// at render time and so need no stable link.
pub fn entry_page_href(
    journal_slug: &str,
    date: chrono::NaiveDate,
    getaway_id: Option<i32>,
    entry_id: i32,
) -> String {
    let path = match getaway_id {
        Some(getaway_id) => Route::JournalGetawayGet(Some(&JournalGetawayGetPath {
            slug: journal_slug.to_string(),
            getaway_id,
        }))
        .as_path(),
        None => Route::JournalDayGet(Some(&JournalDayGetPath {
            slug: journal_slug.to_string(),
            date,
        }))
        .as_path(),
    };
    // [entry-fragment] Each page that lists Entries renders a matching id.
    format!("{path}#entry-{entry_id}")
}
