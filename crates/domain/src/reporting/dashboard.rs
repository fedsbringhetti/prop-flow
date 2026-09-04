use rust_decimal::Decimal;

use crate::shared::OrgId;

/// I numeri che un proprietario vede aprendo il prodotto.
///
/// Letti da `app.v_owner_dashboard`, che è `security_invoker` e applica quindi la
/// Row Level Security del chiamante: non c'è un filtro applicativo per
/// organizzazione, c'è la stessa policy che protegge ogni altra riga.
///
/// # Perché non si ricalcolano qui
///
/// Perché sarebbero una **seconda definizione**. `total_outstanding` somma
/// `app.v_charge_balances`, che a sua volta decide che cosa conta come pagato
/// (`succeeded` e `partially_refunded`) e che cosa esce dall'insoluto (`void` e
/// `written_off`, dalla migration `0024`). Riderivare quei numeri in Rust
/// significherebbe doverli tenere allineati a mano il giorno in cui la vista
/// cambia, ed è la ragione per cui l'intero modulo contabile legge dalla vista.
///
/// # Le due semantiche che erano rimaste aperte, e come sono state decise
///
/// Erano scritte qui perché chi le avrebbe decise sarebbe arrivato da questo tipo.
/// Sono state decise costruendo il modulo dei pagamenti dal lato del proprietario,
/// che è la prima occasione in cui il carico si vede:
///
/// - **`written_off` è ora distinto da `void`**, ed è il nono numero:
///   [`Self::written_off_total`]. Entrambi continuano a uscire dall'insoluto —
///   `total_outstanding` non cambia significato, che sarebbe stata una rottura —
///   ma solo il primo è una perdita, e ora si vede. L'aggiunta è additiva;
/// - **il credito da sovrapagamento resta fuori di qui, e si mostra per
///   addebito.** La vista somma con `WHERE outstanding_amount > 0`, quindi un
///   `outstanding_amount` negativo — che è un credito verso l'inquilino — non
///   compensa gli altri insoluti, ed è giusto così: questo è un cruscotto di ciò
///   che è dovuto, e un credito non è un insoluto negativo. A non farlo sparire
///   del tutto è l'elenco degli addebiti, che lo evidenzia riga per riga.
///
///   **Filtrarlo lato server non si può**, e va detto perché sembra un'omissione:
///   un sovrapagamento porta lo stato ricalcolato a `paid`, quindi
///   `?outstanding=true` lo esclude per costruzione, e `ChargeQuery` non ha nulla
///   sul segno del residuo. Un filtro vero vorrebbe un parametro nuovo e un indice
///   parziale che lo regga; è rinviato finché il caso d'uso «mostrami tutti i
///   crediti» non nasce, perché costruire l'indice prima del caso è il modo di
///   costruire quello sbagliato.
///
/// # `monthly_rent_expected` è un'attesa, non un incasso
///
/// Somma `rent_amount` dei contratti con `status = 'active'`, e lo stato
/// memorizzato **resta `active` anche dopo la scadenza** perché nessuno scheduler
/// lo riscrive (ADR-019). Un contratto scaduto e non disdetto continua quindi a
/// contribuire, ed è la ragione per cui `leases_expiring_90d` sta accanto: è il
/// numero che dice quanto di quell'attesa sta per non esserci più.
#[derive(Debug, Clone)]
pub struct OwnerDashboard {
    pub org_id: OrgId,
    /// Immobili non archiviati, in **ogni** stato: anche le bozze.
    pub properties_count: i64,
    /// Contratti con `status = 'active'` memorizzato, scadenza compresa.
    pub active_leases: i64,
    /// Somma dei canoni dei contratti attivi. Non è un incasso previsto: è la
    /// somma di ciò che i contratti dichiarano.
    pub monthly_rent_expected: Decimal,
    /// Residuo dovuto, sommato dai soli addebiti con residuo **positivo**.
    pub total_outstanding: Decimal,
    /// Addebiti con `is_past_due` secondo il `CURRENT_DATE` **del database**.
    pub overdue_charges: i64,
    /// Segnalazioni non ancora chiuse, annullate o risolte: i sei stati che
    /// l'indice parziale `tickets_open_ix` enumera.
    pub open_tickets: i64,
    /// Contratti `active` o `expiring` la cui `end_date` cade entro novanta
    /// giorni.
    pub leases_expiring_90d: i64,
    /// Bollette in `draft` **o** `pending_review`.
    ///
    /// Sono due popolazioni diverse e il numero le somma: una bolletta appena
    /// registrata senza scansione, e una che la macchina ha letto e che aspetta
    /// una persona. La vista le conta insieme dalla migration `0015` e non è mai
    /// cambiata; è la seconda popolazione a non essere esistita finché nessuno
    /// scriveva `pending_review`.
    pub bills_pending_review: i64,
    /// Quanto si è **rinunciato a incassare**: la somma dei residui degli addebiti
    /// `written_off`. Dalla migration `0032`.
    ///
    /// **È il residuo e non l'importo**, ed è il modo più facile di sbagliare
    /// questo numero: un addebito da 800 con 300 incassati e poi stornato è una
    /// perdita di **500**, perché gli altri trecento sono entrati. La vista somma
    /// `amount - paid_amount`.
    ///
    /// **`outstanding_amount` non poteva produrlo.** La migration `0024` lo azzera
    /// proprio su `written_off`: è la stessa colonna che rende questo numero
    /// necessario — è azzerandola che la perdita è sparita dalla dashboard — e che
    /// non può calcolarlo.
    ///
    /// Non compensa e non si sottrae da niente: sta accanto a
    /// [`Self::total_outstanding`], che continua a dire quanto è ancora dovuto. Il
    /// numero esiste **insieme** allo storno esteso a `partially_paid`: senza
    /// quello conterebbe metà delle perdite, perché un addebito su cui fosse
    /// entrato del denaro non sarebbe stornabile.
    pub written_off_total: Decimal,
}
