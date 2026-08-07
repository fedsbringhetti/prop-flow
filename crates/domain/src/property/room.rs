use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use super::aggregate::Patch;
use crate::error::{DomainError, DomainResult};
use crate::shared::{Currency, Money, OrgId, PropertyId, RoomId, text};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomStatus {
    Available,
    Occupied,
    Unavailable,
}

/// Insieme di modifiche applicabili a una stanza.
///
/// Stessa semantica di [`PropertyChanges`](super::PropertyChanges): `Patch<T>`
/// distingue "campo non menzionato" da "campo azzerato".
#[derive(Debug, Clone, Default)]
pub struct RoomChanges {
    pub name: Option<String>,
    pub status: Option<RoomStatus>,
    pub size_sqm: Patch<Decimal>,
    pub floor: Patch<i16>,
    pub has_private_bathroom: Option<bool>,
    pub is_furnished: Patch<bool>,
    pub monthly_rent: Patch<Decimal>,
    pub utility_share_percent: Patch<Decimal>,
}

impl RoomChanges {
    /// Nomi dei campi menzionati dal chiamante, nell'ordine di dichiarazione.
    /// Finiscono nell'evento `RoomUpdated` al posto dei valori.
    #[must_use]
    pub fn changed_field_names(&self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        if self.name.is_some() {
            fields.push("name");
        }
        if self.status.is_some() {
            fields.push("status");
        }
        if self.size_sqm.is_some() {
            fields.push("size_sqm");
        }
        if self.floor.is_some() {
            fields.push("floor");
        }
        if self.has_private_bathroom.is_some() {
            fields.push("has_private_bathroom");
        }
        if self.is_furnished.is_some() {
            fields.push("is_furnished");
        }
        if self.monthly_rent.is_some() {
            fields.push("monthly_rent");
        }
        if self.utility_share_percent.is_some() {
            fields.push("utility_share_percent");
        }
        fields
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.status.is_none()
            && self.size_sqm.is_none()
            && self.floor.is_none()
            && self.has_private_bathroom.is_none()
            && self.is_furnished.is_none()
            && self.monthly_rent.is_none()
            && self.utility_share_percent.is_none()
    }
}

/// Stanza di un immobile in co-living.
///
/// Non è un aggregato radice: appartiene all'immobile che la contiene, e per
/// questo porta il suo `property_id` insieme all'`org_id`. La coerenza fra i due
/// non è affidata al codice: il trigger `app.tg_assert_org_matches_property`
/// rifiuta una stanza con `org_id` diverso da quello dell'immobile padre, che
/// altrimenti sarebbe una riga invisibile alla RLS e quindi non più correggibile
/// dall'interfaccia.
#[derive(Debug, Clone)]
pub struct Room {
    id: RoomId,
    org_id: OrgId,
    property_id: PropertyId,
    name: String,
    status: RoomStatus,
    size_sqm: Option<Decimal>,
    floor: Option<i16>,
    has_private_bathroom: bool,
    is_furnished: Option<bool>,
    monthly_rent: Option<Money>,
    currency: Currency,
    /// Quota di ripartizione delle utenze, in percentuale. `None` significa
    /// "ripartizione in parti uguali": è l'assenza di una quota dedicata, non
    /// una quota pari a zero.
    utility_share_percent: Option<Decimal>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
}

impl Room {
    /// Una stanza nasce `Available`: lo stato passa a `Occupied` quando un
    /// contratto la impegna, non alla creazione della scheda.
    pub fn create(
        id: RoomId,
        org_id: OrgId,
        property_id: PropertyId,
        name: &str,
        now: DateTime<Utc>,
    ) -> DomainResult<Self> {
        Ok(Self {
            id,
            org_id,
            property_id,
            name: text::required("name", name, 100)?,
            status: RoomStatus::Available,
            size_sqm: None,
            floor: None,
            has_private_bathroom: false,
            is_furnished: None,
            monthly_rent: None,
            currency: Currency::EUR,
            utility_share_percent: None,
            created_at: now,
            updated_at: now,
            archived_at: None,
        })
    }

