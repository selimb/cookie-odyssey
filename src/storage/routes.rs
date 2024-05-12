use app_config::LocalStorageRootUrl;
use axum::{extract::Request, response::IntoResponse};

/// Like [tower_http::services::ServeDir], but also allows uploads.
pub struct LocalFileStoreRoute {
    root_dir: String,
}

impl LocalFileStoreRoute {
    pub fn new(root_dir: String) -> Self {
        LocalFileStoreRoute { root_dir }
    }

    pub fn url_for(root_url: &LocalStorageRootUrl, path: impl AsRef<str>) -> String {}
}
