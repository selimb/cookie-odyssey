use std::collections::HashMap;

use tera::{Tera, Value};

pub fn init_templates() -> Result<Tera, String> {
    let mut tera = Tera::new("templates/**/*.html")
        .map_err(|err| format!("Failed to initialize tera: {err}"))?;
    tera.register_filter("date", date);
    Ok(tera)
}

type FilterArgs = HashMap<String, Value>;
type FilterResult = tera::Result<Value>;

fn date(value: &Value, _: &FilterArgs) -> FilterResult {
    match value {
        Value::String(s) => {
            match chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
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
