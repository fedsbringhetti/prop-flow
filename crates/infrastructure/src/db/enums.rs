//! Conversioni fra gli enum del dominio e i loro valori testuali nel database.
//!
//! Le query leggono gli enum con un cast esplicito a `text` e li scrivono con un
//! cast esplicito al tipo enum di PostgreSQL. È una scelta deliberata rispetto ai
//! derive `sqlx::Type`: la risoluzione degli OID dei tipi definiti dall'utente
//! dipende dal `search_path` e dalla cache della connessione, mentre il cast
//! testuale ha lo stesso comportamento su qualunque connessione e rende visibile
//! nel SQL quale tipo si sta usando.

use propflow_domain::billing::{BillStatus, ChargeStatus, ChargeType, UtilityType};
use propflow_domain::document::{AttachmentOwnerType, DocumentKind};
use propflow_domain::identity::{MemberStatus, OrgKind, OrgRole, PlatformRole, UserStatus};
use propflow_domain::lease::{LeasePartyRole, LeaseStatus, LeaseType, RentFrequency};
use propflow_domain::ports::RepositoryError;
use propflow_domain::property::{EnergyClass, PropertyStatus, PropertyType, RoomStatus};
use propflow_domain::ticket::{TicketActor, TicketCategory, TicketPriority, TicketStatus};

fn unknown(kind: &'static str, value: &str) -> RepositoryError {
    RepositoryError::Backend {
        details: format!("valore `{value}` non riconosciuto per l'enum {kind}"),
    }
}

macro_rules! string_enum {
    ($kind:literal, $enum:ident, $to:ident, $from:ident, { $($variant:ident => $text:literal),+ $(,)? }) => {
        #[must_use]
        pub fn $to(value: $enum) -> &'static str {
            match value {
                $($enum::$variant => $text),+
            }
        }

        pub fn $from(value: &str) -> Result<$enum, RepositoryError> {
            match value {
                $($text => Ok($enum::$variant),)+
                other => Err(unknown($kind, other)),
            }
        }
    };
}

string_enum!("org_role", OrgRole, org_role_to_db, org_role_from_db, {
    Owner => "owner",
    Manager => "manager",
    Tenant => "tenant",
});

string_enum!("member_status", MemberStatus, member_status_to_db, member_status_from_db, {
    Invited => "invited",
    Active => "active",
    Suspended => "suspended",
    Left => "left",
});

string_enum!("user_status", UserStatus, user_status_to_db, user_status_from_db, {
    Pending => "pending",
    Active => "active",
    Suspended => "suspended",
    Deactivated => "deactivated",
});

string_enum!("platform_role", PlatformRole, platform_role_to_db, platform_role_from_db, {
    User => "user",
    Support => "support",
    Admin => "admin",
});

string_enum!("org_kind", OrgKind, org_kind_to_db, org_kind_from_db, {
    Personal => "personal",
    Business => "business",
    Condominium => "condominium",
});

string_enum!("property_type", PropertyType, property_type_to_db, property_type_from_db, {
    Apartment => "apartment",
    House => "house",
    Studio => "studio",
    Villa => "villa",
    Office => "office",
    Retail => "retail",
    Garage => "garage",
    Land => "land",
    Other => "other",
});

string_enum!("property_status", PropertyStatus, property_status_to_db, property_status_from_db, {
    Draft => "draft",
    Available => "available",
    PartiallyRented => "partially_rented",
    Rented => "rented",
    Maintenance => "maintenance",
    Archived => "archived",
});

string_enum!("room_status", RoomStatus, room_status_to_db, room_status_from_db, {
    Available => "available",
    Occupied => "occupied",
    Unavailable => "unavailable",
});

string_enum!("lease_type", LeaseType, lease_type_to_db, lease_type_from_db, {
    WholeProperty => "whole_property",
    Room => "room",
});

// `expiring` ed `expired` esistono nell'enum del database ma l'applicazione non li
// scrive mai: sono derivati dalle date (ADR-019). Restano nella tabella di
// conversione perché una riga scritta a mano potrebbe contenerli, e un valore
// sconosciuto in lettura deve dare errore.
string_enum!("lease_status", LeaseStatus, lease_status_to_db, lease_status_from_db, {
    Draft => "draft",
    PendingSignature => "pending_signature",
    Active => "active",
    Expiring => "expiring",
    Expired => "expired",
    Terminated => "terminated",
    Cancelled => "cancelled",
});

string_enum!("rent_frequency", RentFrequency, rent_frequency_to_db, rent_frequency_from_db, {
    Monthly => "monthly",
    Quarterly => "quarterly",
    Semiannual => "semiannual",
    Annual => "annual",
});

