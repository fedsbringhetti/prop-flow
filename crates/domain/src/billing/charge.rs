use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;

use crate::error::{DomainError, DomainResult, Invariant};
use crate::shared::{ChargeId, Currency, LeaseId, Money, OrgId, UtilityBillId, text};

/// Natura dell'addebito. Rispecchia `app.charge_type` per intero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargeType {
    Rent,
    Deposit,
    UtilityRecharge,
    LateFee,
    /// Esiste, e **non** ha un legame di prima classe con la segnalazione che ha
    /// generato la spesa: `app.charges` ha `utility_bill_id` con il suo indice e
    /// **non ha `ticket_id`**. Un proprietario che voglia riaddebitare una
    /// riparazione crea l'addebito a mano e scrive nella descrizione a cosa si
    /// riferisce. Il giorno in cui quel legame debba diventare interrogabile è
    /// una migration che aggiunge la colonna, ed è lo stesso momento in cui va
    /// decisa la distinzione fra manutenzione ordinaria e straordinaria — che il
    /// modello oggi non rappresenta, e che in una locazione italiana decide chi
    /// paga.
    Maintenance,
    Adjustment,
    Other,
}

impl ChargeType {
    /// Nome stabile sul filo degli eventi. Non è la mappatura verso il database:
    /// quella vive nella tabella di conversione dell'infrastruttura.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rent => "rent",
            Self::Deposit => "deposit",
            Self::UtilityRecharge => "utility_recharge",
            Self::LateFee => "late_fee",
            Self::Maintenance => "maintenance",
            Self::Adjustment => "adjustment",
            Self::Other => "other",
        }
    }
}

/// Stato di un addebito.
///
/// **Quattro valori su sette esistono nell'enum del database e l'applicazione non
/// li scrive mai**, e le ragioni sono diverse fra loro:
///
/// - `Draft` perché un addebito nasce **emesso**. Il flusso in due passi non
///   reggerebbe: la policy `charges_select` è
///   `org_id = current_org_id() AND can_access_lease(lease_id)` e **non guarda lo
///   stato**, quindi una bozza comparirebbe nell'elenco di chi deve pagarla
///   esattamente come un addebito emesso; ed emettere non manda niente a nessuno,
///   perché la porta `Notifier` è dichiarata e senza adattatore. Fra i due stati
///   non resterebbe alcuna differenza osservabile. Un addebito sbagliato **si
///   annulla**;
/// - `Overdue` perché lo stato scaduto **si deriva** e non si memorizza: vedi
///   [`Charge::effective_status`];
/// - `PartiallyPaid` e `Paid` perché li produrrà la riconciliazione del modulo
///   dei pagamenti, che è il successivo. In questa fetta non esistono righe in
///   `app.payments`, quindi `paid_amount` vale sempre `0.00` e
///   `outstanding_amount` coincide sempre con `amount`. Non è un difetto della
///   vista: è l'assenza dei pagamenti.
///
/// Restano fra le varianti perché una riga scritta a mano potrebbe contenerli, e
/// un valore sconosciuto in lettura deve dare errore invece di ricadere su un
/// default plausibile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargeStatus {
    /// Mai scritto dall'applicazione: vedi la nota sul tipo.
    Draft,
    Issued,
    /// Mai scritto dall'applicazione in questa fetta: lo scriverà il modulo dei
    /// pagamenti, che ha già la `GRANT UPDATE (status)` che gli serve.
    PartiallyPaid,
    /// Mai scritto dall'applicazione in questa fetta: vedi `PartiallyPaid`.
    Paid,
    /// **Mai scritto**, in questa fetta e in nessun'altra: è derivato.
    Overdue,
    Void,
    WrittenOff,
}

