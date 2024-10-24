use std::{collections::HashMap, sync::Arc};

use eyre::OptionExt;
use fake_user_agent::get_safari_rua;
use reqwest::{header::HeaderMap, Client, Proxy};
use reqwest_cookie_store::CookieStoreMutex;
use serde::Deserialize;
use totp_rs::{Algorithm, Secret, TOTP};

use super::AccountAuth;

const LOGIN_URL: &str = "https://api.x.com/1.1/onboarding/task.json";
const TOKEN: &str = "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";

pub struct Login {
    client: Client,
    headers: HeaderMap,
    cookie_store: Arc<CookieStoreMutex>,
    username: String,
    password: String,
    email: String,
    totp_code: Option<String>,
}

#[derive(Deserialize, Debug)]
struct TaskResponse {
    flow_token: String,
    subtasks: Vec<Subtask>,
}

#[derive(Deserialize, Debug)]
struct Subtask {
    subtask_id: String,
}

impl Login {
    pub fn new(
        username: String,
        password: String,
        email: String,
        totp_code: Option<String>,
        proxy: Option<String>,
    ) -> eyre::Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::USER_AGENT, get_safari_rua().parse()?);
        headers.insert(reqwest::header::CONTENT_TYPE, "application/json".parse()?);
        headers.insert("authorization", TOKEN.parse()?);
        headers.insert("x-twitter-active-user", "yes".parse()?);
        headers.insert("x-twitter-client-language", "en".parse()?);
        let cookie_store = CookieStoreMutex::default();
        let cookie_store = Arc::new(cookie_store);
        let mut client_builder = Client::builder().cookie_provider(cookie_store.clone());
        if let Some(proxy) = proxy {
            client_builder = client_builder.proxy(Proxy::all(proxy)?);
        }
        let client = client_builder.build()?;
        Ok(Self {
            client,
            cookie_store,
            headers,
            username,
            password,
            totp_code,
            email,
        })
    }

    async fn get_guest_token(&mut self) -> eyre::Result<()> {
        let response = self
            .client
            .post("https://api.x.com/1.1/guest/activate.json")
            .headers(self.headers.clone())
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let guest_token = response["guest_token"].as_str().unwrap();
        self.headers.insert("x-guest-token", guest_token.parse()?);
        Ok(())
    }

    async fn init_login(&self) -> eyre::Result<TaskResponse> {
        let payload = serde_json::json!({
            "input_flow_data": {
                "flow_context": {
                    "debug_overrides": {},
                    "start_location": {
                        "location": "unknown"
                    }
                },
                "subtask_versions": {}
            }
        });
        let response = self
            .client
            .post(LOGIN_URL)
            .headers(self.headers.clone())
            .query(&[("flow_name", "login")])
            .json(&payload)
            .send()
            .await?;

        let task_response: TaskResponse = response.json().await?;
        Ok(task_response)
    }

    async fn instrumentation(&self, flow_token: &str) -> eyre::Result<TaskResponse> {
        let payload = serde_json::json!({
            "flow_token": flow_token,
            "subtask_inputs": [
                {
                    "subtask_id": "LoginJsInstrumentationSubtask",
                    "js_instrumentation": {"response": "{}", "link": "next_link"},
                }
            ],
        });

        let response = self
            .client
            .post(LOGIN_URL)
            .headers(self.headers.clone())
            .json(&payload)
            .send()
            .await?;

        let task_response: TaskResponse = response.json().await?;
        Ok(task_response)
    }

    async fn enter_username(&self, flow_token: &str) -> eyre::Result<TaskResponse> {
        let payload = serde_json::json!({
            "flow_token": flow_token,
            "subtask_inputs": [
                {
                    "subtask_id": "LoginEnterUserIdentifierSSO",
                    "settings_list": {
                        "setting_responses": [
                            {
                                "key": "user_identifier",
                                "response_data": {"text_data": {"result": self.username}},
                            }
                        ],
                        "link": "next_link",
                    },
                }
            ],
        });

        let response = self
            .client
            .post(LOGIN_URL)
            .headers(self.headers.clone())
            .json(&payload)
            .send()
            .await?;

        let task_response: TaskResponse = response.json().await?;
        Ok(task_response)
    }

    async fn enter_password(&self, flow_token: &str) -> eyre::Result<TaskResponse> {
        let payload = serde_json::json!({
            "flow_token": flow_token,
            "subtask_inputs": [
                {
                    "subtask_id": "LoginEnterPassword",
                    "enter_password": {
                        "password": self.password,
                        "link": "next_link"
                    },
                }
            ],
        });

        let response = self
            .client
            .post(LOGIN_URL)
            .headers(self.headers.clone())
            .json(&payload)
            .send()
            .await?;

        let task_response: TaskResponse = response.json().await?;
        Ok(task_response)
    }

    async fn login_two_factor_auth_challenge(
        &self,
        flow_token: &str,
    ) -> eyre::Result<TaskResponse> {
        if self.totp_code.is_none() {
            eyre::bail!("MFA code is required");
        }

        let totp = TOTP::new_unchecked(
            Algorithm::SHA1,
            6,
            1,
            30,
            Secret::Encoded(self.totp_code.clone().unwrap())
                .to_bytes()
                .unwrap(),
        );

        let payload = serde_json::json!({
            "flow_token": flow_token,
            "subtask_inputs": [
                {
                    "subtask_id": "LoginTwoFactorAuthChallenge",
                    "enter_text": {
                        "text": totp.generate_current().unwrap(),
                        "link": "next_link"
                    },
                }
            ],
        });

        let response = self
            .client
            .post(LOGIN_URL)
            .headers(self.headers.clone())
            .json(&payload)
            .send()
            .await?;

        let task_response: TaskResponse = response.json().await?;
        Ok(task_response)
    }

    async fn login_success(&mut self, flow_token: &str) -> eyre::Result<()> {
        let payload = serde_json::json!({
            "flow_token": flow_token,
            "subtask_inputs": [],
        });

        let cookie_store = self.cookie_store.lock().unwrap();
        let mut cookies = HashMap::new();
        for cookie in cookie_store.iter_unexpired() {
            cookies.insert(cookie.name().to_string(), cookie.value().to_string());
        }
        let ct0_cookie = cookies.get("ct0").ok_or_eyre("ct0 cookie not found")?;
        self.headers
            .insert("x-csrf-token", ct0_cookie.parse().unwrap());
        self.headers
            .insert("x-twitter-auth-type", "OAuth2Session".parse().unwrap());
        drop(cookie_store);

        let response = self
            .client
            .post(LOGIN_URL)
            .headers(self.headers.clone())
            .json(&payload)
            .send()
            .await?;
        response.error_for_status()?;

        let cookie_store = self.cookie_store.lock().unwrap();
        let mut cookies = HashMap::new();
        for cookie in cookie_store.iter_unexpired() {
            cookies.insert(cookie.name().to_string(), cookie.value().to_string());
        }
        let ct0_cookie = cookies.get("ct0").ok_or_eyre("ct0 cookie not found")?;
        self.headers
            .insert("x-csrf-token", ct0_cookie.parse().unwrap());
        drop(cookie_store);
        Ok(())
    }

    async fn alternate_identifier(
        &self,
        flow_token: &str,
        email: &str,
    ) -> eyre::Result<TaskResponse> {
        let payload = serde_json::json!({
            "flow_token": flow_token,
            "subtask_inputs": [
                {
                    "subtask_id": "LoginEnterAlternateIdentifierSubtask",
                    "enter_text": {
                        "text": email,
                        "link": "next_link"
                    }
                }
            ]
        });

        let response = self
            .client
            .post(LOGIN_URL)
            .headers(self.headers.clone())
            .json(&payload)
            .send()
            .await?;

        let task_response: TaskResponse = response.json().await?;
        Ok(task_response)
    }

    pub async fn login(&mut self) -> eyre::Result<AccountAuth> {
        self.get_guest_token().await?;
        let res = self.init_login().await?;
        let mut res = self.instrumentation(&res.flow_token).await?;
        loop {
            if res.subtasks.len() == 0 {
                break;
            }
            res = match res.subtasks[0].subtask_id.as_str() {
                "LoginEnterUserIdentifierSSO" => self.enter_username(&res.flow_token).await?,
                "LoginEnterPassword" => self.enter_password(&res.flow_token).await?,
                "LoginTwoFactorAuthChallenge" => {
                    self.login_two_factor_auth_challenge(&res.flow_token)
                        .await?
                }
                "LoginSuccessSubtask" => {
                    self.login_success(&res.flow_token).await?;
                    break;
                }
                "LoginEnterAlternateIdentifierSubtask" => {
                    self.alternate_identifier(&res.flow_token, &self.email)
                        .await?
                }
                _ => {
                    eyre::bail!("Login Failed: {:?}", res);
                }
            };
        }
        let cookie_store = self.cookie_store.lock().unwrap();
        let cookies = cookie_store.to_owned();
        drop(cookie_store);
        let account_auth = AccountAuth::new(self.headers.clone(), cookies);
        Ok(account_auth)
    }
}
