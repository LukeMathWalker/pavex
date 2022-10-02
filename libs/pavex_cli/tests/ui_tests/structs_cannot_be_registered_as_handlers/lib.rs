use pavex_builder::{f, AppBlueprint};

pub struct Streamer;

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new().route(f!(crate::Streamer), "/home")
}
