use std::fmt;

use rust_decimal::Decimal;

/// Entità che una risposta `404` sa nominare.
///
/// Esiste come tipo e non come stringa perché da qui nasce il `code` della
/// risposta: con un `&'static str` il codice si otterrebbe confrontando testo, e
/// un refuso in un letterale cambierebbe il contratto pubblico senza che nulla se
/// ne accorga. Vedi ADR-022.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Entity {
    #[error("Property not found")]
    Property,
    #[error("Room not found")]
    Room,
    #[error("Lease not found")]
    Lease,
    /// Una parte contrattuale che non esiste, o che chi guarda non vede. Ha un
    /// codice suo e non quello del contratto: chi corregge una quota sta agendo su
    /// una riga precisa, e dirgli «contratto non trovato» manderebbe a cercare
    /// nel posto sbagliato.
    #[error("Lease party not found")]
    LeaseParty,
    #[error("Invitation not found")]
    Invitation,
    /// Un'appartenenza che non esiste: si è chiesto di uscire da
    /// un'organizzazione di cui non si è membri. Ha un codice suo e non quello
    /// dell'organizzazione perché a mancare è la **relazione**, non l'ente — e
    /// l'ente, per [ADR-036](../../docs/08-decisioni.md), non si proietta per
    /// identificatore a chi non vi appartiene.
    #[error("Membership not found")]
    Membership,
    #[error("Document not found")]
    Document,
    #[error("Ticket not found")]
    Ticket,
    #[error("Utility bill not found")]
    UtilityBill,
    #[error("Charge not found")]
    Charge,
    #[error("Payment not found")]
    Payment,
    #[error("Expense group not found")]
    ExpenseGroup,
    #[error("Expense not found")]
    Expense,
    #[error("Settlement not found")]
    Settlement,
    /// Rotta sconosciuta. Sta qui e non nel bordo HTTP perché un `404` di rotta e
    /// un `404` di risorsa devono avere la stessa forma per il client.
    #[error("Endpoint not found")]
    Endpoint,
}

impl Entity {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Property => "property.not_found",
            Self::Room => "room.not_found",
            Self::Lease => "lease.not_found",
            Self::LeaseParty => "lease.party_not_found",
            Self::Invitation => "invitation.not_found",
            Self::Membership => "membership.not_found",
            Self::Document => "document.not_found",
            Self::Ticket => "ticket.not_found",
            Self::UtilityBill => "utility_bill.not_found",
            Self::Charge => "charge.not_found",
            Self::Payment => "payment.not_found",
            Self::ExpenseGroup => "expense_group.not_found",
            Self::Expense => "expense.not_found",
            Self::Settlement => "settlement.not_found",
            Self::Endpoint => "endpoint.not_found",
        }
    }
}

