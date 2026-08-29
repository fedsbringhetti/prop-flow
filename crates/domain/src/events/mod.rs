use chrono::{DateTime, Utc};

use crate::document::{AttachmentOwner, DocumentKind};
use crate::identity::OrgRole;
use crate::shared::{
    DocumentId, EventId, InvitationId, LeaseId, LeasePartyId, OrgId, PropertyId, RoomId, TicketId,
    TicketMessageId, UserId,
};
use crate::ticket::{Ticket, TicketMessage, TicketStatus};

/// Evento di dominio pronto per l'outbox.
///
/// Gli eventi li produce il service layer come parte della decisione applicativa,
/// non un trigger del database: dedurre un evento da una scrittura significherebbe
/// spostare logica di business nell'infrastruttura.
#[derive(Debug, Clone)]
pub struct DomainEvent {
    id: EventId,
    org_id: Option<OrgId>,
    payload: DomainEventPayload,
    occurred_at: DateTime<Utc>,
    metadata: EventMetadata,
}

impl DomainEvent {
    #[must_use]
    pub fn new(
        payload: DomainEventPayload,
        occurred_at: DateTime<Utc>,
        metadata: EventMetadata,
    ) -> Self {
        Self {
            id: EventId::generate(),
            org_id: payload.org_id(),
            payload,
            occurred_at,
            metadata,
        }
    }

    #[must_use]
    pub const fn id(&self) -> EventId {
        self.id
    }
    #[must_use]
    pub const fn org_id(&self) -> Option<OrgId> {
        self.org_id
    }
    #[must_use]
    pub const fn payload(&self) -> &DomainEventPayload {
        &self.payload
    }
    #[must_use]
    pub const fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
    #[must_use]
    pub const fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }
    #[must_use]
    pub const fn event_type(&self) -> &'static str {
        self.payload.event_type()
    }
    #[must_use]
    pub const fn aggregate_type(&self) -> &'static str {
        self.payload.aggregate_type()
    }
    #[must_use]
    pub fn aggregate_id(&self) -> uuid::Uuid {
        self.payload.aggregate_id()
    }
}

/// Dati di correlazione per il tracing distribuito. Nessun dato personale:
/// l'outbox non è il posto dove far transitare PII.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct EventMetadata {
    pub actor_id: Option<UserId>,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
}

impl EventMetadata {
    #[must_use]
    pub fn from_context(context: &crate::RequestContext) -> Self {
        Self {
            actor_id: context.user_id(),
            request_id: context.request_id().map(str::to_owned),
            trace_id: None,
        }
    }
}

/// Tutti gli eventi di dominio.
///
/// `untagged`: la serializzazione produce solo il contenuto, perché il tipo di
/// evento vive già nella colonna `event_type` dell'outbox e duplicarlo nel
/// payload significherebbe avere due fonti di verità da tenere allineate.
///
/// Aggiungere un evento è una variante in più: il compilatore individua tutti i
/// punti da aggiornare.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(untagged)]
pub enum DomainEventPayload {
    UserRegistered(UserRegistered),
    OrganizationCreated(OrganizationCreated),
    PropertyCreated(PropertyCreated),
    PropertyUpdated(PropertyUpdated),
    PropertyArchived(PropertyArchived),
    RoomCreated(RoomCreated),
    RoomUpdated(RoomUpdated),
    RoomArchived(RoomArchived),
    LeaseDrafted(LeaseDrafted),
    LeaseActivated(LeaseActivated),
    LeaseTerminated(LeaseTerminated),
    LeasePartyAdded(LeasePartyAdded),
    TenantInvited(TenantInvited),
    InvitationRevoked(InvitationRevoked),
    InvitationAccepted(InvitationAccepted),
    DocumentUploaded(DocumentUploaded),
    DocumentArchived(DocumentArchived),
    TicketOpened(TicketOpened),
    TicketStatusChanged(TicketStatusChanged),
    TicketAssigned(TicketAssigned),
    TicketCostsUpdated(TicketCostsUpdated),
    TicketMessagePosted(TicketMessagePosted),
}

