use chrono::{DateTime, Days, NaiveDate, Utc};

use super::party::{LeaseParty, LeasePartyRole};
use crate::error::{DomainError, DomainResult, Invariant};
use crate::shared::{Currency, LeaseId, Money, OrgId, PropertyId, RoomId, text};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseType {
    WholeProperty,
    Room,
}

impl LeaseType {
    /// Nome stabile del tipo, per gli eventi di dominio. Non è la mappatura verso
    /// il database — quella vive nella tabella di conversione
    /// dell'infrastruttura — ma il valore che viaggia in un payload pubblicato.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WholeProperty => "whole_property",
            Self::Room => "room",
        }
    }
}

/// Stato di un contratto.
///
/// `Expiring` ed `Expired` esistono nell'enum del database ma **non vengono mai
/// scritti dall'applicazione**: sono il risultato di [`Lease::effective_status`],
/// che li calcola dalle date. Restano fra le varianti perché una riga letta dalla
/// persistenza può contenerli, e un valore sconosciuto in lettura deve dare errore
/// invece di ricadere su un default plausibile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseStatus {
    Draft,
    PendingSignature,
    Active,
    Expiring,
    Expired,
    Terminated,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RentFrequency {
    Monthly,
    Quarterly,
    Semiannual,
    Annual,
}

/// Dati necessari a redigere un contratto.
///
/// Una struct e non quindici argomenti posizionali: fra `Option<i16>` e
/// `Option<Money>` dello stesso tipo, scambiarne due compilerebbe senza errori.
#[derive(Debug, Clone)]
pub struct LeaseDraft {
    pub id: LeaseId,
    pub org_id: OrgId,
    pub property_id: PropertyId,
    /// `Some` per un contratto per stanza, `None` per l'immobile intero. Da qui si
    /// deriva `lease_type`, invece di farli dichiarare entrambi: il `CHECK`
    /// `leases_target_ck` impone la coerenza, e due campi indipendenti sarebbero
    /// solo un modo di violarla.
    pub room_id: Option<RoomId>,
    pub start_date: NaiveDate,
    /// `None` = a tempo indeterminato. Esclusiva: il contratto copre
    /// `[start_date, end_date)`, come i vincoli di non sovrapposizione.
    pub end_date: Option<NaiveDate>,
    pub rent: Money,
    pub deposit: Option<Money>,
    pub rent_frequency: RentFrequency,
    pub rent_due_day: i16,
    pub notice_months: Option<i16>,
    pub auto_renew: bool,
    pub renewal_months: Option<i16>,
    pub utilities_included: bool,
    pub istat_indexation: bool,
    pub notes: Option<String>,
}

/// Contratto di locazione.
#[derive(Debug, Clone)]
pub struct Lease {
    id: LeaseId,
    org_id: OrgId,
    property_id: PropertyId,
    room_id: Option<RoomId>,
    lease_type: LeaseType,
    status: LeaseStatus,
    start_date: NaiveDate,
    end_date: Option<NaiveDate>,
    notice_months: Option<i16>,
    auto_renew: bool,
    renewal_months: Option<i16>,
    rent: Money,
    rent_frequency: RentFrequency,
    rent_due_day: i16,
    deposit: Money,
    utilities_included: bool,
    istat_indexation: bool,
    signed_at: Option<DateTime<Utc>>,
    terminated_at: Option<DateTime<Utc>>,
    termination_reason: Option<String>,
    notes: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
}

impl Lease {
    /// Finestra di preavviso oltre la quale un contratto è "in scadenza".
    /// Coincide con quella della dashboard, che conta i contratti a 90 giorni.
    pub const EXPIRING_WINDOW_DAYS: u64 = 90;

