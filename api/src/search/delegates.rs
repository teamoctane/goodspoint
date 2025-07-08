use axum::http::StatusCode;
use bytes::Bytes;
use futures::TryStreamExt;
use mongodb::{
    Collection,
    bson::{Document, doc},
};
use std::{collections::HashMap, env::var, time::SystemTime};

use super::{
    preprocessing::{create_search_variants, has_stopwords, preprocess_text},
    schemas::*,
};
use crate::{
    DB,
    apex::utils::VerboseHTTPError,
    products::schemas::{Product, ProductCategory, ProductQuantity, ProductType},
};

pub async fn optimized_search_products(
    request: SimpleSearchRequest,
    image_files: Vec<(String, Bytes, String)>,
) -> Result<SimpleSearchResponse, VerboseHTTPError> {
    let start_time = SystemTime::now();

    let limit = request
        .limit
        .unwrap_or(DEFAULT_SEARCH_LIMIT)
        .min(MAX_SEARCH_RESULTS);

    let filters = SearchFilters {
        enabled_only: true,
        ..Default::default()
    };

    let mut enhanced_query = None;
    let mut ai_enhancement_triggered = false;
    let mut inferred_category = None;

    let final_query = match request.query {
        Some(ref query) => {
            if query.len() > MAX_SEARCH_QUERY_LENGTH {
                return Err(VerboseHTTPError::Standard(
                    StatusCode::BAD_REQUEST,
                    format!(
                        "Query too long. Maximum {} characters allowed",
                        MAX_SEARCH_QUERY_LENGTH
                    ),
                ));
            }

            if query.trim().is_empty() {
                None
            } else if (query.len() > 10 || has_stopwords(query))
                && !request.force_original.unwrap_or(false)
            {
                ai_enhancement_triggered = true;
                match enhance_query_with_ai(query).await {
                    Ok((enhanced, category)) => {
                        enhanced_query = Some(enhanced.clone());
                        inferred_category = category;
                        Some(enhanced)
                    }
                    Err(_) => {
                        ai_enhancement_triggered = false;
                        enhanced_query = Some(query.clone());
                        Some(query.clone())
                    }
                }
            } else {
                enhanced_query = Some(query.clone());
                Some(query.clone())
            }
        }
        None => None,
    };

    let results = match final_query {
        Some(ref query_text) => {
            match vector_search(
                &Some(query_text.clone()),
                &image_files,
                &filters,
                limit * 2,
                0,
            )
            .await
            {
                Ok(vector_results) if !vector_results.is_empty() => {
                    match text_search(query_text, &filters, limit, 0).await {
                        Ok(text_results) => {
                            hybrid_combine_results(vector_results, text_results, limit, 0)
                        }
                        Err(_) => vector_results.into_iter().take(limit as usize).collect(),
                    }
                }
                Ok(_) => text_search(query_text, &filters, limit, 0)
                    .await
                    .unwrap_or_default(),
                Err(_) => text_search(query_text, &filters, limit, 0)
                    .await
                    .unwrap_or_default(),
            }
        }
        None if !image_files.is_empty() => {
            match vector_search(&None, &image_files, &filters, limit, 0).await {
                Ok(results) => results,
                Err(_) => browse_products(&filters, limit, 0)
                    .await
                    .unwrap_or_default(),
            }
        }
        None => browse_products(&filters, limit, 0)
            .await
            .unwrap_or_default(),
    };

    let total_count = results.len() as u64;
    let processing_time = start_time.elapsed().unwrap_or_default().as_millis() as u64;

    Ok(SimpleSearchResponse {
        results,
        total_count,
        enhanced_query,
        ai_enhancement_triggered,
        processing_time_ms: processing_time,
        inferred_category,
    })
}