    /// Applica una modifica parziale. Restituisce `false` se non c'era nulla da
    /// cambiare, così il chiamante evita una scrittura e un evento inutili.
    pub fn apply(&mut self, changes: RoomChanges, now: DateTime<Utc>) -> DomainResult<bool> {
        self.ensure_mutable()?;
        if changes.is_empty() {
            return Ok(false);
        }

        if let Some(name) = changes.name {
            self.name = text::required("name", &name, 100)?;
        }
        if let Some(status) = changes.status {
            self.status = status;
        }
        if let Some(size) = changes.size_sqm {
            self.size_sqm = size.map(validate_size).transpose()?;
        }
        if let Some(floor) = changes.floor {
            self.floor = floor;
        }
        if let Some(has_private_bathroom) = changes.has_private_bathroom {
            self.has_private_bathroom = has_private_bathroom;
        }
        if let Some(is_furnished) = changes.is_furnished {
            self.is_furnished = is_furnished;
        }
        if let Some(rent) = changes.monthly_rent {
            self.monthly_rent = rent
                .map(|amount| Money::non_negative(amount, self.currency))
                .transpose()?;
        }
        if let Some(share) = changes.utility_share_percent {
            self.utility_share_percent = share.map(validate_percentage).transpose()?;
        }

        self.updated_at = now;
        Ok(true)
    }

