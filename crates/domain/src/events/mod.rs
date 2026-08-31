use chrono::{DateTime, Utc};

use crate::billing::{BillStatus, Charge, ChargeStatus, UtilityBill};
use crate::document::{AttachmentOwner, DocumentKind};
use crate::identity::OrgRole;
use crate::payment::Payment;
use crate::shared::{
    ChargeId, DocumentId, EventId, InvitationId, LeaseId, LeasePartyId, OrgId, PaymentId,
    PropertyId, RoomId, TicketId, TicketMessageId, UserId, UtilityBillId,
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
    UtilityBillRegistered(UtilityBillRegistered),
    UtilityBillUpdated(UtilityBillUpdated),
    UtilityBillStatusChanged(UtilityBillStatusChanged),
    ChargeIssued(ChargeIssued),
    ChargeStatusChanged(ChargeStatusChanged),
    PaymentDeclared(PaymentDeclared),
    PaymentConfirmed(PaymentConfirmed),
    PaymentRejected(PaymentRejected),
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
            Self::UtilityBillRegistered(_) => "billing.utility_bill.registered",
            Self::UtilityBillUpdated(_) => "billing.utility_bill.updated",
            Self::UtilityBillStatusChanged(_) => "billing.utility_bill.status_changed",
            Self::ChargeIssued(_) => "billing.charge.issued",
            Self::ChargeStatusChanged(_) => "billing.charge.status_changed",
            Self::PaymentDeclared(_) => "payment.declared",
            Self::PaymentConfirmed(_) => "payment.confirmed",
            Self::PaymentRejected(_) => "payment.rejected",
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
            Self::UtilityBillRegistered(_)
            | Self::UtilityBillUpdated(_)
            | Self::UtilityBillStatusChanged(_) => "UtilityBill",
            // Il riaddebito produce **due** eventi, uno per aggregato: la bolletta
            // che passa ad `allocated` e l'addebito che nasce. Chi consuma il solo
            // `charge.issued` non saprebbe che la bolletta è stata usata, e chi
            // consuma il solo `utility_bill.status_changed` non saprebbe a chi.
            Self::ChargeIssued(_) | Self::ChargeStatusChanged(_) => "Charge",
            // Il pagamento **è** un aggregato a sé, a differenza del messaggio di
            // una segnalazione: ha il proprio ciclo di vita, la propria
            // autorizzazione e la propria riga di audit. L'addebito che si muove di
            // conseguenza produce il suo `billing.charge.status_changed`, come il
            // riaddebito produce due eventi per due aggregati.
            Self::PaymentDeclared(_) | Self::PaymentConfirmed(_) | Self::PaymentRejected(_) => {
                "Payment"
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
            Self::DocumentUploaded(event) => event.document_id.as_uuid(),
            Self::DocumentArchived(event) => event.document_id.as_uuid(),
            Self::TicketOpened(event) => event.ticket_id.as_uuid(),
            Self::TicketStatusChanged(event) => event.ticket_id.as_uuid(),
            Self::TicketAssigned(event) => event.ticket_id.as_uuid(),
            Self::TicketCostsUpdated(event) => event.ticket_id.as_uuid(),
            Self::TicketMessagePosted(event) => event.ticket_id.as_uuid(),
            Self::UtilityBillRegistered(event) => event.utility_bill_id.as_uuid(),
            Self::UtilityBillUpdated(event) => event.utility_bill_id.as_uuid(),
            Self::UtilityBillStatusChanged(event) => event.utility_bill_id.as_uuid(),
            Self::ChargeIssued(event) => event.charge_id.as_uuid(),
            Self::ChargeStatusChanged(event) => event.charge_id.as_uuid(),
            Self::PaymentDeclared(event) => event.payment_id.as_uuid(),
            Self::PaymentConfirmed(event) => event.payment_id.as_uuid(),
            Self::PaymentRejected(event) => event.payment_id.as_uuid(),
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
            Self::UtilityBillRegistered(event) => Some(event.org_id),
            Self::UtilityBillUpdated(event) => Some(event.org_id),
            Self::UtilityBillStatusChanged(event) => Some(event.org_id),
            Self::ChargeIssued(event) => Some(event.org_id),
            Self::ChargeStatusChanged(event) => Some(event.org_id),
            Self::PaymentDeclared(event) => Some(event.org_id),
            Self::PaymentConfirmed(event) => Some(event.org_id),
            Self::PaymentRejected(event) => Some(event.org_id),
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
        AttachmentOwner::UtilityBill(_) => "utility_bill",
        AttachmentOwner::Charge(_) => "charge",
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

/// Bolletta registrata.
///
/// **Nessun importo viaggia**, ed è la stessa regola di [`TicketCostsUpdated`] e
/// di [`RoomUpdated`], che non porta il canone: un evento pubblicato su un broker
/// è un dato che viaggia e si conserva.
///
/// **Nemmeno `supplier_name` e `supplier_vat`**, che sembrano dati d'azienda e
/// per un fornitore individuale — un idraulico, un amministratore di condominio
/// che fattura in proprio — nominano una persona e ne portano l'identificativo
/// fiscale. Non c'è `invoice_number` per la stessa ragione: insieme alla partita
/// IVA identifica un documento fiscale, e l'insieme dei due è precisamente ciò
/// che l'indice di deduplica tratta come identità.
///
/// Viaggiano invece immobile, stanza e contratto, che sono ciò che serve a sapere
/// **cosa invalidare**, e il tipo di utenza, che è una classificazione chiusa.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UtilityBillRegistered {
    pub utility_bill_id: UtilityBillId,
    pub org_id: OrgId,
    pub property_id: PropertyId,
    pub room_id: Option<RoomId>,
    pub lease_id: Option<LeaseId>,
    pub utility_type: &'static str,
}

impl UtilityBillRegistered {
    /// Costruito **dall'aggregato** e non da sei argomenti: sarebbero due
    /// `Option<…Id>` di fila, e scambiarli compilerebbe senza che nulla se ne
    /// accorga. È anche la garanzia che l'evento descriva la bolletta appena
    /// scritta e non i valori del comando.
    #[must_use]
    pub const fn from_bill(bill: &UtilityBill) -> Self {
        Self {
            utility_bill_id: bill.id(),
            org_id: bill.org_id(),
            property_id: bill.property_id(),
            room_id: bill.room_id(),
            lease_id: bill.lease_id(),
            utility_type: bill.utility_type().as_str(),
        }
    }
}

/// Bolletta corretta.
///
/// **Nomi dei campi, non valori**, come [`PropertyUpdated`] e
/// [`TicketCostsUpdated`]: chi consuma sa cosa invalidare senza che l'importo di
/// una bolletta, o il nome del suo fornitore, viaggino su un broker.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UtilityBillUpdated {
    pub utility_bill_id: UtilityBillId,
    pub org_id: OrgId,
    pub changed_fields: Vec<&'static str>,
}

