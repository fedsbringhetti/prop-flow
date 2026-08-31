use propflow_domain::ports::UnitOfWork;
use propflow_domain::shared::{LeaseId, PropertyId, RoomId};
use propflow_domain::{DomainError, Entity};

use crate::error::{AppError, AppResult};

/// Verifica che stanza e contratto appartengano all'immobile indicato.
///
/// **Lo schema non lo verifica, e questa è l'unica difesa.** Il trigger
/// `app.tg_assert_org_matches_property` su `app.utility_bills` controlla soltanto
/// che l'`org_id` coincida con quello dell'immobile: una stanza di un altro
/// immobile, o il contratto di un altro immobile, passano senza una parola. Il
/// risultato è una bolletta che nei report sta sotto un immobile e
/// nell'interfaccia sotto un altro — un'incoerenza silenziosa che si scopre mesi
/// dopo, quando i totali per immobile non tornano.
///
/// È la stessa trappola delle segnalazioni, con **una differenza che conta**: lì
/// il controllo sta solo nell'apertura, perché dopo l'inserimento quelle colonne
/// non sono più scrivibili (migration `0020`). Qui `room_id` e `lease_id` restano
/// scrivibili anche nel `PATCH`, perché sulla bolletta dicono *chi deve pagare* —
/// una decisione che cambia legittimamente finché la bolletta è una bozza — mentre
/// sulla segnalazione dicono *dov'è il guasto*, che è un fatto fissato al momento
/// in cui è stato segnalato.
///
/// Ne segue che il controllo **va rifatto a ogni correzione**, ed è la ragione per
/// cui vive in una funzione sola chiamata da due punti invece che copiato: due
/// copie divergono, e la copia che divergerebbe è quella del `PATCH`, che è anche
/// quella che nessuno pensa a guardare.
pub(super) async fn ensure_belong_to_property(
    uow: &mut dyn UnitOfWork,
    property_id: PropertyId,
    room_id: Option<RoomId>,
    lease_id: Option<LeaseId>,
) -> AppResult<()> {
    if let Some(room_id) = room_id {
        let room = uow
            .rooms()
            .find_by_id(room_id)
            .await?
            .ok_or(AppError::not_found(Entity::Room))?;
        if room.property_id() != property_id {
            return Err(AppError::Validation(DomainError::validation(
                "room_id",
                "the room does not belong to the given property",
            )));
        }
    }

    if let Some(lease_id) = lease_id {
        let lease = uow
            .leases()
            .find_by_id(lease_id)
            .await?
            .ok_or(AppError::not_found(Entity::Lease))?;
        if lease.property_id() != property_id {
            return Err(AppError::Validation(DomainError::validation(
                "lease_id",
                "the lease does not belong to the given property",
            )));
        }
    }

    Ok(())
}
