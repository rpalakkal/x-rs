use dotenv::dotenv;
use rand::{distributions::Alphanumeric, Rng};
use x_rs::account::{login, Account};

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv().ok();

    let username = std::env::var("X_USERNAME").unwrap();
    let password = std::env::var("X_PASSWORD").unwrap();
    let email = std::env::var("X_EMAIL").unwrap();
    let totp = std::env::var("X_TOTP").ok();

    let mut login =
        login::Login::new(username, password.clone(), email.clone(), totp, None).unwrap();
    let auth = login.login().await.unwrap();

    let mut account = Account::from_auth(auth).unwrap();
    let new_password: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();
    account
        .change_password(&password, &new_password)
        .await
        .unwrap();
    log::info!("Password changed to: {}", new_password);
    account.refresh_cookies().await.unwrap();

    let oauth_applications = account.get_all_oauth_applications().await.unwrap();
    let filtered_applications: Vec<_> = oauth_applications
        .into_iter()
        .filter(|app| app.app_id != "27965877")
        .collect();
    for application in filtered_applications.iter() {
        account
            .revoke_oauth_application(&application.token)
            .await
            .unwrap();
    }
    let phone_email_info = account.get_email_phone_info().await.unwrap();
    assert!(phone_email_info.emails.len() == 1);
    assert!(phone_email_info.emails[0].email == email);
    assert!(phone_email_info.phone_numbers.is_empty());
}
