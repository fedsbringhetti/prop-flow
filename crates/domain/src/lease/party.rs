use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;

use crate::error::{DomainError, DomainResult, Invariant};
use crate::property::Patch;
use crate::shared::{Email, LeaseId, LeasePartyId, OrgId, UserId, text};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeasePartyRole {
    Tenant,
    Cotenant,
    Guarantor,
}

impl LeasePartyRole {
    /// Nome stabile del ruolo, per gli eventi di dominio.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tenant => "tenant",
            Self::Cotenant => "cotenant",
            Self::Guarantor => "guarantor",
        }
    }
}

/// Parte di un contratto: inquilino, coinquilino o garante.
///
/// `user_id` è opzionale, e non per comodità: un proprietario redige il contratto
/// prima che l'inquilino abbia un account. La parte esiste con nome ed email, e
/// l'utente viene collegato quando accetta l'invito.
#[derive(Debug, Clone)]
pub struct LeaseParty {
    id: LeasePartyId,
    org_id: OrgId,
    lease_id: LeaseId,
    user_id: Option<UserId>,
    role: LeasePartyRole,
    full_name: String,
    email: Option<Email>,
    phone: Option<String>,
    share_percent: Decimal,
    is_primary: bool,
    joined_at: Option<NaiveDate>,
    left_at: Option<NaiveDate>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
}

/// Dati di una parte contrattuale nuova.
///
/// Una struct e non otto argomenti posizionali: fra `LeasePartyId`, `OrgId` e
/// `LeaseId` — tre identificatori accanto a un booleano e una percentuale —
/// scambiarne due non darebbe errore di compilazione se fossero posizionali dello
/// stesso tipo, e il tipo non basta a distinguere `is_primary` da un altro flag
/// che potrebbe arrivare domani.
#[derive(Debug, Clone)]
pub struct NewLeaseParty {
    pub id: LeasePartyId,
    pub org_id: OrgId,
    pub lease_id: LeaseId,
    pub role: LeasePartyRole,
    pub full_name: String,
    pub email: Option<Email>,
    pub share_percent: Decimal,
    pub is_primary: bool,
}

/// Le correzioni ammesse su una parte già inserita.
///
/// `None` = invariato, `Some(None)` = azzerato, `Some(Some(v))` = impostato: è la
/// semantica `Patch` del resto del progetto, e serve perché svuotare un telefono
/// inserito per sbaglio dev'essere possibile.
///
/// **`user_id` non c'è, e non è una dimenticanza**: collegare un utente è
/// `link_user`, che ha una regola sua — non si sovrascrive un collegamento — e
/// passa dall'accettazione di un invito. Una correzione amministrativa non deve
/// poter trasferire un contratto a un'altra persona.
#[derive(Debug, Clone, Default)]
pub struct LeasePartyChanges {
    pub full_name: Option<String>,
    pub email: Patch<Email>,
    pub phone: Patch<String>,
    pub role: Option<LeasePartyRole>,
    pub share_percent: Option<Decimal>,
    pub is_primary: Option<bool>,
}

impl LeaseParty {
    pub fn new(party: NewLeaseParty, now: DateTime<Utc>) -> DomainResult<Self> {
        let NewLeaseParty {
            id,
            org_id,
            lease_id,
            role,
            full_name,
            email,
            share_percent,
            is_primary,
        } = party;
        Ok(Self {
            id,
            org_id,
            lease_id,
            user_id: None,
            role,
            full_name: text::required("full_name", &full_name, 200)?,
            email,
            phone: None,
            share_percent: validate_share(share_percent)?,
            is_primary,
            joined_at: None,
            left_at: None,
            created_at: now,
            updated_at: now,
            archived_at: None,
        })
    }

    /// Collega un utente registrato a questa parte. È il passo che trasforma un
    /// nome sul contratto in un inquilino con accesso: da qui in poi la Row Level
    /// Security gli mostra il contratto e l'immobile.
    ///
    /// Rifiuta se un utente è già collegato: sovrascriverlo significherebbe
    /// trasferire il contratto a un'altra persona senza traccia dell'operazione.
    pub fn link_user(&mut self, user_id: UserId, now: DateTime<Utc>) -> DomainResult<()> {
        if let Some(existing) = self.user_id {
            if existing == user_id {
                return Ok(());
            }
            return Err(DomainError::Invariant(Invariant::LeasePartyAlreadyLinked));
        }
        self.user_id = Some(user_id);
        self.joined_at = Some(now.date_naive());
        self.updated_at = now;
        Ok(())
    }

