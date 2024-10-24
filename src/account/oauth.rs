use std::collections::HashMap;

use serde::Deserialize;

use super::Account;

#[derive(Deserialize, Debug)]
pub struct Application {
    token: String,
}

#[derive(Deserialize, Debug)]
pub struct OAuthApplicationList {
    applications: Option<Vec<Application>>,
}

impl Account {
    pub async fn revoke_oauth_application(&self, token: &str) -> eyre::Result<()> {
        let mut params = HashMap::new();
        params.insert("token", token.to_string());
        let response = self
            .client
            .post("https://x.com/i/api/1.1/oauth/revoke.json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;
        response.error_for_status()?;
        Ok(())
    }

    pub async fn get_all_oauth_applications(&self) -> eyre::Result<Vec<Application>> {
        let response = self
            .client
            .get("https://api.x.com/1.1/oauth/list.json")
            .send()
            .await?;
        let response: OAuthApplicationList = response.json().await?;
        Ok(response.applications.unwrap_or_default())
    }

    pub async fn revoke_all_oauth_applications(&self) -> eyre::Result<()> {
        let applications = self.get_all_oauth_applications().await?;
        for application in applications.iter() {
            self.revoke_oauth_application(&application.token).await?;
        }
        Ok(())
    }
}
