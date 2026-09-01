use chrono::{DateTime, Utc};

use super::split::SplitMethod;
use super::value_objects::Weight;
use crate::error::{DomainError, DomainResult, Invariant};
use crate::shared::{
    Currency, ExpenseGroupId, ExpenseGroupMemberId, OrgId, PropertyId, RoomId, UserId, text,
};

/// Che cosa il gruppo tiene insieme. Rispecchia `app.expense_group_kind` per
/// intero: nessuno dei tre valori è fuori perimetro, perché descrivono l'uso e non
/// una capacità del software.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpenseGroupKind {
    /// Coinquilini che dividono le spese di casa. È il default dello schema, ed è
    /// il caso per cui il modulo esiste.
    Coliving,
    /// Un nucleo familiare.
    Household,
    /// Le spese di un immobile, tenute insieme senza essere un co-living.
    Property,
}

impl ExpenseGroupKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Coliving => "coliving",
            Self::Household => "household",
            Self::Property => "property",
        }
    }
}

/// Dati necessari a creare un gruppo di spesa.
///
/// **Non c'è `currency`**, per la ragione di ADR-030: la valuta è una sola. Non
/// c'è `is_archived`, che nasce falso.
#[derive(Debug, Clone)]
pub struct NewExpenseGroup {
    pub id: ExpenseGroupId,
    pub org_id: OrgId,
    /// L'immobile a cui il gruppo si riferisce, se ce n'è uno.
    ///
    /// È **opzionale a schema**, e la conseguenza va saputa: un gruppo senza
    /// immobile non è raggiungibile partendo da una proprietà, ed è la ragione per
    /// cui la ripartizione automatica di una bolletta resta fuori da questa fetta
    /// — non c'è modo di rispondere alla domanda «quale gruppo riguarda questa
    /// bolletta».
    pub property_id: Option<PropertyId>,
    pub kind: ExpenseGroupKind,
    pub name: String,
    pub default_split_method: SplitMethod,
    /// Chi lo crea, e che per la migration `0026` può iscrivere sé stesso.
    pub created_by: UserId,
}

