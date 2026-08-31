use propflow_domain::ports::{LeaseOverlap, RepositoryError};
use propflow_domain::{Denial, DomainError, Entity};

use crate::ports::StorageError;

/// Condizione di unicità violata, nominata.
///
/// Sostituisce la stringa libera che c'era prima (`resource: "utente con questa
/// email"`) perché da qui nasce il `code` della risposta: la tassonomia deve
/// esprimere la **condizione**, non il sintomo. Un client che riceve
/// `auth.email_taken` sa cosa mostrare; uno che riceve `conflict.duplicate` deve
/// leggere la prosa, che è esattamente ciò che i codici tolgono di mezzo.
///
/// I nomi di vincolo che portano qui stanno nei `map_*_conflict` dei casi d'uso:
/// il vincolo è un dettaglio dello schema e non esce mai in risposta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Conflict {
    /// `users_email`
    #[error("An account with this email address already exists")]
    EmailTaken,
    /// `rooms_property_name`
    #[error("A room with this name already exists in the property")]
    RoomNameTaken,
    /// `org_invitations_pending`
    #[error("A pending invitation for this email address already exists")]
    InvitationAlreadyPending,
    /// `lease_parties_primary`
    #[error("The lease already has a primary party")]
    LeasePrimaryPartyExists,
    /// `org_members_org_user`
    #[error("The user already belongs to this organization")]
    MembershipAlreadyExists,
    /// `utility_bills_dedup_uk`
    ///
    /// L'indice è unico su `(org_id, supplier_vat, invoice_number)` e scatta solo
    /// quando **entrambi** i campi sono valorizzati: due bollette senza numero di
    /// fattura non danno conflitto, il che è corretto e va detto perché sembra che
    /// la deduplica non funzioni.
    #[error("A bill with this invoice number from this supplier already exists")]
    DuplicateUtilityBill,
    /// `payments_idempotency_uk`
    ///
    /// **Non è il caso normale del doppio invio**, che risponde `200` con il
    /// pagamento già registrato: quello si riconosce rileggendo la chiave prima di
    /// inserire. Questo codice resta per i due casi in cui quella rilettura non
    /// basta — la stessa chiave usata per un **altro** addebito, dove un `200` con
    /// un pagamento diverso da quello chiesto sarebbe una bugia, e la chiave già
    /// usata da qualcuno di cui il chiamante non vede i pagamenti, che la RLS gli
    /// nasconde. Vedi ADR-033.
    #[error("This idempotency key was already used for a different payment")]
    PaymentIdempotencyKeyReused,
}

impl Conflict {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmailTaken => "auth.email_taken",
            Self::RoomNameTaken => "room.name_taken",
            Self::InvitationAlreadyPending => "invitation.already_pending",
            Self::LeasePrimaryPartyExists => "lease.primary_party_exists",
            Self::MembershipAlreadyExists => "membership.already_exists",
            Self::DuplicateUtilityBill => "utility_bill.duplicate",
            Self::PaymentIdempotencyKeyReused => "payment.idempotency_key_reused",
        }
    }
}

/// Filtro sintatticamente valido che l'API rifiuta di applicare.
///
/// Non è una [`DomainError::Validation`] perché non c'è nulla di malformato: il
/// valore esiste, ma è derivato e filtrarci sopra darebbe sempre zero righe. Il
/// codice proprio serve a distinguerlo da "`status` non è un valore ammesso", che
/// è un errore diverso sullo stesso campo — con `validation.failed` per entrambi,
/// un client che volesse correggere la query da solo dovrebbe leggere il messaggio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UnsupportedQuery {
    #[error("`expiring` and `expired` are derived from the dates: filter by `in_force` instead")]
    LeaseDerivedStatusFilter,
    /// `?status=overdue` sugli addebiti.
    ///
    /// Stessa forma del filtro sui contratti e stessa ragione: `overdue` è
    /// derivato dalla scadenza e nessuna riga lo porta memorizzato, quindi
    /// filtrarci sopra darebbe **sempre** zero risultati. Una pagina vuota sarebbe
    /// il modo peggiore di dirlo.
    #[error("`overdue` is derived from the due date: filter by `outstanding` instead")]
    ChargeDerivedStatusFilter,
}

impl UnsupportedQuery {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::LeaseDerivedStatusFilter => "lease.derived_status_filter",
            Self::ChargeDerivedStatusFilter => "charge.derived_status_filter",
        }
    }

    /// Campo da riportare in `errors[]`: la forma della risposta resta quella di
    /// una validazione, cambia solo il codice.
    #[must_use]
    pub const fn field(self) -> &'static str {
        match self {
            Self::LeaseDerivedStatusFilter | Self::ChargeDerivedStatusFilter => "status",
        }
    }
}

