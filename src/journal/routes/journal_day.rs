use axum::{
    extract::{rejection::FormRejection, Path, Query},
    response::{Html, IntoResponse as _},
    Form,
};
use minijinja::context;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::{
    comment::queries::{add_comment_to_journal, query_comments_for_journal, AddCommentToJournal},
    journal::{
        queries::{query_journal_by_slug, query_media_for_journal_entry, MediaFull},
        routes::{JournalEntryNewPath, JournalEntryNewQuery},
    },
    storage::FileStore,
    utils::date_utils::{date_from_sqlite, date_to_sqlite, time_from_sqlite},
    AppState, AuthSession, Route, RouteError, RouteResult, Templ, Toast,
};
use entities::{prelude::*, *};

#[derive(Debug, Deserialize)]
pub struct JournalDayGetPath {
    pub slug: String,
    pub date: chrono::NaiveDate,
}

pub async fn journal_day_get(
    state: AppState,
    templ: Templ,
    session: AuthSession,
    Path(JournalDayGetPath { slug, date }): Path<JournalDayGetPath>,
) -> RouteResult {
    let journal = query_journal_by_slug(slug.clone(), &state.db).await?;
    let journal = match journal {
        Ok(journal) => journal,
        Err(err) => {
            return Ok(err.render(&templ).into_response());
        }
    };

    let entries =
        query_entries_for_day(&journal, &date, &state.db, &session, &state.storage).await?;
    let comments =
        query_and_render_comments(journal.id, &date, false, &state.db, &templ, &session).await?;

    let datetime = chrono::NaiveDateTime::new(date, Default::default());
    let href_journal_detail = Route::JournalDetailGet { slug: Some(&slug) }.as_path();
    let href_journal_entry_new = Route::JournalEntryNewGet(Some((
        &JournalEntryNewPath { slug },
        &JournalEntryNewQuery { date: Some(date) },
    )))
    .as_path();
    let ctx = context! {
        journal,
        datetime,
        entries,
        comments_fragment => comments.0,
        href_journal_detail,
        href_journal_entry_new,
    };

    let html = templ.render_ctx("journal_day.html", ctx)?;
    Ok(html.into_response())
}

#[derive(Serialize, Debug)]
struct Entry {
    datetime: chrono::NaiveDateTime,
    time: chrono::NaiveTime,
    title: String,
    address: String,
    text: String,
    draft: bool,
    href_edit: String,
    media: Vec<MediaFull>,
}

async fn query_entries_for_day(
    journal: &journal::Model,
    date: &chrono::NaiveDate,
    db: &DatabaseConnection,
    auth: &AuthSession,
    storage: &FileStore,
) -> Result<Vec<Entry>, anyhow::Error> {
    let mut q = JournalEntry::find()
        .filter(journal_entry::Column::JournalId.eq(journal.id))
        .filter(journal_entry::Column::Date.eq(date_to_sqlite(*date)));
    q = auth.backend.filter_journal_entries(auth, q).await?;
    let entries_db = q.all(db).await?;

    // TODO: Avoid N+1 query, but in this case `N` should not be greater than 5, so meh.
    let mut entries: Vec<Entry> = Vec::new();
    for e in entries_db {
        let date = date_from_sqlite(e.date).unwrap();
        let time = time_from_sqlite(e.time).unwrap();
        let datetime = chrono::NaiveDateTime::new(date, time);

        let href_edit = Route::JournalEntryEditGet {
            entry_id: Some(e.id),
        }
        .as_path()
        .into_owned();

        let media = query_media_for_journal_entry(e.id, db, storage).await?;

        let entry = Entry {
            title: e.title,
            address: e.address,
            text: e.text,
            draft: e.draft,
            datetime,
            time,
            href_edit,
            media,
        };
        entries.push(entry);
    }
    entries.sort_by_key(|e| e.datetime);

    Ok(entries)
}

