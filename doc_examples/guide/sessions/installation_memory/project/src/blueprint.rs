use pavex::{blueprint::Blueprint, f};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(
        <pavex::cookie::ProcessorConfig as std::default::Default>::default
    ));
    bp.prefix("/in_memory").nest(crate::in_memory::blueprint());
    bp
}
