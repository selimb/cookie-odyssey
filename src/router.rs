use axum::{
    routing::{get, post},
    Router,
};
use axum_login::{login_required, permission_required};

use crate::auth::{perms::Permission, routes as auth, sessions::AuthBackend};
use crate::journal::routes as journal;
use crate::AppState;

// Idea stolen from https://github.com/jdevries3133/calcount/blob/main/src/routes.rs
// Type-safe routes!
pub enum Route<'a> {
    ForgotPasswordGet,
    ForgotPasswordPost,
    JournalDetailGet { slug: Option<&'a str> },
    JournalListGet,
    JournalNewGet,
    JournalNewPost,
    LoginGet,
    LoginPost,
    LogoutPost,
    // FIXME route
    NotificationsListGet,
    RegisterGet,
    RegisterPost,
    UserListGet,
    UserListApprovePost,
    UserListDeletePost,
}

impl<'a> Route<'a> {
    pub fn as_path(&self) -> String {
        match self {
            Route::ForgotPasswordGet => "/forgot-password".into(),
            Route::ForgotPasswordPost => "/forgot-password".into(),
            Route::JournalDetailGet { slug } => match slug {
                Some(slug) => format!("/journals/{slug}"),
                None => "/journals/:slug".to_string(),
            },
            Route::JournalListGet => "/".into(),
            Route::JournalNewGet => "/new-journal".into(),
            Route::JournalNewPost => "/new-journal".into(),
            Route::LoginGet => "/login".into(),
            Route::LoginPost => "/login".into(),
            Route::LogoutPost => "/logout".into(),
            Route::NotificationsListGet => "/notifications".into(),
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
            &Route::JournalNewGet.as_path(),
            admin!(get(journal::journal_new_get)),
        )
        .route(
            &Route::JournalNewPost.as_path(),
            admin!(post(journal::journal_new_post)),
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
