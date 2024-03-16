use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

pub fn constructor() {
    todo!()
}

#[derive(Debug)]
pub struct Error;

pub fn fallible_constructor_building_unit() -> Result<(), Error> {
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

pub fn unit_wrapping_middleware() {
    todo!()
}

pub fn fallible_wrapping_middleware() -> Result<(), Error> {
    todo!()
}

pub fn unit_pp_middleware(_response: Response) {
    todo!()
}

pub fn fallible_pp_middleware(_response: Response) -> Result<(), Error> {
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
    bp.constructor(f!(crate::constructor), Lifecycle::Singleton);
    bp.constructor(
        f!(crate::fallible_constructor_building_unit),
        Lifecycle::RequestScoped,
    );
    bp.constructor(f!(crate::fallible_constructor), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));

    bp.wrap(f!(crate::unit_wrapping_middleware));
    bp.wrap(f!(crate::fallible_wrapping_middleware))
        .error_handler(f!(crate::error_handler));
    bp.post_process(f!(crate::unit_pp_middleware));
    bp.post_process(f!(crate::fallible_pp_middleware))
        .error_handler(f!(crate::error_handler));
    bp.route(GET, "/home", f!(crate::handler));
    bp.route(GET, "/unit", f!(crate::unit_handler));
    bp.route(GET, "/fallible_unit", f!(crate::fallible_unit_handler))
        .error_handler(f!(crate::error_handler));
    bp
}
