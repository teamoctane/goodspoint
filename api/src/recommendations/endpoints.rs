use axum::{Extension, response::Json};

use super::{delegates, schemas::*};
use crate::{
    apex::utils::VerboseHTTPError, auth::schemas::UserOut, products::schemas::ProductCategory,
};

pub async fn get_recommendations(
    Extension(user): Extension<UserOut>,
) -> Result<Json<RecommendationResponse>, VerboseHTTPError> {
    let recommendations = delegates::get_recommendations(&user).await?;
    Ok(Json(recommendations))
}

pub async fn get_knowledge_graph(
    Extension(user): Extension<UserOut>,
) -> Result<Json<KnowledgeGraphData>, VerboseHTTPError> {
    let kg_data = delegates::get_knowledge_graph_data(&user.uid).await?;
    Ok(Json(kg_data))
}



pub async fn auto_log_signal(
    user_id: &str,
    signal_type: SignalType,
    category: ProductCategory,
    product_id: Option<String>,
    search_query: Option<String>,
) {
    let signal_log = SignalLog {
        user_id: user_id.to_string(),
        category,
        signal_type,
        product_id,
        search_query,
    };

    let _ = delegates::process_signal(signal_log).await;
}
