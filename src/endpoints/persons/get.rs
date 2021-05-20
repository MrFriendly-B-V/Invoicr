use actix_web::{web, get, HttpResponse};
use actix_web_grants::proc_macro::has_permissions;
use serde::Serialize;
use crate::appdata::AppData;
use crate::threads::espocrm::Communication;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct Response {
    contacts: Vec<Contact>
}

#[derive(Serialize)]
pub struct Contact {
    relation_name:  String,
    city:           String,
    street:         String,
    postal:         String,
    country:        String,
    contact_name:   String
}

#[get("/persons/get")]
#[has_permissions("PERSONS_READ")]
pub async fn get_contacts(data: web::Data<AppData>) -> HttpResponse {
    let contacts = Communication::query_contact(&data.espocrm_data);
    let accounts = Communication::query_account(&data.espocrm_data);

    let mut account_ids = HashMap::new();
    for account in accounts {
        account_ids.insert(account.id.clone(), account);
    }

    let mut contacts_result = Vec::new();
    for contact in contacts {
        if contact.account_id.is_none() {
            continue;
        }

        let account = match account_ids.get(&contact.account_id.unwrap()) {
            Some(account) => account,
            None => continue
        };

        let contact_result = Contact {
            city: account.billing_address_city.clone(),
            street: account.billing_address_street.clone(),
            postal: account.billing_address_postal_code.clone(),
            country: account.billing_address_country.clone(),
            relation_name: account.name.clone(),
            contact_name: contact.name.clone()
        };

        contacts_result.push(contact_result);
    }

    HttpResponse::Ok().json(Response { contacts: contacts_result })
}