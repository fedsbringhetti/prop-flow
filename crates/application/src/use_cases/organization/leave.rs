use std::sync::Arc;

use propflow_domain::events::{
    DomainEvent, DomainEventPayload, EventMetadata, OrganizationMemberLeft,
};
use propflow_domain::identity::{OrgMember, OrgRole};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::OrgId;
use propflow_domain::{DomainError, Entity, Invariant, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

/// Uscire da un'organizzazione.
///
/// # `ensure_authenticated`, e l'appartenenza si cerca per la via lunga
///
/// Non `ensure_member`, per la ragione di `ListOrganizations`: pretendere
/// un'organizzazione **attiva** romperebbe il caso in cui questa rotta serve di
/// più — chi ha come attiva proprio quella da cui vuole uscire e ci resta dentro
/// male. `org_members_select` ammette `user_id = app.current_user_id()` come primo
/// ramo, che non guarda `app.current_org_id()`: la propria appartenenza si legge
/// quindi anche in un'organizzazione diversa da quella attiva.
///
/// # Uscire è uno **stato**
///
/// `status = 'left'`, non `deleted_at`. Vedi `OrgMember::leave`: con il soft
/// delete la riga sparirebbe da `find` e `org_members_org_user_uk`, unico e non
/// parziale, impedirebbe per sempre il rientro.
///
/// # I tre rifiuti, e chi li garantisce
///
/// | Caso | Codice | Chi lo garantisce davvero |
/// |---|---|---|
/// | non sei membro | `membership.not_found` | la Row Level Security: la riga non torna |
/// | hai un contratto in essere | `membership.lease_in_force` | la `WITH CHECK` di `org_members_update_self` |
/// | sei l'ultimo proprietario | `membership.last_owner` | **questo caso d'uso, e nient'altro** |
///
/// I primi due sono la versione **parlante** di un rifiuto che il database darebbe
/// comunque: senza, il secondo risponderebbe `auth.denied_by_policy`, che nella
/// tassonomia di ADR-022 significa «difetto nostro» e manderebbe a cercare un
/// guasto invece di una disdetta.
///
/// Il terzo no, ed è il solo che vive qui per intero: lo schema dichiara «almeno un
/// owner per org» come invariante del **service layer**, perché un trigger
/// cross-row sarebbe workflow nel database. Senza questo controllo, uscire dalla
/// propria organizzazione personale la lascerebbe senza nessuno che la amministri —
/// e `organizations_select` è `is_org_member(id)`, quindi da quel momento quella
/// riga, con dentro immobili, contratti e addebiti, non sarebbe più visibile a
/// nessuno. Non esiste nessuna operazione del prodotto per rientrarci.
///
/// # Chi esce dall'unica organizzazione in cui abita
///
/// Resta con la propria, personale e vuota: è il caso di
/// [ADR-045](../../../../../docs/08-decisioni.md) al contrario. **Il client deve
/// dirlo prima**, non lasciarlo scoprire dopo con un riepilogo vuoto e nessuna
/// spiegazione.
pub struct LeaveOrganization {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl LeaveOrganization {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>, clock: Arc<dyn Clock>) -> Self {
        Self {
            unit_of_work,
            clock,
        }
    }

    pub async fn execute(&self, context: &RequestContext, org_id: OrgId) -> AppResult<OrgMember> {
        authorization::ensure_authenticated(context)?;
        let user_id = context.require_user()?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut member = uow
            .members()
            .find(org_id, user_id)
            .await?
            .ok_or(AppError::NotFound(Entity::Membership))?;

        // L'ultimo proprietario, **prima** del contratto in essere: sono due
        // rifiuti diversi e l'ordine decide quale si legge. Questo va per primo
        // perché è quello a cui non c'è rimedio dentro il prodotto — l'altro si
        // risolve aspettando la scadenza o disdicendo.
        if member.role() == OrgRole::Owner
            && member.is_active()
            && uow.members().count_active_owners(org_id).await? <= 1
        {
            return Err(AppError::Validation(DomainError::Invariant(
                Invariant::LastOwnerCannotLeave,
            )));
        }

        // La versione parlante del rifiuto della policy, e la stessa funzione:
        // `has_in_force_for_user` interroga `app.has_lease_in_force`, che è ciò che
        // la `WITH CHECK` valuta. Due condizioni distinte divergerebbero, e il
        // sintomo sarebbe un `auth.denied_by_policy` su un caso previsto.
        if uow.leases().has_in_force_for_user(org_id, user_id).await? {
            return Err(AppError::Validation(DomainError::Invariant(
                Invariant::MembershipHasLeaseInForce,
            )));
        }

        // `leave()` prima di tutto, ma è **in memoria**: mette lo stato a `left` e
        // rifiuta se lo era già. Non tocca il database, e l'ordine di ciò che segue
        // conta.
        member.leave()?;

        // **L'evento va scritto mentre la membership in database è ancora attiva**,
        // e non è una sottigliezza: `domain_events_insert` ha una `WITH CHECK` che
        // ammette `org_id IS NULL OR org_id = current_org_id() OR
        // is_org_member(org_id)`. La rotta non porta `X-Org-Id` — si esce anche da
        // un'organizzazione che non è quella attiva — quindi `current_org_id()` non
        // coincide con `org_id`, e a reggere è il ramo `is_org_member`. Ma
        // `is_org_member` passa da `member_role`, che esige `status = 'active'`:
        // appena la riga diventa `left`, quel ramo diventa falso e l'inserimento
        // dell'evento sbatte contro la policy — un `auth.denied_by_policy` che
        // significherebbe «difetto nostro» su un'operazione riuscita.
        //
        // Quindi: si emette l'evento **prima** dell'`UPDATE` della membership.
        // Vivono nella stessa transazione e commitano insieme, quindi l'evento
        // resta coerente con la riga; ciò che cambia è solo quale delle due
        // scritture vede l'altra a metà. È la stessa forma di ordine di
        // `AcceptInvitation`, dove `has_pending_invitation` regge finché l'invito
        // non è marcato accettato.
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::OrganizationMemberLeft(OrganizationMemberLeft {
                    org_id,
                    user_id,
                    role: member.role().as_str(),
                    left_at: now,
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.members().update(&member).await?;
        uow.commit().await?;

        Ok(member)
    }
}
