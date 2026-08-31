use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;

use super::value_objects::{Consumption, InvoiceNumber, SupplierName, SupplierVat};
use crate::error::{DomainError, DomainResult, Invariant};
use crate::property::Patch;
use crate::shared::{
    Currency, LeaseId, Money, OrgId, PropertyId, RoomId, UserId, UtilityBillId, text,
};

/// Tipo di utenza. Rispecchia `app.utility_type` per intero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtilityType {
    Electricity,
    Gas,
    Water,
    Heating,
    Waste,
    Internet,
    CondoFees,
    Other,
}

impl UtilityType {
    /// Nome stabile sul filo degli eventi. Non è la mappatura verso il database:
    /// quella vive nella tabella di conversione dell'infrastruttura, e tenerle
    /// separate evita che una rinomina dell'una cambi in silenzio l'altra.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Electricity => "electricity",
            Self::Gas => "gas",
            Self::Water => "water",
            Self::Heating => "heating",
            Self::Waste => "waste",
            Self::Internet => "internet",
            Self::CondoFees => "condo_fees",
            Self::Other => "other",
        }
    }
}

/// Stato di una bolletta.
///
/// Il ciclo che l'applicazione percorre è `draft → validated → allocated`, con
/// `void` come uscita e la correzione che **riporta indietro** da `validated` a
/// `draft` (vedi [`UtilityBill::apply`]).
///
/// **Tre valori esistono nell'enum del database e l'applicazione non li scrive
/// mai**, ciascuno per una ragione diversa e nessuna delle tre è una
/// dimenticanza:
///
/// - `PendingReview` appartiene all'OCR, che è il modulo successivo: qui la
///   bolletta la digita una persona e non c'è niente da rivedere;
/// - `Paid` significherebbe «il proprietario ha pagato il fornitore», e non è
///   modellato affatto — `app.payable_type` è
///   `('charge','expense_share','settlement')` e **non comprende
///   `utility_bill`**, quindi non esiste un posto dove registrare quel
///   pagamento;
/// - `Disputed` è una contestazione al fornitore, e **manca la via di ritorno**:
///   uno stato in cui si entra e da cui non si esce è peggio di nessuno stato, e
///   disegnare l'uscita vuol dire decidere cosa succede alle bollette già
///   riaddebitate.
///
/// Restano fra le varianti perché una riga scritta a mano potrebbe contenerli, e
/// un valore sconosciuto in lettura deve dare errore invece di ricadere su un
/// default plausibile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillStatus {
    Draft,
    /// Mai scritto dall'applicazione: vedi la nota sul tipo.
    PendingReview,
    Validated,
    Allocated,
    /// Mai scritto dall'applicazione: vedi la nota sul tipo.
    Paid,
    /// Mai scritto dall'applicazione: vedi la nota sul tipo.
    Disputed,
    Void,
}

impl BillStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::PendingReview => "pending_review",
            Self::Validated => "validated",
            Self::Allocated => "allocated",
            Self::Paid => "paid",
            Self::Disputed => "disputed",
            Self::Void => "void",
        }
    }

    /// `true` finché i numeri della bolletta si possono ancora correggere.
    ///
    /// Da `allocated` in poi **no**: quei numeri sono la base di una pretesa già
    /// fatta a qualcuno, e cambiarli sotto la pretesa è precisamente ciò che non
    /// deve succedere.
    #[must_use]
    pub const fn is_editable(self) -> bool {
        matches!(self, Self::Draft | Self::Validated)
    }
}

/// Dati necessari a registrare una bolletta.
///
/// Una struct e non diciotto argomenti: fra quattro `Option<NaiveDate>` di fila e
/// tre `Option<Decimal>`, scambiarne due compilerebbe senza che il compilatore se
/// ne accorga.
///
/// **Non c'è `currency`**, e non è una svista: la valuta resta `EUR` e non è
/// dichiarabile finché `v_charge_balances` e `v_owner_dashboard.total_outstanding`
/// sommano importi **senza confrontarla**. Prenderla qui come parametro
/// significherebbe accettare un valore che il resto del sistema non sa reggere.
#[derive(Debug, Clone)]
pub struct NewUtilityBill {
    pub id: UtilityBillId,
    pub org_id: OrgId,
    pub property_id: PropertyId,
    /// La stanza a cui l'utenza si riferisce, se il caso è quello.
    ///
    /// Lo schema **non** verifica che appartenga all'immobile: il trigger
    /// `assert_org_matches_property` guarda solo l'`org_id`. Lo verifica il
    /// service layer, e deve rifarlo a ogni `PATCH`.
    pub room_id: Option<RoomId>,
    /// Il contratto a cui la bolletta è imputata.
    ///
    /// Decide **chi la vede**: `utility_bills_select` chiede staff *oppure*
    /// essere parte di questo contratto. Senza, la bolletta è visibile al solo
    /// staff.
    pub lease_id: Option<LeaseId>,
    pub utility_type: UtilityType,
    pub supplier_name: Option<SupplierName>,
    pub supplier_vat: Option<SupplierVat>,
    pub invoice_number: Option<InvoiceNumber>,
    pub issue_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub total_amount: Decimal,
    pub taxable_amount: Option<Decimal>,
    pub vat_amount: Option<Decimal>,
    pub consumption: Option<Consumption>,
    pub notes: Option<String>,
}

