use crate::shared::UserId;

/// Chi abita un immobile, **letto** da `app.property_residents`.
///
/// Si legge da una funzione e non da una query con i join, ed è la decisione
/// centrale della fetta: `leases_select` è per **contratto**, quindi qualunque
/// query che unisca le parti contrattuali ai contratti — e unirle è obbligatorio,
/// perché l'immobile e «in essere» stanno sul contratto — viene rifiltrata al solo
/// contratto del chiamante. Farla funzionare avrebbe richiesto di allargare
/// `leases_select`, e da lì un coinquilino avrebbe letto il **canone** degli altri:
/// `leases.rent_amount` sta sulla riga, e una policy è per riga e non per colonna.
///
/// La funzione **proietta** questi tre campi e nient'altro, quindi non c'è nulla
/// da omettere: né il canone né l'email escono, perché nessuna riga in più diventa
/// leggibile. Vedi la migration `0030`.
///
/// # `user_id` è opzionale, e l'elenco lo dice invece di tacerlo
///
/// `lease_parties.user_id` resta nullo finché l'invitato non accetta: un residente
/// «invitato e non ancora entrato» ha un nome e **nessun identificatore**, quindi
/// non è aggiungibile a un gruppo di spesa. Ometterlo lascerebbe chi guarda senza
/// una spiegazione per una persona che sa di avere in casa e non trova
/// nell'elenco.
///
/// # Chi non compare
///
/// Il **proprietario** non è un residente: `app.can_access_property` lo ammette
/// come staff — quindi l'elenco lo vede — ma non ha una parte contrattuale e non
/// abita lì. Il **garante** non compare per la stessa ragione, ed è la distinzione
/// che [`LeasePartyRole::shares_rent`](crate::lease::LeasePartyRole::shares_rent)
/// fa già nel dominio. Chi aveva un contratto **scaduto** non compare perché il
/// filtro passa da `app.lease_in_force` (ADR-019): lo stato memorizzato resta
/// `active` anche dopo la scadenza, perché nessuno scheduler lo aggiorna.
///
/// # Una riga per parte contrattuale
///
/// Chi affitta due stanze dello stesso immobile compare due volte, con la stanza a
/// distinguerle. È il fatto vero — due contratti, due stanze — e ridurlo a una
/// riga sceglierebbe arbitrariamente quale stanza mostrare.
#[derive(Debug, Clone)]
pub struct PropertyResident {
    /// Assente finché l'invitato non ha accettato l'invito.
    pub user_id: Option<UserId>,
    /// Il nome sulla parte contrattuale, non quello di `app.users`.
    ///
    /// Lo schema denormalizza i nomi dove si mostrano, ed è per questo che le due
    /// metà della migration `0030` sono indipendenti: questo elenco non ha
    /// bisogno che `users_select` mostri niente.
    pub full_name: String,
    /// La stanza, assente sui contratti sull'immobile intero.
    pub room_name: Option<String>,
}

impl PropertyResident {
    /// `true` se questa persona può essere aggiunta a un gruppo di spesa.
    ///
    /// Cioè se ha un identificatore: un invitato che non ha ancora accettato non
    /// ce l'ha, ed è l'unica ragione per cui questo metodo esiste invece di
    /// lasciare che chi chiama interpreti un `Option`.
    #[must_use]
    pub const fn is_addressable(&self) -> bool {
        self.user_id.is_some()
    }
}
