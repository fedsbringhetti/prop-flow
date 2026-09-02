use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;

use super::split::{Participant, Share, Split, SplitMethod};
use super::value_objects::Weight;
use crate::error::{DomainError, DomainResult};
use crate::shared::{
    Currency, ExpenseGroupId, ExpenseId, ExpenseShareId, Money, OrgId, UserId, text,
};

/// Che cosa si è comprato. Rispecchia `app.expense_category` per intero: sono
/// etichette, e nessuna richiede una capacità che non abbiamo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpenseCategory {
    Utilities,
    Groceries,
    Cleaning,
    Connectivity,
    Maintenance,
    Rent,
    Furniture,
    Other,
}

impl ExpenseCategory {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Utilities => "utilities",
            Self::Groceries => "groceries",
            Self::Cleaning => "cleaning",
            Self::Connectivity => "connectivity",
            Self::Maintenance => "maintenance",
            Self::Rent => "rent",
            Self::Furniture => "furniture",
            Self::Other => "other",
        }
    }
}

/// Dati necessari a registrare una spesa.
///
/// **Non c'è `currency`**, per la ragione di ADR-030. **Non ci sono
/// `utility_bill_id` né `document_id`**: la ripartizione automatica di una bolletta
/// e gli allegati alle spese sono fuori dal perimetro di questa fetta, e le due
/// colonne restano nulle.
#[derive(Debug, Clone)]
pub struct NewExpense {
    pub id: ExpenseId,
    pub org_id: OrgId,
    pub group_id: ExpenseGroupId,
    /// **Chi ha anticipato**, che è il verso del credito e non un dettaglio
    /// anagrafico.
    ///
    /// Su questa colonna lo schema **non ha alcun vincolo verso la membership**:
    /// si può dichiarare che ha anticipato una persona che nel gruppo non c'è, e
    /// nulla protesterebbe. Il risultato sarebbe peggio di un errore — la vista
    /// dei saldi parte dai **membri**, quindi quel credito non comparirebbe a
    /// nessuno e la somma dei saldi non farebbe più zero. A impedirlo è lo use
    /// case, che verifica contro i membri attivi.
    pub paid_by_user_id: UserId,
    pub category: ExpenseCategory,
    pub description: String,
    pub amount: Decimal,
    pub expense_date: NaiveDate,
    pub split_method: SplitMethod,
    pub notes: Option<String>,
    pub created_by: UserId,
}

/// Una spesa e le sue quote, costruite **insieme**.
///
/// Non esiste una spesa senza quote: il trigger `expense_shares_balanced` impone
/// che la somma delle quote sia esattamente l'importo, e lo impone **al `COMMIT`**
/// perché è `DEFERRABLE INITIALLY DEFERRED`. Un tipo che permettesse di costruire
/// l'una senza le altre renderebbe rappresentabile uno stato che il database
/// rifiuta, e lo scoprirebbe alla fine della transazione invece che qui.
#[derive(Debug, Clone)]
pub struct RecordedExpense {
    pub expense: Expense,
    pub shares: Vec<ExpenseShare>,
}

