use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prefix("/homes").nest(homes_bp());
    bp
}

pub fn homes_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/", f!(super::list_homes));
    bp.prefix("/{home_id}").nest(home_bp());
    bp
}

pub fn home_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/", f!(super::get_home));
    bp.prefix("/rooms").nest(rooms_bp());
    bp
}

pub fn rooms_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/", f!(super::list_rooms));
    bp.route(GET, "/{room_id}", f!(super::get_room));
    bp
}
