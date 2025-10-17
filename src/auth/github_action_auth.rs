use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::AppState;

const HEADER_NAME: &str = "X-Github-Action-Client-Key";

pub async fn github_action_auth_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let expected = match state.github_client_token {
        Some(token) => token,
        None => {
            return (
                StatusCode::NOT_IMPLEMENTED,
                "GitHub Action client token not configured",
            )
                .into_response()
        }
    };

    let header = match req.headers().get(HEADER_NAME) {
        Some(header) => header,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                format!("Missing header: {HEADER_NAME}"),
            )
                .into_response()
        }
    };

    let header = match header.to_str() {
        Ok(value) => value,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid header value").into_response(),
    };

    if header != expected {
        return (StatusCode::UNAUTHORIZED, "Invalid client token").into_response();
    }

    next.run(req).await
}
