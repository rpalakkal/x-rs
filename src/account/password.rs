use std::collections::HashMap;

use super::Account;

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
}
