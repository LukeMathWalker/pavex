use pavex::blueprint::Blueprint;
use pavex::kit::ApiKit;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    let mut kit = ApiKit::new();
    kit.buffered_body = kit.buffered_body.map(|b| b.clone_if_necessary());
    kit.register(&mut bp);
    bp
}
