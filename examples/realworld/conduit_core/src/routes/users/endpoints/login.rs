use crate::{jwt_auth, schemas::User, routes::users::password};
use anyhow::Context;
use jsonwebtoken::EncodingKey;
use pavex::{
    extract::body::JsonBody,
    hyper::body::Bytes,
    response::{
        body::{raw::Full, Json},
        Response,
    },
};
use secrecy::Secret;
use sqlx::PgPool;
use uuid::Uuid;

/// Login for an existing user.
pub async fn login(
    body: JsonBody<LoginBody>,
    db_pool: &PgPool,
    jwt_key: &EncodingKey,
) -> Result<Response<Full<Bytes>>, LoginError> {
    let UserCredentials { email, password } = body.0.user;
    let user_id = password::validate_credentials(&email, password, db_pool)
        .await
        .map_err(|e| match e {
            password::AuthError::InvalidCredentials(_) => LoginError::InvalidCredentials(e.into()),
            password::AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
        })?;

    let user_record = get_user_by_id(&user_id, db_pool)
        .await
        .map_err(LoginError::UnexpectedError)?;
    let jwt_token = jwt_auth::encode_token(user_id, jwt_key)
        .map_err(LoginError::UnexpectedError)?;

    let body = LoginResponse {
        user: User {
            email: user_record.email,
            token: jwt_token,
            username: user_record.username,
            bio: user_record.bio.unwrap_or_default(),
            image: user_record.image.unwrap_or_default(),
        },
    };
    let body = Json::new(body)
        .map_err(Into::into)
        .map_err(LoginError::UnexpectedError)?;
    Ok(Response::ok().set_typed_body(body))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginBody {
    pub user: UserCredentials,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCredentials {
    pub email: String,
    pub password: Secret<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub user: User,
}

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error(transparent)]
    InvalidCredentials(anyhow::Error),
    #[error("Something went wrong. Please retry later.")]
    UnexpectedError(#[source] anyhow::Error),
}

impl LoginError {
    pub fn into_response(&self) -> Response<Full<Bytes>> {
        match self {
            LoginError::InvalidCredentials(_) => Response::unauthorized(),
            LoginError::UnexpectedError(_) => Response::internal_server_error(),
        }
        .set_typed_body(format!("{self}"))
    }
}

struct GetUserRecord {
    email: String,
    username: String,
    bio: Option<String>,
    image: Option<String>,
}

#[tracing::instrument(name = "Get user by id", skip_all)]
/// Retrieve a user from the database using its id.
///
/// It returns an error if the query fails or if the user doesn't exist.
async fn get_user_by_id(user_id: &Uuid, pool: &PgPool) -> Result<GetUserRecord, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT email, username, bio, image 
        FROM users
        WHERE id = $1
        "#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .with_context(|| format!("Failed to retrieve the user with id {user_id}"))?;
    Ok(GetUserRecord {
        email: row.email,
        username: row.username,
        bio: row.bio,
        image: row.image,
    })
}
