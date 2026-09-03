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
/// # Che cosa questi otto numeri **non** dicono
///
/// Due semantiche restano deliberatamente fuori, e sono scritte qui perché chi le
/// deciderà arriverà da questo tipo:
///
/// - **`written_off` non è distinto da `void`.** Entrambi escono dall'insoluto,
///   quindi non compaiono in `total_outstanding` né in `overdue_charges`. Ma solo
///   il primo è una **perdita**, e la distinzione conta in un report. Aggiungere
///   un nono numero è additivo; cambiare il significato di `total_outstanding` non
///   lo è;
/// - **il credito da sovrapagamento sparisce.** La vista somma con
///   `WHERE outstanding_amount > 0`, quindi un `outstanding_amount` negativo — che
///   è un credito verso l'inquilino — non compensa gli altri insoluti (che è
///   probabilmente giusto) e non compare da nessuna parte (che è il numero che
///   qualcuno cercherà).
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
}