impl DomainEventPayload {
    #[must_use]
    pub const fn event_type(&self) -> &'static str {
        match self {
            Self::UserRegistered(_) => "identity.user.registered",
            Self::OrganizationCreated(_) => "identity.organization.created",
            Self::PropertyCreated(_) => "property.created",
            Self::PropertyUpdated(_) => "property.updated",
            Self::PropertyArchived(_) => "property.archived",
            Self::RoomCreated(_) => "property.room.created",
            Self::RoomUpdated(_) => "property.room.updated",
            Self::RoomArchived(_) => "property.room.archived",
            Self::LeaseDrafted(_) => "lease.drafted",
            Self::LeaseActivated(_) => "lease.activated",
            Self::LeaseTerminated(_) => "lease.terminated",
            Self::LeasePartyAdded(_) => "lease.party.added",
            Self::TenantInvited(_) => "identity.invitation.issued",
            Self::InvitationRevoked(_) => "identity.invitation.revoked",
            Self::InvitationAccepted(_) => "identity.invitation.accepted",
            Self::DocumentUploaded(_) => "document.uploaded",
            Self::DocumentArchived(_) => "document.archived",
            Self::TicketOpened(_) => "ticket.opened",
            Self::TicketStatusChanged(_) => "ticket.status_changed",
            Self::TicketAssigned(_) => "ticket.assigned",
            Self::TicketCostsUpdated(_) => "ticket.costs_updated",
            Self::TicketMessagePosted(_) => "ticket.message.posted",
        }
    }

    #[must_use]
    pub const fn aggregate_type(&self) -> &'static str {
        match self {
            Self::UserRegistered(_) => "User",
            Self::OrganizationCreated(_) => "Organization",
            Self::PropertyCreated(_) | Self::PropertyUpdated(_) | Self::PropertyArchived(_) => {
                "Property"
            }
            Self::RoomCreated(_) | Self::RoomUpdated(_) | Self::RoomArchived(_) => "Room",
            // Anche `LeasePartyAdded` è un evento del contratto: la parte non è un
            // aggregato a sé, e chi consuma reagisce al contratto che è cambiato.
            Self::LeaseDrafted(_)
            | Self::LeaseActivated(_)
            | Self::LeaseTerminated(_)
            | Self::LeasePartyAdded(_) => "Lease",
            Self::TenantInvited(_) | Self::InvitationRevoked(_) | Self::InvitationAccepted(_) => {
                "Invitation"
            }
            Self::DocumentUploaded(_) | Self::DocumentArchived(_) => "Document",
            // Anche `TicketMessagePosted` è un evento della segnalazione: il
            // messaggio non è un aggregato a sé, e chi consuma reagisce al thread
            // che si è mosso.
            Self::TicketOpened(_)
            | Self::TicketStatusChanged(_)
            | Self::TicketAssigned(_)
            | Self::TicketCostsUpdated(_)
            | Self::TicketMessagePosted(_) => "Ticket",
        }
    }

    #[must_use]
    pub fn aggregate_id(&self) -> uuid::Uuid {
        match self {
            Self::UserRegistered(event) => event.user_id.as_uuid(),
            Self::OrganizationCreated(event) => event.org_id.as_uuid(),
            Self::PropertyCreated(event) => event.property_id.as_uuid(),
            Self::PropertyUpdated(event) => event.property_id.as_uuid(),
            Self::PropertyArchived(event) => event.property_id.as_uuid(),
            Self::RoomCreated(event) => event.room_id.as_uuid(),
            Self::RoomUpdated(event) => event.room_id.as_uuid(),
            Self::RoomArchived(event) => event.room_id.as_uuid(),
            Self::LeaseDrafted(event) => event.lease_id.as_uuid(),
            Self::LeaseActivated(event) => event.lease_id.as_uuid(),
            Self::LeaseTerminated(event) => event.lease_id.as_uuid(),
            Self::LeasePartyAdded(event) => event.lease_id.as_uuid(),
            Self::TenantInvited(event) => event.invitation_id.as_uuid(),
            Self::InvitationRevoked(event) => event.invitation_id.as_uuid(),
            Self::InvitationAccepted(event) => event.invitation_id.as_uuid(),
            Self::DocumentUploaded(event) => event.document_id.as_uuid(),
            Self::DocumentArchived(event) => event.document_id.as_uuid(),
            Self::TicketOpened(event) => event.ticket_id.as_uuid(),
            Self::TicketStatusChanged(event) => event.ticket_id.as_uuid(),
            Self::TicketAssigned(event) => event.ticket_id.as_uuid(),
            Self::TicketCostsUpdated(event) => event.ticket_id.as_uuid(),
            Self::TicketMessagePosted(event) => event.ticket_id.as_uuid(),
        }
    }

    #[must_use]
    pub const fn org_id(&self) -> Option<OrgId> {
        match self {
            Self::UserRegistered(_) => None,
            Self::OrganizationCreated(event) => Some(event.org_id),
            Self::PropertyCreated(event) => Some(event.org_id),
            Self::PropertyUpdated(event) => Some(event.org_id),
            Self::PropertyArchived(event) => Some(event.org_id),
            Self::RoomCreated(event) => Some(event.org_id),
            Self::RoomUpdated(event) => Some(event.org_id),
            Self::RoomArchived(event) => Some(event.org_id),
            Self::LeaseDrafted(event) => Some(event.org_id),
            Self::LeaseActivated(event) => Some(event.org_id),
            Self::LeaseTerminated(event) => Some(event.org_id),
            Self::LeasePartyAdded(event) => Some(event.org_id),
            Self::TenantInvited(event) => Some(event.org_id),
            Self::InvitationRevoked(event) => Some(event.org_id),
            Self::InvitationAccepted(event) => Some(event.org_id),
            Self::DocumentUploaded(event) => Some(event.org_id),
            Self::DocumentArchived(event) => Some(event.org_id),
            Self::TicketOpened(event) => Some(event.org_id),
            Self::TicketStatusChanged(event) => Some(event.org_id),
            Self::TicketAssigned(event) => Some(event.org_id),
            Self::TicketCostsUpdated(event) => Some(event.org_id),
            Self::TicketMessagePosted(event) => Some(event.org_id),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UserRegistered {
    pub user_id: UserId,
    /// Non c'è l'email: un evento pubblicato su un broker è un dato che viaggia e
    /// si conserva. Chi ha bisogno dell'indirizzo lo legge dall'aggregato.
    pub registered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OrganizationCreated {
    pub org_id: OrgId,
    pub owner_id: UserId,
    pub kind: &'static str,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PropertyCreated {
    pub property_id: PropertyId,
    pub org_id: OrgId,
    pub owner_id: UserId,
    pub city: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PropertyUpdated {
    pub property_id: PropertyId,
    pub org_id: OrgId,
    /// Nomi dei campi modificati, non i valori: chi consuma sa cosa invalidare
    /// senza che l'evento trasporti il contenuto dell'immobile.
    pub changed_fields: Vec<&'static str>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PropertyArchived {
    pub property_id: PropertyId,
    pub org_id: OrgId,
    pub archived_by_role: &'static str,
}

impl PropertyArchived {
    #[must_use]
    pub const fn new(property_id: PropertyId, org_id: OrgId, role: OrgRole) -> Self {
        Self {
            property_id,
            org_id,
            archived_by_role: role.as_str(),
        }
    }
}

/// Una stanza porta sempre l'immobile che la contiene: chi consuma l'evento
/// invalida la scheda dell'immobile senza dover risalire la relazione.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RoomCreated {
    pub room_id: RoomId,
    pub property_id: PropertyId,
    pub org_id: OrgId,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RoomUpdated {
    pub room_id: RoomId,
    pub property_id: PropertyId,
    pub org_id: OrgId,
    /// Nomi dei campi modificati, non i valori: il canone di una stanza non
    /// deve viaggiare su un broker.
    pub changed_fields: Vec<&'static str>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RoomArchived {
    pub room_id: RoomId,
    pub property_id: PropertyId,
    pub org_id: OrgId,
    pub archived_by_role: &'static str,
}

impl RoomArchived {
    #[must_use]
    pub const fn new(
        room_id: RoomId,
        property_id: PropertyId,
        org_id: OrgId,
        role: OrgRole,
    ) -> Self {
        Self {
            room_id,
            property_id,
            org_id,
            archived_by_role: role.as_str(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LeaseDrafted {
    pub lease_id: LeaseId,
    pub org_id: OrgId,
    pub property_id: PropertyId,
    pub room_id: Option<RoomId>,
    pub lease_type: &'static str,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LeaseActivated {
    pub lease_id: LeaseId,
    pub org_id: OrgId,
    pub property_id: PropertyId,
    pub activated_at: DateTime<Utc>,
}

/// Senza `termination_reason`: è testo libero scritto da una persona, quindi può
/// contenere qualunque cosa, e un evento pubblicato su un broker è un dato che
/// viaggia e si conserva. Chi ha bisogno del motivo lo legge dall'aggregato.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LeaseTerminated {
    pub lease_id: LeaseId,
    pub org_id: OrgId,
    pub property_id: PropertyId,
    pub terminated_at: DateTime<Utc>,
}

/// Nessun nome né email: identificatori e ruolo bastano a sapere cosa invalidare.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LeasePartyAdded {
    pub lease_id: LeaseId,
    pub org_id: OrgId,
    pub party_id: LeasePartyId,
    pub role: &'static str,
}

/// Nessuna email: è l'unico dato interessante dell'invito, ed è precisamente
/// quello che un evento pubblicato su un broker non deve trasportare. Chi ha
/// bisogno dell'indirizzo lo legge dall'aggregato.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TenantInvited {
    pub invitation_id: InvitationId,
    pub org_id: OrgId,
    pub lease_id: Option<LeaseId>,
    pub role: &'static str,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InvitationRevoked {
    pub invitation_id: InvitationId,
    pub org_id: OrgId,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InvitationAccepted {
    pub invitation_id: InvitationId,
    pub org_id: OrgId,
    pub user_id: UserId,
    pub lease_id: Option<LeaseId>,
    pub role: &'static str,
}

/// Documento caricato.
///
/// Nessun `original_filename`, nessun MIME, nessun checksum: il nome del file è un
/// dato personale — `contratto-mario-rossi.pdf` nomina una persona — e un evento
/// pubblicato su un broker è un dato che viaggia e si conserva. Viaggiano gli
/// identificatori e la classificazione, che è ciò che serve a chi consuma per
/// sapere **cosa invalidare**; il resto si legge dall'aggregato.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DocumentUploaded {
    pub document_id: DocumentId,
    pub org_id: OrgId,
    /// Tipo dell'oggetto a cui è allegato (`property`, `room`, `lease`).
    pub owner_type: &'static str,
    pub owner_id: uuid::Uuid,
    pub kind: &'static str,
}

impl DocumentUploaded {
    #[must_use]
    pub const fn new(
        document_id: DocumentId,
        org_id: OrgId,
        owner: AttachmentOwner,
        kind: DocumentKind,
    ) -> Self {
        Self {
            document_id,
            org_id,
            owner_type: owner_type_name(owner),
            owner_id: owner.owner_id(),
            kind: kind.as_str(),
        }
    }
}

/// Documento archiviato: soft delete della riga.
///
/// **L'oggetto nello storage sopravvive all'evento.** In questa fetta nessuno lo
/// rimuove: chi consuma non deve dedurre da qui che i byte siano spariti.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DocumentArchived {
    pub document_id: DocumentId,
    pub org_id: OrgId,
    pub owner_type: &'static str,
    pub owner_id: uuid::Uuid,
    pub archived_by_role: &'static str,
}

impl DocumentArchived {
    #[must_use]
    pub const fn new(
        document_id: DocumentId,
        org_id: OrgId,
        owner: AttachmentOwner,
        role: OrgRole,
    ) -> Self {
        Self {
            document_id,
            org_id,
            owner_type: owner_type_name(owner),
            owner_id: owner.owner_id(),
            archived_by_role: role.as_str(),
        }
    }
}

/// Nome dell'owner sul filo dell'evento.
///
/// Vive qui e non su [`AttachmentOwner`] perché è il contratto verso chi consuma
/// l'outbox: coincide oggi con il valore del database, ma le due cose possono
/// divergere e devono poterlo fare separatamente.
const fn owner_type_name(owner: AttachmentOwner) -> &'static str {
    match owner {
        AttachmentOwner::Property(_) => "property",
        AttachmentOwner::Room(_) => "room",
        AttachmentOwner::Lease(_) => "lease",
        AttachmentOwner::Ticket(_) => "ticket",
    }
}

/// Segnalazione aperta.
///
/// **Nessun titolo e nessuna descrizione.** Sono testo libero scritto da una
/// persona su un guasto in casa propria, e un evento pubblicato su un broker è un
/// dato che viaggia e si conserva (regola 18, stessa ragione per cui
/// `LeaseTerminated` non porta `termination_reason`). Nemmeno `reference`: è
/// derivabile dall'identificatore, e chi consuma legge l'aggregato.
///
/// Viaggiano invece immobile, stanza e contratto, che sono ciò che serve a sapere
/// **cosa invalidare**.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TicketOpened {
    pub ticket_id: TicketId,
    pub org_id: OrgId,
    pub property_id: PropertyId,
    pub room_id: Option<RoomId>,
    pub lease_id: Option<LeaseId>,
    pub category: &'static str,
    pub priority: &'static str,
    pub reported_by_kind: &'static str,
}

impl TicketOpened {
    /// Costruito **dall'aggregato** e non da otto argomenti: sarebbero tre
    /// `Option<…Id>` di fila, e scambiarne due compilerebbe senza che nulla se ne
    /// accorga. È anche la garanzia che l'evento descriva la segnalazione appena
    /// scritta e non i valori del comando.
    #[must_use]
    pub const fn from_ticket(ticket: &Ticket) -> Self {
        Self {
            ticket_id: ticket.id(),
            org_id: ticket.org_id(),
            property_id: ticket.property_id(),
            room_id: ticket.room_id(),
            lease_id: ticket.lease_id(),
            category: ticket.category().as_str(),
            priority: ticket.priority().as_str(),
            reported_by_kind: ticket.reported_by_kind().as_str(),
        }
    }
}

/// Transizione di stato.
///
/// Ogni transizione ne produce uno, compresa l'assegnazione — che ne produce
/// **due**, questo e [`TicketAssigned`]: il cambio di stato non dice a chi il
/// lavoro è finito in mano, e chi consuma il solo `ticket.assigned` perderebbe
/// tutte le altre transizioni.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TicketStatusChanged {
    pub ticket_id: TicketId,
    pub org_id: OrgId,
    pub from_status: &'static str,
    pub to_status: &'static str,
    pub changed_by_role: &'static str,
}

impl TicketStatusChanged {
    #[must_use]
    pub const fn new(
        ticket_id: TicketId,
        org_id: OrgId,
        from_status: TicketStatus,
        to_status: TicketStatus,
        role: OrgRole,
    ) -> Self {
        Self {
            ticket_id,
            org_id,
            from_status: from_status.as_str(),
            to_status: to_status.as_str(),
            changed_by_role: role.as_str(),
        }
    }
}

/// Assegnazione della lavorazione. L'identificatore dell'incaricato viaggia
/// perché è ciò che rende l'evento utile a chi notifica; nessun nome, nessuna
/// email.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TicketAssigned {
    pub ticket_id: TicketId,
    pub org_id: OrgId,
    pub assignee_id: UserId,
}

