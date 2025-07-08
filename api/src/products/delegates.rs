use axum::http::StatusCode;
use bytes::Bytes;
use futures::TryStreamExt;
use mongodb::{Collection, bson::doc, options::FindOptions};
use reqwest::multipart::{Form, Part};
use serde_json;
use std::{
    env::var,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

use super::schemas::*;
use crate::{
    DB,
    apex::utils::VerboseHTTPError,
    auth::schemas::UserOut,
    search::{preprocessing::preprocess_text, schemas::FILEBASE_IPFS_ENDPOINT},
};

#[derive(serde::Deserialize)]
struct FilebaseUploadResponse {
    #[serde(rename = "Hash")]
    hash: String,
    #[serde(rename = "Name")]
    _name: String,
    #[serde(rename = "Size")]
    _size: String,
}

pub async fn upload_file_to_filebase(
    file_name: &str,
    file_data: Bytes,
    content_type: &str,
) -> Result<String, VerboseHTTPError> {
    let access_key = var("FILEBASE_ACCESS_KEY").expect("FILEBASE_ACCESS_KEY must be set");

    let file_part = Part::bytes(file_data.to_vec())
        .file_name(file_name.to_string())
        .mime_str(content_type)
        .unwrap();

    let form = Form::new().part("file", file_part);

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/v0/add?pin=true", FILEBASE_IPFS_ENDPOINT))
        .header("Authorization", format!("Bearer {}", access_key))
        .multipart(form)
        .send()
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to upload to Filebase IPFS".to_string(),
            )
        })?;

    let status = response.status();

    if !status.is_success() {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Filebase upload failed: {}", status),
        ));
    }

    let upload_result: FilebaseUploadResponse = response.json().await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to parse Filebase response".to_string(),
        )
    })?;

    let file_url = format!("https://ipfs.filebase.io/ipfs/{}", upload_result.hash);
    Ok(file_url)
}

