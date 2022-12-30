use pavex_builder::{f, AppBlueprint, Lifecycle};

pub fn bogus_constructor() {
    todo!()
}

pub fn handler() -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.constructor(f!(crate::bogus_constructor), Lifecycle::Singleton);
    bp.route(f!(crate::handler), "/home");
    bp
}