/// Insieme di modifiche applicabili a una bolletta.
///
/// Stessa semantica di [`PropertyChanges`](crate::property::PropertyChanges):
/// `Patch<T>` distingue «campo non menzionato» da «campo azzerato».
///
/// `property_id` **non** è modificabile e non compare qui: è l'ancoraggio
/// strutturale della riga, e una bolletta inserita sull'immobile sbagliato si
/// annulla e si reinserisce — che è possibile proprio perché annullarla libera la
/// chiave di deduplica. `currency` non compare per la ragione detta in
/// [`NewUtilityBill`].
#[derive(Debug, Clone, Default)]
pub struct UtilityBillChanges {
    pub utility_type: Option<UtilityType>,
    pub room_id: Patch<RoomId>,
    pub lease_id: Patch<LeaseId>,
    pub supplier_name: Patch<SupplierName>,
    pub supplier_vat: Patch<SupplierVat>,
    pub invoice_number: Patch<InvoiceNumber>,
    pub issue_date: Patch<NaiveDate>,
    pub due_date: Patch<NaiveDate>,
    pub period_start: Patch<NaiveDate>,
    pub period_end: Patch<NaiveDate>,
    /// `Option` e non `Patch`: a schema è `NOT NULL`, quindi si cambia e non si
    /// azzera.
    pub total_amount: Option<Decimal>,
    pub taxable_amount: Patch<Decimal>,
    pub vat_amount: Patch<Decimal>,
    /// Un solo campo per due colonne (`consumption` e `consumption_unit`): il
    /// value object le tiene insieme, e azzerarlo le azzera entrambe.
    pub consumption: Patch<Consumption>,
    pub notes: Patch<String>,
}

impl UtilityBillChanges {
    /// Nomi dei campi menzionati dal chiamante, nell'ordine di dichiarazione.
    ///
    /// Finiscono nell'evento `UtilityBillUpdated` **al posto dei valori**: chi
    /// consuma sa cosa invalidare senza che l'importo di una bolletta, o il nome
    /// del suo fornitore, viaggino su un broker.
    #[must_use]
    pub fn changed_field_names(&self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        if self.utility_type.is_some() {
            fields.push("utility_type");
        }
        if self.room_id.is_some() {
            fields.push("room_id");
        }
        if self.lease_id.is_some() {
            fields.push("lease_id");
        }
        if self.supplier_name.is_some() {
            fields.push("supplier_name");
        }
        if self.supplier_vat.is_some() {
            fields.push("supplier_vat");
        }
        if self.invoice_number.is_some() {
            fields.push("invoice_number");
        }
        if self.issue_date.is_some() {
            fields.push("issue_date");
        }
        if self.due_date.is_some() {
            fields.push("due_date");
        }
        if self.period_start.is_some() {
            fields.push("period_start");
        }
        if self.period_end.is_some() {
            fields.push("period_end");
        }
        if self.total_amount.is_some() {
            fields.push("total_amount");
        }
        if self.taxable_amount.is_some() {
            fields.push("taxable_amount");
        }
        if self.vat_amount.is_some() {
            fields.push("vat_amount");
        }
        if self.consumption.is_some() {
            fields.push("consumption");
        }
        if self.notes.is_some() {
            fields.push("notes");
        }
        fields
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changed_field_names().is_empty()
    }

    /// `true` se la modifica tocca uno dei due riferimenti che il service layer
    /// deve riverificare contro l'immobile.
    ///
    /// Lo schema non ha alcun vincolo fra `room_id`, `lease_id` e `property_id`:
    /// una stanza di un altro immobile passa senza una parola. Il controllo vive
    /// nel service layer e **va rifatto a ogni `PATCH`**, perché a differenza
    /// della segnalazione — dove `room_id` dice *dov'è il guasto*, un fatto
    /// fissato al momento in cui è stato segnalato — qui dicono *chi deve
    /// pagare*, che cambia legittimamente finché la bolletta è una bozza.
    #[must_use]
    pub const fn touches_property_references(&self) -> bool {
        self.room_id.is_some() || self.lease_id.is_some()
    }
}