impl ChargeStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Issued => "issued",
            Self::PartiallyPaid => "partially_paid",
            Self::Paid => "paid",
            Self::Overdue => "overdue",
            Self::Void => "void",
            Self::WrittenOff => "written_off",
        }
    }

    /// I tre stati dell'insoluto.
    ///
    /// Coincide **esattamente** con il predicato dell'indice parziale
    /// `charges_outstanding_ix`
    /// (`status IN ('issued','partially_paid','overdue')`), ed è deliberato: lo
    /// scadenzario che filtra `status` fuori da questo elenco non userebbe
    /// l'indice. È la stessa forma di [`TicketStatus::WORKING`], macro compresa,
    /// e questa costante è l'unico posto dove i tre valori sono elencati nel
    /// dominio.
    ///
    /// **L'elenco contiene `overdue`, che l'applicazione non scrive.** L'indice
    /// resta corretto — copre più di quanto serva — ma chi lo legge deve sapere
    /// che il terzo valore è lì per righe che non esistono.
    ///
    /// [`TicketStatus::WORKING`]: crate::ticket::TicketStatus::WORKING
    pub const OUTSTANDING: [Self; 3] = [Self::Issued, Self::PartiallyPaid, Self::Overdue];

    /// `true` se l'addebito è ancora da incassare, in tutto o in parte.
    #[must_use]
    pub const fn is_outstanding(self) -> bool {
        matches!(self, Self::Issued | Self::PartiallyPaid | Self::Overdue)
    }

    /// I due stati in cui **la pretesa è caduta**: non è dovuto niente, a
    /// prescindere da quanto sia stato pagato.
    ///
    /// Coincide con l'elenco che `app.v_charge_balances` azzera dalla migration
    /// `0024`, e questa costante è l'unico posto dove i due valori sono elencati
    /// nel dominio. Sono gli unici stati in cui **lo stato è l'autorità** sul
    /// residuo: `paid` non è qui, e la ragione è ADR-032 — azzerare dove
    /// l'aritmetica è già significativa farebbe scavalcare i pagamenti dallo
    /// stato memorizzato, nascondendo esattamente lo scarto che la contabilità
    /// esiste per scoprire.
    pub const CLAIM_DROPPED: [Self; 2] = [Self::Void, Self::WrittenOff];

    /// `true` se la pretesa è caduta: vedi [`ChargeStatus::CLAIM_DROPPED`].
    #[must_use]
    pub const fn is_claim_dropped(self) -> bool {
        matches!(self, Self::Void | Self::WrittenOff)
    }

    /// Uno stato derivato non è memorizzabile: serve alla lettura e alla risposta
    /// HTTP, non alla scrittura.
    ///
    /// È ciò che rende `?status=overdue` un `422` e non una pagina vuota:
    /// restituire zero righe sarebbe il modo peggiore di dire che quello stato in
    /// database non c'è.
    #[must_use]
    pub const fn is_derived(self) -> bool {
        matches!(self, Self::Overdue)
    }
}

/// Dati necessari a emettere un addebito.
///
/// **Non c'è `currency`** per la ragione detta in
/// [`NewUtilityBill`](super::NewUtilityBill), e **non c'è `status`**: un addebito
/// nasce emesso e non c'è altro modo di crearne uno.
#[derive(Debug, Clone)]
pub struct NewCharge {
    pub id: ChargeId,
    pub org_id: OrgId,
    /// Decide **chi lo vede**: `charges_select` chiede
    /// `can_access_lease(lease_id)`.
    ///
    /// La `WITH CHECK` di `charges_insert` invece **non** lo guarda — nomina solo
    /// l'organizzazione — e su `app.charges` non c'è un trigger
    /// `assert_org_matches_*`. Ne segue che il service layer **deve** rileggere
    /// il contratto dal proprio repository prima di inserire, o si scrive una
    /// riga che nessuno potrà mai rileggere né cancellare.
    pub lease_id: LeaseId,
    pub charge_type: ChargeType,
    pub description: Option<String>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub amount: Decimal,
    /// Obbligatoria anche a schema: un addebito senza scadenza non è esigibile e
    /// non entrerebbe mai in nessuno scadenzario.
    pub due_date: NaiveDate,
    /// Valorizzato **solo** dal riaddebito di una bolletta, che è l'unica via per
    /// cui una bolletta e un addebito si legano.
    pub utility_bill_id: Option<UtilityBillId>,
    pub notes: Option<String>,
}

/// Addebito verso l'inquilino: una **pretesa verso una persona**.
///
/// **Non esiste un `apply`, e non è una dimenticanza.** Riscrivere l'importo di
/// un addebito dopo averlo comunicato non è una correzione, è cambiare le carte
/// in tavola: la traccia resta in `audit_log`, ma la riga che l'inquilino rilegge
/// direbbe un'altra cifra senza che nulla, nella riga stessa, ricordi la prima.
/// Un addebito sbagliato **si annulla e si riemette**.
///
/// La proprietà non è affidata a questa scelta soltanto. La `WITH CHECK` di
/// `charges_update` nomina il solo `org_id` e **lascia libera ogni altra
/// colonna** — una policy non può confrontare la riga nuova con quella vecchia,
/// che è la constatazione di ADR-021 e ADR-026 — quindi la migration `0023`
/// revoca l'`UPDATE` e la riconcede sulla **sola colonna `status`**. Il modulo
/// dei pagamenti non avrà bisogno di altro: riconciliare un incasso scrive
/// `status`, che è concessa.
///
/// **Conseguenza da dire ad alta voce: non esiste un `PATCH` sugli addebiti.**
/// Prorogare una scadenza — «ti do una settimana in più» — oggi si fa annullando
/// e riemettendo. È scomodo ed è voluto: è il punto in cui ci si ferma a
/// chiedersi se `due_date` debba davvero essere scrivibile.
///
/// **Fuori perimetro in questa fetta**: `generation_key` (la generazione
/// ricorrente dei canoni richiede uno scheduler, che ADR-019 ha deliberatamente
/// evitato di introdurre; l'indice unico `charges_generation_uk` è ciò che rende
/// additivo aggiungerla) e `document_id`, per la stessa ragione della bolletta —
/// gli allegati passano dal riferimento polimorfico dei documenti.
#[derive(Debug, Clone)]
pub struct Charge {
    id: ChargeId,
    org_id: OrgId,
    lease_id: LeaseId,
    charge_type: ChargeType,
    status: ChargeStatus,
    description: Option<String>,
    period_start: Option<NaiveDate>,
    period_end: Option<NaiveDate>,
    amount: Money,
    currency: Currency,
    due_date: NaiveDate,
    issued_at: Option<DateTime<Utc>>,
    utility_bill_id: Option<UtilityBillId>,
    notes: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

impl Charge {
    /// Emette un addebito. È l'**unico** costruttore, e nasce `issued`.
    ///
    /// `issued_at` è valorizzato qui e non altrove: `charges_issued_ck` è
    /// `status = 'draft' OR issued_at IS NOT NULL`, e poiché `draft` non si
    /// scrive mai, ogni addebito **deve** nascere con la data. Dimenticarla non
    /// dà una riga strana, dà un rifiuto all'inserimento — ed è la ragione per
    /// cui la decisione «niente bozza» è quasi gratuita.
    pub fn issue(parts: NewCharge, now: DateTime<Utc>) -> DomainResult<Self> {
        let NewCharge {
            id,
            org_id,
            lease_id,
            charge_type,
            description,
            period_start,
            period_end,
            amount,
            due_date,
            utility_bill_id,
            notes,
        } = parts;

        // `>=` e non `>`, esattamente come `charges_period_ck`.
        if let (Some(start), Some(end)) = (period_start, period_end)
            && end < start
        {
            return Err(DomainError::validation(
                "period_end",
                "cannot precede the start of the period",
            ));
        }

        let currency = Currency::EUR;
        let amount = Money::non_negative(amount, currency)
            .map_err(|_| DomainError::validation("amount", "must be a non-negative amount"))?;

        Ok(Self {
            id,
            org_id,
            lease_id,
            charge_type,
            status: ChargeStatus::Issued,
            description: text::optional("description", description, 1000)?,
            period_start,
            period_end,
            amount,
            currency,
            due_date,
            issued_at: Some(now),
            utility_bill_id,
            notes: text::optional("notes", notes, 4000)?,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        })
    }

