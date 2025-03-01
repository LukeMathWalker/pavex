use pavex::blueprint::{router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

#[derive(Clone)]
pub struct A<'a> {
    pub a: &'a str,
}

#[derive(Clone)]
/// One generic parameter
pub struct B<T>(T);

#[derive(Clone)]
/// More than one lifetime
pub struct C<'a, 'b, 'c> {
    pub a: &'a str,
    pub b: &'b str,
    pub c: &'c str,
}

#[derive(Clone)]
/// More than one generic parameter
pub struct D<T, S, Z>(T, S, Z);

#[derive(Clone)]
#[allow(dead_code)]
pub struct E<'a>(std::borrow::Cow<'a, str>);

#[derive(Clone)]
#[allow(dead_code)]
pub struct F<'a, 'b>(std::borrow::Cow<'a, str>, &'b str);

pub fn handler(
    _a: A,
    _b: B<String>,
    _c: C,
    _d: D<String, u16, u64>,
    _e: E<'static>,
    _f: F<'static, '_>,
) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.config("a", t!(crate::A));
    bp.config("b", t!(crate::B));
    bp.config("c", t!(crate::C));
    bp.config("d", t!(crate::D));
    // We constrain the lifetime to 'static, so
    // that there are no unconstrained lifetimes in `E`.
    bp.config("e", t!(crate::E<'static>));
    // Some static, some elided.
    bp.config("f", t!(crate::F<'static, '_>));
    // Key is not a valid Rust identifier
    bp.config("12c", t!(std::string::String));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
