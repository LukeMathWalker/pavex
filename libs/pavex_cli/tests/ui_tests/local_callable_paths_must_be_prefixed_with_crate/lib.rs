use pavex_builder::{f, AppBlueprint};

pub fn handler() -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.route(f!(handler), "/home");
    bp
}
