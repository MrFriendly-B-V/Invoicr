use actix_web::{HttpResponse, web, post};
use actix_web_grants::proc_macro::has_permissions;
use serde::{Serialize, Deserialize};
use mysql::prelude::Queryable;
use mysql::{Row, Params, params};
use crate::AppData;
use crate::endpoints::products::Product;
use rand::Rng;
use std::future::Future;
use std::pin::Pin;

#[derive(Deserialize)]
pub struct Request {
    products: Vec<Product>
}

#[derive(Serialize)]
pub struct Response {
    error: Option<String>
}

#[post("/products/add")]
#[has_permissions("PRODUCTS_WRITE")]
pub async fn add_product(data: web::Data<AppData>, request: web::Json<Request>) -> HttpResponse {
    let conn = data.pool.get_conn();
    if conn.is_err() {
        eprintln!("Unable to create database connection: {:?}", conn.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    let mut conn = conn.unwrap();

    let sql_get_products = conn.query::<Row, &str>("SELECT name FROM products");
    if sql_get_products.is_err() {
        eprintln!("Unable to fetch products from the database: {:?}", sql_get_products.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    let mut product_names: Vec<String> = Vec::new();
    for row in sql_get_products.unwrap() {
        let name = row.get("name").unwrap();
        product_names.push(name);
    }

    let mut processors: Vec<Box<dyn Future<Output=(bool, Option<mysql::Error>)>>> = Vec::new();
    for product in request.products.clone() {
        if product_names.contains(&product.name) {
            return HttpResponse::BadRequest().json(Response { error: Some(format!("Product with name '{}' already exists!", &product.name))});
        }

        let product_clone = product.clone();
        let pool = data.pool.clone();

        let processor = async move {
            let mut conn = pool.get_conn().unwrap();
            let id: String = rand::thread_rng().sample_iter(rand::distributions::Alphanumeric).take(32).map(char::from).collect();
            let sql_insert_product=  conn.exec::<usize, &str, Params>("INSERT INTO products (id, name, description, price) VALUES (:id, :name, :description, :price)", params! {
                "id" => id,
                "name" => product_clone.name.clone(),
                "description" => product_clone.description.clone(),
                "price" => product_clone.price
            });

            (sql_insert_product.is_ok(), sql_insert_product.err())
        };

        processors.push(Box::new(processor));
    }

    for processor in processors {
        let success = Pin::from(processor).await;
        if !success.0 {
            eprintln!("Unable to add product to the database: {:?}", success.1.unwrap());
            return HttpResponse::InternalServerError().finish();
        }
    }

    HttpResponse::Ok().finish()
}