/// Costi stimati o sostenuti modificati.
///
/// **Nomi dei campi, non valori**, come `PropertyUpdated`: chi consuma sa cosa
/// invalidare senza che l'importo di una riparazione viaggi su un broker.
///
/// L'evento esiste benché il piano non lo nominasse: senza, il `PATCH` sui costi
/// sarebbe l'unica modifica del progetto priva dell'evento che la descrive, e
/// romperebbe la proprietà su cui l'outbox si regge
/// ([02 — Architettura](../../../../docs/02-architettura.md)).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TicketCostsUpdated {
    pub ticket_id: TicketId,
    pub org_id: OrgId,
    pub changed_fields: Vec<&'static str>,
}

/// Messaggio scritto nel thread.
///
/// **Nessun corpo.** Vale la stessa ragione del titolo: è ciò che una persona ha
/// scritto, e non deve finire su un broker. `is_internal` viaggia perché chi
/// consuma deve sapere se il messaggio è visibile all'inquilino prima di
/// notificarlo.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TicketMessagePosted {
    pub ticket_id: TicketId,
    pub org_id: OrgId,
    pub message_id: TicketMessageId,
    pub author_kind: &'static str,
    pub is_internal: bool,
}

impl TicketMessagePosted {
    #[must_use]
    pub const fn from_message(message: &TicketMessage) -> Self {
        Self {
            ticket_id: message.ticket_id(),
            org_id: message.org_id(),
            message_id: message.id(),
            author_kind: message.author_kind().as_str(),
            is_internal: message.is_internal(),
        }
    }
}