    /// Un contratto nasce `Draft`: non impegna l'immobile e non compare fra
    /// quelli in essere finché non viene attivato.
    pub fn draft(draft: LeaseDraft, now: DateTime<Utc>) -> DomainResult<Self> {
        let LeaseDraft {
            id,
            org_id,
            property_id,
            room_id,
            start_date,
            end_date,
            rent,
            deposit,
            rent_frequency,
            rent_due_day,
            notice_months,
            auto_renew,
            renewal_months,
            utilities_included,
            istat_indexation,
            notes,
        } = draft;

        if let Some(end) = end_date
            && end <= start_date
        {
            return Err(DomainError::validation(
                "end_date",
                "must be later than the start date",
            ));
        }
        if !(1..=28).contains(&rent_due_day) {
            // 28 e non 31: il giorno 30 non esiste a febbraio, e un canone la cui
            // scadenza slitta a seconda del mese non è rappresentabile.
            return Err(DomainError::validation(
                "rent_due_day",
                "must be between 1 and 28",
            ));
        }
        if let Some(months) = notice_months
            && !(0..=24).contains(&months)
        {
            return Err(DomainError::validation(
                "notice_months",
                "must be between 0 and 24",
            ));
        }
        if let Some(months) = renewal_months
            && months <= 0
        {
            return Err(DomainError::validation(
                "renewal_months",
                "must be positive",
            ));
        }

        let rent = Money::non_negative(rent.amount(), rent.currency())?;
        let deposit = match deposit {
            None => Money::non_negative(rust_decimal::Decimal::ZERO, rent.currency())?,
            Some(deposit) => {
                // Il contratto ha una sola colonna `currency`: un deposito in una
                // valuta diversa dal canone non sarebbe rappresentabile, e
                // scriverlo lo convertirebbe implicitamente.
                if deposit.currency() != rent.currency() {
                    return Err(DomainError::validation(
                        "deposit_amount",
                        "must be in the same currency as the rent",
                    ));
                }
                Money::non_negative(deposit.amount(), deposit.currency())?
            }
        };

        Ok(Self {
            id,
            org_id,
            property_id,
            room_id,
            lease_type: if room_id.is_some() {
                LeaseType::Room
            } else {
                LeaseType::WholeProperty
            },
            status: LeaseStatus::Draft,
            start_date,
            end_date,
            notice_months,
            auto_renew,
            renewal_months,
            rent,
            rent_frequency,
            rent_due_day,
            deposit,
            utilities_included,
            istat_indexation,
            signed_at: None,
            terminated_at: None,
            termination_reason: None,
            notes: text::optional("notes", notes, 4000)?,
            created_at: now,
            updated_at: now,
            archived_at: None,
        })
    }

    /// Le parti si aggiungono finché il contratto non è in vigore.
    ///
    /// Senza questo controllo si poteva aggiungere una parte a un contratto
    /// **attivo**, e da lì le quote non sommavano più a 100 senza che niente
    /// protestasse: l'invariante delle quote è verificata *al momento*
    /// dell'attivazione, e nessuno la riverifica dopo. Il caso legittimo dietro —
    /// un coinquilino che subentra a metà contratto — non è un'aggiunta: è un
    /// cambio di composizione che deve tenere la somma a 100, cioè un'operazione
    /// su più righe insieme che non è progettata. Vedi ADR-046.
    pub fn ensure_accepts_parties(&self) -> DomainResult<()> {
        if !matches!(
            self.status,
            LeaseStatus::Draft | LeaseStatus::PendingSignature
        ) {
            return Err(DomainError::Invariant(Invariant::LeasePartiesLocked));
        }
        Ok(())
    }

