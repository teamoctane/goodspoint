use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
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
use crate::DB;

static ARGON2: LazyLock<Argon2> = LazyLock::new(|| Argon2::default());

fn is_valid_password(pwd: &str) -> bool {
    let len = pwd.len();
    if !(8..=32).contains(&len) {
        return false;
    }
    let (upper, lower, digit, symbol) =
        pwd.chars()
            .fold((false, false, false, false), |(u, l, d, s), c| {
                (
                    u || c.is_uppercase(),
                    l || c.is_lowercase(),
                    d || c.is_ascii_digit(),
                    s || !c.is_alphanumeric(),
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
            .map(|hash| (hash.to_string(), salt.clone().to_string()))
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
    let database = DB.get().unwrap();
    let collection: Collection<UserOut> = database.collection("users");

    let now = SystemTime::now();
    let since_epoch = now.duration_since(UNIX_EPOCH).unwrap().as_secs() + 15_552_000;
    let auth_object = AuthObject {
        cookie: Uuid::new_v4().to_string(),
        cookie_expire: since_epoch.to_string(),
    };

    collection
        .update_one(
            doc! { "username": username },
            doc! { "$set": { "auth": to_bson(&auth_object).unwrap() } },
        )
        .await
        .ok()?;

    Some(auth_object)
}

pub async fn kill_cookie(cookie: String) -> bool {
    let database = DB.get().unwrap();
    let collection: Collection<UserOut> = database.collection("users");

    let auth_object = AuthObject {
        cookie: Uuid::new_v4().to_string(),
        cookie_expire: "0".to_string(),
    };

    collection
        .update_one(
            doc! { "auth.cookie": cookie },
            doc! { "$set": { "auth": to_bson(&auth_object).unwrap() } },
        )
        .await
        .is_ok()
}

pub async fn check_user_existence(username: &str, email: &str) -> Option<(bool, bool)> {
    let database = DB.get().unwrap();
    let collection: Collection<UserOut> = database.collection("users");

    let username_user = collection
        .find_one(doc! { "username": username })
        .await
        .ok()
        .flatten();

    let username_exists = username_user.is_some();

    let email_hash = super::schemas::create_email_hash(email);
    let email_user = collection
        .find_one(doc! { "email_hash": &email_hash })
        .await
        .ok()
        .flatten();

    let email_exists = email_user.is_some();

    Some((username_exists, email_exists))
}

pub async fn retrieve_user_by_username_or_email(
    username: Option<&str>,
    email: Option<&str>,
) -> Option<UserOut> {
    let database = DB.get().unwrap();
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
