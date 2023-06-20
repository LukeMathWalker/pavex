use pavex::blueprint::constructor::Lifecycle;
use pavex::blueprint::router::{POST, PUT};
use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub(crate) fn users_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    // Only the users-related routes need access to the encoding key.
    // All the other routes only need access to the decoding key.
    bp.constructor(
        f!(crate::configuration::AuthConfig::encoding_key),
        Lifecycle::Singleton,
    );

    bp.route(POST, "/users", f!(crate::routes::users::signup))
        .error_handler(f!(crate::routes::users::SignupError::into_response));
    bp.route(POST, "/users/login", f!(crate::routes::users::login))
        .error_handler(f!(crate::routes::users::LoginError::into_response));
    bp.route(GET, "/user", f!(crate::routes::users::get_user));
    bp.route(PUT, "/user", f!(crate::routes::users::update_user));
    bp
}

mod endpoints;
mod password;

pub use endpoints::*;