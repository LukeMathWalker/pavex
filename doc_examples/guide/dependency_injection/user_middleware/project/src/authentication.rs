use std::future::IntoFuture;

use pavex::middleware::Next;
use pavex::response::Response;

use crate::user::User;

pub async fn reject_anonymous<C>(user: &User, next: Next<C>) -> Response
    where
        C: IntoFuture<Output=Response>,
{
    if let User::Anonymous = user {
        return Response::unauthorized().box_body();
    }
    next.into_future().await
}
