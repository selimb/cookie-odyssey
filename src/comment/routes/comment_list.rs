use axum::{
    extract::{rejection::FormRejection, Query},
    response::{Html, IntoResponse as _},
    Form,
};
use minijinja::context;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect};
use serde::{Deserialize, Serialize};

use entities::{prelude::*, *};

use crate::{
    comment::queries::{add_comment_to_journal, query_comments_for_journal, AddCommentToJournal},
    AppState, AuthSession, Route, RouteError, RouteResult, Templ, Toast,
};

#[derive(Deserialize, Serialize, Debug)]
pub struct JournalCommentAddQuery {
    journal_id: i32,
    date: Option<chrono::NaiveDate>,
}

#[derive(Deserialize, Debug)]
pub struct JournalCommentAddForm {
    text: String,
}

pub async fn journal_comment_add_post(
    state: AppState,
    templ: Templ,
    session: AuthSession,
    query: Query<JournalCommentAddQuery>,
    form: Result<Form<JournalCommentAddForm>, FormRejection>,
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
        date: query.date,
        journal_id: query.journal_id,
        text,
        user_id,
    };

    add_comment_to_journal(params, &state.db).await?;

    let html = CommentList {
        journal_id: query.journal_id,
        date: query.date,
        partial: true,
    }
    .query_and_render(&state.db, &templ, &session)
    .await?;

    Ok(html.into_response())
}

#[derive(Deserialize, Serialize, Debug)]
pub struct JournalCommentEditQuery {
    journal_id: i32,
    date: Option<chrono::NaiveDate>,
}

#[derive(Deserialize, Debug)]
pub struct JournalCommentEditForm {
    comment_id: i32,
    text: String,
}

pub async fn journal_comment_edit_post(
    state: AppState,
    templ: Templ,
    session: AuthSession,
    query: Query<JournalCommentEditQuery>,
    form: Result<Form<JournalCommentEditForm>, FormRejection>,
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

    let html = CommentList {
        journal_id: query.journal_id,
        date: query.date,
        partial: true,
    }
    .query_and_render(&state.db, &templ, &session)
    .await?;

    Ok(html.into_response())
}

#[derive(Deserialize, Serialize, Debug)]
pub struct JournalCommentDeleteQuery {
    journal_id: i32,
    date: Option<chrono::NaiveDate>,
}

#[derive(Deserialize, Debug)]
pub struct JournalCommentDeleteForm {
    comment_id: i32,
}

pub async fn journal_comment_delete_post(
    state: AppState,
    templ: Templ,
    session: AuthSession,
    query: Query<JournalCommentDeleteQuery>,
    form: Result<Form<JournalCommentDeleteForm>, FormRejection>,
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

    let html = CommentList {
        journal_id: query.journal_id,
        date: query.date,
        partial: true,
    }
    .query_and_render(&state.db, &templ, &session)
    .await?;

    Ok(html.into_response())
}

pub struct CommentList {
    pub journal_id: i32,
    pub date: Option<chrono::NaiveDate>,
    pub partial: bool,
}

impl CommentList {
    pub async fn query_and_render(
        &self,
        db: &DatabaseConnection,
        templ: &Templ,
        auth: &AuthSession,
    ) -> Result<Html<String>, RouteError> {
        let comments = query_comments_for_journal(self.journal_id, self.date, db).await?;

        let href_add_comment = Route::JournalCommentAddPost(Some(&JournalCommentAddQuery {
            date: self.date,
            journal_id: self.journal_id,
        }))
        .as_path();
        let href_edit_comment = Route::JournalCommentEditPost(Some(&JournalCommentEditQuery {
            date: self.date,
            journal_id: self.journal_id,
        }))
        .as_path();
        let href_delete_comment =
            Route::JournalCommentDeletePost(Some(&JournalCommentDeleteQuery {
                date: self.date,
                journal_id: self.journal_id,
            }))
            .as_path();

        let ctx = context! {
            journal_id=> self.journal_id,
            date=> self.date,
            comments,
            user => auth.user,
            href_add_comment,
            href_edit_comment,
            href_delete_comment,
        };
        let html = templ.render_ctx_fragment(
            "comment/comment_list.html",
            ctx,
            if self.partial {
                Some("fragment_comment_list")
            } else {
                None
            },
        )?;
        Ok(html)
    }
}
