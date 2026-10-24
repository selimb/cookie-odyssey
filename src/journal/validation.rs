use crate::FormError;

/// Checks that a Journal slug is safe to insert into a URL as is.
pub fn validate_slug(slug: &str) -> Result<(), FormError> {
    if slug.is_empty() {
        return Err(FormError::new("Slug is required"));
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(FormError::new(
            "Slug may only contain letters, numbers, dashes and underscores",
        ));
    }
    Ok(())
}
