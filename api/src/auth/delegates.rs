use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use axum::http::StatusCode;
use mongodb::{
    Collection,
    bson::{doc, to_bson},
};
use std::{
    sync::LazyLock,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

use super::schemas::{AuthObject, UserOut};
use crate::{DB, apex::utils::VerboseHTTPError};

const COLLECTIONS_USERS: &str = "users";

static ARGON2: LazyLock<Argon2> = LazyLock::new(Argon2::default);

#[inline]
fn is_valid_password(pwd: &str) -> bool {
    let len = pwd.len();
    if len < 8 || len > 32 {
        return false;
    }

    let (upper, lower, digit, symbol) =
        pwd.chars()
            .fold((false, false, false, false), |(u, l, d, s), c| {
                (
                    u || c.is_ascii_uppercase(),
                    l || c.is_ascii_lowercase(),
                    d || c.is_ascii_digit(),
                    s || !c.is_ascii_alphanumeric(),
                )
            });

    upper && lower && digit && symbol
}

pub async fn hash_password(password: String) -> Option<(String, String)> {
    if !is_valid_password(&password) {
        return None;
    }
    let salt = SaltString::generate(&mut OsRng);

    tokio::task::spawn_blocking(move || {
        ARGON2
            .hash_password(password.as_bytes(), &salt)
            .ok()
            .map(|hash| (hash.to_string(), salt.to_string()))
    })
    .await
    .ok()
    .flatten()
}

pub async fn verify_password(
    plaintext_password: String,
    salt: String,
    hashed_password: String,
) -> bool {
    tokio::task::spawn_blocking(move || {
        SaltString::from_b64(&salt)
            .ok()
            .and_then(|salt_string| {
                ARGON2
                    .hash_password(plaintext_password.as_bytes(), &salt_string)
                    .ok()
                    .map(|hash| hash.to_string() == hashed_password)
            })
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}

pub async fn generate_cookie(username: String) -> Option<AuthObject> {
    let database = DB.get()?;
    let collection: Collection<UserOut> = database.collection("users");

    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() + 15_552_000;

    let auth_object = AuthObject {
        cookie: Uuid::new_v4().to_string(),
        cookie_expire: now.to_string(),
    };

    collection
        .update_one(
            doc! { "username": username },
            doc! { "$set": { "auth": to_bson(&auth_object).ok()? } },
        )
        .await
        .ok()?;

    Some(auth_object)
}

pub async fn kill_cookie(cookie: String) -> bool {
    let Some(database) = DB.get() else {
        return false;
    };
    let collection: Collection<UserOut> = database.collection("users");

    let auth_object = AuthObject {
        cookie: Uuid::new_v4().to_string(),
        cookie_expire: "0".to_string(),
    };

    let Some(auth_bson) = to_bson(&auth_object).ok() else {
        return false;
    };

    collection
        .update_one(
            doc! { "auth.cookie": cookie },
            doc! { "$set": { "auth": auth_bson } },
        )
        .await
        .is_ok()
}

pub async fn check_user_existence(username: &str, email: &str) -> Option<(bool, bool)> {
    let database = DB.get()?;
    let collection: Collection<UserOut> = database.collection("users");

    let username_exists = collection
        .find_one(doc! { "username": username })
        .await
        .ok()
        .flatten()
        .is_some();

    let email_hash = super::schemas::create_email_hash(email);
    let email_exists = collection
        .find_one(doc! { "email_hash": &email_hash })
        .await
        .ok()
        .flatten()
        .is_some();

    Some((username_exists, email_exists))
}

pub async fn retrieve_user_by_username_or_email(
    username: Option<&str>,
    email: Option<&str>,
) -> Option<UserOut> {
    let database = DB.get()?;
    let collection: Collection<UserOut> = database.collection("users");

    if let Some(username) = username {
        if let Some(user) = collection
            .find_one(doc! { "username": username })
            .await
            .ok()
            .flatten()
        {
            let _ = user.initialize_encryption();
            return Some(user);
        }
    }

    if let Some(email) = email {
        let email_hash = super::schemas::create_email_hash(email);
        if let Some(user) = collection
            .find_one(doc! { "email_hash": &email_hash })
            .await
            .ok()
            .flatten()
        {
            let _ = user.initialize_encryption();
            return Some(user);
        }
    }

    None
}

pub async fn change_password(
    user: &UserOut,
    old_password: String,
    new_password: String,
) -> Result<super::schemas::ChangePasswordResponse, VerboseHTTPError> {
    use argon2::{
        Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString,
    };

    if !argon2::Argon2::default()
        .verify_password(
            old_password.as_bytes(),
            &PasswordHash::new(&user.password).unwrap(),
        )
        .is_ok()
    {
        return Err(VerboseHTTPError::Standard(
            StatusCode::UNAUTHORIZED,
            "Current password is incorrect".to_string(),
        ));
    }

    let new_salt = SaltString::generate(&mut OsRng);
    let new_password_hash = Argon2::default()
        .hash_password(new_password.as_bytes(), &new_salt)
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to hash new password".to_string(),
            )
        })?
        .to_string();

    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    let collection: Collection<UserOut> = database.collection(COLLECTIONS_USERS);

    collection
        .update_one(
            doc! { "uid": &user.uid },
            doc! {
                "$set": {
                    "password": &new_password_hash,
                    "salt": new_salt.as_str()
                }
            },
        )
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update password".to_string(),
            )
        })?;

    Ok(super::schemas::ChangePasswordResponse {
        success: true,
        message: "Password changed successfully".to_string(),
    })
}

