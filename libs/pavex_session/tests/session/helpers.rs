use std::collections::HashMap;

use pavex::cookie::ResponseCookie;
use pavex_session::SessionId;

/// Parse the response cookie created by finalizing the session
pub struct SetCookie {
    pub id: SessionId,
    pub client_state: HashMap<String, serde_json::Value>,
}

impl SetCookie {
    pub fn parse(cookie: ResponseCookie<'static>) -> Self {
        let mut cookie_values: HashMap<u8, serde_json::Value> =
            serde_json::from_str(cookie.value()).unwrap();
        let id: SessionId = serde_json::from_value(cookie_values[&0].clone()).unwrap();
        let client_state = if let Some(value) = cookie_values.remove(&1) {
            serde_json::from_value(value).unwrap()
        } else {
            HashMap::new()
        };
        Self { id, client_state }
    }

    pub fn id(&self) -> uuid::Uuid {
        self.id.inner()
    }
}
