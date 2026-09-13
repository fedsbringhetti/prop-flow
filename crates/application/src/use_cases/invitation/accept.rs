use std::sync::Arc;

use chrono::{DateTime, Utc};
use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, InvitationAccepted};
use propflow_domain::identity::{Invitation, OrgMember, OrgRole};
use propflow_domain::ports::{Clock, UnitOfWork, UnitOfWorkFactory};
use propflow_domain::shared::{LeaseId, OrgId, UserId};
use propflow_domain::{DomainError, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult, Conflict};
use crate::ports::OpaqueTokenFactory;

#[derive(Debug, Clone)]
pub struct AcceptInvitationCommand {
    /// Token in chiaro ricevuto dall'invitato. Non finisce nei log.
    pub token: String,
}

#[derive(Debug, Clone, Copy)]
pub struct AcceptedInvitation {
    pub org_id: OrgId,
    pub role: OrgRole,
    pub lease_id: Option<LeaseId>,
}

/// Accetta un invito: crea la membership **se manca**, collega la parte
/// contrattuale e marca l'invito, **in una sola transazione**.
///
/// La membership può esserci già, ed è il caso normale di un inquilino che torna:
/// il contratto scade, con la stessa persona se ne fa un altro, e l'unica cosa che
/// gli manca è il collegamento alla parte contrattuale nuova (ADR-051). Vedi il ramo
/// dentro `execute`.
///
/// Non usa il pool privilegiato. La regola 20 riserva `begin_privileged()` ai
/// flussi in cui un contesto di sicurezza non *può* esistere, perché avvengono
/// prima dell'autenticazione; qui il contesto esiste — chi accetta è autenticato —
/// e manca solo la membership. La risposta corretta è estendere la Row Level
/// Security, ed è quello che fa la migration 0018 (ADR-020).
///
/// Sono due controlli indipendenti: l'applicazione prova che chi chiama **possiede**
/// il token, confrontandone lo hash; le policy provano che esiste davvero un invito
/// pendente per quell'email. Nessuno dei due da solo basterebbe.
///
/// L'ordine delle scritture non è arbitrario: `app.has_pending_invitation` abilita
/// la membership e il collegamento della parte, e diventa falso appena l'invito è
/// marcato accettato. Marcarlo per primo si toglierebbe il permesso di completare
/// il resto — e saltare la **prima** non sposta le altre due, perché a renderlo
/// falso è l'ultima.
pub struct AcceptInvitation {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    tokens: Arc<dyn OpaqueTokenFactory>,
    clock: Arc<dyn Clock>,
}

impl AcceptInvitation {
    #[must_use]
    pub const fn new(
        unit_of_work: Arc<dyn UnitOfWorkFactory>,
        tokens: Arc<dyn OpaqueTokenFactory>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            unit_of_work,
            tokens,
            clock,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        command: AcceptInvitationCommand,
    ) -> AppResult<AcceptedInvitation> {
        // `ensure_authenticated` e non `ensure_member`: l'intero senso
        // dell'operazione è che il chiamante non è ancora membro.
        authorization::ensure_authenticated(context)?;
        let user_id = context.require_user()?;
        let now = self.clock.now();
        let token_hash = self.tokens.hash(&command.token);

        let mut uow = self.unit_of_work.begin(context).await?;

        // Un token sconosciuto e un token valido per un altro utente danno la
        // stessa risposta: distinguerli sarebbe un oracolo per capire quali inviti
        // esistono (ADR-017).
        let mut invitation = uow
            .invitations()
            .find_by_token_hash(&token_hash)
            .await?
            .ok_or(AppError::Unauthenticated)?;

        invitation
            .ensure_acceptable(now)
            .map_err(|error| match error {
                // Scaduto o revocato: è una credenziale che non vale più, come un
                // refresh token oltre la scadenza. "Già accettato" invece è uno stato,
                // e chi possiede il token merita di saperlo.
                DomainError::Forbidden(_) => AppError::Unauthenticated,
                other => AppError::Validation(other),
            })?;

        let user = uow
            .users()
            .find_by_id(user_id)
            .await?
            .ok_or(AppError::Unauthenticated)?;
        // Le policy imporrebbero comunque la corrispondenza, ma con un rifiuto della
        // RLS invece di una risposta comprensibile: il service layer serve alla
        // semantica, la RLS all'autorità (ADR-016).
        if user.email() != invitation.email() {
            return Err(AppError::Unauthenticated);
        }

        link_accepted_invitation(
            &mut uow,
            &mut invitation,
            user_id,
            now,
            EventMetadata::from_context(context),
        )
        .await?;
        // Le tre scritture e l'evento vivono o cadono insieme: una membership senza
        // il collegamento al contratto darebbe un inquilino che non vede nulla, e un
        // invito marcato senza membership lo lascerebbe fuori per sempre.
        uow.commit().await?;

        Ok(AcceptedInvitation {
            org_id: invitation.org_id(),
            role: invitation.role(),
            lease_id: invitation.lease_id(),
        })
    }
}

