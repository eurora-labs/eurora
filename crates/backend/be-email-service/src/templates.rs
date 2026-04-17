pub struct VerificationEmail {
    pub subject: &'static str,
    pub html: String,
    pub text: String,
}

pub fn verification_email(verification_url: &str, display_name: Option<&str>) -> VerificationEmail {
    let greeting = match display_name {
        Some(name) => format!("Hi {name},"),
        None => "Hi,".to_string(),
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"></head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 40px 20px; color: #1a1a1a;">
  <h2 style="margin-bottom: 24px;">{greeting}</h2>
  <p>Please verify your email address to start using Eurora.</p>
  <p style="margin: 32px 0;">
    <a href="{verification_url}"
       style="background-color: #111; color: #fff; padding: 12px 32px; text-decoration: none; border-radius: 6px; display: inline-block; font-weight: 500;">
      Verify email
    </a>
  </p>
  <p style="color: #666; font-size: 14px;">This link expires in 24 hours. If you didn't create an account, you can ignore this email.</p>
  <p style="color: #666; font-size: 14px;">Or copy and paste this URL into your browser:</p>
  <p style="color: #666; font-size: 14px; word-break: break-all;">{verification_url}</p>
</body>
</html>"#
    );

    let text = format!(
        "{greeting}\n\n\
         Please verify your email address to start using Eurora.\n\n\
         Click here to verify: {verification_url}\n\n\
         This link expires in 24 hours. If you didn't create an account, you can ignore this email."
    );

    VerificationEmail {
        subject: "Verify your email address",
        html,
        text,
    }
}
