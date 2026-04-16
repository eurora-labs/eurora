pub mod error;
mod templates;

use error::EmailError;
use lettermint_rs::Query;
use lettermint_rs::api::email::SendEmailRequest;
use lettermint_rs::reqwest::LettermintClient;

pub struct EmailService {
    client: LettermintClient,
    from_address: String,
    frontend_url: String,
}

impl EmailService {
    pub fn from_env() -> Result<Self, EmailError> {
        let api_token = std::env::var("LETTERMINT_API_TOKEN")
            .map_err(|_| EmailError::Config("LETTERMINT_API_TOKEN must be set".into()))?;

        let from_address = std::env::var("LETTERMINT_FROM_ADDRESS")
            .unwrap_or_else(|_| "Eurora <noreply@eurora-labs.com>".into());

        let frontend_url =
            std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".into());

        let client = LettermintClient::builder().api_token(api_token).build();

        Ok(Self {
            client,
            from_address,
            frontend_url,
        })
    }

    pub async fn send_verification_email(
        &self,
        to: &str,
        token: &str,
        display_name: Option<&str>,
    ) -> Result<(), EmailError> {
        let verification_url = format!(
            "{}/verify-email?token={}",
            self.frontend_url.trim_end_matches('/'),
            token
        );

        let (subject, html, text) = templates::verification_email(&verification_url, display_name);

        let request = SendEmailRequest::builder()
            .from(&self.from_address)
            .to(vec![to.to_string()])
            .subject(subject)
            .html(html)
            .text(text)
            .build();

        request
            .execute(&self.client)
            .await
            .map_err(|e| EmailError::Send(e.to_string()))?;

        tracing::info!(to = to, "Verification email sent");
        Ok(())
    }
}
