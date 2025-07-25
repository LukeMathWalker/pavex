use pavex::Blueprint;

#[pavex::get(path = "/id")]
pub fn get_id() -> pavex::Response {
    todo!()
}

#[pavex::post(path = "/id")]
pub fn post_id() -> pavex::Response {
    todo!()
}

#[pavex::fallback]
pub fn get_fallback() -> pavex::Response {
    todo!()
}

#[pavex::fallback]
pub fn post_fallback() -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest({
        let mut bp = Blueprint::new();
        bp.route(GET_ID);
        bp.fallback(GET_FALLBACK);
        bp
    });
    bp.nest({
        let mut bp = Blueprint::new();
        bp.route(POST_ID);
        bp.fallback(POST_FALLBACK);
        bp
    });
    bp
}
