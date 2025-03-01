use pavex::blueprint::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest(crate::base::blueprint());
    crate::server::register(&mut bp);
    bp
}
