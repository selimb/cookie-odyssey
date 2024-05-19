use std::borrow::Cow;

use axum::{http::StatusCode, response::IntoResponse};
use minijinja::context;

use crate::{RouteError, Templ};

pub struct NotFound {
    msg: Cow<'static, str>,
}

impl NotFound {
    pub fn new(msg: Cow<'static, str>) -> Self {
        Self { msg }
    }

    pub fn for_entity(entity: impl AsRef<str>) -> Self {
        let entity = entity.as_ref();
        let msg = format!("Oops! Could not find this {entity}.");
        Self { msg: msg.into() }
    }

    pub fn render(&self, templ: &Templ) -> Result<impl IntoResponse, RouteError> {
        let ctx = context! { msg => self.msg };
        let html = templ.render_ctx("oops.html", ctx)?;
        // TODO NOT_FOUND would be more appropriate, but leads to weird behavior in boosted links
        Ok((StatusCode::OK, html))
    }
}
