use pavex::blueprint::{
    reflection::{CreatedAt, RawIdentifiers, WithLocation},
    router::POST,
    Blueprint,
};

pub fn my_f() {}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    let callable = WithLocation {
        value: RawIdentifiers {
            import_path: "my_f,",
            macro_name: "f",
        },
        created_at: CreatedAt {
            package_name: "app",
            package_version: "0.1.0",
            module_path: "app",
        },
    };
    bp.route(POST, "/home", callable);
    bp
}
