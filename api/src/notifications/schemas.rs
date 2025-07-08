use serde::{Deserialize, Serialize};

pub const TWILIO_API_BASE_URL: &str = "https://api.twilio.com/2010-04-01";
pub const SENDGRID_API_BASE_URL: &str = "https://api.sendgrid.com/v3";

#[derive(Debug, Serialize, Deserialize)]
pub struct SendGridEmailRequest {
    pub personalizations: Vec<SendGridPersonalization>,
    pub from: SendGridContact,
    pub subject: String,
    pub content: Vec<SendGridContent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendGridPersonalization {
    pub to: Vec<SendGridContact>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendGridContact {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendGridContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub value: String,
}
