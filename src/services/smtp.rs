// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::Path;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message as LettreMessage, SmtpTransport, Transport};
use serde::{Deserialize, Serialize};

use super::outbox::{record_to_sent_maildir, remove_from_outbox, save_failed_to_drafts, OutboxMessage};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SmtpAccountConfig {
    pub id: String,
    pub name: String,
    pub email: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
}

/// Dispatches an outbox message through the appropriate transport.
/// If real SMTP credentials are provided, it dispatches over the network via `lettre`.
/// Otherwise, it performs a local Maildir sent-recording for offline/demo operation.
pub fn send_message_transport(
    mail_root: &Path,
    msg: &OutboxMessage,
    config: Option<&SmtpAccountConfig>,
) -> Result<String, String> {
    // 1. Build Lettre MIME Message
    let from_addr = msg
        .from
        .parse()
        .map_err(|e| format!("Invalid sender address '{}': {e}", msg.from))?;

    let to_addr = msg
        .to
        .parse()
        .map_err(|e| format!("Invalid recipient address '{}': {e}", msg.to))?;

    let mut builder = LettreMessage::builder()
        .from(from_addr)
        .to(to_addr)
        .subject(&msg.subject)
        .header(ContentType::TEXT_PLAIN);

    if let Some(ref reply_id) = msg.in_reply_to {
        let clean_id = reply_id.trim_matches(|c| c == '<' || c == '>');
        builder = builder.in_reply_to(format!("<{clean_id}>"));
    }

    for ref_id in &msg.references {
        let clean_id = ref_id.trim_matches(|c| c == '<' || c == '>');
        builder = builder.references(format!("<{clean_id}>"));
    }

    let lettre_msg = builder
        .body(msg.body.clone())
        .map_err(|e| format!("Failed to build email message: {e}"))?;

    // 2. Dispatch via real SMTP if host and credentials configured
    if let Some(cfg) = config {
        if !cfg.host.is_empty() && cfg.host != "localhost" && cfg.host != "demo" {
            let mut transport_builder = SmtpTransport::relay(&cfg.host)
                .map_err(|e| format!("SMTP relay configuration error: {e}"))?
                .port(cfg.port);

            if let Some(ref pass) = cfg.password {
                let creds = Credentials::new(cfg.username.clone(), pass.clone());
                transport_builder = transport_builder.credentials(creds);
            }

            let mailer = transport_builder.build();
            mailer
                .send(&lettre_msg)
                .map_err(|e| format!("SMTP delivery failed: {e}"))?;
        }
    }

    // 3. Record sent message in local Maildir Sent folder
    let _ = record_to_sent_maildir(mail_root, &msg.account_name, msg)?;

    // 4. Clean up the outbox queue
    remove_from_outbox(&msg.id);

    Ok(format!("Message sent to {}", msg.to))
}

/// Dispatches the outbox message in a background worker thread.
/// Upon failure, preserves the draft to disk so user work is never lost.
pub fn dispatch_in_background<F>(
    mail_root: std::path::PathBuf,
    msg: OutboxMessage,
    config: Option<SmtpAccountConfig>,
    on_complete: F,
) where
    F: FnOnce(Result<String, String>) + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();

    std::thread::spawn(move || {
        let result = send_message_transport(&mail_root, &msg, config.as_ref());
        if let Err(ref e) = result {
            let _ = save_failed_to_drafts(&msg);
            tracing::error!("Send failed for {}: {e}. Preserved to drafts.", msg.id);
        }
        let _ = tx.send(result);
    });

    let mut callback = Some(on_complete);
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        match rx.try_recv() {
            Ok(res) => {
                if let Some(cb) = callback.take() {
                    cb(res);
                }
                gtk::glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                if let Some(cb) = callback.take() {
                    cb(Err("Background SMTP worker disconnected".to_string()));
                }
                gtk::glib::ControlFlow::Break
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_message_transport_local_fallback() {
        let temp_dir = std::env::temp_dir().join(format!("iris_smtp_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);

        let msg = OutboxMessage::new(
            "demo",
            "bry@omarchy.org",
            "alex@hermes.dev",
            "Local Test",
            "Local delivery body",
            None,
            vec![],
        );

        let res = send_message_transport(&temp_dir, &msg, None);
        assert!(res.is_ok());

        // Verify it was recorded to Sent/cur/
        let sent_file = temp_dir.join("demo/Sent/cur").join(format!("{}:2,S", msg.id));
        assert!(sent_file.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_send_message_transport_invalid_addresses() {
        let temp_dir = std::env::temp_dir().join(format!("iris_smtp_invalid_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);

        let msg = OutboxMessage::new(
            "demo",
            "not-an-email",
            "alex@hermes.dev",
            "Invalid From Test",
            "Body",
            None,
            vec![],
        );

        let res = send_message_transport(&temp_dir, &msg, None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Invalid sender address"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_send_message_transport_with_reply_and_references() {
        let temp_dir = std::env::temp_dir().join(format!("iris_smtp_reply_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);

        let msg = OutboxMessage::new(
            "work",
            "brayan@work.com",
            "colleague@work.com",
            "Re: Project status",
            "Here is the update.",
            Some("parent-id-123@work.com".to_string()),
            vec!["parent-id-123@work.com".to_string(), "root-id-001@work.com".to_string()],
        );

        let res = send_message_transport(&temp_dir, &msg, None);
        assert!(res.is_ok());

        let sent_file = temp_dir.join("work/Sent/cur").join(format!("{}:2,S", msg.id));
        assert!(sent_file.exists());
        let content = std::fs::read_to_string(&sent_file).unwrap();
        assert!(content.contains("In-Reply-To: <parent-id-123@work.com>"));
        assert!(content.contains("References: <parent-id-123@work.com> <root-id-001@work.com>"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
