use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Context};
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    response::Html,
    RequestPartsExt,
};
use serde::Serialize;
use tera::{Tera, Value};

use crate::{
    utils::{date_utils::date_from_sqlite, route_utils::RouteError},
    AppState, AuthSession, AuthUser,
};

pub fn init_templates() -> Result<Tera, anyhow::Error> {
    let mut tera = Tera::new("templates/**/*.html").context("Failed to initialize tera: {err}")?;
    tera.register_filter("date", date);
    Ok(tera)
}

type FilterArgs = HashMap<String, Value>;
type FilterResult = tera::Result<Value>;

fn date(value: &Value, _: &FilterArgs) -> FilterResult {
    match value {
        Value::String(s) => {
            match date_from_sqlite(s) {
                // [datefmt] Match `Intl.DateTimeFormat("en-US", {dateStyle: "long"}`:
                // ```
                // Intl.DateTimeFormat("en-US", {dateStyle: "long"}).format(d)
                // "December 31, 2022"
                // ```
                Ok(date) => Ok(date.format("%B %e, %Y").to_string().into()),
                // FIXME test
                Err(_) => Ok("--".into()),
            }
        }
        // FIXME const
        _ => Ok("--".into()),
    }
}

/// Convenient axum extractor for grabbing all common template context variables.
pub struct Templ {
    tera: Arc<Tera>,
    user: Option<AuthUser>,
}

impl Templ {
    pub fn render(&self, template_name: impl AsRef<str>) -> Result<Html<String>, RouteError> {
        self.render_ctx(template_name, Default::default())
    }

    pub fn render_ctx(
        &self,
        template_name: impl AsRef<str>,
        context: tera::Context,
    ) -> Result<Html<String>, RouteError> {
        let mut ctx = self.build_default_ctx()?;
        ctx.extend(context);
        let body = self.tera.render(template_name.as_ref(), &ctx)?;
        Ok(Html(body))
    }

    fn build_default_ctx(&self) -> Result<tera::Context, tera::Error> {
        let ctx = TemplContext {
            user: &self.user,
            first_login: match &self.user {
                Some(user) => user.0.first_login,
                None => false,
            },
        };
        tera::Context::from_serialize(ctx)
    }
}

#[derive(Debug, Serialize)]
struct TemplContext<'a> {
    user: &'a Option<AuthUser>,
    first_login: bool,
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
            tera: app_state.tera.clone(),
            user,
        })
    }
}
