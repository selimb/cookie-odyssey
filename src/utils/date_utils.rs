const DATE_FMT: &'static str = "%Y-%m-%d";

pub fn date_to_sqlite(d: chrono::NaiveDate) -> String {
    d.format(DATE_FMT).to_string()
}

pub fn date_from_sqlite(s: impl AsRef<str>) -> Result<chrono::NaiveDate, chrono::ParseError> {
    chrono::NaiveDate::parse_from_str(s.as_ref(), DATE_FMT)
}
