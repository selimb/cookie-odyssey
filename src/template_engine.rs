use std::collections::HashMap;

use anyhow::Context;
use tera::{Tera, Value};

use crate::utils::date_utils::date_from_sqlite;

pub fn init_templates() -> Result<Tera, anyhow::Error> {
    let mut tera = Tera::new("templates/**/*.html").context("Failed to initialize tera: {err}")?;
    tera.register_filter("date", date);
    Ok(tera)
}

type FilterArgs = HashMap<String, Value>;
type FilterResult = tera::Result<Value>;

fn date(value: &Value, _: &FilterArgs) -> FilterResult {
    match value {
        Value::String(s) => {
            match date_from_sqlite(s) {
                // [datefmt] Match `Intl.DateTimeFormat("en-US", {dateStyle: "long"}`:
                // ```
                // Intl.DateTimeFormat("en-US", {dateStyle: "long"}).format(d)
                // "December 31, 2022"
                // ```
                Ok(date) => Ok(date.format("%B %e, %Y").to_string().into()),
                // FIXME test
                Err(_) => Ok("--".into()),
            }
        }
        // FIXME const
        _ => Ok("--".into()),
    }
}
