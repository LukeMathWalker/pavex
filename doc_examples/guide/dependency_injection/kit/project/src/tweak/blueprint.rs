use pavex::blueprint::constructor::CloningStrategy;
use pavex::blueprint::Blueprint;
use pavex::kit::ApiKit;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    let mut kit = ApiKit::new();
    kit.buffered_body = kit
        .buffered_body
        .map(|b| b.cloning(CloningStrategy::CloneIfNecessary));
    kit.register(&mut bp);
    bp
}