pub async fn create_product(
    user: &UserOut,
    request: CreateProductRequest,
    thumbnail_file: Option<(String, Bytes, String)>,
    gallery_files: Vec<(String, Bytes, String)>,
) -> Result<Product, VerboseHTTPError> {
    if request.title.trim().is_empty() {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Product title cannot be empty".to_string(),
        ));
    }

    if request.description.trim().is_empty() {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Product description cannot be empty".to_string(),
        ));
    }

    if request.title.len() > MAX_TITLE_LENGTH {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            format!(
                "Product title cannot exceed {} characters",
                MAX_TITLE_LENGTH
            ),
        ));
    }

    if request.description.len() > MAX_DESCRIPTION_LENGTH {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            format!(
                "Product description cannot exceed {} characters",
                MAX_DESCRIPTION_LENGTH
            ),
        ));
    }

    if let Some(ref questions) = request.custom_questions {
        if questions.questions.len() > MAX_QUESTIONS_COUNT {
            return Err(VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                format!(
                    "Cannot have more than {} custom questions",
                    MAX_QUESTIONS_COUNT
                ),
            ));
        }

        for question in &questions.questions {
            if question.question.trim().is_empty() {
                return Err(VerboseHTTPError::Standard(
                    StatusCode::BAD_REQUEST,
                    "Question text cannot be empty".to_string(),
                ));
            }

            if question.question.len() > MAX_QUESTION_LENGTH {
                return Err(VerboseHTTPError::Standard(
                    StatusCode::BAD_REQUEST,
                    format!(
                        "Question text cannot exceed {} characters",
                        MAX_QUESTION_LENGTH
                    )
                    .to_string(),
                ));
            }
        }
    }

    if request.tags.len() > MAX_TAGS_COUNT {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            format!("Cannot have more than {} tags", MAX_TAGS_COUNT).to_string(),
        ));
    }

    for tag in &request.tags {
        if tag.trim().is_empty() {
            return Err(VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Tag cannot be empty".to_string(),
            ));
        }
        if tag.len() > MAX_TAG_LENGTH {
            return Err(VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                format!("Tag cannot exceed {} characters", MAX_TAG_LENGTH).to_string(),
            ));
        }
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if gallery_files.len() > MAX_GALLERY_ITEMS {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            format!(
                "Cannot upload more than {} gallery items",
                MAX_GALLERY_ITEMS
            ),
        ));
    }

    let gallery = if gallery_files.is_empty() {
        Vec::new()
    } else {
        let mut uploaded_items = Vec::new();
        for (i, (file_name, file_data, content_type)) in gallery_files.into_iter().enumerate() {
            match upload_file_to_filebase(&file_name, file_data.clone(), &content_type).await {
                Ok(file_url) => {
                    let item_type = match content_type.as_str() {
                        ct if ct.starts_with("image/") => "picture",
                        ct if ct.starts_with("video/") => "video",
                        ct if ct.starts_with("model/") => "obj",
                        _ => "other",
                    };

                    uploaded_items.push(GalleryItem {
                        id: Uuid::new_v4().to_string(),
                        item_type: item_type.to_string(),
                        url: file_url,
                        size: file_data.len() as u64,
                        order: i as u32,
                        upload_timestamp: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    });
                }
                Err(_) => {
                    return Err(VerboseHTTPError::Standard(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to upload gallery file: {}", file_name),
                    ));
                }
            }
        }
        uploaded_items
    };

    let thumbnail_url = if let Some((file_name, file_data, content_type)) = thumbnail_file {
        match upload_file_to_filebase(&file_name, file_data, &content_type).await {
            Ok(url) => Some(url),
            Err(_) => {
                return Err(VerboseHTTPError::Standard(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to upload thumbnail".to_string(),
                ));
            }
        }
    } else {
        None
    };

    let mut combined_text = format!("{} {}", request.title, user.username);

    for tag in &request.tags {
        combined_text.push_str(" ");
        combined_text.push_str(tag);
    }

    let preprocessed_text = preprocess_text(&combined_text);

    let embedding =
        match generate_combined_embedding(&preprocessed_text, &gallery, thumbnail_url.as_deref())
            .await
        {
            Ok(embedding) => Some(embedding),
            Err(_) => {
                return Err(VerboseHTTPError::Standard(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to generate required embeddings".to_string(),
                ));
            }
        };

    let product = Product {
        product_id: Uuid::new_v4().to_string(),
        user_id: user.uid.clone(),
        username: user.username.clone(),
        title: request.title,
        description: request.description,
        product_type: request.product_type,
        purchase_type: request.purchase_type,
        category: request.category,
        tags: request.tags,
        quantity: request.quantity,
        price: request.price,
        custom_questions: request.custom_questions,
        gallery,
        thumbnail_url,
        embedding,
        created_at: now,
        updated_at: now,
        enabled: true,
    };

    let database = DB.get().unwrap();
    let collection: Collection<Product> = database.collection("products");

    collection.insert_one(&product).await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create product".to_string(),
        )
    })?;

    Ok(product)
}

pub async fn get_product_by_id(product_id: &str) -> Result<Product, VerboseHTTPError> {
    if product_id.trim().is_empty() {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Product ID cannot be empty".to_string(),
        ));
    }

    let database = DB.get().unwrap();
    let collection: Collection<Product> = database.collection("products");

    let product = collection
        .find_one(doc! { "product_id": product_id, "enabled": true })
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or_else(|| {
            VerboseHTTPError::Standard(StatusCode::NOT_FOUND, "Product not found".to_string())
        })?;

    Ok(product)
}

pub async fn get_user_product_by_id(
    user: &UserOut,
    product_id: &str,
) -> Result<Product, VerboseHTTPError> {
    if product_id.trim().is_empty() {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Product ID cannot be empty".to_string(),
        ));
    }

    let database = DB.get().unwrap();
    let collection: Collection<Product> = database.collection("products");

    let product = collection
        .find_one(doc! { "product_id": product_id, "user_id": &user.uid })
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
                "Product not found or access denied".to_string(),
            )
        })?;

    Ok(product)
}

