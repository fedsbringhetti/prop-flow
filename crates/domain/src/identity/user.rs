use chrono::{DateTime, Utc};

use crate::error::{Denial, DomainError, DomainResult};
use crate::property::Patch;
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

/// Ciò che una persona corregge di sé.
///
/// **Tre campi, e l'email non è fra questi.** Non è una mancanza: `users.email` è
/// la credenziale d'accesso e la chiave con cui tre cose si ritrovano — gli inviti
/// pendenti, che uniscono per indirizzo; le policy che poggiano su
/// `app.current_user_email()`; e la riga di `lease_parties` che l'accettazione
/// cerca. Cambiarla bene richiede un giro di verifica sul nuovo indirizzo, che è
/// fuori perimetro; cambiarla male dà un account che non riceve più niente e un
/// invito che non si può più accettare, **con un sintomo muto in entrambi i casi**.
///
/// Nemmeno la password: quella ha un percorso suo, che pretende la vecchia.
#[derive(Debug, Clone, Default)]
pub struct UserProfileChanges {
    pub full_name: Option<String>,
    /// `Patch` e non `Option`: un telefono si toglie, e senza la distinzione fra
    /// «non menzionato» e «azzerato» un PATCH non potrebbe mai cancellarlo.
    pub phone: Patch<String>,
    pub locale: Option<String>,
}

