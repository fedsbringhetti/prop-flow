use std::sync::Arc;

use propflow_domain::events::{
    DomainEventPayload, ExpenseGroupCreated, ExpenseGroupMemberJoined, ExpenseGroupMemberLeft,
};
use propflow_domain::expense::{
    ExpenseGroup, ExpenseGroupKind, ExpenseGroupMember, NewExpenseGroup, NewExpenseGroupMember,
    SplitMethod, Weight,
};
use propflow_domain::ports::{Clock, ExpenseGroupPage, ExpenseGroupQuery, UnitOfWorkFactory};
use propflow_domain::shared::{ExpenseGroupId, ExpenseGroupMemberId, PropertyId, RoomId, UserId};
use propflow_domain::{Denial, Entity, RequestContext};

use super::shared::{append_all, map_membership_conflict, open_group, visible_group};
use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct CreateExpenseGroupCommand {
    pub property_id: Option<PropertyId>,
    pub kind: ExpenseGroupKind,
    pub name: String,
    pub default_split_method: SplitMethod,
    /// Il peso di chi crea, che è anche il primo membro.
    pub weight: Weight,
    pub display_name: Option<String>,
}

/// Crea un gruppo di spesa, e **iscrive chi lo crea nella stessa transazione**.
///
/// I due fatti sono uno solo, ed è la ragione per cui esiste la migration `0026`.
/// Lo schema, letto come usciva dalla `0014`, produceva un gruppo **inutilizzabile
/// da chi lo aveva creato**: `expense_groups_insert` chiede `is_org_member`, che un
/// inquilino soddisfa, ma `expense_group_members_insert` chiedeva
/// `can_access_expense_group`, che appena creato il gruppo è falsa proprio per lui.
/// Il gruppo nasceva vuoto e restava vuoto, a meno che non passasse lo staff.
///
/// `ensure_member` e non `ensure_staff`, e non è una concessione: la policy
/// dell'inserimento era già `is_org_member(org_id)` dalla `0014`. Un co-living che
/// non può organizzarsi da sé non è il prodotto descritto in `docs/01-prodotto.md`.
///
/// **Se l'iscrizione fallisce, cade anche il gruppo.** Sono nella stessa
/// transazione apposta: un gruppo senza il proprio creatore è precisamente il
/// difetto che la `0026` corregge, e lasciarlo scritto sarebbe peggio di non
/// averlo scritto affatto.
pub struct CreateExpenseGroup {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl CreateExpenseGroup {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>, clock: Arc<dyn Clock>) -> Self {
        Self {
            unit_of_work,
            clock,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        command: CreateExpenseGroupCommand,
    ) -> AppResult<(ExpenseGroup, ExpenseGroupMember)> {
        let org_id = authorization::ensure_member(context)?;
        let created_by = context
            .user_id()
            .ok_or(AppError::Forbidden(Denial::AuthenticationRequired))?;
        let now = self.clock.now();

        let group = ExpenseGroup::create(
            NewExpenseGroup {
                id: ExpenseGroupId::generate(),
                org_id,
                property_id: command.property_id,
                kind: command.kind,
                name: command.name,
                default_split_method: command.default_split_method,
                created_by,
            },
            now,
        )?;

        let member = ExpenseGroupMember::join(
            NewExpenseGroupMember {
                id: ExpenseGroupMemberId::generate(),
                org_id,
                group_id: group.id(),
                user_id: created_by,
                room_id: None,
                display_name: command.display_name,
                weight: command.weight,
            },
            now,
        )?;

        let mut uow = self.unit_of_work.begin(context).await?;
        uow.expense_groups().insert(&group).await?;
        uow.expense_groups()
            .insert_member(&member)
            .await
            .map_err(map_membership_conflict)?;

        append_all(
            uow.as_mut(),
            context,
            now,
            vec![
                DomainEventPayload::ExpenseGroupCreated(ExpenseGroupCreated::new(
                    &group, created_by,
                )),
                DomainEventPayload::ExpenseGroupMemberJoined(ExpenseGroupMemberJoined::new(
                    &member, false,
                )),
            ],
        )
        .await?;
        uow.commit().await?;

        Ok((group, member))
    }
}

/// Legge un gruppo.
pub struct GetExpenseGroup {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl GetExpenseGroup {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    /// Restituisce il gruppo **con la sua composizione**, chi è uscito compreso.
    ///
    /// Chi è uscito c'è perché senza di lui un client non saprebbe spiegare le
    /// quote di una spesa vecchia — e perché dalla migration `0027` compare
    /// comunque nei saldi finché ha un conto aperto, quindi ometterlo qui darebbe
    /// due elenchi che non si somigliano.
    pub async fn execute(
        &self,
        context: &RequestContext,
        id: ExpenseGroupId,
    ) -> AppResult<(ExpenseGroup, Vec<ExpenseGroupMember>)> {
        authorization::ensure_member(context)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        let group = visible_group(uow.as_mut(), id).await?;
        let members = uow.expense_groups().list_members(id, false).await?;
        Ok((group, members))
    }
}

#[derive(Debug, Clone)]
pub struct ListExpenseGroupsQuery {
    pub property_id: Option<PropertyId>,
    pub is_archived: Option<bool>,
    pub after: Option<ExpenseGroupId>,
    pub limit: Option<u16>,
}

/// Elenca i gruppi di spesa.
///
/// `ensure_member`, e *quali* gruppi siano di chi li chiede lo decide
/// `expense_groups_select` — staff, oppure chi ha una membership attiva — non un
/// ramo applicativo sul ruolo. È la stessa scelta delle segnalazioni, dei
/// contratti e della contabilità.
pub struct ListExpenseGroups {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl ListExpenseGroups {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        query: ListExpenseGroupsQuery,
    ) -> AppResult<ExpenseGroupPage> {
        authorization::ensure_member(context)?;

        // Il limite si tronca invece di essere rifiutato: un client che chiede
        // 10 000 righe ottiene la pagina massima, non un errore da gestire.
        let limit = query
            .limit
            .unwrap_or(ExpenseGroupQuery::DEFAULT_LIMIT)
            .clamp(1, ExpenseGroupQuery::MAX_LIMIT);

        let mut uow = self.unit_of_work.begin(context).await?;
        uow.expense_groups()
            .list(&ExpenseGroupQuery {
                property_id: query.property_id,
                is_archived: query.is_archived,
                after: query.after,
                limit,
            })
            .await
            .map_err(AppError::from)
    }
}

/// Archivia un gruppo: il conto si chiude e non accetta più spese.
///
/// **Non è un soft delete**: la riga resta leggibile e le spese con lei, perché un
/// conto chiuso si consulta. È la differenza con la spesa, dove il soft delete *è*
/// l'annullamento.
///
/// `ensure_member`: chiudere il conto della casa in cui si abita è un'operazione
/// dei coinquilini, non del proprietario, e `expense_groups_update` chiede
/// `can_access_expense_group` — cioè esattamente questo.
///
/// **Archiviare non guarda i saldi, e non è una svista.** Un gruppo si chiude
/// anche con dei conti aperti: chi ha lasciato la casa senza pagare non deve poter
/// tenere il gruppo in ostaggio, ed è lo stesso argomento per cui la migration
/// `0027` corregge la vista invece di rifiutare l'uscita a saldo aperto. I saldi
/// restano leggibili dopo l'archiviazione.
pub struct ArchiveExpenseGroup {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl ArchiveExpenseGroup {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>, clock: Arc<dyn Clock>) -> Self {
        Self {
            unit_of_work,
            clock,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        id: ExpenseGroupId,
    ) -> AppResult<ExpenseGroup> {
        authorization::ensure_member(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut group = visible_group(uow.as_mut(), id).await?;
        group.archive(now)?;
        uow.expense_groups().update(&group).await?;
        uow.commit().await?;

        Ok(group)
    }
}

#[derive(Debug, Clone)]
pub struct AddGroupMemberCommand {
    pub group_id: ExpenseGroupId,
    pub user_id: UserId,
    pub room_id: Option<RoomId>,
    pub display_name: Option<String>,
    pub weight: Weight,
}

/// Aggiunge qualcuno al gruppo, **o lo fa rientrare**.
///
/// Le due cose sono un'operazione sola perché lo schema le rende tali:
/// `expense_group_members_group_user_uk` è unico su `(group_id, user_id)` e **non
/// è parziale**, quindi chi è uscito non può avere una seconda riga. La membership
/// si rilegge prima di scrivere, e il rientro è un `UPDATE`.
///
/// **Chi scrive questo come un `INSERT` incondizionato prende un conflitto sul
/// vincolo**, ed è la trappola che il piano segnalava.
///
/// `ensure_member`: la policy chiede `can_access_expense_group`, cioè un membro
/// attivo o lo staff. Il gruppo si rilegge prima per dare un `404` invece di
/// `auth.denied_by_policy`.
pub struct AddGroupMember {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl AddGroupMember {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>, clock: Arc<dyn Clock>) -> Self {
        Self {
            unit_of_work,
            clock,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        command: AddGroupMemberCommand,
    ) -> AppResult<ExpenseGroupMember> {
        let org_id = authorization::ensure_member(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let group = open_group(uow.as_mut(), command.group_id).await?;

        let existing = uow
            .expense_groups()
            .find_member(group.id(), command.user_id)
            .await?;

        let (member, rejoined) = match existing {
            Some(mut member) => {
                // Già dentro: `rejoin` rifiuta, e dirlo è meglio di un
                // aggiornamento che non aggiorna. Chi rientra ridichiara il
                // proprio peso, perché la stanza può essere cambiata.
                member.rejoin(command.weight, now)?;
                uow.expense_groups().update_member(&member).await?;
                (member, true)
            }
            None => {
                let member = ExpenseGroupMember::join(
                    NewExpenseGroupMember {
                        id: ExpenseGroupMemberId::generate(),
                        org_id,
                        group_id: group.id(),
                        user_id: command.user_id,
                        room_id: command.room_id,
                        display_name: command.display_name,
                        weight: command.weight,
                    },
                    now,
                )?;
                uow.expense_groups()
                    .insert_member(&member)
                    .await
                    .map_err(map_membership_conflict)?;
                (member, false)
            }
        };

        append_all(
            uow.as_mut(),
            context,
            now,
            vec![DomainEventPayload::ExpenseGroupMemberJoined(
                ExpenseGroupMemberJoined::new(&member, rejoined),
            )],
        )
        .await?;
        uow.commit().await?;

        Ok(member)
    }
}

/// Esce dal gruppo: `left_at` valorizzato, **la riga resta**.
///
/// Deve restare: le quote già intestate a quella persona restano, e
/// `app.v_expense_group_balances` parte proprio dalla membership — dalla migration
/// `0027` chi esce con un conto aperto continua a comparire nei saldi finché il
/// suo netto non è zero. È il difetto che quella migration corregge: prima, chi
/// lasciava il gruppo **spariva portandosi via il debito**.
///
/// **Non si rifiuta l'uscita a saldo aperto**, ed è una scelta: renderebbe il
/// difetto irraggiungibile invece di correggerlo, e il prezzo lo pagherebbe il
/// caso reale — il coinquilino che se ne va davvero senza pagare resterebbe membro
/// per sempre e il gruppo non potrebbe chiudersi. **Il debito è del gruppo, non
/// della membership.**
///
/// # Si esce per sé stessi, e la restrizione **non** è un confine di sicurezza
///
/// La `USING` di `expense_group_members_update` ammette qualunque membro attivo
/// del gruppo, quindi la Row Level Security lascerebbe passare una query scritta a
/// mano che toglie qualcun altro. La restrizione vive qui, nel contratto dell'API,
/// e la ragione è che un coinquilino non deve poter togliere unilateralmente un
/// altro da un conto condiviso — tanto meno uno con un saldo aperto. Lo staff può,
/// perché è chi amministra l'organizzazione.
///
/// Renderlo un confine vero richiederebbe di stringere la `USING` con
/// `user_id = app.current_user_id() OR app.is_org_staff(org_id)`, cioè una
/// migration. Va saputo, e sta scritto qui perché è esattamente il genere di cosa
/// che si legge come una garanzia quando non lo è.
pub struct LeaveGroup {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl LeaveGroup {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>, clock: Arc<dyn Clock>) -> Self {
        Self {
            unit_of_work,
            clock,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        group_id: ExpenseGroupId,
        user_id: UserId,
    ) -> AppResult<ExpenseGroupMember> {
        authorization::ensure_member(context)?;
        let caller = context
            .user_id()
            .ok_or(AppError::Forbidden(Denial::AuthenticationRequired))?;
        if user_id != caller && !context.is_staff() {
            return Err(AppError::Forbidden(Denial::GroupMembershipNotYours));
        }
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let group = visible_group(uow.as_mut(), group_id).await?;

        let mut member = uow
            .expense_groups()
            .find_member(group.id(), user_id)
            .await?
            .ok_or(AppError::not_found(Entity::ExpenseGroup))?;

        member.leave(now)?;
        uow.expense_groups().update_member(&member).await?;

        append_all(
            uow.as_mut(),
            context,
            now,
            vec![DomainEventPayload::ExpenseGroupMemberLeft(
                ExpenseGroupMemberLeft::new(&member),
            )],
        )
        .await?;
        uow.commit().await?;

        Ok(member)
    }
}
