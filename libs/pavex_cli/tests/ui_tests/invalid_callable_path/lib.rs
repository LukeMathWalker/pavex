use pavex_builder::{router::POST, Blueprint, RawCallable};

pub fn my_f() {}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    let callable = RawCallable {
        callable: my_f,
        import_path: "my_f,",
    };
    bp.route(POST, "/home", callable);
    bp
}