/// Bolletta di un'utenza.
///
/// **Fuori perimetro in questa fetta** (`docs/09-stato-e-roadmap.md`): l'OCR
/// (`ai_job_id`, `ocr_confidence`, `ocr_extracted`, e lo stato `pending_review`)
/// e la colonna `document_id`. Le colonne esistono nello schema e restano ai loro
/// default: vederle non è una ragione per riempirle. Su `ocr_extracted` in
/// particolare **non si scrive**, perché lo schema tiene apposta separato ciò che
/// ha letto la macchina da ciò che ha confermato una persona (`validated_by`,
/// `validated_at`), così che l'accuratezza dell'OCR si possa misurare quando
/// arriverà; riempirla con valori umani distruggerebbe quel dato prima di
/// raccoglierlo.
///
/// `document_id` resta non popolata anche per una ragione sua: la scansione si
/// allega con il riferimento polimorfico dei documenti (`owner_type`,
/// `owner_id`), come per ogni altro aggregato. Popolare entrambi significherebbe
/// avere **due risposte** alla domanda «qual è il documento di questa bolletta» e
/// doverle tenere allineate — è la stessa ragione per cui l'API non tocca
/// `leases.contract_document_id` né `users.avatar_document_id`.
#[derive(Debug, Clone)]
pub struct UtilityBill {
    id: UtilityBillId,
    org_id: OrgId,
    property_id: PropertyId,
    room_id: Option<RoomId>,
    lease_id: Option<LeaseId>,
    utility_type: UtilityType,
    status: BillStatus,
    supplier_name: Option<SupplierName>,
    supplier_vat: Option<SupplierVat>,
    invoice_number: Option<InvoiceNumber>,
    issue_date: Option<NaiveDate>,
    due_date: Option<NaiveDate>,
    period_start: Option<NaiveDate>,
    period_end: Option<NaiveDate>,
    total_amount: Money,
    taxable_amount: Option<Money>,
    vat_amount: Option<Money>,
    currency: Currency,
    consumption: Option<Consumption>,
    validated_by: Option<UserId>,
    validated_at: Option<DateTime<Utc>>,
    notes: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

impl UtilityBill {
    /// Registra una bolletta. Nasce sempre `draft`.
    ///
    /// `draft` e non `pending_review`: quest'ultimo dice «una macchina ha letto
    /// questi numeri e una persona deve confermarli», e qui la persona li ha
    /// appena digitati.
    pub fn register(parts: NewUtilityBill, now: DateTime<Utc>) -> DomainResult<Self> {
        let NewUtilityBill {
            id,
            org_id,
            property_id,
            room_id,
            lease_id,
            utility_type,
            supplier_name,
            supplier_vat,
            invoice_number,
            issue_date,
            due_date,
            period_start,
            period_end,
            total_amount,
            taxable_amount,
            vat_amount,
            consumption,
            notes,
        } = parts;

        Self::ensure_period_is_ordered(period_start, period_end)?;

        let currency = Currency::EUR;
        let total_amount = Self::amount("total_amount", total_amount, currency)?;
        let taxable_amount = taxable_amount
            .map(|value| Self::amount("taxable_amount", value, currency))
            .transpose()?;
        let vat_amount = vat_amount
            .map(|value| Self::amount("vat_amount", value, currency))
            .transpose()?;

        // Nessun controllo che `taxable_amount + vat_amount` faccia
        // `total_amount`, e **non va aggiunto per simmetria**: una bolletta di
        // utenza italiana porta abitualmente più aliquote IVA, quote fisse e voci
        // non imponibili, quindi un'uguaglianza rifiuterebbe bollette valide. Lo
        // schema infatti chiede solo che i tre importi siano non negativi.

        Ok(Self {
            id,
            org_id,
            property_id,
            room_id,
            lease_id,
            utility_type,
            status: BillStatus::Draft,
            supplier_name,
            supplier_vat,
            invoice_number,
            issue_date,
            due_date,
            period_start,
            period_end,
            total_amount,
            taxable_amount,
            vat_amount,
            currency,
            consumption,
            validated_by: None,
            validated_at: None,
            notes: text::optional("notes", notes, 4000)?,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        })
    }

