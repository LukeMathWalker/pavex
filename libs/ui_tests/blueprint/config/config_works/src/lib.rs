use pavex::blueprint::{router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct A;

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct B<T>(pub T);

pub fn handler(_a: &A, _b: &B<String>, _d: Vec<String>) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.config("a", t!(crate::A));
    bp.config("b", t!(crate::B<std::string::String>))
        .default_if_missing();
    bp.config("d", t!(std::vec::Vec<std::string::String>));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
