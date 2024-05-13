
use axum::{
    routing::{get, post},
    Router,
};

use crate::journal::routes as journal;
use crate::server::AppState;

// Idea stolen from https://github.com/jdevries3133/calcount/blob/main/src/routes.rs
// Type-safe routes!
pub enum Route<'a> {
    JournalListGet,
    JournalNewGet,
    JournalNewPost,
    JournalDetailGet { slug: Option<&'a str> },
}

impl<'a> Route<'a> {
    pub fn as_path(&self) -> String {
        match self {
            Route::JournalListGet => "/".into(),
            Route::JournalNewGet => "/new-journal".into(),
            Route::JournalNewPost => "/new-journal".into(),
            Route::JournalDetailGet { slug } => match slug {
                Some(slug) => format!("/journals/{slug}"),
                None => "/journals/:slug".to_string(),
            },
        }
    }
}

pub fn init_router() -> Router<AppState> {
    Router::new()
        .route(&Route::JournalListGet.as_path(), get(journal::journal_list))
        .route(
            &Route::JournalNewGet.as_path(),
            get(journal::journal_new_get),
        )
        .route(
            &Route::JournalNewPost.as_path(),
            post(journal::journal_new_post),
        )
        .route(
            &Route::JournalDetailGet { slug: None }.as_path(),
            get(journal::journal_detail_get),
        )
}