/// Errori del service layer. Sono la sola cosa che i bordi HTTP devono tradurre:
/// né `DomainError` né `RepositoryError` arrivano fino agli handler.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(#[from] DomainError),

    /// Credenziali assenti o non valide. Il messaggio è volutamente generico:
    /// distinguere "utente inesistente" da "password errata" regala a un
    /// attaccante un oracolo per enumerare gli account. Per lo stesso motivo qui
    /// confluiscono anche sessione scaduta, refresh token riusato e token
    /// d'invito rifiutato, che pure il dominio sa distinguere.
    #[error("authentication failed")]
    Unauthenticated,

    #[error("{0}")]
    Forbidden(Denial),

    /// La RLS ha rifiutato una scrittura che il service layer aveva già
    /// autorizzato. Se scatta è **un difetto nostro**, non un errore del
    /// chiamante: il codice lo nomina perché nei log sia cercabile. Lo stato resta
    /// `403` come prima di ADR-022; portarlo a `500` è una decisione sul
    /// comportamento, non sulla tassonomia.
    #[error("denied by a row level security policy")]
    DeniedByPolicy,

    #[error("{0}")]
    NotFound(Entity),

    #[error("{0}")]
    Conflict(Conflict),

    #[error("{0}")]
    UnsupportedQuery(UnsupportedQuery),

    /// Il periodo richiesto si sovrappone a una locazione già presente. Resta
    /// distinto da [`Self::Integrity`] perché è un errore comprensibile del
    /// chiamante, e merita una risposta che dica cosa è occupato.
    #[error("an active lease already covers {0} for the given period")]
    OverlappingLease(LeaseOverlap),

    /// Un vincolo del database ha rifiutato la scrittura per un motivo che non
    /// sappiamo tradurre in un messaggio di dominio.
    ///
    /// Il codice `conflict.constraint_violated` è deliberatamente muto: dice al
    /// client "conflitto, non so dirti quale". Vederlo nei log ripetutamente sullo
    /// stesso vincolo è il segnale che serve una variante di [`Conflict`].
    #[error("integrity violation: {details}")]
    Integrity { details: String },

    #[error("internal error")]
    Internal { details: String },
}

/// Lo storage oggetti che non risponde è un errore **nostro**, non del chiamante:
/// nulla di ciò che ha inviato può averlo causato. Per questo confluisce in
/// [`AppError::Internal`] invece di avere un codice proprio da gestire lato client.
///
/// `NotFound` compreso: un documento la cui riga esiste e il cui oggetto non c'è è
/// un'incoerenza fra database e bucket, che al chiamante non si può spiegare e a
/// noi serve nei log. Un `404` direbbe "non esiste", che è falso.
impl From<StorageError> for AppError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::NotFound => Self::Internal {
                details: "oggetto assente nello storage per un documento esistente".to_owned(),
            },
            StorageError::Backend { details } => Self::Internal { details },
        }
    }
}

impl AppError {
    pub fn internal(details: impl std::fmt::Display) -> Self {
        Self::Internal {
            details: details.to_string(),
        }
    }

    #[must_use]
    pub const fn not_found(entity: Entity) -> Self {
        Self::NotFound(entity)
    }

    /// Codice stabile della condizione, parte del contratto pubblico (ADR-022).
    ///
    /// Il bordo HTTP lo trascrive senza inventarlo: ogni variante o lo conosce o
    /// lo delega al tipo che lo porta.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Validation(error) => error.code(),
            Self::Unauthenticated => "auth.invalid_credentials",
            Self::Forbidden(denial) => denial.code(),
            Self::DeniedByPolicy => "auth.denied_by_policy",
            Self::NotFound(entity) => entity.code(),
            Self::Conflict(conflict) => conflict.code(),
            Self::UnsupportedQuery(query) => query.code(),
            Self::OverlappingLease(_) => "lease.period_overlap",
            Self::Integrity { .. } => "conflict.constraint_violated",
            Self::Internal { .. } => "internal.error",
        }
    }
}

impl From<RepositoryError> for AppError {
    fn from(error: RepositoryError) -> Self {
        match error {
            // Il nome del vincolo è un dettaglio dello schema: resta nei log,
            // non nella risposta. La mappatura verso una condizione parlante la fa
            // lo use case, che sa cosa stava scrivendo.
            RepositoryError::Conflict { constraint } => Self::Integrity {
                details: format!("vincolo `{constraint}`"),
            },
            RepositoryError::Integrity { details } => Self::Integrity { details },
            RepositoryError::OverlappingLease(target) => Self::OverlappingLease(target),
            RepositoryError::RowLevelSecurity => Self::DeniedByPolicy,
            RepositoryError::Backend { details } => Self::Internal { details },
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_conflict_has_a_distinct_code() {
        let codes = [
            Conflict::EmailTaken.code(),
            Conflict::RoomNameTaken.code(),
            Conflict::InvitationAlreadyPending.code(),
            Conflict::LeasePrimaryPartyExists.code(),
            Conflict::MembershipAlreadyExists.code(),
            Conflict::DuplicateUtilityBill.code(),
            Conflict::PaymentIdempotencyKeyReused.code(),
        ];
        let mut unique = codes.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            codes.len(),
            "due conflitti condividono un codice: il client non li distinguerebbe"
        );
    }

    /// Il codice muto è riservato ai vincoli che non abbiamo ancora nominato. Se
    /// una condizione nota ci finisse sopra, il client perderebbe l'informazione.
    #[test]
    fn only_the_unmapped_constraint_is_mute() {
        assert_eq!(
            AppError::Integrity {
                details: "vincolo `qualcosa_di_nuovo`".to_owned()
            }
            .code(),
            "conflict.constraint_violated"
        );
        assert_eq!(
            AppError::Conflict(Conflict::EmailTaken).code(),
            "auth.email_taken"
        );
    }

    /// La RLS che rifiuta una scrittura già autorizzata non deve confondersi con
    /// un permesso mancante del chiamante: sono due diagnosi diverse.
    #[test]
    fn policy_denial_is_not_a_missing_role() {
        assert_ne!(
            AppError::DeniedByPolicy.code(),
            AppError::Forbidden(Denial::StaffRoleRequired).code()
        );
    }
}
