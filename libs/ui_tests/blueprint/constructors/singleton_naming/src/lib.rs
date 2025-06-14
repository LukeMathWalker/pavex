use pavex::blueprint::{from, Blueprint};
use pavex::f;

#[derive(Clone)]
/// When lowercased, `type` is a keyword in Rust.
/// Pavex needs to escape it.
pub struct Type;

impl Type {
    pub fn new() -> Self {
        todo!()
    }
}

#[derive(Clone)]
/// Pavex needs to assign different names depending on the parameters
/// used as `T`, if more than one instance if around.
pub struct Generic<T>(T);

impl<T> Generic<T> {
    pub fn new() -> Self {
        todo!()
    }
}

#[derive(Clone)]
pub struct Singleton;

impl Singleton {
    pub fn new() -> Self {
        todo!()
    }
}

pub mod a {
    #[derive(Clone)]
    /// Same name as the one above, so Pavex will
    /// have to include enough path segments to disambiguate.
    pub struct Singleton;

    impl Singleton {
        pub fn new() -> Self {
            todo!()
        }
    }
}

#[derive(Clone)]
/// Same type name as the singleton coming from another crate,
/// so Pavex will have to include the crate name to disambiguate.
pub struct CrossCrateConflict;

impl CrossCrateConflict {
    pub fn new() -> Self {
        todo!()
    }
}

#[pavex::get(path = "/")]
pub fn handler(
    _t: &Type,
    _g1: &Generic<String>,
    _g2: &Generic<u64>,
    _s1: &Singleton,
    _s2: &a::Singleton,
) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(crate::Type::new));
    bp.singleton(f!(crate::Singleton::new));
    bp.singleton(f!(crate::a::Singleton::new));
    bp.singleton(f!(crate::Generic::new));
    bp.singleton(f!(crate::CrossCrateConflict::new));
    bp.singleton(f!(dep::CrossCrateConflict::new));
    bp.routes(from![crate]);
    bp
}
