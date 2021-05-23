use actix_web::{web, get, HttpResponse};
use actix_web_grants::proc_macro::has_permissions;
use serde::Serialize;
use crate::AppData;
use mysql::prelude::Queryable;
use mysql::Row;

#[derive(Serialize)]
pub struct Response {
    id: i64
}

#[get("/id/quote")]
#[has_permissions("QUOTE_READ")]
pub async fn get_quite_id(data: web::Data<AppData>) -> HttpResponse {
    let mut conn = match data.pool.get_conn() {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("Failed to create database connection: {:?}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let sql_row = match conn.query::<Row, &str>("SELECT id FROM quotes ORDER BY id DESC LIMIT 1") {
        Ok(row) => row,
        Err(err) => {
            eprintln!("Failed to query ID from quotes: {:?}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut id = 0;
    for row in sql_row {
        id = row.get::<i64, &str>("id").unwrap();
    }

    HttpResponse::Ok().json(Response { id })
}