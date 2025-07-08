use axum::http::StatusCode;
use reqwest::Client;
use std::env::var;

use super::schemas::*;
use crate::apex::utils::VerboseHTTPError;

pub async fn send_whatsapp_internal(
    phone_number: &str,
    message: &str,
) -> Result<(), VerboseHTTPError> {
    let account_sid = var("TWILIO_ACCOUNT_SID").map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Missing Twilio configuration".to_string(),
        )
    })?;
    let auth_token = var("TWILIO_AUTH_TOKEN").map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Missing Twilio configuration".to_string(),
        )
    })?;
    let from_number = format!(
        "whatsapp:{}",
        var("TWILIO_PHONE_NUMBER").map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Missing Twilio configuration".to_string(),
            )
        })?
    );
    let to_number = format!("whatsapp:{}", phone_number);

    let client = Client::new();
    let url = format!(
        "{}/Accounts/{}/Messages.json",
        TWILIO_API_BASE_URL, account_sid
    );

    let params = [
        ("To", to_number.as_str()),
        ("From", &from_number),
        ("Body", message),
    ];

    let response = client
        .post(&url)
        .basic_auth(&account_sid, Some(&auth_token))
        .form(&params)
        .send()
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to send WhatsApp message".to_string(),
            )
        })?;

    if !response.status().is_success() {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "WhatsApp service unavailable".to_string(),
        ));
    }

    Ok(())
}

pub async fn send_email_internal(
    to_email: &str,
    to_name: Option<&str>,
    subject: &str,
    html_content: &str,
) -> Result<(), VerboseHTTPError> {
    let api_key = var("SENDGRID_API_KEY").map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Missing SendGrid configuration".to_string(),
        )
    })?;
    let client = Client::new();
    let url = format!("{}/mail/send", SENDGRID_API_BASE_URL);

    let email_request = SendGridEmailRequest {
        personalizations: vec![SendGridPersonalization {
            to: vec![SendGridContact {
                email: to_email.to_string(),
                name: to_name.map(|s| s.to_string()),
            }],
        }],
        from: SendGridContact {
            email: "comms@goodspoint.tech".to_string(),
            name: Some("Goodspoint".to_string()),
        },
        subject: subject.to_string(),
        content: vec![SendGridContent {
            content_type: "text/html".to_string(),
            value: html_content.to_string(),
        }],
    };

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&email_request)
        .send()
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to send email".to_string(),
            )
        })?;

    if !response.status().is_success() {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Email service unavailable".to_string(),
        ));
    }

    Ok(())
}
