use pavex::blueprint::router::{DELETE, GET, POST, PUT};
use pavex::blueprint::Blueprint;
use pavex::f;

pub(crate) fn articles_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "", f!(crate::routes::articles::list_articles));
    bp.route(POST, "", f!(crate::routes::articles::publish_article));
    bp.route(GET, "/feed", f!(crate::routes::articles::get_feed));
    bp.route(GET, "/:slug", f!(crate::routes::articles::get_article));
    bp.route(DELETE, "/:slug", f!(crate::routes::articles::delete_article));
    bp.route(PUT, "/:slug", f!(crate::routes::articles::update_article));
    bp.route(
        DELETE,
        "/:slug/favorite",
        f!(crate::routes::articles::unfavorite_article),
    );
    bp.route(
        POST,
        "/:slug/favorite",
        f!(crate::routes::articles::favorite_article),
    );
    bp.route(
        GET,
        "/:slug/comments",
        f!(crate::routes::articles::list_comments),
    );
    bp.route(
        POST,
        "/:slug/comments",
        f!(crate::routes::articles::publish_comment),
    );
    bp.route(
        DELETE,
        "/:slug/comments/:comment_id",
        f!(crate::routes::articles::delete_comment),
    );
    bp
}

mod delete_article;
mod delete_comment;
mod favorite_article;
mod get_article;
mod get_feed;
mod list_articles;
mod list_comments;
mod publish_article;
mod publish_comment;
mod unfavorite_article;
mod update_article;

pub use delete_article::*;
pub use delete_comment::*;
pub use favorite_article::*;
pub use get_article::*;
pub use get_feed::*;
pub use list_articles::*;
pub use list_comments::*;
pub use publish_article::*;
pub use publish_comment::*;
pub use unfavorite_article::*;
pub use update_article::*;