    /// Annulla l'addebito: la richiesta è caduta.
    ///
    /// L'addebito annullato **resta visibile** — `status = 'void'`, `deleted_at`
    /// intatto — ed è l'asimmetria deliberata con la bolletta, che annullata
    /// sparisce. La ragione è che qualcuno si è visto chiedere quella somma e
    /// deve poter vedere che la richiesta non c'è più; una bolletta inserita male
    /// invece è un documento nostro, e non c'è nessuno a cui dirlo.
    ///
    /// **Ammesso dal solo `issued`, e resta così.** La domanda che
    /// `partially_paid` porrebbe — «che ne è del denaro già incassato» — non ha
    /// risposta qui: annullare significa che la richiesta non è mai esistita, e
    /// del denaro ricevuto contro una pretesa mai esistita questo modello non sa
    /// dire niente. Chi si trova in quel caso **storna**, che dice il vero:
    /// l'incassato resta incassato e si rinuncia al resto — vedi
    /// [`Self::write_off`], che da `partially_paid` è invece ammesso.
    ///
    /// L'asimmetria fra i due è deliberata e non una svista da appianare.
    pub fn void(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if !matches!(self.status, ChargeStatus::Issued) {
            return Err(DomainError::Invariant(Invariant::ChargeNotVoidable));
        }
        self.status = ChargeStatus::Void;
        self.updated_at = now;
        Ok(())
    }

    /// Rinuncia all'incasso: l'addebito era dovuto e non si pretende più.
    ///
    /// Diverso dall'annullamento, e la distinzione conta nei report: `void` dice
    /// «non doveva essere chiesto», `written_off` dice «era dovuto e ci
    /// rinunciamo». Entrambi escono dall'insoluto, ma solo il secondo è una
    /// perdita.
    ///
    /// # Ammesso anche da `partially_paid`, e senza quello il numero mentirebbe
    ///
    /// Un addebito su cui è entrato del denaro **è** stornabile, ed è anzi il caso
    /// in cui una perdita si materializza per davvero: l'inquilino che versa
    /// trecento euro su ottocento e sparisce. Limitare lo storno a `issued` — come
    /// faceva la fetta contabile, quando i pagamenti non esistevano — renderebbe
    /// `written_off_total` un numero che conta **metà** delle perdite senza dirlo,
    /// e un numero che dice meno del proprio nome è peggio di un numero assente.
    ///
    /// La domanda che l'annullamento non sa reggere qui non si pone: il denaro
    /// incassato **resta incassato**, e si rinuncia soltanto al residuo. È per
    /// questo che `void` resta al solo `issued` e questo no.
    ///
    /// **La perdita è quindi il residuo e non l'importo**, e non si legge da
    /// `outstanding_amount`: la migration `0024` lo azzera proprio su
    /// `written_off`. La somma la fa `app.v_owner_dashboard` come
    /// `amount - paid_amount`.
    ///
    /// Nessuna migration: la `0023` concede già l'`UPDATE` sulla sola `status`, e
    /// `partially_paid` è un valore che quella colonna già porta.
    pub fn write_off(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if !matches!(
            self.status,
            ChargeStatus::Issued | ChargeStatus::PartiallyPaid
        ) {
            return Err(DomainError::Invariant(Invariant::ChargeNotWriteOffable));
        }
        self.status = ChargeStatus::WrittenOff;
        self.updated_at = now;
        Ok(())
    }

