use axum::{
    Json, Router,
    middleware::from_fn as middleware_from_fn,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use axum_csrf::{CsrfConfig, CsrfToken, Key};
use mongodb::{Client, Database, options::ClientOptions};
use std::{env::var, net::SocketAddr, sync::OnceLock};
use dotenv::dotenv;
use serde_json::json;

mod apex;
mod auth;
mod chat;
mod products;
mod search;

use apex::endpoints::*;
use auth::endpoints::*;
use chat::endpoints::*;
use products::endpoints::*;
use search::endpoints::*;

pub(crate) static DB: OnceLock<Database> = OnceLock::new();

async fn csrf_endpoint(token: CsrfToken) -> impl IntoResponse {
    Json(json!({ "csrf_token": token.authenticity_token().unwrap() }))
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let mongodb_uri = var("MONGODB_URI").unwrap();
    let client_options = ClientOptions::parse(mongodb_uri).await.unwrap();
    let client = Client::with_options(client_options).expect("Failed to create Mongo client");

    DB.set(client.database("goodspoint_main")).unwrap();

    let domain = var("DOMAIN").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("Failed to parse PORT");

    let addr = SocketAddr::from((
        domain
            .parse::<std::net::IpAddr>()
            .expect("Failed to parse DOMAIN"),
        port,
    ));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let cookie_key = Key::generate();
    let our_domain = var("DOMAIN").unwrap_or_else(|_| "localhost".to_string());
    let _config = CsrfConfig::default()
        .with_key(Some(cookie_key))
        .with_cookie_domain(Some(our_domain));

    let protected_routes = Router::new()
        .route("/auth/user", get(get_user))
        .route("/auth/logout", post(logout_user))
        .route("/seller/products/create", post(create_product_endpoint))
        .route("/seller/products/list", get(list_my_products_endpoint))
        .route(
            "/seller/products/{product_id}",
            get(get_user_product_endpoint),
        )
        .route(
            "/seller/products/{product_id}",
            put(update_product_endpoint),
        )
        .route(
            "/seller/products/{product_id}",
            delete(delete_product_endpoint),
        )
        .route(
            "/seller/products/{product_id}/gallery",
            get(get_gallery_endpoint),
        )
        .route(
            "/seller/products/{product_id}/gallery/replace",
            post(replace_gallery_endpoint),
        )
        .route(
            "/seller/products/{product_id}/gallery/add",
            post(add_gallery_items_endpoint),
        )
        .route(
            "/seller/products/{product_id}/gallery/reorder",
            post(reorder_gallery_endpoint),
        )
        .route(
            "/seller/products/{product_id}/questions",
            get(get_questions_endpoint),
        )
        .route(
            "/seller/products/{product_id}/questions/set",
            post(set_questions_endpoint),
        )
        .route(
            "/seller/products/{product_id}/questions/generate",
            post(generate_questions_endpoint),
        )
        .route("/chat/conversations", get(get_conversations_endpoint))
        .route(
            "/chat/{other_user_id}/messages",
            post(send_message_endpoint),
        )
        .route("/chat/{other_user_id}/messages", get(get_messages_endpoint))
        .route(
            "/chat/messages/{message_id}/edit",
            put(edit_message_endpoint),
        )
        .route(
            "/chat/messages/{message_id}/history",
            get(get_message_history_endpoint),
        )
        .layer(middleware_from_fn(cookie_auth));

    let unprotected_routes = Router::new()
        .route("/auth/register", post(register_user))
        .route("/auth/login", post(login_user))
        .route("/products/{product_id}", get(get_product_endpoint))
        .route("/products/search", post(optimized_search_products_endpoint))
        .route("/audio/transcribe", post(transcribe_audio_endpoint))
        .route("/audio/translate", post(translate_audio_endpoint))
        .route(
            "/api/knowledge-graph/{user_id}",
            get(render_knowledge_graph_endpoint),
        );

    let app = Router::new()
        .merge(protected_routes)
        .merge(unprotected_routes)
        .route("/", get(root_endpoint))
        .route("/auth/csrf_token", get(csrf_endpoint));

    axum::serve(listener, app).await.unwrap();
}
