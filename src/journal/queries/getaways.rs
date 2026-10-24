use std::collections::HashMap;

use chrono::Datelike as _;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
};
use serde::Serialize;

use crate::FormError;
use entities::{prelude::*, *};

/// [entry-getaway-resolve] Resolves the Getaway typed on an Entry form.
/// A new name creates a Getaway, an existing one joins it, and an empty one
/// means the Entry is loose.
/// Names match ignoring case, see [getaway-name-nocase], and surrounding
/// spaces, which `name` is expected to be already trimmed of.
/// The lookup uses [getaway-name-index].
pub async fn resolve_entry_getaway(
    db: &impl ConnectionTrait,
    journal: &journal::Model,
    name: &str,
) -> Result<Result<Option<i32>, FormError>, DbErr> {
    if name.is_empty() {
        return Ok(Ok(None));
    }
    // [journal-no-getaway] Only a Scrapbook has Getaways, so in a Journal every
    // Entry is loose.
    if !journal.scrapbook {
        return Ok(Err(FormError::new("Only a Scrapbook can have Getaways")));
    }
    let existing = JournalGetaway::find()
        .filter(journal_getaway::Column::JournalId.eq(journal.id))
        .filter(journal_getaway::Column::Name.eq(name))
        .one(db)
        .await?;
    let id = match existing {
        Some(getaway) => getaway.id,
        None => {
            let data = journal_getaway::ActiveModel {
                journal_id: sea_orm::ActiveValue::Set(journal.id),
                name: sea_orm::ActiveValue::Set(name.to_string()),
                ..Default::default()
            };
            JournalGetaway::insert(data).exec(db).await?.last_insert_id
        }
    };
    Ok(Ok(Some(id)))
}

/// Deletes the Getaway if it has no Entries left, Drafts included.
/// Its Comments are deleted by the database, see [getaway-comment-cascade].
pub async fn delete_getaway_if_empty(
    db: &impl ConnectionTrait,
    getaway_id: i32,
) -> Result<(), DbErr> {
    let remaining = JournalEntry::find()
        .filter(journal_entry::Column::GetawayId.eq(getaway_id))
        .count(db)
        .await?;
    if remaining == 0 {
        JournalGetaway::delete_by_id(getaway_id).exec(db).await?;
    }
    Ok(())
}

/// A Getaway suggested on an Entry form.
#[derive(Serialize, Debug)]
pub struct GetawayOption {
    pub name: String,
    /// The dates of all of the Getaway's Entries, Drafts included.
    pub dates: String,
}

/// Formats the dates of a Getaway as a label, e.g. "Feb 6 - Feb 8".
/// The year is left out only when both dates are in the year of `today`.
///
/// [getaway-option-dates] This is formatted on the server, unlike other dates,
/// because it is the text of an `<option>`, which cannot hold the `<time>`
/// element that the `datetime` macros render.
/// See [date-as-datetime] for the usual way.
fn format_date_range(
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
    today: chrono::NaiveDate,
) -> String {
    let with_year = start.year() != today.year() || end.year() != today.year();
    let format = |date: chrono::NaiveDate, with_year: bool| {
        if with_year {
            date.format("%b %-d, %Y").to_string()
        } else {
            date.format("%b %-d").to_string()
        }
    };
    if start == end {
        format(start, with_year)
    } else if start.year() == end.year() {
        // A shared year is only written once, at the end.
        format!("{} - {}", format(start, false), format(end, with_year))
    } else {
        format!("{} - {}", format(start, true), format(end, true))
    }
}

/// Lists the Scrapbook's Getaways, latest first.
pub async fn query_getaway_options(
    db: &impl ConnectionTrait,
    journal_id: i32,
) -> Result<Vec<GetawayOption>, DbErr> {
    let getaways = JournalGetaway::find()
        .filter(journal_getaway::Column::JournalId.eq(journal_id))
        .all(db)
        .await?;
    let rows = JournalEntry::find()
        .select_only()
        .column(journal_entry::Column::GetawayId)
        .column(journal_entry::Column::Date)
        .filter(journal_entry::Column::JournalId.eq(journal_id))
        .filter(journal_entry::Column::GetawayId.is_not_null())
        .order_by_asc(journal_entry::Column::Date)
        .into_tuple::<(i32, chrono::NaiveDate)>()
        .all(db)
        .await?;
    let mut ranges: HashMap<i32, (chrono::NaiveDate, chrono::NaiveDate)> = HashMap::new();
    for (getaway_id, date) in rows {
        // Rows are in date order, so the first date seen is the start.
        let range = ranges.entry(getaway_id).or_insert((date, date));
        range.1 = date;
    }

    let today = chrono::Utc::now().date_naive();
    let mut options: Vec<(chrono::NaiveDate, GetawayOption)> = getaways
        .into_iter()
        .filter_map(|g| {
            let (start, end) = ranges.get(&g.id)?;
            let dates = format_date_range(*start, *end, today);
            Some((
                *end,
                GetawayOption {
                    name: g.name,
                    dates,
                },
            ))
        })
        .collect();
    options.sort_by_key(|(end, _)| std::cmp::Reverse(*end));
    Ok(options.into_iter().map(|(_, o)| o).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn format_date_range_current_year() {
        let today = date(2026, 10, 24);
        assert_eq!(
            format_date_range(date(2026, 2, 6), date(2026, 2, 6), today),
            "Feb 6"
        );
        assert_eq!(
            format_date_range(date(2026, 2, 6), date(2026, 2, 8), today),
            "Feb 6 - Feb 8"
        );
    }

    #[test]
    fn format_date_range_other_year() {
        let today = date(2026, 10, 24);
        assert_eq!(
            format_date_range(date(2025, 10, 12), date(2025, 10, 12), today),
            "Oct 12, 2025"
        );
        assert_eq!(
            format_date_range(date(2025, 10, 12), date(2025, 10, 15), today),
            "Oct 12 - Oct 15, 2025"
        );
    }

    #[test]
    fn format_date_range_across_years() {
        let today = date(2026, 10, 24);
        assert_eq!(
            format_date_range(date(2025, 12, 30), date(2026, 1, 2), today),
            "Dec 30, 2025 - Jan 2, 2026"
        );
        // The current year does not shorten a range that crosses years.
        assert_eq!(
            format_date_range(date(2026, 12, 30), date(2027, 1, 2), today),
            "Dec 30, 2026 - Jan 2, 2027"
        );
    }
}