/// Transizione di stato della bolletta.
///
/// Ne produce uno ogni transizione, **compresa quella implicita** con cui una
/// correzione riporta a `draft` una bolletta validata: è il caso in cui l'evento
/// serve di più, perché è l'unico modo in cui chi consuma scopre che una
/// validazione è stata disfatta senza che nessuno abbia chiesto di disfarla.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UtilityBillStatusChanged {
    pub utility_bill_id: UtilityBillId,
    pub org_id: OrgId,
    pub from_status: &'static str,
    pub to_status: &'static str,
    pub changed_by_role: &'static str,
}

impl UtilityBillStatusChanged {
    #[must_use]
    pub const fn new(
        utility_bill_id: UtilityBillId,
        org_id: OrgId,
        from_status: BillStatus,
        to_status: BillStatus,
        role: OrgRole,
    ) -> Self {
        Self {
            utility_bill_id,
            org_id,
            from_status: from_status.as_str(),
            to_status: to_status.as_str(),
            changed_by_role: role.as_str(),
        }
    }
}

/// Addebito emesso.
///
/// **Nessun importo viaggia**, per la ragione di sempre — e qui la ragione è più
/// forte che altrove: questo importo è ciò che una persona identificabile
/// (`lease_id` porta al contratto e da lì alle sue parti) deve pagare, e la coppia
/// «chi» e «quanto» è esattamente il dato che non deve conservarsi su un broker.
/// Nemmeno `description`, che è testo libero scritto da una persona.
///
/// `due_date` **sì**: è una data, non un importo, ed è ciò che serve a chi
/// programma un sollecito — che è il consumatore per cui questo evento esiste.
///
/// `utility_bill_id` viaggia quando l'addebito nasce da un riaddebito: chi
/// consuma deve poter collegare i due eventi della stessa transazione.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChargeIssued {
    pub charge_id: ChargeId,
    pub org_id: OrgId,
    pub lease_id: LeaseId,
    pub charge_type: &'static str,
    pub due_date: chrono::NaiveDate,
    pub utility_bill_id: Option<UtilityBillId>,
}

impl ChargeIssued {
    #[must_use]
    pub const fn from_charge(charge: &Charge) -> Self {
        Self {
            charge_id: charge.id(),
            org_id: charge.org_id(),
            lease_id: charge.lease_id(),
            charge_type: charge.charge_type().as_str(),
            due_date: charge.due_date(),
            utility_bill_id: charge.utility_bill_id(),
        }
    }
}