    /// Applica una modifica parziale. `true` se qualcosa è cambiato.
    ///
    /// Ammessa da `draft` **e da `validated`**, e su una bolletta validata
    /// **disfa la validazione**: lo stato torna a `draft` e `validated_at` e
    /// `validated_by` si azzerano. Non è un effetto collaterale — validare è
    /// l'affermazione «questi numeri sono quelli», e cambiare i numeri la
    /// smentisce, quindi chi la valida di nuovo deve rifarlo apposta. È la stessa
    /// regola per cui riaprire una segnalazione cancella `closed_at` e la nota di
    /// risoluzione.
    ///
    /// **`validated_at` va azzerato qui, e nessuno lo fa al posto nostro.** Il
    /// vincolo `utility_bills_validated_ck` è
    /// `status NOT IN ('validated','allocated','paid') OR validated_at IS NOT NULL`:
    /// costringe una sola direzione — obbliga ad avere la data quando si è
    /// validati, e **non** obbliga a toglierla tornando in bozza. Il database
    /// accetterebbe senza dire niente una bolletta `draft` con `validated_at`
    /// valorizzato, cioè una che dichiara di essere stata controllata e risulta
    /// da controllare. È la stessa forma di `tickets_closed_ck`, ed è la seconda
    /// volta che questo schema la presenta.
    pub fn apply(
        &mut self,
        changes: &UtilityBillChanges,
        now: DateTime<Utc>,
    ) -> DomainResult<bool> {
        if !self.status.is_editable() {
            return Err(DomainError::Invariant(Invariant::UtilityBillNotEditable));
        }

        let period_start = apply_patch(self.period_start, changes.period_start);
        let period_end = apply_patch(self.period_end, changes.period_end);
        Self::ensure_period_is_ordered(period_start, period_end)?;

        let total_amount = match changes.total_amount {
            None => self.total_amount,
            Some(value) => Self::amount("total_amount", value, self.currency)?,
        };
        let taxable_amount = self.resolve_amount(
            "taxable_amount",
            changes.taxable_amount,
            self.taxable_amount,
        )?;
        let vat_amount = self.resolve_amount("vat_amount", changes.vat_amount, self.vat_amount)?;

        let utility_type = changes.utility_type.unwrap_or(self.utility_type);
        let room_id = apply_patch(self.room_id, changes.room_id);
        let lease_id = apply_patch(self.lease_id, changes.lease_id);
        let supplier_name = apply_patch_cloned(&self.supplier_name, &changes.supplier_name);
        let supplier_vat = apply_patch_cloned(&self.supplier_vat, &changes.supplier_vat);
        let invoice_number = apply_patch_cloned(&self.invoice_number, &changes.invoice_number);
        let issue_date = apply_patch(self.issue_date, changes.issue_date);
        let due_date = apply_patch(self.due_date, changes.due_date);
        let consumption = apply_patch_cloned(&self.consumption, &changes.consumption);
        let notes = match &changes.notes {
            None => self.notes.clone(),
            Some(None) => None,
            Some(Some(value)) => text::optional("notes", Some(value.clone()), 4000)?,
        };

        let changed = utility_type != self.utility_type
            || room_id != self.room_id
            || lease_id != self.lease_id
            || supplier_name != self.supplier_name
            || supplier_vat != self.supplier_vat
            || invoice_number != self.invoice_number
            || issue_date != self.issue_date
            || due_date != self.due_date
            || period_start != self.period_start
            || period_end != self.period_end
            || total_amount != self.total_amount
            || taxable_amount != self.taxable_amount
            || vat_amount != self.vat_amount
            || consumption != self.consumption
            || notes != self.notes;
        if !changed {
            // Un PATCH che non cambia nulla non deve produrre scrittura, evento e
            // riga di audit — e soprattutto non deve disfare una validazione.
            return Ok(false);
        }

        self.utility_type = utility_type;
        self.room_id = room_id;
        self.lease_id = lease_id;
        self.supplier_name = supplier_name;
        self.supplier_vat = supplier_vat;
        self.invoice_number = invoice_number;
        self.issue_date = issue_date;
        self.due_date = due_date;
        self.period_start = period_start;
        self.period_end = period_end;
        self.total_amount = total_amount;
        self.taxable_amount = taxable_amount;
        self.vat_amount = vat_amount;
        self.consumption = consumption;
        self.notes = notes;

        if matches!(self.status, BillStatus::Validated) {
            self.status = BillStatus::Draft;
            self.validated_at = None;
            self.validated_by = None;
        }
        self.updated_at = now;
        Ok(true)
    }

