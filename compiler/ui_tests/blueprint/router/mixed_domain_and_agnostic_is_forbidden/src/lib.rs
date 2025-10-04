use pavex::Blueprint;

#[pavex::get(path = "/")]
pub fn domain_agnostic() -> String {
    todo!()
}

#[pavex::get(path = "/")]
pub fn domain_specific() -> String {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    // Domain-specific
    bp.domain("company.com").nest({
        let mut bp = Blueprint::new();
        bp.route(DOMAIN_SPECIFIC);
        bp
    });
    // Domain-agnostic
    bp.route(DOMAIN_AGNOSTIC);
    bp
}
