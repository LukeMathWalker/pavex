use pavex_builder::{f, AppBlueprint, Lifecycle};

pub fn static_str() -> &'static str {
    todo!()
}

pub fn handler(_x: &'static str) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.constructor(f!(crate::static_str), Lifecycle::Singleton);
    bp.route(f!(crate::handler), "/handler");
    bp
}
