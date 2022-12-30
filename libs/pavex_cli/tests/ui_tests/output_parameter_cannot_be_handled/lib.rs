use pavex_builder::{f, AppBlueprint};

pub fn c() -> (usize, usize) {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.route(f!(crate::c), "/home");
    bp
}
