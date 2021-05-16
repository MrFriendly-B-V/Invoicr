pub mod get;
pub mod add;
pub mod del;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Product {
    id:             Option<String>,
    name:           String,
    description:    String,
    price:          f64
}