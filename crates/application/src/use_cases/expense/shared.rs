use chrono::{DateTime, Utc};
use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata};
use propflow_domain::expense::{ExpenseGroup, ExpenseGroupMember, Participant};
use propflow_domain::ports::{RepositoryError, UnitOfWork};
use propflow_domain::shared::{ExpenseGroupId, UserId};
use propflow_domain::{Entity, Invariant, RequestContext};

use crate::error::{AppError, AppResult};

/// Rilegge il gruppo **sotto la RLS del chiamante**, che è la parte che non si può
/// saltare.
///
/// È la **terza** occorrenza dello stesso rimedio, dopo `ensure_owner_is_visible`
/// dei documenti, `build_issued_charge` degli addebiti e `visible_charge` dei
/// pagamenti, e qui vale per due ragioni invece che per una:
///
/// 1. **`app.expense_groups` non ha il trigger `tg_assert_org_matches_property`**,
///    che invece esiste su `rooms`, `leases`, `tickets` e `utility_bills` —
///    verificato nella `0013`, non dedotto. Un `property_id` di un'altra
///    organizzazione entra senza che nulla protesti, e la riga resta a citare un
///    immobile che chi l'ha scritta non vede;
/// 2. su spese e regolamenti la `WITH CHECK` chiede `can_access_expense_group`,
///    quindi il gruppo è protetto dalla policy — ma un `404` onesto è meglio di
///    `auth.denied_by_policy`, che è il codice riservato ai **nostri** difetti.
pub(super) async fn visible_group(
    uow: &mut dyn UnitOfWork,
    group_id: ExpenseGroupId,
) -> AppResult<ExpenseGroup> {
    uow.expense_groups()
        .find_by_id(group_id)
        .await?
        .ok_or(AppError::not_found(Entity::ExpenseGroup))
}

/// Il gruppo, e la garanzia che sia ancora aperto.
///
/// Le scritture passano da qui, le letture no: un conto chiuso si consulta.
pub(super) async fn open_group(
    uow: &mut dyn UnitOfWork,
    group_id: ExpenseGroupId,
) -> AppResult<ExpenseGroup> {
    let group = visible_group(uow, group_id).await?;
    group.ensure_open()?;
    Ok(group)
}

/// I membri **attivi** del gruppo, pronti per la ripartizione.
///
/// L'ordine non conta e non va garantito qui: [`Split`](propflow_domain::expense::Split)
/// riordina per `user_id` da sé, ed è precisamente ciò che rende la ripartizione
/// riproducibile qualunque cosa faccia questa query.
pub(super) fn participants(members: &[ExpenseGroupMember]) -> Vec<Participant> {
    members
        .iter()
        .filter(|member| member.is_active())
        .map(|member| Participant {
            user_id: member.user_id(),
            weight: member.weight(),
        })
        .collect()
}

/// Verifica che chi ha anticipato sia un membro attivo del gruppo.
///
/// **Lo schema non lo impone**: su `app.expenses.paid_by_user_id` non c'è alcun
/// vincolo verso la membership, e una spesa anticipata da un estraneo entrerebbe
/// senza che nulla protesti. Il danno sarebbe silenzioso e non un errore:
/// `app.v_expense_group_balances` parte dai **membri**, quindi quel credito non
/// comparirebbe a nessuno e la somma dei saldi del gruppo smetterebbe di fare
/// zero. È la ragione della decisione 8 del piano.
///
/// Chi ha anticipato **non** deve necessariamente partecipare alla spesa — si può
/// pagare per gli altri — ma deve essere del gruppo.
pub(super) fn ensure_payer_belongs(members: &[ExpenseGroupMember], payer: UserId) -> AppResult<()> {
    if members
        .iter()
        .any(|member| member.is_active() && member.user_id() == payer)
    {
        return Ok(());
    }
    Err(AppError::Validation(
        propflow_domain::DomainError::Invariant(Invariant::ExpensePayerNotInGroup),
    ))
}

/// Traduce il conflitto sul vincolo di membership in una condizione parlante.
///
/// `expense_group_members_group_user_uk` è unico su `(group_id, user_id)` e **non
/// è parziale**: chi è uscito e viene reinserito con un `INSERT` prende un
/// conflitto invece di rientrare. Lo use case lo evita rileggendo la membership
/// prima, e questa mappatura resta per la corsa fra due richieste concorrenti —
/// dove il vincolo è l'unica difesa e il messaggio deve dire cosa è successo.
pub(super) fn map_membership_conflict(error: RepositoryError) -> AppError {
    match &error {
        RepositoryError::Conflict { constraint }
            if constraint == "expense_group_members_group_user_uk" =>
        {
            AppError::Conflict(crate::error::Conflict::GroupMembershipAlreadyExists)
        }
        _ => AppError::from(error),
    }
}

/// Chiude la transazione di una spesa, sapendo che **l'errore del trigger arriva
/// dal `COMMIT`**.
///
/// `expense_shares_balanced` è `DEFERRABLE INITIALLY DEFERRED`: la violazione non
/// esce dall'`INSERT` che l'ha causata, esce da `uow.commit()`. È la trappola più
/// cara di questo modulo, e la conseguenza pratica è che il commit va mappato —
/// scriverlo come `uow.commit().await?` per abitudine trasformerebbe la
/// diagnostica in un `500` muto.
///
/// **Perché basta riconoscere il tipo di errore e non il messaggio.**
/// `map_sqlx_error` costruisce i dettagli di una `check_violation` dal **nome del
/// vincolo**, e un `RAISE EXCEPTION` di PL/pgSQL non ne ha uno: il messaggio del
/// trigger non arriva fin qui. Riconoscerlo dal testo era la via ovvia e non
/// funziona. Non serve: `expense_shares_balanced` è **l'unico vincolo differito
/// dell'intero schema** — verificato, `DEFERRABLE` compare solo nella `0009` — e
/// questa transazione ha scritto una spesa, le sue quote e una riga di outbox.
/// Un'integrità che si manifesta al commit di questa transazione è quella, e non
/// può essere altro.
///
/// In pratica non dovrebbe mai scattare, perché
/// [`Split`](propflow_domain::expense::Split) garantisce la somma. Se scatta è un
/// difetto **nostro**, e i dettagli finiscono nei log — mai nella risposta, dove
/// il codice resta `conflict.constraint_violated`.
pub(super) async fn commit_expense(uow: Box<dyn UnitOfWork>) -> AppResult<()> {
    uow.commit().await.map_err(|error| match error {
        RepositoryError::Integrity { details } => AppError::Integrity {
            details: format!(
                "invariante contabile violata al commit: la somma delle quote non \
                 corrisponde all'importo della spesa ({details})"
            ),
        },
        other => AppError::from(other),
    })
}

pub(super) async fn append_all(
    uow: &mut dyn UnitOfWork,
    context: &RequestContext,
    now: DateTime<Utc>,
    payloads: Vec<DomainEventPayload>,
) -> AppResult<()> {
    for payload in payloads {
        uow.events()
            .append(&DomainEvent::new(
                payload,
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
    }
    Ok(())
}
