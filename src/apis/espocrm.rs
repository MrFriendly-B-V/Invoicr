use crate::appdata::Config;
use espocrm_rs::{EspoApiClient, Params, Method, NoGeneric, Where, FilterType, Value};
use serde::Deserialize;
use async_recursion::async_recursion;

#[derive(Deserialize, Clone)]
pub struct EspoResponse<T> {
    list: Vec<T>
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]

pub struct EspoContact {
    pub account_id: Option<String>,
    pub name:       String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EspoAccount {
    pub name:                           String,
    pub billing_address_city:           String,
    pub billing_address_country:        String,
    pub billing_address_postal_code:    String,
    pub billing_address_street:         String,
    pub id:                             String
}

/**
Get all Contacts from EspoCRM, only contacts linked to an Account are returned
*/
#[async_recursion]
pub async fn get_contacts(config: &Config, offset: Option<i64>) -> crate::Result<Vec<EspoContact>> {
    let client = EspoApiClient::new(&config.espocrm_host)
        .set_api_key(&config.espocrm_api_key)
        .set_secret_key(&config.espocrm_secret_key)
        .build();

    let params = Params::new()
        .set_offset(offset.unwrap_or_else(|| 0i64))
        .set_select("id,name,accounts")
        .build();

    let response = client.request::<NoGeneric, &str>(Method::Get, "Contact", Some(params), None).await;
    if response.is_err() {
        return Err(response.err().unwrap().to_string());
    }

    let mut response_data: EspoResponse<EspoContact> = response.unwrap().json().await.unwrap();

    let mut results = Vec::new();

    if response_data.list.len() == 200 {
        results.append(&mut get_contacts(config, Some(offset.unwrap_or_else(|| 0i64) + 200)).await?);
    }

    results.append(&mut response_data.list);


    Ok(results)
}

/**
Get all Accounts from EspoCRM, only contacts linked to a Contact are returned
*/
#[async_recursion]
pub async fn get_accounts(config: &Config, offset: Option<i64>) -> crate::Result<Vec<EspoAccount>> {

    let client = EspoApiClient::new(&config.espocrm_host)
        .set_api_key(&config.espocrm_api_key)
        .set_secret_key(&config.espocrm_secret_key)
        .build();

    let params = Params::new()
        .set_offset(offset.unwrap_or_else(|| 0i64))
        .set_select("id,name,contacts,billingAddressCity,billingAddressCountry,billingAddressPostalCode,billingAddressStreet")
        .set_where(vec![
            Where {
                r#type: FilterType::IsNotNull,
                attribute: "billingAddressCity".to_string(),
                value: None
            },
            Where {
                r#type: FilterType::IsNotNull,
                attribute: "billingAddressCountry".to_string(),
                value: None
            },
            Where {
                r#type: FilterType::IsNotNull,
                attribute: "billingAddressPostalCode".to_string(),
                value: None
            },
            Where {
                r#type: FilterType::IsNotNull,
                attribute: "billingAddressStreet".to_string(),
                value: None
            },
            Where {
                r#type: FilterType::NotEquals,
                attribute: "billingAddressCity".to_string(),
                value: Some(Value::str(""))
            },
            Where {
                r#type: FilterType::NotEquals,
                attribute: "billingAddressCountry".to_string(),
                value: Some(Value::str(""))
            },
            Where {
                r#type: FilterType::NotEquals,
                attribute: "billingAddressPostalCode".to_string(),
                value: Some(Value::str(""))
            },
            Where {
                r#type: FilterType::NotEquals,
                attribute: "billingAddressStreet".to_string(),
                value: Some(Value::str(""))
            }
        ])
        .build();

    let response = client.request::<NoGeneric, &str>(Method::Get, "Account", Some(params), None).await;
    if response.is_err() {
        return Err(response.err().unwrap().to_string());
    }

    let mut response_data: EspoResponse<EspoAccount> = response.unwrap().json().await.unwrap();

    let mut results = Vec::new();
    if response_data.list.len() == 200 {
        results.append(&mut get_accounts(config, Some(offset.unwrap_or_else(|| 0i64) + 200i64)).await?);
    }

    results.append(&mut response_data.list);

    Ok(results)
}