    /// Valida la bolletta: una persona ha controllato che i numeri siano quelli.
    ///
    /// `validated_at` è obbligatorio anche a schema quando lo stato è
    /// `validated`, `allocated` o `paid` (`utility_bills_validated_ck`).
    pub fn validate(&mut self, validated_by: UserId, now: DateTime<Utc>) -> DomainResult<()> {
        if !matches!(self.status, BillStatus::Draft) {
            return Err(DomainError::Invariant(Invariant::UtilityBillNotValidatable));
        }
        self.status = BillStatus::Validated;
        self.validated_by = Some(validated_by);
        self.validated_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    /// Registra che la bolletta è stata riaddebitata a un contratto.
    ///
    /// Ammessa da `validated` **e da `allocated`**: riaddebitare una bolletta in
    /// parti a più contratti — il caso del co-living con contratti per stanza — è
    /// una ripetizione dell'operazione, come riassegnare una segnalazione già
    /// assegnata.
    ///
    /// **Nessuno verifica che le parti sommino al totale**, e non è una
    /// dimenticanza: lo schema non ha un vincolo, un controllo applicativo
    /// sarebbe soggetto a race condition fra due riaddebiti concorrenti — che è
    /// il criterio di ADR-015 per cui quel genere di invariante sta nel database
    /// o non sta da nessuna parte — e il posto in cui «le parti sommano» è già
    /// espresso correttamente è il `CONSTRAINT TRIGGER … DEFERRABLE` di
    /// `expense_shares`, cioè il modulo co-living.
    pub fn mark_allocated(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if !matches!(self.status, BillStatus::Validated | BillStatus::Allocated) {
            return Err(DomainError::Invariant(Invariant::UtilityBillNotAllocatable));
        }
        self.status = BillStatus::Allocated;
        self.updated_at = now;
        Ok(())
    }

    /// Annulla la bolletta, che **sparisce**: `status = 'void'` *e*
    /// `deleted_at = now`.
    ///
    /// È l'asimmetria deliberata con l'addebito, che annullato **resta
    /// visibile**. La ragione è che qui non c'è nessuno a cui dirlo — è un
    /// documento nostro inserito male — mentre un addebito annullato l'ha visto
    /// chi doveva pagarlo, e deve poter vedere che la richiesta è caduta.
    ///
    /// E c'è una ragione tecnica per liberare la riga: `utility_bills_dedup_uk` è
    /// unico su `(org_id, supplier_vat, invoice_number)` ed è parziale su
    /// **`deleted_at IS NULL`**, non su `status <> 'void'`. Senza il soft delete
    /// una bolletta annullata continuerebbe a occupare la chiave e la stessa
    /// fattura non sarebbe reinseribile mai più. La traccia di ciò che è stato
    /// annullato resta in `audit_log`, che è append-only.
    ///
    /// **Non si annulla da `allocated` in poi.** Il piano non lo dice a parole ma
    /// lo impone la sua stessa logica: da lì la bolletta è la base di un addebito
    /// già emesso a qualcuno, e farla sparire lascerebbe quell'addebito a puntare
    /// (`charges.utility_bill_id`) una riga che non si vede più. L'ordine è
    /// annullare prima gli addebiti.
    pub fn void(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if !self.status.is_editable() {
            return Err(DomainError::Invariant(Invariant::UtilityBillNotVoidable));
        }
        self.status = BillStatus::Void;
        self.deleted_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    fn ensure_period_is_ordered(
        start: Option<NaiveDate>,
        end: Option<NaiveDate>,
    ) -> DomainResult<()> {
        // `>=` e non `>`, esattamente come `utility_bills_period_ck`: una bolletta
        // che copre un giorno solo è valida.
        if let (Some(start), Some(end)) = (start, end)
            && end < start
        {
            return Err(DomainError::validation(
                "period_end",
                "cannot precede the start of the period",
            ));
        }
        Ok(())
    }

    fn amount(field: &'static str, value: Decimal, currency: Currency) -> DomainResult<Money> {
        Money::non_negative(value, currency)
            .map_err(|_| DomainError::validation(field, "must be a non-negative amount"))
    }

    fn resolve_amount(
        &self,
        field: &'static str,
        patch: Patch<Decimal>,
        current: Option<Money>,
    ) -> DomainResult<Option<Money>> {
        match patch {
            None => Ok(current),
            Some(None) => Ok(None),
            Some(Some(value)) => Ok(Some(Self::amount(field, value, self.currency)?)),
        }
    }

    #[must_use]
    pub const fn id(&self) -> UtilityBillId {
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
    pub const fn lease_id(&self) -> Option<LeaseId> {
        self.lease_id
    }
    #[must_use]
    pub const fn utility_type(&self) -> UtilityType {
        self.utility_type
    }
    #[must_use]
    pub const fn status(&self) -> BillStatus {
        self.status
    }
    #[must_use]
    pub const fn supplier_name(&self) -> Option<&SupplierName> {
        self.supplier_name.as_ref()
    }
    #[must_use]
    pub const fn supplier_vat(&self) -> Option<&SupplierVat> {
        self.supplier_vat.as_ref()
    }
    #[must_use]
    pub const fn invoice_number(&self) -> Option<&InvoiceNumber> {
        self.invoice_number.as_ref()
    }
    #[must_use]
    pub const fn issue_date(&self) -> Option<NaiveDate> {
        self.issue_date
    }
    #[must_use]
    pub const fn due_date(&self) -> Option<NaiveDate> {
        self.due_date
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
    pub const fn total_amount(&self) -> Money {
        self.total_amount
    }
    #[must_use]
    pub const fn taxable_amount(&self) -> Option<Money> {
        self.taxable_amount
    }
    #[must_use]
    pub const fn vat_amount(&self) -> Option<Money> {
        self.vat_amount
    }
    #[must_use]
    pub const fn currency(&self) -> Currency {
        self.currency
    }
    #[must_use]
    pub const fn consumption(&self) -> Option<&Consumption> {
        self.consumption.as_ref()
    }
    #[must_use]
    pub const fn validated_by(&self) -> Option<UserId> {
        self.validated_by
    }
    #[must_use]
    pub const fn validated_at(&self) -> Option<DateTime<Utc>> {
        self.validated_at
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

    /// Ricostruzione da persistenza: non riesegue le invarianti, perché una
    /// bolletta già salvata deve poter essere riletta anche se le regole sono
    /// cambiate da allora.
    #[must_use]
    pub fn from_persistence(parts: UtilityBillParts) -> Self {
        let UtilityBillParts {
            id,
            org_id,
            property_id,
            room_id,
            lease_id,
            utility_type,
            status,
            supplier_name,
            supplier_vat,
            invoice_number,
            issue_date,
            due_date,
            period_start,
            period_end,
            total_amount,
            taxable_amount,
            vat_amount,
            currency,
            consumption,
            validated_by,
            validated_at,
            notes,
            created_at,
            updated_at,
            deleted_at,
        } = parts;
        Self {
            id,
            org_id,
            property_id,
            room_id,
            lease_id,
            utility_type,
            status,
            supplier_name,
            supplier_vat,
            invoice_number,
            issue_date,
            due_date,
            period_start,
            period_end,
            total_amount,
            taxable_amount,
            vat_amount,
            currency,
            consumption,
            validated_by,
            validated_at,
            notes,
            created_at,
            updated_at,
            deleted_at,
        }
    }
}

/// Dati grezzi di una bolletta letta dalla persistenza.
#[derive(Debug, Clone)]
pub struct UtilityBillParts {
    pub id: UtilityBillId,
    pub org_id: OrgId,
    pub property_id: PropertyId,
    pub room_id: Option<RoomId>,
    pub lease_id: Option<LeaseId>,
    pub utility_type: UtilityType,
    pub status: BillStatus,
    pub supplier_name: Option<SupplierName>,
    pub supplier_vat: Option<SupplierVat>,
    pub invoice_number: Option<InvoiceNumber>,
    pub issue_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub total_amount: Money,
    pub taxable_amount: Option<Money>,
    pub vat_amount: Option<Money>,
    pub currency: Currency,
    pub consumption: Option<Consumption>,
    pub validated_by: Option<UserId>,
    pub validated_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Risolve un `Patch` su un valore `Copy`.
fn apply_patch<T: Copy>(current: Option<T>, patch: Patch<T>) -> Option<T> {
    match patch {
        None => current,
        Some(next) => next,
    }
}

/// Come [`apply_patch`], per i valori che non sono `Copy`.
fn apply_patch_cloned<T: Clone>(current: &Option<T>, patch: &Patch<T>) -> Option<T> {
    match patch {
        None => current.clone(),
        Some(next) => next.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::billing::value_objects::ConsumptionUnit;
    use std::str::FromStr;

    fn date(raw: &str) -> NaiveDate {
        NaiveDate::from_str(raw).expect("data di test valida")
    }

    fn dec(raw: &str) -> Decimal {
        Decimal::from_str(raw).expect("decimale di test valido")
    }

    fn new_bill() -> NewUtilityBill {
        NewUtilityBill {
            id: UtilityBillId::generate(),
            org_id: OrgId::generate(),
            property_id: PropertyId::generate(),
            room_id: None,
            lease_id: Some(LeaseId::generate()),
            utility_type: UtilityType::Gas,
            supplier_name: Some(SupplierName::parse("Enel Energia").expect("valido")),
            supplier_vat: Some(SupplierVat::parse("IT12345678901").expect("valida")),
            invoice_number: Some(InvoiceNumber::parse("2026/A/00417").expect("valido")),
            issue_date: Some(date("2026-08-01")),
            due_date: Some(date("2026-08-31")),
            period_start: Some(date("2026-06-01")),
            period_end: Some(date("2026-07-31")),
            total_amount: dec("184.20"),
            taxable_amount: Some(dec("151.00")),
            vat_amount: Some(dec("33.20")),
            consumption: Some(
                Consumption::new(
                    dec("123.4567"),
                    ConsumptionUnit::parse("Smc").expect("unità valida"),
                )
                .expect("consumo valido"),
            ),
            notes: None,
        }
    }

    fn bill() -> UtilityBill {
        UtilityBill::register(new_bill(), Utc::now()).expect("bolletta valida")
    }

    /// Porta la bolletta fino allo stato richiesto percorrendo il flusso
    /// nominale. Fallisce se una transizione del percorso non è ammessa, il che è
    /// esattamente ciò che si vuole sapere.
    fn at(status: BillStatus) -> UtilityBill {
        let now = Utc::now();
        let mut bill = bill();
        match status {
            BillStatus::Draft => {}
            BillStatus::Validated => {
                bill.validate(UserId::generate(), now).expect("validata");
            }
            BillStatus::Allocated => {
                bill.validate(UserId::generate(), now).expect("validata");
                bill.mark_allocated(now).expect("riaddebitata");
            }
            BillStatus::Void => {
                bill.void(now).expect("annullata");
            }
            // Mai prodotti dall'applicazione: si costruiscono solo da persistenza,
            // e i test che li riguardano lo fanno esplicitamente.
            BillStatus::PendingReview | BillStatus::Paid | BillStatus::Disputed => {}
        }
        bill
    }

    #[test]
    fn a_registered_bill_is_a_draft_nobody_has_checked() {
        let bill = bill();
        assert_eq!(bill.status(), BillStatus::Draft);
        assert_eq!(bill.validated_at(), None);
        assert_eq!(bill.validated_by(), None);
        assert_eq!(bill.deleted_at(), None);
        assert_eq!(bill.currency(), Currency::EUR);
    }

    /// La tabella delle transizioni, ammesse e rifiutate, verificata per intero.
    #[test]
    fn the_transition_table_is_complete() {
        use BillStatus::{Allocated, Draft, Validated, Void};

        // apply: da `draft` e da `validated`.
        for from in [Draft, Validated, Allocated, Void] {
            let changes = UtilityBillChanges {
                notes: Some(Some("una nota".to_owned())),
                ..UtilityBillChanges::default()
            };
            assert_eq!(
                at(from).apply(&changes, Utc::now()).is_ok(),
                from.is_editable(),
                "apply da {from:?}"
            );
        }

        // validate: solo da `draft`.
        for from in [Draft, Validated, Allocated, Void] {
            let allowed = matches!(from, Draft);
            assert_eq!(
                at(from).validate(UserId::generate(), Utc::now()).is_ok(),
                allowed,
                "validate da {from:?}"
            );
        }

        // mark_allocated: da `validated` e da `allocated` (riaddebito ripetuto).
        for from in [Draft, Validated, Allocated, Void] {
            let allowed = matches!(from, Validated | Allocated);
            assert_eq!(
                at(from).mark_allocated(Utc::now()).is_ok(),
                allowed,
                "mark_allocated da {from:?}"
            );
        }

        // void: da `draft` e da `validated`, non da `allocated`.
        for from in [Draft, Validated, Allocated, Void] {
            assert_eq!(
                at(from).void(Utc::now()).is_ok(),
                from.is_editable(),
                "void da {from:?}"
            );
        }
    }

    /// La trappola: `utility_bills_validated_ck` costringe una sola direzione,
    /// quindi il database accetterebbe una bolletta `draft` che dichiara di essere
    /// stata controllata. Deve azzerarlo il dominio.
    #[test]
    fn correcting_a_validated_bill_undoes_the_validation() {
        let validator = UserId::generate();
        let mut bill = bill();
        bill.validate(validator, Utc::now()).expect("validata");
        assert_eq!(bill.status(), BillStatus::Validated);
        assert_eq!(bill.validated_by(), Some(validator));
        assert!(bill.validated_at().is_some());

        let changed = bill
            .apply(
                &UtilityBillChanges {
                    invoice_number: Some(Some(
                        InvoiceNumber::parse("2026/A/00418").expect("valido"),
                    )),
                    ..UtilityBillChanges::default()
                },
                Utc::now(),
            )
            .expect("correggibile");

        assert!(changed);
        assert_eq!(bill.status(), BillStatus::Draft, "va rivalidata apposta");
        assert_eq!(
            bill.validated_at(),
            None,
            "una bozza con la data di validazione è una riga che mente"
        );
        assert_eq!(bill.validated_by(), None);
    }

    /// Un PATCH che non cambia nulla non produce scrittura, evento e riga di
    /// audit — e soprattutto **non disfa una validazione**, che sarebbe il modo
    /// più silenzioso di perderla.
    #[test]
    fn an_empty_patch_changes_nothing_and_keeps_the_validation() {
        let mut bill = at(BillStatus::Validated);
        let changed = bill
            .apply(&UtilityBillChanges::default(), Utc::now())
            .expect("valido");
        assert!(!changed);
        assert_eq!(bill.status(), BillStatus::Validated);
        assert!(bill.validated_at().is_some());
        assert!(UtilityBillChanges::default().is_empty());
    }

    /// Anche un patch che *nomina* un campo senza cambiarne il valore non deve
    /// disfare la validazione: il confronto è sul valore, non sulla menzione.
    #[test]
    fn a_patch_that_sets_the_same_value_is_not_a_change() {
        let mut bill = at(BillStatus::Validated);
        let changed = bill
            .apply(
                &UtilityBillChanges {
                    utility_type: Some(UtilityType::Gas),
                    total_amount: Some(dec("184.20")),
                    ..UtilityBillChanges::default()
                },
                Utc::now(),
            )
            .expect("valido");
        assert!(!changed);
        assert_eq!(bill.status(), BillStatus::Validated);
    }

    #[test]
    fn a_patch_can_clear_an_optional_field() {
        let mut bill = bill();
        assert!(bill.consumption().is_some());
        let changed = bill
            .apply(
                &UtilityBillChanges {
                    consumption: Some(None),
                    vat_amount: Some(None),
                    ..UtilityBillChanges::default()
                },
                Utc::now(),
            )
            .expect("valido");
        assert!(changed);
        assert_eq!(bill.consumption(), None);
        assert_eq!(bill.vat_amount(), None);
    }

    #[test]
    fn the_period_cannot_run_backwards() {
        let mut input = new_bill();
        input.period_start = Some(date("2026-07-31"));
        input.period_end = Some(date("2026-06-01"));
        assert!(UtilityBill::register(input, Utc::now()).is_err());

        // Un periodo di un giorno solo è valido: il vincolo a schema è `>=`.
        let mut input = new_bill();
        input.period_start = Some(date("2026-06-01"));
        input.period_end = Some(date("2026-06-01"));
        assert!(UtilityBill::register(input, Utc::now()).is_ok());
    }

    /// La stessa verifica deve valere sul `PATCH`, dove si cambia un estremo
    /// solo e l'altro resta quello di prima.
    #[test]
    fn the_period_is_rechecked_against_the_stored_end_on_a_patch() {
        let mut bill = bill();
        let error = bill
            .apply(
                &UtilityBillChanges {
                    period_start: Some(Some(date("2026-12-01"))),
                    ..UtilityBillChanges::default()
                },
                Utc::now(),
            )
            .expect_err("supererebbe la fine memorizzata");
        assert!(matches!(error, DomainError::Validation { .. }));
    }

    #[test]
    fn amounts_cannot_be_negative() {
        let mut input = new_bill();
        input.total_amount = dec("-1.00");
        assert!(UtilityBill::register(input, Utc::now()).is_err());

        let mut bill = bill();
        assert!(
            bill.apply(
                &UtilityBillChanges {
                    vat_amount: Some(Some(dec("-0.01"))),
                    ..UtilityBillChanges::default()
                },
                Utc::now()
            )
            .is_err()
        );
    }

    /// Una bolletta italiana porta abitualmente più aliquote, quote fisse e voci
    /// non imponibili: chiedere che imponibile e IVA sommino al totale
    /// rifiuterebbe bollette valide, e lo schema infatti non lo chiede.
    #[test]
    fn the_taxable_and_vat_amounts_need_not_add_up_to_the_total() {
        let mut input = new_bill();
        input.total_amount = dec("184.20");
        input.taxable_amount = Some(dec("10.00"));
        input.vat_amount = Some(dec("2.20"));
        assert!(UtilityBill::register(input, Utc::now()).is_ok());
    }

    #[test]
    fn voiding_a_bill_makes_it_disappear_so_the_invoice_number_comes_free() {
        let now = Utc::now();
        let mut bill = bill();
        bill.void(now).expect("annullata");
        assert_eq!(bill.status(), BillStatus::Void);
        assert_eq!(
            bill.deleted_at(),
            Some(now),
            "senza il soft delete la stessa fattura non sarebbe reinseribile: \
             `utility_bills_dedup_uk` è parziale su `deleted_at IS NULL`"
        );
    }

    /// Da `allocated` la bolletta è la base di un addebito già emesso: farla
    /// sparire lascerebbe quell'addebito a puntare una riga che non si vede più.
    #[test]
    fn a_bill_already_recharged_cannot_be_voided() {
        let mut bill = at(BillStatus::Allocated);
        let error = bill
            .void(Utc::now())
            .expect_err("c'è un addebito che la cita");
        assert!(matches!(
            error,
            DomainError::Invariant(Invariant::UtilityBillNotVoidable)
        ));
    }

    /// Riaddebitare in parti a più contratti è una ripetizione dell'operazione,
    /// come riassegnare una segnalazione già assegnata.
    #[test]
    fn a_bill_can_be_recharged_more_than_once() {
        let mut bill = at(BillStatus::Validated);
        bill.mark_allocated(Utc::now()).expect("primo riaddebito");
        bill.mark_allocated(Utc::now()).expect("secondo riaddebito");
        assert_eq!(bill.status(), BillStatus::Allocated);
    }

    /// I tre stati che l'enum ha e l'applicazione non scrive: nessuna transizione
    /// deve produrli. Se qualcuno ne aggiungesse una, questo test lo dice prima
    /// che una riga con quello stato compaia in produzione senza un modo di
    /// uscirne.
    #[test]
    fn no_transition_ever_produces_an_unwritten_status() {
        let now = Utc::now();
        for from in [BillStatus::Draft, BillStatus::Validated] {
            let mut bill = at(from);
            let _ = bill.apply(
                &UtilityBillChanges {
                    notes: Some(Some("nota".to_owned())),
                    ..UtilityBillChanges::default()
                },
                now,
            );
            let _ = bill.validate(UserId::generate(), now);
            let _ = bill.mark_allocated(now);
            let _ = bill.void(now);
            assert!(
                !matches!(
                    bill.status(),
                    BillStatus::PendingReview | BillStatus::Paid | BillStatus::Disputed
                ),
                "da {from:?} si è arrivati a {:?}",
                bill.status()
            );
        }
    }

    /// I due riferimenti che il service layer deve riverificare contro
    /// l'immobile: lo schema non li vincola, e il `PATCH` li lascia scrivibili.
    #[test]
    fn a_patch_touching_the_references_is_recognizable() {
        assert!(!UtilityBillChanges::default().touches_property_references());
        assert!(
            UtilityBillChanges {
                room_id: Some(Some(RoomId::generate())),
                ..UtilityBillChanges::default()
            }
            .touches_property_references()
        );
        assert!(
            UtilityBillChanges {
                lease_id: Some(None),
                ..UtilityBillChanges::default()
            }
            .touches_property_references()
        );
        // Un patch che tocca solo i numeri non richiede la rilettura di stanza e
        // contratto: sarebbe una query in più per nulla.
        assert!(
            !UtilityBillChanges {
                total_amount: Some(dec("10.00")),
                ..UtilityBillChanges::default()
            }
            .touches_property_references()
        );
    }

    #[test]
    fn the_changed_field_names_carry_no_values() {
        let changes = UtilityBillChanges {
            total_amount: Some(dec("999.99")),
            supplier_vat: Some(Some(SupplierVat::parse("IT99999999999").expect("valida"))),
            ..UtilityBillChanges::default()
        };
        assert_eq!(
            changes.changed_field_names(),
            vec!["supplier_vat", "total_amount"]
        );
    }
}
