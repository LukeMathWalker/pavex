use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};

pub fn constructor_with_output_tuple() -> (usize, isize) {
    todo!()
}

pub fn handler_with_input_tuple(input: (usize, isize)) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(
        f!(crate::constructor_with_output_tuple),
        Lifecycle::Singleton,
    );
    bp.route(GET, "/home", f!(crate::handler_with_input_tuple));
    bp
}
