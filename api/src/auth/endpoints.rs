use axum::{
    Json,
    body::Body,
    http::{
        Request, StatusCode,
        header::{COOKIE, SET_COOKIE},
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
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

    if let Some(ref email) = payload.email {
        if !EmailAddress::is_valid(email) {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Invalid email format".to_string(),
            )
            .into_response();
        }
    }

    let Some(user) =
        retrieve_user_by_username_or_email(payload.username.as_deref(), payload.email.as_deref())
            .await
    else {
        return VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Invalid username or password".to_string(),
        )
        .into_response();
    };

    if !verify_password(payload.password, user.salt.clone(), user.password.clone()).await {
        return VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Invalid username or password".to_string(),
        )
        .into_response();
    }

    if !user.email_verified {
        return VerboseHTTPError::Standard(
            StatusCode::FORBIDDEN,
            "Email not verified. Please verify your email before logging in.".to_string(),
        )
        .into_response();
    }

    let Some(auth_object) = generate_cookie(user.username.clone()).await else {
        return VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
        .into_response();
    };

    let expire_time =
        UNIX_EPOCH + Duration::from_secs(auth_object.cookie_expire.parse::<u64>().unwrap_or(0));
    let formatted_expire_time = fmt_http_date(SystemTime::from(expire_time));
    let domain = var("DOMAIN").unwrap_or_else(|_| ".goodspoint.com".to_string());

    let headers = [(
        SET_COOKIE,
        format!(
            "GOODSPOINT_AUTHENTICATION={}; HttpOnly; Path=/; Domain={}; expires={}",
            auth_object.cookie, domain, formatted_expire_time
        ),
    )];

    (headers, Json(json!({ "status": "ok" }))).into_response()
}

pub(crate) async fn register_user(Json(payload): Json<UserIn>) -> impl IntoResponse {
    if let Some(ref email) = payload.email {
        if !EmailAddress::is_valid(email) {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Invalid email format".to_string(),
            )
            .into_response();
        }
    }

    let Some((username_exists, email_exists)) = check_user_existence(
        payload.username.as_deref().unwrap_or(""),
        payload.email.as_deref().unwrap_or(""),
    )
    .await
    else {
        return VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
        .into_response();
    };

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

    let Some((hashed_password, salt)) = hash_password(payload.password).await else {
        return VerboseHTTPError::Standard(StatusCode::BAD_REQUEST, "Invalid password".to_string())
            .into_response();
    };

    let Some(auth_object) = generate_cookie(payload.username.clone().unwrap_or_default()).await
    else {
        return VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
        .into_response();
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

    let Some(database) = DB.get() else {
        return VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
        .into_response();
    };

    let collection: Collection<UserOut> = database.collection("users");

    if collection.insert_one(&user).await.is_err() {
        return VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
        .into_response();
    }

    if let Some(ref email) = payload.email {
        let _ = super::delegates::send_email_otp(email).await;
    }

    Json(json!({
        "status": "ok",
        "message": "Account created successfully. Please check your email for verification code.",
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

pub(crate) async fn get_whatsapp_status(req: Request<Body>) -> impl IntoResponse {
    if let Some(user) = req.extensions().get::<UserOut>() {
        return Json(json!({
            "whatsapp_verified": user.whatsapp_verified,
            "whatsapp_number": user.whatsapp_number.as_ref().map(|n| n.to_string()),
        }))
        .into_response();
    }

    VerboseHTTPError::Standard(StatusCode::UNAUTHORIZED, "Unauthorized".to_string()).into_response()
}

pub async fn cookie_auth(mut req: Request<Body>, next: Next) -> Result<Response, VerboseHTTPError> {
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    let collection: Collection<UserOut> = database.collection("users");

    if let Some(cookie_header) = req.headers().get(COOKIE).and_then(|h| h.to_str().ok()) {
        if let Some(cookie) = cookie_header.split(';').map(str::trim).find_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            match (parts.next(), parts.next()) {
                (Some("GOODSPOINT_AUTHENTICATION"), Some(value)) => Some(value.to_string()),
                _ => None,
            }
        }) {
            if let Some(user) = collection
                .find_one(doc! {"auth.cookie": &cookie})
                .await
                .ok()
                .flatten()
            {
                let _ = user.initialize_encryption();
                if let Ok(expire) = user.auth.cookie_expire.parse::<u64>() {
                    if SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_or(false, |now| expire > now.as_secs())
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

pub async fn change_password_endpoint(req: Request<Body>) -> impl IntoResponse {
    let Some(user) = req.extensions().get::<UserOut>().cloned() else {
        return VerboseHTTPError::Standard(StatusCode::UNAUTHORIZED, "Unauthorized".to_string())
            .into_response();
    };

    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Failed to read request body".to_string(),
            )
            .into_response();
        }
    };

    let request: super::schemas::ChangePasswordRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(_) => {
            return VerboseHTTPError::Standard(StatusCode::BAD_REQUEST, "Invalid JSON".to_string())
                .into_response();
        }
    };

    match super::delegates::change_password(&user, request.old_password, request.new_password).await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => error.into_response(),
    }
}

pub(crate) async fn send_email_otp_endpoint(
    Json(request): Json<super::schemas::SendEmailOTPRequest>,
) -> impl IntoResponse {
    match super::delegates::send_email_otp(&request.email).await {
        Ok(_) => {
            Json(json!({"success": true, "message": "OTP sent to email"})).into_response()
        }
        Err(error) => {
            error.into_response()
        }
    }
}

pub(crate) async fn verify_email_otp_endpoint(
    Json(request): Json<super::schemas::VerifyEmailOTPRequest>,
) -> impl IntoResponse {
    match super::delegates::verify_email_otp(&request.email, &request.otp).await {
        Ok(_) => {
            Json(json!({"success": true, "message": "Email verified successfully"})).into_response()
        }
        Err(error) => {
            error.into_response()
        }
    }
}

pub(crate) async fn send_whatsapp_otp_endpoint(req: Request<Body>) -> impl IntoResponse {
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Failed to read request body".to_string(),
            )
            .into_response();
        }
    };

    let request: super::schemas::SendWhatsAppOTPRequest = match serde_json::from_slice(&body_bytes)
    {
        Ok(req) => req,
        Err(_) => {
            return VerboseHTTPError::Standard(StatusCode::BAD_REQUEST, "Invalid JSON".to_string())
                .into_response();
        }
    };

    match super::delegates::send_whatsapp_otp(&request.whatsapp_number).await {
        Ok(_) => Json(json!({"success": true, "message": "OTP sent to WhatsApp"})).into_response(),
        Err(error) => error.into_response(),
    }
}

pub(crate) async fn verify_whatsapp_otp_endpoint(req: Request<Body>) -> impl IntoResponse {
    let Some(user) = req.extensions().get::<UserOut>().cloned() else {
        return VerboseHTTPError::Standard(StatusCode::UNAUTHORIZED, "Unauthorized".to_string())
            .into_response();
    };

    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Failed to read request body".to_string(),
            )
            .into_response();
        }
    };

    let request: super::schemas::VerifyWhatsAppOTPRequest =
        match serde_json::from_slice(&body_bytes) {
            Ok(req) => req,
            Err(_) => {
                return VerboseHTTPError::Standard(
                    StatusCode::BAD_REQUEST,
                    "Invalid JSON".to_string(),
                )
                .into_response();
            }
        };

    match super::delegates::verify_whatsapp_otp(&user, &request.whatsapp_number, &request.otp).await
    {
        Ok(_) => Json(json!({"success": true, "message": "WhatsApp verified successfully"}))
            .into_response(),
        Err(error) => error.into_response(),
    }
}
