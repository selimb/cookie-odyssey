use std::borrow::Cow;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use axum_login::{login_required, permission_required};

use crate::journal::routes::{self as journal, JournalEntryNewQuery};
use crate::storage::routes as storage;
use crate::AppState;
use crate::{
    auth::{perms::Permission, routes as auth, sessions::AuthBackend},
    journal::routes::JournalEntryNewPath,
};

// Idea stolen from https://github.com/jdevries3133/calcount/blob/main/src/routes.rs
// Type-safe routes!
pub enum Route<'a> {
    ForgotPasswordGet,
    ForgotPasswordPost,
    JournalDetailGet { slug: Option<&'a str> },
    JournalListGet,
    JournalNewGet,
    JournalNewPost,
    JournalEntryNewGet(Option<(&'a JournalEntryNewPath, &'a JournalEntryNewQuery)>),
    JournalEntryNewPost { slug: Option<&'a str> },
    JournalEntryEditGet { entry_id: Option<i32> },
    JournalEntryEditPost { entry_id: Option<i32> },
    JournalEntryPublishPost { entry_id: Option<i32> },
    JournalDayGet(Option<&'a journal::JournalDayGetPath>),
    JournalEntryMediaCommitPost(Option<&'a journal::JournalEntryMediaCommitParams>),
    JournalEntryMediaEditCaptionPost,
    JournalEntryMediaDelete,
    JournalEntryMediaReorder,
    LoginGet,
    LoginPost,
    LogoutPost,
    MediaUploadUrlGet,
    MediaUploadProxyPut(Option<&'a storage::MediaUploadProxyParams>),
    RegisterGet,
    RegisterPost,
    UserListGet,
    UserListApprovePost,
    UserListDeletePost,
}

const EXPECT_QS: &'static str = "Should be a valid querystring";

impl<'a> Route<'a> {
    pub fn as_path(&self) -> Cow<'static, str> {
        match self {
            Route::ForgotPasswordGet => "/forgot-password".into(),
            Route::ForgotPasswordPost => "/forgot-password".into(),
            Route::JournalDetailGet { slug } => match slug {
                Some(slug) => format!("/journal/{slug}").into(),
                None => "/journal/:slug".into(),
            },
            Route::JournalListGet => "/".into(),
            Route::JournalNewGet => "/new-journal".into(),
            Route::JournalNewPost => "/new-journal".into(),
            Route::JournalEntryNewGet(params) => match params {
                None => "/journal/:slug/new-entry".into(),
                Some((path, query)) => {
                    let slug = &path.slug;
                    let qs = serde_qs::to_string(query).expect(EXPECT_QS);
                    format!("/journal/{slug}/new-entry?{qs}").into()
                }
            },
            Route::JournalEntryNewPost { slug } => match slug {
                Some(slug) => format!("/journal/{slug}/new-entry").into(),
                None => "/journal/:slug/new-entry".into(),
            },
            Route::JournalEntryEditGet { entry_id } => match entry_id {
                // Giving up on "pretty URLs"
                Some(entry_id) => format!("/entry/{entry_id}/edit").into(),
                None => "/entry/:entry_id/edit".into(),
            },
            Route::JournalEntryEditPost { entry_id } => match entry_id {
                Some(entry_id) => format!("/entry/{entry_id}/edit").into(),
                None => "/entry/:entry_id/edit".into(),
            },
            Route::JournalEntryPublishPost { entry_id } => match entry_id {
                Some(entry_id) => format!("/entry/{entry_id}/publish").into(),
                None => "/entry/:entry_id/publish".into(),
            },
            Route::JournalDayGet(params) => match params {
                None => "/journal/:slug/entry/:date".into(),
                Some(params) => format!("/journal/{}/entry/{}", params.slug, params.date).into(),
            },
            Route::LoginGet => "/login".into(),
            Route::LoginPost => "/login".into(),
            Route::LogoutPost => "/logout".into(),
            Route::MediaUploadUrlGet => "/api/media-upload-url".into(),
            Route::MediaUploadProxyPut(params) => match params {
                Some(params) => {
                    let qs = serde_qs::to_string(params).expect(EXPECT_QS);
                    format!("/api/media-upload?{qs}").into()
                }
                None => "/api/media-upload".into(),
            },
            Route::JournalEntryMediaCommitPost(params) => match params {
                Some(params) => {
                    let qs = serde_qs::to_string(params).expect(EXPECT_QS);
                    format!("/api/entry-commit?{qs}").into()
                }
                None => "/api/entry-commit".into(),
            },
            Route::JournalEntryMediaEditCaptionPost => "/api/media-caption-edit".into(),
            Route::JournalEntryMediaDelete => "/api/media-delete".into(),
            Route::JournalEntryMediaReorder => "/api/media-reorder".into(),
            Route::RegisterGet => "/register".into(),
            Route::RegisterPost => "/register".into(),
            Route::UserListGet => "/users".into(),
            Route::UserListApprovePost => "/hx/users/approve".into(),
            Route::UserListDeletePost => "/hx/users/delete".into(),
        }
    }
}

macro_rules! admin {
    ($route:expr) => {
        $route.route_layer(permission_required!(AuthBackend, Permission::Admin))
    };
}

fn get_protected_routes() -> Router<AppState> {
    Router::new()
        .route(&Route::LogoutPost.as_path(), get(auth::logout_post))
        .route(&Route::JournalListGet.as_path(), get(journal::journal_list))
        .route(
            &Route::JournalDetailGet { slug: None }.as_path(),
            get(journal::journal_detail_get),
        )
        .route(
            &Route::JournalDayGet(None).as_path(),
            get(journal::journal_day_get),
        )
        .route(
            &Route::JournalNewGet.as_path(),
            admin!(get(journal::journal_new_get)),
        )
        .route(
            &Route::JournalNewPost.as_path(),
            admin!(post(journal::journal_new_post)),
        )
        .route(
            &Route::JournalEntryNewGet(None).as_path(),
            admin!(get(journal::journal_entry_new_get)),
        )
        .route(
            &Route::JournalEntryNewPost { slug: None }.as_path(),
            admin!(post(journal::journal_entry_new_post)),
        )
        .route(
            &Route::JournalEntryEditGet { entry_id: None }.as_path(),
            admin!(get(journal::journal_entry_edit_get)),
        )
        .route(
            &Route::JournalEntryEditPost { entry_id: None }.as_path(),
            admin!(post(journal::journal_entry_edit_post)),
        )
        .route(
            &Route::JournalEntryPublishPost { entry_id: None }.as_path(),
            admin!(post(journal::journal_entry_publish_post)),
        )
        .route(
            &Route::JournalEntryMediaCommitPost(None).as_path(),
            admin!(post(journal::journal_entry_media_commit_post)),
        )
        .route(
            &Route::JournalEntryMediaEditCaptionPost.as_path(),
            admin!(post(journal::journal_entry_media_caption_edit)),
        )
        .route(
            &Route::JournalEntryMediaDelete.as_path(),
            admin!(delete(journal::journal_entry_media_delete)),
        )
        .route(
            &Route::JournalEntryMediaReorder.as_path(),
            admin!(post(journal::journal_entry_media_reorder)),
        )
        .route(
            &Route::MediaUploadUrlGet.as_path(),
            admin!(get(storage::media_upload_url_get)),
        )
        .route(
            &Route::MediaUploadProxyPut(None).as_path(),
            admin!(put(storage::media_upload_proxy)),
        )
        .route(
            &Route::UserListGet.as_path(),
            admin!(get(auth::user_list_get)),
        )
        .route(
            &Route::UserListApprovePost.as_path(),
            admin!(post(auth::user_approve_post)),
        )
        .route(
            &Route::UserListDeletePost.as_path(),
            admin!(post(auth::user_delete_post)),
        )
}

fn get_public_routes() -> Router<AppState> {
    Router::new()
        .route(&Route::LoginGet.as_path(), get(auth::login_get))
        .route(&Route::LoginPost.as_path(), post(auth::login_post))
        .route(&Route::RegisterGet.as_path(), get(auth::register_get))
        .route(&Route::RegisterPost.as_path(), post(auth::register_post))
        .route(
            &Route::ForgotPasswordGet.as_path(),
            get(auth::forgot_password_get),
        )
        .route(
            &Route::ForgotPasswordPost.as_path(),
            post(auth::forgot_password_post),
        )
}

pub fn init_router() -> Router<AppState> {
    get_protected_routes()
        .route_layer(login_required!(
            AuthBackend,
            login_url = &Route::LoginGet.as_path()
        ))
        .merge(get_public_routes())
}
