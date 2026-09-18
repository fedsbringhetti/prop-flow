use chrono::{DateTime, Duration, Utc};

use crate::identity::TokenHash;
use crate::shared::{PasswordResetCodeId, UserId};

/// Un codice di recupero della password: **una credenziale a tempo, a uso singolo**.
///
/// Porta lo **hash** e non il codice, come [`RefreshSession`](super::RefreshSession) e
/// come [`Invitation`](super::Invitation): il valore in chiaro esiste solo nell'email,
/// e chi legge il database non trova niente di utilizzabile ([ADR-052](../../../../docs/08-decisioni.md)).
#[derive(Debug, Clone)]
pub struct PasswordResetCode {
    id: PasswordResetCodeId,
    user_id: UserId,
    code_hash: TokenHash,
    expires_at: DateTime<Utc>,
    used_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl PasswordResetCode {
    /// Quanto vive un codice.
    ///
    /// **Molto più corto di un invito**, che dura sette giorni, e la differenza è
    /// deliberata: un invito lo si riceve e lo si usa con calma, un recupero lo si
    /// chiede *mentre* si sta cercando di entrare. Trenta minuti coprono il caso
    /// vero — apro la posta, copio, incollo — e riducono la finestra in cui una
    /// casella compromessa vale un account.
    pub const LIFETIME_MINUTES: i64 = 30;

    /// Quanto si aspetta prima di poterne chiedere un altro.
    ///
    /// È il limite agli invii, ed esiste perché un recupero senza freno è un modo
    /// per far arrivare cinquanta email a un indirizzo altrui. Due minuti non danno
    /// fastidio a chi ha davvero sbagliato indirizzo al primo tentativo, e rendono
    /// l'abuso inutile.
    pub const RESEND_COOLDOWN_MINUTES: i64 = 2;

    #[must_use]
    pub fn issue(
        id: PasswordResetCodeId,
        user_id: UserId,
        code_hash: TokenHash,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            code_hash,
            expires_at: now + Duration::minutes(Self::LIFETIME_MINUTES),
            used_at: None,
            created_at: now,
        }
    }

    #[must_use]
    pub const fn from_persistence(
        id: PasswordResetCodeId,
        user_id: UserId,
        code_hash: TokenHash,
        expires_at: DateTime<Utc>,
        used_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            code_hash,
            expires_at,
            used_at,
            created_at,
        }
    }

    /// Il codice è ancora spendibile?
    ///
    /// **Scaduto e già usato sono la stessa risposta**, e non è pigrizia: chi
    /// presenta un codice che non vale non deve poter distinguere «esisteva e l'ho
    /// consumato» da «esisteva e non vale più», che direbbe qualcosa su un account
    /// altrui. Un `bool` e non un errore tipizzato per la stessa ragione: la
    /// risposta sul filo è comunque una sola, e una variante di `Denial` che non
    /// arriva mai a nessuno sarebbe un codice d'errore da documentare per niente.
    #[must_use]
    pub fn is_usable(&self, now: DateTime<Utc>) -> bool {
        self.used_at.is_none() && self.expires_at > now
    }

    /// Lo consuma, e dice se ci è riuscito.
    ///
    /// Verifica **dentro** invece di fidarsi del chiamante: «a uso singolo» diventa
    /// così un fatto e non una promessa, e una seconda chiamata non riapre nulla.
    #[must_use]
    pub fn mark_used(&mut self, now: DateTime<Utc>) -> bool {
        if !self.is_usable(now) {
            return false;
        }
        self.used_at = Some(now);
        true
    }

    /// Se ne può chiedere un altro, o è troppo presto?
    #[must_use]
    pub fn allows_resend_at(&self, now: DateTime<Utc>) -> bool {
        now >= self.created_at + Duration::minutes(Self::RESEND_COOLDOWN_MINUTES)
    }

    #[must_use]
    pub const fn id(&self) -> PasswordResetCodeId {
        self.id
    }
    #[must_use]
    pub const fn user_id(&self) -> UserId {
        self.user_id
    }
    #[must_use]
    pub const fn code_hash(&self) -> &TokenHash {
        &self.code_hash
    }
    #[must_use]
    pub const fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
    #[must_use]
    pub const fn used_at(&self) -> Option<DateTime<Utc>> {
        self.used_at
    }
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash() -> TokenHash {
        TokenHash::from_bytes([7; 32])
    }

    fn at(minutes: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_757_000_000 + minutes * 60, 0).expect("istante valido")
    }

    fn code(now: DateTime<Utc>) -> PasswordResetCode {
        PasswordResetCode::issue(
            PasswordResetCodeId::generate(),
            UserId::generate(),
            hash(),
            now,
        )
    }

    #[test]
    fn a_fresh_code_is_usable_and_a_used_one_is_not() {
        let mut code = code(at(0));
        assert!(code.is_usable(at(1)));
        assert!(code.mark_used(at(1)));
        assert!(!code.is_usable(at(2)));
        // La seconda volta non passa: è ciò che rende «a uso singolo» un fatto e non
        // una promessa.
        assert!(!code.mark_used(at(2)));
    }

    /// Scaduto e già usato **non si distinguono**, ed è la regola di ADR-052: chi
    /// prova un codice non deve poter dedurre nulla su un account altrui.
    #[test]
    fn an_expired_code_and_a_used_one_are_indistinguishable() {
        let expired = code(at(0));
        let mut used = code(at(0));
        assert!(used.mark_used(at(1)));

        let after_expiry = at(PasswordResetCode::LIFETIME_MINUTES + 1);
        assert!(!expired.is_usable(after_expiry));
        assert!(!used.is_usable(after_expiry));
    }

    /// Il limite agli invii: due minuti, e non uno in meno.
    #[test]
    fn a_second_code_has_to_wait() {
        let code = code(at(0));
        assert!(!code.allows_resend_at(at(1)));
        assert!(code.allows_resend_at(at(PasswordResetCode::RESEND_COOLDOWN_MINUTES)));
    }
}
