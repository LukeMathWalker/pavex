//! px:basic
use pavex::middleware::Next;
use pavex::response::Response;
use pavex::wrap;

#[wrap]
pub async fn debug_wrapper<C>(next: Next<C>) -> Response
where
    C: IntoFuture<Output = Response>,
{
    println!("Before the handler");
    let response = next.await; // px::ann:1
    println!("After the handler");
    response
}