string_enum!("lease_party_role", LeasePartyRole, lease_party_role_to_db, lease_party_role_from_db, {
    Tenant => "tenant",
    Cotenant => "cotenant",
    Guarantor => "guarantor",
});

string_enum!("document_kind", DocumentKind, document_kind_to_db, document_kind_from_db, {
    LeaseContract => "lease_contract",
    IdDocument => "id_document",
    UtilityBill => "utility_bill",
    Invoice => "invoice",
    Receipt => "receipt",
    Photo => "photo",
    InspectionReport => "inspection_report",
    EnergyCertificate => "energy_certificate",
    CadastralExtract => "cadastral_extract",
    InsurancePolicy => "insurance_policy",
    Other => "other",
});

// `app.attachment_owner_type` ha undici valori, il dominio quattro: sono gli unici
// aggregati che hanno già dominio, caso d'uso ed endpoint. La conseguenza è che un
// documento allegato a una bolletta dà **errore** in lettura invece di essere
// silenziosamente ignorato — è la stessa regola degli altri enum, e qui vale a
// maggior ragione: un documento che l'API non sa mostrare non deve sparire
// dall'elenco senza che nessuno se ne accorga.
string_enum!("attachment_owner_type", AttachmentOwnerType, attachment_owner_type_to_db, attachment_owner_type_from_db, {
    Property => "property",
    Room => "room",
    Lease => "lease",
    Ticket => "ticket",
});

string_enum!("ticket_category", TicketCategory, ticket_category_to_db, ticket_category_from_db, {
    Plumbing => "plumbing",
    Electrical => "electrical",
    HeatingCooling => "heating_cooling",
    Appliance => "appliance",
    Structural => "structural",
    Pest => "pest",
    Cleaning => "cleaning",
    Connectivity => "connectivity",
    Security => "security",
    Other => "other",
});

string_enum!("ticket_priority", TicketPriority, ticket_priority_to_db, ticket_priority_from_db, {
    Low => "low",
    Medium => "medium",
    High => "high",
    Urgent => "urgent",
});

// `resolved` esiste nell'enum del database e l'applicazione non lo scrive mai: il
// flusso con conferma dell'inquilino è stato scartato perché senza scheduler non si
// chiude mai da sé, e la riapertura ottiene lo stesso risultato senza un orologio.
// Resta nella tabella di conversione perché una riga scritta a mano potrebbe
// contenerlo, e un valore sconosciuto in lettura deve dare errore.
string_enum!("ticket_status", TicketStatus, ticket_status_to_db, ticket_status_from_db, {
    Open => "open",
    Triaged => "triaged",
    Assigned => "assigned",
    InProgress => "in_progress",
    WaitingTenant => "waiting_tenant",
    WaitingOwner => "waiting_owner",
    Resolved => "resolved",
    Closed => "closed",
    Cancelled => "cancelled",
});

// `system` e `ai` valgono come `resolved`: nell'enum ci sono, l'applicazione non li
// produce finché il triage AI non esiste.
string_enum!("ticket_actor", TicketActor, ticket_actor_to_db, ticket_actor_from_db, {
    Tenant => "tenant",
    Owner => "owner",
    Manager => "manager",
    System => "system",
    Ai => "ai",
});

string_enum!("utility_type", UtilityType, utility_type_to_db, utility_type_from_db, {
    Electricity => "electricity",
    Gas => "gas",
    Water => "water",
    Heating => "heating",
    Waste => "waste",
    Internet => "internet",
    CondoFees => "condo_fees",
    Other => "other",
});

// Tre valori che l'applicazione non scrive mai, ciascuno per una ragione sua:
// `pending_review` appartiene all'OCR (modulo successivo); `paid` significherebbe
// «il proprietario ha pagato il fornitore» e non è modellato affatto, perché
// `app.payable_type` non comprende `utility_bill`; `disputed` è una contestazione
// senza via di ritorno disegnata. Restano nella tabella di conversione perché una
// riga scritta a mano potrebbe contenerli, e un valore sconosciuto in lettura deve
// dare errore.
string_enum!("bill_status", BillStatus, bill_status_to_db, bill_status_from_db, {
    Draft => "draft",
    PendingReview => "pending_review",
    Validated => "validated",
    Allocated => "allocated",
    Paid => "paid",
    Disputed => "disputed",
    Void => "void",
});

string_enum!("charge_type", ChargeType, charge_type_to_db, charge_type_from_db, {
    Rent => "rent",
    Deposit => "deposit",
    UtilityRecharge => "utility_recharge",
    LateFee => "late_fee",
    Maintenance => "maintenance",
    Adjustment => "adjustment",
    Other => "other",
});

