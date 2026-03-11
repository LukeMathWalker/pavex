use pavex::{blueprint::from, Blueprint};

#[pavex::request_scoped]
pub fn build_fn_pointer() -> fn(u32) -> u8 {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_f: fn(u32) -> u8) -> pavex::Response {
    todo!()
}

#[pavex::request_scoped]
pub fn build_fn_pointer_no_output() -> fn(u32) {
    todo!()
}

#[pavex::get(path = "/no_output")]
pub fn handler_no_output(_f: fn(u32)) -> pavex::Response {
    todo!()
}

#[pavex::request_scoped]
pub fn build_fn_pointer_no_input() -> fn() -> u8 {
    todo!()
}

#[pavex::get(path = "/no_input")]
pub fn handler_no_input(_f: fn() -> u8) -> pavex::Response {
    todo!()
}

#[pavex::request_scoped]
pub fn build_fn_pointer_no_input_no_output() -> fn() {
    todo!()
}

#[pavex::get(path = "/no_input_no_output")]
pub fn handler_no_input_no_output(_f: fn()) -> pavex::Response {
    todo!()
}

#[pavex::request_scoped]
pub fn build_fn_pointer_two_inputs() -> fn(u32, String) -> u8 {
    todo!()
}

#[pavex::get(path = "/two_inputs")]
pub fn handler_two_inputs(_f: fn(u32, String) -> u8) -> pavex::Response {
    todo!()
}

#[pavex::request_scoped]
pub fn build_unsafe_fn_pointer() -> unsafe fn(u32) -> u8 {
    todo!()
}

#[pavex::get(path = "/unsafe")]
pub fn handler_unsafe(_f: unsafe fn(u32) -> u8) -> pavex::Response {
    todo!()
}

#[pavex::request_scoped]
pub fn build_extern_c_fn_pointer() -> extern "C" fn(u32) -> u8 {
    todo!()
}

#[pavex::get(path = "/extern_c")]
pub fn handler_extern_c(_f: extern "C" fn(u32) -> u8) -> pavex::Response {
    todo!()
}

#[pavex::request_scoped]
pub fn build_unsafe_extern_c_fn_pointer() -> unsafe extern "C" fn(u32) -> u8 {
    todo!()
}

#[pavex::get(path = "/unsafe_extern_c")]
pub fn handler_unsafe_extern_c(_f: unsafe extern "C" fn(u32) -> u8) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
