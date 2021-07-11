mod appdata;
mod endpoints;
mod apis;
mod threads;
mod authenticator;

use crate::appdata::{Config, AppData};
use actix_web::{HttpServer, App};
use actix_web_grants::GrantsMiddleware;

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

    println!("Starting on port 8090");
    HttpServer::new(move || {
        let _cors = actix_cors::Cors::default()
            .allow_any_header()
            .allowed_methods(vec!["GET", "POST"])
            .allowed_origin("http://localhost")
            .allowed_origin("https://invoicr.intern.mrfriendly.nl");

        let cors = actix_cors::Cors::permissive()
            .allow_any_origin()
            .allow_any_header()
            .allow_any_method();
        let auth = GrantsMiddleware::with_extractor(authenticator::check_permission);

        App::new()
            .wrap(cors)
            .wrap(auth)
            .data(appdata.clone())
            .service(crate::endpoints::products::get::get_products)
            .service(crate::endpoints::products::add::add_product)
            .service(crate::endpoints::products::del::del_product)
            .service(crate::endpoints::persons::get::get_contacts)
            .service(crate::endpoints::pdf::invoice::create_invoice)
            .service(crate::endpoints::pdf::quote::create_quote)
            .service(crate::endpoints::history::invoice::get_invoice_history)
            .service(crate::endpoints::history::quote::get_quote_history)
            .service(crate::endpoints::ids::quote::get_quite_id)
    })
    .bind("0.0.0.0:8090")?
    .run()
    .await
}