// Quattro valori su sette che l'applicazione non scrive: `draft` perché un
// addebito nasce emesso, `overdue` perché si deriva dalla scadenza (ADR-019), e
// `partially_paid`/`paid` perché li produrrà la riconciliazione del modulo dei
// pagamenti. Valgono le stesse ragioni di `lease_status`.
string_enum!("charge_status", ChargeStatus, charge_status_to_db, charge_status_from_db, {
    Draft => "draft",
    Issued => "issued",
    PartiallyPaid => "partially_paid",
    Paid => "paid",
    Overdue => "overdue",
    Void => "void",
    WrittenOff => "written_off",
});

// Le classi energetiche restano maiuscole anche nel database: 'A4', 'B', 'G'.
string_enum!("energy_class", EnergyClass, energy_class_to_db, energy_class_from_db, {
    A4 => "A4",
    A3 => "A3",
    A2 => "A2",
    A1 => "A1",
    A => "A",
    B => "B",
    C => "C",
    D => "D",
    E => "E",
    F => "F",
    G => "G",
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_is_lossless() {
        for role in [OrgRole::Owner, OrgRole::Manager, OrgRole::Tenant] {
            assert_eq!(org_role_from_db(org_role_to_db(role)).expect("noto"), role);
        }
        for status in [
            PropertyStatus::Draft,
            PropertyStatus::Available,
            PropertyStatus::PartiallyRented,
            PropertyStatus::Rented,
            PropertyStatus::Maintenance,
            PropertyStatus::Archived,
        ] {
            assert_eq!(
                property_status_from_db(property_status_to_db(status)).expect("noto"),
                status
            );
        }
        for status in [
            RoomStatus::Available,
            RoomStatus::Occupied,
            RoomStatus::Unavailable,
        ] {
            assert_eq!(
                room_status_from_db(room_status_to_db(status)).expect("noto"),
                status
            );
        }
        for status in [
            LeaseStatus::Draft,
            LeaseStatus::PendingSignature,
            LeaseStatus::Active,
            LeaseStatus::Expiring,
            LeaseStatus::Expired,
            LeaseStatus::Terminated,
            LeaseStatus::Cancelled,
        ] {
            assert_eq!(
                lease_status_from_db(lease_status_to_db(status)).expect("noto"),
                status
            );
        }
        for role in [
            LeasePartyRole::Tenant,
            LeasePartyRole::Cotenant,
            LeasePartyRole::Guarantor,
        ] {
            assert_eq!(
                lease_party_role_from_db(lease_party_role_to_db(role)).expect("noto"),
                role
            );
        }
        for frequency in [
            RentFrequency::Monthly,
            RentFrequency::Quarterly,
            RentFrequency::Semiannual,
            RentFrequency::Annual,
        ] {
            assert_eq!(
                rent_frequency_from_db(rent_frequency_to_db(frequency)).expect("noto"),
                frequency
            );
        }
        for kind in [
            DocumentKind::LeaseContract,
            DocumentKind::IdDocument,
            DocumentKind::UtilityBill,
            DocumentKind::Invoice,
            DocumentKind::Receipt,
            DocumentKind::Photo,
            DocumentKind::InspectionReport,
            DocumentKind::EnergyCertificate,
            DocumentKind::CadastralExtract,
            DocumentKind::InsurancePolicy,
            DocumentKind::Other,
        ] {
            assert_eq!(
                document_kind_from_db(document_kind_to_db(kind)).expect("noto"),
                kind
            );
            // Il nome sul filo degli eventi e quello del database coincidono oggi,
            // e il test lo fissa: se qualcuno cambiasse l'uno per una rinomina
            // cosmetica, l'altro non lo seguirebbe in silenzio.
            assert_eq!(document_kind_to_db(kind), kind.as_str());
        }
        for owner in [
            AttachmentOwnerType::Property,
            AttachmentOwnerType::Room,
            AttachmentOwnerType::Lease,
            AttachmentOwnerType::Ticket,
        ] {
            assert_eq!(
                attachment_owner_type_from_db(attachment_owner_type_to_db(owner)).expect("noto"),
                owner
            );
        }
        for status in [
            TicketStatus::Open,
            TicketStatus::Triaged,
            TicketStatus::Assigned,
            TicketStatus::InProgress,
            TicketStatus::WaitingTenant,
            TicketStatus::WaitingOwner,
            TicketStatus::Resolved,
            TicketStatus::Closed,
            TicketStatus::Cancelled,
        ] {
            assert_eq!(
                ticket_status_from_db(ticket_status_to_db(status)).expect("noto"),
                status
            );
            // Il nome sul filo degli eventi e quello del database coincidono, e il
            // test lo fissa: la costante `TicketStatus::WORKING` compare come
            // letterale SQL nel repository, e una rinomina che tocchi solo uno dei
            // due farebbe smettere la coda di usare `tickets_open_ix`.
            assert_eq!(ticket_status_to_db(status), status.as_str());
        }
        for category in [
            TicketCategory::Plumbing,
            TicketCategory::Electrical,
            TicketCategory::HeatingCooling,
            TicketCategory::Appliance,
            TicketCategory::Structural,
            TicketCategory::Pest,
            TicketCategory::Cleaning,
            TicketCategory::Connectivity,
            TicketCategory::Security,
            TicketCategory::Other,
        ] {
            assert_eq!(
                ticket_category_from_db(ticket_category_to_db(category)).expect("noto"),
                category
            );
            assert_eq!(ticket_category_to_db(category), category.as_str());
        }
        for priority in [
            TicketPriority::Low,
            TicketPriority::Medium,
            TicketPriority::High,
            TicketPriority::Urgent,
        ] {
            assert_eq!(
                ticket_priority_from_db(ticket_priority_to_db(priority)).expect("noto"),
                priority
            );
        }
        for actor in [
            TicketActor::Tenant,
            TicketActor::Owner,
            TicketActor::Manager,
            TicketActor::System,
            TicketActor::Ai,
        ] {
            assert_eq!(
                ticket_actor_from_db(ticket_actor_to_db(actor)).expect("noto"),
                actor
            );
        }
        for utility in [
            UtilityType::Electricity,
            UtilityType::Gas,
            UtilityType::Water,
            UtilityType::Heating,
            UtilityType::Waste,
            UtilityType::Internet,
            UtilityType::CondoFees,
            UtilityType::Other,
        ] {
            assert_eq!(
                utility_type_from_db(utility_type_to_db(utility)).expect("noto"),
                utility
            );
            assert_eq!(utility_type_to_db(utility), utility.as_str());
        }
        for status in [
            BillStatus::Draft,
            BillStatus::PendingReview,
            BillStatus::Validated,
            BillStatus::Allocated,
            BillStatus::Paid,
            BillStatus::Disputed,
            BillStatus::Void,
        ] {
            assert_eq!(
                bill_status_from_db(bill_status_to_db(status)).expect("noto"),
                status
            );
            assert_eq!(bill_status_to_db(status), status.as_str());
        }
        for charge_type in [
            ChargeType::Rent,
            ChargeType::Deposit,
            ChargeType::UtilityRecharge,
            ChargeType::LateFee,
            ChargeType::Maintenance,
            ChargeType::Adjustment,
            ChargeType::Other,
        ] {
            assert_eq!(
                charge_type_from_db(charge_type_to_db(charge_type)).expect("noto"),
                charge_type
            );
            assert_eq!(charge_type_to_db(charge_type), charge_type.as_str());
        }
        for status in [
            ChargeStatus::Draft,
            ChargeStatus::Issued,
            ChargeStatus::PartiallyPaid,
            ChargeStatus::Paid,
            ChargeStatus::Overdue,
            ChargeStatus::Void,
            ChargeStatus::WrittenOff,
        ] {
            assert_eq!(
                charge_status_from_db(charge_status_to_db(status)).expect("noto"),
                status
            );
            // Come per `TicketStatus`: la costante `ChargeStatus::OUTSTANDING`
            // compare come letterale SQL nel repository, e una rinomina che
            // toccasse solo uno dei due farebbe smettere lo scadenzario di usare
            // `charges_outstanding_ix`.
            assert_eq!(charge_status_to_db(status), status.as_str());
        }
        assert_eq!(lease_type_to_db(LeaseType::WholeProperty), "whole_property");
        assert_eq!(energy_class_to_db(EnergyClass::A4), "A4");
    }

    #[test]
    fn unknown_values_are_reported_not_defaulted() {
        // Un enum esteso nel database e non nel codice deve dare errore, non
        // ricadere silenziosamente su un valore plausibile.
        assert!(org_role_from_db("accountant").is_err());
        assert!(property_status_from_db("").is_err());
        // Un owner fuori perimetro non ricade su un valore plausibile: nessuno dei
        // quattro che l'API espone è "abbastanza vicino" a una bolletta.
        assert!(attachment_owner_type_from_db("utility_bill").is_err());
        assert!(ticket_status_from_db("escalated").is_err());
        assert!(ticket_actor_from_db("vendor").is_err());
        assert!(bill_status_from_db("archived").is_err());
        assert!(charge_status_from_db("refunded").is_err());
        assert!(utility_type_from_db("sewage").is_err());
    }
}
