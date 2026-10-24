use axum::{
    extract::{rejection::FormRejection, Path, State},
    response::IntoResponse,
    Form,
};
use minijinja::context;
use sea_orm::{EntityTrait, SqlErr};
use serde::Deserialize;

use crate::{
    journal::queries::query_journal_by_slug,
    utils::serde_utils::{empty_as_none, string_trim},
    AppState, FormError, Route, RouteResult, Templ,
};
use entities::{prelude::*, *};

#[derive(Deserialize, Debug)]
pub struct JournalEdit {
    #[serde(deserialize_with = "string_trim")]
    pub name: String,
    #[serde(deserialize_with = "string_trim")]
    pub slug: String,
    pub start_date: chrono::NaiveDate,
    #[serde(deserialize_with = "empty_as_none")]
    pub end_date: Option<chrono::NaiveDate>,
    /// The cover photo to keep, or `None` for no cover.
    /// Prefilled with the current cover, so an upload replaces it and the
    /// remove button clears it.
    #[serde(deserialize_with = "empty_as_none")]
    pub cover_file_id: Option<i32>,
}

impl JournalEdit {
    fn validate(&self) -> Result<(), FormError> {
        if self.name.is_empty() || self.slug.is_empty() {
            return Err(FormError::new("Name and slug are required"));
        }
        if self.end_date.is_some_and(|end| end < self.start_date) {
            return Err(FormError::new("End date must not be before start date"));
        }
        Ok(())
    }
}

pub async fn page_journal_edit_get(
    state: State<AppState>,
    templ: Templ,
    Path(slug): Path<String>,
) -> RouteResult {
    let journal = match query_journal_by_slug(slug.clone(), &state.db).await? {
        Ok(journal) => journal,
        Err(err) => {
            return Ok(err.render(&templ).into_response());
        }
    };

    let cover_url = match journal.cover_id {
        None => None,
        Some(cover_id) => match File::find_by_id(cover_id).one(&state.db).await? {
            None => None,
            Some(file) => Some(state.storage.sign_url(file.bucket, file.key).await?),
        },
    };

    let ctx = context! {
        journal,
        cover_url,
        href_edit => Route::JournalEditPost { slug: Some(&slug) }.as_path(),
        href_journal_detail => Route::JournalDetailGet { slug: Some(&slug) }.as_path(),
        href_get_upload_url => Route::MediaUploadUrlPost.as_path(),
    };
    let html = templ.render_ctx("journal_edit.html", ctx)?;
    Ok(html.into_response())
}

pub async fn hx_journal_edit_post(
    state: State<AppState>,
    templ: Templ,
    Path(slug): Path<String>,
    form: Result<Form<JournalEdit>, FormRejection>,
) -> RouteResult {
    let form = match form {
        Err(err) => {
            let resp = FormError::from(err).render(&state)?;
            return Ok(resp.into_response());
        }
        Ok(Form(form)) => form,
    };
    let journal = match query_journal_by_slug(slug.clone(), &state.db).await? {
        Ok(journal) => journal,
        Err(err) => {
            return Ok(err.render(&templ).into_response());
        }
    };

    if let Err(err) = form.validate() {
        return Ok(err.render(&state)?.into_response());
    }
    let data = journal::ActiveModel {
        id: sea_orm::ActiveValue::Unchanged(journal.id),
        name: sea_orm::ActiveValue::Set(form.name),
        slug: sea_orm::ActiveValue::Set(form.slug.clone()),
        start_date: sea_orm::ActiveValue::Set(form.start_date),
        end_date: sea_orm::ActiveValue::Set(form.end_date),
        cover_id: sea_orm::ActiveValue::Set(form.cover_file_id),
    };
    if let Err(err) = Journal::update(data).exec(&state.db).await {
        // Both name and slug are unique, and the message says which one.
        let msg = match err.sql_err() {
            Some(SqlErr::UniqueConstraintViolation(msg)) if msg.contains("journal.slug") => {
                "A journal with this slug already exists"
            }
            Some(SqlErr::UniqueConstraintViolation(msg)) if msg.contains("journal.name") => {
                "A journal with this name already exists"
            }
            _ => return Err(err.into()),
        };
        let resp = FormError::new(msg).render(&state)?;
        return Ok(resp.into_response());
    }

    let href = Route::JournalDetailGet {
        slug: Some(&form.slug),
    }
    .as_path();
    let resp = [("HX-Location", href.as_ref())];
    Ok(resp.into_response())
}
