/// Returns the one-based number of calendar days since the Journal's start
/// date, or `None` in a Scrapbook, whose Days are not numbered.
pub fn get_journal_day_number_for_date(
    journal: &entities::journal::Model,
    date: chrono::NaiveDate,
) -> Option<i64> {
    (!journal.scrapbook).then(|| (date - journal.start_date).num_days() + 1)
}