/// Motivo per cui un'operazione è negata a chi la chiede.
///
/// **Non tutte queste varianti raggiungono il bordo HTTP.** Quelle su account,
/// sessione e invito vengono ripiegate su `auth.invalid_credentials` dai casi
/// d'uso di login, refresh e accettazione: distinguerle in risposta regalerebbe a
/// un attaccante un oracolo per sapere se un account esiste, se è bloccato o se un
/// token d'invito è solo scaduto. Il codice esiste comunque perché il tipo lo
/// richiede e perché nei log la distinzione serve; non compare in
/// `docs/05-api.md`, che documenta ciò che l'API può produrre davvero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Denial {
    #[error("Authentication is required")]
    AuthenticationRequired,
    #[error("No organization is selected")]
    OrgNotSelected,
    #[error("You do not belong to this organization")]
    NotAMember,
    #[error("The owner or manager role is required")]
    StaffRoleRequired,
    /// Chi ha segnalato può riaprire e annullare; chi condivide il contratto no.
    ///
    /// Non è una scelta di prodotto rivedibile nel codice: la `USING` di
    /// `tickets_update` ammette staff, `reported_by` e `assigned_to`, e non passa
    /// dal contratto. Esiste come variante propria perché
    /// `org.staff_role_required` direbbe al coinquilino che gli manca un ruolo,
    /// mentre gli manca l'essere stato lui a segnalare — due rimedi diversi.
    #[error("This ticket was reported by someone else")]
    TicketNotYours,
    /// Confermare o rifiutare un regolamento spetta a chi **riceve**.
    ///
    /// Non è `org.staff_role_required`: dire a un coinquilino che gli manca un
    /// ruolo lo manderebbe a cercare un permesso, mentre gli manca l'essere il
    /// destinatario del bonifico. È la stessa distinzione di
    /// [`Self::TicketNotYours`], e il rimedio è chiedere all'altro di confermare.
    #[error("This settlement is addressed to someone else")]
    SettlementNotAddressedToYou,
    /// Ritirare un regolamento spetta a chi lo ha **dichiarato**.
    ///
    /// Codice distinto dal precedente perché i due rimedi sono opposti: là si
    /// aspetta che il creditore confermi, qui si chiede al creditore di rifiutare.
    #[error("This settlement was declared by someone else")]
    SettlementNotDeclaredByYou,
    /// Uscire da un gruppo di spesa si fa per sé stessi.
    ///
    /// **La Row Level Security dice la stessa cosa**, dalla migration `0029`: la
    /// `WITH CHECK` di `expense_group_members_update` ammette la riga di un altro
    /// solo se il risultato lo lascia **dentro** il gruppo. Questo codice è quindi
    /// la versione parlante di un rifiuto che la policy darebbe comunque — e serve
    /// perché la policy da sola risponderebbe `auth.denied_by_policy`, che
    /// significa «difetto nostro».
    ///
    /// La ragione del confine è che un coinquilino non deve poter togliere
    /// unilateralmente un altro da un conto condiviso, tanto meno uno con un saldo
    /// aperto. Lo staff può, perché è chi amministra.
    #[error("Only staff can remove someone else from an expense group")]
    GroupMembershipNotYours,
    /// Una spesa la annulla chi l'ha registrata, chi l'ha anticipata, o lo staff.
    ///
    /// **Qui la Row Level Security è più larga**: la `USING` di `expenses_update`
    /// ammette qualunque membro attivo del gruppo — verificato eseguendo, e non
    /// «chi l'ha registrata o lo staff» come il piano della sezione 7 diceva.
    /// Questo è quindi un confine del **contratto dell'API** e non di sicurezza,
    /// come `LeaveGroup` prima della `0029`: un coinquilino non deve poter far
    /// sparire dai saldi una spesa che non ha né scritto né anticipato. Chi ha
    /// anticipato entra perché è il suo credito che la riga afferma, e chi l'ha
    /// registrata potrebbe essere uscito dal gruppo.
    #[error("Only whoever recorded or paid an expense, or staff, can cancel it")]
    ExpenseNotYours,

    // Da qui in giù: mai in risposta, solo nei log.
    #[error("The account is not active")]
    AccountInactive,
    #[error("The account is temporarily locked")]
    AccountLocked,
    #[error("The account has no local credentials")]
    NoLocalCredentials,
    #[error("The session was revoked")]
    SessionRevoked,
    #[error("The session has expired")]
    SessionExpired,
    #[error("The invitation was revoked")]
    InvitationRevoked,
    #[error("The invitation has expired")]
    InvitationExpired,
}

impl Denial {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::AuthenticationRequired => "auth.required",
            Self::OrgNotSelected => "org.not_selected",
            Self::NotAMember => "org.not_a_member",
            Self::StaffRoleRequired => "org.staff_role_required",
            Self::TicketNotYours => "ticket.not_yours",
            Self::SettlementNotAddressedToYou => "settlement.not_addressed_to_you",
            Self::SettlementNotDeclaredByYou => "settlement.not_declared_by_you",
            Self::GroupMembershipNotYours => "expense_group.membership_not_yours",
            Self::ExpenseNotYours => "expense.not_yours",
            Self::AccountInactive => "account.inactive",
            Self::AccountLocked => "account.locked",
            Self::NoLocalCredentials => "account.no_local_credentials",
            Self::SessionRevoked => "session.revoked",
            Self::SessionExpired => "session.expired",
            Self::InvitationRevoked => "invitation.revoked",
            Self::InvitationExpired => "invitation.expired",
        }
    }
}

