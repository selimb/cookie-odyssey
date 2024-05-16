use axum::{
    extract::{rejection::FormRejection, Path, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use minijinja::context;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    utils::date_utils::date_to_sqlite, AppState, FormError, Route, RouteError, RouteResult, Templ,
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
pub async fn journal_list(State(state): State<AppState>, templ: Templ) -> RouteResult {
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
    let ctx = context! { journals, href_new => Route::JournalNewGet.as_path() };
    let html = templ.render_ctx("journal_list.html", ctx)?;
    Ok(html.into_response())
}

#[derive(Deserialize, Debug)]
pub struct JournalNew {
    pub name: String,
    pub slug: String,
    pub start_date: chrono::NaiveDate,
    pub end_date: Option<chrono::NaiveDate>,
}

pub async fn journal_new_get(templ: Templ) -> RouteResult {
    let html = templ.render("journal_new.html")?;
    Ok(html.into_response())
}

pub async fn journal_new_post(
    state: State<AppState>,
    form: Result<Form<JournalNew>, FormRejection>,
) -> Result<Response, RouteError> {
    match form {
        Err(err) => {
            let resp = FormError::from(err).render(&state)?;
            Ok(resp.into_response())
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

pub async fn journal_detail_get(
    state: State<AppState>,
    templ: Templ,
    Path(slug): Path<String>,
) -> RouteResult {
    let journal = Journal::find()
        .filter(journal::Column::Slug.eq(slug))
        .one(&state.db)
        .await?;
    let ctx = context! { journal };
    let html = templ.render_ctx("journal_detail.html", ctx)?;
    Ok(html.into_response())
}
