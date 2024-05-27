use chrono::Utc;
use serde::Serialize;

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

// pub struct Datetimetz(chrono::DateTime<Utc>);

// impl From<i64> for Datetimetz {
//     fn from(value: i64) -> Self {
//         let dt = chrono::DateTime::from_timestamp(value, 0).expect("Should be a valid date");
//         Self(dt)
//     }
// }

// impl Serialize for Datetimetz {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         Ok(self.0.to_rfc3339())
//     }
// }
