use pavex_builder::{f, AppBlueprint, Lifecycle};

pub struct Logger<T>(T);

pub fn new_logger<T>() -> Logger<T> {
    todo!()
}

pub fn handler<T>(logger: Logger<T>) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.constructor(f!(crate::new_logger::<String>), Lifecycle::Singleton);
    bp.route(f!(crate::handler::<std::string::String>), "/home");
    bp
}
