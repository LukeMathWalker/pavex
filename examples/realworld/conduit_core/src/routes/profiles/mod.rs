use pavex::blueprint::router::{DELETE, POST};
use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub(crate) fn profiles_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/:username", f!(crate::routes::profiles::get_profile));
    bp.route(
        POST,
        "/:username/follow",
        f!(crate::routes::profiles::follow_profile),
    );
    bp.route(
        DELETE,
        "/:username/follow",
        f!(crate::routes::profiles::unfollow_profile),
    );
    bp
}

mod follow_profile;
mod get_profile;
mod unfollow_profile;

pub use follow_profile::*;
pub use get_profile::*;
pub use unfollow_profile::*;
