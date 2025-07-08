use serde::{Deserialize, Serialize};

pub const MAX_SEARCH_QUERY_LENGTH: usize = 1000;
pub const MAX_SEARCH_RESULTS: u32 = 80;
pub const DEFAULT_SEARCH_LIMIT: u32 = 20;
pub const MIN_SEARCH_CANDIDATES: u32 = 20;
pub const SEARCH_SIMILARITY_THRESHOLD: f32 = 0.3;

pub const GROQ_AI_MODEL: &str = "compound-beta";
pub const GROQ_API_ENDPOINT: &str = "https://api.groq.com/openai/v1/chat/completions";
pub const FILEBASE_IPFS_ENDPOINT: &str = "https://rpc.filebase.io";

pub const HYBRID_VECTOR_WEIGHT: f32 = 0.7;
pub const HYBRID_TEXT_WEIGHT: f32 = 0.3;
pub const VECTOR_SEARCH_CANDIDATES_MULTIPLIER: u32 = 10;

pub const MAX_IMAGE_SIZE: usize = 5 * 1024 * 1024;
pub const MAX_IMAGES_PER_REQUEST: usize = 2;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    Vector,
    Text,
    Combined,
    Hybrid,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchSort {
    Relevance,
    Price,
    CreatedAt,
    Popularity,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: Option<String>,
    pub category: Option<crate::products::schemas::ProductCategory>,
    pub product_type: Option<crate::products::schemas::ProductType>,
    pub price_min: Option<f64>,
    pub price_max: Option<f64>,
    pub mode: Option<SearchMode>,
    pub sort: Option<SearchSort>,
    pub sort_order: Option<SortOrder>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub use_ai_enhancement: Option<bool>,
    pub conversation_id: Option<String>,
    pub should_refine: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleSearchRequest {
    pub query: Option<String>,
    pub limit: Option<u32>,
    pub force_original: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub product_id: String,
    pub title: String,
    pub description: String,
    pub product_type: crate::products::schemas::ProductType,
    pub category: crate::products::schemas::ProductCategory,
    pub tags: Vec<String>,
    pub quantity: crate::products::schemas::ProductQuantity,
    pub price: Option<String>,
    pub thumbnail_url: Option<String>,
    pub created_at: u64,
    pub similarity_score: Option<f32>,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub status: String,
    pub results: Vec<SearchResult>,
    pub total_count: u64,
    pub enhanced_query: Option<String>,
    pub ai_enhancement_triggered: bool,
    pub search_mode: SearchMode,
    pub processing_time_ms: u64,
    pub conversation_id: Option<String>,
    pub ai_suggestions: Option<Vec<String>>,
    pub needs_refinement: Option<bool>,
    pub refinement_questions: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleSearchResponse {
    pub results: Vec<SearchResult>,
    pub total_count: u64,
    pub enhanced_query: Option<String>,
    pub ai_enhancement_triggered: bool,
    pub processing_time_ms: u64,
    pub inferred_category: Option<crate::products::schemas::ProductCategory>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqQueryEnhancementRequest {
    pub model: String,
    pub messages: Vec<GroqMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub response_format: Option<serde_json::Value>,
    pub tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqEnhancementResponse {
    pub enhanced_query: String,
    pub category: Option<crate::products::schemas::ProductCategory>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroqMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqResponse {
    pub choices: Vec<GroqChoice>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqChoice {
    pub message: GroqResponseMessage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqResponseMessage {
    pub role: String,
    pub content: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchConversation {
    pub conversation_id: String,
    pub turns: Vec<ConversationTurn>,
    pub created_at: u64,
    pub updated_at: u64,
    pub user_session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub user_query: String,
    pub enhanced_query: Option<String>,
    pub ai_response: Option<String>,
    pub search_results_count: Option<u32>,
    pub suggestions: Option<Vec<String>>,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryRefinementRequest {
    pub conversation_id: String,
    pub user_input: String,
    pub previous_query: Option<String>,
    pub search_results_count: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryRefinementResponse {
    pub refined_query: Option<String>,
    pub suggestions: Vec<String>,
    pub should_search_immediately: bool,
    pub clarification_questions: Option<Vec<String>>,
    pub conversation_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VectorSearchParams {
    pub vector: Vec<f32>,
    pub path: String,
    pub num_candidates: u32,
    pub limit: u32,
    pub index: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRefinementTool {
    pub action: String,
    pub enhanced_query: Option<String>,
    pub suggestions: Vec<String>,
    pub clarification_questions: Option<Vec<String>>,
    pub should_search_immediately: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClipSearchRequest {
    pub text: String,
    pub image_urls: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClipTextRequest {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClipEmbeddingResponse {
    pub embedding: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextSearchQuery {
    pub query: String,
    pub fields: Vec<String>,
    pub boost_factors: Option<std::collections::HashMap<String, f32>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VectorSearchQuery {
    pub embedding: Vec<f32>,
    pub k: u32,
    pub threshold: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchFilters {
    pub category: Option<crate::products::schemas::ProductCategory>,
    pub product_type: Option<crate::products::schemas::ProductType>,
    pub price_min: Option<f64>,
    pub price_max: Option<f64>,
    pub user_id: Option<String>,
    pub created_after: Option<u64>,
    pub created_before: Option<u64>,
    pub has_images: Option<bool>,
    pub enabled_only: bool,
}



impl Default for SearchFilters {
    fn default() -> Self {
        Self {
            category: None,
            product_type: None,
            price_min: None,
            price_max: None,
            user_id: None,
            created_after: None,
            created_before: None,
            has_images: None,
            enabled_only: true,
        }
    }
}

impl Default for SearchMode {
    fn default() -> Self {
        Self::Hybrid
    }
}

impl Default for SearchSort {
    fn default() -> Self {
        Self::Relevance
    }
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Desc
    }
}
