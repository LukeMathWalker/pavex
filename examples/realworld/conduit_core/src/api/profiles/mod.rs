use pavex_builder::router::{DELETE, POST};
use pavex_builder::{f, router::GET, Blueprint};

pub(crate) fn profiles_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/:username", f!(crate::api::profiles::get_user));
    bp.route(POST, "/:username/follow", f!(crate::api::profiles::follow_user));
    bp.route(
        DELETE,
        "/:username/follow",
        f!(crate::api::profiles::unfollow_user),
    );
    bp
}

mod get_user;
mod follow_user;
mod unfollow_user;

pub use get_user::*;
pub use follow_user::*;
pub use unfollow_user::*;