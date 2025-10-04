use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    dep::dep_blueprint(&mut bp);
    bp
}
