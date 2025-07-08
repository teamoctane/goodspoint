use axum::http::StatusCode;
use futures::TryStreamExt;
use mongodb::{
    Collection,
    bson::{DateTime as BsonDateTime, doc},
};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::schemas::*;
use crate::{
    DB,
    apex::utils::VerboseHTTPError,
    auth::schemas::UserOut,
    products::schemas::{Product, ProductCategory},
};

impl SignalType {
    pub fn boost_value(&self) -> f64 {
        match self {
            SignalType::Query => TIER_1_BOOST,
            SignalType::ProductView => TIER_2_BOOST,
            SignalType::Search => TIER_3_BOOST,
        }
    }

    pub fn decay_value(&self) -> f64 {
        match self {
            SignalType::Query => TIER_1_DECAY,
            SignalType::ProductView => TIER_2_DECAY,
            SignalType::Search => TIER_3_DECAY,
        }
    }
}



pub async fn apply_time_decay(user_id: &str) -> Result<(), VerboseHTTPError> {
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    let collection: Collection<UserCategorySignal> =
        database.collection(COLLECTIONS_USER_CATEGORY_SIGNALS);
    let now = BsonDateTime::now();
    let now_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let cursor = collection
        .find(doc! { "user_id": user_id })
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    let signals: Vec<UserCategorySignal> = cursor.try_collect().await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    for signal in signals {
        let last_decay_timestamp = signal.last_decay_check.timestamp_millis() / 1000;
        let days_since_decay = (now_timestamp as i64 - last_decay_timestamp) / 86400;

        if days_since_decay > 0 {
            let decay_factor = TIME_DECAY_FACTOR.powi(days_since_decay as i32);
            let new_strength = (signal.signal_strength * decay_factor).max(MIN_EDGE_WEIGHT);

            collection
                .update_one(
                    doc! { "_id": signal.id },
                    doc! {
                        "$set": {
                            "signal_strength": new_strength,
                            "last_decay_check": now
                        }
                    },
                )
                .await
                .map_err(|_| {
                    VerboseHTTPError::Standard(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to update signal".to_string(),
                    )
                })?;
        }
    }

    Ok(())
}

pub async fn process_signal(signal_log: SignalLog) -> Result<(), VerboseHTTPError> {
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    apply_time_decay(&signal_log.user_id).await?;

    let signals_collection: Collection<UserCategorySignal> =
        database.collection(COLLECTIONS_USER_CATEGORY_SIGNALS);
    let now = BsonDateTime::now();
    
    let relationships = super::schemas::get_category_relationships();

    let existing_signal = signals_collection
        .find_one(doc! {
            "user_id": &signal_log.user_id,
            "category": format!("{:?}", signal_log.category)
        })
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    let boost = signal_log.signal_type.boost_value();
    let decay = signal_log.signal_type.decay_value();

    if let Some(mut signal) = existing_signal {
        signal.signal_strength += boost;
        signal.last_updated = now;
        signal.last_decay_check = now;

        signals_collection
            .replace_one(doc! { "_id": signal.id }, &signal)
            .await
            .map_err(|_| {
                VerboseHTTPError::Standard(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to update signal".to_string(),
                )
            })?;
    } else {
        let initial_strength = MIN_EDGE_WEIGHT + boost;
        
        let new_signal = UserCategorySignal {
            id: None,
            user_id: signal_log.user_id.clone(),
            category: signal_log.category,
            signal_strength: initial_strength,
            last_updated: now,
            last_decay_check: now,
        };

        signals_collection
            .insert_one(&new_signal)
            .await
            .map_err(|_| {
                VerboseHTTPError::Standard(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to create signal".to_string(),
                )
            })?;
    }

    let all_user_signals = signals_collection
        .find(doc! { "user_id": &signal_log.user_id })
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .try_collect::<Vec<UserCategorySignal>>()
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    let mut related_categories: HashMap<ProductCategory, f64> = HashMap::new();
    for rel in relationships {
        if rel.category_a == signal_log.category {
            related_categories.insert(rel.category_b, rel.relationship_strength);
        }
        if rel.bidirectional && rel.category_b == signal_log.category {
            related_categories.insert(rel.category_a, rel.relationship_strength);
        }
    }

    for mut user_signal in all_user_signals {
        if user_signal.category == signal_log.category {
            continue;
        }

        if let Some(relationship_strength) = related_categories.get(&user_signal.category) {
            let related_boost = boost * relationship_strength;
            user_signal.signal_strength += related_boost;
        } else {
            user_signal.signal_strength =
                (user_signal.signal_strength - decay).max(MIN_EDGE_WEIGHT);
        }

        user_signal.last_updated = now;

        signals_collection
            .replace_one(doc! { "_id": user_signal.id }, &user_signal)
            .await
            .map_err(|_| {
                VerboseHTTPError::Standard(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to update related signal".to_string(),
                )
            })?;
    }

    Ok(())
}

