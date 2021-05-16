use crate::appdata::AppData;
use espocrm_rs::{Params, Where, FilterType, Value};
use reqwest::Method;
use serde::Deserialize;

pub mod quote;
pub mod invoice;

#[derive(Deserialize, Clone)]
pub struct EspoResponse<T> {
    list: Vec<T>
}

#[derive(Deserialize, Clone)]
pub struct EspoContact {
    name: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EspoAccount {
    name:                           String,
    billing_address_city:           Option<String>,
    billing_address_country:        Option<String>,
    billing_address_postal_code:    Option<String>,
    billing_address_street:         Option<String>
}

pub async fn query_espo_contact(appdata: &AppData, contact_id: &str) -> Result<(Option<EspoContact>, Option<EspoAccount>), ()> {
    let runtime = tokio::runtime::Runtime::new();
    let runtime = if runtime.is_err() {
        eprintln!("Unable to create Tokio 1.x Runtime: {:?}", runtime.err().unwrap());
        return Err(());
    } else {
        runtime.unwrap()
    };
    let _guard = runtime.enter();

    let client = espocrm_rs::EspoApiClient::new(&appdata.config.espocrm_host)
        .set_api_key(&appdata.config.espocrm_api_key)
        .set_secret_key(&appdata.config.espocrm_secret_key)
        .build();

    let params = Params::new()
        .set_where(vec![
            Where {
                r#type: FilterType::Equals,
                attribute: "id".to_string(),
                value: Some(Value::str(contact_id))
            }
        ])
        .set_offset(0)
        .build();

    let contact_data = client.request::<()>(Method::GET, "Contact".to_string(), Some(params), None);

    let params = Params::new()
        .set_where(vec![
            Where {
                r#type: FilterType::LinkedWith,
                attribute: "contacts".to_string(),
                value: Some(Value::str(contact_id))
            }
        ])
        .set_offset(0)
        .build();


    let account_data = client.request::<()>(Method::GET, "Account".to_string(), Some(params), None);

    let contact_data = contact_data.await;
    if contact_data.is_err() {
        eprintln!("Unable to fetch Contact from EspoCRM: {:?}", contact_data.err().unwrap());
        return Err(());
    }

    let contact = contact_data.unwrap().json();

    let account_data = account_data.await;
    if account_data.is_err() {
        eprintln!("Unable to fetch Account from EspoCRM: {:?}", account_data.err().unwrap());
        return Err(());
    }
    let account = account_data.unwrap().json();

    let contact: EspoResponse<EspoContact> = contact.await.unwrap();
    let account: EspoResponse<EspoAccount> = account.await.unwrap();

    let first_contact = contact.list.get(0);

    let first_account = account.list.get(0);

    let contact = if first_contact.is_some() {
        Some(first_contact.unwrap().clone())
    } else {
        None
    };

    let account = if first_account.is_some() {
        Some(first_account.unwrap().clone())
    } else {
        None
    };

    runtime.shutdown_background();
    Ok((contact, account))
}