use chrono::{DateTime, Utc};

use crate::identity::OrgRole;
use crate::shared::{EventId, OrgId, PropertyId, UserId};

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
