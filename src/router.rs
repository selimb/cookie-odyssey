use crate::{journal::routes as journal, server::AppState, storage::routes as storage};
use app_config::AppConfig;
use axum::{
    http::Method,
    routing::{get, on, post, MethodFilter},
    Router,
};

// Idea stolen from https://github.com/jdevries3133/calcount/blob/main/src/routes.rs
// Type-safe routes!
pub enum Route {
    JournalList,
    JournalNew,
}

impl Route {
    pub fn as_path(&self) -> String {
        match self {
            Route::JournalList => "/".into(),
            Route::JournalNew => "/journals/new".into(),
        }
    }
}

pub fn init_router(conf: &AppConfig) -> Router<AppState> {
    let mut router = Router::new()
        .route(&Route::JournalList.as_path(), get(journal::journal_list))
        .route(
            &Route::JournalNew.as_path(),
            on(
                MethodFilter::GET.or(MethodFilter::POST),
                journal::journal_new,
            ),
        );
    if let app_config::StorageConfig::Local(c) = conf.storage {
        router = router.nest_service(&c.root_url.0, storage::LocalFileStoreRoute::new(c.root_dir))
    };
    router
}
