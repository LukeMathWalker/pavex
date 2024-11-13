use pavex::blueprint::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest(crate::intro::bp());
    bp.prefix("/deep").nest(crate::deep::bp());
    bp.prefix("/consecutive").nest(crate::consecutive::bp());
    bp
}
