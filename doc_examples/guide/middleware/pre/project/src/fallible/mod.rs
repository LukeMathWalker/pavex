pub use blueprint::blueprint;
pub use errors::auth_error_handler;
pub use mw::reject_anonymous;
pub use routes::handler;

mod blueprint;
mod errors;
mod mw;
mod routes;

#[derive(Debug)]
pub struct AuthError;

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "No `Authorization` header found")
    }
}
