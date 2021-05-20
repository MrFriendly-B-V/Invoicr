use actix_web::{post, web, HttpResponse};
use actix_web_grants::proc_macro::has_permissions;
use crate::AppData;
use crate::apis::pdf::{generate_quote, PdfGenerationResponse, PdfQuotePayload};
use mysql::prelude::Queryable;
use mysql::{Params, params, Row};
use rand::Rng;

#[post("/pdf/quote")]
#[has_permissions("QUOTE_CREATE")]
pub async fn create_quote(data: web::Data<AppData>, payload: web::Json<PdfQuotePayload>) -> HttpResponse {
    let mut conn = match data.pool.get_conn() {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("Failed to create database connection: {:?}", err.to_string());
            return HttpResponse::InternalServerError().finish();
        }
    };

    //Fetch all quote IDs to make sure we don't have double IDs
    let sql_get_quote_ids = match conn.query::<Row, &str>("SELECT id FROM quotes") {
        Ok(row) => row,
        Err(err) => {
            eprintln!("Failed to query quote IDs from the database: {:?}", err.to_string());
            return HttpResponse::InternalServerError().finish();
        }
    };

    let quote_ids = {
        let mut quote_ids = Vec::with_capacity(sql_get_quote_ids.len());
        for row in sql_get_quote_ids {
            let id = row.get::<i64, &str>("id").unwrap();
            quote_ids.push(id);
        }

        quote_ids
    };

    if quote_ids.contains(&payload.common.id) {
        return HttpResponse::Conflict().body(format!("Quote with ID {} already exists.", &payload.common.id));
    }

    let sql_create_quote = conn.exec::<usize, &str, Params>("INSERT INTO quotes \
        (id, template_name, language, attention_of, receiver, reference, notes, expiry_date, creation_date, city, country, postal_code, street, quote_topic, quote_contact_person, debit_id) \
        VALUES (:id, :template_name, :language, :attention_of, :receiver, :reference, :notes, :expiry_date, :creation_date, :city, :country, :postal_code, :street, :quote_topic, :quote_contact_person, :debit_id)", params! {

        "id" => &payload.common.id,
        "template_name" => &payload.common.template_name,
        "language" => &payload.common.language,
        "attention_of" => &payload.common.attention_of,
        "receiver" => &payload.common.receiver,
        "reference" => &payload.common.reference,
        "notes" => &payload.common.notes,
        "expiry_date" => &payload.common.expiry_date,
        "creation_date" => &payload.common.creation_date,
        "city" => &payload.common.address.city,
        "country" => &payload.common.address.country,
        "postal_code" => &payload.common.address.postal_code,
        "street" => &payload.common.address.street,
        "quote_topic" => &payload.quote_topic,
        "quote_contact_person" => &payload.quote_contact_person,
        "debit_id" => &payload.debit_id
    });

    if sql_create_quote.is_err() {
        eprintln!("Failed to create new quote in database: {:?}", sql_create_quote.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    for row in payload.common.rows.iter() {
        let id: String = rand::thread_rng().sample_iter(rand::distributions::Alphanumeric).take(32).map(char::from).collect();
        let sql_create_item_row = conn.exec::<usize, &str, Params>("INSERT INTO itemrows \
            (id, parent_id, parent_type, comment, name, description, discount_perc, vat_perc, price, quantity) \
            VALUES (:id, :parent_id, :parent_type,:comment, :name, :description, :discount_perc, :vat_perc, :price, :quantity)", params! {

            "id" => id,
            "parent_id" => &payload.common.id,
            "parent_type" => "quotes",
            "comment" => &row.comment,
            "name" => &row.name,
            "description" => &row.description,
            "discount_perc" => &row.discount_perc,
            "vat_perc" => &row.vat_perc,
            "price" => &row.price,
            "quantity" => &row.quantity
        });

        if sql_create_item_row.is_err() {
            eprintln!("Failed to insert ItemRow into database: {:?}", sql_create_item_row.err().unwrap());
            return HttpResponse::InternalServerError().finish();
        }
    }

    //Generate the PDF
    let id = match generate_quote(&data.config, &*payload).await {
        Ok(id) => id,
        Err(err) => {
            eprintln!("Failed to send Quote generation request: {:?}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };

    HttpResponse::Ok().json(PdfGenerationResponse { id: Some(id), error: None })
}