/// Transizione di stato dell'addebito: annullamento o rinuncia all'incasso.
///
/// `to_status` non porta mai `overdue`, e non perché l'evento lo escluda: quello
/// stato **non si scrive**, si deriva dalla scadenza. Un consumatore che aspetti
/// un evento per sapere che un addebito è scaduto aspetterebbe per sempre, e deve
/// invece guardare `due_date` di [`ChargeIssued`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChargeStatusChanged {
    pub charge_id: ChargeId,
    pub org_id: OrgId,
    pub from_status: &'static str,
    pub to_status: &'static str,
    pub changed_by_role: &'static str,
}

impl ChargeStatusChanged {
    #[must_use]
    pub const fn new(
        charge_id: ChargeId,
        org_id: OrgId,
        from_status: ChargeStatus,
        to_status: ChargeStatus,
        role: OrgRole,
    ) -> Self {
        Self {
            charge_id,
            org_id,
            from_status: from_status.as_str(),
            to_status: to_status.as_str(),
            changed_by_role: role.as_str(),
        }
    }
}

/// Un inquilino dichiara di aver pagato.
///
/// **Nessun importo viaggia**, e qui a maggior ragione che altrove: la coppia
/// «chi» e «quanto ha pagato» è esattamente il dato che non deve conservarsi su un
/// broker. Vale la regola generale — un evento pubblicato è un dato che viaggia e
/// si conserva — con l'aggravante che qui la coppia è finanziaria e riferita a una
/// persona identificata.
///
/// Viaggiano invece gli identificatori, che chi consuma può risolvere sotto la
/// propria autorizzazione, e il **metodo**: `card` e `cash` chiedono seguiti
/// diversi, ed è ciò per cui questo evento esiste.
///
/// `charge_id` c'è perché una dichiarazione da sola non dice a chi si riferisce, e
/// chi consuma non deve dover leggere il pagamento per saperlo.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PaymentDeclared {
    pub payment_id: PaymentId,
    pub org_id: OrgId,
    pub charge_id: ChargeId,
    pub declared_by: UserId,
    pub method: &'static str,
}

impl PaymentDeclared {
    #[must_use]
    pub const fn new(payment: &Payment, declared_by: UserId) -> Self {
        Self {
            payment_id: payment.id(),
            org_id: payment.org_id(),
            charge_id: payment.payable().charge_id(),
            declared_by,
            method: payment.method().as_str(),
        }
    }
}

/// Il denaro è vero: il pagamento è entrato in `succeeded`.
///
/// **Le due vie convergono qui.** Lo emette sia la conferma di una dichiarazione
/// sia la registrazione diretta di un incasso da parte dello staff, ed è
/// deliberato: chi consuma reagisce al fatto che il denaro è diventato vero, non
/// alla porta da cui è passato. A distinguere le due vie basta l'esistenza — o
/// l'assenza — del [`PaymentDeclared`] che lo precede, che porta lo stesso
/// `payment_id`.
///
/// **Nessun importo**, come per la dichiarazione. Chi ha bisogno della cifra la
/// legge dall'aggregato sotto la propria autorizzazione.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PaymentConfirmed {
    pub payment_id: PaymentId,
    pub org_id: OrgId,
    pub charge_id: ChargeId,
    pub method: &'static str,
    pub confirmed_by_role: &'static str,
}

impl PaymentConfirmed {
    #[must_use]
    pub const fn new(payment: &Payment, role: OrgRole) -> Self {
        Self {
            payment_id: payment.id(),
            org_id: payment.org_id(),
            charge_id: payment.payable().charge_id(),
            method: payment.method().as_str(),
            confirmed_by_role: role.as_str(),
        }
    }
}

/// Una dichiarazione è stata rifiutata: il pagamento va a `cancelled`.
///
/// **Non porta una motivazione**, e non è una dimenticanza: `app.payments` non ha
/// una colonna dove metterla — `failure_message` descrive un fallimento presso un
/// provider — e un evento che trasportasse un testo libero scritto da una persona
/// su un pagamento di un'altra persona sarebbe il posto peggiore dove metterlo. Il
/// giorno in cui una motivazione servisse, è una colonna e una migration.
///
/// Non è `failed`: quello significherebbe che il pagamento non è andato a buon fine
/// **presso il provider**, e con `manual` un provider non c'è.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PaymentRejected {
    pub payment_id: PaymentId,
    pub org_id: OrgId,
    pub charge_id: ChargeId,
    pub rejected_by_role: &'static str,
}

impl PaymentRejected {
    #[must_use]
    pub const fn new(payment: &Payment, role: OrgRole) -> Self {
        Self {
            payment_id: payment.id(),
            org_id: payment.org_id(),
            charge_id: payment.payable().charge_id(),
            rejected_by_role: role.as_str(),
        }
    }
}
