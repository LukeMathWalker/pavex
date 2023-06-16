use pavex::blueprint::{reflection::RawCallable, router::POST, Blueprint};

pub fn my_f() {}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    let callable = RawCallable {
        import_path: "my_f,",
        registered_at: "app",
    };
    bp.route(POST, "/home", callable);
    bp
}
