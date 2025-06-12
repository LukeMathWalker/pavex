use pavex::blueprint::from;
use pavex::blueprint::Blueprint;
use pavex::response::Response;

#[pavex::singleton]
pub fn annotated_constructor() {
    todo!()
}

#[pavex::request_scoped]
pub fn fallible_annotated_unit_constructor() -> Result<(), Error> {
    todo!()
}

#[pavex::request_scoped]
pub fn fallible_annotated_constructor() -> Result<u64, Error> {
    todo!()
}

#[pavex::singleton]
pub fn constructor() {
    todo!()
}

#[derive(Debug)]
pub struct Error;

#[pavex::error_handler]
pub fn error_handler(_e: &Error) {
    todo!()
}

#[pavex::request_scoped]
pub fn fallible_unit_constructor() -> Result<(), Error> {
    todo!()
}

#[pavex::request_scoped]
pub fn fallible_constructor() -> Result<String, Error> {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler() -> Response {
    todo!()
}

#[pavex::wrap]
pub fn unit_wrapping() {
    todo!()
}

#[pavex::wrap]
pub fn fallible_unit_wrapping() -> Result<(), Error> {
    todo!()
}

#[pavex::pre_process]
pub fn unit_pre() {
    todo!()
}

#[pavex::post_process]
pub fn unit_post(_response: Response) {
    todo!()
}

#[pavex::pre_process]
pub fn fallible_unit_pre() -> Result<(), Error> {
    todo!()
}

#[pavex::post_process]
pub fn fallible_unit_post(_response: Response) -> Result<(), Error> {
    todo!()
}

#[pavex::get(path = "/unit")]
pub fn unit_handler() {
    todo!()
}

#[pavex::get(path = "/fallible_unit")]
pub fn fallible_unit_handler() -> Result<(), Error> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate, pavex]);

    bp.pre_process(UNIT_PRE);
    bp.pre_process(FALLIBLE_UNIT_PRE);

    bp.wrap(UNIT_WRAPPING);
    bp.wrap(FALLIBLE_UNIT_WRAPPING);

    bp.post_process(UNIT_POST);
    bp.post_process(FALLIBLE_UNIT_POST);

    bp.routes(from![crate]);
    bp
}