/// Le tre scritture che rendono una persona effettivamente parte
/// dell'organizzazione dopo un invito da inquilino — membership, parte
/// contrattuale, invito marcato — più l'evento che le racconta insieme.
///
/// **Condivisa da [`AcceptInvitation`] e da
/// `RegisterTenantFromInvitation`** (ADR-053), attraverso una funzione e non una
/// copia: i due differiscono solo per come arrivano a `user_id` — uno lo ha già
/// da un'autenticazione, l'altro lo ha appena generato registrando l'utente — e
/// per il pool su cui girano, ma le tre scritture e il loro ordine sono gli
/// stessi.
///
/// **L'ordine non cambia fra i due chiamanti**, anche se solo il primo gira
/// sotto la Row Level Security che quell'ordine protegge
/// (`app.has_pending_invitation`, che diventa falso solo dopo la terza
/// scrittura): il secondo gira sul pool privilegiato, che non è soggetto alla
/// RLS, ma un secondo ordine da spiegare costerebbe più di quanto
/// risparmierebbe. Vedi il commento su ciascun ramo, invariato da
/// `AcceptInvitation`.
pub(crate) async fn link_accepted_invitation(
    uow: &mut Box<dyn UnitOfWork>,
    invitation: &mut Invitation,
    user_id: UserId,
    now: DateTime<Utc>,
    metadata: EventMetadata,
) -> AppResult<()> {
    let org_id = invitation.org_id();

    // ADR-051. Un inquilino che torna passa da qui, e non da una rotta sua: il contratto
    // scade, con la stessa persona se ne fa un altro, e l'invito è il gesto che
    // il proprietario compie comunque. Senza questo ramo `insert` sbatteva contro
    // `org_members_org_user_uk` e l'accettazione moriva con `409
    // membership.already_exists`, su un codice che il proprietario aveva appena
    // consegnato — e siccome `lease_parties.user_id` lo scrive **solo** questo
    // percorso, la persona restava fuori dal contratto nuovo per sempre.
    //
    // **Attiva, non una qualunque**, e da questa sezione la distinzione ha tre
    // esiti invece di due. La guardia era stata scritta prevedendo il giorno in
    // cui da un'organizzazione si sarebbe potuti uscire: quel giorno è `R16`, ed
    // è arrivato.
    //
    // - membership **attiva**: non si tocca niente;
    // - membership **non attiva** (`left`): si **riattiva**. Senza questo ramo,
    //   `insert` sbatte contro `org_members_org_user_uk` — unico e **non
    //   parziale** — e l'accettazione muore con `409 membership.already_exists`,
    //   cioè lo stesso `409` che ADR-051 aveva chiuso, ricomparso da un'altra
    //   porta. Il sintomo sarebbe arrivato mesi dopo, su un invito che sembra
    //   buono;
    // - **nessuna** membership: si inserisce, come prima. È l'unico ramo che
    //   `RegisterTenantFromInvitation` incontra davvero: un utente appena
    //   registrato non può avere una membership precedente in nessuna
    //   organizzazione.
    //
    // **Il ruolo non si tocca in nessuno dei tre casi**: chi è già `owner` o
    // `manager` e viene invitato come `tenant` resta quello che è. L'invito serve
    // a collegare la parte contrattuale, non a retrocedere qualcuno — ed è il
    // caso vero di chi affitta una casa e ne abita un'altra. Sulla riattivazione
    // vale con più forza: chi era proprietario ed è uscito rientra proprietario,
    // e il privilegio di colonna della `0039` lo garantisce comunque, perché
    // `role` non è scrivibile da questo pool.
    //
    // L'ordine delle scritture non cambia: `app.has_pending_invitation` è ancora
    // vero quando si collega la parte e quando si marca l'invito, perché a
    // renderlo falso è la terza scrittura e non la prima. La `SELECT` qui sotto
    // passa dal primo ramo di `org_members_select` — `user_id =
    // app.current_user_id()` — che non chiede di essere già membri; sotto il pool
    // privilegiato la Row Level Security non si applica affatto, ma l'ordine
    // resta lo stesso per non avere due ordini da spiegare.
    //
    // **La riattivazione precede il collegamento della parte contrattuale**, e
    // non è indifferente: la `WITH CHECK` di `org_members_update_self` nega
    // l'uscita di chi ha un contratto in essere, e il ramo `status <> 'left'`
    // ammette il rientro **in ogni caso** proprio perché quell'ordine non deve
    // diventare una precondizione da ricordare.
    match uow.members().find(org_id, user_id).await? {
        Some(member) if member.is_active() => {}
        Some(mut member) => {
            member.rejoin(now);
            uow.members().update(&member).await?;
        }
        None => {
            let membership = OrgMember::active(org_id, user_id, invitation.role(), now);
            uow.members()
                .insert(&membership)
                .await
                .map_err(|error| map_membership_conflict(error.into()))?;
        }
    }

    // Collegare la parte contrattuale è ciò che rende visibile il contratto:
    // `app.can_access_lease` passa da `lease_parties.user_id`.
    if let Some(lease_id) = invitation.lease_id() {
        let parties = uow.lease_parties().list_for_lease(lease_id).await?;
        let mut party = parties
            .into_iter()
            .find(|party| party.email() == Some(invitation.email()))
            .ok_or_else(|| {
                // L'invito nomina un contratto in cui non esiste una riga con
                // questa email: nessuna scrittura, e un errore interno, perché
                // è `InviteTenant` a doverla avere creata.
                AppError::internal(
                    "invito legato a un contratto senza la corrispondente parte contrattuale",
                )
            })?;
        party.link_user(user_id, now)?;
        uow.lease_parties().update(&party).await?;
    }

    invitation.accept(user_id, now)?;
    uow.invitations().update(invitation).await?;

    uow.events()
        .append(&DomainEvent::new(
            DomainEventPayload::InvitationAccepted(InvitationAccepted {
                invitation_id: invitation.id(),
                org_id,
                user_id,
                lease_id: invitation.lease_id(),
                role: invitation.role().as_str(),
            }),
            now,
            metadata,
        ))
        .await?;

    Ok(())
}

/// Resta per ciò che i tre rami del `match` non coprono: una riga **soft
/// deleted**.
///
/// Il ramo `left` adesso c'è, ed è la riattivazione. Ma `find` filtra
/// `deleted_at IS NULL` e `org_members_org_user_uk` non è parziale su quella
/// colonna: una membership cancellata non si vede, quindi si prende il ramo
/// `None` e l'`INSERT` sbatte comunque sul vincolo.
///
/// **Oggi nessun percorso del prodotto la produce** — è precisamente la ragione
/// per cui l'uscita è uno stato e non un soft delete — e il giorno in cui qualcosa
/// la producesse, questo `409` sarebbe la domanda giusta da porsi e non un errore
/// da nascondere.
fn map_membership_conflict(error: AppError) -> AppError {
    match &error {
        AppError::Integrity { details } if details.contains("org_members_org_user") => {
            AppError::Conflict(Conflict::MembershipAlreadyExists)
        }
        _ => error,
    }
}