pub async fn get_recommendations(
    user: &UserOut,
) -> Result<RecommendationResponse, VerboseHTTPError> {
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    apply_time_decay(&user.uid).await?;

    let signals_collection: Collection<UserCategorySignal> =
        database.collection(COLLECTIONS_USER_CATEGORY_SIGNALS);
    let products_collection: Collection<Product> = database.collection("products");

    let mut rows = Vec::new();

    let strongest_signal = signals_collection
        .find(doc! { "user_id": &user.uid })
        .sort(doc! { "signal_strength": -1 })
        .limit(1)
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .try_collect::<Vec<UserCategorySignal>>()
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .into_iter()
        .next();

    if let Some(signal) = strongest_signal {
        let category_str = format!("{:?}", signal.category);

        let cursor = products_collection
            .find(doc! {
                "category": &category_str,
                "enabled": true
            })
            .await
            .map_err(|_| {
                VerboseHTTPError::Standard(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            })?;

        let mut products: Vec<Product> = cursor.try_collect().await.map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

        let mut rng = rand::thread_rng();
        products.shuffle(&mut rng);

        let category_products: Vec<ProductSummary> = products
            .into_iter()
            .take(6)
            .map(|product| ProductSummary {
                product_id: product.product_id,
                title: product.title,
                price_in_inr: Some(product.price),
                thumbnail_url: product.thumbnail_url,
                category: category_str.clone(),
                relevance_score: 1.0,
            })
            .collect();

        if !category_products.is_empty() {
            rows.push(RecommendationRow {
                title: format!(
                    "Products in the {} category",
                    category_str.replace("Category::", "")
                ),
                products: category_products,
            });
        }
    } else {
        let cursor = products_collection
            .find(doc! { "enabled": true })
            .sort(doc! { "created_at": -1 })
            .limit(6)
            .await
            .map_err(|_| {
                VerboseHTTPError::Standard(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            })?;

        let latest_products: Vec<ProductSummary> = cursor
            .try_collect::<Vec<Product>>()
            .await
            .map_err(|_| {
                VerboseHTTPError::Standard(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            })?
            .into_iter()
            .map(|product| ProductSummary {
                product_id: product.product_id,
                title: product.title,
                price_in_inr: Some(product.price),
                thumbnail_url: product.thumbnail_url,
                category: format!("{:?}", product.category),
                relevance_score: 1.0,
            })
            .collect();

        if !latest_products.is_empty() {
            rows.push(RecommendationRow {
                title: "Latest Products".to_string(),
                products: latest_products,
            });
        }
    }

    Ok(RecommendationResponse {
        user_id: user.uid.clone(),
        rows,
        generated_at: BsonDateTime::now(),
    })
}



pub async fn get_knowledge_graph_data(
    user_id: &str,
) -> Result<KnowledgeGraphData, VerboseHTTPError> {
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    apply_time_decay(user_id).await?;
    
    let relationships = super::schemas::get_category_relationships();

    let signals_collection: Collection<UserCategorySignal> =
        database.collection(COLLECTIONS_USER_CATEGORY_SIGNALS);

    let cursor = signals_collection
        .find(doc! { "user_id": user_id })
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    let signals: Vec<UserCategorySignal> = cursor.try_collect().await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut total_signal_strength = 0.0;
    let mut strongest_category = None;
    let mut max_strength = 0.0;
    
    let mut all_categories = std::collections::HashSet::new();
    
    for rel in &relationships {
        all_categories.insert(rel.category_a);
        all_categories.insert(rel.category_b);
    }
    
    for category in all_categories {
        let category_str = format!("{:?}", category);
        let node_id = format!("category:{}", category_str);
        
        if !nodes.iter().any(|n: &GraphNode| n.id == node_id) {
            nodes.push(GraphNode {
                id: node_id,
                label: category_str.replace("Category::", ""),
                node_type: "category".to_string(),
                weight: 0.5,
            });
        }
    }
    
    for rel in &relationships {
        let cat_a_str = format!("{:?}", rel.category_a);
        let cat_b_str = format!("{:?}", rel.category_b);
        
        edges.push(GraphEdge {
            source: format!("category:{}", cat_a_str),
            target: format!("category:{}", cat_b_str),
            weight: rel.relationship_strength,
            last_updated: BsonDateTime::now(),
        });
        
        if rel.bidirectional {
            edges.push(GraphEdge {
                source: format!("category:{}", cat_b_str),
                target: format!("category:{}", cat_a_str),
                weight: rel.relationship_strength,
                last_updated: BsonDateTime::now(),
            });
        }
    }

    nodes.push(GraphNode {
        id: format!("user:{}", user_id),
        label: "You".to_string(),
        node_type: "user".to_string(),
        weight: 1.0,
    });

    for signal in signals {
        let category_str = format!("{:?}", signal.category);
        total_signal_strength += signal.signal_strength;

        if signal.signal_strength > max_strength {
            max_strength = signal.signal_strength;
            strongest_category = Some(category_str.clone());
        }

        let node_id = format!("category:{}", category_str);
        if let Some(existing_node) = nodes.iter_mut().find(|n| n.id == node_id) {
            existing_node.weight = signal.signal_strength;
        } else {
            nodes.push(GraphNode {
                id: node_id.clone(),
                label: category_str.replace("Category::", ""),
                node_type: "category".to_string(),
                weight: signal.signal_strength,
            });
        }

        edges.push(GraphEdge {
            source: format!("user:{}", user_id),
            target: node_id,
            weight: signal.signal_strength,
            last_updated: signal.last_updated,
        });
    }

    let category_count = nodes.iter().filter(|n| n.node_type == "category").count();

    
    Ok(KnowledgeGraphData {
        user_id: user_id.to_string(),
        nodes,
        edges,
        stats: KgStats {
            total_categories: category_count,
            strongest_category,
            total_signal_strength,
        },
    })
}
