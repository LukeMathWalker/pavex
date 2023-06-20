use crate::{
    jwt_auth::encode_token, routes::users::password::compute_password_hash, schemas::User,
};
use anyhow::Context;
use jsonwebtoken::EncodingKey;
use pavex::{
    extract::body::JsonBody,
    response::{
        body::{
            raw::{Bytes, Full},
            Json,
        },
        Response,
    },
};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

/// Create a new user.
pub async fn signup(
    body: JsonBody<Signup>,
    db_pool: &PgPool,
    jwt_key: &EncodingKey,
) -> Result<Response<Full<Bytes>>, SignupError> {
    let UserDetails {
        username,
        email,
        password,
    } = body.0.user;
    let password_hash = compute_password_hash(password).map_err(SignupError::UnexpectedError)?;
    let user_id = insert_user_record(&username, &email, &password_hash, db_pool)
        .await
        .map_err(SignupError::UnexpectedError)?;
    let token = encode_token(user_id, jwt_key).map_err(SignupError::UnexpectedError)?;

    let body = SignupResponse {
        user: User {
            email,
            token,
            username,
            bio: "".into(),
            image: "".into(),
        },
    };
    let body = Json::new(body)
        .map_err(Into::into)
        .map_err(SignupError::UnexpectedError)?;
    Ok(Response::created().set_typed_body(body))
}

#[derive(serde::Deserialize)]
pub struct Signup {
    pub user: UserDetails,
}

#[derive(serde::Deserialize)]
pub struct UserDetails {
    pub username: String,
    pub email: String,
    pub password: Secret<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignupResponse {
    pub user: User,
}

#[derive(Debug, thiserror::Error)]
pub enum SignupError {
    #[error("Something went wrong. Please retry later.")]
    UnexpectedError(#[source] anyhow::Error),
}

impl SignupError {
    pub fn into_response(&self) -> Response<Full<Bytes>> {
        match self {
            SignupError::UnexpectedError(_) => Response::internal_server_error(),
        }
        .set_typed_body(format!("{self}"))
    }
}

/// Insert a new user record in the database.
///
/// If all goes well, it returns the ID of the newly created user.
async fn insert_user_record(
    username: &str,
    email: &str,
    password_hash: &Secret<String>,
    pool: &PgPool,
) -> Result<uuid::Uuid, anyhow::Error> {
    let user_id = uuid::Uuid::new_v4();

    // If one wanted to be paranoid, logic should be introduced to
    // handle the case where the generated UUID is already in use.
    // This is extremely unlikely to happen in practice, but it's
    // still a possibility.
    //
    // It's left as an exercise to the reader.
    sqlx::query!(
        r#"
INSERT INTO users (id, username, email, password_hash)
VALUES ($1, $2, $3, $4)
"#,
        user_id,
        username,
        email,
        password_hash.expose_secret(),
    )
    .execute(pool)
    .await
    .context("Failed to insert user record.")?;

    Ok(user_id)
}