/// Invariante di aggregato che l'operazione richiesta violerebbe.
///
/// Più varianti possono condividere un `code`: le tre transizioni di contratto
/// danno tutte `lease.invalid_transition`, e le due operazioni su un invito già
/// accettato danno `invitation.already_accepted`. È deliberato — il codice dice al
/// client **cosa fare**, e in quei casi il rimedio è lo stesso. Le varianti
/// restano distinte perché il messaggio che accompagna il codice non lo è.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Invariant {
    #[error("An archived property cannot be modified")]
    PropertyArchived,
    #[error("Archiving requires the dedicated operation, not a status update")]
    PropertyArchiveRequiresDedicatedOperation,
    /// Un immobile in bozza non si affitta.
    ///
    /// Il rifiuto è **all'attivazione** e non alla bozza del contratto: nel modello
    /// derivato «affittato» comincia con `app.lease_in_force` (ADR-019), e una
    /// bozza di contratto su una bozza di immobile sono due cose che si preparano
    /// insieme. Regola del service layer per la ragione di ADR-015: il legame è
    /// cross-row.
    #[error("A draft property cannot be let")]
    PropertyNotLettable,
    /// Un immobile con un contratto in essere non cambia natura.
    ///
    /// Vale per due operazioni — tornare in bozza e archiviare — ed è di
    /// proposito **un** invariante: è la stessa regola, e chi la incontra ha la
    /// stessa cosa da fare, cioè disdire il contratto. Archiviare **non** disdice:
    /// sarebbe un atto con una data e delle conseguenze nascosto dentro un soft
    /// delete, dove nessuno lo cerca.
    #[error("A property with a lease in force cannot be archived or moved back to draft")]
    PropertyHasLeaseInForce,
    #[error("An archived room cannot be modified")]
    RoomArchived,

    #[error("Only a draft or a lease awaiting signature can be activated")]
    LeaseNotActivatable,
    #[error("Only an active lease can be terminated")]
    LeaseNotTerminable,
    #[error("Only a draft or a lease awaiting signature can be cancelled")]
    LeaseNotCancellable,
    #[error("This lease party is already linked to another user")]
    LeasePartyAlreadyLinked,
    #[error("The email of a party already linked to a user cannot be changed")]
    LeasePartyEmailLocked,
    #[error("A party linked to a user cannot be removed, only marked as left")]
    LeasePartyHasUser,
    #[error("The lease has no tenant among its parties")]
    LeaseWithoutTenantParty,
    #[error("Tenant shares add up to {total} instead of 100")]
    LeaseSharesNotHundred { total: Decimal },
    #[error("A tenant with a zero share is a leftover row, not a tenant")]
    LeaseTenantWithoutShare,
    #[error("Parties can only be added while the lease is a draft or awaiting signature")]
    LeasePartiesLocked,

    #[error("The invitation has already been accepted")]
    InvitationAlreadyAccepted,
    #[error("An invitation that was already accepted cannot be revoked")]
    AcceptedInvitationCannotBeRevoked,
    /// Un invito da proprietario già accettato. Codice **proprio** e non
    /// `invitation.already_accepted`: sono due tabelle e due aggregati diversi
    /// (ADR-053), e il caso d'uso che lo produce lo appiattisce comunque su
    /// `401 auth.invalid_credentials` insieme a scaduto e revocato — la
    /// distinzione resta perché il tipo la richiede e perché nei log serve.
    #[error("The owner invitation has already been accepted")]
    OwnerInvitationAlreadyAccepted,

    #[error("The document is already archived")]
    DocumentArchived,

    #[error("Only an open ticket can be triaged")]
    TicketNotTriageable,
    #[error("A ticket can only be assigned while it is open, triaged or already assigned")]
    TicketNotAssignable,
    #[error("Work can only start on an assigned or suspended ticket")]
    TicketNotStartable,
    #[error("Only a ticket being worked on can be suspended")]
    TicketNotSuspendable,
    #[error("Only a ticket still being handled can be closed")]
    TicketNotClosable,
    #[error("Only a closed ticket can be reopened")]
    TicketNotReopenable,
    #[error("Only a ticket still being handled can be cancelled")]
    TicketNotCancellable,
    #[error("A cancelled ticket cannot be modified")]
    TicketCancelled,
    /// Il thread di una segnalazione terminata non riceve messaggi.
    ///
    /// Codice proprio e non `ticket.invalid_transition`, perché **non è una
    /// transizione rifiutata**: nessuno stava chiedendo di muovere la
    /// segnalazione. E non è `ticket.cancelled`, che vale solo per l'annullata,
    /// mentre qui i casi sono tre — `closed`, `cancelled` e `resolved`, che
    /// l'applicazione non scrive ma una riga può contenere.
    ///
    /// Il rimedio è preciso e diverso da entrambi: **riaprire**, che chiede il
    /// motivo e lo scrive come primo messaggio del giro nuovo (ADR-050).
    #[error("A ticket that is no longer being handled does not take messages")]
    TicketThreadClosed,

    #[error("Only a draft or a validated bill can be corrected")]
    UtilityBillNotEditable,
    /// Una lettura automatica si propone solo su una bozza.
    ///
    /// Fra l'accodamento e la fine del job una persona può aver validato,
    /// riaddebitato o annullato la bolletta: scriverci sopra la proposta di una
    /// macchina significherebbe riportare indietro qualcosa che qualcuno ha già
    /// deciso.
    #[error("Only a draft bill can take a machine reading")]
    UtilityBillNotReviewable,
    #[error("Only a draft bill can be validated")]
    UtilityBillNotValidatable,
    #[error("Only a validated or already allocated bill can be recharged")]
    UtilityBillNotAllocatable,
    /// In bozza o in revisione i numeri non li ha confermati nessuno: dichiarare
    /// pagato un importo che può ancora cambiare registra un fatto su un dato
    /// provvisorio.
    #[error("Only a validated or allocated bill can be declared paid")]
    UtilityBillNotPayable,
    /// Da `allocated` in poi la bolletta è la base di un addebito già emesso:
    /// farla sparire lascerebbe quell'addebito a citare una riga invisibile.
    #[error("A bill that has already been recharged cannot be voided")]
    UtilityBillNotVoidable,

    #[error("Only an issued charge can be voided")]
    ChargeNotVoidable,
    #[error("Only an issued charge can be written off")]
    ChargeNotWriteOffable,
    /// La pretesa è caduta: incassare su un addebito annullato o stornato è
    /// un'incoerenza, non un caso d'uso.
    #[error("A voided or written off charge cannot take a payment")]
    ChargeNotPayable,

    #[error("Only a pending payment can be confirmed")]
    PaymentNotConfirmable,
    #[error("Only a pending payment can be rejected")]
    PaymentNotRejectable,

    #[error("An archived expense group cannot be modified")]
    ExpenseGroupArchived,
    /// La spesa è già annullata. In pratica non si raggiunge dall'API — una
    /// spesa annullata non si legge più, e la seconda richiesta trova `404` —
    /// ma l'aggregato deve poterlo dire a chi lo costruisce a mano.
    #[error("The expense is already cancelled")]
    ExpenseCancelled,
    #[error("This person has already left the group")]
    GroupMemberAlreadyLeft,
    #[error("This person is already an active member of the group")]
    GroupMemberAlreadyIn,
    /// La spesa dichiara che ha anticipato qualcuno che nel gruppo non c'è.
    ///
    /// Ha una variante propria e non è una `Validation` perché non c'è nulla di
    /// malformato: l'identificatore è un utente vero, e a mancare è la relazione.
    /// Il danno che evita è silenzioso — `app.v_expense_group_balances` parte dai
    /// **membri**, quindi quel credito non comparirebbe a nessuno e la somma dei
    /// saldi del gruppo smetterebbe di fare zero.
    #[error("The person who paid is not a member of this group")]
    ExpensePayerNotInGroup,
    #[error("Only a pending settlement can be confirmed")]
    SettlementNotConfirmable,
    #[error("Only a pending settlement can be rejected")]
    SettlementNotRejectable,
    #[error("Only a pending settlement can be withdrawn")]
    SettlementNotCancellable,

    #[error("Only a queued job can be started")]
    AiJobNotStartable,
    #[error("Only a running job can be finished")]
    AiJobNotFinishable,

    /// Si è già usciti da quell'organizzazione.
    ///
    /// La riga resta — uscire è uno **stato** e non un soft delete — quindi il
    /// caso è osservabile e merita un messaggio invece di un secondo `200` muto.
    #[error("You have already left this organization")]
    MembershipAlreadyLeft,
    /// Non si esce da un'organizzazione in cui si ha un contratto in essere.
    ///
    /// **La Row Level Security dice la stessa cosa**, dalla migration `0039`: la
    /// `WITH CHECK` di `org_members_update_self` ammette l'uscita solo se
    /// `app.has_lease_in_force` è falsa. Questo codice è la versione parlante di un
    /// rifiuto che la policy darebbe comunque — e serve perché la policy da sola
    /// risponderebbe `auth.denied_by_policy`, che significa «difetto nostro». È la
    /// stessa forma di [`Denial::GroupMembershipNotYours`] sulle spese co-living.
    #[error("You are still party to a lease in force in this organization")]
    MembershipHasLeaseInForce,
    /// L'ultimo proprietario non se ne va.
    ///
    /// Lo schema dichiara «almeno un owner per org» come invariante **del service
    /// layer** — un trigger cross-row sarebbe workflow nel database — e questo è
    /// il posto in cui vive. Senza, uscire dalla propria organizzazione personale
    /// la lascerebbe senza nessuno che la amministri: gli immobili, i contratti e
    /// gli addebiti resterebbero in una riga che `organizations_select` non mostra
    /// più a nessuno, e non c'è nessuna operazione del prodotto per rientrarci.
    #[error("The last owner of an organization cannot leave it")]
    LastOwnerCannotLeave,

    #[error("Refresh token already rotated: possible reuse, revoke the chain")]
    RefreshTokenReused,
}

