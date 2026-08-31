use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use super::value_objects::IdempotencyKey;
use crate::error::{DomainError, DomainResult, Invariant};
use crate::shared::{ChargeId, Currency, Money, OrgId, PaymentId, UserId, text};

/// Che cosa il pagamento salda. Rispecchia `app.payable_type` **in parte**.
///
/// L'enum del database ha tre valori — `charge`, `expense_share`, `settlement` —
/// e qui ce n'è uno solo, deliberatamente: gli altri due appartengono al modulo
/// delle spese co-living, che non esiste. È la stessa scelta di
/// [`AttachmentOwner`](crate::document::AttachmentOwner), che ha lasciato fuori gli
/// owner dei moduli non ancora costruiti, e ha la stessa conseguenza voluta:
/// leggere una riga con un `payable_type` fuori perimetro **dà errore** invece di
/// ricadere su un valore plausibile. Nessuna riga simile può esistere oggi, perché
/// l'applicazione è l'unica a scrivere su `app.payments`.
///
/// Il campo `payable_id` è polimorfico e non ha una foreign key: a validarlo è il
/// trigger `app.tg_check_polymorphic_ref`, che verifica soltanto che la riga
/// **esista**. Non guarda l'organizzazione — vedi la nota su [`Payment`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Payable {
    Charge(ChargeId),
}

impl Payable {
    /// Nome stabile sul filo degli eventi e valore dell'enum in database.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Charge(_) => "charge",
        }
    }

    #[must_use]
    pub const fn charge_id(self) -> ChargeId {
        match self {
            Self::Charge(id) => id,
        }
    }
}

/// Stato di un pagamento.
///
/// **Quattro valori su sette esistono nell'enum del database e l'applicazione non
/// li scrive mai**, e le ragioni sono diverse fra loro:
///
/// - `Authorized` perché descrive denaro **autorizzato presso un provider** e non
///   ancora incassato. Con `provider = 'manual'` un provider non c'è. Il valore va
///   saputo comunque, perché `payments_payable_ix` lo elenca e copiare l'elenco
///   dell'indice dentro il calcolo del saldo farebbe scendere l'insoluto su denaro
///   che non è arrivato: il letterale da usare è quello **della vista**;
/// - `Failed` per la stessa ragione: significa che il pagamento non è andato a
///   buon fine presso il provider;
/// - `Refunded` e `PartiallyRefunded` perché i rimborsi sono fuori dal perimetro
///   della fetta. `PartiallyRefunded` è comunque **contato** da
///   `app.v_charge_balances`, il che va saputo: il giorno in cui un rimborso si
///   scriverà, il saldo lo seguirà da sé.
///
/// Restano fra le varianti perché una riga scritta a mano potrebbe contenerli, e
/// un valore sconosciuto in lettura deve dare errore invece di ricadere su un
/// default plausibile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentStatus {
    /// Dichiarato dall'inquilino e non ancora confermato. **Non muove il saldo**:
    /// è ciò che rende la dichiarazione visibile senza renderla vera.
    Pending,
    /// Mai scritto: vedi la nota sul tipo.
    Authorized,
    Succeeded,
    /// Mai scritto: vedi la nota sul tipo.
    Failed,
    /// L'uscita negativa di `Pending`: la dichiarazione è stata rifiutata.
    Cancelled,
    /// Mai scritto: i rimborsi sono fuori perimetro.
    Refunded,
    /// Mai scritto, e **contato dalla vista**: vedi la nota sul tipo.
    PartiallyRefunded,
}

