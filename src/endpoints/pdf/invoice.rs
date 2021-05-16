use actix_web::{post, web, HttpResponse};
use serde::{Deserialize, Serialize};
use mysql::prelude::Queryable;
use crate::AppData;
use crate::endpoints::products::Product;
use genpdf::{elements, style};
use crate::endpoints::pdf::query_espo_contact;
use genpdf::elements::Alignment;


#[derive(Deserialize)]
pub struct Request {
    receiver:       String,
    attention_of:   Option<String>,
    rows:           Vec<RequestRow>,
    discount_perc:  f64,
    exp_date:       i64,
    invoice_date:   i64,
    reference:      String,
    notes:          Option<String>
}

#[derive(Deserialize, Clone)]
pub struct RequestRow {
    #[serde(flatten)]
    product:        Product,
    discount_perc:  f64,
    quantity:       i64,
    vat_perc:       f64,
    comment:        Option<String>
}

#[cfg(windows)]
const FONT_FOLDER: &str = r#"C:\Program Files\Invoicr\Fonts\"#;
#[cfg(windows)]
const COMPANY_LOGO: &str = r#"C:\Program Files\Invoicr\logo.png"#;

#[cfg(unix)]
const FONT_FOLDER: &str = "/etc/invoicr/fonts/";
#[cfg(unix)]
const COMPANY_LOGO: &str = "/etc/invoicr/logo.png";

