use std::env;

use chrono::{NaiveDate, NaiveDateTime, Utc};
use futures::StreamExt;
use mongodb::bson::{doc, to_bson};
use mongodb::{options::ClientOptions, Client};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
struct Position {
    x: u64,
    y: u64,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Debug, Deserialize, Serialize)]
struct Date(NaiveDateTime);

impl Default for Date {
    fn default() -> Self {
        Date(NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0))
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct CanvasPixel {
    timestamp: Date,
    position: Position,
    color: Color,
    user: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct UserPixel {
    position: Position,
    color: Color,
}

pub async fn create_handle() -> mongodb::Collection<CanvasPixel> {
    let mongo_uri = env::var("MONGO_URI")
        .unwrap_or_else(|_| "mongodb://root:example@localhost:27017".to_string());
    let client_options = ClientOptions::parse(mongo_uri)
        .await
        .expect("Unable to connect to the database");
    let client = Client::with_options(client_options).unwrap();
    let db = client.database("bplace");

    db.collection::<CanvasPixel>("canvas")
}

pub async fn get_canvas() -> Result<Vec<CanvasPixel>, mongodb::error::Error> {
    let handle = create_handle().await;
    let cursor = handle.find(doc! {}, None).await?;
    let res: Vec<CanvasPixel> = cursor.map(|m| m.unwrap()).collect().await;
    Ok(res)
}

pub async fn create_pixel(
    new_pixel: UserPixel,
    username: String,
) -> Result<(), mongodb::error::Error> {
    let handle = create_handle().await;
    let position = to_bson(&new_pixel.position).unwrap();
    let new = CanvasPixel {
        timestamp: Date(Utc::now().naive_utc()),
        position: new_pixel.position,
        user: username,
        color: new_pixel.color,
    };
    let replaced = handle
        .find_one_and_replace(doc! { "position": position }, &new, None)
        .await?;
    if replaced.is_none() {
        handle.insert_one(new, None).await?;
    }
    Ok(())
}