use rand::Rng;
use sha2::{Digest, Sha256};

const COLLECTIONS_OTP_VERIFICATIONS: &str = "otp_verifications";
const OTP_EXPIRY_MINUTES: u64 = 10;
const MAX_OTP_ATTEMPTS: u32 = 5;

fn generate_otp() -> String {
    let mut rng = rand::thread_rng();
    (0..6).map(|_| rng.gen_range(0..10).to_string()).collect()
}

fn hash_otp(otp: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{}{}", otp, salt));
    format!("{:x}", hasher.finalize())
}

pub async fn send_email_otp(email: &str) -> Result<(), VerboseHTTPError> {
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    let users: Collection<UserOut> = database.collection(COLLECTIONS_USERS);

    if let Ok(Some(user)) = users
        .find_one(doc! { "email_hash": super::schemas::create_email_hash(email) })
        .await
    {
        if user.email_verified {
            return Err(VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Email already verified".to_string(),
            ));
        }
    } else {
    }

    let otp = generate_otp();
    let salt = Uuid::new_v4().to_string();
    let otp_hash = hash_otp(&otp, &salt);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let expires_at = now + (OTP_EXPIRY_MINUTES * 60);

    let verification = super::schemas::OTPVerification {
        identifier: email.to_string(),
        otp_hash: format!("{}:{}", otp_hash, salt),
        created_at: now,
        expires_at,
        attempts: 0,
        verification_type: "email".to_string(),
    };

    let otps: Collection<super::schemas::OTPVerification> =
        database.collection(COLLECTIONS_OTP_VERIFICATIONS);

    let _ = otps
        .delete_many(doc! { "identifier": email, "verification_type": "email" })
        .await;

    otps.insert_one(&verification).await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to store OTP".to_string(),
        )
    })?;

    match crate::notifications::delegates::send_email_internal(
        email,
        None,
        "Email Verification - GoodsPoint",
        &format!("Your verification code is: {}", otp),
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

pub async fn verify_email_otp(email: &str, otp: &str) -> Result<(), VerboseHTTPError> {
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    let otps: Collection<super::schemas::OTPVerification> =
        database.collection(COLLECTIONS_OTP_VERIFICATIONS);

    let verification = otps
        .find_one(doc! { "identifier": email, "verification_type": "email" })
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or_else(|| {
            VerboseHTTPError::Standard(
                StatusCode::NOT_FOUND,
                "No verification request found".to_string(),
            )
        })?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if now > verification.expires_at {
        let _ = otps
            .delete_one(doc! { "identifier": email, "verification_type": "email" })
            .await;
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "OTP expired".to_string(),
        ));
    }

    if verification.attempts >= MAX_OTP_ATTEMPTS {
        let _ = otps
            .delete_one(doc! { "identifier": email, "verification_type": "email" })
            .await;
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Too many attempts".to_string(),
        ));
    }

    let parts: Vec<&str> = verification.otp_hash.split(':').collect();

    if parts.len() != 2 {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid OTP format".to_string(),
        ));
    }

    let stored_hash = parts[0];
    let salt = parts[1];
    let provided_hash = hash_otp(otp, salt);

    if provided_hash != stored_hash {
        let _ = otps
            .update_one(
                doc! { "identifier": email, "verification_type": "email" },
                doc! { "$inc": { "attempts": 1 } },
            )
            .await;

        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Invalid OTP".to_string(),
        ));
    }

    let users: Collection<UserOut> = database.collection(COLLECTIONS_USERS);

    match users
        .update_one(
            doc! { "email_hash": super::schemas::create_email_hash(email) },
            doc! { "$set": { "email_verified": true } },
        )
        .await
    {
        Ok(_) => {}
        Err(_) => {
            return Err(VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to verify email".to_string(),
            ));
        }
    }

    let _ = otps
        .delete_one(doc! { "identifier": email, "verification_type": "email" })
        .await;

    Ok(())
}

