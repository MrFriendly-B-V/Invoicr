use actix_web::dev::ServiceRequest;

pub async fn check_permission(_req: &ServiceRequest) -> Result<Vec<String>, actix_web::Error> {
    //TODO Perform user authentication, will need to write an RBAC server first

    //Stub code for now
    Ok(vec![
        "INVOICE_CREATE".to_string(),
        "INVOICE_READ".to_string(),
        "QUOTE_CREATE".to_string(),
        "QUOTE_READ".to_string(),
        "PERSONS_READ".to_string(),
        "PRODUCTS_READ".to_string(),
        "PRODUCTS_WRITE".to_string()
    ])
}