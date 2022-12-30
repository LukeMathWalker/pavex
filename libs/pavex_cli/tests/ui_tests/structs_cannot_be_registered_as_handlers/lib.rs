use pavex_builder::{f, AppBlueprint};

pub struct Streamer;

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.route(f!(crate::Streamer), "/home");
    bp
}
