use propflow_domain::document::AttachmentOwner;
use propflow_domain::ports::UnitOfWork;
use propflow_domain::{Entity, RequestContext};

use crate::error::{AppError, AppResult};

/// Verifica che l'owner esista **e sia accessibile a chi chiede**, dentro la
/// transazione già aperta.
///
/// Questa funzione è la metà applicativa di un controllo che esiste due volte, ed
/// è voluto. Le policy `documents_insert` e `documents_select` pretendono
/// `app.is_org_staff(org_id) OR app.can_access_attachment(owner_type, owner_id)`:
/// se il service layer non ripetesse lo stesso criterio, il rifiuto arriverebbe
/// dalla Row Level Security come `403 auth.denied_by_policy`, che è il codice che
/// significa «difetto nostro» ([ADR-016]) e non «non esiste».
///
/// Il criterio non è riscritto a mano: si legge l'aggregato attraverso il proprio
/// repository, che gira sotto le stesse policy. `properties_select` è
/// `org_id = current_org_id() AND can_access_property(id)`, cioè esattamente il
/// ramo `property` di `can_access_attachment`; lo stesso per stanze e contratti.
/// Riscriverlo come una condizione applicativa lo farebbe divergere dalla policy
/// alla prima modifica di una delle due.
///
/// L'esito negativo è `404` e non `403`: sotto RLS «non è tuo» e «non esiste» sono
/// indistinguibili, e confermare l'esistenza di un immobile altrui sarebbe un
/// oracolo.
///
/// [ADR-016]: ../../../../docs/08-decisioni.md
pub(super) async fn ensure_owner_is_visible(
    uow: &mut dyn UnitOfWork,
    owner: AttachmentOwner,
) -> AppResult<()> {
    match owner {
        AttachmentOwner::Property(id) => uow
            .properties()
            .find_by_id(id)
            .await?
            .map(|_| ())
            .ok_or(AppError::not_found(Entity::Property)),
        AttachmentOwner::Room(id) => uow
            .rooms()
            .find_by_id(id)
            .await?
            .map(|_| ())
            .ok_or(AppError::not_found(Entity::Room)),
        AttachmentOwner::Lease(id) => uow
            .leases()
            .find_by_id(id)
            .await?
            .map(|_| ())
            .ok_or(AppError::not_found(Entity::Lease)),
        AttachmentOwner::Ticket(id) => uow
            .tickets()
            .find_by_id(id)
            .await?
            .map(|_| ())
            .ok_or(AppError::not_found(Entity::Ticket)),
        // La rilettura vale **doppio** per la bolletta. `can_access_attachment`
        // risolveva `utility_bill` su `can_access_property`, più largo di
        // `utility_bills_select`; la migration `0022` le riallinea, e questa riga è
        // la metà applicativa dello stesso criterio — legge la bolletta sotto la
        // policy che la governa, invece di riscriverne la condizione.
        AttachmentOwner::UtilityBill(id) => uow
            .utility_bills()
            .find_by_id(id)
            .await?
            .map(|_| ())
            .ok_or(AppError::not_found(Entity::UtilityBill)),
        // `find_by_id` e non `find_balance_by_id`: qui serve sapere **se** la riga
        // è visibile, non quanto è stato incassato.
        AttachmentOwner::Charge(id) => uow
            .charges()
            .find_by_id(id)
            .await?
            .map(|_| ())
            .ok_or(AppError::not_found(Entity::Charge)),
    }
}

/// Chi può archiviare un documento.
///
/// Ricalca la `USING` di `documents_update`, che è
/// `app.is_org_staff(org_id) OR uploaded_by = app.current_user_id()`: lo staff
/// archivia qualunque documento dell'organizzazione, e chi ha caricato può
/// disfare ciò che ha fatto. Senza questo controllo l'inquilino che tenta di
/// archiviare il documento di un altro otterrebbe zero righe aggiornate, cioè
/// `403 auth.denied_by_policy` — il codice riservato ai nostri difetti.
pub(super) fn can_archive(
    context: &RequestContext,
    uploaded_by: Option<propflow_domain::shared::UserId>,
) -> bool {
    context.is_staff() || (uploaded_by.is_some() && uploaded_by == context.user_id())
}
