use pavex::{blueprint::from, Blueprint};

#[pavex::delete(path = "/delete")]
pub fn delete() -> pavex::Response {
    todo!()
}

#[pavex::get(path = "/get")]
pub fn get() -> pavex::Response {
    todo!()
}

#[pavex::head(path = "/head")]
pub fn head() -> pavex::Response {
    todo!()
}

#[pavex::options(path = "/options")]
pub fn options() -> pavex::Response {
    todo!()
}

#[pavex::patch(path = "/patch")]
pub fn patch() -> pavex::Response {
    todo!()
}

#[pavex::post(path = "/post")]
pub fn post() -> pavex::Response {
    todo!()
}

#[pavex::put(path = "/put")]
pub fn put() -> pavex::Response {
    todo!()
}

#[pavex::route(method = "TRACE", path = "/trace")]
pub fn trace() -> pavex::Response {
    todo!()
}

#[pavex::route(method = "CONNECT", path = "/connect")]
pub fn connect() -> pavex::Response {
    todo!()
}

#[pavex::route(method = ["PATCH", "POST"], path = "/mixed")]
pub fn mixed() -> pavex::Response {
    todo!()
}

#[pavex::route(method = "CUSTOM", path = "/custom", allow(non_standard_methods))]
pub fn custom() -> pavex::Response {
    todo!()
}

#[pavex::route(method = ["CUSTOM", "HEY"], path = "/mixed_custom", allow(non_standard_methods))]
pub fn mixed_custom() -> pavex::Response {
    todo!()
}

#[pavex::route(path = "/any", allow(any_method))]
pub fn any() -> pavex::Response {
    todo!()
}

#[pavex::route(path = "/any_with_extensions", allow(any_method, non_standard_methods))]
pub fn any_with_extensions() -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]);
    bp
}
