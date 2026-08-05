//! Conversioni fra gli enum del dominio e i loro valori testuali nel database.
//!
//! Le query leggono gli enum con un cast esplicito a `text` e li scrivono con un
//! cast esplicito al tipo enum di PostgreSQL. È una scelta deliberata rispetto ai
//! derive `sqlx::Type`: la risoluzione degli OID dei tipi definiti dall'utente
//! dipende dal `search_path` e dalla cache della connessione, mentre il cast
//! testuale ha lo stesso comportamento su qualunque connessione e rende visibile
//! nel SQL quale tipo si sta usando.

use propflow_domain::identity::{MemberStatus, OrgKind, OrgRole, PlatformRole, UserStatus};
use propflow_domain::ports::RepositoryError;
use propflow_domain::property::{EnergyClass, PropertyStatus, PropertyType, RoomStatus};

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
        assert_eq!(energy_class_to_db(EnergyClass::A4), "A4");
    }

    #[test]
    fn unknown_values_are_reported_not_defaulted() {
        // Un enum esteso nel database e non nel codice deve dare errore, non
        // ricadere silenziosamente su un valore plausibile.
        assert!(org_role_from_db("accountant").is_err());
        assert!(property_status_from_db("").is_err());
    }
}
