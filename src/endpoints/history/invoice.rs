use actix_web::{get, web, HttpResponse};
use serde::Serialize;
use mysql::prelude::Queryable;
use mysql::Row;
use crate::appdata::AppData;

#[derive(Serialize)]
pub struct Response {
    invoices: Vec<Invoice>
}

#[derive(Serialize)]
pub struct Invoice {
    id:             i32,
    receiver:       String,
    is_paid:        bool
}

#[get("/history/invoice")]
pub async fn get_invoice_history(data: web::Data<AppData>) -> HttpResponse {
    let conn = data.pool.get_conn();
    if conn.is_err() {
        eprintln!("Unable to create database connection: {:?}", conn.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }
    let mut conn = conn.unwrap();

    let sql_get_invoices = conn.query::<Row, &str>("SELECT id,receiver,is_paid FROM invoices");
    if sql_get_invoices.is_err() {
        eprintln!("Unable to retrieve invoices from the database: {:?}", sql_get_invoices.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    let mut invoices = Vec::new();
    for row in sql_get_invoices.unwrap() {
        let id = row.get("id").unwrap();
        let receiver = row.get("receiver").unwrap();
        let is_paid = row.get("is_paid").unwrap();

        let invoice = Invoice {
            id,
            receiver,
            is_paid
        };

        invoices.push(invoice);
    }

    HttpResponse::Ok().json(Response { invoices })
}