impl Invariant {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::PropertyArchived => "property.archived",
            Self::PropertyArchiveRequiresDedicatedOperation => {
                "property.archive_requires_dedicated_operation"
            }
            Self::PropertyNotLettable => "property.not_lettable",
            Self::PropertyHasLeaseInForce => "property.lease_in_force",
            Self::RoomArchived => "room.archived",
            Self::LeaseNotActivatable | Self::LeaseNotTerminable | Self::LeaseNotCancellable => {
                "lease.invalid_transition"
            }
            Self::LeasePartyAlreadyLinked => "lease.party_already_linked",
            Self::LeasePartyEmailLocked => "lease.party_email_locked",
            Self::LeasePartyHasUser => "lease.party_has_user",
            Self::LeaseWithoutTenantParty => "lease.no_tenant_party",
            Self::LeaseSharesNotHundred { .. } => "lease.shares_not_hundred",
            Self::LeaseTenantWithoutShare => "lease.tenant_without_share",
            Self::LeasePartiesLocked => "lease.parties_locked",
            Self::InvitationAlreadyAccepted | Self::AcceptedInvitationCannotBeRevoked => {
                "invitation.already_accepted"
            }
            Self::OwnerInvitationAlreadyAccepted => "owner_invitation.already_accepted",
            Self::DocumentArchived => "document.archived",
            // Sette varianti, un codice: il rimedio è lo stesso — rileggere lo
            // stato attuale e chiedere una transizione ammessa — e le tre
            // transizioni di contratto hanno già lo stesso trattamento. Restano
            // distinte perché il messaggio che accompagna il codice non lo è.
            Self::TicketNotTriageable
            | Self::TicketNotAssignable
            | Self::TicketNotStartable
            | Self::TicketNotSuspendable
            | Self::TicketNotClosable
            | Self::TicketNotReopenable
            | Self::TicketNotCancellable => "ticket.invalid_transition",
            Self::TicketCancelled => "ticket.cancelled",
            Self::TicketThreadClosed => "ticket.thread_closed",
            // Stesso trattamento delle transizioni di contratto e di
            // segnalazione: il rimedio è lo stesso — rileggere lo stato attuale e
            // chiedere una transizione ammessa — e i due aggregati restano
            // distinti perché il rimedio *non* è lo stesso fra loro.
            Self::UtilityBillNotEditable
            | Self::UtilityBillNotValidatable
            | Self::UtilityBillNotAllocatable
            | Self::UtilityBillNotPayable
            | Self::UtilityBillNotVoidable
            | Self::UtilityBillNotReviewable => "utility_bill.invalid_transition",
            Self::ChargeNotVoidable | Self::ChargeNotWriteOffable | Self::ChargeNotPayable => {
                "charge.invalid_transition"
            }
            // Stesso trattamento degli altri aggregati: il rimedio è rileggere lo
            // stato attuale e chiedere una transizione ammessa. Il codice è
            // **distinto** da `charge.invalid_transition` perché un pagamento che
            // non si può confermare e un addebito che non accetta incassi si
            // rimediano in due posti diversi.
            Self::PaymentNotConfirmable | Self::PaymentNotRejectable => {
                "payment.invalid_transition"
            }
            Self::ExpenseGroupArchived => "expense_group.archived",
            Self::ExpenseCancelled => "expense.cancelled",
            // Le due membership condividono un codice: il rimedio è lo stesso,
            // rileggere la composizione del gruppo. Sono varianti distinte perché
            // il messaggio non lo è — «è già uscito» e «c'è già» mandano la
            // persona a guardare due cose diverse.
            Self::GroupMemberAlreadyLeft | Self::GroupMemberAlreadyIn => {
                "expense_group.membership_unchanged"
            }
            Self::ExpensePayerNotInGroup => "expense.payer_not_in_group",
            // Stesso trattamento degli altri aggregati, e un codice **distinto**
            // da `payment.invalid_transition`: un regolamento fra coinquilini e un
            // pagamento verso il proprietario si rimediano in due posti diversi, e
            // per giunta qui le uscite di `pending` sono tre invece di due.
            Self::SettlementNotConfirmable
            | Self::SettlementNotRejectable
            | Self::SettlementNotCancellable => "settlement.invalid_transition",
            // Stesso trattamento degli altri aggregati. **Nessuno di questi due
            // codici raggiunge un client**: le transizioni del job le compie il
            // worker, che non risponde a nessuno — un `ai_job.invalid_transition`
            // che comparisse in una risposta HTTP sarebbe il segnale che qualcuno
            // ha esposto il ciclo del job come se fosse un comando.
            Self::AiJobNotStartable | Self::AiJobNotFinishable => "ai_job.invalid_transition",
            // Due codici distinti e non uno: «sei già uscito» e «sei l'ultimo
            // proprietario» mandano a fare due cose opposte — la prima non chiede
            // niente, la seconda chiede di nominare qualcun altro prima.
            Self::MembershipAlreadyLeft => "membership.already_left",
            Self::MembershipHasLeaseInForce => "membership.lease_in_force",
            Self::LastOwnerCannotLeave => "membership.last_owner",
            Self::RefreshTokenReused => "session.refresh_token_reused",
        }
    }
}

