pub mod get;
pub mod add;
pub mod del;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Product {
    pub id:             Option<String>,
    pub name:           String,
    pub description:    String,
    pub price:          f64
}