/// Un gruppo di spesa: il contenitore di un conto fra pari.
///
/// **Il proprietario può guardarlo ma non ne è parte.**
/// `can_access_expense_group` risponde vero per lo staff dell'organizzazione, che
/// vede quindi spese e saldi; ma un gruppo è denaro che i coinquilini si devono
/// **fra loro**, e nessuna riga di questo modulo entra nella contabilità
/// dell'organizzazione.
///
/// **Nessun `INSERT … RETURNING` su questa tabella.** `expense_groups_select`
/// chiama `can_access_expense_group(id)`, cioè interroga **la riga che si sta
/// inserendo**, e la funzione è `STABLE`: vede lo snapshot di inizio statement, in
/// cui la riga non c'è. È il muro già incontrato sulle segnalazioni, e la regola
/// generale vale ancora — il muro c'è solo quando la policy di `SELECT` dipende
/// dalla riga stessa. Sugli addebiti non c'era, perché `charges_select` interroga
/// `app.leases`. Qui c'è. L'identificatore lo assegna il dominio, come ovunque,
/// quindi non se ne sente la mancanza.
#[derive(Debug, Clone)]
pub struct ExpenseGroup {
    id: ExpenseGroupId,
    org_id: OrgId,
    property_id: Option<PropertyId>,
    kind: ExpenseGroupKind,
    name: String,
    currency: Currency,
    default_split_method: SplitMethod,
    created_by: Option<UserId>,
    is_archived: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ExpenseGroup {
    /// `length(btrim(name)) BETWEEN 1 AND 120`, come lo schema.
    pub const MAX_NAME_LENGTH: usize = 120;

    pub fn create(parts: NewExpenseGroup, now: DateTime<Utc>) -> DomainResult<Self> {
        let NewExpenseGroup {
            id,
            org_id,
            property_id,
            kind,
            name,
            default_split_method,
            created_by,
        } = parts;

        // Un gruppo che nasce con un metodo che non sappiamo calcolare è un
        // gruppo in cui la prima spesa fallirà: meglio dirlo alla creazione.
        if !default_split_method.is_supported() {
            return Err(DomainError::validation(
                "default_split_method",
                "only `equal` and `weighted` are supported",
            ));
        }

        Ok(Self {
            id,
            org_id,
            property_id,
            kind,
            name: text::required("name", &name, Self::MAX_NAME_LENGTH)?,
            currency: Currency::EUR,
            default_split_method,
            created_by: Some(created_by),
            is_archived: false,
            created_at: now,
            updated_at: now,
        })
    }

    /// Archivia il gruppo. Non è un soft delete: la riga resta leggibile e le
    /// spese con lei, perché un conto chiuso si consulta.
    pub fn archive(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if self.is_archived {
            return Err(DomainError::Invariant(Invariant::ExpenseGroupArchived));
        }
        self.is_archived = true;
        self.updated_at = now;
        Ok(())
    }

    /// Rifiuta le operazioni su un gruppo archiviato.
    ///
    /// Vive sull'aggregato e non nello use case perché è un'invariante del gruppo:
    /// registrare una spesa in un conto chiuso è la stessa cosa che riaprirlo di
    /// nascosto.
    pub const fn ensure_open(&self) -> DomainResult<()> {
        if self.is_archived {
            return Err(DomainError::Invariant(Invariant::ExpenseGroupArchived));
        }
        Ok(())
    }

    #[must_use]
    pub const fn id(&self) -> ExpenseGroupId {
        self.id
    }
    #[must_use]
    pub const fn org_id(&self) -> OrgId {
        self.org_id
    }
    #[must_use]
    pub const fn property_id(&self) -> Option<PropertyId> {
        self.property_id
    }
    #[must_use]
    pub const fn kind(&self) -> ExpenseGroupKind {
        self.kind
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub const fn currency(&self) -> Currency {
        self.currency
    }
    #[must_use]
    pub const fn default_split_method(&self) -> SplitMethod {
        self.default_split_method
    }
    #[must_use]
    pub const fn created_by(&self) -> Option<UserId> {
        self.created_by
    }
    #[must_use]
    pub const fn is_archived(&self) -> bool {
        self.is_archived
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
    pub fn from_persistence(parts: ExpenseGroupParts) -> Self {
        let ExpenseGroupParts {
            id,
            org_id,
            property_id,
            kind,
            name,
            currency,
            default_split_method,
            created_by,
            is_archived,
            created_at,
            updated_at,
        } = parts;
        Self {
            id,
            org_id,
            property_id,
            kind,
            name,
            currency,
            default_split_method,
            created_by,
            is_archived,
            created_at,
            updated_at,
        }
    }
}

/// Dati grezzi di un gruppo letto dalla persistenza.
#[derive(Debug, Clone)]
pub struct ExpenseGroupParts {
    pub id: ExpenseGroupId,
    pub org_id: OrgId,
    pub property_id: Option<PropertyId>,
    pub kind: ExpenseGroupKind,
    pub name: String,
    pub currency: Currency,
    pub default_split_method: SplitMethod,
    pub created_by: Option<UserId>,
    pub is_archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Dati necessari a iscrivere qualcuno a un gruppo.
#[derive(Debug, Clone)]
pub struct NewExpenseGroupMember {
    pub id: ExpenseGroupMemberId,
    pub org_id: OrgId,
    pub group_id: ExpenseGroupId,
    pub user_id: UserId,
    /// La stanza occupata, quando il gruppo è legato a un immobile in co-living.
    pub room_id: Option<RoomId>,
    /// Il nome da mostrare dentro il gruppo, se diverso da quello dell'anagrafica.
    pub display_name: Option<String>,
    pub weight: Weight,
}

/// La membership di una persona in un gruppo di spesa.
///
/// **Uscire non cancella la riga**: valorizza `left_at`. La riga deve restare,
/// perché le quote già intestate a quella persona restano, e perché
/// `app.v_expense_group_balances` parte proprio da qui — dalla migration `0027`,
/// chi esce con un conto aperto continua a comparire nei saldi finché il suo netto
/// non è zero.
///
/// **Rientrare è un `UPDATE`, non un `INSERT`.**
/// `expense_group_members_group_user_uk` è unico su `(group_id, user_id)` e **non
/// è parziale**: chi esce e vuole rientrare non può avere una seconda riga, e chi
/// scrive `join` come un inserimento incondizionato prende un conflitto sul
/// vincolo. Da qui [`ExpenseGroupMember::rejoin`].
#[derive(Debug, Clone)]
pub struct ExpenseGroupMember {
    id: ExpenseGroupMemberId,
    org_id: OrgId,
    group_id: ExpenseGroupId,
    user_id: UserId,
    room_id: Option<RoomId>,
    display_name: Option<String>,
    weight: Weight,
    joined_at: DateTime<Utc>,
    left_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ExpenseGroupMember {
    /// Come `app.users.full_name`: il nome mostrato non ha un limite proprio a
    /// schema — `display_name` è `TEXT` — e questo è il nostro.
    pub const MAX_DISPLAY_NAME_LENGTH: usize = 200;

    pub fn join(parts: NewExpenseGroupMember, now: DateTime<Utc>) -> DomainResult<Self> {
        let NewExpenseGroupMember {
            id,
            org_id,
            group_id,
            user_id,
            room_id,
            display_name,
            weight,
        } = parts;

        Ok(Self {
            id,
            org_id,
            group_id,
            user_id,
            room_id,
            display_name: text::optional(
                "display_name",
                display_name,
                Self::MAX_DISPLAY_NAME_LENGTH,
            )?,
            weight,
            joined_at: now,
            left_at: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Esce dal gruppo: `left_at` valorizzato, la riga **resta**.
    pub fn leave(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if self.left_at.is_some() {
            return Err(DomainError::Invariant(Invariant::GroupMemberAlreadyLeft));
        }
        self.left_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    /// Rientra: `left_at` torna nullo, e `joined_at` con esso alla data del
    /// rientro.
    ///
    /// `joined_at` si riscrive perché `expense_group_members_dates_ck` è
    /// `left_at IS NULL OR left_at >= joined_at`: lasciare la data vecchia sarebbe
    /// ammesso dal vincolo ma direbbe che la persona è nel gruppo da prima di
    /// esserci tornata. **Ed è precisamente la ragione per cui l'ordine del resto
    /// nella ripartizione non è per `joined_at`**: questa colonna cambia.
    ///
    /// **Il peso si ridichiara**, e non è un dettaglio: chi rientra può prendere
    /// una stanza diversa, e un rientro che conservasse in silenzio il peso
    /// vecchio farebbe pagare la doppia a chi è tornato nella singola. Il
    /// parametro è obbligatorio proprio per questo — con un `Option` che ricade
    /// sul valore precedente, dimenticarlo sarebbe indistinguibile dal volerlo.
    pub fn rejoin(&mut self, weight: Weight, now: DateTime<Utc>) -> DomainResult<()> {
        if self.left_at.is_none() {
            return Err(DomainError::Invariant(Invariant::GroupMemberAlreadyIn));
        }
        self.left_at = None;
        self.joined_at = now;
        self.weight = weight;
        self.updated_at = now;
        Ok(())
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.left_at.is_none()
    }

    #[must_use]
    pub const fn id(&self) -> ExpenseGroupMemberId {
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
    pub const fn user_id(&self) -> UserId {
        self.user_id
    }
    #[must_use]
    pub const fn room_id(&self) -> Option<RoomId> {
        self.room_id
    }
    #[must_use]
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }
    #[must_use]
    pub const fn weight(&self) -> Weight {
        self.weight
    }
    #[must_use]
    pub const fn joined_at(&self) -> DateTime<Utc> {
        self.joined_at
    }
    #[must_use]
    pub const fn left_at(&self) -> Option<DateTime<Utc>> {
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

    /// Ricostruzione da persistenza: non riesegue le invarianti.
    #[must_use]
    pub fn from_persistence(parts: ExpenseGroupMemberParts) -> Self {
        let ExpenseGroupMemberParts {
            id,
            org_id,
            group_id,
            user_id,
            room_id,
            display_name,
            weight,
            joined_at,
            left_at,
            created_at,
            updated_at,
        } = parts;
        Self {
            id,
            org_id,
            group_id,
            user_id,
            room_id,
            display_name,
            weight,
            joined_at,
            left_at,
            created_at,
            updated_at,
        }
    }
}

/// Dati grezzi di una membership letta dalla persistenza.
#[derive(Debug, Clone)]
pub struct ExpenseGroupMemberParts {
    pub id: ExpenseGroupMemberId,
    pub org_id: OrgId,
    pub group_id: ExpenseGroupId,
    pub user_id: UserId,
    pub room_id: Option<RoomId>,
    pub display_name: Option<String>,
    pub weight: Weight,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_group() -> NewExpenseGroup {
        NewExpenseGroup {
            id: ExpenseGroupId::generate(),
            org_id: OrgId::generate(),
            property_id: None,
            kind: ExpenseGroupKind::Coliving,
            name: "  Casa di via Roma  ".to_owned(),
            default_split_method: SplitMethod::Equal,
            created_by: UserId::generate(),
        }
    }

    fn new_member() -> NewExpenseGroupMember {
        NewExpenseGroupMember {
            id: ExpenseGroupMemberId::generate(),
            org_id: OrgId::generate(),
            group_id: ExpenseGroupId::generate(),
            user_id: UserId::generate(),
            room_id: None,
            display_name: None,
            weight: Weight::one(),
        }
    }

    #[test]
    fn a_group_trims_its_name_and_bounds_it() {
        let group = ExpenseGroup::create(new_group(), Utc::now()).expect("valido");
        assert_eq!(group.name(), "Casa di via Roma");
        assert!(!group.is_archived());

        assert!(
            ExpenseGroup::create(
                NewExpenseGroup {
                    name: "   ".to_owned(),
                    ..new_group()
                },
                Utc::now()
            )
            .is_err()
        );
        assert!(
            ExpenseGroup::create(
                NewExpenseGroup {
                    name: "n".repeat(ExpenseGroup::MAX_NAME_LENGTH + 1),
                    ..new_group()
                },
                Utc::now()
            )
            .is_err()
        );
    }

    /// Un gruppo che nascesse con un metodo non calcolabile fallirebbe alla prima
    /// spesa, che è il momento peggiore per accorgersene.
    #[test]
    fn a_group_refuses_a_default_method_it_cannot_compute() {
        for method in [
            SplitMethod::Percentage,
            SplitMethod::Exact,
            SplitMethod::ByUsage,
        ] {
            assert!(
                ExpenseGroup::create(
                    NewExpenseGroup {
                        default_split_method: method,
                        ..new_group()
                    },
                    Utc::now()
                )
                .is_err(),
                "{}",
                method.as_str()
            );
        }
    }

    #[test]
    fn an_archived_group_refuses_a_second_archiving_and_every_operation() {
        let mut group = ExpenseGroup::create(new_group(), Utc::now()).expect("valido");
        assert!(group.ensure_open().is_ok());

        group.archive(Utc::now()).expect("archiviato");
        assert!(group.is_archived());
        assert!(group.archive(Utc::now()).is_err());
        assert!(group.ensure_open().is_err());
    }

    /// **Uscire non cancella la riga**, e rientrare è un aggiornamento: il vincolo
    /// unico su `(group_id, user_id)` non è parziale, quindi una seconda riga non
    /// esiste.
    #[test]
    fn leaving_keeps_the_row_and_rejoining_is_an_update() {
        let now = Utc::now();
        let mut member = ExpenseGroupMember::join(new_member(), now).expect("valido");
        assert!(member.is_active());
        assert_eq!(member.left_at(), None);

        member.leave(now).expect("uscito");
        assert!(!member.is_active());
        assert_eq!(member.left_at(), Some(now));
        assert!(member.leave(now).is_err(), "non si esce due volte");

        let later = now + chrono::Duration::days(30);
        let double = Weight::parse(rust_decimal::Decimal::TWO).expect("peso valido");
        member.rejoin(double, later).expect("rientrato");
        assert!(member.is_active());
        assert_eq!(
            member.joined_at(),
            later,
            "`joined_at` segue il rientro: `expense_group_members_dates_ck` \
             ammetterebbe la data vecchia, ma direbbe una cosa falsa"
        );
        assert_eq!(
            member.weight(),
            double,
            "chi rientra ridichiara la propria stanza: conservare il peso vecchio \
             farebbe pagare la doppia a chi è tornato nella singola"
        );
        assert!(
            member.rejoin(double, later).is_err(),
            "non si rientra due volte"
        );
    }
}
