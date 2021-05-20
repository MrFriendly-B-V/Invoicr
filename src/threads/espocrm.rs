use std::sync::mpsc::{Sender, channel};
use crate::apis::espocrm::{EspoAccount, EspoContact, get_contacts, get_accounts};
use std::thread::{spawn, sleep};
use crate::appdata::Config;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::cell::Cell;
use lazy_static::lazy_static;

enum QueryType {
    Contact,
    Account
}

enum QueryResponse {
    Contact(Vec<EspoContact>),
    Account(Vec<EspoAccount>)
}

pub struct Communication {
    query_type:     QueryType,
    query_result:   Sender<QueryResponse>
}

impl Communication {
    pub fn query_account(tx: &Sender<Communication>) -> Vec<EspoAccount> {
        let (tx_result, rx_result) = channel();

        let comm = Communication {
            query_type:     QueryType::Account,
            query_result:   tx_result
        };

        tx.send(comm).expect("Failed to send Communication struct");
        match rx_result.recv().unwrap() {
            QueryResponse::Account(account) => account,
            _ => unreachable!()
        }
    }

    pub fn query_contact(tx: &Sender<Communication>) -> Vec<EspoContact> {
        let (tx_result, rx_result) = channel();

        let comm = Communication {
            query_type:     QueryType::Contact,
            query_result:   tx_result
        };

        tx.send(comm).expect("Failed to send Communication struct");
        match rx_result.recv().unwrap() {
            QueryResponse::Contact(contact) => contact,
            _ => unreachable!()
        }
    }
}

const QUERY_INTERVAL_SECONDS: u64 = 900;

lazy_static! {
    static ref CONTACT_CACHE: Arc<Mutex<Cell<Vec<EspoContact>>>> = Arc::new(Mutex::new(Cell::new(Vec::new())));
    static ref ACCOUNT_CACHE: Arc<Mutex<Cell<Vec<EspoAccount>>>> = Arc::new(Mutex::new(Cell::new(Vec::new())));
}

pub fn start(config: Config) -> crate::Result<Sender<Communication>> {
    let (tx, rx) = channel();

    //This thread is responsible for answering queries from other threads
    spawn(move || {
        loop {
            let comm: Communication = match rx.recv() {
                Ok(comm) => comm,
                Err(_) => panic!("All Senders for EspoCRM communication closed")
            };

            match comm.query_type {
                QueryType::Contact => {
                    let contacts_lock = CONTACT_CACHE.lock().expect("Failed to lock Contacts Mutex");
                    let data = contacts_lock.take();
                    contacts_lock.set(data.clone());

                    comm.query_result.send(QueryResponse::Contact(data)).expect("Failed to send Contact cache.");
                },
                QueryType::Account => {
                    let accounts_lock = ACCOUNT_CACHE.lock().expect("Failed to lock Accounts Mutex");
                    let data = accounts_lock.take();
                    accounts_lock.set(data.clone());

                    comm.query_result.send(QueryResponse::Account(data)).expect("Failed to send Account cache");
                }
            }

        }
    });

    //This thread is responsible for keeping the espo cache up to date
    spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            loop {
                println!("Starting EspoCRM Cache Refresh thread.");

                let contacts = match get_contacts(&config, None).await {
                    Ok(contact) => contact,
                    Err(err) => {
                        eprintln!("Failed to query Contacts. Retrying in {} seconds: {:?}", QUERY_INTERVAL_SECONDS, err);
                        sleep(Duration::from_secs(QUERY_INTERVAL_SECONDS));
                        continue;
                    }
                };

                let contact_size = {
                    let cache_lock = match CONTACT_CACHE.lock() {
                        Ok(lock) => lock,
                        Err(err) => {
                            eprintln!("Failed to lock Contact cache. Retrying in {} seconds: {:?}", QUERY_INTERVAL_SECONDS, err);
                            sleep(Duration::from_secs(QUERY_INTERVAL_SECONDS));
                            continue;
                        }
                    };
                    let contact_size = contacts.len();
                    cache_lock.set(contacts);
                    contact_size
                };

                let accounts = match get_accounts(&config, None).await {
                    Ok(accounts) => accounts,
                    Err(err) => {
                        eprintln!("Failed to query Accounts. Retrying in {} seconds: {:?}", QUERY_INTERVAL_SECONDS, err);
                        sleep(Duration::from_secs(QUERY_INTERVAL_SECONDS));
                        continue;
                    }
                };

                let account_size = {
                    let cache_lock = match ACCOUNT_CACHE.lock() {
                        Ok(lock) => lock,
                        Err(err) => {
                            eprintln!("Failed to lock Account cache. Retrying in {} seconds: {:?}", QUERY_INTERVAL_SECONDS, err);
                            sleep(Duration::from_secs(QUERY_INTERVAL_SECONDS));
                            continue;
                        }
                    };
                    let acc_size = accounts.len();
                    cache_lock.set(accounts);
                    acc_size
                };

                println!("Updated EspoCRM Account and Contact cache. There are now {} Accounts and {} Contacts in the cache. Next run is in {} seconds.", account_size, contact_size, QUERY_INTERVAL_SECONDS);
                sleep(Duration::from_secs(QUERY_INTERVAL_SECONDS));
            }
        });
    });

    Ok(tx)
}