use chrono::{DateTime, Utc};

use crate::error::{Denial, DomainError, DomainResult};
use crate::shared::{Email, UserId, text};

/// Hash della password, opaco per il dominio.
///
/// Il dominio non sa e non deve sapere quale algoritmo lo ha prodotto: Argon2id
/// oggi, qualcosa d'altro domani. L'unica regola qui è che non sia vuoto.
#[derive(Clone, PartialEq, Eq)]
pub struct PasswordHash(String);

impl PasswordHash {
    pub fn new(value: impl Into<String>) -> DomainResult<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainError::validation("password_hash", "cannot be empty"));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// `Debug` non mostra il contenuto: un hash finito in un log è un hash offerto
/// a un attacco offline.
impl std::fmt::Debug for PasswordHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PasswordHash(<redatto>)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformRole {
    User,
    Support,
    Admin,
}

impl PlatformRole {
    #[must_use]
    pub const fn is_admin(self) -> bool {
        matches!(self, Self::Admin)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStatus {
    Pending,
    Active,
    Suspended,
    Deactivated,
}

impl UserStatus {
    /// Solo un utente attivo può autenticarsi. `Pending` esiste per il flusso di
    /// verifica email, `Suspended` per un blocco amministrativo.
    #[must_use]
    pub const fn can_authenticate(self) -> bool {
        matches!(self, Self::Active)
    }
}

#[derive(Debug, Clone)]
pub struct User {
    id: UserId,
    email: Email,
    email_verified_at: Option<DateTime<Utc>>,
    password_hash: Option<PasswordHash>,
    full_name: String,
    phone: Option<String>,
    locale: String,
    platform_role: PlatformRole,
    status: UserStatus,
    last_login_at: Option<DateTime<Utc>>,
    failed_login_count: u8,
    locked_until: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl User {
    /// Numero di tentativi falliti consecutivi dopo i quali l'account si blocca.
    pub const MAX_FAILED_LOGINS: u8 = 10;

    /// Registrazione di un proprietario con credenziali proprie.
    ///
    /// Nasce già `Active`: nell'MVP non c'è verifica email bloccante, ma il campo
    /// `email_verified_at` è presente perché il flusso arriverà senza migrazione.
    pub fn register(
        id: UserId,
        email: Email,
        full_name: &str,
        password_hash: PasswordHash,
        now: DateTime<Utc>,
    ) -> DomainResult<Self> {
        Ok(Self {
            id,
            email,
            email_verified_at: None,
            password_hash: Some(password_hash),
            full_name: text::required("full_name", full_name, 200)?,
            phone: None,
            locale: "it-IT".to_owned(),
            platform_role: PlatformRole::User,
            status: UserStatus::Active,
            last_login_at: None,
            failed_login_count: 0,
            locked_until: None,
            created_at: now,
        })
    }

    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn from_persistence(
        id: UserId,
        email: Email,
        email_verified_at: Option<DateTime<Utc>>,
        password_hash: Option<PasswordHash>,
        full_name: String,
        phone: Option<String>,
        locale: String,
        platform_role: PlatformRole,
        status: UserStatus,
        last_login_at: Option<DateTime<Utc>>,
        failed_login_count: u8,
        locked_until: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            email,
            email_verified_at,
            password_hash,
            full_name,
            phone,
            locale,
            platform_role,
            status,
            last_login_at,
            failed_login_count,
            locked_until,
            created_at,
        }
    }

    /// Verifica che l'account sia in condizione di autenticarsi, senza guardare
    /// la password: quella la confronta l'infrastruttura, che conosce l'algoritmo.
    pub fn ensure_can_authenticate(&self, now: DateTime<Utc>) -> DomainResult<&PasswordHash> {
        if !self.status.can_authenticate() {
            return Err(DomainError::Forbidden(Denial::AccountInactive));
        }
        if self.locked_until.is_some_and(|until| until > now) {
            return Err(DomainError::Forbidden(Denial::AccountLocked));
        }
        self.password_hash
            .as_ref()
            .ok_or(DomainError::Forbidden(Denial::NoLocalCredentials))
    }

    pub fn record_successful_login(&mut self, now: DateTime<Utc>) {
        self.last_login_at = Some(now);
        self.failed_login_count = 0;
        self.locked_until = None;
    }

    /// Registra un tentativo fallito e blocca l'account al superamento della soglia.
    /// Il blocco è temporaneo: un attacco a dizionario non deve tradursi in un
    /// lockout permanente sfruttabile come denial of service verso l'utente.
    pub fn record_failed_login(&mut self, now: DateTime<Utc>, lock_for: chrono::Duration) {
        self.failed_login_count = self.failed_login_count.saturating_add(1);
        if self.failed_login_count >= Self::MAX_FAILED_LOGINS {
            self.locked_until = Some(now + lock_for);
        }
    }

    /// Sostituisce la password, e **sblocca l'account**.
    ///
    /// Il contatore dei tentativi falliti e il blocco temporaneo si azzerano, ed è
    /// deliberato: chi ha dimostrato di controllare la casella di posta ha superato
    /// una prova più forte di quella che il blocco difende, e lasciarlo fuori per
    /// altri quindici minuti sarebbe punire la persona sbagliata.
    ///
    /// **Non revoca le sessioni**: quelle non sono un attributo dell'utente e
    /// vivono in `app.user_sessions`. Lo fa il caso d'uso, che è anche l'unico
    /// posto da cui si vede che le due cose vanno insieme ([ADR-052](../../../../docs/08-decisioni.md)).
    pub fn set_password(&mut self, password_hash: PasswordHash) {
        self.password_hash = Some(password_hash);
        self.failed_login_count = 0;
        self.locked_until = None;
    }

    pub fn mark_email_verified(&mut self, now: DateTime<Utc>) {
        if self.email_verified_at.is_none() {
            self.email_verified_at = Some(now);
        }
    }

    #[must_use]
    pub const fn id(&self) -> UserId {
        self.id
    }
    #[must_use]
    pub const fn email(&self) -> &Email {
        &self.email
    }
    #[must_use]
    pub const fn email_verified_at(&self) -> Option<DateTime<Utc>> {
        self.email_verified_at
    }
    #[must_use]
    pub const fn password_hash(&self) -> Option<&PasswordHash> {
        self.password_hash.as_ref()
    }
    #[must_use]
    pub fn full_name(&self) -> &str {
        &self.full_name
    }
    #[must_use]
    pub fn phone(&self) -> Option<&str> {
        self.phone.as_deref()
    }
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.locale
    }
    #[must_use]
    pub const fn platform_role(&self) -> PlatformRole {
        self.platform_role
    }
    #[must_use]
    pub const fn status(&self) -> UserStatus {
        self.status
    }
    #[must_use]
    pub const fn last_login_at(&self) -> Option<DateTime<Utc>> {
        self.last_login_at
    }
    #[must_use]
    pub const fn failed_login_count(&self) -> u8 {
        self.failed_login_count
    }
    #[must_use]
    pub const fn locked_until(&self) -> Option<DateTime<Utc>> {
        self.locked_until
    }
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user() -> User {
        User::register(
            UserId::generate(),
            Email::parse("mario@example.it").expect("email valida"),
            "Mario Rossi",
            PasswordHash::new("$argon2id$stub").expect("hash valido"),
            Utc::now(),
        )
        .expect("utente valido")
    }

    #[test]
    fn password_hash_is_not_leaked_by_debug() {
        let hash = PasswordHash::new("$argon2id$v=19$segretissimo").expect("valido");
        assert_eq!(format!("{hash:?}"), "PasswordHash(<redatto>)");
    }

    #[test]
    fn registered_user_can_authenticate() {
        assert!(user().ensure_can_authenticate(Utc::now()).is_ok());
    }

    #[test]
    fn lockout_triggers_after_threshold_and_blocks_authentication() {
        let mut user = user();
        let now = Utc::now();
        for _ in 0..User::MAX_FAILED_LOGINS {
            user.record_failed_login(now, chrono::Duration::minutes(15));
        }
        assert!(user.locked_until().is_some());
        assert!(user.ensure_can_authenticate(now).is_err());
        // scaduto il blocco l'utente torna autenticabile senza intervento manuale
        assert!(
            user.ensure_can_authenticate(now + chrono::Duration::minutes(16))
                .is_ok()
        );
    }

    #[test]
    fn successful_login_clears_the_lockout_counter() {
        let mut user = user();
        user.record_failed_login(Utc::now(), chrono::Duration::minutes(15));
        user.record_successful_login(Utc::now());
        assert_eq!(user.failed_login_count(), 0);
        assert!(user.locked_until().is_none());
    }
}
