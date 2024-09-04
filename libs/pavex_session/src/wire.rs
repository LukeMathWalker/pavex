use crate::SessionId;
use serde_json::Value;
use std::{borrow::Cow, collections::HashMap};

#[derive(serde::Deserialize, serde::Serialize)]
/// The schema for the session cookie value.
///
/// We rename field names to numbers to minimise the size of the payload.
pub(crate) struct WireClientState<'a> {
    #[serde(rename = "0")]
    pub(crate) session_id: SessionId,
    #[serde(rename = "1", skip_serializing_if = "HashMap::is_empty", default)]
    pub(crate) user_values: Cow<'a, HashMap<String, Value>>,
}
