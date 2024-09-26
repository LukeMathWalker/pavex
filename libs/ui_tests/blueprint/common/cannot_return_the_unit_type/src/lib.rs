use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

pub fn constructor() {
    todo!()
}

#[derive(Debug)]
pub struct Error;

pub fn fallible_unit_constructor() -> Result<(), Error> {
    todo!()
}

pub fn fallible_constructor() -> Result<String, Error> {
    todo!()
}

pub fn error_handler(e: &Error) {
    todo!()
}

pub fn handler() -> Response {
    todo!()
}

pub fn unit_wrapping() {
    todo!()
}

pub fn fallible_unit_wrapping() -> Result<(), Error> {
    todo!()
}

pub fn unit_pre() {
    todo!()
}

pub fn unit_post(_response: Response) {
    todo!()
}

pub fn fallible_unit_pre() -> Result<(), Error> {
    todo!()
}

pub fn fallible_unit_post(_response: Response) -> Result<(), Error> {
    todo!()
}

pub fn unit_handler() {
    todo!()
}

pub fn fallible_unit_handler() -> Result<(), Error> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(crate::constructor));
    bp.request_scoped(f!(crate::fallible_unit_constructor));
    bp.request_scoped(f!(crate::fallible_constructor))
        .error_handler(f!(crate::error_handler));

    bp.pre_process(f!(crate::unit_pre));
    bp.pre_process(f!(crate::fallible_unit_pre))
        .error_handler(f!(crate::error_handler));

    bp.wrap(f!(crate::unit_wrapping));
    bp.wrap(f!(crate::fallible_unit_wrapping))
        .error_handler(f!(crate::error_handler));

    bp.post_process(f!(crate::unit_post));
    bp.post_process(f!(crate::fallible_unit_post))
        .error_handler(f!(crate::error_handler));

    bp.route(GET, "/home", f!(crate::handler));
    bp.route(GET, "/unit", f!(crate::unit_handler));
    bp.route(GET, "/fallible_unit", f!(crate::fallible_unit_handler))
        .error_handler(f!(crate::error_handler));
    bp
}
