use chrono::{DateTime, Utc};

use crate::identity::OrgRole;
use crate::shared::{
    EventId, InvitationId, LeaseId, LeasePartyId, OrgId, PropertyId, RoomId, UserId,
};

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
