use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::error::{DomainError, DomainResult, Invariant};
use crate::payment::PaymentMethod;
use crate::shared::{Currency, ExpenseGroupId, Money, OrgId, SettlementId, UserId, text};

/// Stato di un regolamento fra coinquilini.
///
/// **Tutti e quattro si scrivono**, e questa è la differenza più netta con i
/// pagamenti, dove tre valori su sette restano fuori. La ragione non è una scelta
/// di prodotto: è che qui lo schema dice una cosa diversa.
///
/// ```text
/// (chi paga) ──declare──►  pending ──confirm (chi riceve)──►  confirmed
///                             ├─────reject  (chi riceve)───►  rejected
///                             └─────cancel  (chi paga)─────►  cancelled
/// ```
///
/// **Solo `confirmed` muove i conti**: `app.v_expense_group_balances` somma
/// `WHERE st.status = 'confirmed'`, e questo lo decide la vista, non noi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettlementStatus {
    /// Dichiarato da chi paga e non ancora confermato. **Non muove i saldi**: è
    /// ciò che rende la dichiarazione visibile senza renderla vera.
    Pending,
    /// Chi riceve ha confermato che il denaro è arrivato. Da qui i saldi si
    /// muovono.
    Confirmed,
    /// Chi riceve dice che il denaro non è arrivato.
    Rejected,
    /// Chi ha dichiarato ritira la propria dichiarazione.
    Cancelled,
}

impl SettlementStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Confirmed => "confirmed",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
        }
    }

    /// L'unico stato che la vista dei saldi conta.
    #[must_use]
    pub const fn counts_towards_balance(self) -> bool {
        matches!(self, Self::Confirmed)
    }
}

/// Dati necessari a dichiarare un regolamento.
///
/// **Non c'è `currency`**, per la ragione di ADR-030. **Non c'è `payment_id`**:
/// vedi la nota su [`Settlement`].
#[derive(Debug, Clone)]
pub struct NewSettlement {
    pub id: SettlementId,
    pub org_id: OrgId,
    pub group_id: ExpenseGroupId,
    /// Chi paga, cioè chi dichiara. La policy `settlements_insert` chiede
    /// `from_user_id = app.current_user_id() OR app.is_org_staff(org_id)`.
    pub from_user_id: UserId,
    /// Chi riceve, e che dovrà confermare.
    pub to_user_id: UserId,
    pub amount: Decimal,
    pub method: PaymentMethod,
    /// Quando il denaro si è mosso, se chi dichiara lo sa. `None` significa
    /// «adesso», e la conferma lo risolve.
    pub settled_at: Option<DateTime<Utc>>,
    pub note: Option<String>,
}

