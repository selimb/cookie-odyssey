use axum::{
    extract::{rejection::FormRejection, Path, State},
    response::IntoResponse,
    Form,
};
use minijinja::context;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
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
    pub start_date: chrono::NaiveDate,
    #[serde(deserialize_with = "empty_as_none")]
    pub end_date: Option<chrono::NaiveDate>,
    /// The cover photo to keep, or `None` for no cover.
    /// Prefilled with the current cover, so an upload replaces it and the
    /// remove button clears it.
    #[serde(deserialize_with = "empty_as_none")]
    pub cover_file_id: Option<i32>,
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

    if form.name.is_empty() {
        let resp = FormError::new("Name is required").render(&state)?;
        return Ok(resp.into_response());
    }
    if let Some(end_date) = form.end_date {
        if end_date < form.start_date {
            let resp = FormError::new("End date must not be before start date").render(&state)?;
            return Ok(resp.into_response());
        }
    }
    let name_taken = Journal::find()
        .filter(journal::Column::Name.eq(form.name.clone()))
        .filter(journal::Column::Id.ne(journal.id))
        .one(&state.db)
        .await?
        .is_some();
    if name_taken {
        let resp = FormError::new("A journal with this name already exists").render(&state)?;
        return Ok(resp.into_response());
    }

    if let Some(cover_file_id) = form.cover_file_id {
        let exists = File::find_by_id(cover_file_id)
            .one(&state.db)
            .await?
            .is_some();
        if !exists {
            let resp = FormError::new("Unknown cover photo").render(&state)?;
            return Ok(resp.into_response());
        }
    }
    // The Slug is deliberately left alone, so that existing links keep working.
    let data = journal::ActiveModel {
        id: sea_orm::ActiveValue::Unchanged(journal.id),
        name: sea_orm::ActiveValue::Set(form.name),
        start_date: sea_orm::ActiveValue::Set(form.start_date),
        end_date: sea_orm::ActiveValue::Set(form.end_date),
        cover_id: sea_orm::ActiveValue::Set(form.cover_file_id),
        ..Default::default()
    };
    Journal::update(data).exec(&state.db).await?;

    let href = Route::JournalDetailGet { slug: Some(&slug) }.as_path();
    let resp = [("HX-Location", href.as_ref())];
    Ok(resp.into_response())
}