pub async fn send_whatsapp_otp(whatsapp_number: &str) -> Result<(), VerboseHTTPError> {
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    let users: Collection<UserOut> = database.collection(COLLECTIONS_USERS);
    let mut whatsapp_already_verified = false;

    if let Ok(mut cursor) = users.find(doc! {}).await {
        use futures::TryStreamExt;
        while let Ok(Some(user)) = cursor.try_next().await {
            if let Some(ref whatsapp) = user.whatsapp_number {
                if user.whatsapp_verified && whatsapp.to_string() == whatsapp_number {
                    whatsapp_already_verified = true;
                    break;
                }
            }
        }
    }

    if whatsapp_already_verified {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "WhatsApp number already verified".to_string(),
        ));
    }

    let otp = generate_otp();
    let salt = Uuid::new_v4().to_string();
    let otp_hash = hash_otp(&otp, &salt);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let expires_at = now + (OTP_EXPIRY_MINUTES * 60);

    let verification = super::schemas::OTPVerification {
        identifier: whatsapp_number.to_string(),
        otp_hash: format!("{}:{}", otp_hash, salt),
        created_at: now,
        expires_at,
        attempts: 0,
        verification_type: "whatsapp".to_string(),
    };

    let otps: Collection<super::schemas::OTPVerification> =
        database.collection(COLLECTIONS_OTP_VERIFICATIONS);

    let _ = otps
        .delete_many(doc! { "identifier": whatsapp_number, "verification_type": "whatsapp" })
        .await;

    otps.insert_one(&verification).await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to store OTP".to_string(),
        )
    })?;

    crate::notifications::delegates::send_whatsapp_internal(
        whatsapp_number,
        &format!("Your GoodsPoint verification code is: {}", otp),
    )
    .await?;

    Ok(())
}

pub async fn verify_whatsapp_otp(
    user: &UserOut,
    whatsapp_number: &str,
    otp: &str,
) -> Result<(), VerboseHTTPError> {
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    let otps: Collection<super::schemas::OTPVerification> =
        database.collection(COLLECTIONS_OTP_VERIFICATIONS);

    let verification = otps
        .find_one(doc! { "identifier": whatsapp_number, "verification_type": "whatsapp" })
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or_else(|| {
            VerboseHTTPError::Standard(
                StatusCode::NOT_FOUND,
                "No verification request found".to_string(),
            )
        })?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if now > verification.expires_at {
        let _ = otps
            .delete_one(doc! { "identifier": whatsapp_number, "verification_type": "whatsapp" })
            .await;
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "OTP expired".to_string(),
        ));
    }

    if verification.attempts >= MAX_OTP_ATTEMPTS {
        let _ = otps
            .delete_one(doc! { "identifier": whatsapp_number, "verification_type": "whatsapp" })
            .await;
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Too many attempts".to_string(),
        ));
    }

    let parts: Vec<&str> = verification.otp_hash.split(':').collect();
    if parts.len() != 2 {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid OTP format".to_string(),
        ));
    }

    let stored_hash = parts[0];
    let salt = parts[1];
    let provided_hash = hash_otp(otp, salt);

    if provided_hash != stored_hash {
        let _ = otps
            .update_one(
                doc! { "identifier": whatsapp_number, "verification_type": "whatsapp" },
                doc! { "$inc": { "attempts": 1 } },
            )
            .await;

        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Invalid OTP".to_string(),
        ));
    }

    let encrypted_whatsapp = super::schemas::EncryptedString::new(whatsapp_number, &user.salt)
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to encrypt WhatsApp number".to_string(),
            )
        })?;

    let users: Collection<UserOut> = database.collection(COLLECTIONS_USERS);
    users
        .update_one(
            doc! { "uid": &user.uid },
            doc! {
                "$set": {
                    "whatsapp_number": to_bson(&encrypted_whatsapp).unwrap(),
                    "whatsapp_verified": true
                }
            },
        )
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to verify WhatsApp number".to_string(),
            )
        })?;

    let _ = otps
        .delete_one(doc! { "identifier": whatsapp_number, "verification_type": "whatsapp" })
        .await;

    Ok(())
}
