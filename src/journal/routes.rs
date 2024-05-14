use axum::{
    extract::{rejection::FormRejection, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use tera::Context;
use url::Url;

use crate::{
    router::Route,
    server::AppState,
    utils::{
        date_utils::date_to_sqlite,
        route_utils::{HtmlResult, RouteError},
    },
};
use entities::{prelude::*, *};

#[derive(Serialize, Debug)]
struct JournalListItem {
    id: i32,
    name: String,
    start_date: String,
    end_date: Option<String>,
    href: String,
    cover_url: Option<Url>,
}

// FIXME auth
pub async fn journal_list(State(state): State<AppState>) -> HtmlResult {
    let journals = Journal::find()
        .find_also_related(File)
        .order_by_desc(journal::Column::StartDate)
        .all(&state.db)
        .await?;
    let journals = journals
        .into_iter()
        .map(|(journal, _cover)| JournalListItem {
            id: journal.id,
            name: journal.name,
            start_date: journal.start_date,
            end_date: journal.end_date,
            cover_url: None,
            href: Route::JournalDetailGet {
                slug: Some(&journal.slug),
            }
            .as_path(),
        })
        .collect::<Vec<_>>();
    let mut context = Context::new();
    context.insert("journals", &journals);
    let resp = state.render("journal_list.html", &context)?;
    Ok(resp)
}

#[derive(Deserialize, Debug)]
pub struct JournalNew {
    pub name: String,
    pub slug: String,
    pub start_date: chrono::NaiveDate,
    pub end_date: Option<chrono::NaiveDate>,
}

pub async fn journal_new_get(State(state): State<AppState>) -> HtmlResult {
    let context = Context::new();
    let resp = state.render("journal_new.html", &context)?;
    Ok(resp)
}

pub async fn journal_new_post(
    state: State<AppState>,
    form: Result<Form<JournalNew>, FormRejection>,
) -> Result<Response, RouteError> {
    let mut context = Context::new();
    match form {
        Err(err) => {
            context.insert("error", &err.body_text());
            let body = state.tera.render("common/form_error.html", &context)?;
            let resp = (StatusCode::UNPROCESSABLE_ENTITY, Html(body)).into_response();
            Ok(resp)
        }
        Ok(form) => {
            let data = journal::ActiveModel {
                name: sea_orm::ActiveValue::Set(form.name.clone()),
                slug: sea_orm::ActiveValue::Set(form.slug.clone()),
                start_date: sea_orm::ActiveValue::Set(date_to_sqlite(form.start_date)),
                end_date: sea_orm::ActiveValue::Set(form.end_date.map(date_to_sqlite)),
                ..Default::default()
            };
            Journal::insert(data).exec(&state.db).await?;
            let resp = Redirect::to(&Route::JournalListGet.as_path()).into_response();
            Ok(resp)
        }
    }
}

pub async fn journal_detail_get(state: State<AppState>, Path(slug): Path<String>) -> HtmlResult {
    let mut context = Context::new();
    let journal = Journal::find()
        .filter(journal::Column::Slug.eq(slug))
        .one(&state.db)
        .await?;
    context.insert("journal", &journal);
    let resp = state.render("journal_detail.html", &context)?;
    Ok(resp)
}
