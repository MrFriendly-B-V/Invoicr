use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use mysql::prelude::Queryable;
use mysql::{Row, Params, params};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use crate::threads::espocrm::Communication;

#[derive(Clone)]
pub struct AppData {
    pub config:         Config,
    pub pool:           mysql::Pool,
    pub espocrm_data:   Sender<Communication>
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub mysql_host:         String,
    pub mysql_database:     String,
    pub mysql_username:     String,
    pub mysql_password:     String,
    pub espocrm_host:       String,
    pub espocrm_api_key:    String,
    pub espocrm_secret_key: String,
    pub invoicr_pdf_host:   String,
    pub invoicr_pdf_key:    String,
    pub invoicr_pdf_secret: String
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mysql_host: "mysql.example.com".to_string(),
            mysql_database: "mysql_example_database".to_string(),
            mysql_username: "some_mysql_user".to_string(),
            mysql_password: "a_super_secure_password".to_string(),
            espocrm_host: "espocrm.example.com".to_string(),
            espocrm_api_key: "espocrm_api_key".to_string(),
            espocrm_secret_key: "espocrm_secret_key".to_string(),
            invoicr_pdf_host: "invoicr_pdf_host".to_string(),
            invoicr_pdf_key: "your_invoicr_pdf_key".to_string(),
            invoicr_pdf_secret: "your_invoicr_pdf_secret".to_string()
        }
    }
}

impl Config {
    pub fn read() -> Self {
        if std::env::var("CONFIG_ENV").is_ok() {
            println!("Reading configuration from environmental variables.");
            Self::read_from_env()
        } else {
            println!("Reading configuration from file.");
            Self::read_from_file()
        }
    }

    fn read_from_file() -> Self{
        #[cfg(windows)]
        let mut config = PathBuf::from(r#"C:\Program Files\Invoicr"#.to_string());

        #[cfg(unix)]
        let mut config = PathBuf::from("/etc/invoicr".to_string());

        if !config.exists() {
            std::fs::create_dir_all(config.as_path()).expect("Failed to create configuration directory.");
        }

        config.push("config.yml");

        if !config.exists() {
            std::fs::write(config.as_path(), serde_yaml::to_string(&Self::default()).unwrap()).expect("Failed to write default configuration to file.");
            println!("A default configuration file has been created at '{}'. Please configure invoicr and then start it again.", config.as_path().to_str().unwrap());
            std::process::exit(0);
        }

        let config_contents = std::fs::read_to_string(config.as_path()).expect("Failed to read configuration file.");
        let config: Self = serde_yaml::from_str(&config_contents).expect("Failed to parse configuration file.");

        config
    }

    fn read_from_env() -> Self {
        use std::env::var;

        let mysql_host = var("MYSQL_HOST");
        if mysql_host.is_err() {
            eprintln!("Required environmental variable 'MYSQL_HOST' is not set. Exiting.");
            std::process::exit(1);
        }

        let mysql_database = var("MYSQL_DATABASE");
        if mysql_database.is_err() {
            eprintln!("Required environmental variable 'MYSQL_DATABASE' is not set. Exiting.");
            std::process::exit(1);
        }

        let mysql_username = var("MYSQL_USERNAME");
        if mysql_username.is_err() {
            eprintln!("Required environmental variable 'MYSQL_USERNAME' is not set. Exiting.");
            std::process::exit(1);
        }

        let mysql_password = var("MYSQL_PASSWORD");
        if mysql_password.is_err() {
            eprintln!("Required environmental variable 'MYSQL_PASSWORD' is not set. Exiting.");
            std::process::exit(1);
        }

        let espocrm_host = var("ESPOCRM_HOST");
        if espocrm_host.is_err() {
            eprintln!("Required environmental variable 'ESPOCRM_HOST' is not set. Exiting.");
            std::process::exit(1);
        }

        let espocrm_api_key = var("ESPOCRM_API_KEY");
        if espocrm_api_key.is_err() {
            eprintln!("Required environmental variable 'ESPOCRM_API_KEY' is not set. Exiting.");
            std::process::exit(1);
        }

        let espocrm_secret_key = var("ESPOCRM_SECRET_KEY");
        if espocrm_secret_key.is_err() {
            eprintln!("Required environmental variable 'ESPOCRM_SECRET_KEY' is not set. Exiting.");
            std::process::exit(1);
        }

        let invoicr_pdf_host = var("INVOICR_PDF_HOST");
        if invoicr_pdf_host.is_err() {
            eprintln!("Required environmental variable 'INVOICR_PDF_HOST' is not set. Exiting.");
            std::process::exit(1);
        }

        let invoicr_pdf_key = var("INVOICR_PDF_KEY");
        if invoicr_pdf_key.is_err() {
            eprintln!("Required environmental variable 'INVOICR_PDF_KEY' is not set. Exiting.");
            std::process::exit(1);
        }

        let invoicr_pdf_secret = var("INVOICR_PDF_SECRET");
        if invoicr_pdf_secret.is_err() {
            eprintln!("Required environmental variable 'INVOICR_PDF_SECRET' is not set. Exiting.");
            std::process::exit(1);
        }

        Self {
            mysql_host: mysql_host.unwrap(),
            mysql_database: mysql_database.unwrap(),
            mysql_username: mysql_username.unwrap(),
            mysql_password: mysql_password.unwrap(),
            espocrm_host: espocrm_host.unwrap(),
            espocrm_api_key: espocrm_api_key.unwrap(),
            espocrm_secret_key: espocrm_secret_key.unwrap(),
            invoicr_pdf_host: invoicr_pdf_host.unwrap(),
            invoicr_pdf_key: invoicr_pdf_key.unwrap(),
            invoicr_pdf_secret: invoicr_pdf_secret.unwrap()
        }
    }
}

impl AppData {
    pub fn new(config: &Config) -> Self {
        let mysql_uri = format!("mysql://{username}:{password}@{host}/{database}",
            username = config.mysql_username,
            password = config.mysql_password,
            host = config.mysql_host,
            database = config.mysql_database
        );

        let pool = mysql::Pool::new(mysql_uri);
        if pool.is_err() {
            eprintln!("Unable to create MySQL Pool: {:?}", pool.err().unwrap());
            std::process::exit(1);
        }

        Self {
            config: config.clone(),
            pool: pool.unwrap(),
            espocrm_data: crate::threads::espocrm::start(config.clone()).unwrap()
        }
    }

