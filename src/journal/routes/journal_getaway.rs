use axum::{
    extract::{rejection::FormRejection, Path},
    response::IntoResponse as _,
    Form,
};
use minijinja::context;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use crate::{
    auth::utils::unwrap_user_from_session,
    comment::{queries::CommentScope, routes::CommentList},
    journal::{
        queries::{
            query_entry_views, query_journal_by_slug, query_neighbours, CardKey, EntryScope,
        },
        routes::{JournalEntryNewPath, JournalEntryNewQuery},
    },
    notification::queries::{mark_read, ReadScope},
    utils::serde_utils::string_trim,
    AppState, AuthSession, FormError, NotFound, Route, RouteResult, Templ,
};
use entities::{prelude::*, *};

#[derive(Debug, Deserialize)]
pub struct JournalGetawayGetPath {
    pub slug: String,
    pub getaway_id: i32,
}

pub async fn page_journal_getaway_get(
    state: AppState,
    mut templ: Templ,
    session: AuthSession,
    Path(JournalGetawayGetPath { slug, getaway_id }): Path<JournalGetawayGetPath>,
) -> RouteResult {
    let journal = match query_journal_by_slug(slug.clone(), &state.db).await? {
        Ok(journal) => journal,
        Err(err) => {
            return Ok(err.render(&templ).into_response());
        }
    };
    let getaway = JournalGetaway::find_by_id(getaway_id)
        .filter(journal_getaway::Column::JournalId.eq(journal.id))
        .one(&state.db)
        .await?;
    let Some(getaway) = getaway else {
        return Ok(NotFound::for_entity("getaway")
            .render(&templ)?
            .into_response());
    };
    let scope = EntryScope::Getaway {
        journal_id: journal.id,
        getaway_id,
    };
    let entries = query_entry_views(&scope, &state.db, &session, &state.storage).await?;
    // A Getaway without visible Entries does not exist for this User.
    let (start, end) = match (entries.first(), entries.last()) {
        (Some(first), Some(last)) => (first.datetime.date(), last.datetime.date()),
        _ => {
            return Ok(NotFound::for_entity("getaway")
                .render(&templ)?
                .into_response());
        }
    };

    // Viewing a Getaway marks the Notifications of its Entries as read,
    // regardless of how the User got here.
    let user_id = unwrap_user_from_session(&session).0.id;
    mark_read(&state.db, user_id, ReadScope::Getaway { getaway_id }).await?;
    templ.refresh_badges(&state.db).await?;

    let comments = CommentList {
        scope: CommentScope {
            journal_id: journal.id,
            date: None,
            getaway_id: Some(getaway_id),
        },
        partial: false,
    }
    .query_and_render(&state.db, &templ, &session)
    .await?;

    let key = CardKey::getaway(start, getaway_id);
    let (nav_prev, nav_next) = query_neighbours(&journal, &key, &state.db, &session).await?;

    let href_journal_detail = Route::JournalDetailGet { slug: Some(&slug) }.as_path();
    let href_journal_entry_new = Route::JournalEntryNewGet(Some((
        &JournalEntryNewPath { slug: slug.clone() },
        &JournalEntryNewQuery {
            date: Some(end),
            getaway_id: Some(getaway_id),
        },
    )))
    .as_path();
    let href_rename = Route::JournalGetawayRenamePost {
        getaway_id: Some(getaway_id),
    }
    .as_path();

    let ctx = context! {
        journal,
        getaway,
        // See [date-as-datetime].
        start => chrono::NaiveDateTime::new(start, Default::default()),
        end => chrono::NaiveDateTime::new(end, Default::default()),
        entries,
        comments_fragment => comments.0,
        href_journal_detail,
        href_journal_entry_new,
        href_rename,
        nav_prev,
        nav_next,
    };
    let html = templ.render_ctx("journal_getaway.html", ctx)?;
    Ok(html.into_response())
}

#[derive(Deserialize, Debug)]
pub struct JournalGetawayRename {
    #[serde(deserialize_with = "string_trim")]
    name: String,
}

pub async fn hx_journal_getaway_rename_post(
    state: AppState,
    Path(getaway_id): Path<i32>,
    form: Result<Form<JournalGetawayRename>, FormRejection>,
) -> RouteResult {
    let Form(JournalGetawayRename { name }) = match form {
        Ok(form) => form,
        Err(err) => {
            let resp = FormError::from(err).render(&state)?;
            return Ok(resp.into_response());
        }
    };
    let getaway = JournalGetaway::find_by_id(getaway_id)
        .find_also_related(Journal)
        .one(&state.db)
        .await?;
    let (getaway, journal) = match getaway {
        Some((getaway, Some(journal))) => (getaway, journal),
        _ => {
            let resp = FormError::new("This Getaway no longer exists").render(&state)?;
            return Ok(resp.into_response());
        }
    };

    if name.is_empty() {
        let resp = FormError::new("Name is required").render(&state)?;
        return Ok(resp.into_response());
    }
    // Names match ignoring case, see [getaway-name-nocase].
    // The lookup uses [getaway-name-index].
    let name_taken = JournalGetaway::find()
        .filter(journal_getaway::Column::JournalId.eq(journal.id))
        .filter(journal_getaway::Column::Name.eq(name.clone()))
        .filter(journal_getaway::Column::Id.ne(getaway.id))
        .one(&state.db)
        .await?
        .is_some();
    if name_taken {
        let resp = FormError::new("A Getaway with this name already exists").render(&state)?;
        return Ok(resp.into_response());
    }

    let data = journal_getaway::ActiveModel {
        id: sea_orm::ActiveValue::Unchanged(getaway.id),
        name: sea_orm::ActiveValue::Set(name),
        ..Default::default()
    };
    JournalGetaway::update(data).exec(&state.db).await?;

    let href = Route::JournalGetawayGet(Some(&JournalGetawayGetPath {
        slug: journal.slug,
        getaway_id,
    }))
    .as_path();
    let resp = [("HX-Location", href.as_ref())];
    Ok(resp.into_response())
}
