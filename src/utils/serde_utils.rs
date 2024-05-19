use serde::Deserialize as _;

// Copied from https://github.com/baoyachi/serde_trim/blob/b927f4b5b032dbe2cc87c47a8fcd735dffcbda02/lib.rs#L6
pub fn string_trim<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let ret = String::deserialize(d)?;
    Ok(ret)
}