pub async fn update_product(
    user: &UserOut,
    product_id: &str,
    request: UpdateProductRequest,
    thumbnail_data: Option<Vec<u8>>,
) -> Result<Product, VerboseHTTPError> {
    let existing_product = get_user_product_by_id(user, product_id).await?;

    if let Some(ref title) = request.title {
        if title.trim().is_empty() {
            return Err(VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Product title cannot be empty".to_string(),
            ));
        }
        if title.len() > MAX_TITLE_LENGTH {
            return Err(VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                format!(
                    "Product title cannot exceed {} characters",
                    MAX_TITLE_LENGTH
                ),
            ));
        }
    }

    if let Some(ref description) = request.description {
        if description.trim().is_empty() {
            return Err(VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Product description cannot be empty".to_string(),
            ));
        }
        if description.len() > MAX_DESCRIPTION_LENGTH {
            return Err(VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                format!(
                    "Product description cannot exceed {} characters",
                    MAX_DESCRIPTION_LENGTH
                ),
            ));
        }
    }

    if let Some(ref questions) = request.custom_questions {
        if questions.questions.len() > 12 {
            return Err(VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Cannot have more than 12 custom questions".to_string(),
            ));
        }

        for question in &questions.questions {
            if question.question.trim().is_empty() {
                return Err(VerboseHTTPError::Standard(
                    StatusCode::BAD_REQUEST,
                    "Question text cannot be empty".to_string(),
                ));
            }

            if question.question.len() > MAX_QUESTION_LENGTH {
                return Err(VerboseHTTPError::Standard(
                    StatusCode::BAD_REQUEST,
                    format!(
                        "Question text cannot exceed {} characters",
                        MAX_QUESTION_LENGTH
                    )
                    .to_string(),
                ));
            }
        }
    }

    if let Some(ref tags) = request.tags {
        if tags.len() > 32 {
            return Err(VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                format!("Cannot have more than {} tags", MAX_TAGS_COUNT).to_string(),
            ));
        }

        for tag in tags {
            if tag.trim().is_empty() {
                return Err(VerboseHTTPError::Standard(
                    StatusCode::BAD_REQUEST,
                    "Tag cannot be empty".to_string(),
                ));
            }
            if tag.len() > MAX_TAG_LENGTH {
                return Err(VerboseHTTPError::Standard(
                    StatusCode::BAD_REQUEST,
                    format!("Tag cannot exceed {} characters", MAX_TAG_LENGTH).to_string(),
                ));
            }
        }
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut update_doc = doc! { "updated_at": now as i64 };

    let mut regenerate_embedding = false;
    let final_title = request
        .title
        .as_ref()
        .unwrap_or(&existing_product.title)
        .clone();
    let final_tags = request
        .tags
        .as_ref()
        .unwrap_or(&existing_product.tags)
        .clone();

    if request.title.is_some() || request.tags.is_some() {
        regenerate_embedding = true;
    }

    if regenerate_embedding {
        let mut combined_text = format!("{} {}", final_title, user.username);
        for tag in &final_tags {
            combined_text.push_str(" ");
            combined_text.push_str(tag);
        }

        let preprocessed_text = preprocess_text(&combined_text);

        match generate_combined_embedding(
            &preprocessed_text,
            &existing_product.gallery,
            existing_product.thumbnail_url.as_deref(),
        )
        .await
        {
            Ok(embedding) => {
                update_doc.insert("embedding", embedding);
            }
            Err(_) => {
                return Err(VerboseHTTPError::Standard(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to regenerate required embeddings".to_string(),
                ));
            }
        }
    }

    if let Some(title) = request.title {
        update_doc.insert("title", title);
    }
    if let Some(description) = request.description {
        update_doc.insert("description", description);
    }
    if let Some(product_type) = request.product_type {
        update_doc.insert(
            "product_type",
            mongodb::bson::to_bson(&product_type).unwrap(),
        );
    }
    if let Some(category) = request.category {
        update_doc.insert("category", mongodb::bson::to_bson(&category).unwrap());
    }
    if let Some(tags) = request.tags {
        update_doc.insert("tags", tags);
    }
    if let Some(quantity) = request.quantity {
        update_doc.insert("quantity", mongodb::bson::to_bson(&quantity).unwrap());
    }
    if let Some(price) = request.price {
        update_doc.insert("price", price);
    }
    if let Some(custom_questions) = request.custom_questions {
        update_doc.insert(
            "custom_questions",
            mongodb::bson::to_bson(&custom_questions).unwrap(),
        );
    }

    if let Some(_thumbnail_data) = thumbnail_data {
        let thumbnail_url = format!("thumbnail_{}.jpg", Uuid::new_v4());
        update_doc.insert("thumbnail_url", thumbnail_url);
    }

    let database = DB.get().unwrap();
    let collection: Collection<Product> = database.collection("products");

    collection
        .update_one(
            doc! { "product_id": product_id, "user_id": &user.uid },
            doc! { "$set": update_doc },
        )
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update product".to_string(),
            )
        })?;

    get_user_product_by_id(user, product_id).await
}

