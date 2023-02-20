use pavex_builder::{f, Blueprint, Lifecycle};

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

pub fn handler() -> pavex_runtime::response::Response {
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
    bp.route(f!(crate::handler), "/home");
    bp
}
