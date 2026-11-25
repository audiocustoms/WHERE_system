//! Sending one message through the e-mail account of the signed-in user.
//!
//! The transport is SMTP with the settings of `"User_ITPrefs"`: port 465 speaks TLS from
//! the first byte (`smtps`), any other port starts plain and switches with `STARTTLS`
//! when the server offers it. The account name is the address of the account, the
//! password the stored one.

use lettre::message::header::ContentType;
use lettre::message::{Attachment, Mailbox, MultiPart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use where_domain::DomainError;
use where_persistence::MailAccount;

/// Sends `subject` and `body` with the calendar file attached.
pub async fn send_calendar(
    account: &MailAccount,
    recipient: (&str, &str),
    subject: &str,
    body: &str,
    file_name: &str,
    calendar: &str,
) -> Result<(), DomainError> {
    let from: Mailbox = account.email.trim().parse().map_err(|_| {
        DomainError::Validation("The e-mail address of the account cannot be used.".to_string())
    })?;
    let to: Mailbox = recipient.1.trim().parse().map_err(|_| {
        DomainError::Validation("The e-mail address of the employee cannot be used.".to_string())
    })?;
    let mut builder = Message::builder()
        .from(from)
        .to(Mailbox::new(Some(recipient.0.trim().to_string()), to.email));
    if let Some(bcc) = account
        .bcc
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        if let Ok(address) = bcc.parse() {
            builder = builder.bcc(address);
        }
    }
    let message = builder
        .subject(subject)
        .multipart(
            MultiPart::mixed()
                .multipart(MultiPart::alternative_plain_html(
                    body.to_string(),
                    format!("<pre>{}</pre>", escape_html(body)),
                ))
                .singlepart(
                    Attachment::new(file_name.to_string()).body(
                        calendar.to_string(),
                        ContentType::parse("text/calendar; charset=utf-8; method=PUBLISH")
                            .expect("a constant content type"),
                    ),
                ),
        )
        .map_err(|problem| {
            DomainError::Validation(format!("The message is not valid: {problem}"))
        })?;
    transport(account)?.send(message).await.map_err(|problem| {
        DomainError::Conflict(format!(
            "The e-mail could not be sent: {}",
            problem
                .to_string()
                .lines()
                .next()
                .unwrap_or("unknown answer")
        ))
    })?;
    Ok(())
}

/// The transport of the account: `smtps` on 465, `STARTTLS` when the server offers it.
fn transport(account: &MailAccount) -> Result<AsyncSmtpTransport<Tokio1Executor>, DomainError> {
    let server = account.smtp_server.trim();
    if server.is_empty() || account.smtp_port == 0 {
        return Err(DomainError::Validation(
            "For this account no e-mail server is set up.".to_string(),
        ));
    }
    let builder = if account.smtp_port == 465 {
        AsyncSmtpTransport::<Tokio1Executor>::relay(server).map_err(|problem| {
            DomainError::Validation(format!("The e-mail server cannot be used: {problem}"))
        })?
    } else {
        // A server that offers `STARTTLS` is upgraded, one that does not stays plain.
        let tls = TlsParameters::new(server.to_string()).map_err(|problem| {
            DomainError::Validation(format!("The e-mail server cannot be used: {problem}"))
        })?;
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(server).tls(Tls::Opportunistic(tls))
    }
    .port(account.smtp_port);
    let builder = if account.password.trim().is_empty() {
        builder
    } else {
        builder.credentials(Credentials::new(
            account.email.trim().to_string(),
            account.password.clone(),
        ))
    };
    Ok(builder.build())
}

/// Keeps a body of plain text from becoming markup.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(port: u16) -> MailAccount {
        MailAccount {
            email: "planung@theater.example".into(),
            smtp_server: "smtp.theater.example".into(),
            smtp_port: port,
            password: "geheim".into(),
            signature: None,
            bcc: None,
        }
    }

    #[test]
    fn a_missing_server_is_named() {
        let mut konto = account(587);
        konto.smtp_server = String::new();
        let error = transport(&konto).unwrap_err();
        assert!(
            matches!(&error, DomainError::Validation(text) if text.contains("no e-mail server")),
            "{error:?}"
        );
        let mut ohne_port = account(587);
        ohne_port.smtp_port = 0;
        assert!(transport(&ohne_port).is_err());
    }
}
