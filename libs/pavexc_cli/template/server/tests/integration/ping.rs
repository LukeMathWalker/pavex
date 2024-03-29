use crate::helpers::TestApi;
use pavex::http::StatusCode;

#[tokio::test]
async fn ping_works() {
    let api = TestApi::spawn().await;

    let response = api.get_ping().await;

    assert_eq!(response.status(), StatusCode::OK);
}
