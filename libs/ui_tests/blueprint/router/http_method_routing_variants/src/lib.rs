use pavex::{blueprint::from, Blueprint};

#[pavex::delete(path = "/delete")]
pub fn delete() -> pavex::response::Response {
    todo!()
}

#[pavex::get(path = "/get")]
pub fn get() -> pavex::response::Response {
    todo!()
}

#[pavex::head(path = "/head")]
pub fn head() -> pavex::response::Response {
    todo!()
}

#[pavex::options(path = "/options")]
pub fn options() -> pavex::response::Response {
    todo!()
}

#[pavex::patch(path = "/patch")]
pub fn patch() -> pavex::response::Response {
    todo!()
}

#[pavex::post(path = "/post")]
pub fn post() -> pavex::response::Response {
    todo!()
}

#[pavex::put(path = "/put")]
pub fn put() -> pavex::response::Response {
    todo!()
}

#[pavex::route(method = "TRACE", path = "/trace")]
pub fn trace() -> pavex::response::Response {
    todo!()
}

#[pavex::route(method = "CONNECT", path = "/connect")]
pub fn connect() -> pavex::response::Response {
    todo!()
}

#[pavex::route(method = ["PATCH", "POST"], path = "/mixed")]
pub fn mixed() -> pavex::response::Response {
    todo!()
}

#[pavex::route(method = "CUSTOM", path = "/custom", allow(non_standard_methods))]
pub fn custom() -> pavex::response::Response {
    todo!()
}

#[pavex::route(method = ["CUSTOM", "HEY"], path = "/mixed_custom", allow(non_standard_methods))]
pub fn mixed_custom() -> pavex::response::Response {
    todo!()
}

#[pavex::route(path = "/any", allow(any_method))]
pub fn any() -> pavex::response::Response {
    todo!()
}

#[pavex::route(path = "/any_with_extensions", allow(any_method, non_standard_methods))]
pub fn any_with_extensions() -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]);
    bp
}
