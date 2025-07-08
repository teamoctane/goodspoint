use axum::{
    Json,
    extract::{Multipart, Query},
    http::StatusCode,
    response::IntoResponse,
};
use bytes::Bytes;
use serde::Deserialize;

use super::{
    delegates::{optimized_search_products, transcribe_audio, translate_audio},
    schemas::{
        AudioTranscriptionRequest, AudioTranscriptionResponse, AudioTranslationRequest,
        AudioTranslationResponse, SimpleSearchRequest,
    },
};

#[derive(Debug, Deserialize)]
pub struct SearchQueryParams {
    pub force_original: Option<bool>,
}

pub async fn optimized_search_products_endpoint(
    Query(params): Query<SearchQueryParams>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut request = SimpleSearchRequest {
        query: None,
        limit: None,
        force_original: params.force_original, // Use query parameter
    };
    let mut image_files = Vec::new();
    let mut image_count = 0;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "body" => {
                if let Ok(data) = field.bytes().await {
                    if let Ok(mut json_request) =
                        serde_json::from_slice::<SimpleSearchRequest>(&data)
                    {
                        // If force_original wasn't set in JSON body, use query parameter
                        if json_request.force_original.is_none() {
                            json_request.force_original = params.force_original;
                        }
                        request = json_request;
                    } else {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({
                                "error": "Invalid JSON in body field"
                            })),
                        )
                            .into_response();
                    }
                }
            }
            "images" => {
                if image_count >= 2 {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": "Maximum 2 images allowed per search request"
                        })),
                    )
                        .into_response();
                }

                let filename = field.file_name().unwrap_or("image").to_string();
                let content_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();

                if !content_type.starts_with("image/") {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": format!("File '{}' is not a valid image", filename)
                        })),
                    )
                        .into_response();
                }

                if let Ok(data) = field.bytes().await {
                    if data.len() > 5 * 1024 * 1024 {
                        // 5MB limit
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({
                                "error": format!("Image '{}' exceeds 5MB size limit", filename)
                            })),
                        )
                            .into_response();
                    }

                    image_files.push((filename, data, content_type));
                    image_count += 1;
                } else {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": format!("Failed to read image data for '{}'", filename)
                        })),
                    )
                        .into_response();
                }
            }
            _ => {
                // Ignore unknown fields
            }
        }
    }

    match optimized_search_products(request, image_files).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(error) => error.into_response(),
    }
}

// Audio transcription endpoint
pub async fn transcribe_audio_endpoint(
    Query(params): Query<AudioTranscriptionRequest>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut audio_data: Option<Bytes> = None;

    // Extract audio file from multipart
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "audio" {
            if let Some(filename) = field.file_name() {
                let content_type = field.content_type().unwrap_or("audio/wav").to_string();

                // Validate audio file type
                if !content_type.starts_with("audio/") {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": format!("File '{}' is not a valid audio file", filename)
                        })),
                    )
                        .into_response();
                }

                if let Ok(data) = field.bytes().await {
                    if data.len() > 25 * 1024 * 1024 {
                        // 25MB limit
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({
                                "error": "Audio file exceeds 25MB size limit"
                            })),
                        )
                            .into_response();
                    }

                    audio_data = Some(data);
                } else {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": "Failed to read audio data"
                        })),
                    )
                        .into_response();
                }
            }
            break;
        }
    }

    let audio_data = match audio_data {
        Some(data) => data,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No audio file provided"
                })),
            )
                .into_response();
        }
    };

    // Transcribe audio
    match transcribe_audio(audio_data, params.language).await {
        Ok(transcribed_text) => (
            StatusCode::OK,
            Json(AudioTranscriptionResponse {
                text: transcribed_text,
            }),
        )
            .into_response(),
        Err(error) => error.into_response(),
    }
}

// Audio translation endpoint (Hindi to English)
pub async fn translate_audio_endpoint(
    Query(_params): Query<AudioTranslationRequest>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut audio_data: Option<Bytes> = None;

    // Extract audio file from multipart
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "audio" {
            if let Some(filename) = field.file_name() {
                let content_type = field.content_type().unwrap_or("audio/wav").to_string();

                // Validate audio file type
                if !content_type.starts_with("audio/") {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": format!("File '{}' is not a valid audio file", filename)
                        })),
                    )
                        .into_response();
                }

                if let Ok(data) = field.bytes().await {
                    if data.len() > 25 * 1024 * 1024 {
                        // 25MB limit
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({
                                "error": "Audio file exceeds 25MB size limit"
                            })),
                        )
                            .into_response();
                    }

                    audio_data = Some(data);
                } else {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": "Failed to read audio data"
                        })),
                    )
                        .into_response();
                }
            }
            break;
        }
    }

    let audio_data = match audio_data {
        Some(data) => data,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No audio file provided"
                })),
            )
                .into_response();
        }
    };

    // Translate audio (Hindi to English)
    match translate_audio(audio_data).await {
        Ok(translated_text) => (
            StatusCode::OK,
            Json(AudioTranslationResponse {
                text: translated_text,
            }),
        )
            .into_response(),
        Err(error) => error.into_response(),
    }
}
