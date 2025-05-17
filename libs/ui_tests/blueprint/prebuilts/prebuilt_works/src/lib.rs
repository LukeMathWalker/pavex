use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
pub struct B<T>(pub T);

#[derive(Clone)]
pub struct C<'a>(pub &'a str);

#[derive(Clone)]
#[pavex::prebuilt(clone_if_necessary)]
pub struct A1;

pub fn handler(_a: &A, _b: &B<String>, _c: C<'static>, _d: Vec<String>, _a1: &A1) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.prebuilt(t!(crate::A));
    bp.prebuilt(t!(crate::B<std::string::String>));
    bp.prebuilt(t!(crate::C<'static>)).clone_if_necessary();
    bp.prebuilt(t!(std::vec::Vec<std::string::String>))
        .clone_if_necessary();
    bp.route(GET, "/", f!(crate::handler));
    bp
}
