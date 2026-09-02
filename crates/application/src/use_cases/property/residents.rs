use std::sync::Arc;

use propflow_domain::ports::UnitOfWorkFactory;
use propflow_domain::property::PropertyResident;
use propflow_domain::shared::PropertyId;
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

/// Chi abita un immobile.
///
/// È il pezzo che mancava perché le spese co-living fossero usabili nel caso
/// reale: in un co-living **per stanza** ogni coinquilino ha il proprio contratto,
/// quindi non è parte di quello degli altri e non ne vedeva l'identificatore da
/// nessuna parte. Chi creava un gruppo di spesa non poteva aggiungerci nessuno
/// finché il proprietario non gli passava gli `user_id` a mano.
///
/// # `ensure_member`, e a decidere quali righe escono è la funzione
///
/// Non `ensure_staff`: l'elenco dei propri coinquilini è la ragione per cui questo
/// use case esiste. Quali immobili siano «casa propria» lo decide
/// `app.can_access_property` **dentro** `app.property_residents`, non un ramo
/// applicativo sul ruolo — è la stessa scelta di [`ListRooms`](super::super::room::ListRooms).
///
/// # Perché non è una query con i join
///
/// Perché non funzionerebbe, e il modo in cui non funziona è silenzioso.
/// `leases_select` è `can_access_lease(id)`, cioè **per contratto**: qualunque
/// query che unisca `app.lease_parties` ad `app.leases` — e unirle serve, perché
/// l'immobile e «in essere» stanno sul contratto — viene rifiltrata al solo
/// contratto del chiamante, e l'elenco torna con una riga sola senza alcun errore.
///
/// Farla funzionare avrebbe richiesto di allargare anche `leases_select`, e da lì
/// un coinquilino avrebbe letto il **canone** degli altri. La funzione
/// `SECURITY DEFINER` proietta tre colonne e non rende leggibile nessuna riga in
/// più. Vedi la migration `0030`.
///
/// # L'immobile si rilegge prima
///
/// Senza, un identificatore inesistente o non visibile darebbe un elenco **vuoto**
/// invece di un `404` — e un elenco vuoto è indistinguibile da un immobile in cui
/// non abita nessuno, che è uno stato legittimo (un immobile sfitto). Il confine
/// resta comunque dentro la funzione: la rilettura serve alla semantica della
/// risposta, non alla sicurezza.
pub struct ListPropertyResidents {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl ListPropertyResidents {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        property_id: PropertyId,
    ) -> AppResult<Vec<PropertyResident>> {
        authorization::ensure_member(context)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        uow.properties()
            .find_by_id(property_id)
            .await?
            .ok_or(AppError::not_found(Entity::Property))?;

        uow.properties()
            .list_residents(property_id)
            .await
            .map_err(AppError::from)
    }
}
