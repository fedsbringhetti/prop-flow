use std::sync::Arc;

use chrono::{DateTime, Utc};
use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, TenantInvited};
use propflow_domain::identity::{Invitation, NewInvitation, OrgRole};
use propflow_domain::lease::{LeaseParty, LeasePartyRole, NewLeaseParty};
use propflow_domain::ports::{Clock, UnitOfWork, UnitOfWorkFactory};
use propflow_domain::shared::{Email, InvitationId, LeaseId, LeasePartyId, OrgId, UserId};
use propflow_domain::{DomainError, Entity, Invariant, RequestContext};
use rust_decimal::Decimal;

use crate::authorization;
use crate::error::{AppError, AppResult, Conflict};
use crate::ports::{Notification, NotificationContent, Notifier, OpaqueToken, OpaqueTokenFactory};

#[derive(Debug, Clone)]
pub struct InviteTenantCommand {
    pub lease_id: LeaseId,
    pub email: Email,
    /// Usato solo se la parte contrattuale con questa email non esiste ancora.
    pub full_name: String,
}

/// Invito emesso. Il token in chiaro è qui e da nessun'altra parte.
#[derive(Debug, Clone)]
pub struct IssuedInvitation {
    pub invitation: Invitation,
    /// Da consegnare all'invitato **una sola volta**: in database ne resta lo
    /// hash, come per i refresh token. Non va registrato nei log.
    pub token: String,
}

/// Invita un inquilino su un contratto.
///
/// # L'email parte **dopo il commit, e best-effort**
///
/// Non dentro la transazione, e non prima: se il relay non risponde, l'invito resta
/// valido e il token resta nella risposta HTTP, dove il proprietario lo trova e lo
/// consegna come faceva prima che la posta esistesse. Legare la validità di un
/// invito alla disponibilità di un servizio esterno sarebbe il baratto sbagliato.
///
/// **E non passa dall'outbox**, che è la trappola più cara di questo passo. Il token
/// in chiaro esiste solo qui e nella risposta: in database ne resta lo hash, e gli
/// eventi di dominio trasportano identificatori e nomi di campo, mai valori. Un
/// worker che leggesse `TenantInvited` non avrebbe niente da spedire, e metterci il
/// token dentro significherebbe scrivere una credenziale in una tabella fatta per
/// essere pubblicata su un broker.
///
/// Il fallimento va nei log **senza il destinatario e senza il token**: si registra
/// l'identificatore dell'invito e la classe dell'errore, che è quanto basta per
/// sapere che è successo senza scrivere dati personali in una traccia.
pub struct InviteTenant {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    tokens: Arc<dyn OpaqueTokenFactory>,
    clock: Arc<dyn Clock>,
    notifier: Arc<dyn Notifier>,
}