async fn query_and_render_comments(
    journal_id: i32,
    date: &chrono::NaiveDate,
    fragment: bool,
    db: &DatabaseConnection,
    templ: &Templ,
    auth: &AuthSession,
) -> Result<Html<String>, RouteError> {
    let comments = query_comments_for_journal(journal_id, Some(*date), db).await?;

    let href_add_comment = Route::JournalDayAddCommentPost(Some(&JournalDayAddCommentQuery {
        date: *date,
        journal_id,
    }))
    .as_path();
    let href_edit_comment = Route::JournalDayEditCommentPost(Some(&JournalDayEditCommentQuery {
        date: *date,
        journal_id,
    }))
    .as_path();
    let href_delete_comment =
        Route::JournalDayDeleteCommentPost(Some(&JournalDayDeleteCommentQuery {
            date: *date,
            journal_id,
        }))
        .as_path();

    let ctx = context! {
        journal_id,
        date,
        comments,
        user => auth.user,
        href_add_comment,
        href_edit_comment,
        href_delete_comment,
    };
    let html = if fragment {
        templ.render_ctx_fragment("comment/comment_list.html", ctx, "fragment_comment_list")?
    } else {
        templ.render_ctx("comment/comment_list.html", ctx)?
    };
    Ok(html)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct JournalDayAddCommentQuery {
    journal_id: i32,
    date: chrono::NaiveDate,
}

#[derive(Deserialize, Debug)]
pub struct JournalDayAddCommentForm {
    text: String,
}

pub async fn journal_day_add_comment_post(
    state: AppState,
    templ: Templ,
    session: AuthSession,
    query: Query<JournalDayAddCommentQuery>,
    form: Result<Form<JournalDayAddCommentForm>, FormRejection>,
) -> RouteResult {
    let form = match form {
        Err(err) => {
            let resp = Toast::error(err);
            return Ok(resp.into_response());
        }
        Ok(form) => form,
    };

    let text = form.0.text;
    let user_id = session.user.as_ref().expect("Should be authenticated").0.id;
    let params = AddCommentToJournal {
        date: Some(query.date),
        journal_id: query.journal_id,
        text,
        user_id,
    };

    add_comment_to_journal(params, &state.db).await?;

    let html = query_and_render_comments(
        query.journal_id,
        &query.date,
        true,
        &state.db,
        &templ,
        &session,
    )
    .await?;
    Ok(html.into_response())
}

#[derive(Deserialize, Serialize, Debug)]
pub struct JournalDayEditCommentQuery {
    journal_id: i32,
    date: chrono::NaiveDate,
}

#[derive(Deserialize, Debug)]
pub struct JournalDayEditCommentForm {
    comment_id: i32,
    text: String,
}

pub async fn journal_day_edit_comment_post(
    state: AppState,
    templ: Templ,
    session: AuthSession,
    query: Query<JournalDayEditCommentQuery>,
    form: Result<Form<JournalDayEditCommentForm>, FormRejection>,
) -> RouteResult {
    let form = match form {
        Err(err) => {
            let resp = Toast::error(err);
            return Ok(resp.into_response());
        }
        Ok(form) => form,
    };

    let data = journal_comment::ActiveModel {
        id: sea_orm::ActiveValue::Set(form.comment_id),
        text: sea_orm::ActiveValue::Set(form.0.text),
        ..Default::default()
    };
    // TODO [perms] Verify permissions server-side.
    JournalComment::update(data).exec(&state.db).await?;

    let html = query_and_render_comments(
        query.journal_id,
        &query.date,
        true,
        &state.db,
        &templ,
        &session,
    )
    .await?;
    Ok(html.into_response())
}

#[derive(Deserialize, Serialize, Debug)]
pub struct JournalDayDeleteCommentQuery {
    journal_id: i32,
    date: chrono::NaiveDate,
}

#[derive(Deserialize, Debug)]
pub struct JournalDayDeleteCommentForm {
    comment_id: i32,
}

pub async fn journal_day_delete_comment_post(
    state: AppState,
    templ: Templ,
    session: AuthSession,
    query: Query<JournalDayDeleteCommentQuery>,
    form: Result<Form<JournalDayDeleteCommentForm>, FormRejection>,
) -> RouteResult {
    let form = match form {
        Err(err) => {
            let resp = Toast::error(err);
            return Ok(resp.into_response());
        }
        Ok(form) => form,
    };

    // TODO [perms] Verify permissions server-side.
    JournalComment::delete_by_id(form.comment_id)
        .exec(&state.db)
        .await?;

    let html = query_and_render_comments(
        query.journal_id,
        &query.date,
        true,
        &state.db,
        &templ,
        &session,
    )
    .await?;
    Ok(html.into_response())
}
