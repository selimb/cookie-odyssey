use crate::{AuthSession, AuthUser};

/// Returns the logged-in User, for routes that require authentication.
pub fn unwrap_user_from_session(session: &AuthSession) -> &AuthUser {
    session.user.as_ref().expect("Should be authenticated")
}