    /// Attiva il contratto: da qui impegna l'immobile o la stanza, e il vincolo di
    /// esclusione del database rifiuta un periodo già occupato.
    ///
    /// Le parti si passano perché le due invarianti che contano sono su di loro, e
    /// un contratto attivo senza inquilino o con quote incoerenti sarebbe
    /// impossibile da addebitare.
    pub fn activate(&mut self, parties: &[LeaseParty], now: DateTime<Utc>) -> DomainResult<()> {
        if !matches!(
            self.status,
            LeaseStatus::Draft | LeaseStatus::PendingSignature
        ) {
            return Err(DomainError::Invariant(Invariant::LeaseNotActivatable));
        }
        LeaseParty::ensure_tenancy_complete(parties)?;

        self.status = LeaseStatus::Active;
        self.signed_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    /// Disdetta anticipata. `terminated_at` è obbligatorio anche a schema
    /// (`leases_terminated_ck`): uno stato "terminato" senza la data del fatto non
    /// direbbe quando è finito il rapporto.
    pub fn terminate(&mut self, reason: Option<String>, now: DateTime<Utc>) -> DomainResult<()> {
        if !matches!(self.status, LeaseStatus::Active) {
            return Err(DomainError::Invariant(Invariant::LeaseNotTerminable));
        }
        self.status = LeaseStatus::Terminated;
        self.terminated_at = Some(now);
        self.termination_reason = text::optional("termination_reason", reason, 1000)?;
        self.updated_at = now;
        Ok(())
    }

    /// Annulla un contratto mai entrato in vigore. Diverso dalla disdetta: qui non
    /// c'è stato alcun rapporto, quindi non c'è una data di fine da registrare.
    pub fn cancel(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if !matches!(
            self.status,
            LeaseStatus::Draft | LeaseStatus::PendingSignature
        ) {
            return Err(DomainError::Invariant(Invariant::LeaseNotCancellable));
        }
        self.status = LeaseStatus::Cancelled;
        self.updated_at = now;
        Ok(())
    }

    /// Un subentro si fa solo su un contratto **in essere oggi** (ADR-056).
    ///
    /// Non basta lo stato `Active`: uno scaduto resta `active` in memoria perché
    /// nessuno scheduler lo cambia (ADR-019), e uno firmato che comincia fra un mese
    /// non ha ancora nessuno che ne esca. In bozza e in firma le parti si
    /// aggiungono, e il subentro non serve.
    pub fn ensure_open_to_handover(&self, today: NaiveDate) -> DomainResult<()> {
        if !self.is_in_force(today) {
            return Err(DomainError::Invariant(Invariant::LeaseNotHandoverable));
        }
        Ok(())
    }

    /// `true` se il contratto è attivo e `today` cade in `[start_date, end_date)`.
    ///
    /// Stessa definizione della funzione `app.lease_in_force`, deliberatamente:
    /// il dominio e le viste devono rispondere la stessa cosa, altrimenti la
    /// dashboard e la scheda del contratto si contraddicono.
    #[must_use]
    pub fn is_in_force(&self, today: NaiveDate) -> bool {
        matches!(self.status, LeaseStatus::Active)
            && self.start_date <= today
            && self.end_date.is_none_or(|end| end > today)
    }

    /// `true` se il contratto è in essere e finisce entro `days` giorni.
    #[must_use]
    pub fn expires_within(&self, days: u64, today: NaiveDate) -> bool {
        let Some(end) = self.end_date else {
            return false;
        };
        let Some(horizon) = today.checked_add_days(Days::new(days)) else {
            return false;
        };
        self.is_in_force(today) && end <= horizon
    }

    /// Stato effettivo alla data indicata.
    ///
    /// Lo stato memorizzato cambia solo su azioni esplicite; scadenza e imminenza
    /// sono fatti del calendario e si calcolano. Nessuno scheduler li scrive, così
    /// non esiste la finestra in cui il database dice una cosa e il calendario
    /// un'altra (ADR-019).
    #[must_use]
    pub fn effective_status(&self, today: NaiveDate) -> LeaseStatus {
        if !matches!(self.status, LeaseStatus::Active) {
            return self.status;
        }
        match self.end_date {
            Some(end) if end <= today => LeaseStatus::Expired,
            _ if self.expires_within(Self::EXPIRING_WINDOW_DAYS, today) => LeaseStatus::Expiring,
            _ => self.status,
        }
    }

    #[must_use]
    pub const fn id(&self) -> LeaseId {
        self.id
    }
    #[must_use]
    pub const fn org_id(&self) -> OrgId {
        self.org_id
    }
    #[must_use]
    pub const fn property_id(&self) -> PropertyId {
        self.property_id
    }
    #[must_use]
    pub const fn room_id(&self) -> Option<RoomId> {
        self.room_id
    }
    #[must_use]
    pub const fn lease_type(&self) -> LeaseType {
        self.lease_type
    }
    #[must_use]
    pub const fn status(&self) -> LeaseStatus {
        self.status
    }
    #[must_use]
    pub const fn start_date(&self) -> NaiveDate {
        self.start_date
    }
    #[must_use]
    pub const fn end_date(&self) -> Option<NaiveDate> {
        self.end_date
    }
    #[must_use]
    pub const fn notice_months(&self) -> Option<i16> {
        self.notice_months
    }
    #[must_use]
    pub const fn auto_renew(&self) -> bool {
        self.auto_renew
    }
    #[must_use]
    pub const fn renewal_months(&self) -> Option<i16> {
        self.renewal_months
    }
    #[must_use]
    pub const fn rent(&self) -> Money {
        self.rent
    }
    #[must_use]
    pub const fn rent_frequency(&self) -> RentFrequency {
        self.rent_frequency
    }
    #[must_use]
    pub const fn rent_due_day(&self) -> i16 {
        self.rent_due_day
    }
    #[must_use]
    pub const fn deposit(&self) -> Money {
        self.deposit
    }
    #[must_use]
    pub const fn currency(&self) -> Currency {
        self.rent.currency()
    }
    #[must_use]
    pub const fn utilities_included(&self) -> bool {
        self.utilities_included
    }
    #[must_use]
    pub const fn istat_indexation(&self) -> bool {
        self.istat_indexation
    }
    #[must_use]
    pub const fn signed_at(&self) -> Option<DateTime<Utc>> {
        self.signed_at
    }
    #[must_use]
    pub const fn terminated_at(&self) -> Option<DateTime<Utc>> {
        self.terminated_at
    }
    #[must_use]
    pub fn termination_reason(&self) -> Option<&str> {
        self.termination_reason.as_deref()
    }
    #[must_use]
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
    #[must_use]
    pub const fn archived_at(&self) -> Option<DateTime<Utc>> {
        self.archived_at
    }

    /// Ricostruzione da persistenza: non riesegue le invarianti.
    #[must_use]
    pub fn from_persistence(parts: LeaseParts) -> Self {
        let LeaseParts {
            id,
            org_id,
            property_id,
            room_id,
            lease_type,
            status,
            start_date,
            end_date,
            notice_months,
            auto_renew,
            renewal_months,
            rent,
            rent_frequency,
            rent_due_day,
            deposit,
            utilities_included,
            istat_indexation,
            signed_at,
            terminated_at,
            termination_reason,
            notes,
            created_at,
            updated_at,
            archived_at,
        } = parts;
        Self {
            id,
            org_id,
            property_id,
            room_id,
            lease_type,
            status,
            start_date,
            end_date,
            notice_months,
            auto_renew,
            renewal_months,
            rent,
            rent_frequency,
            rent_due_day,
            deposit,
            utilities_included,
            istat_indexation,
            signed_at,
            terminated_at,
            termination_reason,
            notes,
            created_at,
            updated_at,
            archived_at,
        }
    }
}

/// Dati grezzi di un contratto letti dalla persistenza.
#[derive(Debug, Clone)]
pub struct LeaseParts {
    pub id: LeaseId,
    pub org_id: OrgId,
    pub property_id: PropertyId,
    pub room_id: Option<RoomId>,
    pub lease_type: LeaseType,
    pub status: LeaseStatus,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub notice_months: Option<i16>,
    pub auto_renew: bool,
    pub renewal_months: Option<i16>,
    pub rent: Money,
    pub rent_frequency: RentFrequency,
    pub rent_due_day: i16,
    pub deposit: Money,
    pub utilities_included: bool,
    pub istat_indexation: bool,
    pub signed_at: Option<DateTime<Utc>>,
    pub terminated_at: Option<DateTime<Utc>>,
    pub termination_reason: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

impl LeaseStatus {
    /// Uno stato derivato non è memorizzabile: serve alla lettura e alla risposta
    /// HTTP, non alla scrittura.
    #[must_use]
    pub const fn is_derived(self) -> bool {
        matches!(self, Self::Expiring | Self::Expired)
    }
}

impl LeasePartyRole {
    /// Le quote di ripartizione riguardano chi abita, non chi garantisce.
    #[must_use]
    pub const fn shares_rent(self) -> bool {
        matches!(self, Self::Tenant | Self::Cotenant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lease::NewLeaseParty;
    use crate::shared::{LeasePartyId, UserId};
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn date(raw: &str) -> NaiveDate {
        NaiveDate::from_str(raw).expect("data di test valida")
    }

    fn money(raw: &str) -> Money {
        Money::new(
            Decimal::from_str(raw).expect("decimale valido"),
            Currency::EUR,
        )
        .expect("importo valido")
    }

    fn draft(end: Option<&str>) -> LeaseDraft {
        LeaseDraft {
            id: LeaseId::generate(),
            org_id: OrgId::generate(),
            property_id: PropertyId::generate(),
            room_id: None,
            start_date: date("2026-01-01"),
            end_date: end.map(date),
            rent: money("800.00"),
            deposit: Some(money("1600.00")),
            rent_frequency: RentFrequency::Monthly,
            rent_due_day: 5,
            notice_months: Some(6),
            auto_renew: false,
            renewal_months: None,
            utilities_included: false,
            istat_indexation: true,
            notes: None,
        }
    }

    fn lease(end: Option<&str>) -> Lease {
        Lease::draft(draft(end), Utc::now()).expect("contratto valido")
    }

    fn tenant() -> LeaseParty {
        LeaseParty::new(
            NewLeaseParty {
                id: LeasePartyId::generate(),
                org_id: OrgId::generate(),
                lease_id: LeaseId::generate(),
                role: LeasePartyRole::Tenant,
                full_name: "Mario Rossi".to_owned(),
                email: None,
                share_percent: Decimal::from(100),
                is_primary: true,
            },
            Utc::now(),
        )
        .expect("parte valida")
    }

    #[test]
    fn a_new_lease_is_a_draft_without_signature() {
        let lease = lease(Some("2030-01-01"));
        assert_eq!(lease.status(), LeaseStatus::Draft);
        assert_eq!(lease.signed_at(), None);
        assert!(!lease.is_in_force(date("2026-06-01")));
    }

    #[test]
    fn lease_type_follows_the_presence_of_a_room() {
        assert_eq!(lease(None).lease_type(), LeaseType::WholeProperty);
        let mut input = draft(None);
        input.room_id = Some(RoomId::generate());
        let per_room = Lease::draft(input, Utc::now()).expect("valido");
        assert_eq!(per_room.lease_type(), LeaseType::Room);
    }

    #[test]
    fn the_period_must_be_non_empty() {
        let mut input = draft(None);
        input.end_date = Some(input.start_date);
        assert!(Lease::draft(input, Utc::now()).is_err());
    }

    #[test]
    fn rent_due_day_stays_within_the_shortest_month() {
        for bad in [0_i16, 29, 31] {
            let mut input = draft(None);
            input.rent_due_day = bad;
            assert!(
                Lease::draft(input, Utc::now()).is_err(),
                "{bad} doveva essere rifiutato"
            );
        }
    }

    #[test]
    fn deposit_must_share_the_currency_of_the_rent() {
        let mut input = draft(None);
        input.deposit = Some(
            Money::new(
                Decimal::from(100),
                Currency::parse("USD").expect("valuta valida"),
            )
            .expect("importo valido"),
        );
        assert!(Lease::draft(input, Utc::now()).is_err());
    }

    #[test]
    fn an_absent_deposit_becomes_zero_in_the_rent_currency() {
        let mut input = draft(None);
        input.deposit = None;
        let lease = Lease::draft(input, Utc::now()).expect("valido");
        assert_eq!(lease.deposit().amount(), Decimal::ZERO);
        assert_eq!(lease.deposit().currency(), lease.rent().currency());
    }

    #[test]
    fn activation_requires_at_least_one_tenant() {
        let mut lease = lease(Some("2030-01-01"));
        let error = lease
            .activate(&[], Utc::now())
            .expect_err("senza parti non si attiva");
        assert!(matches!(error, DomainError::Invariant(_)));

        lease
            .activate(&[tenant()], Utc::now())
            .expect("con un inquilino si attiva");
        assert_eq!(lease.status(), LeaseStatus::Active);
        assert!(lease.signed_at().is_some());
    }

    #[test]
    fn a_lease_cannot_be_activated_twice() {
        let mut lease = lease(None);
        lease.activate(&[tenant()], Utc::now()).expect("attivato");
        assert!(lease.activate(&[tenant()], Utc::now()).is_err());
    }

    #[test]
    fn only_an_active_lease_can_be_terminated() {
        let mut lease = lease(None);
        assert!(lease.terminate(None, Utc::now()).is_err());

        lease.activate(&[tenant()], Utc::now()).expect("attivato");
        lease
            .terminate(Some("trasferimento".to_owned()), Utc::now())
            .expect("disdetta");
        assert_eq!(lease.status(), LeaseStatus::Terminated);
        assert!(lease.terminated_at().is_some());
        assert_eq!(lease.termination_reason(), Some("trasferimento"));
    }

    #[test]
    fn only_a_draft_can_be_cancelled() {
        let mut lease = lease(None);
        lease.cancel(Utc::now()).expect("annullabile");
        assert_eq!(lease.status(), LeaseStatus::Cancelled);
        assert!(lease.cancel(Utc::now()).is_err());
    }

    #[test]
    fn in_force_uses_a_half_open_period() {
        let mut lease = lease(Some("2027-01-01"));
        lease.activate(&[tenant()], Utc::now()).expect("attivato");

        assert!(!lease.is_in_force(date("2025-12-31")));
        assert!(lease.is_in_force(date("2026-01-01")));
        assert!(lease.is_in_force(date("2026-12-31")));
        // end_date è esclusiva, come nel vincolo di non sovrapposizione: il giorno
        // della fine il posto è già libero per il contratto successivo.
        assert!(!lease.is_in_force(date("2027-01-01")));
    }

    #[test]
    fn an_open_ended_lease_never_expires() {
        let mut lease = lease(None);
        lease.activate(&[tenant()], Utc::now()).expect("attivato");
        assert!(lease.is_in_force(date("2099-01-01")));
        assert!(!lease.expires_within(90, date("2099-01-01")));
        assert_eq!(
            lease.effective_status(date("2099-01-01")),
            LeaseStatus::Active
        );
    }

    #[test]
    fn effective_status_derives_expiring_and_expired_without_touching_the_stored_state() {
        let mut lease = lease(Some("2027-01-01"));
        lease.activate(&[tenant()], Utc::now()).expect("attivato");

        assert_eq!(
            lease.effective_status(date("2026-06-01")),
            LeaseStatus::Active
        );
        assert_eq!(
            lease.effective_status(date("2026-11-15")),
            LeaseStatus::Expiring,
            "a 47 giorni dalla fine il contratto è in scadenza"
        );
        assert_eq!(
            lease.effective_status(date("2027-06-01")),
            LeaseStatus::Expired
        );

        // Lo stato memorizzato non è cambiato: è il punto della decisione.
        assert_eq!(lease.status(), LeaseStatus::Active);
    }

    #[test]
    fn a_terminated_lease_keeps_its_stored_state_as_effective() {
        let mut lease = lease(Some("2027-01-01"));
        lease.activate(&[tenant()], Utc::now()).expect("attivato");
        lease.terminate(None, Utc::now()).expect("disdetta");
        assert_eq!(
            lease.effective_status(date("2099-01-01")),
            LeaseStatus::Terminated,
            "una disdetta non diventa una scadenza col passare del tempo"
        );
    }

    #[test]
    fn derived_states_are_recognizable() {
        assert!(LeaseStatus::Expiring.is_derived());
        assert!(LeaseStatus::Expired.is_derived());
        assert!(!LeaseStatus::Active.is_derived());
    }

    #[test]
    fn only_tenants_and_cotenants_share_the_rent() {
        assert!(LeasePartyRole::Tenant.shares_rent());
        assert!(LeasePartyRole::Cotenant.shares_rent());
        assert!(!LeasePartyRole::Guarantor.shares_rent());
    }

    #[test]
    fn the_domain_does_not_care_which_user_signed() {
        // Una parte può esistere senza utente: l'inquilino invitato non si è
        // ancora registrato, e il contratto è già redigibile.
        let party = tenant();
        assert_eq!(party.user_id(), None::<UserId>);
    }
}
