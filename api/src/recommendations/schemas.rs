use crate::products::schemas::ProductCategory;
use mongodb::bson::{DateTime, oid::ObjectId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    Query,
    ProductView,
    Search,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserCategorySignal {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: String,
    pub category: ProductCategory,
    pub signal_strength: f64,
    pub last_updated: DateTime,
    pub last_decay_check: DateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategoryRelationship {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub category_a: ProductCategory,
    pub category_b: ProductCategory,
    pub relationship_strength: f64,
    pub bidirectional: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserLastProduct {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: String,
    pub product_id: String,
    pub product_title: String,
    pub visited_at: DateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignalLog {
    pub user_id: String,
    pub category: ProductCategory,
    pub signal_type: SignalType,
    pub product_id: Option<String>,
    pub search_query: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProductSummary {
    pub product_id: String,
    pub title: String,
    pub price_in_inr: Option<f64>,
    pub thumbnail_url: Option<String>,
    pub category: String,
    pub relevance_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecommendationRow {
    pub title: String,
    pub products: Vec<ProductSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecommendationResponse {
    pub user_id: String,
    pub rows: Vec<RecommendationRow>,
    pub generated_at: DateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductViewLog {
    pub product_id: String,
    pub duration_seconds: Option<u32>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub weight: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub weight: f64,
    pub last_updated: DateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KnowledgeGraphData {
    pub user_id: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub stats: KgStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KgStats {
    pub total_categories: usize,
    pub strongest_category: Option<String>,
    pub total_signal_strength: f64,
}

pub const MIN_EDGE_WEIGHT: f64 = 1.0;

pub fn get_category_relationships() -> Vec<CategoryRelationship> {
    vec![
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::UnisexClothing,
            category_b: ProductCategory::MensClothing,
            relationship_strength: 0.5,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::UnisexClothing,
            category_b: ProductCategory::WomensClothing,
            relationship_strength: 0.5,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::MensClothing,
            category_b: ProductCategory::Shoes,
            relationship_strength: 0.3,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::WomensClothing,
            category_b: ProductCategory::Shoes,
            relationship_strength: 0.3,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::MensClothing,
            category_b: ProductCategory::Accessories,
            relationship_strength: 0.2,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::WomensClothing,
            category_b: ProductCategory::Accessories,
            relationship_strength: 0.2,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::WomensClothing,
            category_b: ProductCategory::Jewelry,
            relationship_strength: 0.3,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::WomensClothing,
            category_b: ProductCategory::Bags,
            relationship_strength: 0.3,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::WomensClothing,
            category_b: ProductCategory::Beauty,
            relationship_strength: 0.2,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::Smartphones,
            category_b: ProductCategory::Accessories,
            relationship_strength: 0.4,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::Computers,
            category_b: ProductCategory::Gaming,
            relationship_strength: 0.3,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::Audio,
            category_b: ProductCategory::Smartphones,
            relationship_strength: 0.2,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::Gaming,
            category_b: ProductCategory::Audio,
            relationship_strength: 0.2,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::Cameras,
            category_b: ProductCategory::Computers,
            relationship_strength: 0.2,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::Furniture,
            category_b: ProductCategory::HomeDecor,
            relationship_strength: 0.4,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::Kitchen,
            category_b: ProductCategory::HomeDecor,
            relationship_strength: 0.2,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::Garden,
            category_b: ProductCategory::HomeTools,
            relationship_strength: 0.3,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::HomeTools,
            category_b: ProductCategory::HomeImprovement,
            relationship_strength: 0.5,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::FitnessEquipment,
            category_b: ProductCategory::OutdoorGear,
            relationship_strength: 0.3,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::SportsEquipment,
            category_b: ProductCategory::FitnessEquipment,
            relationship_strength: 0.4,
            bidirectional: true,
        },
        CategoryRelationship {
            id: None,
            category_a: ProductCategory::Bicycles,
            category_b: ProductCategory::OutdoorGear,
            relationship_strength: 0.3,
            bidirectional: true,
        },
    ]
}
pub const TIER_1_BOOST: f64 = 3.0;
pub const TIER_2_BOOST: f64 = 2.0;
pub const TIER_3_BOOST: f64 = 1.0;
pub const TIER_1_DECAY: f64 = 0.3;
pub const TIER_2_DECAY: f64 = 0.2;
pub const TIER_3_DECAY: f64 = 0.1;
pub const TIME_DECAY_FACTOR: f64 = 0.95;

pub const COLLECTIONS_USER_CATEGORY_SIGNALS: &str = "user_category_signals";
