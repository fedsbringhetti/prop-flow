use async_trait::async_trait;
use propflow_domain::shared::Email;

/// Errori dell'invio di notifiche.
#[derive(Debug, Clone, thiserror::Error)]
pub enum NotifyError {
    #[error("destinatario rifiutato dal fornitore")]
    RejectedRecipient,
    #[error("fornitore di notifiche non disponibile: {details}")]
    Backend { details: String },
}

/// Contenuto di una notifica.
///
/// A differenza degli eventi di dominio, una notifica **deve** trasportare dati
/// personali: senza l'indirizzo non si può recapitare nulla. Per questo non
/// implementa `Debug` derivato — vedi sotto — e non va registrata nei log.
pub struct Notification {
    pub to: Email,
    pub content: NotificationContent,
}

/// Cosa comunicare. Un enum e non un testo già composto: la resa in HTML o in
/// testo semplice appartiene all'adattatore, e passargli una stringa pronta
/// significherebbe scrivere il template nel service layer.
pub enum NotificationContent {
    /// Invito di un inquilino. `accept_token` è il token **in chiaro**: esiste solo
    /// qui e nella risposta HTTP che lo consegna.
    TenantInvitation {
        organization_name: String,
        accept_token: String,
    },
}

// `Debug` scritto a mano: il derive stamperebbe l'indirizzo del destinatario e il
// token di accettazione, che è una credenziale. La regola è la stessa di
// `PasswordHash` e `TokenHash`.
impl std::fmt::Debug for Notification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Notification")
            .field("to", &"<redatto>")
            .field("content", &self.content)
            .finish()
    }
}

impl std::fmt::Debug for NotificationContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TenantInvitation { .. } => f.write_str("TenantInvitation(<redatto>)"),
        }
    }
}

/// Recapito di notifiche verso l'esterno: email nell'MVP, push in seguito.
///
/// Dichiarata ora e implementata allo step delle notifiche, come `ObjectStorage`:
/// il vincolo "nessun vendor lock-in" si rispetta decidendo la forma
/// dell'astrazione **prima** di scrivere il primo adattatore. Finché non esiste
/// un'implementazione, il token di invito torna nella risposta HTTP e il
/// proprietario lo consegna come preferisce.
#[async_trait]
pub trait Notifier: Send + Sync {
    async fn send(&self, notification: &Notification) -> Result<(), NotifyError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_does_not_leak_the_recipient_nor_the_token() {
        let notification = Notification {
            to: Email::parse("inquilino@example.test").expect("email valida"),
            content: NotificationContent::TenantInvitation {
                organization_name: "Org di prova".to_owned(),
                accept_token: "token-segretissimo".to_owned(),
            },
        };
        let rendered = format!("{notification:?}");
        assert!(!rendered.contains("inquilino@example.test"), "{rendered}");
        assert!(!rendered.contains("token-segretissimo"), "{rendered}");
    }
}
