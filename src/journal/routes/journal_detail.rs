use anyhow::Context as _;
use axum::{
    extract::{rejection::FormRejection, Path, Query, State},
    response::IntoResponse as _,
    Form,
};
use itertools::Itertools;
use minijinja::context;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect};
use serde::{Deserialize, Serialize};

use crate::{
    journal::queries::query_journal_by_slug,
    utils::date_utils::{date_from_sqlite, time_from_sqlite},
    AppState, AuthSession, Route, RouteResult, Templ, Toast,
};
use entities::{prelude::*, *};

use super::{JournalDayGetPath, JournalEntryNewPath, JournalEntryNewQuery};

pub async fn journal_detail_get(
    state: State<AppState>,
    templ: Templ,
    session: AuthSession,
    Path(slug): Path<String>,
) -> RouteResult {
    let journal = query_journal_by_slug(slug.clone(), &state.db).await?;
    let journal = match journal {
        Ok(journal) => journal,
        Err(err) => {
            return Ok(err.render(&templ).into_response());
        }
    };

    let entries_by_day = query_entries_by_day(&journal, &state.db, &session).await?;

    let href_journal_entry_new = Route::JournalEntryNewGet(Some((
        &JournalEntryNewPath { slug },
        &JournalEntryNewQuery::default(),
    )))
    .as_path();

    let ctx = context! { journal, entries_by_day, href_journal_entry_new };
    let html = templ.render_ctx("journal_detail.html", ctx)?;
    Ok(html.into_response())
}

#[derive(Serialize, Debug)]
struct EntrySlim {
    id: i32,
    title: String,
    date: chrono::NaiveDate,
    datetime: chrono::NaiveDateTime,
    draft: bool,
}

#[derive(Serialize, Debug)]
struct Day {
    date: chrono::NaiveDateTime,
    /// One-based number of days since the journal's start date
    day_number: i64,
    href: String,
    entries: Vec<EntrySlim>,
}

async fn query_entries_by_day(
    journal: &journal::Model,
    db: &DatabaseConnection,
    auth: &AuthSession,
) -> Result<Vec<Day>, anyhow::Error> {
    let mut q = JournalEntry::find()
        .columns([
            journal_entry::Column::Id,
            journal_entry::Column::Title,
            journal_entry::Column::Date,
            journal_entry::Column::Time,
            journal_entry::Column::Draft,
        ])
        .filter(journal_entry::Column::JournalId.eq(journal.id));
    q = auth.backend.filter_journal_entries(auth, q).await?;
    let entries = q.all(db).await.context("DB query failed")?;

    let entries = entries
        .into_iter()
        .map(|e| {
            let date = date_from_sqlite(e.date).unwrap();
            let time = time_from_sqlite(e.time).unwrap();
            let datetime = chrono::NaiveDateTime::new(date, time);
            EntrySlim {
                id: e.id,
                title: e.title,
                date,
                datetime,
                draft: e.draft,
            }
        })
        .sorted_by_key(|e| e.datetime);

    let journal_start = date_from_sqlite(&journal.start_date).unwrap();
    let mut entries_by_day: Vec<Day> = Vec::new();
    for (date, chunk) in &entries.into_iter().chunk_by(|e| e.date) {
        let day_number = (date - journal_start).num_days() + 1;
        let href = Route::JournalDayGet(Some(&JournalDayGetPath {
            slug: journal.slug.clone(),
            date,
        }))
        .as_path()
        .to_string();

        entries_by_day.push(Day {
            date: chrono::NaiveDateTime::new(date, Default::default()),
            day_number,
            href,
            entries: chunk.collect_vec(),
        })
    }

    Ok(entries_by_day)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct JournalDetailAddCommentQuery {
    journal_id: i32,
}

#[derive(Deserialize, Debug)]
pub struct JournalDetailAddCommentForm {
    text: String,
}

pub async fn journal_detail_add_comment_post(
    state: AppState,
    templ: Templ,
    session: AuthSession,
    query: Query<JournalDetailAddCommentQuery>,
    form: Result<Form<JournalDetailAddCommentForm>, FormRejection>,
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
pub struct JournalDetailEditCommentQuery {
    journal_id: i32,
    date: chrono::NaiveDate,
}

#[derive(Deserialize, Debug)]
pub struct JournalDetailEditCommentForm {
    comment_id: i32,
    text: String,
}

pub async fn journal_detail_edit_comment_post(
    state: AppState,
    templ: Templ,
    session: AuthSession,
    query: Query<JournalDetailEditCommentQuery>,
    form: Result<Form<JournalDetailEditCommentForm>, FormRejection>,
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
pub struct JournalDetailDeleteCommentQuery {
    journal_id: i32,
    date: chrono::NaiveDate,
}

#[derive(Deserialize, Debug)]
pub struct JournalDetailDeleteCommentForm {
    comment_id: i32,
}

pub async fn journal_detail_delete_comment_post(
    state: AppState,
    templ: Templ,
    session: AuthSession,
    query: Query<JournalDetailDeleteCommentQuery>,
    form: Result<Form<JournalDetailDeleteCommentForm>, FormRejection>,
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
