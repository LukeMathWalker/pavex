use pavex::blueprint::{reflection::RawIdentifiers, router::POST, Blueprint};

pub fn my_f() {}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    let callable = RawIdentifiers {
        import_path: "my_f,",
        crate_name: "app",
        module_path: "app",
        macro_name: "f",
    };
    bp.route(POST, "/home", callable);
    bp
}
