use pavex::middleware::{Next, Processing};
use pavex::response::Response;
use pavex::{post_process, pre_process, wrap};

#[wrap]
pub async fn wrap1<C>(next: Next<C>) -> Response
where
    C: IntoFuture<Output = Response>,
{
    next.await
}

#[wrap]
pub async fn wrap2<C>(next: Next<C>) -> Response
where
    C: IntoFuture<Output = Response>,
{
    next.await
}

#[pre_process]
pub fn pre1() -> Processing {
    Processing::Continue
}

#[pre_process]
pub fn pre2() -> Processing {
    Processing::Continue
}

#[pre_process]
pub fn pre3() -> Processing {
    Processing::Continue
}

#[post_process]
pub fn post1(response: Response) -> Response {
    response
}

#[post_process]
pub fn post2(response: Response) -> Response {
    response
}
