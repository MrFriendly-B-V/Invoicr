mod appdata;
mod endpoints;
mod apis;
mod threads;

use actix_web::{HttpServer, App};
use crate::appdata::{Config, AppData};

type Result<T> = std::result::Result<T, String>;

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    println!("Welcome to Invoicr by MrFriendly");
    let config = Config::read();
    let appdata = AppData::new(&config);
    if !appdata.check_db() {
        println!("Database check failed, some tables are missing. Creating them now.");
        appdata.init_db();
    }

    println!("Starting on port 8080");
    HttpServer::new(move || {
        let cors = actix_cors::Cors::permissive();
        App::new()
            .wrap(cors)
            .data(appdata.clone())
            .service(crate::endpoints::products::get::get_products)
            .service(crate::endpoints::products::add::add_product)
            .service(crate::endpoints::products::del::del_product)
            .service(crate::endpoints::persons::get::get_contacts)
            .service(crate::endpoints::pdf::invoice::create_invoice)
            .service(crate::endpoints::history::invoice::get_invoice_history)
            .service(crate::endpoints::history::quote::get_quote_history)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}