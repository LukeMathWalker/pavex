use pavex_builder::{f, AppBlueprint, Lifecycle};

pub fn constructor_with_output_tuple() -> (usize, isize) {
    todo!()
}

pub fn handler_with_input_tuple(input: (usize, isize)) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.constructor(
        f!(crate::constructor_with_output_tuple),
        Lifecycle::Singleton,
    );
    bp.route(f!(crate::handler_with_input_tuple), "/home");
    bp
}
