use pavex_builder::{reflection::RawCallable, router::POST, Blueprint};

pub fn my_f() {}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    let callable = RawCallable {
        import_path: "my_f,",
    };
    bp.route(POST, "/home", callable);
    bp
}
