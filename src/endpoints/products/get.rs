use actix_web::{HttpResponse, web, get};
use serde::Serialize;
use mysql::prelude::Queryable;
use mysql::Row;
use crate::appdata::AppData;
use crate::endpoints::products::Product;

#[derive(Serialize)]
pub struct Response {
    products: Vec<Product>
}

#[get("/products/get")]
pub async fn get_products(data: web::Data<AppData>) -> HttpResponse {
    let conn = data.pool.get_conn();
    if conn.is_err() {
        eprintln!("Unable to create a database connection: {:?}", conn.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    let mut conn = conn.unwrap();

    let sql_products = conn.query::<Row, &str>("SELECT id,name,description,price FROM products");
    if sql_products.is_err() {
        eprintln!("Unable to fetch products from the database: {:?}", sql_products.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    let mut products = Vec::new();
    for row in sql_products.unwrap() {
        let id = row.get("id").unwrap();
        let name = row.get("name").unwrap();
        let description = row.get("description").unwrap();
        let price = row.get("price").unwrap();

        products.push(Product {
            id: Some(id),
            name,
            description,
            price
        });
    }

    HttpResponse::Ok().json(Response { products })
}