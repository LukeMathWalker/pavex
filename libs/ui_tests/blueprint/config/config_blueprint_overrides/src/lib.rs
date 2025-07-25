use pavex::Response;
use pavex::{blueprint::from, Blueprint};

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[pavex::config(key = "a", id = "CONFIG_A")]
pub struct A;

#[pavex::get(path = "/")]
pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.config(CONFIG_A).default_if_missing().include_if_unused();
    bp.routes(from![crate]);
    bp
}