impl PaymentStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Authorized => "authorized",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Refunded => "refunded",
            Self::PartiallyRefunded => "partially_refunded",
        }
    }

    /// I due stati che **contano nel saldo**.
    ///
    /// Coincide con l'array di `app.v_charge_balances`
    /// (`'succeeded','partially_refunded'`) e **non** con il predicato di
    /// `payments_payable_ix`, che comprende anche `authorized`. La differenza non è
    /// cosmetica: contare gli `authorized` farebbe scendere l'insoluto su denaro
    /// autorizzato e non incassato. Il predicato più stretto resta comunque servito
    /// dall'indice, quindi non si paga nulla in prestazioni per avere ragione.
    pub const COUNTS_TOWARDS_BALANCE: [Self; 2] = [Self::Succeeded, Self::PartiallyRefunded];

    /// `true` se il pagamento fa scendere il residuo dell'addebito.
    #[must_use]
    pub const fn counts_towards_balance(self) -> bool {
        matches!(self, Self::Succeeded | Self::PartiallyRefunded)
    }
}

/// Strumento con cui il denaro si è mosso. Rispecchia `app.payment_method` per
/// intero: nessuno di questi valori è fuori perimetro, perché descrivono come una
/// persona ha pagato e non che cosa un provider ha fatto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentMethod {
    BankTransfer,
    SepaDirectDebit,
    Card,
    Cash,
    Check,
    Other,
}

impl PaymentMethod {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BankTransfer => "bank_transfer",
            Self::SepaDirectDebit => "sepa_direct_debit",
            Self::Card => "card",
            Self::Cash => "cash",
            Self::Check => "check",
            Self::Other => "other",
        }
    }
}

/// Dati necessari a registrare o dichiarare un pagamento.
///
/// **Non c'è `currency`** per la ragione detta in
/// [`NewCharge`](crate::billing::NewCharge): la valuta è una sola, ed è
/// `Currency::EUR`. **Non c'è `status`**: lo decide il costruttore, ed è l'unica
/// differenza fra i due.
#[derive(Debug, Clone)]
pub struct NewPayment {
    pub id: PaymentId,
    pub org_id: OrgId,
    pub payable: Payable,
    /// Chi ha pagato.
    ///
    /// Non è un dettaglio anagrafico: `payments_select` mostra la riga a
    /// `payer_user_id = current_user_id()`, quindi **è ciò che rende il pagamento
    /// visibile a chi lo ha fatto**. Un incasso registrato dallo staff senza
    /// attribuirlo resta visibile allo staff soltanto.
    pub payer_user_id: Option<UserId>,
    pub amount: Decimal,
    pub method: PaymentMethod,
    /// Quando il denaro si è mosso, se chi scrive lo sa. `None` significa
    /// «adesso», e il costruttore lo risolve.
    pub paid_at: Option<DateTime<Utc>>,
    /// Causale o riferimento bancario.
    pub reference: Option<String>,
    pub idempotency_key: IdempotencyKey,
}

