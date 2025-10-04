use pavex::{blueprint::from, Blueprint};

#[derive(Clone)]
pub struct Singleton;

impl Default for Singleton {
    fn default() -> Self {
        Self::new()
    }
}

#[pavex::methods]
impl Singleton {
    #[singleton]
    pub fn new() -> Singleton {
        todo!()
    }
}

pub struct RequestScoped;

#[pavex::request_scoped]
pub fn request_scoped() -> RequestScoped {
    todo!()
}

pub struct Transient;

#[pavex::transient]
pub fn transient() -> Transient {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn stream_file(
    _s: &Singleton,
    _r: &RequestScoped,
    _t: &Transient,
) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