async fn enhance_query_with_ai(
    query: &str,
) -> Result<(String, Option<crate::products::schemas::ProductCategory>), VerboseHTTPError> {
    let groq_api_key = var("GROQ_API_KEY").map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "GROQ API key not configured".to_string(),
        )
    })?;

    let groq_model = GROQ_AI_MODEL.to_string();

    let prompt = format!(
        "You are a product search query optimizer for an e-commerce platform. Transform the following casual search query into optimized product search terms and categorize it.

Original query: \"{}\"

Your task:
1. Extract key product attributes, brands, categories, and descriptive terms
2. Remove conversational language from the original query  
3. Add relevant synonyms and related terms
4. Focus on searchable keywords that would appear in product listings
5. Categorize the query into one of the following exact product categories:
   Smartphones, Computers, Audio, Cameras, Gaming, Wearables, HomeElectronics, MensClothing, WomensClothing, 
   UnisexClothing, Shoes, Accessories, Jewelry, Bags, Beauty, Furniture, HomeDecor, Kitchen, Garden, HomeTools, 
   HomeImprovement, FitnessEquipment, OutdoorGear, SportsEquipment, Bicycles, WaterSports, WinterSports, 
   CarParts, Motorcycles, AutoTools, CarAccessories, Books, Music, Movies, VideoGames, HealthEquipment, 
   PersonalCare, Supplements, MedicalDevices, BabyClothing, Toys, BabyGear, KidsElectronics, Collectibles, 
   Antiques, Art, Crafts, OfficeSupplies, IndustrialEquipment, BusinessEquipment, Other

Return only a JSON object with this exact format:
{{
  \"enhanced_query\": \"your optimized search terms here\",
  \"category\": \"ExactCategoryNameFromTheList\"
}}

Important: Do not include any other text, explanations, or formatting like markdown code blocks. Do not call any scripts, functions or attempt to execute any code.",
        query
    );

    let enhancement_request = GroqQueryEnhancementRequest {
        model: groq_model.clone(),
        messages: vec![
            GroqMessage {
                role: "system".to_string(),
                content: "You are a product search query optimizer. Respond only with a JSON object containing the enhanced query. No markdown formatting, script execution, function calls or extra text.".to_string(),
            },
            GroqMessage {
                role: "user".to_string(),
                content: prompt,
            }
        ],
        temperature: 0.3,
        max_tokens: 100,
        response_format: None,
        tools: None,
    };

    let client = reqwest::Client::new();

    let response = client
        .post(GROQ_API_ENDPOINT)
        .header("Authorization", format!("Bearer {}", groq_api_key))
        .header("Content-Type", "application/json")
        .json(&enhancement_request)
        .send()
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to call Groq API for query enhancement".to_string(),
            )
        })?;

    let status_code = response.status();

    if !status_code.is_success() {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "Groq API request failed for query enhancement: {}",
                status_code
            ),
        ));
    }

    let response_text = response.text().await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to read Groq response".to_string(),
        )
    })?;

    let groq_response: GroqResponse = serde_json::from_str(&response_text).map_err(|_| {
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

    if let Some(content) = &choice.message.content {
        if let Ok(parsed_json) = serde_json::from_str::<GroqEnhancementResponse>(content) {
            let enhanced_query = parsed_json.enhanced_query.trim().to_string();
            return Ok((enhanced_query, parsed_json.category));
        }

        let cleaned_content = content
            .trim()
            .trim_matches('`')
            .trim_start_matches("json")
            .trim()
            .trim_matches('"');

        if let Ok(parsed_json) = serde_json::from_str::<GroqEnhancementResponse>(cleaned_content) {
            let enhanced_query = parsed_json.enhanced_query.trim().to_string();
            return Ok((enhanced_query, parsed_json.category));
        }

        let fallback_query = cleaned_content.to_string();
        return Ok((fallback_query, None));
    }

    Ok((query.to_string(), None))
}

#[inline]
fn hybrid_combine_results(
    vector_results: Vec<SearchResult>,
    text_results: Vec<SearchResult>,
    limit: u32,
    offset: u32,
) -> Vec<SearchResult> {
    let mut result_map: HashMap<String, SearchResult> =
        HashMap::with_capacity(vector_results.len() + text_results.len());
    let mut scores: HashMap<String, f32> =
        HashMap::with_capacity(vector_results.len() + text_results.len());

    for (index, mut result) in vector_results.into_iter().enumerate() {
        let vector_score = result.similarity_score.unwrap_or(0.0);
        let position_penalty = (index as f32) * 0.01;
        let weighted_score = (vector_score * HYBRID_VECTOR_WEIGHT) - position_penalty;

        result.similarity_score = Some(weighted_score);
        scores.insert(result.product_id.clone(), weighted_score);
        result_map.insert(result.product_id.clone(), result);
    }

    for (index, result) in text_results.into_iter().enumerate() {
        let text_score = 1.0 - (index as f32 * 0.05);
        let position_penalty = (index as f32) * 0.01;
        let weighted_score = (text_score * HYBRID_TEXT_WEIGHT) - position_penalty;

        let product_id = result.product_id.clone();

        match scores.get(&product_id) {
            Some(existing_score) => {
                let combined_score = existing_score + weighted_score;
                scores.insert(product_id.clone(), combined_score);

                if let Some(existing_result) = result_map.get_mut(&product_id) {
                    existing_result.similarity_score = Some(combined_score);
                }
            }
            None => {
                let mut new_result = result;
                new_result.similarity_score = Some(weighted_score);
                scores.insert(product_id.clone(), weighted_score);
                result_map.insert(product_id, new_result);
            }
        }
    }

    let mut final_results: Vec<SearchResult> = result_map.into_values().collect();
    final_results.sort_unstable_by(|a, b| {
        let score_a = a.similarity_score.unwrap_or(0.0);
        let score_b = b.similarity_score.unwrap_or(0.0);
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let start = offset as usize;
    let end = start + (limit as usize);

    if start >= final_results.len() {
        Vec::new()
    } else {
        final_results[start..end.min(final_results.len())].to_vec()
    }
}

async fn vector_search(
    query: &Option<String>,
    image_files: &[(String, Bytes, String)],
    filters: &SearchFilters,
    limit: u32,
    offset: u32,
) -> Result<Vec<SearchResult>, VerboseHTTPError> {
    let embedding = generate_search_embedding(query, image_files).await?;

    let database = DB.get().unwrap();
    let collection: Collection<Product> = database.collection("products");

    match ann_vector_search(&collection, &embedding, filters, limit, offset).await {
        Ok(results) if !results.is_empty() => Ok(results),
        Ok(_) => linear_vector_search(&collection, &embedding, filters, limit, offset).await,
        Err(_) => linear_vector_search(&collection, &embedding, filters, limit, offset).await,
    }
}

async fn ann_vector_search(
    collection: &Collection<Product>,
    embedding: &[f32],
    filters: &SearchFilters,
    limit: u32,
    offset: u32,
) -> Result<Vec<SearchResult>, VerboseHTTPError> {
    let mut pipeline = vec![];

    let candidates = std::cmp::max(
        MIN_SEARCH_CANDIDATES,
        limit * VECTOR_SEARCH_CANDIDATES_MULTIPLIER,
    )
    .min(1000);
    let vector_search_stage = doc! {
        "$vectorSearch": {
            "index": "product_embeddings_index",
            "path": "embedding",
            "queryVector": embedding,
            "numCandidates": candidates,
            "limit": limit + offset,
        }
    };
    pipeline.push(vector_search_stage);

    pipeline.push(doc! {
        "$addFields": {
            "similarity": { "$meta": "vectorSearchScore" }
        }
    });

    let match_stage = build_filter_stage(filters);
    if !match_stage.is_empty() {
        pipeline.push(doc! { "$match": match_stage });
    }

    pipeline.push(doc! {
        "$match": {
            "similarity": { "$gte": SEARCH_SIMILARITY_THRESHOLD }
        }
    });

    pipeline.push(doc! {
        "$lookup": {
            "from": "users",
            "localField": "user_id",
            "foreignField": "uid",
            "as": "user_info"
        }
    });

    if offset > 0 {
        pipeline.push(doc! { "$skip": offset as i64 });
    }
    pipeline.push(doc! { "$limit": limit as i64 });

    let mut cursor = collection.aggregate(pipeline).await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ANN vector search failed".to_string(),
        )
    })?;

    let mut results = Vec::new();
    while let Ok(Some(doc)) = cursor.try_next().await {
        if let Ok(search_result) = convert_doc_to_search_result(doc) {
            results.push(search_result);
        }
    }

    Ok(results)
}

