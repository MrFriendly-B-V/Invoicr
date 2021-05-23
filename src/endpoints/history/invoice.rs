use actix_web::{get, web, HttpResponse};
use actix_web_grants::proc_macro::has_permissions;
use serde::Serialize;
use mysql::prelude::Queryable;
use mysql::{Row, Params, params};
use crate::appdata::AppData;
use crate::apis::pdf::{PdfCommonPayload, Address, ItemRow};

#[derive(Serialize)]
pub struct Response {
    invoices: Vec<PdfCommonPayload>
}

#[get("/history/invoice")]
#[has_permissions("INVOICE_READ")]
pub async fn get_invoice_history(data: web::Data<AppData>) -> HttpResponse {
    let mut conn = match data.pool.get_conn() {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("Failed to create database connection: {:?}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let sql_get_invoice = match conn.query::<Row, &str>("SELECT * FROM invoices") {
        Ok(result) => result,
        Err(err) => {
            eprintln!("Failed to query invoices from the database: {:?}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut invoices = Vec::new();
    for row in sql_get_invoice {
        let id = row.get("id").unwrap();

        let sql_get_rows = conn.exec::<Row, &str, Params>("SELECT * FROM itemrows WHERE parent_id = :parent_id AND parent_type = :parent_type", params! {
            "parent_id" => &id,
            "parent_type" => "invoices"
        });
        if sql_get_rows.is_err() {
            eprintln!("Failed to query itemrows for invoice {}", &id);
            return HttpResponse::InternalServerError().finish();
        }

        let mut itemrows = Vec::new();
        for itemrow in sql_get_rows.unwrap() {
            let entry = ItemRow {
                id: itemrow.get("product_id").unwrap(),
                name: itemrow.get("name").unwrap(),
                comment: itemrow.get("comment"),
                description: itemrow.get("description").unwrap(),
                discount_perc: itemrow.get("discount_perc"),
                vat_perc: itemrow.get("vat_perc").unwrap(),
                price: itemrow.get("price").unwrap(),
                quantity: itemrow.get("quantity").unwrap()
            };

            itemrows.push(entry);
        }


        let result = PdfCommonPayload {
            id,
            template_name: row.get("template_name").unwrap(),
            language: row.get("language").unwrap(),
            attention_of: row.get("attention_of"),
            receiver: row.get("receiver").unwrap(),
            reference: row.get("reference").unwrap(),
            notes: row.get("notes"),
            expiry_date: row.get("expiry_date").unwrap(),
            creation_date: row.get("creation_date").unwrap(),
            address: Address {
                city: row.get("city").unwrap(),
                country: row.get("country").unwrap(),
                postal_code: row.get("postal_code").unwrap(),
                street: row.get("street").unwrap()
            },
            rows: itemrows
        };

        invoices.push(result);
    }

    HttpResponse::Ok().json(Response { invoices })
}