    /// Ricalcola lo stato dai pagamenti riusciti.
    ///
    /// È la **prima scrittura** che la `GRANT UPDATE (status)` della migration
    /// `0023` rende possibile, ed è l'unica di cui il modulo dei pagamenti ha
    /// bisogno: la fetta contabile l'aveva posata apposta, ed è la ragione per cui
    /// i pagamenti non portano con sé alcuna migration sugli addebiti.
    ///
    /// Lo stato si **ricalcola**, non si dichiara: chi chiama passa la somma dei
    /// pagamenti che contano nel saldo — gli stessi due che conta
    /// `app.v_charge_balances`, mai gli `authorized` — e le tre soglie sono quelle
    /// dell'aritmetica, non di un comando.
    ///
    /// ```text
    ///   paid <= 0        →  issued
    ///   0 < paid < amount →  partially_paid
    ///   paid >= amount    →  paid
    /// ```
    ///
    /// **Restituisce `false` quando non c'è niente da cambiare**, così un secondo
    /// pagamento su un addebito già coperto non produce `UPDATE`, evento e riga di
    /// audit per nulla. È la stessa forma di `Property::apply`.
    ///
    /// **Da `void` e `written_off` è rifiutato.** La pretesa è caduta, e incassare
    /// su una pretesa caduta è un'incoerenza, non un caso d'uso — è lo stesso
    /// elenco che la vista azzera, e la coincidenza non è casuale: dove lo stato è
    /// l'autorità sul residuo, un pagamento non ha un residuo da toccare.
    ///
    /// **Il sovrapagamento non è impedito.** `paid > amount` porta semplicemente a
    /// `paid`, e nella vista `outstanding_amount` diventa **negativo**, che è un
    /// credito verso l'inquilino — rappresentabile, perché `app.money_amount`
    /// ammette i negativi. Un controllo applicativo sarebbe soggetto a race
    /// condition fra due pagamenti concorrenti, e vale il criterio di ADR-015: quel
    /// genere di invariante sta nel database o non sta da nessuna parte. Lo schema
    /// non ha il vincolo, quindi non sta da nessuna parte, ed è registrato invece
    /// che scoperto dopo.
    pub fn apply_payment(
        &mut self,
        paid_amount: Decimal,
        now: DateTime<Utc>,
    ) -> DomainResult<bool> {
        if self.status.is_claim_dropped() {
            return Err(DomainError::Invariant(Invariant::ChargeNotPayable));
        }

        let recalculated = if paid_amount <= Decimal::ZERO {
            ChargeStatus::Issued
        } else if paid_amount < self.amount.amount() {
            ChargeStatus::PartiallyPaid
        } else {
            ChargeStatus::Paid
        };

        if recalculated == self.status {
            return Ok(false);
        }
        self.status = recalculated;
        self.updated_at = now;
        Ok(true)
    }

    /// `true` se l'addebito è scaduto e non ancora coperto.
    ///
    /// **Stessa formula di `v_charge_balances.is_past_due`**, deliberatamente:
    /// `status NOT IN ('void','written_off') AND due_date < CURRENT_DATE
    /// AND amount > COALESCE(paid_amount, 0)`. È la
    /// coppia di `app.lease_in_force` e [`Lease::is_in_force`], con lo stesso
    /// obbligo — il dominio e la vista devono rispondere la stessa cosa,
    /// altrimenti la scheda dell'addebito e la dashboard si contraddicono.
    ///
    /// **`paid_amount` è un parametro e non un campo**, perché non è un dato
    /// dell'aggregato: vive in `app.payments` e lo calcola la vista. Passarlo
    /// esplicitamente è ciò che rende questa funzione *la stessa* formula invece
    /// di una che le somiglia. In questa fetta il chiamante passa sempre zero,
    /// perché non esistono pagamenti.
    ///
    /// Il confronto è fra `Decimal` e non fra [`Money`], perché `Money` non
    /// implementa `PartialOrd` apposta (ADR-002): confrontare 10 EUR con 10 USD
    /// non è un'operazione di dominio. Qui è lecito **solo** perché la valuta è
    /// una sola — la vista fa lo stesso confronto senza poterlo verificare, ed è
    /// proprio la ragione per cui la valuta resta una sola.
    ///
    /// **Guarda lo stato per i soli [`CLAIM_DROPPED`]**, e la migration `0024` è
    /// la ragione per cui lo fa. Fino a quella migration la vista non guardava
    /// `status` affatto, e un addebito annullato con una scadenza nel passato
    /// risultava scaduto in SQL: la formula del dominio ripeteva quell'asimmetria
    /// e a escludere l'annullato era il solo [`Charge::effective_status`]. Ora la
    /// vista azzera per `void` e `written_off`, e la formula la segue — se non la
    /// seguisse, questa funzione smetterebbe di essere *la stessa* formula, che è
    /// l'unica ragione per cui esiste.
    ///
    /// `paid` e `draft` restano fuori dal controllo, qui come nella vista: lì il
    /// residuo lo dice l'aritmetica.
    ///
    /// [`CLAIM_DROPPED`]: ChargeStatus::CLAIM_DROPPED
    /// [`Lease::is_in_force`]: crate::lease::Lease::is_in_force
    #[must_use]
    pub fn is_past_due(&self, today: NaiveDate, paid_amount: Decimal) -> bool {
        !self.status.is_claim_dropped()
            && self.due_date < today
            && self.amount.amount() > paid_amount
    }

