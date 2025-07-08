use axum::http::StatusCode;
use futures::TryStreamExt;
use mongodb::{Collection, bson::doc};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use super::schemas::*;
use crate::{DB, apex::utils::VerboseHTTPError, auth::schemas::UserOut};

pub async fn list_orders(
    user: &UserOut,
    limit: u32,
    offset: u32,
) -> Result<Vec<OrderResponse>, VerboseHTTPError> {
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    let collection: Collection<Order> = database.collection(COLLECTIONS_ORDERS);

    let cursor = collection
        .find(doc! { "buyer_id": &user.uid })
        .skip(offset as u64)
        .limit(limit as i64)
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    let orders: Vec<Order> = cursor.try_collect().await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    Ok(orders
        .into_iter()
        .map(|order| OrderResponse {
            order_id: order.order_id,
            product_id: order.product_id,
            seller_id: order.seller_id,
            buyer_id: order.buyer_id,
            quantity: order.quantity,
            price: order.price,
            status: order.status,
            created_at: order.created_at,
            updated_at: order.updated_at,
        })
        .collect())
}

pub async fn list_seller_orders(
    user: &UserOut,
    limit: u32,
    offset: u32,
) -> Result<Vec<OrderResponse>, VerboseHTTPError> {
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    let collection: Collection<Order> = database.collection(COLLECTIONS_ORDERS);

    let cursor = collection
        .find(doc! { "seller_id": &user.uid })
        .skip(offset as u64)
        .limit(limit as i64)
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    let orders: Vec<Order> = cursor.try_collect().await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    Ok(orders
        .into_iter()
        .map(|order| OrderResponse {
            order_id: order.order_id,
            product_id: order.product_id,
            seller_id: order.seller_id,
            buyer_id: order.buyer_id,
            quantity: order.quantity,
            price: order.price,
            status: order.status,
            created_at: order.created_at,
            updated_at: order.updated_at,
        })
        .collect())
}

pub async fn confirm_order(
    user: &UserOut,
    order_id: String,
) -> Result<OrderResponse, VerboseHTTPError> {
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    let collection: Collection<Order> = database.collection(COLLECTIONS_ORDERS);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let update_result = collection
        .find_one_and_update(
            doc! {
                "order_id": &order_id,
                "buyer_id": &user.uid,
                "status": "unpaid"
            },
            doc! {
                "$set": {
                    "status": "delivery_pending",
                    "updated_at": now as i64
                }
            },
        )
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    match update_result {
        Some(order) => Ok(OrderResponse {
            order_id: order.order_id,
            product_id: order.product_id,
            seller_id: order.seller_id,
            buyer_id: order.buyer_id,
            quantity: order.quantity,
            price: order.price,
            status: OrderStatus::DeliveryPending,
            created_at: order.created_at,
            updated_at: now,
        }),
        None => Err(VerboseHTTPError::Standard(
            StatusCode::NOT_FOUND,
            "Order not found or not eligible for confirmation".to_string(),
        )),
    }
}

pub async fn create_order_internal(
    product_id: String,
    seller_id: String,
    buyer_id: String,
    quantity: u32,
    price: f64,
) -> Result<OrderResponse, VerboseHTTPError> {
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    let collection: Collection<Order> = database.collection(COLLECTIONS_ORDERS);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let order_id = Uuid::new_v4().to_string();

    let order = Order {
        order_id: order_id.clone(),
        product_id: product_id.clone(),
        seller_id: seller_id.clone(),
        buyer_id: buyer_id.clone(),
        quantity,
        price,
        status: OrderStatus::Unpaid,
        created_at: now,
        updated_at: now,
    };

    collection.insert_one(&order).await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create order".to_string(),
        )
    })?;

    Ok(OrderResponse {
        order_id,
        product_id,
        seller_id,
        buyer_id,
        quantity,
        price,
        status: OrderStatus::Unpaid,
        created_at: now,
        updated_at: now,
    })
}
