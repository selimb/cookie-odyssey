use serde::Deserialize as _;

// Copied from https://github.com/baoyachi/serde_trim/blob/b927f4b5b032dbe2cc87c47a8fcd735dffcbda02/lib.rs#L6
pub fn string_trim<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let ret = String::deserialize(d)?;
    Ok(ret.trim().to_string())
}

/// Treats an empty (or blank) form value as `None`.
/// Use it for optional fields that the browser submits even when empty, such
/// as a non-required `<input>` or a hidden input.
/// It is not needed for checkboxes, or for query strings built by `Route`,
/// where the key is absent instead, and a plain `Option<T>` works.
pub fn empty_as_none<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::de::Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let raw = String::deserialize(d)?;
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    raw.parse().map(Some).map_err(serde::de::Error::custom)
}
