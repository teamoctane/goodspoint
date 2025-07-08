use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit, OsRng, rand_core::RngCore},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{env::var, error::Error, ops::Deref, sync::OnceLock};

#[derive(Serialize, Deserialize)]
pub struct EncryptedString {
    data: String,
    nonce: String,
    #[serde(skip)]
    salt: Option<String>,
    #[serde(skip)]
    decrypted_data: OnceLock<String>,
}

impl Clone for EncryptedString {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            nonce: self.nonce.clone(),
            salt: self.salt.clone(),
            decrypted_data: OnceLock::new(),
        }
    }
}

impl EncryptedString {
    pub fn new(text: &str, salt: &str) -> Result<Self, Box<dyn Error>> {
        let key_material = format!("{}{}", var("ENCRYPTION_KEY")?, salt);
        let mut key_bytes = [0u8; 32];
        let bytes = key_material.as_bytes();
        key_bytes[..bytes.len().min(32)].copy_from_slice(&bytes[..bytes.len().min(32)]);

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, text.as_bytes())
            .map_err(|e| format!("Failed to encrypt data: {}", e))?;

        Ok(Self {
            data: STANDARD.encode(&ciphertext),
            nonce: STANDARD.encode(&nonce_bytes),
            salt: Some(salt.to_string()),
            decrypted_data: {
                let cell = OnceLock::new();
                let _ = cell.set(text.to_string());
                cell
            },
        })
    }

    pub fn set_salt(&self, salt: &str) -> Result<(), Box<dyn Error>> {
        unsafe {
            let ptr = self as *const Self as *mut Self;
            (*ptr).salt = Some(salt.to_string());
        }
        Ok(())
    }

    fn decrypt(&self) -> Result<String, Box<dyn Error>> {
        let salt = self.salt.as_ref().ok_or("Salt not set")?;
        let key_material = format!("{}{}", var("ENCRYPTION_KEY")?, salt);
        let mut key_bytes = [0u8; 32];
        let bytes = key_material.as_bytes();
        key_bytes[..bytes.len().min(32)].copy_from_slice(&bytes[..bytes.len().min(32)]);

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
        let ciphertext = STANDARD.decode(&self.data)?;
        let nonce_bytes = STANDARD.decode(&self.nonce)?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_slice())
            .map_err(|e| format!("Failed to decrypt data: {}", e))?;

        Ok(String::from_utf8(plaintext)?)
    }

    pub fn to_string(&self) -> String {
        self.decrypted_data
            .get_or_init(|| self.decrypt().unwrap_or_else(|_| "ERROR".to_string()))
            .clone()
    }
}

impl Deref for EncryptedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.decrypted_data
            .get_or_init(|| self.decrypt().unwrap_or_else(|_| "ERROR".to_string()))
    }
}

#[inline]
pub fn create_email_hash(email: &str) -> String {
    let mut hasher = Sha256::default();
    hasher.update(email.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(Serialize, Deserialize)]
pub struct UserIn {
    pub username: Option<String>,
    pub password: String,
    pub email: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AuthObject {
    pub cookie: String,
    #[serde(rename = "cookie-expire")]
    pub cookie_expire: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserOut {
    pub username: String,
    pub email: EncryptedString,
    pub email_hash: String,
    pub email_verified: bool,
    pub whatsapp_number: Option<EncryptedString>,
    pub whatsapp_verified: bool,
    pub password: String,
    pub salt: String,
    pub auth: AuthObject,
    pub uid: String,
    pub enabled: bool,
}

impl UserOut {
    pub fn new(
        username: String,
        email: String,
        password: String,
        salt: String,
        auth: AuthObject,
        uid: String,
        enabled: bool,
    ) -> Result<Self, Box<dyn Error>> {
        let encrypted_email = EncryptedString::new(&email, &salt)?;
        let email_hash = create_email_hash(&email);

        Ok(Self {
            username,
            email: encrypted_email,
            email_hash,
            email_verified: false,
            whatsapp_number: None,
            whatsapp_verified: false,
            password,
            salt,
            auth,
            uid,
            enabled,
        })
    }

    #[inline]
    pub fn initialize_encryption(&self) -> Result<(), Box<dyn Error>> {
        self.email.set_salt(&self.salt)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserQuery {
    pub username: Option<String>,
    pub email: Option<String>,
    pub uid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangePasswordResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendEmailOTPRequest {
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyEmailOTPRequest {
    pub email: String,
    pub otp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddWhatsAppRequest {
    pub whatsapp_number: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendWhatsAppOTPRequest {
    pub whatsapp_number: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyWhatsAppOTPRequest {
    pub whatsapp_number: String,
    pub otp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OTPVerification {
    pub identifier: String,
    pub otp_hash: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub attempts: u32,
    pub verification_type: String,
}