/// Errori che il dominio sa produrre. Non contengono mai dettagli di
/// infrastruttura: la traduzione verso HTTP avviene ai bordi.
///
/// I messaggi sono in inglese perché finiscono nel campo `detail` della risposta,
/// che è superficie pubblica. I commenti di questo crate restano in italiano.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// Un valore in ingresso non rispetta le regole del value object.
    ///
    /// Il `code` è sempre `validation.failed`: a discriminare è `field`, che il
    /// bordo HTTP riporta in `errors[]`. Un codice per campo moltiplicherebbe il
    /// contratto senza dire al client nulla che `field` non dica già.
    #[error("invalid field `{field}`: {message}")]
    Validation {
        field: &'static str,
        message: String,
    },

    /// Un'invariante dell'aggregato sarebbe violata dall'operazione richiesta.
    #[error("{0}")]
    Invariant(Invariant),

    /// L'entità richiesta non esiste (o non è visibile a chi la chiede: la RLS
    /// rende i due casi indistinguibili, ed è voluto — non si conferma
    /// l'esistenza di dati che non si possono vedere).
    #[error("{0}")]
    NotFound(Entity),

    /// L'attore non ha i permessi per l'operazione.
    #[error("{0}")]
    Forbidden(Denial),
}

impl DomainError {
    pub fn validation(field: &'static str, message: impl fmt::Display) -> Self {
        Self::Validation {
            field,
            message: message.to_string(),
        }
    }

