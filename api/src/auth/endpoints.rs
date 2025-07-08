use axum::{
    Json,
    body::Body,
    http::{
        Method, Request, StatusCode,
        header::{COOKIE, SET_COOKIE},
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_csrf::CsrfToken;
use email_address::EmailAddress;
use httpdate::fmt_http_date;
use mongodb::{Collection, bson::doc};
use serde_json::json;
use std::{
    env::var,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use super::{
    delegates::{
        check_user_existence, generate_cookie, hash_password, kill_cookie,
        retrieve_user_by_username_or_email, verify_password,
    },
    schemas::{UserIn, UserOut, UserQuery},
};
use crate::{DB, apex::utils::VerboseHTTPError};

pub(crate) async fn logout_user(req: Request<Body>) -> impl IntoResponse {
    if let Some(user) = req.extensions().get::<UserOut>() {
        if kill_cookie(user.auth.cookie.clone()).await {
            let domain = var("DOMAIN").unwrap_or_else(|_| ".goodspoint.com".to_string());
            let headers = [(
                SET_COOKIE,
                format!(
                    "GOODSPOINT_AUTHENTICATION=null; expires=Thu, 01 Jan 1970 00:00:00 GMT; Path=/; Domain={}; HttpOnly",
                    domain
                ),
            )];
            return (headers, Json(json!({ "status": "ok" }))).into_response();
        }
    }

    VerboseHTTPError::Standard(StatusCode::UNAUTHORIZED, "Unauthorized".to_string()).into_response()
}

pub(crate) async fn login_user(Json(payload): Json<UserIn>) -> impl IntoResponse {
    if payload.username.is_none() && payload.email.is_none() {
        return VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Missing credentials".to_string(),
        )
        .into_response();
    }

    if let Some(email) = &payload.email {
        if !EmailAddress::is_valid(email) {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Invalid email format".to_string(),
            )
            .into_response();
        }
    }

    if let Some(user) =
        retrieve_user_by_username_or_email(payload.username.as_deref(), payload.email.as_deref())
            .await
    {
        let salt = user.salt;
        if verify_password(payload.password.clone(), salt, user.password.clone()).await {
            if let Some(auth_object) = generate_cookie(user.username.clone()).await {
                let expire_time = UNIX_EPOCH
                    + Duration::from_secs(auth_object.cookie_expire.parse::<u64>().unwrap_or(0));
                let formatted_expire_time = fmt_http_date(SystemTime::from(expire_time));
                let domain = var("DOMAIN").unwrap_or_else(|_| ".goodspoint.com".to_string());

                let headers = [(
                    SET_COOKIE,
                    format!(
                        "GOODSPOINT_AUTHENTICATION={}; HttpOnly; Path=/; Domain={}; expires={}",
                        auth_object.cookie, domain, formatted_expire_time
                    ),
                )];

                return (headers, Json(json!({ "status": "ok" }))).into_response();
            }
        }
    }

    VerboseHTTPError::Standard(
        StatusCode::BAD_REQUEST,
        "Invalid username or password".to_string(),
    )
    .into_response()
}

pub(crate) async fn register_user(Json(payload): Json<UserIn>) -> impl IntoResponse {
    if let Some(email) = &payload.email {
        if !EmailAddress::is_valid(email) {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Invalid email format".to_string(),
            )
            .into_response();
        }
    }

    if let Some((username_exists, email_exists)) = check_user_existence(
        payload.username.as_deref().unwrap_or(""),
        payload.email.as_deref().unwrap_or(""),
    )
    .await
    {
        if username_exists && email_exists {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Username and email already taken".to_string(),
            )
            .into_response();
        } else if username_exists {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Username already taken".to_string(),
            )
            .into_response();
        } else if email_exists {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Email already taken".to_string(),
            )
            .into_response();
        }
    }

    let (hashed_password, salt) = match hash_password(payload.password).await {
        Some((hash, salt)) => (hash, salt),
        None => {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Invalid password".to_string(),
            )
            .into_response();
        }
    };

    let auth_object = match generate_cookie(payload.username.clone().unwrap_or_default()).await {
        Some(auth) => auth,
        None => {
            return VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            )
            .into_response();
        }
    };

    let user = match UserOut::new(
        payload.username.clone().unwrap_or_default(),
        payload.email.clone().unwrap_or_default(),
        hashed_password,
        salt,
        auth_object,
        uuid::Uuid::new_v4().to_string(),
        true,
    ) {
        Ok(user) => user,
        Err(_) => {
            return VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create user with encryption".to_string(),
            )
            .into_response();
        }
    };

    let database = DB.get().unwrap();
    let collection: Collection<UserOut> = database.collection("users");

    if collection.insert_one(&user).await.is_err() {
        return VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
        .into_response();
    }

    Json(json!({
        "status": "ok",
        "user": UserQuery {
            username: Some(user.username.clone()),
            email: Some(user.email.to_string()),
            uid: Some(user.uid.clone()),
        }
    }))
    .into_response()
}

pub(crate) async fn get_user(req: Request<Body>) -> impl IntoResponse {
    if let Some(user) = req.extensions().get::<UserOut>() {
        let response = UserQuery {
            username: Some(user.username.clone()),
            email: Some(user.email.to_string()),
            uid: Some(user.uid.clone()),
        };
        return Json(json!({
            "user": response,
        }))
        .into_response();
    }

    VerboseHTTPError::Standard(StatusCode::UNAUTHORIZED, "Unauthorized".to_string()).into_response()
}

pub async fn cookie_auth(mut req: Request<Body>, next: Next) -> Result<Response, VerboseHTTPError> {
    let database = DB.get().unwrap();
    let collection: Collection<UserOut> = database.collection("users");

    if let Some(cookie_header) = req.headers().get(COOKIE).and_then(|h| h.to_str().ok()) {
        if let Some(cookie) = cookie_header.split(';').map(|s| s.trim()).find_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            if let (Some(name), Some(value)) = (parts.next(), parts.next()) {
                if name == "GOODSPOINT_AUTHENTICATION" {
                    return Some(value.to_string());
                }
            }
            None
        }) {
            if let Some(user) = collection
                .find_one(doc! {"auth.cookie": cookie.clone()})
                .await
                .ok()
                .flatten()
            {
                let _ = user.initialize_encryption();
                if let Ok(expire) = user.auth.cookie_expire.parse::<u64>() {
                    if SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|now| expire > now.as_secs())
                        .unwrap_or(false)
                    {
                        req.extensions_mut().insert(user);
                        return Ok(next.run(req).await);
                    }
                }
                kill_cookie(cookie).await;
            }
        }
    }

    Err(VerboseHTTPError::Standard(
        StatusCode::UNAUTHORIZED,
        "Unauthorized".to_string(),
    ))
}

#[allow(unused)]
pub async fn auth_middleware(
    token: CsrfToken,
    method: Method,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    if method == Method::POST {
        if let Some(authenticity_token) = request
            .headers()
            .get("x-authenticity-token")
            .and_then(|h| h.to_str().ok())
        {
            if token.verify(authenticity_token).is_err() {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "Invalid authenticity token".to_string(),
                )
                    .into_response());
            }
        } else {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Missing authenticity token".to_string(),
            )
                .into_response());
        }
    }

    Ok(next.run(request).await)
}