    /// Stato effettivo alla data indicata.
    ///
    /// Lo stato memorizzato cambia solo su azioni esplicite; la scadenza è un
    /// fatto del calendario e si calcola. Nessuno scrive `overdue`, così non
    /// esiste la finestra in cui il database dice una cosa e il calendario
    /// un'altra — è ADR-019, che ha già stabilito la regola per i contratti dopo
    /// averla pagata: lo stato memorizzato invecchia da solo perché nessuno lo
    /// aggiorna, e la dashboard finisce per contare cose finite mesi prima.
    ///
    /// La derivazione si applica ai soli stati dell'insoluto: un addebito
    /// annullato o stornato con una scadenza nel passato **non** è scaduto, per
    /// quanto la formula della vista, che non guarda lo stato, dica di sì.
    #[must_use]
    pub fn effective_status(&self, today: NaiveDate, paid_amount: Decimal) -> ChargeStatus {
        if self.status.is_outstanding() && self.is_past_due(today, paid_amount) {
            return ChargeStatus::Overdue;
        }
        self.status
    }

    #[must_use]
    pub const fn id(&self) -> ChargeId {
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
    pub const fn charge_type(&self) -> ChargeType {
        self.charge_type
    }
    #[must_use]
    pub const fn status(&self) -> ChargeStatus {
        self.status
    }
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    #[must_use]
    pub const fn period_start(&self) -> Option<NaiveDate> {
        self.period_start
    }
    #[must_use]
    pub const fn period_end(&self) -> Option<NaiveDate> {
        self.period_end
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
    pub const fn due_date(&self) -> NaiveDate {
        self.due_date
    }
    #[must_use]
    pub const fn issued_at(&self) -> Option<DateTime<Utc>> {
        self.issued_at
    }
    #[must_use]
    pub const fn utility_bill_id(&self) -> Option<UtilityBillId> {
        self.utility_bill_id
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
    pub const fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }

    /// Ricostruzione da persistenza: non riesegue le invarianti.
    #[must_use]
    pub fn from_persistence(parts: ChargeParts) -> Self {
        let ChargeParts {
            id,
            org_id,
            lease_id,
            charge_type,
            status,
            description,
            period_start,
            period_end,
            amount,
            currency,
            due_date,
            issued_at,
            utility_bill_id,
            notes,
            created_at,
            updated_at,
            deleted_at,
        } = parts;
        Self {
            id,
            org_id,
            lease_id,
            charge_type,
            status,
            description,
            period_start,
            period_end,
            amount,
            currency,
            due_date,
            issued_at,
            utility_bill_id,
            notes,
            created_at,
            updated_at,
            deleted_at,
        }
    }
}

/// Dati grezzi di un addebito letto dalla persistenza.
#[derive(Debug, Clone)]
pub struct ChargeParts {
    pub id: ChargeId,
    pub org_id: OrgId,
    pub lease_id: LeaseId,
    pub charge_type: ChargeType,
    pub status: ChargeStatus,
    pub description: Option<String>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub amount: Money,
    pub currency: Currency,
    pub due_date: NaiveDate,
    pub issued_at: Option<DateTime<Utc>>,
    pub utility_bill_id: Option<UtilityBillId>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Un addebito con il suo saldo, letto da `app.v_charge_balances`.
///
/// Esiste come tipo perché il saldo **non è un dato dell'aggregato**: vive in
/// `app.payments` e lo calcola la vista. Copiare la `LEFT JOIN LATERAL` dentro il
/// repository sarebbe una seconda definizione di «pagato», e divergerebbe alla
/// prima modifica — il giorno in cui il modulo dei pagamenti aggiungerà uno stato
/// all'array `('succeeded','partially_refunded')`, deve cambiare in un posto solo.
///
/// **In questa fetta `paid_amount` vale sempre `0.00` e `outstanding_amount`
/// coincide sempre con `amount`.** Non è un difetto della vista: è l'assenza di
/// righe in `app.payments`, e va detto nell'API perché chi legge la risposta
/// altrimenti pensa che il campo sia rotto.
#[derive(Debug, Clone)]
pub struct ChargeBalance {
    pub charge: Charge,
    pub paid_amount: Money,
    pub outstanding_amount: Money,
    /// Come l'ha calcolato la vista, con il `CURRENT_DATE` **del database**.
    ///
    /// Si porta dietro il valore invece di ricalcolarlo perché è la vista la
    /// fonte: `Charge::is_past_due` esiste per essere verificabile senza
    /// database, non per sostituirla. Nota che il fuso è quello della sessione
    /// PostgreSQL, non quello di chi guarda — con il container in UTC e un utente
    /// in Italia lo scarto è di una o due ore, una volta al giorno, sul solo
    /// giorno della scadenza. Correggerlo significa decidere di chi è il fuso
    /// orario di riferimento, che è una domanda di prodotto.
    pub is_past_due: bool,
}

impl ChargeBalance {
    /// Stato effettivo secondo il saldo letto dalla vista.
    ///
    /// Usa `is_past_due` **della vista** e non ricalcola la data: le due
    /// definizioni devono coincidere, e in caso di scarto di fuso orario vince
    /// quella che ha prodotto il numero mostrato accanto.
    #[must_use]
    pub const fn effective_status(&self) -> ChargeStatus {
        if self.charge.status().is_outstanding() && self.is_past_due {
            return ChargeStatus::Overdue;
        }
        self.charge.status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn date(raw: &str) -> NaiveDate {
        NaiveDate::from_str(raw).expect("data di test valida")
    }

    fn dec(raw: &str) -> Decimal {
        Decimal::from_str(raw).expect("decimale di test valido")
    }

    fn new_charge() -> NewCharge {
        NewCharge {
            id: ChargeId::generate(),
            org_id: OrgId::generate(),
            lease_id: LeaseId::generate(),
            charge_type: ChargeType::Rent,
            description: Some("Canone di settembre".to_owned()),
            period_start: Some(date("2026-09-01")),
            period_end: Some(date("2026-09-30")),
            amount: dec("800.00"),
            due_date: date("2026-09-05"),
            utility_bill_id: None,
            notes: None,
        }
    }

    fn charge() -> Charge {
        Charge::issue(new_charge(), Utc::now()).expect("addebito valido")
    }

    /// La decisione 1 in forma di test: non c'è modo di ottenere una bozza,
    /// perché non c'è un costruttore che la produca.
    #[test]
    fn a_charge_is_born_issued_with_its_date() {
        let now = Utc::now();
        let charge = Charge::issue(new_charge(), now).expect("valido");
        assert_eq!(charge.status(), ChargeStatus::Issued);
        assert_eq!(
            charge.issued_at(),
            Some(now),
            "`charges_issued_ck` rifiuterebbe l'inserimento senza la data"
        );
        assert_eq!(charge.deleted_at(), None);
        assert_eq!(charge.currency(), Currency::EUR);
    }

    #[test]
    fn the_transition_table_is_complete() {
        // void dal solo `issued`, write_off anche da `partially_paid`; nessuno dei
        // due è ripetibile.
        let mut voided = charge();
        voided.void(Utc::now()).expect("annullabile");
        assert_eq!(voided.status(), ChargeStatus::Void);
        assert!(voided.void(Utc::now()).is_err(), "non è ripetibile");
        assert!(
            voided.write_off(Utc::now()).is_err(),
            "un addebito annullato non si storna"
        );

        let mut written_off = charge();
        written_off.write_off(Utc::now()).expect("stornabile");
        assert_eq!(written_off.status(), ChargeStatus::WrittenOff);
        assert!(written_off.write_off(Utc::now()).is_err());
        assert!(
            written_off.void(Utc::now()).is_err(),
            "un addebito stornato non si annulla"
        );
    }

    /// Il caso per cui il nono numero della dashboard esiste: trecento euro su
    /// ottocento, e poi l'inquilino sparisce.
    ///
    /// La perdita è **cinquecento**, non ottocento — gli altri trecento sono
    /// entrati — e l'asimmetria con l'annullamento è la parte che va fissata: da
    /// `partially_paid` si storna e non si annulla.
    #[test]
    fn a_partially_paid_charge_is_written_off_but_never_voided() {
        let now = Utc::now();

        let mut partial = charge(); // 800.00, issued
        assert!(partial.apply_payment(dec("300.00"), now).expect("ammesso"));
        assert_eq!(partial.status(), ChargeStatus::PartiallyPaid);

        let mut voidable = partial.clone();
        assert!(
            voidable.void(now).is_err(),
            "annullare direbbe che la pretesa non è mai esistita, e trecento euro sono entrati"
        );
        assert_eq!(voidable.status(), ChargeStatus::PartiallyPaid);

        partial
            .write_off(now)
            .expect("stornabile da partially_paid");
        assert_eq!(partial.status(), ChargeStatus::WrittenOff);
        assert!(partial.write_off(now).is_err(), "non è ripetibile");

        // La perdita è il residuo: l'aritmetica che la dashboard somma come
        // `amount - paid_amount`, e che `outstanding_amount` non può dare perché
        // la `0024` lo azzera proprio su questo stato.
        assert_eq!(
            partial.amount().amount() - dec("300.00"),
            dec("500.00"),
            "la perdita è il residuo, non l'importo"
        );
    }

    /// Gli altri tre stati restano fuori, e ognuno per una ragione sua: da `paid`
    /// non c'è niente a cui rinunciare, e `void` e `written_off` sono terminali.
    #[test]
    fn writing_off_is_refused_from_every_other_state() {
        let now = Utc::now();

        let mut paid = charge();
        paid.apply_payment(dec("800.00"), now).expect("ammesso");
        assert_eq!(paid.status(), ChargeStatus::Paid);
        assert!(
            paid.write_off(now).is_err(),
            "un addebito coperto non ha un residuo a cui rinunciare"
        );
    }

    /// L'annullamento **non** cancella la riga: chi si è visto chiedere quella
    /// somma deve poter vedere che la richiesta è caduta. È l'asimmetria con la
    /// bolletta, che annullata sparisce.
    #[test]
    fn voiding_a_charge_leaves_it_visible() {
        let mut charge = charge();
        charge.void(Utc::now()).expect("annullato");
        assert_eq!(charge.status(), ChargeStatus::Void);
        assert_eq!(
            charge.deleted_at(),
            None,
            "l'inquilino deve continuare a vederlo, marcato come annullato"
        );
    }

    /// La formula deve essere **la stessa** di `v_charge_balances.is_past_due`:
    /// `status NOT IN ('void','written_off') AND due_date < CURRENT_DATE
    /// AND amount > COALESCE(paid_amount, 0)`.
    #[test]
    fn is_past_due_repeats_the_formula_of_the_view() {
        let charge = charge(); // scade il 2026-09-05, 800.00

        assert!(!charge.is_past_due(date("2026-09-04"), Decimal::ZERO));
        assert!(
            !charge.is_past_due(date("2026-09-05"), Decimal::ZERO),
            "il giorno della scadenza non è ancora scaduto: il confronto è `<`"
        );
        assert!(charge.is_past_due(date("2026-09-06"), Decimal::ZERO));

        // Coperto per intero: non è più scaduto, per quanto la data sia passata.
        assert!(!charge.is_past_due(date("2026-09-06"), dec("800.00")));
        // Coperto in parte: lo è ancora.
        assert!(charge.is_past_due(date("2026-09-06"), dec("799.99")));
    }

    /// Dalla migration `0024` la vista guarda lo stato per i due terminali, e la
    /// formula del dominio la segue: un addebito annullato o stornato **non** è
    /// scaduto, né qui né in SQL. Prima della `0024` questo test asseriva
    /// l'opposto, e la sola difesa era `effective_status`.
    #[test]
    fn a_dropped_claim_is_never_past_due_here_either() {
        let mut voided = charge();
        voided.void(Utc::now()).expect("annullato");
        assert!(
            !voided.is_past_due(date("2026-12-01"), Decimal::ZERO),
            "la pretesa è caduta: non è dovuto niente, e la vista dice lo stesso"
        );
        assert_eq!(
            voided.effective_status(date("2026-12-01"), Decimal::ZERO),
            ChargeStatus::Void
        );

        let mut written_off = charge();
        written_off.write_off(Utc::now()).expect("stornato");
        assert!(!written_off.is_past_due(date("2026-12-01"), Decimal::ZERO));

        // `paid` e `draft` **non** sono nell'elenco: lì il residuo lo dice
        // l'aritmetica, e la coincidenza con la vista si vede da questo lato
        // perché l'elenco è uno solo.
        assert_eq!(
            ChargeStatus::CLAIM_DROPPED,
            [ChargeStatus::Void, ChargeStatus::WrittenOff],
            "l'elenco che azzera il residuo è quello della migration 0024"
        );
        assert!(!ChargeStatus::Paid.is_claim_dropped());
        assert!(!ChargeStatus::Draft.is_claim_dropped());
    }

    /// Le tre soglie del ricalcolo, e il fatto che sia un **ricalcolo**: lo stato
    /// non è un comando, è la conseguenza di quanto è entrato.
    #[test]
    fn the_state_is_recalculated_from_the_payments_across_three_thresholds() {
        let now = Utc::now();

        let mut running = charge(); // 800.00, issued
        assert!(
            !running.apply_payment(Decimal::ZERO, now).expect("ammesso"),
            "senza pagamenti non c'è niente da cambiare, e niente da scrivere"
        );
        assert_eq!(running.status(), ChargeStatus::Issued);

        assert!(running.apply_payment(dec("300.00"), now).expect("ammesso"));
        assert_eq!(running.status(), ChargeStatus::PartiallyPaid);

        assert!(running.apply_payment(dec("800.00"), now).expect("ammesso"));
        assert_eq!(running.status(), ChargeStatus::Paid);

        assert!(
            !running.apply_payment(dec("800.00"), now).expect("ammesso"),
            "un secondo pagamento che non muove lo stato non produce scrittura"
        );

        // Il sovrapagamento è possibile e non lo si impedisce: porta a `paid`, e il
        // credito lo mostra la vista con un `outstanding_amount` negativo.
        let mut overpaid = charge();
        assert!(overpaid.apply_payment(dec("900.00"), now).expect("ammesso"));
        assert_eq!(overpaid.status(), ChargeStatus::Paid);

        // Il confine è `>=`, non `>`: coperto per intero è pagato.
        let mut exact = charge();
        assert!(exact.apply_payment(dec("800.00"), now).expect("ammesso"));
        assert_eq!(exact.status(), ChargeStatus::Paid);

        let mut almost = charge();
        assert!(almost.apply_payment(dec("799.99"), now).expect("ammesso"));
        assert_eq!(almost.status(), ChargeStatus::PartiallyPaid);
    }

    /// Incassare su una pretesa caduta è un'incoerenza, non un caso d'uso — ed è
    /// lo stesso elenco che la vista azzera.
    #[test]
    fn a_dropped_claim_refuses_a_payment() {
        let now = Utc::now();

        let mut voided = charge();
        voided.void(now).expect("annullato");
        assert!(voided.apply_payment(dec("100.00"), now).is_err());
        assert_eq!(voided.status(), ChargeStatus::Void);

        let mut written_off = charge();
        written_off.write_off(now).expect("stornato");
        assert!(written_off.apply_payment(dec("100.00"), now).is_err());
        assert_eq!(written_off.status(), ChargeStatus::WrittenOff);
    }

    /// Il cuore della decisione 2: lo stato memorizzato non cambia mai da sé.
    #[test]
    fn overdue_is_derived_and_never_stored() {
        let charge = charge();
        assert_eq!(
            charge.effective_status(date("2026-09-01"), Decimal::ZERO),
            ChargeStatus::Issued
        );
        assert_eq!(
            charge.effective_status(date("2026-10-01"), Decimal::ZERO),
            ChargeStatus::Overdue
        );
        assert_eq!(
            charge.status(),
            ChargeStatus::Issued,
            "lo stato memorizzato non è cambiato: è il punto della decisione"
        );
    }

    #[test]
    fn a_written_off_charge_does_not_become_overdue() {
        let mut charge = charge();
        charge.write_off(Utc::now()).expect("stornato");
        assert_eq!(
            charge.effective_status(date("2027-01-01"), Decimal::ZERO),
            ChargeStatus::WrittenOff
        );
    }

    /// La costante deve restare allineata al predicato di
    /// `charges_outstanding_ix`: se qualcuno aggiungesse uno stato dell'insoluto
    /// senza toccare l'indice, lo scadenzario smetterebbe di usarlo senza che
    /// nulla fallisca.
    #[test]
    fn the_outstanding_set_and_the_predicate_agree() {
        for status in ChargeStatus::OUTSTANDING {
            assert!(status.is_outstanding(), "{status:?}");
        }
        for status in [
            ChargeStatus::Draft,
            ChargeStatus::Paid,
            ChargeStatus::Void,
            ChargeStatus::WrittenOff,
        ] {
            assert!(!status.is_outstanding(), "{status:?}");
        }
        // `overdue` è nell'elenco dell'indice pur non essendo mai scritto:
        // l'indice copre più di quanto serva, e va saputo.
        assert!(ChargeStatus::OUTSTANDING.contains(&ChargeStatus::Overdue));
        assert!(ChargeStatus::Overdue.is_derived());
        assert!(!ChargeStatus::Issued.is_derived());
    }

    #[test]
    fn amounts_cannot_be_negative_and_the_period_cannot_run_backwards() {
        let mut input = new_charge();
        input.amount = dec("-1.00");
        assert!(Charge::issue(input, Utc::now()).is_err());

        let mut input = new_charge();
        input.period_start = Some(date("2026-09-30"));
        input.period_end = Some(date("2026-09-01"));
        assert!(Charge::issue(input, Utc::now()).is_err());
    }

    /// In questa fetta la vista risponde sempre così, e l'API deve dirlo: non è
    /// un difetto: è l'assenza di righe in `app.payments`.
    #[test]
    fn without_payments_the_balance_is_the_whole_amount() {
        let charge = charge();
        let balance = ChargeBalance {
            paid_amount: Money::non_negative(Decimal::ZERO, Currency::EUR).expect("valido"),
            outstanding_amount: charge.amount(),
            is_past_due: false,
            charge,
        };
        assert_eq!(balance.paid_amount.amount(), Decimal::ZERO);
        assert_eq!(
            balance.outstanding_amount.amount(),
            balance.charge.amount().amount()
        );
        assert_eq!(balance.effective_status(), ChargeStatus::Issued);
    }

    /// Il saldo letto dalla vista decide, e non si ricalcola la data: se le due
    /// definizioni divergessero per un fuso orario, vince quella che ha prodotto
    /// il numero mostrato accanto.
    #[test]
    fn the_balance_derives_the_effective_state_from_the_view() {
        let issued = charge();
        let amount = issued.amount();
        let balance = ChargeBalance {
            paid_amount: Money::non_negative(Decimal::ZERO, Currency::EUR).expect("valido"),
            outstanding_amount: amount,
            is_past_due: true,
            charge: issued,
        };
        assert_eq!(balance.effective_status(), ChargeStatus::Overdue);

        let mut voided = charge();
        voided.void(Utc::now()).expect("annullato");
        let balance = ChargeBalance {
            paid_amount: Money::non_negative(Decimal::ZERO, Currency::EUR).expect("valido"),
            outstanding_amount: voided.amount(),
            is_past_due: true,
            charge: voided,
        };
        assert_eq!(balance.effective_status(), ChargeStatus::Void);
    }
}
