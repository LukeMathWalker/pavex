use pavex_builder::{AppBlueprint, RawCallable};

pub fn my_f() {}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    let callable = RawCallable {
        callable: my_f,
        import_path: "my_f,",
    };
    bp.route(callable, "/home");
    bp
}
