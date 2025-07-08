use axum::{
    Router,
    middleware::from_fn as middleware_from_fn,
    routing::{delete, get, post, put},
};
use mongodb::{Client, Database, options::ClientOptions};
use std::{env::var, net::SocketAddr, sync::OnceLock};
use dotenv::dotenv;

mod apex;
mod auth;
mod chat;
mod notifications;
mod orders;
mod products;
mod recommendations;
mod search;

use apex::endpoints::*;
use auth::endpoints::*;
use chat::endpoints::*;
use orders::endpoints::*;
use products::endpoints::*;
use recommendations::endpoints::{get_recommendations, get_knowledge_graph};
use search::endpoints::*;

pub(crate) static DB: OnceLock<Database> = OnceLock::new();

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

    let protected_routes = Router::new()
        .route("/auth/user", get(get_user))
        .route("/auth/logout", post(logout_user))
        .route("/auth/change-password", post(change_password_endpoint))
        .route("/auth/send-whatsapp-otp", post(send_whatsapp_otp_endpoint))
        .route(
            "/auth/verify-whatsapp-otp",
            post(verify_whatsapp_otp_endpoint),
        )
        .route("/auth/whatsapp-status", get(get_whatsapp_status))
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
        .route(
            "/chat/quotes/create-order",
            post(create_order_from_quote_endpoint),
        )
        .route("/products/buy-now", post(buy_now_endpoint))
        .route("/orders/list", get(list_orders_endpoint))
        .route("/orders/confirm", post(confirm_order_endpoint))
        .route("/sellers/orders/list", get(list_seller_orders_endpoint))
        .route("/homepage/recommendations", get(get_recommendations))
        .route("/homepage/knowledge-graph", get(get_knowledge_graph))
        .layer(middleware_from_fn(cookie_auth));

    let unprotected_routes = Router::new()
        .route("/auth/register", post(register_user))
        .route("/auth/login", post(login_user))
        .route("/auth/send-email-otp", post(send_email_otp_endpoint))
        .route("/auth/verify-email-otp", post(verify_email_otp_endpoint))
        .route("/products/{product_id}", get(get_product_endpoint))
        .route("/products/search", post(optimized_search_products_endpoint));

    let app = Router::new()
        .merge(protected_routes)
        .merge(unprotected_routes)
        .route("/", get(root_endpoint));

    axum::serve(listener, app).await.unwrap();
}