    /// Corregge una parte inserita male.
    ///
    /// # Perché esiste
    ///
    /// Senza, una quota sbagliata era definitiva: il contratto non si attivava
    /// (`ensure_tenancy_complete` rifiuta), e non si poteva nemmeno rifare, perché
    /// annullare non era esposto. `remaining_share` dell'invito porta scritto da
    /// sempre che una quota a zero «chi amministra la correggerà» — questa è
    /// l'operazione che quel commento presupponeva e che non esisteva.
    ///
    /// # L'email si tocca solo finché la parte non ha un utente
    ///
    /// L'email è ciò a cui l'invito è intestato e ciò che l'accettazione confronta:
    /// finché nessuno l'ha usata, correggerla è rimediare a un refuso. Dopo, la
    /// stessa modifica sposterebbe un rapporto già stabilito sotto i piedi di chi
    /// l'ha accettato, e chi legge la riga non vedrebbe che è successo.
    ///
    /// # La somma delle quote **non** si ricontrolla qui
    ///
    /// È deliberato, e senza sarebbe impossibile rimediare: per portare due parti
    /// da `100 / 0` a `50 / 50` bisogna passare da uno stato intermedio in cui la
    /// somma non fa 100. L'invariante vive dove è sempre vissuto — l'attivazione —
    /// e su un contratto già attivo, che è il caso dei contratti rotti che esistono
    /// oggi, la responsabilità è di chi corregge.
    pub fn correct(
        &mut self,
        changes: LeasePartyChanges,
        now: DateTime<Utc>,
    ) -> DomainResult<bool> {
        let LeasePartyChanges {
            full_name,
            email,
            phone,
            role,
            share_percent,
            is_primary,
        } = changes;

        if email.is_some() && self.user_id.is_some() {
            return Err(DomainError::Invariant(Invariant::LeasePartyEmailLocked));
        }

        let changed = full_name.is_some()
            || email.is_some()
            || phone.is_some()
            || role.is_some()
            || share_percent.is_some()
            || is_primary.is_some();
        if !changed {
            return Ok(false);
        }

        if let Some(name) = full_name {
            self.full_name = text::required("full_name", &name, 200)?;
        }
        if let Some(email) = email {
            self.email = email;
        }
        if let Some(phone) = phone {
            self.phone = match phone {
                Some(value) => text::optional("phone", Some(value), 40)?,
                None => None,
            };
        }
        if let Some(role) = role {
            self.role = role;
        }
        if let Some(share) = share_percent {
            self.share_percent = validate_share(share)?;
        }
        if let Some(is_primary) = is_primary {
            self.is_primary = is_primary;
        }

        self.updated_at = now;
        Ok(true)
    }

