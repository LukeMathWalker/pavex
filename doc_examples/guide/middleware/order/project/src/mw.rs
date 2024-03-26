use pavex::middleware::{Next, Processing};
use pavex::response::Response;
use std::future::IntoFuture;

pub async fn wrap1<C>(next: Next<C>) -> Response
where
    C: IntoFuture<Output = Response>,
{
    next.await
}

pub async fn wrap2<C>(next: Next<C>) -> Response
    where
        C: IntoFuture<Output = Response>,
{
    next.await
}

pub async fn pre1() -> Processing
{
    Processing::Continue
}

pub async fn pre2() -> Processing
{
    Processing::Continue
}

pub async fn pre3() -> Processing
{
    Processing::Continue
}

pub async fn post1(response: Response) -> Response
{
    response
}

pub async fn post2(response: Response) -> Response
{
    response
}
