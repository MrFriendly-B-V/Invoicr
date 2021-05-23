use actix_web::{post, web, HttpResponse};
use actix_web_grants::proc_macro::has_permissions;
use crate::AppData;
use crate::apis::pdf::{PdfCommonPayload, generate_invoice, PdfGenerationResponse};
use mysql::prelude::Queryable;
use mysql::{Params, params, Row};
use rand::Rng;

#[post("/pdf/invoice")]
#[has_permissions("INVOICE_CREATE")]
pub async fn create_invoice(data: web::Data<AppData>, payload: web::Json<PdfCommonPayload>) -> HttpResponse {
    let mut conn = match data.pool.get_conn() {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("Failed to create database connection: {:?}", err.to_string());
            return HttpResponse::InternalServerError().finish();
        }
    };

    //Fetch all invoice IDs to make sure we don't have double IDs
    let sql_get_invoice_ids = match conn.query::<Row, &str>("SELECT id FROM invoices") {
        Ok(row) => row,
        Err(err) => {
            eprintln!("Failed to query invoice IDs from the database: {:?}", err.to_string());
            return HttpResponse::InternalServerError().finish();
        }
    };

    let invoice_ids = {
        let mut invoice_ids = Vec::with_capacity(sql_get_invoice_ids.len());
        for row in sql_get_invoice_ids {
            let id = row.get::<i64, &str>("id").unwrap();
            invoice_ids.push(id);
        }

        invoice_ids
    };

    if invoice_ids.contains(&payload.id) {
        return HttpResponse::Conflict().body(format!("Invoice with ID {} already exists.", &payload.id));
    }

    let sql_create_invoice = conn.exec::<usize, &str, Params>("INSERT INTO invoices \
        (id, template_name, language, attention_of, receiver, reference, notes, expiry_date, creation_date, city, country, postal_code, street) \
        VALUES (:id, :template_name, :language, :attention_of, :receiver, :reference, :notes, :expiry_date, :creation_date, :city, :country, :postal_code, :street)", params! {

        "id" => &payload.id,
        "template_name" => &payload.template_name,
        "language" => &payload.language,
        "attention_of" => &payload.attention_of,
        "receiver" => &payload.receiver,
        "reference" => &payload.reference,
        "notes" => &payload.notes,
        "expiry_date" => &payload.expiry_date,
        "creation_date" => &payload.creation_date,
        "city" => &payload.address.city,
        "country" => &payload.address.country,
        "postal_code" => &payload.address.postal_code,
        "street" => &payload.address.street
    });

    if sql_create_invoice.is_err() {
        eprintln!("Failed to create new invoice in database: {:?}", sql_create_invoice.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    for row in payload.rows.iter() {
        let id: String = rand::thread_rng().sample_iter(rand::distributions::Alphanumeric).take(32).map(char::from).collect();
        let sql_create_item_row = conn.exec::<usize, &str, Params>("INSERT INTO itemrows \
            (id, product_id, parent_id, parent_type, comment, name, description, discount_perc, vat_perc, price, quantity) \
            VALUES (:id, :product_id, :parent_id, :parent_type, :comment, :name, :description, :discount_perc, :vat_perc, :price, :quantity)", params! {

            "id" => id,
            "product_id" => &row.id,
            "parent_id" => &payload.id,
            "parent_type" => "invoices",
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
    let id = match generate_invoice(&data.config, &*payload).await {
        Ok(id) => id,
        Err(err) => {
            eprintln!("Failed to send Invoice generation request: {:?}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };

    HttpResponse::Ok().json(PdfGenerationResponse { id: Some(id), error: None })
}