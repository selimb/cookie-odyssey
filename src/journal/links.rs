use crate::{journal::routes::JournalDayGetPath, Route};

/// The current page of an Entry, scrolled to the Entry.
/// Shared by [entry-link] and by pages that list Entries, which build the link
/// at render time and so need no stable link.
pub fn entry_page_href(slug: &str, date: chrono::NaiveDate, entry_id: i32) -> String {
    let path = Route::JournalDayGet(Some(&JournalDayGetPath {
        slug: slug.to_string(),
        date,
    }))
    .as_path();
    // [entry-fragment] Each page that lists Entries renders a matching id.
    format!("{path}#entry-{entry_id}")
}
