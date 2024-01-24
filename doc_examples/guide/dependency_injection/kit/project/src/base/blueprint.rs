use pavex::blueprint::Blueprint;
use pavex::kit::ApiKit;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    ApiKit::new().register(&mut bp);
    bp
}