pub async fn delete_product(user: &UserOut, product_id: &str) -> Result<(), VerboseHTTPError> {
    let database = DB.get().unwrap();
    let collection: Collection<Product> = database.collection("products");

    let result = collection
        .update_one(
            doc! { "product_id": product_id, "user_id": &user.uid, "enabled": true },
            doc! { "$set": { "enabled": false } },
        )
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    if result.matched_count == 0 {
        return Err(VerboseHTTPError::Standard(
            StatusCode::NOT_FOUND,
            "Product not found or access denied".to_string(),
        ));
    }

    Ok(())
}

pub async fn list_user_products(
    user: &UserOut,
    limit: u32,
    offset: u32,
) -> Result<Vec<ProductListItem>, VerboseHTTPError> {
    let database = DB.get().unwrap();
    let collection: Collection<Product> = database.collection("products");

    let filter = doc! { "user_id": &user.uid, "enabled": true };

    let options = FindOptions::builder()
        .limit(limit as i64)
        .skip(offset as u64)
        .sort(doc! { "created_at": -1 })
        .build();

    let mut cursor = collection
        .find(filter)
        .with_options(options)
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    let mut products = Vec::new();
    while let Ok(Some(product)) = cursor.try_next().await {
        products.push(ProductListItem {
            product_id: product.product_id,
            title: product.title,
            product_type: product.product_type,
            quantity: product.quantity,
            created_at: product.created_at,
            enabled: product.enabled,
            thumbnail_url: product.thumbnail_url,
        });
    }

    Ok(products)
}

pub async fn generate_questions_with_groq(
    user: &UserOut,
    request: GenerateQuestionsRequest,
) -> Result<ProductQuestions, VerboseHTTPError> {
    let groq_api_key = var("GROQ_API_KEY").map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "GROQ API key not configured".to_string(),
        )
    })?;

    let groq_model = "compound-beta".to_string();

    let product = get_user_product_by_id(user, &request.product_id).await?;

    let product_type_str = match product.product_type {
        super::schemas::ProductType::New => "new",
        super::schemas::ProductType::Used => "used",
    };

    let prompt = format!(
        "Based on the seller's request: '{}', generate specific questions to collect the information they need about their {} product: '{}' - '{}'.

        The seller wants to gather specific details from potential buyers or to complete their product listing. Generate ONLY the questions needed to collect the information they specifically mentioned. 

        Aim for 2-3 questions maximum, only create more if absolutely necessary (max 12). Each question should be directly related to what the seller requested.

        Examples:
        - If seller says 'I need to know the color and size preferences' → create questions about color choice and size requirements
        - If seller says 'Ask about delivery location and timeline' → create questions about delivery address and preferred delivery date
        - If seller says 'Find out their budget and payment method' → create questions about budget range and payment preferences

        Make questions clear, specific, and actionable. Mark questions as mandatory if they are essential for completing the transaction.
        
        Important: Do not attempt to call any scripts, functions, or execute any code. Use only the provided tool to format your response.",
        request.description, product_type_str, product.title, product.description
    );

    let generate_questions_tool = GroqTool {
        tool_type: "function".to_string(),
        function: GroqFunction {
            name: "generate_product_questions".to_string(),
            description: "Generate specific questions based on seller's requirements to collect needed information".to_string(),
            parameters: GroqFunctionParameters {
                param_type: "object".to_string(),
                properties: serde_json::json!({
                    "questions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "question": {
                                    "type": "string",
                                    "description": "The specific question text related to seller's requirements"
                                },
                                "type": {
                                    "type": "string",
                                    "enum": ["yes_no", "free_response"],
                                    "description": "Whether the question requires a yes/no answer or free response"
                                },
                                "mandatory": {
                                    "type": "boolean",
                                    "description": "Whether this question is required to be answered"
                                }
                            },
                            "required": ["question", "type", "mandatory"]
                        },
                        "minItems": 1,
                        "maxItems": 12,
                        "description": "Generate 2-3 questions maximum, only more if absolutely necessary"
                    }
                }),
                required: vec!["questions".to_string()],
            },
        },
    };

    let chat_completion = GroqChatCompletion {
        model: groq_model,
        messages: vec![GroqMessage {
            role: "user".to_string(),
            content: prompt,
        }],
        temperature: 0.7,
        max_tokens: AI_MAX_TOKENS,
        tools: vec![generate_questions_tool],
        tool_choice: "required".to_string(),
    };

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", groq_api_key))
        .header("Content-Type", "application/json")
        .json(&chat_completion)
        .send()
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to call Groq API".to_string(),
            )
        })?;

    if !response.status().is_success() {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Groq API request failed".to_string(),
        ));
    }

    let groq_response: GroqResponse = response.json().await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to parse Groq response".to_string(),
        )
    })?;

    if groq_response.choices.is_empty() {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "No response from Groq API".to_string(),
        ));
    }

    let choice = &groq_response.choices[0];

    let tool_calls = choice.message.tool_calls.as_ref().ok_or_else(|| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "No tool calls in Groq response".to_string(),
        )
    })?;

    if tool_calls.is_empty() {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Empty tool calls in Groq response".to_string(),
        ));
    }

    let tool_call = &tool_calls[0];
    if tool_call.function.name != "generate_product_questions" {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unexpected tool call function name".to_string(),
        ));
    }

    let arguments: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to parse tool call arguments".to_string(),
            )
        })?;

    let questions_array = arguments
        .get("questions")
        .and_then(|q| q.as_array())
        .ok_or_else(|| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid questions format in tool call".to_string(),
            )
        })?;

    let questions: Vec<Question> = questions_array
        .iter()
        .enumerate()
        .filter_map(|(i, q)| {
            let question_text = q.get("question")?.as_str()?;
            let question_type = q.get("type")?.as_str()?;
            let mandatory = q.get("mandatory")?.as_bool().unwrap_or(false);

            Some(Question {
                id: format!("q_{}", i + 1),
                question: question_text.to_string(),
                question_type: match question_type {
                    "yes_no" => QuestionType::YesNo,
                    _ => QuestionType::FreeResponse,
                },
                mandatory,
            })
        })
        .collect();

    if questions.is_empty() {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "No valid questions generated".to_string(),
        ));
    }

    Ok(ProductQuestions { questions })
}

