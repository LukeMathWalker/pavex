use pavex_builder::{f, router::GET, Blueprint, Lifecycle};

pub struct Logger<T>(T);

pub fn new_logger<T>() -> Logger<T> {
    todo!()
}

pub fn handler<T>(logger: Logger<T>) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::new_logger::<String>), Lifecycle::Singleton);
    bp.route(GET, "/home", f!(crate::handler::<std::string::String>));
    bp
}