    /// Toglie una parte inserita per sbaglio: cancellazione soft.
    ///
    /// **Solo se non è mai stata collegata a un utente.** Con un utente dietro non
    /// è più una correzione: è una persona che esce di casa, che ha una data
    /// (`left_at`), conseguenze sui suoi addebiti e una domanda aperta su che cosa
    /// veda dopo. Confondere le due operazioni farebbe sparire senza traccia un
    /// inquilino vero.
    pub fn remove(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if self.user_id.is_some() {
            return Err(DomainError::Invariant(Invariant::LeasePartyHasUser));
        }
        if self.archived_at.is_some() {
            return Ok(());
        }
        self.archived_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    /// La parte esce dal contratto: è il lato «chi esce» del subentro (ADR-056).
    ///
    /// `today` e non una data a scelta: chi legge `left_at` —
    /// `app.has_lease_in_force`, `app.property_residents`,
    /// [`Self::ensure_tenancy_complete`] — guarda **se c'è** e non che data porta,
    /// quindi una data futura farebbe uscire la persona da subito dicendo il
    /// contrario. `lease_parties_dates_ck` (`left_at >= joined_at`) è rispettato per
    /// costruzione: `joined_at` lo scrive [`Self::link_user`] con la data dello
    /// stesso orologio, che non può essere dopo oggi.
    ///
    /// **Toglie l'intestazione, e dice se c'era.** Chi è uscito non intesta niente,
    /// e `lease_parties_primary_uk` non guarda `left_at`: l'intestazione passa a chi
    /// entra, e questa riga va scritta **prima** della sua, o per un istante gli
    /// intestatari sono due. Restituirla qui tiene l'ordine in un posto solo.
    pub fn leave(&mut self, today: NaiveDate, now: DateTime<Utc>) -> DomainResult<bool> {
        if self.left_at.is_some() {
            return Err(DomainError::Invariant(Invariant::LeasePartyAlreadyLeft));
        }
        let was_primary = self.is_primary;
        self.left_at = Some(today);
        self.is_primary = false;
        self.updated_at = now;
        Ok(was_primary)
    }

    /// Le quote di chi abita devono coprire il canone per intero, e ci deve essere
    /// almeno un inquilino.
    ///
    /// Sta nell'applicazione e non nel database perché è una regola su più righe
    /// che vale solo al momento dell'attivazione: un contratto in bozza a cui si
    /// stanno aggiungendo le parti passa per stati intermedi legittimi in cui la
    /// somma non è ancora 100.
    pub fn ensure_tenancy_complete(parties: &[Self]) -> DomainResult<()> {
        let tenancy: Vec<&Self> = parties
            .iter()
            .filter(|party| party.role.shares_rent() && party.left_at.is_none())
            .collect();

        if tenancy.is_empty() {
            return Err(DomainError::Invariant(Invariant::LeaseWithoutTenantParty));
        }

        // **Una quota a zero è un errore, e la somma non lo vede.** `100 + 0` fa
        // 100 e passava: era il caso osservato in database, dove il 100% stava su
        // una parte che non aveva mai accettato l'invito e l'inquilino vero non
        // doveva niente. Vedi ADR-046.
        if tenancy
            .iter()
            .any(|party| party.share_percent == Decimal::ZERO)
        {
            return Err(DomainError::Invariant(Invariant::LeaseTenantWithoutShare));
        }

        let total: Decimal = tenancy.iter().map(|party| party.share_percent).sum();
        if total != Decimal::from(100) {
            return Err(DomainError::Invariant(Invariant::LeaseSharesNotHundred {
                total,
            }));
        }
        Ok(())
    }

    #[must_use]
    pub const fn id(&self) -> LeasePartyId {
        self.id
    }
    #[must_use]
    pub const fn org_id(&self) -> OrgId {
        self.org_id
    }
    #[must_use]
    pub const fn lease_id(&self) -> LeaseId {
        self.lease_id
    }
    #[must_use]
    pub const fn user_id(&self) -> Option<UserId> {
        self.user_id
    }
    #[must_use]
    pub const fn role(&self) -> LeasePartyRole {
        self.role
    }
    #[must_use]
    pub fn full_name(&self) -> &str {
        &self.full_name
    }
    #[must_use]
    pub const fn email(&self) -> Option<&Email> {
        self.email.as_ref()
    }
    #[must_use]
    pub fn phone(&self) -> Option<&str> {
        self.phone.as_deref()
    }
    #[must_use]
    pub const fn share_percent(&self) -> Decimal {
        self.share_percent
    }
    #[must_use]
    pub const fn is_primary(&self) -> bool {
        self.is_primary
    }
    #[must_use]
    pub const fn joined_at(&self) -> Option<NaiveDate> {
        self.joined_at
    }
    #[must_use]
    pub const fn left_at(&self) -> Option<NaiveDate> {
        self.left_at
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
    pub fn from_persistence(parts: LeasePartyParts) -> Self {
        let LeasePartyParts {
            id,
            org_id,
            lease_id,
            user_id,
            role,
            full_name,
            email,
            phone,
            share_percent,
            is_primary,
            joined_at,
            left_at,
            created_at,
            updated_at,
            archived_at,
        } = parts;
        Self {
            id,
            org_id,
            lease_id,
            user_id,
            role,
            full_name,
            email,
            phone,
            share_percent,
            is_primary,
            joined_at,
            left_at,
            created_at,
            updated_at,
            archived_at,
        }
    }
}

/// Dati grezzi di una parte contrattuale letti dalla persistenza.
#[derive(Debug, Clone)]
pub struct LeasePartyParts {
    pub id: LeasePartyId,
    pub org_id: OrgId,
    pub lease_id: LeaseId,
    pub user_id: Option<UserId>,
    pub role: LeasePartyRole,
    pub full_name: String,
    pub email: Option<Email>,
    pub phone: Option<String>,
    pub share_percent: Decimal,
    pub is_primary: bool,
    pub joined_at: Option<NaiveDate>,
    pub left_at: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

/// Replica il dominio `app.percentage`: validare qui dà un 422 leggibile invece di
/// una violazione di vincolo.
fn validate_share(value: Decimal) -> DomainResult<Decimal> {
    if value < Decimal::ZERO || value > Decimal::from(100) {
        return Err(DomainError::validation(
            "share_percent",
            "must be between 0 and 100",
        ));
    }
    if value.scale() > 2 {
        return Err(DomainError::validation(
            "share_percent",
            "allows at most two decimal places",
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(raw: &str) -> Decimal {
        Decimal::from_str(raw).expect("decimale di test valido")
    }

    fn input(role: LeasePartyRole, full_name: &str, share: &str) -> NewLeaseParty {
        NewLeaseParty {
            id: LeasePartyId::generate(),
            org_id: OrgId::generate(),
            lease_id: LeaseId::generate(),
            role,
            full_name: full_name.to_owned(),
            email: None,
            share_percent: dec(share),
            is_primary: false,
        }
    }

    fn party(role: LeasePartyRole, share: &str) -> LeaseParty {
        LeaseParty::new(input(role, "Mario Rossi", share), Utc::now()).expect("parte valida")
    }

    #[test]
    fn full_name_is_required() {
        assert!(LeaseParty::new(input(LeasePartyRole::Tenant, "  ", "100"), Utc::now()).is_err());
    }

    #[test]
    fn share_must_be_a_percentage() {
        for bad in ["-1", "100.01", "33.333"] {
            assert!(
                LeaseParty::new(
                    input(LeasePartyRole::Tenant, "Mario Rossi", bad),
                    Utc::now()
                )
                .is_err(),
                "{bad} doveva essere rifiutata"
            );
        }
    }

    #[test]
    fn shares_of_the_tenancy_must_sum_to_one_hundred() {
        let complete = [
            party(LeasePartyRole::Tenant, "60"),
            party(LeasePartyRole::Cotenant, "40"),
        ];
        assert!(LeaseParty::ensure_tenancy_complete(&complete).is_ok());

        let incomplete = [
            party(LeasePartyRole::Tenant, "60"),
            party(LeasePartyRole::Cotenant, "30"),
        ];
        let error = LeaseParty::ensure_tenancy_complete(&incomplete).expect_err("quote incomplete");
        assert!(matches!(error, DomainError::Invariant(_)));
    }

    #[test]
    fn a_guarantor_does_not_count_towards_the_shares() {
        let parties = [
            party(LeasePartyRole::Tenant, "100"),
            party(LeasePartyRole::Guarantor, "0"),
        ];
        assert!(LeaseParty::ensure_tenancy_complete(&parties).is_ok());

        // Un contratto con il solo garante non ha inquilini: non è attivabile.
        let only_guarantor = [party(LeasePartyRole::Guarantor, "100")];
        assert!(LeaseParty::ensure_tenancy_complete(&only_guarantor).is_err());
    }

    #[test]
    fn linking_a_user_is_idempotent_but_not_transferable() {
        let mut party = party(LeasePartyRole::Tenant, "100");
        let user = UserId::generate();
        party.link_user(user, Utc::now()).expect("collegamento");
        assert_eq!(party.user_id(), Some(user));
        assert!(party.joined_at().is_some());

        // Riaccettare lo stesso invito non è un errore.
        party
            .link_user(user, Utc::now())
            .expect("stesso utente: idempotente");

        // Un utente diverso sarebbe un trasferimento silenzioso del contratto.
        assert!(party.link_user(UserId::generate(), Utc::now()).is_err());
    }
}
