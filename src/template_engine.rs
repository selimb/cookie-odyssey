use std::sync::Arc;
use std::{borrow::Cow, ops::Deref};

use anyhow::{anyhow, Context};
use axum::http::HeaderMap;
use axum::{
    extract::{FromRef, FromRequestParts},
    response::Html,
    RequestPartsExt,
};
use itertools::Itertools;
use minijinja::context;
use once_cell::sync::Lazy;
use serde::Serialize;

use crate::{AppState, AuthSession, AuthUser, Route, RouteError};

pub type TemplateEngine = minijinja::Environment<'static>;

pub fn init_templates() -> TemplateEngine {
    let mut env = minijinja::Environment::new();
    env.set_loader(minijinja::path_loader("templates"));
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
    env.add_filter("splitlines", splitlines);
    env.add_filter("clsx", clsx);

    env
}

fn splitlines(value: &str) -> Vec<String> {
    value.split("\n\n").map(|s| s.into()).collect_vec()
}

fn clsx(value: &str) -> minijinja::Value {
    let s = value.split('\n').map(|line| line.trim()).join(" ");
    minijinja::Value::from_safe_string(s)
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
    home: Cow<'static, str>,
    admin_users_list: Cow<'static, str>,
    logout: Cow<'static, str>,
}

static TEMPL_CONTEXT_LINKS: Lazy<TemplContextLinks> = Lazy::new(|| TemplContextLinks {
    home: "/".into(),
    admin_users_list: Route::UserListGet.as_path(),
    logout: Route::LogoutPost.as_path(),
});

/// Template renderer, which is pre-populated with common context variables (see [TemplContext]).
/// Implements an axum extractor.
pub struct Templ {
    engine: Arc<TemplateEngine>,
    user: Option<TemplContextUser>,
    hx_boosted: bool,
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
        self.render_ctx_fragment(template_name, ctx, None::<String>)
    }

    pub fn render_ctx_fragment(
        &self,
        template_name: impl AsRef<str>,
        ctx: minijinja::Value,
        block_name: Option<impl AsRef<str>>,
    ) -> Result<Html<String>, RouteError> {
        // NOTE: Precedence is from left to right, so `ctx` must come first
        //   in order to override `wide_layout`.
        let context = context! { ..ctx, ..self.build_default_ctx() };
        let template = self.engine.get_template(template_name.as_ref())?;
        let body = match block_name {
            Some(block_name) => template
                .eval_to_state(context)?
                .render_block(block_name.as_ref())?,
            None => template.render(context)?,
        };
        Ok(Html(body))
    }

    fn build_default_ctx(&self) -> minijinja::Value {
        // See [TemplContext].
        context! {
            user => self.user,
            links => TEMPL_CONTEXT_LINKS.deref(),
            hx_boosted => self.hx_boosted,
            wide_layout => false,
        }
    }
}

#[allow(dead_code)] // Serves as documentation.
#[derive(Debug, Serialize)]
struct TemplContext<'a> {
    user: &'a Option<TemplContextUser>,
    links: &'a TemplContextLinks,
    hx_boosted: bool,
    /// Can be overriden.
    wide_layout: bool,
}

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

        let header_map = HeaderMap::from_request_parts(parts, state)
            .await
            .expect("HeaderMap extractor is infallible");
        let hx_boosted = header_map.contains_key("HX-Boosted");

        Ok(Self {
            engine: app_state.template_engine,
            user: user.map(TemplContextUser::from),
            hx_boosted,
        })
    }
}