/// Un regolamento: il denaro che passa **fra due coinquilini**.
///
/// # A confermare è chi riceve, non lo staff
///
/// È la forma dei pagamenti con la differenza che conta: là confermava lo
/// **staff**, qui conferma il **creditore**, perché è lui l'unico che sa se il
/// bonifico è arrivato. Lo schema lo dice già e non lo si sta forzando:
///
/// | | |
/// |---|---|
/// | `settlements_insert` | `from_user_id = current_user_id() OR is_org_staff(org_id)` |
/// | `settlements_update` | `to_user_id` **oppure** `from_user_id` **oppure** staff |
/// | `settlements.confirmed_by` | esiste, e la colonna dice a chi tocca |
///
/// # Chi dichiara **può** ritirare, e qui è l'opposto dei pagamenti
///
/// La `USING` di `settlements_update` ammette anche `from_user_id`, mentre
/// `payments_update` è staff soltanto. Ne segue che [`Settlement::cancel`] esiste
/// e [`Payment::reject`](crate::payment::Payment::reject) è l'unica uscita di una
/// dichiarazione di pagamento — e la ragione dell'asimmetria non è nostra, è dello
/// schema: **fra pari il ritiro è lecito, verso il proprietario no.** Portare qui
/// il vincolo dei pagamenti significherebbe copiarne la forma senza copiarne la
/// ragione.
///
/// # Il regolamento **non** crea un pagamento
///
/// `settlements.payment_id` è una foreign key verso `app.payments`, e
/// `app.payable_type` comprende `settlement`: sono **due modi diversi** di legare
/// la stessa coppia di righe, e tenerli entrambi darebbe due risposte alla domanda
/// «qual è il pagamento di questo regolamento». In questa fetta **nessuno dei due
/// si scrive**, e la ragione non è solo la simmetria: un regolamento fra
/// coinquilini non è denaro che l'organizzazione incassa, e farne una riga in
/// `app.payments` metterebbe nel registro degli incassi del proprietario un
/// movimento che non lo riguarda.
///
/// Ne segue che [`Payable`](crate::payment::Payable) resta a **una** variante. Il
/// piano dei pagamenti prevedeva che questo modulo la portasse a tre; la
/// previsione era sbagliata, e va detto perché è esattamente il genere di
/// conclusione che si eredita senza riverificarla.
///
/// # Nessun soft delete
///
/// `app.settlements` non ha `deleted_at` — verificato, non dedotto — e il ruolo
/// applicativo non ha `DELETE`. Un regolamento sbagliato si ritira muovendone lo
/// stato. Ne segue una regola per le query: **non filtrare `deleted_at`**, che qui
/// non esiste. Copiare il pattern da un altro repository dà un errore di colonna
/// inesistente, fastidioso e innocuo perché fallisce subito.
#[derive(Debug, Clone)]
pub struct Settlement {
    id: SettlementId,
    org_id: OrgId,
    group_id: ExpenseGroupId,
    from_user_id: UserId,
    to_user_id: UserId,
    amount: Money,
    currency: Currency,
    status: SettlementStatus,
    method: PaymentMethod,
    note: Option<String>,
    settled_at: Option<DateTime<Utc>>,
    confirmed_at: Option<DateTime<Utc>>,
    confirmed_by: Option<UserId>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Settlement {
    /// `note` è `TEXT` senza vincoli a schema: il tetto è nostro.
    pub const MAX_NOTE_LENGTH: usize = 1_000;

    /// Chi paga dichiara di aver saldato: nasce **`pending`** e **non muove i
    /// saldi**.
    pub fn declare(parts: NewSettlement, now: DateTime<Utc>) -> DomainResult<Self> {
        let NewSettlement {
            id,
            org_id,
            group_id,
            from_user_id,
            to_user_id,
            amount,
            method,
            settled_at,
            note,
        } = parts;

        // `settlements_distinct_parties_ck` è `from_user_id <> to_user_id`:
        // dirlo qui dà un errore di dominio invece di una violazione di vincolo
        // che il chiamante non saprebbe interpretare.
        if from_user_id == to_user_id {
            return Err(DomainError::validation(
                "to_user_id",
                "a settlement needs two different people",
            ));
        }

        // `> 0` come `app.settlements.amount CHECK (amount > 0)`.
        let currency = Currency::EUR;
        let amount = Money::new(amount, currency)
            .map_err(|_| DomainError::validation("amount", "must be a valid amount"))?;
        if amount.amount() <= Decimal::ZERO {
            return Err(DomainError::validation(
                "amount",
                "must be greater than zero",
            ));
        }

        Ok(Self {
            id,
            org_id,
            group_id,
            from_user_id,
            to_user_id,
            amount,
            currency,
            status: SettlementStatus::Pending,
            method,
            note: text::optional("note", note, Self::MAX_NOTE_LENGTH)?,
            settled_at,
            confirmed_at: None,
            confirmed_by: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Chi riceve conferma: `pending` → `confirmed`, e da qui i saldi si muovono.
    ///
    /// `confirmed_at` è valorizzato **sempre**, e non per comodità:
    /// `settlements_confirmed_ck` è `status <> 'confirmed' OR confirmed_at IS NOT
    /// NULL`, e dimenticarlo non darebbe una riga strana ma un rifiuto alla
    /// scrittura. `confirmed_by` registra **chi** ha confermato, che è
    /// l'informazione per cui la colonna esiste.
    ///
    /// `settled_at` ricade su `now` se chi ha dichiarato non lo ha detto: è la
    /// data in cui il denaro si è mosso, e confermare significa dire che si è
    /// mosso. La data **dichiarata non si riscrive**: è il fatto dichiarato, non
    /// il momento in cui qualcuno l'ha guardato.
    pub fn confirm(&mut self, confirmed_by: UserId, now: DateTime<Utc>) -> DomainResult<()> {
        self.ensure_pending(Invariant::SettlementNotConfirmable)?;
        self.status = SettlementStatus::Confirmed;
        self.settled_at = Some(self.settled_at.unwrap_or(now));
        self.confirmed_at = Some(now);
        self.confirmed_by = Some(confirmed_by);
        self.updated_at = now;
        Ok(())
    }

    /// Chi riceve dice che il denaro non è arrivato: `pending` → `rejected`.
    ///
    /// Nasce **insieme** a `pending` e non dopo, per la ragione di sempre: uno
    /// stato in cui si entra e da cui non si esce è peggio di nessuno stato.
    pub fn reject(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        self.ensure_pending(Invariant::SettlementNotRejectable)?;
        self.status = SettlementStatus::Rejected;
        self.clear_transfer(now);
        Ok(())
    }

    /// Chi ha dichiarato ritira: `pending` → `cancelled`.
    ///
    /// **Questa transizione esiste perché la RLS la ammette**, e non per simmetria
    /// con i pagamenti — dove infatti non esiste. Vedi la nota sul tipo.
    pub fn cancel(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        self.ensure_pending(Invariant::SettlementNotCancellable)?;
        self.status = SettlementStatus::Cancelled;
        self.clear_transfer(now);
        Ok(())
    }

    const fn ensure_pending(&self, invariant: Invariant) -> DomainResult<()> {
        if matches!(self.status, SettlementStatus::Pending) {
            Ok(())
        } else {
            Err(DomainError::Invariant(invariant))
        }
    }

    /// Le due uscite negative **azzerano `settled_at`**, e tocca al dominio farlo.
    ///
    /// `settlements_confirmed_ck` vincola **una sola direzione** — obbliga ad avere
    /// `confirmed_at` quando lo stato è `confirmed`, e non obbliga a togliere
    /// niente muovendosi altrove. È la **quarta** occorrenza di questa forma dopo
    /// `tickets_closed_ck`, `utility_bills_validated_ck` e `payments_succeeded_ck`:
    /// ormai non è una trappola, è una convenzione dello schema.
    ///
    /// Lasciare la data dichiarerebbe un trasferimento che non è avvenuto.
    fn clear_transfer(&mut self, now: DateTime<Utc>) {
        self.settled_at = None;
        self.confirmed_at = None;
        self.confirmed_by = None;
        self.updated_at = now;
    }

    /// `true` se questo regolamento è contato dai saldi del gruppo.
    #[must_use]
    pub const fn counts_towards_balance(&self) -> bool {
        self.status.counts_towards_balance()
    }

    #[must_use]
    pub const fn id(&self) -> SettlementId {
        self.id
    }
    #[must_use]
    pub const fn org_id(&self) -> OrgId {
        self.org_id
    }
    #[must_use]
    pub const fn group_id(&self) -> ExpenseGroupId {
        self.group_id
    }
    #[must_use]
    pub const fn from_user_id(&self) -> UserId {
        self.from_user_id
    }
    #[must_use]
    pub const fn to_user_id(&self) -> UserId {
        self.to_user_id
    }
    #[must_use]
    pub const fn amount(&self) -> Money {
        self.amount
    }
    #[must_use]
    pub const fn currency(&self) -> Currency {
        self.currency
    }
    #[must_use]
    pub const fn status(&self) -> SettlementStatus {
        self.status
    }
    #[must_use]
    pub const fn method(&self) -> PaymentMethod {
        self.method
    }
    #[must_use]
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
    #[must_use]
    pub const fn settled_at(&self) -> Option<DateTime<Utc>> {
        self.settled_at
    }
    #[must_use]
    pub const fn confirmed_at(&self) -> Option<DateTime<Utc>> {
        self.confirmed_at
    }
    #[must_use]
    pub const fn confirmed_by(&self) -> Option<UserId> {
        self.confirmed_by
    }
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Ricostruzione da persistenza: non riesegue le invarianti.
    #[must_use]
    pub fn from_persistence(parts: SettlementParts) -> Self {
        let SettlementParts {
            id,
            org_id,
            group_id,
            from_user_id,
            to_user_id,
            amount,
            currency,
            status,
            method,
            note,
            settled_at,
            confirmed_at,
            confirmed_by,
            created_at,
            updated_at,
        } = parts;
        Self {
            id,
            org_id,
            group_id,
            from_user_id,
            to_user_id,
            amount,
            currency,
            status,
            method,
            note,
            settled_at,
            confirmed_at,
            confirmed_by,
            created_at,
            updated_at,
        }
    }
}

/// Dati grezzi di un regolamento letto dalla persistenza.
#[derive(Debug, Clone)]
pub struct SettlementParts {
    pub id: SettlementId,
    pub org_id: OrgId,
    pub group_id: ExpenseGroupId,
    pub from_user_id: UserId,
    pub to_user_id: UserId,
    pub amount: Money,
    pub currency: Currency,
    pub status: SettlementStatus,
    pub method: PaymentMethod,
    pub note: Option<String>,
    pub settled_at: Option<DateTime<Utc>>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub confirmed_by: Option<UserId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(raw: &str) -> Decimal {
        Decimal::from_str(raw).expect("decimale di test valido")
    }

    fn new_settlement() -> NewSettlement {
        NewSettlement {
            id: SettlementId::generate(),
            org_id: OrgId::generate(),
            group_id: ExpenseGroupId::generate(),
            from_user_id: UserId::generate(),
            to_user_id: UserId::generate(),
            amount: dec("33.33"),
            method: PaymentMethod::BankTransfer,
            settled_at: None,
            note: Some("Saldo di settembre".to_owned()),
        }
    }

    /// La decisione 5 in forma di test: la dichiarazione è visibile e **non è
    /// vera**, cioè non conta nei saldi.
    #[test]
    fn a_declaration_is_born_pending_and_does_not_move_the_balance() {
        let settlement = Settlement::declare(new_settlement(), Utc::now()).expect("valido");
        assert_eq!(settlement.status(), SettlementStatus::Pending);
        assert!(!settlement.counts_towards_balance());
        assert_eq!(settlement.confirmed_at(), None);
        assert_eq!(settlement.confirmed_by(), None);
    }

    /// `settlements_confirmed_ck` esige `confirmed_at` quando lo stato è
    /// `confirmed`, e `confirmed_by` registra chi ha confermato: è l'informazione
    /// per cui la colonna esiste.
    #[test]
    fn confirming_records_who_confirmed_and_when() {
        let mut settlement = Settlement::declare(new_settlement(), Utc::now()).expect("valido");
        let creditor = settlement.to_user_id();

        let now = Utc::now();
        settlement.confirm(creditor, now).expect("confermato");

        assert_eq!(settlement.status(), SettlementStatus::Confirmed);
        assert!(settlement.counts_towards_balance());
        assert_eq!(settlement.confirmed_at(), Some(now));
        assert_eq!(settlement.confirmed_by(), Some(creditor));
        assert_eq!(
            settlement.settled_at(),
            Some(now),
            "confermare significa dire che il denaro si è mosso"
        );
    }

    /// La data dichiarata sopravvive alla conferma: è il fatto dichiarato, non il
    /// momento in cui qualcuno l'ha guardato.
    #[test]
    fn the_declared_date_survives_the_confirmation() {
        let claimed = Utc::now() - chrono::Duration::days(2);
        let mut settlement = Settlement::declare(
            NewSettlement {
                settled_at: Some(claimed),
                ..new_settlement()
            },
            Utc::now(),
        )
        .expect("valido");
        let creditor = settlement.to_user_id();

        settlement
            .confirm(creditor, Utc::now())
            .expect("confermato");
        assert_eq!(settlement.settled_at(), Some(claimed));
    }

    /// La trappola del `CHECK` a una direzione sola, quarta occorrenza: le uscite
    /// negative azzerano la data, e tocca al dominio farlo.
    #[test]
    fn the_two_negative_exits_take_the_claimed_date_away() {
        let claimed = Utc::now() - chrono::Duration::days(2);
        for negative in ["reject", "cancel"] {
            let mut settlement = Settlement::declare(
                NewSettlement {
                    settled_at: Some(claimed),
                    ..new_settlement()
                },
                Utc::now(),
            )
            .expect("valido");

            if negative == "reject" {
                settlement.reject(Utc::now()).expect("rifiutato");
                assert_eq!(settlement.status(), SettlementStatus::Rejected);
            } else {
                settlement.cancel(Utc::now()).expect("ritirato");
                assert_eq!(settlement.status(), SettlementStatus::Cancelled);
            }

            assert_eq!(
                settlement.settled_at(),
                None,
                "un trasferimento che non è avvenuto non ha una data in cui è avvenuto"
            );
            assert!(!settlement.counts_towards_balance());
        }
    }

    /// Le transizioni rifiutate contano quanto quelle ammesse: `pending` ha tre
    /// uscite e nessun ingresso di ritorno.
    #[test]
    fn the_transition_table_is_complete() {
        let now = Utc::now();
        let confirmer = UserId::generate();

        for terminal in ["confirm", "reject", "cancel"] {
            let mut settlement = Settlement::declare(new_settlement(), now).expect("valido");
            match terminal {
                "confirm" => settlement.confirm(confirmer, now).expect("confermato"),
                "reject" => settlement.reject(now).expect("rifiutato"),
                _ => settlement.cancel(now).expect("ritirato"),
            }

            assert!(settlement.confirm(confirmer, now).is_err(), "{terminal}");
            assert!(settlement.reject(now).is_err(), "{terminal}");
            assert!(settlement.cancel(now).is_err(), "{terminal}");
        }
    }

    /// `settlements_distinct_parties_ck`, detto dal dominio: un errore di dominio
    /// invece di una violazione di vincolo che il chiamante non saprebbe leggere.
    #[test]
    fn nobody_settles_with_themselves() {
        let same = UserId::generate();
        assert!(
            Settlement::declare(
                NewSettlement {
                    from_user_id: same,
                    to_user_id: same,
                    ..new_settlement()
                },
                Utc::now()
            )
            .is_err()
        );
    }

    #[test]
    fn an_amount_that_is_not_money_moving_is_refused() {
        for amount in ["0", "0.00", "-10.00", "10.1234"] {
            assert!(
                Settlement::declare(
                    NewSettlement {
                        amount: dec(amount),
                        ..new_settlement()
                    },
                    Utc::now()
                )
                .is_err(),
                "`{amount}` non è denaro che si muove"
            );
        }
    }

    /// **Solo `confirmed` conta**, e lo decide la vista: `st.status =
    /// 'confirmed'`. Il test lo fissa dove il dominio lo dichiara.
    #[test]
    fn only_confirmed_counts_towards_the_balance() {
        assert!(SettlementStatus::Confirmed.counts_towards_balance());
        for status in [
            SettlementStatus::Pending,
            SettlementStatus::Rejected,
            SettlementStatus::Cancelled,
        ] {
            assert!(!status.counts_towards_balance(), "{}", status.as_str());
        }
    }
}
