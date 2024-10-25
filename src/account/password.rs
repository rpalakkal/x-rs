use std::collections::HashMap;

use reqwest::Client;

use super::{Account, AccountAuth};

const CHANGE_PASSWORD_URL: &str = "https://x.com/i/api/i/account/change_password.json";

impl Account {
    pub async fn change_password(&self, old: &str, new: &str) -> eyre::Result<()> {
        let mut params = HashMap::new();
        params.insert("current_password", old);
        params.insert("password", new);
        params.insert("password_confirmation", new);

        let response = self
            .client
            .post(CHANGE_PASSWORD_URL)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;
        if !response.status().is_success() {
            let body = response.text().await?;
            eyre::bail!("Password change failed: {}", body);
        }
        Ok(())
    }

    pub async fn refresh_cookies(&mut self) -> eyre::Result<()> {
        let url = "https://x.com/i/api/2/notifications/all.json";
        let response = self.client.get(url).send().await?;
        let cookies_set: Vec<_> = response.cookies().collect();
        if let Some(ct0_cookie) = cookies_set.iter().find(|c| c.name() == "ct0") {
            if let Ok(ct0_value) = ct0_cookie.value().parse() {
                self.headers.insert("x-csrf-token", ct0_value);
            }
        }
        let client = Client::builder()
            .cookie_provider(self.cookie_store.clone())
            .default_headers(self.headers.clone())
            .build()
            .unwrap();
        self.client = client;
        if let Some(auth_path) = &self.auth_path {
            let cookie_store = self.cookie_store.lock().unwrap();
            let cookies = cookie_store.to_owned();
            drop(cookie_store);
            let auth = AccountAuth::new(self.headers.clone(), cookies);
            let auth_json = serde_json::to_string(&auth).unwrap();
            std::fs::write(auth_path, auth_json).unwrap();
        }
        Ok(())
    }
}