/// Un pagamento: il denaro che entra.
///
/// **Non ha `deleted_at`, e non è una dimenticanza:** `app.payments` non ha quella
/// colonna — verificato, non dedotto — e il ruolo applicativo non ha `DELETE`. Un
/// pagamento sbagliato si corregge muovendone lo stato, non togliendo la riga; è
/// coerente con l'audit, che su questa tabella ha il proprio trigger con la
/// redazione di `provider_payload`. Ne segue una regola per le query: **non
/// filtrare `deleted_at`**, che qui non esiste. Copiare il pattern da un altro
/// repository dà un errore di colonna inesistente — fastidioso e innocuo, perché
/// fallisce subito.
///
/// **Il controllo sull'organizzazione dell'addebito non è nello schema.**
/// `app.tg_check_polymorphic_ref` verifica che la riga referenziata esista
/// (`SELECT 1 FROM app.charges WHERE id = $1`) ed è `SECURITY DEFINER`, quindi la
/// verifica non dipende dalla visibilità RLS del chiamante e **non guarda
/// l'organizzazione**. Si potrebbe inserire un pagamento con il proprio `org_id`
/// che punta all'addebito di un'altra organizzazione, e il trigger lo
/// accetterebbe. A impedirlo è lo use case, che rilegge l'addebito dal proprio
/// repository sotto la RLS del chiamante: è la terza volta che questo progetto usa
/// lo stesso rimedio, dopo `ensure_owner_is_visible` dei documenti e
/// `build_issued_charge` degli addebiti.
///
/// **Fuori perimetro in questa fetta**: `provider`, `provider_payment_id` e
/// `provider_payload` (restano ai loro default, `manual` e vuoti);
/// `failure_code` e `failure_message`, che descrivono un fallimento **presso un
/// provider** e con `manual` non hanno origine; `metadata`; e i rimborsi, per cui
/// `refunded_amount` esiste qui in sola lettura e vale sempre zero.
#[derive(Debug, Clone)]
pub struct Payment {
    id: PaymentId,
    org_id: OrgId,
    payable: Payable,
    payer_user_id: Option<UserId>,
    amount: Money,
    currency: Currency,
    method: PaymentMethod,
    status: PaymentStatus,
    idempotency_key: IdempotencyKey,
    reference: Option<String>,
    paid_at: Option<DateTime<Utc>>,
    reconciled_at: Option<DateTime<Utc>>,
    refunded_amount: Money,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Payment {
    /// Registra un incasso già avvenuto: nasce **`succeeded`** e muove il saldo.
    ///
    /// È la via dello staff, che vede il movimento sull'estratto conto. Non c'è un
    /// passo intermedio perché non c'è un secondo osservatore da attendere, ed è lo
    /// stesso argomento per cui un addebito non ha la bozza.
    ///
    /// `paid_at` è valorizzato **sempre**, e non per comodità:
    /// `payments_succeeded_ck` è `status <> 'succeeded' OR paid_at IS NOT NULL`, e
    /// dimenticarlo non darebbe una riga strana ma un rifiuto all'inserimento.
    ///
    /// `reconciled_at` pure: registrare un incasso visto sull'estratto conto **è**
    /// la riconciliazione, in questo modulo. Il giorno in cui arriverà un provider,
    /// i pagamenti nati da un webhook saranno `succeeded` con `reconciled_at` nullo,
    /// ed è allora che `payments_unreconciled_ix` comincerà a servire a qualcosa.
    pub fn record(parts: NewPayment, now: DateTime<Utc>) -> DomainResult<Self> {
        let mut payment = Self::build(parts, PaymentStatus::Succeeded, now)?;
        payment.paid_at = Some(payment.paid_at.unwrap_or(now));
        payment.reconciled_at = Some(now);
        Ok(payment)
    }

    /// Dichiara un pagamento: nasce **`pending`** e **non muove il saldo**.
    ///
    /// È la via dell'inquilino, e lo schema è costruito per essa senza che si debba
    /// forzare niente: `payments_insert` ammette
    /// `is_org_staff(org_id) OR payer_user_id = current_user_id()`, mentre
    /// `payments_update` è staff soltanto. `pending` non è uno stato di comodo — è
    /// ciò che rende la dichiarazione **visibile senza renderla vera**.
    ///
    /// **Chi dichiara non può ritirare**, e non è una scelta di prodotto rivedibile
    /// nel codice: `payments_update` è solo staff, quindi un ritiro otterrebbe zero
    /// righe aggiornate, cioè il codice riservato ai nostri difetti. L'uscita
    /// negativa esiste ed è [`Payment::reject`], che è dello staff.
    pub fn declare(parts: NewPayment, now: DateTime<Utc>) -> DomainResult<Self> {
        Self::build(parts, PaymentStatus::Pending, now)
    }

    fn build(parts: NewPayment, status: PaymentStatus, now: DateTime<Utc>) -> DomainResult<Self> {
        let NewPayment {
            id,
            org_id,
            payable,
            payer_user_id,
            amount,
            method,
            paid_at,
            reference,
            idempotency_key,
        } = parts;

        // `> 0` come `app.payments.amount CHECK (amount > 0)`: uno zero non è un
        // pagamento, e un negativo è un rimborso, che è fuori perimetro.
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
            payable,
            payer_user_id,
            amount,
            currency,
            method,
            status,
            idempotency_key,
            reference: text::optional("reference", reference, 200)?,
            paid_at,
            reconciled_at: None,
            refunded_amount: Money::non_negative(Decimal::ZERO, currency)
                .map_err(|error| DomainError::validation("refunded_amount", error))?,
            created_at: now,
            updated_at: now,
        })
    }

    /// Conferma una dichiarazione: `pending` → `succeeded`. Da qui il saldo si
    /// muove.
    ///
    /// `paid_at` ricade su `now` se chi ha dichiarato non lo ha detto, perché
    /// `payments_succeeded_ck` lo esige. `reconciled_at` è valorizzato per la stessa
    /// ragione di [`Payment::record`]: confermare è l'atto con cui una persona
    /// riconosce il movimento.
    pub fn confirm(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if !matches!(self.status, PaymentStatus::Pending) {
            return Err(DomainError::Invariant(Invariant::PaymentNotConfirmable));
        }
        self.status = PaymentStatus::Succeeded;
        self.paid_at = Some(self.paid_at.unwrap_or(now));
        self.reconciled_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    /// Rifiuta una dichiarazione: `pending` → `cancelled`.
    ///
    /// L'uscita negativa nasce **insieme** a `pending` e non dopo: uno stato in cui
    /// si entra e da cui non si esce è peggio di nessuno stato, ed è precisamente la
    /// ragione per cui `bill_status.disputed` è rimasto fuori dalla fetta contabile.
    ///
    /// **`paid_at` viene azzerato**, ed è il dominio a doverlo fare.
    /// `payments_succeeded_ck` vincola una sola direzione — obbliga ad avere la data
    /// quando si è riusciti, e **non** obbliga a toglierla muovendosi altrove — ed è
    /// la terza volta che questo schema presenta questa forma, dopo
    /// `tickets_closed_ck` e `utility_bills_validated_ck`. Lasciarla dichiarerebbe
    /// una data di pagamento per un pagamento che non è avvenuto.
    ///
    /// Non è `failed`: quello significa che il pagamento non è andato a buon fine
    /// **presso il provider**, e con `manual` un provider non c'è.
    pub fn reject(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if !matches!(self.status, PaymentStatus::Pending) {
            return Err(DomainError::Invariant(Invariant::PaymentNotRejectable));
        }
        self.status = PaymentStatus::Cancelled;
        self.paid_at = None;
        self.updated_at = now;
        Ok(())
    }

    /// `true` se questo pagamento è contato dal saldo dell'addebito.
    #[must_use]
    pub const fn counts_towards_balance(&self) -> bool {
        self.status.counts_towards_balance()
    }

    #[must_use]
    pub const fn id(&self) -> PaymentId {
        self.id
    }
    #[must_use]
    pub const fn org_id(&self) -> OrgId {
        self.org_id
    }
    #[must_use]
    pub const fn payable(&self) -> Payable {
        self.payable
    }
    #[must_use]
    pub const fn payer_user_id(&self) -> Option<UserId> {
        self.payer_user_id
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
    pub const fn method(&self) -> PaymentMethod {
        self.method
    }
    #[must_use]
    pub const fn status(&self) -> PaymentStatus {
        self.status
    }
    #[must_use]
    pub const fn idempotency_key(&self) -> &IdempotencyKey {
        &self.idempotency_key
    }
    #[must_use]
    pub fn reference(&self) -> Option<&str> {
        self.reference.as_deref()
    }
    #[must_use]
    pub const fn paid_at(&self) -> Option<DateTime<Utc>> {
        self.paid_at
    }
    #[must_use]
    pub const fn reconciled_at(&self) -> Option<DateTime<Utc>> {
        self.reconciled_at
    }
    #[must_use]
    pub const fn refunded_amount(&self) -> Money {
        self.refunded_amount
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
    pub fn from_persistence(parts: PaymentParts) -> Self {
        let PaymentParts {
            id,
            org_id,
            payable,
            payer_user_id,
            amount,
            currency,
            method,
            status,
            idempotency_key,
            reference,
            paid_at,
            reconciled_at,
            refunded_amount,
            created_at,
            updated_at,
        } = parts;
        Self {
            id,
            org_id,
            payable,
            payer_user_id,
            amount,
            currency,
            method,
            status,
            idempotency_key,
            reference,
            paid_at,
            reconciled_at,
            refunded_amount,
            created_at,
            updated_at,
        }
    }
}

/// Dati grezzi di un pagamento letto dalla persistenza.
#[derive(Debug, Clone)]
pub struct PaymentParts {
    pub id: PaymentId,
    pub org_id: OrgId,
    pub payable: Payable,
    pub payer_user_id: Option<UserId>,
    pub amount: Money,
    pub currency: Currency,
    pub method: PaymentMethod,
    pub status: PaymentStatus,
    pub idempotency_key: IdempotencyKey,
    pub reference: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub reconciled_at: Option<DateTime<Utc>>,
    pub refunded_amount: Money,
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

    fn new_payment() -> NewPayment {
        NewPayment {
            id: PaymentId::generate(),
            org_id: OrgId::generate(),
            payable: Payable::Charge(ChargeId::generate()),
            payer_user_id: Some(UserId::generate()),
            amount: dec("800.00"),
            method: PaymentMethod::BankTransfer,
            paid_at: None,
            reference: Some("Bonifico canone settembre".to_owned()),
            idempotency_key: IdempotencyKey::parse("chiave-di-test").expect("valida"),
        }
    }

    /// La decisione 2 in forma di test: la dichiarazione è visibile e **non è
    /// vera**, cioè non conta nel saldo.
    #[test]
    fn a_declaration_is_born_pending_and_does_not_move_the_balance() {
        let payment = Payment::declare(new_payment(), Utc::now()).expect("valido");
        assert_eq!(payment.status(), PaymentStatus::Pending);
        assert!(!payment.counts_towards_balance());
        assert_eq!(
            payment.reconciled_at(),
            None,
            "nessuno ha ancora riconosciuto il movimento"
        );
    }

    /// L'incasso visto sull'estratto conto nasce vero, senza un passo intermedio
    /// che nessuno osserverebbe.
    #[test]
    fn a_recorded_payment_is_born_succeeded_with_both_its_dates() {
        let now = Utc::now();
        let payment = Payment::record(new_payment(), now).expect("valido");
        assert_eq!(payment.status(), PaymentStatus::Succeeded);
        assert!(payment.counts_towards_balance());
        assert_eq!(
            payment.paid_at(),
            Some(now),
            "`payments_succeeded_ck` esige la data quando lo stato è `succeeded`"
        );
        assert_eq!(payment.reconciled_at(), Some(now));
    }

    /// Chi dichiara può dire *quando* ha pagato, e la conferma non riscrive quella
    /// data: è il fatto dichiarato, non il momento in cui qualcuno l'ha guardato.
    #[test]
    fn the_declared_date_survives_the_confirmation() {
        let claimed = Utc::now() - chrono::Duration::days(1);
        let mut payment = Payment::declare(
            NewPayment {
                paid_at: Some(claimed),
                ..new_payment()
            },
            Utc::now(),
        )
        .expect("valido");

        let now = Utc::now();
        payment.confirm(now).expect("confermato");
        assert_eq!(payment.paid_at(), Some(claimed));
        assert_eq!(payment.reconciled_at(), Some(now));
    }

    /// Una dichiarazione senza data confermata **deve** comunque avere `paid_at`,
    /// o `payments_succeeded_ck` rifiuta la scrittura.
    #[test]
    fn confirming_a_declaration_without_a_date_falls_back_to_now() {
        let mut payment = Payment::declare(new_payment(), Utc::now()).expect("valido");
        assert_eq!(payment.paid_at(), None);

        let now = Utc::now();
        payment.confirm(now).expect("confermato");
        assert_eq!(payment.status(), PaymentStatus::Succeeded);
        assert_eq!(payment.paid_at(), Some(now));
    }

    /// La trappola del `CHECK` a una direzione sola: `payments_succeeded_ck`
    /// obbliga ad **avere** la data quando si è riusciti e non obbliga a
    /// **toglierla** muovendosi altrove. L'azzeramento tocca al dominio.
    #[test]
    fn rejecting_a_declaration_takes_its_claimed_date_away() {
        let claimed = Utc::now() - chrono::Duration::days(1);
        let mut payment = Payment::declare(
            NewPayment {
                paid_at: Some(claimed),
                ..new_payment()
            },
            Utc::now(),
        )
        .expect("valido");

        payment.reject(Utc::now()).expect("rifiutato");
        assert_eq!(payment.status(), PaymentStatus::Cancelled);
        assert_eq!(
            payment.paid_at(),
            None,
            "un pagamento rifiutato non ha una data in cui è avvenuto"
        );
        assert!(!payment.counts_towards_balance());
    }

    /// Le transizioni rifiutate contano quanto quelle ammesse: `pending` ha due
    /// uscite e nessun ingresso di ritorno.
    #[test]
    fn the_transition_table_is_complete() {
        let now = Utc::now();

        // Da `succeeded` non si torna indietro.
        let mut recorded = Payment::record(new_payment(), now).expect("valido");
        assert!(recorded.confirm(now).is_err());
        assert!(recorded.reject(now).is_err());

        // Da `cancelled` nemmeno.
        let mut rejected = Payment::declare(new_payment(), now).expect("valido");
        rejected.reject(now).expect("rifiutato");
        assert!(rejected.confirm(now).is_err());
        assert!(rejected.reject(now).is_err());

        // Una conferma non si ripete.
        let mut confirmed = Payment::declare(new_payment(), now).expect("valido");
        confirmed.confirm(now).expect("confermato");
        assert!(confirmed.reject(now).is_err());
    }

    #[test]
    fn an_amount_that_is_not_money_moving_is_refused() {
        let now = Utc::now();
        for amount in ["0", "0.00", "-10.00"] {
            assert!(
                Payment::record(
                    NewPayment {
                        amount: dec(amount),
                        ..new_payment()
                    },
                    now
                )
                .is_err(),
                "`{amount}` non è denaro che si muove"
            );
        }
        // Quattro decimali: `app.money_amount` è `NUMERIC(14,2)`.
        assert!(
            Payment::record(
                NewPayment {
                    amount: dec("10.1234"),
                    ..new_payment()
                },
                now
            )
            .is_err()
        );
    }

    /// L'elenco che conta nel saldo è quello **della vista**, non quello
    /// dell'indice: `authorized` è denaro autorizzato e non incassato, e contarlo
    /// farebbe scendere l'insoluto su un incasso che non è arrivato.
    #[test]
    fn only_the_two_states_of_the_view_count_towards_the_balance() {
        assert_eq!(
            PaymentStatus::COUNTS_TOWARDS_BALANCE,
            [PaymentStatus::Succeeded, PaymentStatus::PartiallyRefunded]
        );
        assert!(!PaymentStatus::Authorized.counts_towards_balance());
        assert!(!PaymentStatus::Pending.counts_towards_balance());
        assert!(!PaymentStatus::Cancelled.counts_towards_balance());
        assert!(!PaymentStatus::Failed.counts_towards_balance());
        assert!(!PaymentStatus::Refunded.counts_towards_balance());
    }
}
