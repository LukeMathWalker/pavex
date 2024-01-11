use pavex::middleware::Next;
use pavex::response::Response;
use std::future::IntoFuture;
use tokio::time::error::Elapsed;

pub async fn timeout<C>(next: Next<C>) -> Result<Response, Elapsed>
where
    C: IntoFuture<Output = Response>,
{
    let max_duration = std::time::Duration::from_secs(20);
    tokio::time::timeout(max_duration, next.into_future()).await
}