    /// Archivia la stanza: soft delete. Il canone e lo storico restano leggibili
    /// perché i contratti che la citano continuano a esistere.
    pub fn archive(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        self.ensure_mutable()?;
        self.archived_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    fn ensure_mutable(&self) -> DomainResult<()> {
        if self.archived_at.is_some() {
            return Err(DomainError::invariant(
                "una stanza archiviata non può essere modificata",
            ));
        }
        Ok(())
    }

    #[must_use]
    pub const fn id(&self) -> RoomId {
        self.id
    }
    #[must_use]
    pub const fn org_id(&self) -> OrgId {
        self.org_id
    }
    #[must_use]
    pub const fn property_id(&self) -> PropertyId {
        self.property_id
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub const fn status(&self) -> RoomStatus {
        self.status
    }
    #[must_use]
    pub const fn size_sqm(&self) -> Option<Decimal> {
        self.size_sqm
    }
    #[must_use]
    pub const fn floor(&self) -> Option<i16> {
        self.floor
    }
    #[must_use]
    pub const fn has_private_bathroom(&self) -> bool {
        self.has_private_bathroom
    }
    #[must_use]
    pub const fn is_furnished(&self) -> Option<bool> {
        self.is_furnished
    }
    #[must_use]
    pub const fn monthly_rent(&self) -> Option<Money> {
        self.monthly_rent
    }
    #[must_use]
    pub const fn currency(&self) -> Currency {
        self.currency
    }
    #[must_use]
    pub const fn utility_share_percent(&self) -> Option<Decimal> {
        self.utility_share_percent
    }
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
    #[must_use]
    pub const fn archived_at(&self) -> Option<DateTime<Utc>> {
        self.archived_at
    }

    /// Ricostruzione da persistenza: non riesegue le invarianti, perché una
    /// stanza già salvata deve poter essere riletta anche se le regole sono
    /// cambiate da allora.
    #[must_use]
    pub fn from_persistence(parts: RoomParts) -> Self {
        let RoomParts {
            id,
            org_id,
            property_id,
            name,
            status,
            size_sqm,
            floor,
            has_private_bathroom,
            is_furnished,
            monthly_rent,
            currency,
            utility_share_percent,
            created_at,
            updated_at,
            archived_at,
        } = parts;
        Self {
            id,
            org_id,
            property_id,
            name,
            status,
            size_sqm,
            floor,
            has_private_bathroom,
            is_furnished,
            monthly_rent,
            currency,
            utility_share_percent,
            created_at,
            updated_at,
            archived_at,
        }
    }
}

/// Dati grezzi di una stanza letti dalla persistenza.
#[derive(Debug, Clone)]
pub struct RoomParts {
    pub id: RoomId,
    pub org_id: OrgId,
    pub property_id: PropertyId,
    pub name: String,
    pub status: RoomStatus,
    pub size_sqm: Option<Decimal>,
    pub floor: Option<i16>,
    pub has_private_bathroom: bool,
    pub is_furnished: Option<bool>,
    pub monthly_rent: Option<Money>,
    pub currency: Currency,
    pub utility_share_percent: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

fn validate_size(value: Decimal) -> DomainResult<Decimal> {
    if value <= Decimal::ZERO {
        return Err(DomainError::validation("size_sqm", "deve essere positiva"));
    }
    if value.scale() > 2 {
        return Err(DomainError::validation(
            "size_sqm",
            "ammessi al massimo due decimali",
        ));
    }
    Ok(value)
}

/// Replica il dominio `app.percentage` (`NUMERIC(5,2)` fra 0 e 100): validare qui
/// dà un 422 leggibile invece di una violazione di vincolo.
fn validate_percentage(value: Decimal) -> DomainResult<Decimal> {
    if value < Decimal::ZERO || value > Decimal::from(100) {
        return Err(DomainError::validation(
            "utility_share_percent",
            "deve essere compresa fra 0 e 100",
        ));
    }
    if value.scale() > 2 {
        return Err(DomainError::validation(
            "utility_share_percent",
            "ammessi al massimo due decimali",
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn room() -> Room {
        Room::create(
            RoomId::generate(),
            OrgId::generate(),
            PropertyId::generate(),
            "Camera matrimoniale",
            Utc::now(),
        )
        .expect("stanza valida")
    }

    fn dec(raw: &str) -> Decimal {
        Decimal::from_str(raw).expect("decimale di test valido")
    }

    #[test]
    fn new_room_is_available_and_not_archived() {
        let room = room();
        assert_eq!(room.status(), RoomStatus::Available);
        assert_eq!(room.archived_at(), None);
        assert!(!room.has_private_bathroom());
    }

    #[test]
    fn name_is_required_and_trimmed() {
        assert!(
            Room::create(
                RoomId::generate(),
                OrgId::generate(),
                PropertyId::generate(),
                "   ",
                Utc::now(),
            )
            .is_err()
        );

        let room = Room::create(
            RoomId::generate(),
            OrgId::generate(),
            PropertyId::generate(),
            "  Singola  ",
            Utc::now(),
        )
        .expect("valida");
        assert_eq!(room.name(), "Singola");
    }

    #[test]
    fn empty_patch_reports_no_change() {
        let mut room = room();
        assert!(
            !room
                .apply(RoomChanges::default(), Utc::now())
                .expect("nessun errore")
        );
    }

    #[test]
    fn changed_field_names_lists_only_mentioned_fields() {
        let changes = RoomChanges {
            name: Some("Doppia".into()),
            monthly_rent: Some(None),
            ..Default::default()
        };
        assert_eq!(changes.changed_field_names(), vec!["name", "monthly_rent"]);
        assert!(RoomChanges::default().changed_field_names().is_empty());
    }

    #[test]
    fn rent_can_be_set_and_cleared() {
        let mut room = room();
        room.apply(
            RoomChanges {
                monthly_rent: Some(Some(dec("450.00"))),
                ..Default::default()
            },
            Utc::now(),
        )
        .expect("valido");
        assert_eq!(
            room.monthly_rent().map(|money| money.amount()),
            Some(dec("450.00"))
        );

        room.apply(
            RoomChanges {
                monthly_rent: Some(None),
                ..Default::default()
            },
            Utc::now(),
        )
        .expect("valido");
        assert_eq!(room.monthly_rent(), None);
    }

    #[test]
    fn rent_cannot_be_negative() {
        let mut room = room();
        assert!(
            room.apply(
                RoomChanges {
                    monthly_rent: Some(Some(dec("-1.00"))),
                    ..Default::default()
                },
                Utc::now(),
            )
            .is_err()
        );
    }

    #[test]
    fn utility_share_must_stay_within_zero_and_hundred() {
        let mut room = room();
        for bad in ["-0.01", "100.01", "33.333"] {
            assert!(
                room.apply(
                    RoomChanges {
                        utility_share_percent: Some(Some(dec(bad))),
                        ..Default::default()
                    },
                    Utc::now(),
                )
                .is_err(),
                "{bad} doveva essere rifiutata"
            );
        }
        assert!(
            room.apply(
                RoomChanges {
                    utility_share_percent: Some(Some(dec("33.33"))),
                    ..Default::default()
                },
                Utc::now(),
            )
            .is_ok()
        );
    }

    #[test]
    fn archived_room_is_immutable() {
        let mut room = room();
        room.archive(Utc::now()).expect("archiviabile");
        let error = room
            .apply(
                RoomChanges {
                    name: Some("Nuovo nome".into()),
                    ..Default::default()
                },
                Utc::now(),
            )
            .expect_err("deve rifiutare");
        assert!(matches!(error, DomainError::Invariant(_)));
        assert!(room.archive(Utc::now()).is_err());
    }
}