    pub fn check_db(&self) -> bool {
        let mut conn = self.pool.get_conn().expect("Unable to create database connection");
        let sql_get_tables = conn.exec::<Row, &str, Params>("SELECT table_name FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = :table_schema", params!{
           "table_schema" => self.config.mysql_database.clone()
        }).expect("Unable to fetch tables from the database");

        let mut required_tables_map = HashMap::new();
        required_tables_map.insert("products".to_string(), false);
        required_tables_map.insert("invoices".to_string(), false);
        required_tables_map.insert("quotes".to_string(), false);

        for row in sql_get_tables {
            let table_name = row.get::<String, &str>("table_name").expect("Unable to get table_name from row.");
            required_tables_map.insert(table_name, true);
        }

        let mut table_missing = false;
        for (k, v) in required_tables_map {
            if v == false {
                println!("Missing required table {}", k);
                table_missing = true;
            }
        }

        !table_missing
    }

    pub fn init_db(&self) {
        let mut conn = self.pool.get_conn().expect("Unable to create database connection.");

        conn.query::<usize, &str>("CREATE TABLE `products` (`id` varchar(32) NOT NULL, `name` varchar(255) NOT NULL, `description` text NOT NULL, `price` double NOT NULL, PRIMARY KEY (`id`), KEY `name` (`name`)) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4").expect("Unable to create table 'products'");
        println!("Created table 'products'");

        conn.query::<usize, &str>("CREATE TABLE `invoices` (`id` int(11) NOT NULL, `receiver` varchar(255) NOT NULL, `is_paid` tinyint(1) NOT NULL, PRIMARY KEY (`id`), KEY `receiver` (`receiver`)) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4").expect("Unable to create table 'invoices'");
        println!("Created table 'invoices'");

        conn.query::<usize, &str>("CREATE TABLE `quotes` (`id` int(11) NOT NULL, `invoice_id` varchar(32) DEFAULT NULL, `receiver` varchar(255) NOT NULL, `valid_until` bigint(20) NOT NULL, PRIMARY KEY (`id`), KEY `invoice_id` (`invoice_id`)) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4").expect("Unable to create table 'quotes'");
        println!("Created table 'quotes'");
    }
}