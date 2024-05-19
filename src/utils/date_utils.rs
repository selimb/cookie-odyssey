const DATE_FMT: &str = "%Y-%m-%d";
const TIME_FMT: &str = "%H:%M";

pub fn date_to_sqlite(d: chrono::NaiveDate) -> String {
    d.format(DATE_FMT).to_string()
}

pub fn time_to_sqlite(d: chrono::NaiveTime) -> String {
    d.format(TIME_FMT).to_string()
}

pub fn date_from_sqlite(s: impl AsRef<str>) -> Result<chrono::NaiveDate, chrono::ParseError> {
    chrono::NaiveDate::parse_from_str(s.as_ref(), DATE_FMT)
}

pub fn time_from_sqlite(s: impl AsRef<str>) -> Result<chrono::NaiveTime, chrono::ParseError> {
    chrono::NaiveTime::parse_from_str(s.as_ref(), TIME_FMT)
}
