use pavex_builder::{f, AppBlueprint, Lifecycle};

pub fn request_scoped() -> Result<String, ErrorType> {
    todo!()
}

#[derive(Debug)]
pub struct ErrorType;

// It does not implement IntoResponse!
pub struct MyCustomOutputType;

pub fn handler(_s: String) -> Result<MyCustomOutputType, ErrorType> {
    todo!()
}

pub fn error_handler(e: &ErrorType) -> MyCustomOutputType {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.constructor(f!(crate::request_scoped), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.route(f!(crate::handler), "/home")
        .error_handler(f!(crate::error_handler));
    bp
}
