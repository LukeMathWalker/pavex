use pavex::blueprint::{constructor::CloningStrategy, router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
pub struct B<T>(T);

#[derive(Clone)]
pub struct C<'a>(&'a str);

pub fn handler(a: &A, b: &B<String>, c: C<'static>) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.state_input(t!(crate::A));
    bp.state_input(t!(crate::B<std::string::String>));
    bp.state_input(t!(crate::C<'static>))
        .cloning(CloningStrategy::CloneIfNecessary);
    bp.route(GET, "/", f!(crate::handler));
    bp
}