async fn linear_vector_search(
    collection: &Collection<Product>,
    embedding: &[f32],
    filters: &SearchFilters,
    limit: u32,
    offset: u32,
) -> Result<Vec<SearchResult>, VerboseHTTPError> {
    let mut pipeline = vec![];

    let match_stage = build_filter_stage(filters);
    if !match_stage.is_empty() {
        pipeline.push(doc! { "$match": match_stage });
    }

    pipeline.push(doc! {
        "$addFields": {
            "similarity": {
                "$let": {
                    "vars": {
                        "dotProduct": {
                            "$reduce": {
                                "input": { "$zip": { "inputs": ["$embedding", embedding.to_vec()] } },
                                "initialValue": 0.0,
                                "in": { "$add": ["$$value", { "$multiply": [{ "$arrayElemAt": ["$$this", 0] }, { "$arrayElemAt": ["$$this", 1] }] }] }
                            }
                        }
                    },
                    "in": "$$dotProduct"
                }
            }
        }
    });

    pipeline.push(doc! {
        "$match": {
            "similarity": { "$gte": SEARCH_SIMILARITY_THRESHOLD }
        }
    });

    pipeline.push(doc! {
        "$sort": { "similarity": -1 }
    });

    pipeline.push(doc! { "$skip": offset as i64 });
    pipeline.push(doc! { "$limit": limit as i64 });

    pipeline.push(doc! {
        "$lookup": {
            "from": "users",
            "localField": "user_id",
            "foreignField": "uid",
            "as": "user_info"
        }
    });

    let mut cursor = collection.aggregate(pipeline).await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Linear vector search failed".to_string(),
        )
    })?;

    let mut results = Vec::new();
    while let Ok(Some(doc)) = cursor.try_next().await {
        if let Ok(search_result) = convert_doc_to_search_result(doc) {
            results.push(search_result);
        }
    }

    Ok(results)
}

