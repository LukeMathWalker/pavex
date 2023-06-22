use crate::helpers::TestApi;
use pavex::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn signup_works() {
    let api = TestApi::spawn().await;

    let response = api.post_signup(&json!({
        "user": {
            "username": "Ursula",
            "email": "ursulaleguin@scifi.com",
            "password": "earthsea",
        }
    }))
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
}
