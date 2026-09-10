use std::sync::Arc;

use propflow_domain::identity::PasswordResetCode;
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::{Email, PasswordResetCodeId};

use crate::error::{AppError, AppResult};
use crate::password_policy;
use crate::ports::{
    Notification, NotificationContent, Notifier, OpaqueTokenFactory, PasswordHasher,
};

#[derive(Debug, Clone)]
pub struct RequestPasswordResetCommand {
    pub email: Email,
}

#[derive(Debug, Clone)]
pub struct ConfirmPasswordResetCommand {
    /// Il codice in chiaro, come arriva incollato dall'email. Non finisce nei log.
    pub code: String,
    pub new_password: String,
}

/// Chiede un codice di recupero.
///
/// # Il pool privilegiato, e perché qui è legittimo
///
/// La regola 27 di `CLAUDE.md` riserva `begin_privileged()` ai flussi in cui un
/// contesto di sicurezza **non può** esistere. Chi ha dimenticato la password non è
/// autenticato **per definizione**: non c'è un `current_user_id` da installare, e la
/// Row Level Security non avrebbe nulla su cui appoggiarsi.
///
/// È la differenza con l'accettazione di un invito, dove
/// [ADR-020](../../../../../docs/08-decisioni.md) ha scelto di estendere la RLS
/// invece di aggirarla: là il contesto esiste — chi accetta è autenticato — e manca
/// solo la membership. Qui non esiste affatto. Vedi ADR-052.
///
/// # La risposta è sempre la stessa, e anche il tempo
///
/// Un indirizzo noto e uno sconosciuto ricevono `202`, sempre: distinguerli sarebbe
/// un oracolo per sapere chi ha un account ([ADR-017](../../../../../docs/08-decisioni.md)).
///
/// **E il costo dei due rami è lo stesso**, che è la metà che si dimentica insieme:
/// per un indirizzo senza account si calcola comunque lo hash di un valore fittizio,
/// altrimenti la latenza direbbe ciò che la risposta tace.
pub struct RequestPasswordReset {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    codes: Arc<dyn OpaqueTokenFactory>,
    notifier: Arc<dyn Notifier>,
    clock: Arc<dyn Clock>,
}

impl RequestPasswordReset {
    #[must_use]
    pub const fn new(
        unit_of_work: Arc<dyn UnitOfWorkFactory>,
        codes: Arc<dyn OpaqueTokenFactory>,
        notifier: Arc<dyn Notifier>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            unit_of_work,
            codes,
            notifier,
            clock,
        }
    }

    pub async fn execute(&self, command: RequestPasswordResetCommand) -> AppResult<()> {
        let now = self.clock.now();
        // Generato **prima** del ramo, e non dentro: è il lavoro che rende i due
        // percorsi indistinguibili nel tempo. Su un indirizzo sconosciuto viene
        // buttato via, ed è precisamente il punto.
        let material = self.codes.generate().map_err(AppError::internal)?;

        let mut uow = self.unit_of_work.begin_privileged().await?;
        let Some(user) = uow.users().find_by_email(&command.email).await? else {
            return Ok(());
        };

        // Il limite agli invii: per **email** e non per IP. Un rifiuto silenzioso,
        // perché dirlo sarebbe di nuovo l'oracolo — «troppo presto» conferma che
        // l'account esiste.
        if let Some(last) = uow.password_reset_codes().last_issued(user.id()).await?
            && !last.allows_resend_at(now)
        {
            return Ok(());
        }

        let code = PasswordResetCode::issue(
            PasswordResetCodeId::generate(),
            user.id(),
            material.hash,
            now,
        );
        uow.password_reset_codes().insert(&code).await?;
        uow.commit().await?;

        // Dopo il commit e best-effort, come l'invito: se il relay non risponde il
        // codice resta valido e la persona può richiederlo. **A differenza
        // dell'invito, però, non c'è una seconda copia**: il codice non torna nella
        // risposta, perché la risposta è la stessa per chiunque. È la ragione per cui
        // un relay caduto va visto nel controllo di prontezza e non qui.
        self.deliver(&command.email, &material.plaintext).await;
        Ok(())
    }

    async fn deliver(&self, email: &Email, code: &str) {
        let notification = Notification {
            to: email.clone(),
            content: NotificationContent::PasswordReset {
                code: code.to_owned(),
                valid_minutes: PasswordResetCode::LIFETIME_MINUTES,
            },
        };
        if let Err(error) = self.notifier.send(&notification).await {
            // **Né l'indirizzo né il codice**: `docs/04-sicurezza.md` vieta email e
            // credenziali nelle tracce, e qui ci sarebbero tutte e due insieme.
            tracing::warn!(error = %error, "codice di recupero non recapitato");
        }
    }
}

/// Consuma un codice e imposta la password nuova.
///
/// # Revoca **tutte** le sessioni, ed è la parte che rende il recupero una difesa
///
/// Chi ha ottenuto l'accesso a un account ha in mano un refresh token che vive
/// trenta giorni: cambiare la password senza revocarlo non lo butta fuori. Resta
/// fuori anche chi recupera, e va bene così — non sa quali sessioni fossero sue.
///
/// # Pool privilegiato, per la stessa ragione di `RequestPasswordReset`
///
/// Chi presenta un codice non è ancora autenticato: sta dimostrando di controllare
/// una casella di posta, non di avere una sessione.
pub struct ConfirmPasswordReset {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    codes: Arc<dyn OpaqueTokenFactory>,
    hasher: Arc<dyn PasswordHasher>,
    clock: Arc<dyn Clock>,
}

impl ConfirmPasswordReset {
    #[must_use]
    pub const fn new(
        unit_of_work: Arc<dyn UnitOfWorkFactory>,
        codes: Arc<dyn OpaqueTokenFactory>,
        hasher: Arc<dyn PasswordHasher>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            unit_of_work,
            codes,
            hasher,
            clock,
        }
    }

    pub async fn execute(&self, command: ConfirmPasswordResetCommand) -> AppResult<()> {
        // La politica **prima** di toccare il database: una password debole è un
        // errore del chiamante e va detto, mentre un codice sbagliato non lo è mai.
        password_policy::validate(&command.new_password)?;
        let now = self.clock.now();
        let code_hash = self.codes.hash(&command.code);

        let mut uow = self.unit_of_work.begin_privileged().await?;
        let Some(mut code) = uow
            .password_reset_codes()
            .find_by_code_hash(&code_hash)
            .await?
        else {
            return Err(AppError::Unauthenticated);
        };
        // Scaduto, già usato e inesistente danno la **stessa** risposta: distinguerli
        // direbbe qualcosa su un account altrui.
        if !code.mark_used(now) {
            return Err(AppError::Unauthenticated);
        }

        let Some(mut user) = uow.users().find_by_id(code.user_id()).await? else {
            return Err(AppError::Unauthenticated);
        };

        let password_hash = self
            .hasher
            .hash(&command.new_password)
            .await
            .map_err(AppError::internal)?;
        user.set_password(password_hash);

        uow.password_reset_codes().mark_used(&code).await?;
        uow.users().update(&user).await?;
        // **Tutte**, e nella stessa transazione del cambio password: se le due cose
        // si separassero, ci sarebbe una finestra in cui la password è nuova e la
        // sessione di chi è entrato con quella vecchia è ancora viva.
        uow.sessions().revoke_all_for_user(user.id()).await?;
        uow.commit().await?;

        Ok(())
    }
}
