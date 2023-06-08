use pavex_builder::router::{DELETE, POST, PUT};
use pavex_builder::{f, router::GET, Blueprint};

pub(crate) fn articles_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "", f!(crate::api::articles::list_articles));
    bp.route(POST, "", f!(crate::api::articles::publish_article));
    bp.route(GET, "/feed", f!(crate::api::articles::get_feed));
    bp.route(GET, "/:slug", f!(crate::api::articles::get_article));
    bp.route(DELETE, "/:slug", f!(crate::api::articles::delete_article));
    bp.route(PUT, "/:slug", f!(crate::api::articles::update_article));
    bp.route(
        DELETE,
        "/:slug/favorite",
        f!(crate::api::articles::unfavorite_article),
    );
    bp.route(
        POST,
        "/:slug/favorite",
        f!(crate::api::articles::favorite_article),
    );
    bp.route(
        GET,
        "/:slug/comments",
        f!(crate::api::articles::list_comments),
    );
    bp.route(
        POST,
        "/:slug/comments",
        f!(crate::api::articles::publish_comment),
    );
    bp.route(
        DELETE,
        "/:slug/comments/:comment_id",
        f!(crate::api::articles::delete_comment),
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