impl InviteTenant {
    #[must_use]
    pub const fn new(
        unit_of_work: Arc<dyn UnitOfWorkFactory>,
        tokens: Arc<dyn OpaqueTokenFactory>,
        clock: Arc<dyn Clock>,
        notifier: Arc<dyn Notifier>,
    ) -> Self {
        Self {
            unit_of_work,
            tokens,
            clock,
            notifier,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        command: InviteTenantCommand,
    ) -> AppResult<IssuedInvitation> {
        let org_id = authorization::ensure_staff(context)?;
        let invited_by = context.require_user()?;
        let now = self.clock.now();

        let material = self.tokens.generate().map_err(AppError::internal)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        // Il nome dell'organizzazione serve **all'email**, non all'invito: chi
        // riceve un codice deve capire da chi arriva, e «PropFlow ti ha invitato»
        // non lo dice. Letto sotto RLS come tutto il resto — `organizations_select`
        // è `is_org_member(id)`, e chi invita è staff.
        let organization = uow
            .organizations()
            .find_by_id(org_id)
            .await?
            // Non un `404`: chi invita è staff di questa organizzazione, e
            // `organizations_select` è `is_org_member(id)`. Se qui non si trova
            // nulla è un difetto nostro, non una riga che il chiamante non vede.
            .ok_or_else(|| {
                AppError::internal("organizzazione non leggibile da chi la amministra")
            })?;
        let lease = uow
            .leases()
            .find_by_id(command.lease_id)
            .await?
            .ok_or(AppError::not_found(Entity::Lease))?;

        // La parte contrattuale deve esistere prima dell'accettazione: è la riga
        // che l'invitato collegherà a sé, e senza di essa accettare non lo
        // renderebbe parte di nulla.
        let parties = uow.lease_parties().list_for_lease(lease.id()).await?;
        // `list_for_lease` non filtra `left_at`: le righe uscite ci sono. Trovarle è
        // necessario — è così che il subentro impedisce due parti con la stessa email
        // — ma **una riga uscita non vale come parte esistente** per decidere se la
        // porta di ADR-046 vada aperta. Senza questo rifiuto, l'email di chi è appena
        // uscito faceva saltare `ensure_accepts_parties` e l'invito usciva su un
        // contratto attivo: la porta chiusa dal lato del subentro si riapriva dal lato
        // dell'invito. Chi è uscito rientra con un contratto nuovo (ADR-056), non con
        // un invito su quello da cui è uscito.
        if parties
            .iter()
            .any(|party| party.email() == Some(&command.email) && party.left_at().is_some())
        {
            return Err(AppError::Validation(DomainError::Invariant(
                Invariant::LeasePartyAlreadyLeft,
            )));
        }
        if !parties
            .iter()
            .any(|party| party.email() == Some(&command.email))
        {
            // **Qui, e non in testa**: invitare una parte che esiste è ammesso in ogni
            // stato — «si attiva, poi si invita» è il giro legittimo di ADR-046 — ma
            // crearne una su un contratto attivo era la seconda porta che
            // quell'ADR non aveva visto. Nasceva a `remaining_share`, cioè a zero, con
            // chi doveva uscire ancora dentro. Il subentro è la via (ADR-056).
            lease.ensure_accepts_parties()?;
            let party = LeaseParty::new(
                NewLeaseParty {
                    id: LeasePartyId::generate(),
                    org_id,
                    lease_id: lease.id(),
                    role: LeasePartyRole::Tenant,
                    full_name: command.full_name,
                    email: Some(command.email.clone()),
                    share_percent: remaining_share(&parties),
                    is_primary: parties.is_empty(),
                },
                now,
            )?;
            uow.lease_parties().insert(&party).await?;
        }

        let issued = issue_tenant_invitation(
            uow.as_mut(),
            material,
            TenantInvitationTarget {
                org_id,
                lease_id: lease.id(),
                email: command.email,
                invited_by,
            },
            context,
            now,
        )
        .await?;
        uow.commit().await?;

        deliver_tenant_invitation(self.notifier.as_ref(), &issued, organization.name()).await;
        Ok(issued)
    }
}

/// A chi va un invito da inquilino, e da parte di chi.
pub(crate) struct TenantInvitationTarget {
    pub org_id: OrgId,
    pub lease_id: LeaseId,
    pub email: Email,
    pub invited_by: UserId,
}

/// Emette un invito da inquilino **dentro la transazione di chi chiama**: la riga
/// in `app.org_invitations` e l'evento `TenantInvited`, niente commit e niente
/// posta.
///
/// È fuori da [`InviteTenant`] perché il subentro (ADR-056) invita chi entra nella
/// stessa transazione in cui fa uscire chi esce, e non può chiamare `InviteTenant`:
/// quello crea da sé una parte con `remaining_share`, e il subentro l'ha già
/// costruita con la quota di chi esce. Un invito già pendente per quell'email fa
/// fallire la transazione intera, uscita compresa, ed è il comportamento voluto.
pub(crate) async fn issue_tenant_invitation(
    uow: &mut dyn UnitOfWork,
    material: OpaqueToken,
    target: TenantInvitationTarget,
    context: &RequestContext,
    now: DateTime<Utc>,
) -> AppResult<IssuedInvitation> {
    let TenantInvitationTarget {
        org_id,
        lease_id,
        email,
        invited_by,
    } = target;
    let invitation = Invitation::issue(
        NewInvitation {
            id: InvitationId::generate(),
            org_id,
            email,
            role: OrgRole::Tenant,
            token_hash: material.hash,
            lease_id: Some(lease_id),
            invited_by: Some(invited_by),
        },
        now,
    )?;

    uow.invitations()
        .insert(&invitation)
        .await
        .map_err(|error| map_pending_conflict(error.into()))?;
    uow.events()
        .append(&DomainEvent::new(
            DomainEventPayload::TenantInvited(TenantInvited {
                invitation_id: invitation.id(),
                org_id,
                lease_id: invitation.lease_id(),
                role: invitation.role().as_str(),
            }),
            now,
            EventMetadata::from_context(context),
        ))
        .await?;

    Ok(IssuedInvitation {
        invitation,
        token: material.plaintext,
    })
}

/// Recapita il codice **dopo il commit**, e **non fallisce**.
///
/// Il tipo di ritorno è `()` di proposito: non c'è niente che il chiamante
/// possa fare con un errore qui, perché a questo punto l'invito è già scritto e
/// il token è già nella risposta. Restituire un `Result` inviterebbe qualcuno a
/// propagarlo, e propagarlo significherebbe annullare un invito valido perché
/// un server di posta era occupato.
pub(crate) async fn deliver_tenant_invitation(
    notifier: &dyn Notifier,
    issued: &IssuedInvitation,
    organization_name: &str,
) {
    let notification = Notification {
        to: issued.invitation.email().clone(),
        content: NotificationContent::TenantInvitation {
            organization_name: organization_name.to_owned(),
            accept_token: issued.token.clone(),
        },
    };
    if let Err(error) = notifier.send(&notification).await {
        // **Né il destinatario né il token**: `docs/04-sicurezza.md` vieta email
        // e credenziali nelle tracce, e questa è la riga in cui è più facile
        // scriverli. L'identificatore dell'invito basta a ritrovare la riga.
        tracing::warn!(
            invitation_id = %issued.invitation.id(),
            error = %error,
            "invito non recapitato per posta: il codice resta nella risposta"
        );
    }
}

/// Quota residua da assegnare a una parte creata dall'invito.
///
/// L'attivazione richiede che le quote degli inquilini sommino a 100, quindi il
/// residuo è l'unico valore che non rompe l'invariante appena assegnato. Se le
/// quote sono già complete la nuova parte nasce a zero, e chi amministra la
/// correggerà: meglio una quota visibilmente sbagliata che un contratto non più
/// attivabile senza capire perché.
fn remaining_share(parties: &[LeaseParty]) -> Decimal {
    let assigned: Decimal = parties
        .iter()
        .filter(|party| party.role().shares_rent() && party.left_at().is_none())
        .map(LeaseParty::share_percent)
        .sum();
    (Decimal::from(100) - assigned).max(Decimal::ZERO)
}

/// Il vincolo `org_invitations_pending_uk` ammette un solo invito pendente per
/// email e organizzazione. Tradurlo evita che un doppio invito esca come "vincolo
/// violato" senza dire quale.
fn map_pending_conflict(error: AppError) -> AppError {
    match &error {
        AppError::Integrity { details } if details.contains("org_invitations_pending") => {
            AppError::Conflict(Conflict::InvitationAlreadyPending)
        }
        _ => error,
    }
}
