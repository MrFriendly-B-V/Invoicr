use actix_web::{post, HttpResponse, web};
use serde::{Serialize, Deserialize};
use mysql::prelude::Queryable;
use mysql::{Row, Params, params};
use crate::AppData;

#[derive(Deserialize)]
pub struct Request {
    products: Vec<String>
}

#[derive(Serialize)]
pub struct Response {
    error: Option<String>
}

#[post("/products/del")]
pub async fn del_product(data: web::Data<AppData>, request: web::Json<Request>) -> HttpResponse {
    let conn = data.pool.get_conn();
    if conn.is_err() {
        eprintln!("Unable to create database connection: {:?}", conn.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    let mut conn = conn.unwrap();
    let sql_get_products = conn.query::<Row, &str>("SELECT id FROM products");
    if sql_get_products.is_err() {
        eprintln!("Unable to fetch products from the database: {:?}", sql_get_products.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    let mut product_ids: Vec<String> = Vec::new();
    for row in sql_get_products.unwrap() {
        let id = row.get("id").unwrap();
        product_ids.push(id);
    }

    for product_id in request.products.clone() {
        if !product_ids.contains(&product_id) {
            return HttpResponse::BadRequest().json(Response { error: Some(format!("Product with id '{}' does not exist!", product_id))});
        }

        let sql_delete_product = conn.exec::<usize, &str, Params>("DELETE FROM products WHERE id = :id", params! {
            "id" => product_id
        });

        if sql_delete_product.is_err() {
            eprintln!("Unable to delete product from database: {:?}", sql_delete_product.err().unwrap());
            return HttpResponse::InternalServerError().finish();
        }
    }

    HttpResponse::Ok().finish()
}