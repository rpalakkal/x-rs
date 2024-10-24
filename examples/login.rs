use dotenv::dotenv;
use x_rs::account::login;

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv().ok();

    let username = std::env::var("X_USERNAME").unwrap();
    let password = std::env::var("X_PASSWORD").unwrap();
    let email = std::env::var("X_EMAIL").unwrap();
    let totp = std::env::var("X_TOTP").ok();
    let proxy_url = std::env::var("PROXY_URL").ok();

    let mut login = login::Login::new(username, password, email, totp, proxy_url).unwrap();
    let auth = login.login().await.unwrap();
    let auth_json = serde_json::to_string(&auth).unwrap();
    std::fs::write("auth.txt", auth_json).unwrap();
}