pub async fn get_gallery(
    user: &UserOut,
    product_id: &str,
) -> Result<Vec<GalleryItem>, VerboseHTTPError> {
    let product = get_user_product_by_id(user, product_id).await?;

    Ok(product.gallery)
}

pub async fn replace_gallery(
    user: &UserOut,
    product_id: &str,
    gallery_files: Vec<(String, Bytes, String)>,
) -> Result<Vec<GalleryItem>, VerboseHTTPError> {
    let mut gallery_items = Vec::new();

    for (i, (file_name, file_data, content_type)) in gallery_files.into_iter().enumerate() {
        match upload_file_to_filebase(&file_name, file_data.clone(), &content_type).await {
            Ok(file_url) => {
                let item_type = match content_type.as_str() {
                    ct if ct.starts_with("image/") => "picture",
                    ct if ct.starts_with("video/") => "video",
                    ct if ct.starts_with("model/") => "obj",
                    _ => "other",
                };

                gallery_items.push(GalleryItem {
                    id: Uuid::new_v4().to_string(),
                    item_type: item_type.to_string(),
                    url: file_url,
                    size: file_data.len() as u64,
                    order: i as u32,
                    upload_timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                });
            }
            Err(_) => {
                return Err(VerboseHTTPError::Standard(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to upload gallery file: {}", file_name),
                ));
            }
        }
    }

    let existing_product = get_user_product_by_id(user, product_id).await?;

    let mut combined_text = format!("{} {}", existing_product.title, user.username);
    for tag in &existing_product.tags {
        combined_text.push_str(" ");
        combined_text.push_str(tag);
    }

    let preprocessed_text = preprocess_text(&combined_text);

    let embedding = match generate_combined_embedding(
        &preprocessed_text,
        &gallery_items,
        existing_product.thumbnail_url.as_deref(),
    )
    .await
    {
        Ok(embedding) => embedding,
        Err(_) => {
            return Err(VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to regenerate embeddings".to_string(),
            ));
        }
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let database = DB.get().unwrap();
    let collection: Collection<Product> = database.collection("products");

    collection
        .update_one(
            doc! { "product_id": product_id, "user_id": &user.uid },
            doc! {
                "$set": {
                    "gallery": mongodb::bson::to_bson(&gallery_items).unwrap(),
                    "embedding": embedding,
                    "updated_at": now as i64
                }
            },
        )
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to replace gallery".to_string(),
            )
        })?;

    Ok(gallery_items)
}