async fn text_search(
    query: &str,
    filters: &SearchFilters,
    limit: u32,
    offset: u32,
) -> Result<Vec<SearchResult>, VerboseHTTPError> {
    let database = DB.get().unwrap();
    let collection: Collection<Product> = database.collection("products");

    let search_variants = create_search_variants(query);
    let processed_query = preprocess_text(query);

    let mut text_conditions = Vec::new();

    for variant in &search_variants {
        if !variant.is_empty() {
            text_conditions.push(doc! {
                "$or": [
                    { "title": { "$regex": variant, "$options": "i" } },
                    { "tags": { "$regex": variant, "$options": "i" } }
                ]
            });
        }
    }

    if !processed_query.is_empty() {
        let keywords: Vec<&str> = processed_query.split_whitespace().collect();
        for keyword in keywords {
            if keyword.len() >= 2 {
                text_conditions.push(doc! {
                    "$or": [
                        { "title": { "$regex": keyword, "$options": "i" } },
                        { "tags": { "$regex": keyword, "$options": "i" } }
                    ]
                });
            }
        }
    }

    let mut match_stage = build_filter_stage(filters);

    if !text_conditions.is_empty() {
        match_stage.insert("$or", text_conditions);
    }

    let mut pipeline = vec![];

    if !match_stage.is_empty() {
        pipeline.push(doc! { "$match": match_stage });
    }

    pipeline.push(doc! {
        "$lookup": {
            "from": "users",
            "localField": "user_id",
            "foreignField": "uid",
            "as": "user_info"
        }
    });

    pipeline.push(doc! { "$sort": { "created_at": -1 } });
    pipeline.push(doc! { "$skip": offset as i64 });
    pipeline.push(doc! { "$limit": limit as i64 });

    let mut cursor = collection.aggregate(pipeline).await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Text search failed".to_string(),
        )
    })?;

    let mut results = Vec::new();
    while let Ok(Some(doc)) = cursor.try_next().await {
        if let Ok(search_result) = convert_doc_to_search_result(doc) {
            results.push(search_result);
        } else {
        }
    }

    Ok(results)
}

