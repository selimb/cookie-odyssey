use std::ops::Deref;
use std::sync::Arc;

use anyhow::{anyhow, Context};
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    response::Html,
    RequestPartsExt,
};
use minijinja::context;
use once_cell::sync::Lazy;
use serde::Serialize;

use crate::{
    utils::date_utils::date_from_sqlite, AppState, AuthSession, AuthUser, Route, RouteError,
};

pub type TemplateEngine = minijinja::Environment<'static>;

pub fn init_templates() -> TemplateEngine {
    let mut env = minijinja::Environment::new();
    env.set_loader(minijinja::path_loader("templates"));
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
    env.add_filter("date", date);
    env
}

fn date(value: String) -> String {
    match date_from_sqlite(value) {
        // [datefmt] Match `Intl.DateTimeFormat("en-US", {dateStyle: "long"}`:
        // ```
        // Intl.DateTimeFormat("en-US", {dateStyle: "long"}).format(d)
        // "December 31, 2022"
        // ```
        Ok(date) => date.format("%B %e, %Y").to_string(),
        // FIXME test
        Err(_) => "--".into(),
    }
}

#[derive(Debug, Serialize)]
struct TemplContext<'a> {
    user: &'a Option<TemplContextUser>,
    links: &'a TemplContextLinks,
}

/// Mostly like AuthUser/User, but without `password`, and maybe with some extra
/// fields eventually (like the profile image URL).
#[derive(Debug, Serialize)]
struct TemplContextUser {
    admin: bool,
    email: String,
    first_name: String,
    id: i32,
    last_name: String,
}

impl From<AuthUser> for TemplContextUser {
    fn from(value: AuthUser) -> Self {
        let user = value.0;
        Self {
            admin: user.admin,
            email: user.email,
            first_name: user.first_name,
            id: user.id,
            last_name: user.last_name,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TemplContextLinks {
    home: String,
    admin_users_list: String,
    logout: String,
    notifications: String,
}

static TEMPL_CONTEXT_LINKS: Lazy<TemplContextLinks> = Lazy::new(|| TemplContextLinks {
    home: "/".to_string(),
    admin_users_list: Route::UserListGet.as_path(),
    logout: Route::LogoutPost.as_path(),
    notifications: Route::NotificationsListGet.as_path(),
});

/// Template renderer, which is pre-populated with common context variables (see [TemplContext]).
/// Implements an axum extractor.
pub struct Templ {
    engine: Arc<TemplateEngine>,
    user: Option<TemplContextUser>,
}

impl Templ {
    pub fn render(&self, template_name: impl AsRef<str>) -> Result<Html<String>, RouteError> {
        self.render_ctx(template_name, context! {})
    }

    pub fn render_ctx(
        &self,
        template_name: impl AsRef<str>,
        ctx: minijinja::Value,
    ) -> Result<Html<String>, RouteError> {
        let context = context! { ..self.build_default_ctx(), ..ctx };
        let body = self
            .engine
            .get_template(template_name.as_ref())?
            .render(context)?;
        Ok(Html(body))
    }

    pub fn render_ctx_fragment(
        &self,
        template_name: impl AsRef<str>,
        ctx: minijinja::Value,
        block_name: impl AsRef<str>,
    ) -> Result<Html<String>, RouteError> {
        let context = context! { ..self.build_default_ctx(), ..ctx };
        let body = self
            .engine
            .get_template(template_name.as_ref())?
            .eval_to_state(context)?
            .render_block(block_name.as_ref())?;
        Ok(Html(body))
    }

    fn build_default_ctx(&self) -> minijinja::Value {
        context! {
            user => self.user,
            links => TEMPL_CONTEXT_LINKS.deref(),
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Templ
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = RouteError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let app_state = parts
            .extract_with_state::<AppState, _>(state)
            .await
            .context("Failed to extract state from request")?;

        let session = AuthSession::from_request_parts(parts, state)
            .await
            .map_err(|err| anyhow!("Failed to extract session from request: {err:?}"))?;
        let user = session.user;

        Ok(Self {
            engine: app_state.template_engine,
            user: user.map(TemplContextUser::from),
        })
    }
}
