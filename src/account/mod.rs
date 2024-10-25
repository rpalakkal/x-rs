use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use reqwest::{header::HeaderMap, Client};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use serde::{Deserialize, Serialize};

pub mod login;
pub mod oauth;
pub mod password;

pub struct Account {
    client: Client,
    cookie_store: Arc<CookieStoreMutex>,
    headers: HeaderMap,
    auth_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccountAuth {
    headers: HashMap<String, String>,
    cookies: String,
}

impl AccountAuth {
    pub fn new(headers: HeaderMap, cookies: CookieStore) -> Self {
        let headers_map: HashMap<String, String> = headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let cookies_string = {
            let mut buffer = Vec::new();
            {
                let mut writer = std::io::BufWriter::new(&mut buffer);
                cookies.save_json(&mut writer).unwrap();
            }
            String::from_utf8(buffer).unwrap()
        };
        Self {
            headers: headers_map,
            cookies: cookies_string,
        }
    }
}

impl Account {
    pub fn from_auth(auth: AccountAuth) -> eyre::Result<Self> {
        let header_map = HeaderMap::from_iter(auth.headers.into_iter().map(|(k, v)| {
            (
                reqwest::header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                reqwest::header::HeaderValue::from_str(&v).unwrap(),
            )
        }));
        let cookie_store =
            CookieStore::load_json(auth.cookies.as_bytes()).map_err(|e| eyre::eyre!(e))?;
        let cookie_store = Arc::new(reqwest_cookie_store::CookieStoreMutex::new(cookie_store));
        let client = Client::builder()
            .cookie_provider(cookie_store.clone())
            .default_headers(header_map.clone())
            .build()
            .unwrap();
        Ok(Self {
            client,
            cookie_store,
            headers: header_map,
            auth_path: None,
        })
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> eyre::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let auth: AccountAuth = serde_json::from_reader(std::fs::File::open(path.clone())?)?;
        let mut account = Self::from_auth(auth)?;
        account.auth_path = Some(path);
        Ok(account)
    }

    pub async fn get_email_phone_info(&self) -> eyre::Result<EmailPhoneResponse> {
        let url = "https://x.com/i/api/1.1/users/email_phone_info.json";
        let response: EmailPhoneResponse = self.client.get(url).send().await?.json().await?;
        Ok(response)
    }
}

#[derive(Debug, Deserialize)]
pub struct EmailInfo {
    pub email: String,
    pub email_verified: bool,
}

#[derive(Debug, Deserialize)]
pub struct PhoneInfo();

#[derive(Debug, Deserialize)]
pub struct EmailPhoneResponse {
    pub emails: Vec<EmailInfo>,
    pub phone_numbers: Vec<PhoneInfo>,
}