async fn browse_products(
    filters: &SearchFilters,
    limit: u32,
    offset: u32,
) -> Result<Vec<SearchResult>, VerboseHTTPError> {
    let database = DB.get().unwrap();
    let collection: Collection<Product> = database.collection("products");

    let match_stage = build_filter_stage(filters);

    let mut pipeline = vec![];

    if !match_stage.is_empty() {
        pipeline.push(doc! { "$match": match_stage });
    }

    pipeline.push(doc! {
        "$lookup": {
            "from": "users",
            "localField": "user_id",
            "foreignField": "uid",
            "as": "user_info"
        }
    });

    pipeline.push(doc! { "$sort": { "created_at": -1 } });
    pipeline.push(doc! { "$skip": offset as i64 });
    pipeline.push(doc! { "$limit": limit as i64 });

    let mut cursor = collection.aggregate(pipeline).await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Browse products failed".to_string(),
        )
    })?;

    let mut results = Vec::new();
    while let Ok(Some(doc)) = cursor.try_next().await {
        if let Ok(search_result) = convert_doc_to_search_result(doc) {
            results.push(search_result);
        } else {
        }
    }

    Ok(results)
}

async fn generate_search_embedding(
    query: &Option<String>,
    image_files: &[(String, Bytes, String)],
) -> Result<Vec<f32>, VerboseHTTPError> {
    let clip_api_url =
        var("CLIP_EMBEDDINGS_API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());

    let client = reqwest::Client::new();

    if let Some(query_text) = query {
        if !image_files.is_empty() {
            let image_urls = upload_temp_images_for_search(image_files).await?;

            let request = ClipSearchRequest {
                text: preprocess_text(query_text),
                image_urls,
            };

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

            let embedding_response: ClipEmbeddingResponse =
                response.json().await.map_err(|_| {
                    VerboseHTTPError::Standard(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to parse CLIP embedding response".to_string(),
                    )
                })?;

            Ok(embedding_response.embedding)
        } else {
            let request = ClipTextRequest {
                text: preprocess_text(query_text),
            };

            let response = client
                .post(&format!("{}/embed/text", clip_api_url))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
                .map_err(|_| {
                    VerboseHTTPError::Standard(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to call CLIP text embedding API".to_string(),
                    )
                })?;

            if !response.status().is_success() {
                return Err(VerboseHTTPError::Standard(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "CLIP text embedding API request failed".to_string(),
                ));
            }

            let embedding_response: ClipEmbeddingResponse =
                response.json().await.map_err(|_| {
                    VerboseHTTPError::Standard(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to parse CLIP text embedding response".to_string(),
                    )
                })?;

            Ok(embedding_response.embedding)
        }
    } else if !image_files.is_empty() {
        let image_urls = upload_temp_images_for_search(image_files).await?;

        let request = ClipSearchRequest {
            text: String::new(),
            image_urls,
        };

        let response = client
            .post(&format!("{}/embed/image", clip_api_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|_| {
                VerboseHTTPError::Standard(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to call CLIP image embedding API".to_string(),
                )
            })?;

        let status_code = response.status();
        if !status_code.is_success() {
            return Err(VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "CLIP image embedding API request failed with status: {}",
                    status_code
                ),
            ));
        }

        let embedding_response: ClipEmbeddingResponse = response.json().await.map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to parse CLIP image embedding response".to_string(),
            )
        })?;

        Ok(embedding_response.embedding)
    } else {
        Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Search requires either query text or images".to_string(),
        ))
    }
}

async fn upload_temp_images_for_search(
    image_files: &[(String, Bytes, String)],
) -> Result<Vec<String>, VerboseHTTPError> {
    let mut image_urls = Vec::new();

    for (file_name, file_data, content_type) in image_files {
        if !content_type.starts_with("image/") {
            continue;
        }

        match crate::products::delegates::upload_file_to_filebase(
            file_name,
            file_data.clone(),
            content_type,
        )
        .await
        {
            Ok(url) => {
                image_urls.push(url);
            }
            Err(_) => {
                return Err(VerboseHTTPError::Standard(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to upload search image: {}", file_name),
                ));
            }
        }
    }

    Ok(image_urls)
}