pub async fn add_gallery_items(
    user: &UserOut,
    product_id: &str,
    gallery_files: Vec<(String, Bytes, String)>,
) -> Result<Vec<GalleryItem>, VerboseHTTPError> {
    let mut new_items = Vec::new();

    for (file_name, file_data, content_type) in gallery_files.into_iter() {
        match upload_file_to_filebase(&file_name, file_data.clone(), &content_type).await {
            Ok(file_url) => {
                let item_type = match content_type.as_str() {
                    ct if ct.starts_with("image/") => "picture",
                    ct if ct.starts_with("video/") => "video",
                    ct if ct.starts_with("model/") => "obj",
                    _ => "other",
                };

                new_items.push(GalleryItem {
                    id: Uuid::new_v4().to_string(),
                    item_type: item_type.to_string(),
                    url: file_url,
                    size: file_data.len() as u64,
                    order: 0,
                    upload_timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                });
            }
            Err(_) => {
                return Err(VerboseHTTPError::Standard(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to upload gallery file: {}", file_name),
                ));
            }
        }
    }

    let existing_product = get_user_product_by_id(user, product_id).await?;

    let mut updated_gallery = existing_product.gallery;
    let next_order = updated_gallery.len() as u32;

    if updated_gallery.len() + new_items.len() > MAX_GALLERY_ITEMS {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            format!(
                "Adding {} items would exceed the maximum gallery limit of {}",
                new_items.len(),
                MAX_GALLERY_ITEMS
            ),
        ));
    }

    for (i, mut item) in new_items.into_iter().enumerate() {
        item.order = next_order + i as u32;
        updated_gallery.push(item);
    }

    let mut combined_text = format!("{} {}", existing_product.title, user.username);
    for tag in &existing_product.tags {
        combined_text.push_str(" ");
        combined_text.push_str(tag);
    }

    let preprocessed_text = preprocess_text(&combined_text);

    let embedding = match generate_combined_embedding(
        &preprocessed_text,
        &updated_gallery,
        existing_product.thumbnail_url.as_deref(),
    )
    .await
    {
        Ok(embedding) => embedding,
        Err(_) => {
            return Err(VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to regenerate embeddings".to_string(),
            ));
        }
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let database = DB.get().unwrap();
    let collection: Collection<Product> = database.collection("products");

    collection
        .update_one(
            doc! { "product_id": product_id, "user_id": &user.uid },
            doc! {
                "$set": {
                    "gallery": mongodb::bson::to_bson(&updated_gallery).unwrap(),
                    "embedding": embedding,
                    "updated_at": now as i64
                }
            },
        )
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to add gallery items".to_string(),
            )
        })?;

    Ok(updated_gallery)
}

pub async fn reorder_gallery(
    user: &UserOut,
    product_id: &str,
    item_ids: Vec<String>,
) -> Result<Vec<GalleryItem>, VerboseHTTPError> {
    let existing_product = get_user_product_by_id(user, product_id).await?;

    let mut reordered_gallery = Vec::new();

    for (new_order, item_id) in item_ids.into_iter().enumerate() {
        if let Some(mut item) = existing_product
            .gallery
            .iter()
            .find(|g| g.id == item_id)
            .cloned()
        {
            item.order = new_order as u32;
            reordered_gallery.push(item);
        }
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let database = DB.get().unwrap();
    let collection: Collection<Product> = database.collection("products");

    collection
        .update_one(
            doc! { "product_id": product_id, "user_id": &user.uid },
            doc! {
                "$set": {
                    "gallery": mongodb::bson::to_bson(&reordered_gallery).unwrap(),
                    "updated_at": now as i64
                }
            },
        )
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to reorder gallery".to_string(),
            )
        })?;

    Ok(reordered_gallery)
}

