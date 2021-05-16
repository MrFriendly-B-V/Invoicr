use actix_web::{web, get, HttpResponse};
use serde::{Serialize, Deserialize};
use espocrm_rs::{EspoApiClient, Params, Order, Where, FilterType, Value};
use crate::appdata::AppData;
use reqwest::Method;

#[derive(Serialize)]
pub struct Response {
    contacts: Vec<Contact>
}

#[derive(Serialize)]
pub struct Contact {
    id:             String,
    relation_name:  Option<String>,
    city:           Option<String>,
    street:         Option<String>,
    postal:         Option<String>,
    country:        Option<String>,
    contact_name:   String
}

#[derive(Deserialize)]
struct EspoResponse<T> {
    list: Vec<T>
}

#[derive(Deserialize, Clone)]
struct EspoContact {
    id:     String,
    name:   String
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EspoAccount {
    name:                           String,
    billing_address_city:           Option<String>,
    billing_address_country:        Option<String>,
    billing_address_postal_code:    Option<String>,
    billing_address_street:         Option<String>
}

#[get("/persons/get")]
pub async fn get_contacts(data: web::Data<AppData>) -> HttpResponse {
    let tokio_runtime = tokio::runtime::Runtime::new().expect("Unable to launch new Tokio 1.x Runtime");
    let _guard = tokio_runtime.enter();

    let client = EspoApiClient::new(&data.config.espocrm_host)
        .set_api_key(&data.config.espocrm_api_key)
        .set_secret_key(&data.config.espocrm_secret_key)
        .build();

    //Contacts
    let params = Params::new()
        .set_order_by("name")
        .set_order(Order::Desc)
        .set_offset(0)
        .set_select("id,name")
        .build();

    let contacts = client.request::<()>(Method::GET, "Contact".to_string(), Some(params), None).await;
    if contacts.is_err() {
        eprintln!("Unable to fetch contacts from EspoCRM: {:?}", contacts.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }

    let mut processors = Vec::new();

    let contacts: EspoResponse<EspoContact> = contacts.unwrap().json().await.unwrap();
    for contact in contacts.list {
        let contact_clone = contact.clone();

        let processor = async {
            let params = Params::new()
                .set_order_by("name")
                .set_order(Order::Desc)
                .set_offset(0)
                .set_select("name,billingAddressCity,billingAddressCountry,billingAddressPostalCode,billingAddressStreet")
                .set_where(vec![
                    Where {
                        r#type: FilterType::LinkedWith,
                        attribute: "contacts".to_string(),
                        value: Some(Value::string(contact_clone.id.clone()))
                    }
                ])
                .build();

            let response = client.request::<()>(Method::GET, "Account".to_string(), Some(params), None).await;
            if response.is_err() {
                eprintln!("Unable to fetch Account from EspoCRM: {:?}", response.err().unwrap());
                Err(())
            } else {
                let accounts: EspoResponse<EspoAccount> = response.unwrap().json().await.unwrap();
                let account = accounts.list.get(0);

                if account.is_some() {
                    let account_unwrapped = account.unwrap();
                    let contact = Contact {
                        id:             contact_clone.id,
                        contact_name:   contact_clone.name,
                        city:           account_unwrapped.billing_address_city.clone(),
                        street:         account_unwrapped.billing_address_street.clone(),
                        postal:         account_unwrapped.billing_address_postal_code.clone(),
                        country:        account_unwrapped.billing_address_country.clone(),
                        relation_name:  Some(account_unwrapped.name.clone())
                    };

                    Ok(Some(contact))
                } else {
                    Ok(None)
                }
            }
        };

        processors.push(Box::new(processor));
    }

    let mut contacts = Vec::new();
    for processor in processors {
        let result = std::pin::Pin::from(processor).await;
        if result.is_err() {
            return HttpResponse::InternalServerError().finish();
        } else {
            let result_option = result.unwrap();
            if result_option.is_some() {
                contacts.push(result_option.unwrap());
            }
        };
    }

    tokio_runtime.shutdown_background();
    HttpResponse::Ok().json(Response { contacts })
}