fn build_filter_stage(filters: &SearchFilters) -> Document {
    let mut match_doc = Document::new();

    if filters.enabled_only {
        match_doc.insert("enabled", true);
    }

    if let Some(ref category) = filters.category {
        match_doc.insert("category", mongodb::bson::to_bson(category).unwrap());
    }

    if let Some(ref product_type) = filters.product_type {
        match_doc.insert(
            "product_type",
            mongodb::bson::to_bson(product_type).unwrap(),
        );
    }

    if filters.price_min.is_some() || filters.price_max.is_some() {
        let mut price_filter = Document::new();

        if let Some(min_price) = filters.price_min {
            price_filter.insert("$gte", min_price);
        }

        if let Some(max_price) = filters.price_max {
            price_filter.insert("$lte", max_price);
        }

        match_doc.insert("price", price_filter);
    }

    if let Some(ref user_id) = filters.user_id {
        match_doc.insert("user_id", user_id);
    }

    if filters.created_after.is_some() || filters.created_before.is_some() {
        let mut date_filter = Document::new();

        if let Some(after) = filters.created_after {
            date_filter.insert("$gte", after as i64);
        }

        if let Some(before) = filters.created_before {
            date_filter.insert("$lte", before as i64);
        }

        match_doc.insert("created_at", date_filter);
    }

    if let Some(has_images) = filters.has_images {
        if has_images {
            match_doc.insert(
                "$or",
                vec![
                    doc! { "thumbnail_url": { "$exists": true, "$ne": null } },
                    doc! { "gallery": { "$not": { "$size": 0 } } },
                ],
            );
        } else {
            match_doc.insert(
                "$and",
                vec![
                    doc! { "thumbnail_url": { "$exists": false } },
                    doc! { "gallery": { "$size": 0 } },
                ],
            );
        }
    }

    match_doc
}

#[inline]
fn convert_doc_to_search_result(doc: Document) -> Result<SearchResult, Box<dyn std::error::Error>> {
    let product_id = doc.get_str("product_id")?.to_string();
    let title = doc.get_str("title")?.to_string();
    let description = doc.get_str("description")?.to_string();

    let product_type = match doc.get_str("product_type")? {
        "new" => ProductType::New,
        "used" => ProductType::Used,
        _ => ProductType::New,
    };

    let category_str = doc.get_str("category")?;
    let category = serde_json::from_str::<ProductCategory>(&format!("\"{}\"", category_str))?;

    let tags = doc
        .get_array("tags")?
        .iter()
        .filter_map(|tag| tag.as_str().map(str::to_string))
        .collect();

    let quantity_doc = doc.get_document("quantity")?;
    let quantity = ProductQuantity {
        min_quantity: quantity_doc.get_i32("min_quantity").unwrap_or(1) as u32,
        max_quantity: quantity_doc.get_i32("max_quantity").unwrap_or(1) as u32,
    };

    let price = doc
        .get_str("price")
        .map(str::to_string)
        .or_else(|_| doc.get_f64("price").map(|p| p.to_string()))
        .or_else(|_| doc.get_i32("price").map(|p| p.to_string()))
        .or_else(|_| doc.get_i64("price").map(|p| p.to_string()))
        .ok();

    let thumbnail_url = doc.get_str("thumbnail_url").ok().map(str::to_string);
    let created_at = doc.get_i64("created_at")? as u64;
    let similarity_score = doc.get_f64("similarity").ok().map(|s| s as f32);

    let user_info = doc.get_array("user_info")?;
    let username = user_info
        .first()
        .and_then(|user_doc| user_doc.as_document())
        .and_then(|user_obj| user_obj.get_str("username").ok())
        .unwrap_or("unknown")
        .to_string();

    Ok(SearchResult {
        product_id,
        title,
        description,
        product_type,
        category,
        tags,
        quantity,
        price,
        thumbnail_url,
        created_at,
        similarity_score,
        username,
    })
}
