use pavex_builder::{f, AppBlueprint};

pub fn c() -> (usize, usize) {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new().route(f!(crate::c), "/home")
}