/// Una spesa condivisa: qualcuno ha anticipato, e il gruppo gliela deve.
///
/// **Non c'è uno stato.** `app.expenses` non ha una colonna di stato — verificato,
/// non dedotto — e ne segue che una spesa sbagliata si **annulla con il soft
/// delete** e si riscrive. È la differenza con l'addebito, che si annulla passando
/// a `void` e resta visibile: là `deleted_at` è fra le colonne negate dalla
/// migration `0023`, qui è fra le concesse dalla `0028`.
///
/// **`amount` non si riscrive**, ed è la ragione per cui la `0028` esiste: il
/// trigger che difende l'invariante contabile è su `app.expense_shares` e non
/// scatta su una `UPDATE` di questa tabella, quindi cambiare l'importo senza
/// toccare le quote sbilancerebbe i conti in silenzio. Il privilegio di colonna è
/// l'unica difesa che copre quella porta.
#[derive(Debug, Clone)]
pub struct Expense {
    id: ExpenseId,
    org_id: OrgId,
    group_id: ExpenseGroupId,
    paid_by_user_id: UserId,
    category: ExpenseCategory,
    description: String,
    amount: Money,
    currency: Currency,
    expense_date: NaiveDate,
    split_method: SplitMethod,
    notes: Option<String>,
    created_by: Option<UserId>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Expense {
    /// `length(btrim(description)) BETWEEN 1 AND 300`, come lo schema.
    pub const MAX_DESCRIPTION_LENGTH: usize = 300;
    /// `notes` è `TEXT` senza vincoli a schema: il tetto è nostro.
    pub const MAX_NOTES_LENGTH: usize = 2_000;

    /// Registra la spesa e ne calcola le quote, in un colpo solo.
    ///
    /// `participants` sono i **membri attivi** del gruppo. Chi ha anticipato non è
    /// tenuto a essere fra loro — si può pagare per gli altri senza partecipare
    /// alla spesa — ma **deve** essere un membro, e a verificarlo è lo use case:
    /// vedi la nota su [`NewExpense::paid_by_user_id`].
    pub fn record(
        parts: NewExpense,
        participants: &[Participant],
        now: DateTime<Utc>,
    ) -> DomainResult<RecordedExpense> {
        let NewExpense {
            id,
            org_id,
            group_id,
            paid_by_user_id,
            category,
            description,
            amount,
            expense_date,
            split_method,
            notes,
            created_by,
        } = parts;

        // `> 0` come `app.expenses.amount CHECK (amount > 0)`: uno zero non è una
        // spesa, e un negativo è un rimborso, che questo modulo non modella.
        let currency = Currency::EUR;
        let amount = Money::new(amount, currency)
            .map_err(|_| DomainError::validation("amount", "must be a valid amount"))?;
        if amount.amount() <= Decimal::ZERO {
            return Err(DomainError::validation(
                "amount",
                "must be greater than zero",
            ));
        }

        let expense = Self {
            id,
            org_id,
            group_id,
            paid_by_user_id,
            category,
            description: text::required("description", &description, Self::MAX_DESCRIPTION_LENGTH)?,
            amount,
            currency,
            expense_date,
            split_method,
            notes: text::optional("notes", notes, Self::MAX_NOTES_LENGTH)?,
            created_by: Some(created_by),
            created_at: now,
            updated_at: now,
        };

        let shares = Split::by_method(split_method, amount, participants)?
            .into_iter()
            .map(|share| ExpenseShare::from_split(org_id, id, share, now))
            .collect();

        Ok(RecordedExpense { expense, shares })
    }

    #[must_use]
    pub const fn id(&self) -> ExpenseId {
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
    pub const fn paid_by_user_id(&self) -> UserId {
        self.paid_by_user_id
    }
    #[must_use]
    pub const fn category(&self) -> ExpenseCategory {
        self.category
    }
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
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
    pub const fn expense_date(&self) -> NaiveDate {
        self.expense_date
    }
    #[must_use]
    pub const fn split_method(&self) -> SplitMethod {
        self.split_method
    }
    #[must_use]
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }
    #[must_use]
    pub const fn created_by(&self) -> Option<UserId> {
        self.created_by
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
    pub fn from_persistence(parts: ExpenseParts) -> Self {
        let ExpenseParts {
            id,
            org_id,
            group_id,
            paid_by_user_id,
            category,
            description,
            amount,
            currency,
            expense_date,
            split_method,
            notes,
            created_by,
            created_at,
            updated_at,
        } = parts;
        Self {
            id,
            org_id,
            group_id,
            paid_by_user_id,
            category,
            description,
            amount,
            currency,
            expense_date,
            split_method,
            notes,
            created_by,
            created_at,
            updated_at,
        }
    }
}

/// Dati grezzi di una spesa letta dalla persistenza.
#[derive(Debug, Clone)]
pub struct ExpenseParts {
    pub id: ExpenseId,
    pub org_id: OrgId,
    pub group_id: ExpenseGroupId,
    pub paid_by_user_id: UserId,
    pub category: ExpenseCategory,
    pub description: String,
    pub amount: Money,
    pub currency: Currency,
    pub expense_date: NaiveDate,
    pub split_method: SplitMethod,
    pub notes: Option<String>,
    pub created_by: Option<UserId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Quanto una persona deve di una spesa.
///
/// **`is_settled` e `settled_at` non si scrivono, e restano ai loro default.**
/// `app.v_expense_group_balances` non li guarda: il saldo netto è
/// `anticipato − dovuto + uscite − entrate`, tutto derivato dai movimenti.
/// Scriverli sarebbe una **seconda definizione di «saldato»** che nessuno
/// riconcilia — esattamente il rapporto fra `charges.status` e
/// `v_charge_balances` — con l'aggravante che qui non esiste un'operazione che li
/// produca: un regolamento è un **trasferimento netto** fra due persone, e
/// imputarlo alle singole quote richiederebbe un algoritmo di allocazione che
/// nessuno ha chiesto e che nessuna colonna registra.
///
/// Restano quindi `false` e nulli, come `refunded_amount` sui pagamenti, e va
/// saputo leggendoli.
#[derive(Debug, Clone)]
pub struct ExpenseShare {
    id: ExpenseShareId,
    org_id: OrgId,
    expense_id: ExpenseId,
    user_id: UserId,
    share_amount: Money,
    share_weight: Option<Weight>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ExpenseShare {
    fn from_split(org_id: OrgId, expense_id: ExpenseId, share: Share, now: DateTime<Utc>) -> Self {
        Self {
            id: ExpenseShareId::generate(),
            org_id,
            expense_id,
            user_id: share.user_id,
            share_amount: share.amount,
            share_weight: share.weight,
            created_at: now,
            updated_at: now,
        }
    }

    #[must_use]
    pub const fn id(&self) -> ExpenseShareId {
        self.id
    }
    #[must_use]
    pub const fn org_id(&self) -> OrgId {
        self.org_id
    }
    #[must_use]
    pub const fn expense_id(&self) -> ExpenseId {
        self.expense_id
    }
    #[must_use]
    pub const fn user_id(&self) -> UserId {
        self.user_id
    }
    #[must_use]
    pub const fn share_amount(&self) -> Money {
        self.share_amount
    }
    #[must_use]
    pub const fn share_weight(&self) -> Option<Weight> {
        self.share_weight
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
    pub fn from_persistence(parts: ExpenseShareParts) -> Self {
        let ExpenseShareParts {
            id,
            org_id,
            expense_id,
            user_id,
            share_amount,
            share_weight,
            created_at,
            updated_at,
        } = parts;
        Self {
            id,
            org_id,
            expense_id,
            user_id,
            share_amount,
            share_weight,
            created_at,
            updated_at,
        }
    }
}

/// Dati grezzi di una quota letta dalla persistenza.
#[derive(Debug, Clone)]
pub struct ExpenseShareParts {
    pub id: ExpenseShareId,
    pub org_id: OrgId,
    pub expense_id: ExpenseId,
    pub user_id: UserId,
    pub share_amount: Money,
    pub share_weight: Option<Weight>,
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

    fn new_expense() -> NewExpense {
        NewExpense {
            id: ExpenseId::generate(),
            org_id: OrgId::generate(),
            group_id: ExpenseGroupId::generate(),
            paid_by_user_id: UserId::generate(),
            category: ExpenseCategory::Groceries,
            description: "  Spesa alla Coop  ".to_owned(),
            amount: dec("100.00"),
            expense_date: NaiveDate::from_ymd_opt(2026, 9, 1).expect("data valida"),
            split_method: SplitMethod::Equal,
            notes: None,
            created_by: UserId::generate(),
        }
    }

    fn three_participants() -> Vec<Participant> {
        (0..3)
            .map(|_| Participant {
                user_id: UserId::generate(),
                weight: Weight::one(),
            })
            .collect()
    }

    /// Spesa e quote nascono **insieme**, e le quote sommano all'importo: è
    /// l'invariante che il trigger difende al commit.
    #[test]
    fn an_expense_is_born_with_its_shares_and_they_add_up() {
        let recorded =
            Expense::record(new_expense(), &three_participants(), Utc::now()).expect("registrata");

        assert_eq!(recorded.expense.description(), "Spesa alla Coop");
        assert_eq!(recorded.shares.len(), 3);
        assert_eq!(
            recorded
                .shares
                .iter()
                .map(|share| share.share_amount().amount())
                .sum::<Decimal>(),
            recorded.expense.amount().amount()
        );
    }

    /// Le quote di una divisione in parti uguali non portano il peso: non ha
    /// partecipato al calcolo.
    #[test]
    fn an_equal_split_writes_no_weight_on_its_shares() {
        let recorded =
            Expense::record(new_expense(), &three_participants(), Utc::now()).expect("registrata");
        assert!(
            recorded
                .shares
                .iter()
                .all(|share| share.share_weight().is_none())
        );
    }

    #[test]
    fn a_weighted_split_carries_the_weight_onto_the_shares() {
        let recorded = Expense::record(
            NewExpense {
                split_method: SplitMethod::Weighted,
                ..new_expense()
            },
            &three_participants(),
            Utc::now(),
        )
        .expect("registrata");
        assert!(
            recorded
                .shares
                .iter()
                .all(|share| share.share_weight().is_some())
        );
    }

    #[test]
    fn an_amount_that_is_not_a_purchase_is_refused() {
        for amount in ["0", "0.00", "-10.00", "10.1234"] {
            assert!(
                Expense::record(
                    NewExpense {
                        amount: dec(amount),
                        ..new_expense()
                    },
                    &three_participants(),
                    Utc::now()
                )
                .is_err(),
                "`{amount}` non è una spesa"
            );
        }
    }

    /// Una spesa senza partecipanti non è registrabile: le quote non
    /// sommerebbero all'importo, e il trigger la rifiuterebbe al commit.
    #[test]
    fn an_expense_without_participants_is_refused_before_the_commit() {
        assert!(Expense::record(new_expense(), &[], Utc::now()).is_err());
    }

    /// I tre metodi fuori perimetro fermano la registrazione invece di ricadere
    /// su `equal`.
    #[test]
    fn an_unsupported_split_method_stops_the_expense() {
        for method in [
            SplitMethod::Percentage,
            SplitMethod::Exact,
            SplitMethod::ByUsage,
        ] {
            assert!(
                Expense::record(
                    NewExpense {
                        split_method: method,
                        ..new_expense()
                    },
                    &three_participants(),
                    Utc::now()
                )
                .is_err(),
                "{}",
                method.as_str()
            );
        }
    }
}