impl UserProfileChanges {
    /// Nomi dei campi menzionati dal chiamante, nell'ordine di dichiarazione.
    ///
    /// Finiscono nell'evento al posto dei valori: `docs/04-sicurezza.md` vieta i
    /// dati personali negli eventi di dominio, e un nome e un telefono lo sono.
    #[must_use]
    pub fn changed_field_names(&self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        if self.full_name.is_some() {
            fields.push("full_name");
        }
        if self.phone.is_some() {
            fields.push("phone");
        }
        if self.locale.is_some() {
            fields.push("locale");
        }
        fields
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changed_field_names().is_empty()
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

    /// Sostituisce la password **da dentro**, e non sblocca l'account.
    ///
    /// È la differenza con [`User::set_password`], ed è deliberata in due sensi.
    /// Il primo è di privilegi: la migration `0038` concede al ruolo applicativo
    /// `full_name`, `phone`, `locale` e `password_hash` e nient'altro, quindi
    /// `failed_login_count` e `locked_until` **non sono scrivibili** da una
    /// richiesta autenticata — un metodo che li azzerasse mentirebbe al chiamante,
    /// perché la scrittura non arriverebbe in database.
    ///
    /// Il secondo è di significato: chi recupera la password ha dimostrato di
    /// controllare la casella di posta, cioè ha superato una prova **diversa** da
    /// quella che il blocco difende. Chi la cambia da dentro sta rifacendo la
    /// stessa prova di sempre — conosce la password corrente — e non c'è motivo di
    /// annullare un blocco che quella prova non supera. Un account bloccato lo
    /// resta per i suoi quindici minuti.
    pub fn change_password(&mut self, password_hash: PasswordHash) {
        self.password_hash = Some(password_hash);
    }

    /// Applica una correzione parziale ai propri dati anagrafici.
    ///
    /// Restituisce `false` quando il chiamante non ha menzionato nessun campo: il
    /// caso d'uso lo usa per non scrivere e non emettere un evento per una
    /// richiesta vuota.
    pub fn apply_profile(&mut self, changes: UserProfileChanges) -> DomainResult<bool> {
        if changes.is_empty() {
            return Ok(false);
        }
        if let Some(full_name) = changes.full_name {
            self.full_name = text::required("full_name", &full_name, 200)?;
        }
        if let Some(phone) = changes.phone {
            self.phone = text::optional("phone", phone, 32)?
                .map(validate_phone)
                .transpose()?;
        }
        if let Some(locale) = changes.locale {
            self.locale = validate_locale(&text::required("locale", &locale, 35)?)?;
        }
        Ok(true)
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

/// Il telefono, con la **stessa forma** del `CHECK` di `app.users.phone`.
///
/// La regola è ripetuta qui e nello schema di proposito, come per i limiti di
/// lunghezza di `text`: senza, un numero malformato passerebbe il dominio e
/// morirebbe sul vincolo del database, cioè con un `409 conflict.constraint_violated`
/// dove il chiamante merita un `422` che nomina il campo.
fn validate_phone(value: String) -> DomainResult<String> {
    let digits = value.chars().filter(char::is_ascii_digit).count();
    let shape_ok = value.chars().enumerate().all(|(index, c)| {
        c.is_ascii_digit() || matches!(c, ' ' | '(' | ')' | '.' | '-') || (index == 0 && c == '+')
    });
    if !shape_ok || !(6..=32).contains(&value.chars().count()) || digits == 0 {
        return Err(DomainError::validation(
            "phone",
            "must be 6 to 32 characters of digits, spaces and ( ) . - with an optional leading +",
        ));
    }
    Ok(value)
}

/// La lingua, come etichetta BCP-47 plausibile.
///
/// Non si verifica che esista davvero — l'elenco delle lingue non appartiene a
/// questo dominio — ma che abbia la **forma**, ed è una difesa concreta e non
/// formale: il client formatta date e importi con `Intl` passando questo valore
/// (regola 19), e `Intl.DateTimeFormat` **solleva** su un'etichetta malformata. Un
/// `locale` di spazzatura accettato qui diventa una schermata che non si apre più,
/// e il rimedio sarebbe un `UPDATE` a mano.
fn validate_locale(value: &str) -> DomainResult<String> {
    let well_formed = !value.is_empty()
        && value.split('-').enumerate().all(|(index, part)| {
            let len = part.chars().count();
            part.chars().all(|c| c.is_ascii_alphanumeric())
                && if index == 0 {
                    (2..=8).contains(&len) && part.chars().all(|c| c.is_ascii_alphabetic())
                } else {
                    (1..=8).contains(&len)
                }
        });
    if !well_formed {
        return Err(DomainError::validation(
            "locale",
            "must be a BCP 47 language tag, such as `it-IT`",
        ));
    }
    Ok(value.to_owned())
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

    #[test]
    fn an_empty_patch_changes_nothing_and_says_so() {
        let mut user = user();
        assert!(
            !user
                .apply_profile(UserProfileChanges::default())
                .expect("valido")
        );
        assert_eq!(user.full_name(), "Mario Rossi");
    }

    #[test]
    fn a_profile_patch_sets_what_it_names_and_clears_what_it_nulls() {
        let mut user = user();
        assert!(
            user.apply_profile(UserProfileChanges {
                full_name: Some("  Maria Rossi  ".to_owned()),
                phone: Some(Some("+39 333 1234567".to_owned())),
                locale: Some("en-GB".to_owned()),
            })
            .expect("valido")
        );
        assert_eq!(user.full_name(), "Maria Rossi");
        assert_eq!(user.phone(), Some("+39 333 1234567"));
        assert_eq!(user.locale(), "en-GB");

        // Il telefono si toglie, ed è la ragione per cui è un `Patch` e non un
        // `Option`: il nome, non menzionato, resta quello di prima.
        assert!(
            user.apply_profile(UserProfileChanges {
                phone: Some(None),
                ..UserProfileChanges::default()
            })
            .expect("valido")
        );
        assert_eq!(user.phone(), None);
        assert_eq!(user.full_name(), "Maria Rossi");
    }

    /// La forma del telefono è quella del `CHECK` di `app.users.phone`: senza
    /// questa validazione il rifiuto arriverebbe dal database, come un conflitto di
    /// vincolo invece che come un `422` che nomina il campo.
    #[test]
    fn a_malformed_phone_is_a_validation_error_and_not_a_constraint_violation() {
        for bad in ["abc123", "12345", "+39-333-123456789012345678901234567890"] {
            let mut user = user();
            assert!(
                user.apply_profile(UserProfileChanges {
                    phone: Some(Some(bad.to_owned())),
                    ..UserProfileChanges::default()
                })
                .is_err(),
                "`{bad}` doveva essere rifiutato"
            );
        }
    }

    /// `Intl.DateTimeFormat` **solleva** su un'etichetta malformata: un `locale` di
    /// spazzatura accettato qui diventa una schermata che non si apre più.
    #[test]
    fn a_malformed_locale_is_refused() {
        for bad in ["", "it_IT", "it--IT", "1t-IT", "abcdefghi-IT"] {
            let mut user = user();
            assert!(
                user.apply_profile(UserProfileChanges {
                    locale: Some(bad.to_owned()),
                    ..UserProfileChanges::default()
                })
                .is_err(),
                "`{bad}` doveva essere rifiutato"
            );
        }
    }

    /// Cambiare la password **da dentro** non sblocca l'account, al contrario del
    /// recupero: la migration `0038` non concede `locked_until` a una richiesta
    /// autenticata, e un metodo che l'azzerasse mentirebbe al chiamante.
    #[test]
    fn changing_the_password_from_the_inside_does_not_lift_a_lockout() {
        let mut user = user();
        let now = Utc::now();
        for _ in 0..User::MAX_FAILED_LOGINS {
            user.record_failed_login(now, chrono::Duration::minutes(15));
        }

        user.change_password(PasswordHash::new("$argon2id$nuovo").expect("hash valido"));
        assert_eq!(
            user.password_hash().map(PasswordHash::as_str),
            Some("$argon2id$nuovo")
        );
        assert!(user.locked_until().is_some());
        assert_eq!(user.failed_login_count(), User::MAX_FAILED_LOGINS);

        // Il recupero, invece, sblocca: le due prove d'identità sono diverse.
        user.set_password(PasswordHash::new("$argon2id$recuperata").expect("hash valido"));
        assert!(user.locked_until().is_none());
        assert_eq!(user.failed_login_count(), 0);
    }
}
