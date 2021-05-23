use actix_web::{get, web, HttpResponse};
use actix_web_grants::proc_macro::has_permissions;
use serde::Serialize;
use mysql::prelude::Queryable;
use mysql::{Row, Params, params};
use crate::appdata::AppData;
use crate::apis::pdf::{ItemRow, PdfQuotePayload, PdfCommonPayload, Address};

#[derive(Serialize)]
pub struct Response {
    quotes: Vec<PdfQuotePayload>
}

#[get("/history/quote")]
#[has_permissions("QUOTE_READ")]
pub async fn get_quote_history(data: web::Data<AppData>) -> HttpResponse {
    let mut conn = match data.pool.get_conn() {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("Failed to create database connection: {:?}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let sql_get_quotes = match conn.query::<Row, &str>("SELECT * FROM quotes") {
        Ok(result) => result,
        Err(err) => {
            eprintln!("Failed to query invoices from the database: {:?}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut quotes = Vec::new();
    for row in sql_get_quotes {
        let id = row.get("id").unwrap();

        let sql_get_rows = conn.exec::<Row, &str, Params>("SELECT * FROM itemrows WHERE parent_id = :parent_id AND parent_type = :parent_type", params! {
            "parent_id" => &id,
            "parent_type" => "quotes"
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


        let result = PdfQuotePayload {
            quote_topic: row.get("quote_topic").unwrap(),
            quote_contact_person: row.get("quote_contact_person").unwrap(),
            debit_id: row.get("debit_id").unwrap(),
            common: PdfCommonPayload {
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
                rows: itemrows,
            }
        };

        quotes.push(result);
    }

    HttpResponse::Ok().json(Response { quotes })
}