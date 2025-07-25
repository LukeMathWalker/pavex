use pavex::{blueprint::from, Blueprint};

#[pavex::get(path = "/id")]
pub fn handler() -> pavex::Response {
    todo!()
}

#[pavex::post(path = "/users/yo")]
pub fn post_handler() -> pavex::Response {
    todo!()
}

#[pavex::fallback]
pub fn fallback1() -> pavex::Response {
    todo!()
}

pub fn fallback2() -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prefix("/users").nest({
        let mut bp = Blueprint::new();
        bp.routes(from![crate]);
        bp.fallback(FALLBACK_1);
        bp
    });
    bp.routes(from![crate]);
    bp
}