async fn generate_combined_embedding(
    text: &str,
    gallery: &[GalleryItem],
    thumbnail_url: Option<&str>,
) -> Result<Vec<f32>, VerboseHTTPError> {
    let clip_api_url =
        var("CLIP_EMBEDDINGS_API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());

    let has_images = gallery.iter().any(|g| g.item_type == "picture") || thumbnail_url.is_some();

    if has_images {
        let image_url = if let Some(thumb) = thumbnail_url {
            thumb
        } else {
            gallery
                .iter()
                .find(|g| g.item_type == "picture")
                .map(|g| g.url.as_str())
                .unwrap()
        };

        let request = serde_json::json!({
            "text": text,
            "image_url": image_url
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&format!("{}/embed/combined", clip_api_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|_| {
                VerboseHTTPError::Standard(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to call CLIP embedding API".to_string(),
                )
            })?;

        if !response.status().is_success() {
            return Err(VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CLIP embedding API request failed".to_string(),
            ));
        }

        let embedding_response: ClipEmbeddingResponse = response.json().await.map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to parse CLIP embedding response".to_string(),
            )
        })?;

        Ok(embedding_response.embedding)
    } else {
        let request = ClipCombinedRequest {
            text: text.to_string(),
        };

        let client = reqwest::Client::new();
        let response = client
            .post(&format!("{}/embed/text", clip_api_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|_| {
                VerboseHTTPError::Standard(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to call CLIP embedding API".to_string(),
                )
            })?;

        if !response.status().is_success() {
            return Err(VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CLIP embedding API request failed".to_string(),
            ));
        }

        let embedding_response: ClipEmbeddingResponse = response.json().await.map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to parse CLIP embedding response".to_string(),
            )
        })?;

        Ok(embedding_response.embedding)
    }
}

pub fn is_allowed_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "image/jpeg"
            | "image/jpg"
            | "image/png"
            | "image/gif"
            | "image/webp"
            | "video/mp4"
            | "video/quicktime"
            | "video/x-msvideo"
            | "model/obj"
            | "model/gltf+json"
            | "model/gltf-binary"
            | "application/octet-stream"
    )
}

pub fn is_allowed_image_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "image/jpeg" | "image/jpg" | "image/png" | "image/gif" | "image/webp"
    )
}

pub async fn set_product_questions(
    user: &UserOut,
    product_id: &str,
    questions: ProductQuestions,
) -> Result<ProductQuestions, VerboseHTTPError> {
    if questions.questions.len() > 12 {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Cannot have more than 12 custom questions".to_string(),
        ));
    }

    for question in &questions.questions {
        if question.question.trim().is_empty() {
            return Err(VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Question text cannot be empty".to_string(),
            ));
        }

        if question.question.len() > MAX_QUESTION_LENGTH {
            return Err(VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                format!(
                    "Question text cannot exceed {} characters",
                    MAX_QUESTION_LENGTH
                )
                .to_string(),
            ));
        }
    }

    let _product = get_user_product_by_id(user, product_id).await?;

    let database = DB.get().unwrap();
    let collection: Collection<Product> = database.collection("products");

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    collection
        .update_one(
            doc! { "product_id": product_id, "user_id": &user.uid },
            doc! {
                "$set": {
                    "custom_questions": mongodb::bson::to_bson(&questions).unwrap(),
                    "updated_at": now as i64
                }
            },
        )
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update questions".to_string(),
            )
        })?;

    Ok(questions)
}

pub async fn buy_now_product(
    user: &UserOut,
    product_id: String,
    quantity: u32,
) -> Result<crate::orders::schemas::OrderResponse, VerboseHTTPError> {
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    let collection: Collection<Product> = database.collection("products");

    let product = collection
        .find_one(doc! { "product_id": &product_id })
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or_else(|| {
            VerboseHTTPError::Standard(StatusCode::NOT_FOUND, "Product not found".to_string())
        })?;

    if product.purchase_type != PurchaseType::BuyNow {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Product is not available for buy now".to_string(),
        ));
    }

    if quantity < product.quantity.min_quantity || quantity > product.quantity.max_quantity {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Quantity is outside allowed range".to_string(),
        ));
    }

    let price = product.price;
    let total_price = price * quantity as f64;

    crate::orders::delegates::create_order_internal(
        product_id,
        product.user_id,
        user.uid.clone(),
        quantity,
        total_price,
    )
    .await
}
