mod appdata;
mod endpoints;

use actix_web::{HttpServer, App};
use crate::appdata::{Config, AppData};

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    println!("Welcome to Invoicr by MrFriendly");
    let config = Config::read();
    let appdata = AppData::new(&config);
    if !appdata.check_db() {
        println!("Database check failed, some tables are missing. Creating them now.");
        appdata.init_db();
    }

    HttpServer::new(move || {
        let cors = actix_cors::Cors::permissive();
        App::new()
            .wrap(cors)
            .data(appdata.clone())
            .service(crate::endpoints::products::get::get_products)
            .service(crate::endpoints::products::add::add_product)
            .service(crate::endpoints::products::del::del_product)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}