use actix_web::{get, web, HttpResponse};
use serde::Serialize;
use mysql::prelude::Queryable;
use mysql::Row;
use crate::appdata::AppData;

#[derive(Serialize)]
pub struct Response {
    quotes: Vec<Quote>
}

#[derive(Serialize)]
pub struct Quote {
    id:             i32,
    invoice_id:     Option<i32>,
    receiver:       String,
    valid_until:    i64
}

#[get("/history/quotes")]
pub async fn get_quote_history(data: web::Data<AppData>) -> HttpResponse {
    let conn = data.pool.get_conn();
    if conn.is_err() {
        eprintln!("Unable to create database connection: {:?}", conn.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }
    let mut conn = conn.unwrap();

    let sql_get_invoices = conn.query::<Row, &str>("SELECT id,invoice_id,receiver,valid_until FROM quotes");
    if sql_get_invoices.is_err() {
        eprintln!("Unable to retrieve quotes from the database: {:?}", sql_get_invoices.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    let mut quotes = Vec::new();
    for row in sql_get_invoices.unwrap() {
        let id = row.get("id").unwrap();
        let invoice_id = row.get("invoice_id");
        let receiver = row.get("receiver").unwrap();
        let valid_until = row.get("valid_until").unwrap();

        let invoice = Quote {
            id,
            invoice_id,
            receiver,
            valid_until
        };

        quotes.push(invoice);
    }

    HttpResponse::Ok().json(Response { quotes })
}