#[post("/pdf/invoice")]
pub async fn create_invoice(data: web::Data<AppData>, request: web::Json<Request>) -> HttpResponse {

    //Query EspoCRM for information about the Account and Contact for the provided contact id ('receiver')
    //This is async, because espocrm isn't always the fastest.
    let contact_query = query_espo_contact(&data, &request.receiver);

    //Create a connection to the database
    //to query the latest invoice
    let conn = data.pool.get_conn();
    if conn.is_err() {
        eprintln!("Unable to create database connection: {:?}", conn.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    }
    let mut conn = conn.unwrap();

    //Query the latest invoice
    //Our new invoice ID is the last ID +1, or if this is the first, just 1
    let sql_latest_invoice = conn.query::<i32, &str>("SELECT id FROM invoices ORDER BY id DESC LIMIT 1");
    let invoice_id = if sql_latest_invoice.is_err() {
        eprintln!("Unable to fetch latest invoice: {:?}", sql_latest_invoice.err().unwrap());
        return HttpResponse::InternalServerError().finish();
    } else {
        let latest_invoice = sql_latest_invoice.unwrap();
        let first_elem = latest_invoice.get(0);
        if first_elem.is_some() {
            first_elem.unwrap().clone() + 1
        } else {
            1i32
        }
    };

    //Create the PDF document, and configure basic options on it
    let mut doc = genpdf::Document::new(FONT_FOLDER, "Roboto").expect("Failed to create PDF");
    doc.set_margins(10);
    doc.set_title(format!("MrFriendly Invoice #{}", invoice_id));

    //Now wait for EspoCRM to give us it's response
    let contact_query = contact_query.await;
    if contact_query.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    let (contact, account) = contact_query.unwrap();
    let contact = if contact.is_none() {
        return HttpResponse::NotFound().body("Contact not found");
    } else {
        contact.unwrap()
    };

    let account = if account.is_none() {
        return HttpResponse::NotFound().body("Account not found");
    } else {
        account.unwrap()
    };

    //The heading contains to whom this invoice is addressed,
    //their address (street, postal code, city, country)
    //Also contains the company logo (When I figure out how)
    let heading = {
        let receiver_layout = {
            let mut alignment = elements::LinearLayout::vertical();

            //Name of the person to whom this invoice is addressed
            let receiver_title = elements::StyledElement::new(elements::Text::new(contact.name), style::Effect::Bold);
            alignment.push(receiver_title);

            //If the attention_of field is set, we add it to the alignment (i.e the holder)
            if request.attention_of.is_some() {
                let tav = elements::Text::new(request.attention_of.as_ref().unwrap());
                alignment.push(tav);
            }

            //If the billing address is known, add it,
            //else return an error
            if account.billing_address_street.is_some() {
                let street = elements::Text::new(account.billing_address_street.unwrap());
                alignment.push(street);
            } else {
                return HttpResponse::NotFound().body("Account billing address street is not filled in.");
            }

            //If the postal code AND city are known, add it,
            //else return an error
            if account.billing_address_postal_code.is_some() && account.billing_address_city.is_some() {
                let postal = elements::Text::new(format!("{} {}", account.billing_address_postal_code.unwrap(), account.billing_address_city.unwrap()));
                alignment.push(postal);

            } else {
                if account.billing_address_postal_code.is_none() {
                    return HttpResponse::NotFound().body("Account billing address postal code is not filled in.");

                } else if account.billing_address_city.is_none() {
                    return HttpResponse::NotFound().body("Account billing address city is not filled in.");
                }
            }

            //If the country is known, add it,
            //else return an error
            if account.billing_address_country.is_some() {
                let country = elements::Text::new(account.billing_address_country.unwrap());
                alignment.push(country);
            } else {
                return HttpResponse::NotFound().body("Account billing address city is not country in.");
            }

            alignment
        };

        //We want to give the receiver block some padding
        let receiver_padded = elements::PaddedElement::new(receiver_layout, genpdf::Margins::trbl(5, 2, 5, 10));

        //The company logo, eventually
        //let mrfriendly_elem = elements::Text::new("Yet to figure out how to do images...");
        let mrfriendly_elem = crate::pdf::image::Image::png(COMPANY_LOGO);

        //Create a table to stick the receiver padding block, and the company logo in
        let mut table = elements::TableLayout::new(vec![1,1]);
        let mut row = table.row();
        row.push_element(receiver_padded);
        row.push_element(mrfriendly_elem);
        row.push().expect("Invalid Row");

        table
    };
    doc.push(heading);

    //Bold style
    let mut bold_style = style::Style::new();
    bold_style.set_bold();

    //Add "INVOICE" to the doc
    doc.push(elements::StyledElement::new(elements::Paragraph::new("INVOICE").aligned(Alignment::Right), bold_style.with_font_size(20)));

    doc.push(elements::Break::new(1));

    //This block describes the invoice itself
    // - Reference
    // - Invoice date
    // - Expiry Date
    // - Invoice ID
    let reference = {
        use chrono::prelude::*;

        //Firstly create a table, and add a header for each column
        let mut table = elements::TableLayout::new(vec![3, 1, 1, 1]);
        let mut row1 = table.row();
        row1.push_element(elements::StyledElement::new(elements::Text::new("Reference"), style::Effect::Bold));
        row1.push_element(elements::StyledElement::new(elements::Text::new("Invoice Date"), style::Effect::Bold));
        row1.push_element(elements::StyledElement::new(elements::Text::new("Expiry Date"), style::Effect::Bold));
        row1.push_element(elements::StyledElement::new(elements::Text::new("Invoice nr."), style::Effect::Bold));
        row1.push().expect("Failed to push row");

        //Create a second row for the values associated with the above headers
        let mut row2 = table.row();
        row2.push_element(elements::Paragraph::new(request.reference.clone()));

        //Convert the invoice date epoch timestamp to dd-mm-YYYY and add it
        let invoice_date = chrono::DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(request.invoice_date, 0u32), Utc);
        row2.push_element(elements::Text::new(format!("{}-{}-{}", invoice_date.day(), invoice_date.month(), invoice_date.year())));

        //Convert the expiry date epoch timestamp to dd-mm-YYYY and add it
        let expiry_date = chrono::DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(request.exp_date, 0u32), Utc);
        row2.push_element(elements::Text::new(format!("{}-{}-{}", expiry_date.day(), expiry_date.month(), expiry_date.year())));

        //Add the invoice nr, use the format! macro to make sure it is always 6 digits (prefixed with zeroes)
        row2.push_element(elements::Text::new(format!("{:06}", invoice_id)));
        row2.push().expect("Failed to push row");

        table
    };
    doc.push(reference);
    doc.push(elements::Break::new(3));

    //These are gradually added to
    let mut total_price_ex_vat = 0f64;
    let mut totals_vat = 0f64;

    //Now for the most important bit of the invoice, the products
    let products_tables = {
        //Create a vector containing all tables used for this block
        let mut tables = Vec::new();

        //Create a table with the headers for the main table
        let mut header_table = elements::TableLayout::new(vec![1, 2, 1, 1, 1, 1, 1]);
        let mut header_row = header_table.row();
        header_row.push_element(elements::StyledElement::new(elements::Text::new("Article nr."), style::Effect::Bold));
        header_row.push_element(elements::StyledElement::new(elements::Text::new("Article description"), style::Effect::Bold));
        header_row.push_element(elements::StyledElement::new(elements::Text::new("Quantity"), style::Effect::Bold));
        header_row.push_element(elements::StyledElement::new(elements::Text::new("Price"), style::Effect::Bold));
        header_row.push_element(elements::StyledElement::new(elements::Text::new("Discount"), style::Effect::Bold));
        header_row.push_element(elements::StyledElement::new(elements::Text::new("Amount"), style::Effect::Bold));
        header_row.push_element(elements::StyledElement::new(elements::Text::new("Total"), style::Effect::Bold));
        header_row.push().expect("Failed to push Row");
        tables.push(header_table);

        //Iterate over every product we received
        for row in request.rows.clone() {
            //Create an another table containing all the values
            let mut table = elements::TableLayout::new(vec![1, 2, 1, 1, 1, 1, 1]);
            let mut trow = table.row();
            trow.push_element(elements::Text::new(row.product.id.unwrap()));
            trow.push_element(elements::Paragraph::new(row.product.description));
            trow.push_element(elements::Text::new(format!("{}", &row.quantity)));
            trow.push_element(elements::Text::new(format!("€{:.2}", &row.product.price)));
            trow.push_element(elements::Text::new(format!("{:.1}%", &row.discount_perc)));
            trow.push_element(elements::Text::new(format!("€{:.2}", &row.product.price * (100f64 - &row.discount_perc)/100f64)));
            trow.push_element(elements::Text::new(format!("€{:.2}", &row.product.price * (100f64 - &row.discount_perc)/100f64 * &(row.quantity as f64))));
            trow.push().expect("Failed to push row");
            tables.push(table);

            //Calculate the price and add it to the variables above
            let price = &row.product.price * (100f64 - &row.discount_perc)/100f64 * &(row.quantity as f64);
            total_price_ex_vat += price;
            totals_vat += &row.vat_perc/100f64 * price;

            //If there's a comment, create a new table for this and populate it
            //We need a separate table due to alignment
            if row.comment.is_some() {
                let mut table = elements::TableLayout::new(vec![5, 1]);
                let mut trow = table.row();
                trow.push_element(elements::Paragraph::new(row.comment.as_ref().unwrap()));
                trow.push_element(elements::Text::new(""));
                trow.push().expect("Failed to push row");
                tables.push(table);
            }
        }

        tables
    };

    //Add every table created in the products block to the doc
    for table in products_tables {
        doc.push(table);
    }

    doc.push(elements::Break::new(2));

    //If there's a footnote, add it.
    if request.notes.is_some() {
        let notes = elements::Paragraph::new(request.notes.as_ref().unwrap());
        doc.push(notes);
    }

    //Now for the final bit, the totals.
    let mut totals_table = elements::TableLayout::new(vec![6, 1, 3]);
    let mut row1 = totals_table.row();
    row1.push_element(elements::Text::new("Total excl. VAT"));
    row1.push_element(elements::Text::new("€"));
    row1.push_element(elements::Text::new(format!("{:.2}", total_price_ex_vat)));
    row1.push().expect("Failed to push row");

    let mut row2 = totals_table.row();
    row2.push_element(elements::Text::new("VAT"));
    row2.push_element(elements::Text::new("€"));
    row2.push_element(elements::Text::new(format!("{:.2}", totals_vat)));
    row2.push().expect("Failed to push row");

    let mut row3 = totals_table.row();
    row3.push_element(elements::Text::new("Total"));
    row3.push_element(elements::Text::new("€"));
    row3.push_element(elements::Text::new(format!("{:.2}", total_price_ex_vat + totals_vat)));
    row3.push().expect("Failed to push row");

    doc.push(totals_table);

    doc.render_to_file("output.pdf").expect("Failed to write PDF");

    HttpResponse::Ok().finish()
}