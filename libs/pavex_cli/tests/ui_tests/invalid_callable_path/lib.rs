use pavex_builder::AppBlueprint;

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new().route("hello,", "/home")
}
