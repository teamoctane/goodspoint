use serde::{Deserialize, Deserializer, Serialize};

fn ascii_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if !s.is_ascii() {
        return Err(serde::de::Error::custom("non-ASCII characters not allowed"));
    }
    Ok(s)
}

fn ascii_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let vec = Vec::<String>::deserialize(deserializer)?;
    for s in &vec {
        if !s.is_ascii() {
            return Err(serde::de::Error::custom("non-ASCII characters not allowed"));
        }
    }
    Ok(vec)
}

fn ascii_option_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    if let Some(ref s) = opt {
        if !s.is_ascii() {
            return Err(serde::de::Error::custom("non-ASCII characters not allowed"));
        }
    }
    Ok(opt)
}

fn ascii_option_vec<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<Vec<String>>::deserialize(deserializer)?;
    if let Some(ref vec) = opt {
        for s in vec {
            if !s.is_ascii() {
                return Err(serde::de::Error::custom("non-ASCII characters not allowed"));
            }
        }
    }
    Ok(opt)
}

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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ProductType {
    New,
    Used,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    YesNo,
    FreeResponse,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Question {
    #[serde(deserialize_with = "ascii_string")]
    pub id: String,
    #[serde(deserialize_with = "ascii_string")]
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
    pub title: String,
    pub description: String,
    pub product_type: ProductType,
    pub category: ProductCategory,
    pub tags: Vec<String>,
    pub quantity: ProductQuantity,
    pub price: Option<String>,
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
    #[serde(deserialize_with = "ascii_string")]
    pub title: String,
    #[serde(deserialize_with = "ascii_string")]
    pub description: String,
    pub product_type: ProductType,
    pub category: ProductCategory,
    #[serde(default, deserialize_with = "ascii_vec")]
    pub tags: Vec<String>,
    pub quantity: ProductQuantity,
    pub price: Option<String>,
    pub custom_questions: Option<ProductQuestions>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProductRequest {
    #[serde(deserialize_with = "ascii_option_string")]
    pub title: Option<String>,
    #[serde(deserialize_with = "ascii_option_string")]
    pub description: Option<String>,
    pub product_type: Option<ProductType>,
    pub category: Option<ProductCategory>,
    #[serde(deserialize_with = "ascii_option_vec")]
    pub tags: Option<Vec<String>>,
    pub quantity: Option<ProductQuantity>,
    pub price: Option<String>,
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
    #[serde(deserialize_with = "ascii_string")]
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