    /// Codice stabile della condizione, parte del contratto pubblico (ADR-022).
    ///
    /// Nasce qui e non al bordo HTTP: il bordo lo trascrive, non lo inventa.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Validation { .. } => "validation.failed",
            Self::Invariant(invariant) => invariant.code(),
            Self::NotFound(entity) => entity.code(),
            Self::Forbidden(denial) => denial.code(),
        }
    }
}

pub type DomainResult<T> = Result<T, DomainError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Il formato `<dominio>.<condizione>` è il contratto: un codice che non lo
    /// rispetta è un codice che un client non può instradare.
    #[test]
    fn every_code_is_a_dotted_pair_of_snake_case_segments() {
        let codes: Vec<&'static str> = [
            Entity::Property.code(),
            Entity::Room.code(),
            Entity::Lease.code(),
            Entity::Invitation.code(),
            Entity::Document.code(),
            Entity::Ticket.code(),
            Entity::UtilityBill.code(),
            Entity::Charge.code(),
            Entity::Payment.code(),
            Entity::ExpenseGroup.code(),
            Entity::Expense.code(),
            Entity::Settlement.code(),
            Entity::Endpoint.code(),
            Denial::AuthenticationRequired.code(),
            Denial::OrgNotSelected.code(),
            Denial::NotAMember.code(),
            Denial::StaffRoleRequired.code(),
            Denial::TicketNotYours.code(),
            Denial::SettlementNotAddressedToYou.code(),
            Denial::SettlementNotDeclaredByYou.code(),
            Denial::GroupMembershipNotYours.code(),
            Denial::ExpenseNotYours.code(),
            Denial::AccountInactive.code(),
            Denial::AccountLocked.code(),
            Denial::NoLocalCredentials.code(),
            Denial::SessionRevoked.code(),
            Denial::SessionExpired.code(),
            Denial::InvitationRevoked.code(),
            Denial::InvitationExpired.code(),
            Invariant::PropertyArchived.code(),
            Invariant::PropertyArchiveRequiresDedicatedOperation.code(),
            Invariant::PropertyNotLettable.code(),
            Invariant::PropertyHasLeaseInForce.code(),
            Invariant::RoomArchived.code(),
            Invariant::LeaseNotActivatable.code(),
            Invariant::LeasePartyAlreadyLinked.code(),
            Invariant::LeaseWithoutTenantParty.code(),
            Invariant::LeaseSharesNotHundred {
                total: Decimal::from(90),
            }
            .code(),
            Invariant::InvitationAlreadyAccepted.code(),
            Invariant::OwnerInvitationAlreadyAccepted.code(),
            Invariant::DocumentArchived.code(),
            Invariant::TicketNotTriageable.code(),
            Invariant::TicketNotAssignable.code(),
            Invariant::TicketNotStartable.code(),
            Invariant::TicketNotSuspendable.code(),
            Invariant::TicketNotClosable.code(),
            Invariant::TicketNotReopenable.code(),
            Invariant::TicketNotCancellable.code(),
            Invariant::TicketCancelled.code(),
            Invariant::TicketThreadClosed.code(),
            Invariant::UtilityBillNotEditable.code(),
            Invariant::UtilityBillNotValidatable.code(),
            Invariant::UtilityBillNotAllocatable.code(),
            Invariant::UtilityBillNotVoidable.code(),
            Invariant::ChargeNotVoidable.code(),
            Invariant::ChargeNotWriteOffable.code(),
            Invariant::ChargeNotPayable.code(),
            Invariant::PaymentNotConfirmable.code(),
            Invariant::PaymentNotRejectable.code(),
            Invariant::ExpenseGroupArchived.code(),
            Invariant::ExpenseCancelled.code(),
            Invariant::GroupMemberAlreadyLeft.code(),
            Invariant::GroupMemberAlreadyIn.code(),
            Invariant::ExpensePayerNotInGroup.code(),
            Invariant::SettlementNotConfirmable.code(),
            Invariant::SettlementNotRejectable.code(),
            Invariant::SettlementNotCancellable.code(),
            Invariant::RefreshTokenReused.code(),
            DomainError::validation("field", "message").code(),
        ]
        .to_vec();

        for code in codes {
            let (domain, condition) = code
                .split_once('.')
                .unwrap_or_else(|| panic!("`{code}` non è nella forma <dominio>.<condizione>"));
            assert!(!domain.is_empty() && !condition.is_empty(), "`{code}`");
            assert!(
                code.chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c == '.'),
                "`{code}` deve essere snake_case minuscolo"
            );
            assert_eq!(condition.find('.'), None, "`{code}` ha più di due segmenti");
        }
    }

    /// Le tre transizioni di contratto condividono un codice perché il rimedio è
    /// lo stesso; se qualcuno le separasse, il test dice che è una scelta e non
    /// una svista.
    #[test]
    fn lease_transitions_share_one_code() {
        assert_eq!(
            Invariant::LeaseNotActivatable.code(),
            Invariant::LeaseNotTerminable.code()
        );
        assert_eq!(
            Invariant::LeaseNotActivatable.code(),
            Invariant::LeaseNotCancellable.code()
        );
        assert_eq!(
            Invariant::InvitationAlreadyAccepted.code(),
            Invariant::AcceptedInvitationCannotBeRevoked.code()
        );
    }

    /// L'invito da proprietario ha una tabella e un aggregato suoi: un codice
    /// condiviso con l'invito da inquilino direbbe che sono la stessa cosa.
    #[test]
    fn owner_invitation_already_accepted_has_its_own_code() {
        assert_ne!(
            Invariant::OwnerInvitationAlreadyAccepted.code(),
            Invariant::InvitationAlreadyAccepted.code()
        );
    }

    /// Le sette transizioni di segnalazione condividono un codice per la stessa
    /// ragione delle tre di contratto: il rimedio non cambia. `TicketCancelled`
    /// invece **non** lo condivide, perché non è una transizione rifiutata ma una
    /// modifica su un aggregato che non ne accetta più.
    #[test]
    fn ticket_transitions_share_one_code_but_cancellation_does_not() {
        let transitions = [
            Invariant::TicketNotTriageable,
            Invariant::TicketNotAssignable,
            Invariant::TicketNotStartable,
            Invariant::TicketNotSuspendable,
            Invariant::TicketNotClosable,
            Invariant::TicketNotReopenable,
            Invariant::TicketNotCancellable,
        ];
        for invariant in &transitions {
            assert_eq!(invariant.code(), "ticket.invalid_transition");
        }
        assert_ne!(
            Invariant::TicketCancelled.code(),
            Invariant::TicketNotClosable.code()
        );
        // Nemmeno il thread chiuso lo condivide, e per una ragione ulteriore: non
        // è una transizione rifiutata, perché nessuno stava chiedendo di muovere
        // la segnalazione. Il rimedio è riaprire, e va detto con un codice suo.
        assert_ne!(
            Invariant::TicketThreadClosed.code(),
            Invariant::TicketNotClosable.code()
        );
        assert_ne!(
            Invariant::TicketThreadClosed.code(),
            Invariant::TicketCancelled.code()
        );
    }

    /// I due aggregati contabili condividono un codice **ciascuno**, e non fra
    /// loro: da una bolletta che non si può correggere e da un addebito che non
    /// si può annullare non si esce nello stesso modo, e un client che instrada
    /// su questi codici deve poterli distinguere.
    #[test]
    fn billing_transitions_share_one_code_per_aggregate() {
        for invariant in [
            Invariant::UtilityBillNotEditable,
            Invariant::UtilityBillNotValidatable,
            Invariant::UtilityBillNotAllocatable,
            Invariant::UtilityBillNotVoidable,
        ] {
            assert_eq!(invariant.code(), "utility_bill.invalid_transition");
        }
        for invariant in [
            Invariant::ChargeNotVoidable,
            Invariant::ChargeNotWriteOffable,
            Invariant::ChargeNotPayable,
        ] {
            assert_eq!(invariant.code(), "charge.invalid_transition");
        }
        assert_ne!(
            Invariant::UtilityBillNotVoidable.code(),
            Invariant::ChargeNotVoidable.code()
        );
    }

    /// Le tre transizioni del regolamento condividono un codice fra loro e **non**
    /// con il pagamento, per quanto le due macchine si assomiglino: un regolamento
    /// fra coinquilini e un pagamento verso il proprietario si rimediano in due
    /// posti diversi, e qui le uscite di `pending` sono **tre** invece di due —
    /// perché `settlements_update` ammette anche `from_user_id`, mentre
    /// `payments_update` è staff soltanto.
    #[test]
    fn settlement_transitions_have_their_own_code() {
        for invariant in [
            Invariant::SettlementNotConfirmable,
            Invariant::SettlementNotRejectable,
            Invariant::SettlementNotCancellable,
        ] {
            assert_eq!(invariant.code(), "settlement.invalid_transition");
        }
        assert_ne!(
            Invariant::SettlementNotConfirmable.code(),
            Invariant::PaymentNotConfirmable.code()
        );
    }

    /// Le due transizioni del pagamento condividono un codice fra loro e **non**
    /// con l'addebito: un pagamento che non si può confermare e un addebito che
    /// non accetta incassi si rimediano in due posti diversi, e un client che
    /// instrada su questi codici deve poterli distinguere.
    #[test]
    fn payment_transitions_have_their_own_code() {
        for invariant in [
            Invariant::PaymentNotConfirmable,
            Invariant::PaymentNotRejectable,
        ] {
            assert_eq!(invariant.code(), "payment.invalid_transition");
        }
        assert_ne!(
            Invariant::PaymentNotConfirmable.code(),
            Invariant::ChargeNotPayable.code()
        );
    }
}
