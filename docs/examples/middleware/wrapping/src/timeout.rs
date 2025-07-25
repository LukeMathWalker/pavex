//! px:timeout
use pavex::Response;
use pavex::middleware::Next;
use pavex::wrap;
use tokio::time::error::Elapsed;

#[wrap]
pub async fn timeout<C>(next: Next<C>) -> Result<Response, Elapsed>
where
    C: IntoFuture<Output = Response>, // px::ann:1
{
    let max_duration = std::time::Duration::from_secs(20);
    tokio::time::timeout(max_duration, next.into_future()).await
}
