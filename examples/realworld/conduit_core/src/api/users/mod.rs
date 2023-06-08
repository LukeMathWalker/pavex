use pavex_builder::router::{POST, PUT};
use pavex_builder::{f, router::GET, Blueprint};

pub(crate) fn users_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(POST, "/users", f!(crate::api::users::signup));
    bp.route(POST, "/users/login", f!(crate::api::users::login));
    bp.route(GET, "/user", f!(crate::api::users::get_user));
    bp.route(PUT, "/user", f!(crate::api::users::update_user));
    bp
}

mod get_user;
mod login;
mod signup;
mod update_user;

pub use get_user::*;
pub use login::*;
pub use signup::*;
pub use update_user::*;
