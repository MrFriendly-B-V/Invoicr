use serde::{Serialize, Deserialize};
use crate::appdata::Config;
use hmac::{Hmac, NewMac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PdfCommonPayload {
    pub template_name:  String,
    pub language:       String,
    pub id:             i64,
    pub attention_of:   Option<String>,
    pub receiver:       String,
    pub reference:      String,
    pub notes:          Option<String>,
    pub expiry_date:    i64,
    pub creation_date:  i64,
    pub rows:           Vec<ItemRow>,
    pub address:        Address
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PdfQuotePayload {
    #[serde(flatten)]
    pub common:                 PdfCommonPayload,
    pub quote_topic:            String,
    pub quote_contact_person:   String,
    pub debit_id:               String
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemRow {
    pub comment:        Option<String>,
    pub id:             String,
    pub name:           String,
    pub description:    String,
    pub discount_perc:  Option<f64>,
    pub vat_perc:       f64,
    pub price:          f64,
    pub quantity:       i64
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub city:           String,
    pub country:        String,
    pub postal_code:    String,
    pub street:         String
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PdfGenerationResponse {
    pub id:     Option<String>,
    pub error:  Option<String>
}

pub async fn generate_invoice(config: &Config, payload: &PdfCommonPayload) -> crate::Result<String> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    const PATH: &str = "generate/invoice";

    let client = reqwest::Client::new();
    let res = client.post(&format!("{}/{}", &config.invoicr_pdf_host, PATH))
        .json(payload)
        .header("X-Hmac-Authorization", get_hmac(config, "POST", PATH)?)
        .send()
        .await;

    let result: reqwest::Result<PdfGenerationResponse>  = match res {
        Ok(response) => response.json().await,
        Err(err) => return Err(err.to_string())
    };

    let id = match result {
        Ok(result) => {
            if result.id.is_some() {
                result.id.unwrap()
            } else {
                return Err(result.error.unwrap());
            }
        }
        Err(err) => return Err(err.to_string())
    };

    rt.shutdown_background();
    Ok(id)
}

pub async fn generate_quote(config: &Config, payload: &PdfQuotePayload) -> crate::Result<String> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    const PATH: &str = "generate/quote";

    let client = reqwest::Client::new();
    let res = client.post(&format!("{}/{}", &config.invoicr_pdf_host, PATH))
        .json(&payload)
        .header("X-Hmac-Authorization", get_hmac(config, "POST", PATH)?)
        .send()
        .await;

    let result: reqwest::Result<PdfGenerationResponse>  = match res {
        Ok(response) => response.json().await,
        Err(err) => return Err(err.to_string())
    };

    let id = match result {
        Ok(result) => {
            if result.id.is_some() {
                result.id.unwrap()
            } else {
                return Err(result.error.unwrap());
            }
        },
        Err(err) => return Err(err.to_string())
    };

    rt.shutdown_background();
    Ok(id)
}

fn get_hmac<A, B>(config: &Config, method: A, path: B) -> crate::Result<String> where
    A: AsRef<str>,
    B: AsRef<str> {
    let method = method.as_ref().to_string();
    let path = path.as_ref().to_string();

    let mut mac = match HmacSha256::new_from_slice(config.invoicr_pdf_secret.as_bytes()) {
        Ok(mac) => mac,
        Err(err) => return Err(err.to_string())
    };

    let data = format!("{} /{}", method, path);
    mac.update(data.as_bytes());
    let mac_result = mac.finalize().into_bytes();

    let hmac_string = format!("{}{}{}",
        base64::encode(&config.invoicr_pdf_key.as_bytes()),
        ":",
        base64::encode(mac_result)
    );

    Ok(hmac_string)
}