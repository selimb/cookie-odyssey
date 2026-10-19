//! Generic SMTP email sending.
use anyhow::Context as _;
use app_config::EmailConfig;
use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport as _, Message, Tokio1Executor,
};

const FROM_NAME: &str = "Cookie Odyssey";

#[derive(Debug, Clone)]
pub struct EmailRecipient {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone)]
pub struct EmailMessage {
    pub to: EmailRecipient,
    pub subject: String,
    pub text: String,
    pub html: String,
}

/// Sends emails over SMTP.
#[derive(Debug)]
pub struct EmailClient {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
    reply_to: Mailbox,
}

impl EmailClient {
    pub fn new(conf: &EmailConfig) -> anyhow::Result<Self> {
        let builder = if conf.smtp_implicit_tls {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&conf.smtp_host)
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&conf.smtp_host)
        }
        .context("Failed to build SMTP transport")?;
        let transport = builder
            .port(conf.smtp_port)
            .credentials(Credentials::new(
                conf.smtp_username.clone(),
                conf.smtp_password.clone(),
            ))
            .build();

        let from = Mailbox::new(
            Some(FROM_NAME.to_string()),
            conf.from_address
                .parse()
                .context("Invalid email from_address")?,
        );
        let reply_to = Mailbox::new(
            None,
            conf.reply_to_address
                .parse()
                .context("Invalid email reply_to_address")?,
        );
        Ok(Self {
            transport,
            from,
            reply_to,
        })
    }

    pub async fn send(&self, msg: EmailMessage) -> anyhow::Result<()> {
        let to = Mailbox::new(
            Some(msg.to.name),
            msg.to
                .address
                .parse()
                .context("Invalid recipient address")?,
        );
        let email = Message::builder()
            .from(self.from.clone())
            .reply_to(self.reply_to.clone())
            .to(to)
            .subject(msg.subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(msg.text),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(msg.html),
                    ),
            )
            .context("Failed to build email")?;
        self.transport
            .send(email)
            .await
            .context("Failed to send email")?;
        Ok(())
    }
}
