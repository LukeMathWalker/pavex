use pavex_session::SessionConfig;

mod cookie;
mod state;

#[test]
fn all_fields_have_a_default_value() {
    assert!(serde_json::from_str::<SessionConfig>("{}").is_ok());
}
