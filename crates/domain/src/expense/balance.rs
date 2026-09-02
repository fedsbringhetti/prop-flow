use rust_decimal::Decimal;

use crate::shared::{ExpenseGroupId, Money, OrgId, UserId};

/// Il saldo di una persona in un gruppo, **letto** da
/// `app.v_expense_group_balances`.
///
/// Si legge e non si ricalcola, per la stessa ragione per cui il saldo di un
/// addebito viene dalla vista: due definizioni dello stesso numero si
/// contraddicono al primo caso limite, e quella che conta è la definizione che ha
/// prodotto il numero mostrato all'utente.
///
/// ```text
/// net_balance = total_paid − total_owed + total_settled_out − total_settled_in
/// ```
///
/// Positivo = **in credito**, il gruppo gli deve. Negativo = in debito.
///
/// # La somma dei saldi di un gruppo fa zero
///
/// È l'invariante che rende questi numeri leggibili, ed è ciò che la migration
/// `0027` ha rimesso in piedi: prima, chi usciva dal gruppo spariva dalla vista
/// **portandosi via il debito**, e il credito di chi aveva anticipato restava lì
/// senza una contropartita. Dalla `0027` chi esce resta finché il suo netto non è
/// zero.
///
/// # Chi è uscito compare nei saldi degli altri e non nei propri
///
/// La vista è `security_invoker`, quindi le righe passano da
/// `expense_group_members_select`, che chiede `can_access_expense_group` — vera
/// per lo staff e per chi ha una membership **attiva**. Il debitore uscito è quindi
/// visibile a chi è rimasto e per sé stesso non vede più il gruppo. È corretto
/// rispetto all'isolamento, e va saputo prima di leggere questi numeri da un
/// client.
///
/// # `is_settled` non c'entra
///
/// `app.expense_shares.is_settled` esiste e la vista **non lo guarda**: il saldo è
/// tutto derivato dai movimenti. Vedi la nota su
/// [`ExpenseShare`](super::ExpenseShare).
#[derive(Debug, Clone)]
pub struct GroupBalance {
    pub org_id: OrgId,
    pub group_id: ExpenseGroupId,
    pub user_id: UserId,
    /// Quanto ha anticipato per il gruppo.
    pub total_paid: Money,
    /// Quanto gli è stato imputato in quote.
    pub total_owed: Money,
    /// Quanto ha versato in regolamenti **confermati**.
    pub total_settled_out: Money,
    /// Quanto ha ricevuto in regolamenti **confermati**.
    pub total_settled_in: Money,
    /// Positivo se è in credito, negativo se è in debito.
    pub net_balance: Money,
}

impl GroupBalance {
    /// `true` se il gruppo gli deve del denaro.
    #[must_use]
    pub fn is_creditor(&self) -> bool {
        self.net_balance.amount() > Decimal::ZERO
    }

    /// `true` se deve del denaro al gruppo.
    #[must_use]
    pub fn is_debtor(&self) -> bool {
        self.net_balance.amount() < Decimal::ZERO
    }

    /// `true` se è in pari.
    ///
    /// **Chi è in pari e ha lasciato il gruppo non compare affatto** nella vista
    /// dalla migration `0027`: questo metodo risponde `true` solo per chi è ancora
    /// membro attivo. Non è un caso limite da coprire, è la definizione del
    /// filtro.
    #[must_use]
    pub fn is_settled_up(&self) -> bool {
        self.net_balance.amount() == Decimal::ZERO
    }
}
