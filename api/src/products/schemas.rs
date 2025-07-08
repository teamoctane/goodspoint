use serde::{Deserialize, Serialize};

pub const MAX_TITLE_LENGTH: usize = 200;
pub const MAX_DESCRIPTION_LENGTH: usize = 2000;
pub const MAX_QUESTIONS_COUNT: usize = 12;
pub const MAX_QUESTION_LENGTH: usize = 1300;
pub const MAX_TAGS_COUNT: usize = 32;
pub const MAX_TAG_LENGTH: usize = 50;
pub const MAX_GALLERY_ITEMS: usize = 6;
pub const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;
pub const DEFAULT_PAGE_LIMIT: u32 = 20;
pub const MAX_PAGE_LIMIT: u32 = 100;
pub const AI_MAX_TOKENS: u32 = 2048;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProductType {
    New,
    Used,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PurchaseType {
    BuyNow,
    Inquire,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum ProductCategory {
    Smartphones,
    Computers,
    Audio,
    Cameras,
    Gaming,
    Wearables,
    HomeElectronics,
    MensClothing,
    WomensClothing,
    UnisexClothing,
    Shoes,
    Accessories,
    Jewelry,
    Bags,
    Beauty,
    Furniture,
    HomeDecor,
    Kitchen,
    Garden,
    HomeTools,
    HomeImprovement,
    FitnessEquipment,
    OutdoorGear,
    SportsEquipment,
    Bicycles,
    WaterSports,
    WinterSports,
    CarParts,
    Motorcycles,
    AutoTools,
    CarAccessories,
    Books,
    Music,
    Movies,
    VideoGames,
    HealthEquipment,
    PersonalCare,
    Supplements,
    MedicalDevices,
    BabyClothing,
    Toys,
    BabyGear,
    KidsElectronics,
    Collectibles,
    Antiques,
    Art,
    Crafts,
    OfficeSupplies,
    IndustrialEquipment,
    BusinessEquipment,
    Other,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    YesNo,
    FreeResponse,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Question {
    pub id: String,
    pub question: String,
    pub question_type: QuestionType,
    pub mandatory: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProductQuestions {
    pub questions: Vec<Question>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GalleryItem {
    pub id: String,
    pub item_type: String,
    pub url: String,
    pub size: u64,
    pub order: u32,
    pub upload_timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProductQuantity {
    pub min_quantity: u32,
    pub max_quantity: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Product {
    pub product_id: String,
    pub user_id: String,
    pub username: String,
    pub title: String,
    pub description: String,
    pub product_type: ProductType,
    pub purchase_type: PurchaseType,
    pub category: ProductCategory,
    pub tags: Vec<String>,
    pub quantity: ProductQuantity,
    pub price: f64,
    pub custom_questions: Option<ProductQuestions>,
    #[serde(default)]
    pub gallery: Vec<GalleryItem>,
    pub thumbnail_url: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub created_at: u64,
    pub updated_at: u64,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateProductRequest {
    pub title: String,
    pub description: String,
    pub product_type: ProductType,
    pub purchase_type: PurchaseType,
    pub category: ProductCategory,
    #[serde(default)]
    pub tags: Vec<String>,
    pub quantity: ProductQuantity,
    pub price: f64,
    pub custom_questions: Option<ProductQuestions>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProductRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub product_type: Option<ProductType>,
    pub purchase_type: Option<PurchaseType>,
    pub category: Option<ProductCategory>,
    pub tags: Option<Vec<String>>,
    pub quantity: Option<ProductQuantity>,
    pub price: Option<f64>,
    pub custom_questions: Option<ProductQuestions>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductListItem {
    pub product_id: String,
    pub title: String,
    pub product_type: ProductType,
    pub quantity: ProductQuantity,
    pub created_at: u64,
    pub enabled: bool,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateQuestionsRequest {
    pub product_id: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateQuestionsPayload {
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqChatCompletion {
    pub model: String,
    pub messages: Vec<GroqMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub tools: Vec<GroqTool>,
    pub tool_choice: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqTool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: GroqFunction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqFunction {
    pub name: String,
    pub description: String,
    pub parameters: GroqFunctionParameters,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqFunctionParameters {
    #[serde(rename = "type")]
    pub param_type: String,
    pub properties: serde_json::Value,
    pub required: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
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
    pub tool_calls: Option<Vec<GroqToolCall>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: GroqToolCallFunction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClipCombinedRequest {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClipEmbeddingResponse {
    pub embedding: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReorderGalleryRequest {
    pub item_ids: Vec<String>,
}

#[derive(serde::Deserialize, Default)]
pub struct ListMyProductsQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Unpaid,
    DeliveryPending,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    pub order_id: String,
    pub product_id: String,
    pub seller_id: String,
    pub buyer_id: String,
    pub quantity: u32,
    pub price: f64,
    pub status: OrderStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuyNowRequest {
    pub product_id: String,
    pub quantity: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfirmOrderRequest {
    pub order_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateOrderFromQuoteRequest {
    pub message_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderResponse {
    pub order_id: String,
    pub product_id: String,
    pub seller_id: String,
    pub buyer_id: String,
    pub quantity: u32,
    pub price: f64,
    pub status: OrderStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(serde::Deserialize, Default)]
pub struct ListOrdersQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}
