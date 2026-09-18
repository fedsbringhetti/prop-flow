use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::error::{DomainError, DomainResult, Invariant};
use crate::shared::{Address, Currency, Money, OrgId, PropertyId, UserId, text};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyType {
    Apartment,
    House,
    Studio,
    Villa,
    Office,
    Retail,
    Garage,
    Land,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyStatus {
    Draft,
    Available,
    PartiallyRented,
    Rented,
    Maintenance,
    Archived,
}

impl PropertyStatus {
    /// Se questo valore **discende dai contratti** invece di essere una decisione.
    ///
    /// `rented` e `partially_rented` non li scrive nessuno: si calcolano dalle
    /// stanze occupate e dai contratti sull'immobile intero, come `expiring` ed
    /// `expired` si calcolano dalle date (ADR-019). Restano nell'enum perché una
    /// riga scritta a mano potrebbe contenerli, e un valore sconosciuto in lettura
    /// deve dare errore invece di ricadere su un default plausibile.
    #[must_use]
    pub const fn is_derived(self) -> bool {
        matches!(self, Self::Rented | Self::PartiallyRented)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyClass {
    A4,
    A3,
    A2,
    A1,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
}

/// Semantica di una modifica parziale (PATCH).
///
/// `None` = campo non menzionato dal chiamante, resta com'è.
/// `Some(None)` = campo esplicitamente azzerato.
/// `Some(Some(v))` = campo impostato a `v`.
///
/// Senza questa distinzione un PATCH non può cancellare un valore, e un
/// `Option<T>` semplice confonde "non inviato" con "metti a null".
pub type Patch<T> = Option<Option<T>>;

/// Insieme di modifiche applicabili a un immobile.
#[derive(Debug, Clone, Default)]
pub struct PropertyChanges {
    pub label: Option<String>,
    pub code: Patch<String>,
    pub property_type: Option<PropertyType>,
    pub status: Option<PropertyStatus>,
    pub is_coliving: Option<bool>,
    pub address: Option<Address>,
    pub size_sqm: Patch<Decimal>,
    pub rooms_count: Patch<i16>,
    pub bathrooms_count: Patch<i16>,
    pub floor: Patch<i16>,
    pub has_elevator: Patch<bool>,
    pub energy_class: Patch<EnergyClass>,
    pub notes: Patch<String>,
}

impl PropertyChanges {
    /// Nomi dei campi menzionati dal chiamante, nell'ordine di dichiarazione.
    /// Finiscono nell'evento `PropertyUpdated` al posto dei valori, così chi
    /// consuma sa cosa invalidare senza ricevere il contenuto dell'immobile.
    #[must_use]
    pub fn changed_field_names(&self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        if self.label.is_some() {
            fields.push("label");
        }
        if self.code.is_some() {
            fields.push("code");
        }
        if self.property_type.is_some() {
            fields.push("property_type");
        }
        if self.status.is_some() {
            fields.push("status");
        }
        if self.is_coliving.is_some() {
            fields.push("is_coliving");
        }
        if self.address.is_some() {
            fields.push("address");
        }
        if self.size_sqm.is_some() {
            fields.push("size_sqm");
        }
        if self.rooms_count.is_some() {
            fields.push("rooms_count");
        }
        if self.bathrooms_count.is_some() {
            fields.push("bathrooms_count");
        }
        if self.floor.is_some() {
            fields.push("floor");
        }
        if self.has_elevator.is_some() {
            fields.push("has_elevator");
        }
        if self.energy_class.is_some() {
            fields.push("energy_class");
        }
        if self.notes.is_some() {
            fields.push("notes");
        }
        fields
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.label.is_none()
            && self.code.is_none()
            && self.property_type.is_none()
            && self.status.is_none()
            && self.is_coliving.is_none()
            && self.address.is_none()
            && self.size_sqm.is_none()
            && self.rooms_count.is_none()
            && self.bathrooms_count.is_none()
            && self.floor.is_none()
            && self.has_elevator.is_none()
            && self.energy_class.is_none()
            && self.notes.is_none()
    }
}

/// Immobile: aggregato radice del patrimonio.
#[derive(Debug, Clone)]
pub struct Property {
    id: PropertyId,
    org_id: OrgId,
    owner_id: UserId,
    code: Option<String>,
    label: String,
    property_type: PropertyType,
    status: PropertyStatus,
    is_coliving: bool,
    address: Address,
    size_sqm: Option<Decimal>,
    rooms_count: Option<i16>,
    bathrooms_count: Option<i16>,
    floor: Option<i16>,
    has_elevator: Option<bool>,
    energy_class: Option<EnergyClass>,
    purchase_price: Option<Money>,
    current_value: Option<Money>,
    currency: Currency,
    notes: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
}

impl Property {
    /// Un immobile nasce in `Draft`: diventa `Available` quando il proprietario
    /// lo pubblica, così una scheda incompleta non compare fra quelle affittabili.
    pub fn create(
        id: PropertyId,
        org_id: OrgId,
        owner_id: UserId,
        label: &str,
        property_type: PropertyType,
        address: Address,
        now: DateTime<Utc>,
    ) -> DomainResult<Self> {
        Ok(Self {
            id,
            org_id,
            owner_id,
            code: None,
            label: text::required("label", label, 200)?,
            property_type,
            status: PropertyStatus::Draft,
            is_coliving: false,
            address,
            size_sqm: None,
            rooms_count: None,
            bathrooms_count: None,
            floor: None,
            has_elevator: None,
            energy_class: None,
            purchase_price: None,
            current_value: None,
            currency: Currency::EUR,
            notes: None,
            created_at: now,
            updated_at: now,
            archived_at: None,
        })
    }

    /// Applica una modifica parziale. Restituisce `false` se non c'era nulla da
    /// cambiare, così il chiamante può evitare una scrittura e un evento inutili.
    pub fn apply(&mut self, changes: PropertyChanges, now: DateTime<Utc>) -> DomainResult<bool> {
        self.ensure_mutable()?;
        if changes.is_empty() {
            return Ok(false);
        }

        if let Some(label) = changes.label {
            self.label = text::required("label", &label, 200)?;
        }
        if let Some(code) = changes.code {
            self.code = text::optional("code", code, 32)?;
        }
        if let Some(property_type) = changes.property_type {
            self.property_type = property_type;
        }
        if let Some(status) = changes.status {
            self.set_status(status)?;
        }
        if let Some(is_coliving) = changes.is_coliving {
            self.is_coliving = is_coliving;
        }
        if let Some(address) = changes.address {
            self.address = address;
        }
        if let Some(size) = changes.size_sqm {
            self.size_sqm = size.map(validate_size).transpose()?;
        }
        if let Some(rooms) = changes.rooms_count {
            self.rooms_count = rooms.map(|v| non_negative("rooms_count", v)).transpose()?;
        }
        if let Some(bathrooms) = changes.bathrooms_count {
            self.bathrooms_count = bathrooms
                .map(|v| non_negative("bathrooms_count", v))
                .transpose()?;
        }
        if let Some(floor) = changes.floor {
            self.floor = floor;
        }
        if let Some(elevator) = changes.has_elevator {
            self.has_elevator = elevator;
        }
        if let Some(energy_class) = changes.energy_class {
            self.energy_class = energy_class;
        }
        if let Some(notes) = changes.notes {
            self.notes = text::optional("notes", notes, 4000)?;
        }

        self.updated_at = now;
        Ok(true)
    }

    /// Archivia l'immobile (soft delete a livello di dominio).
    pub fn archive(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        self.ensure_mutable()?;
        self.status = PropertyStatus::Archived;
        self.archived_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    /// Un immobile archiviato è storia: non si modifica più. Riaprirlo sarebbe una
    /// transizione di dominio a sé, non un `PATCH`.
    fn ensure_mutable(&self) -> DomainResult<()> {
        if matches!(self.status, PropertyStatus::Archived) {
            return Err(DomainError::Invariant(Invariant::PropertyArchived));
        }
        Ok(())
    }

    fn set_status(&mut self, status: PropertyStatus) -> DomainResult<()> {
        if matches!(status, PropertyStatus::Archived) {
            return Err(DomainError::Invariant(
                Invariant::PropertyArchiveRequiresDedicatedOperation,
            ));
        }
        self.status = status;
        Ok(())
    }

    #[must_use]
    pub const fn id(&self) -> PropertyId {
        self.id
    }
    #[must_use]
    pub const fn org_id(&self) -> OrgId {
        self.org_id
    }
    #[must_use]
    pub const fn owner_id(&self) -> UserId {
        self.owner_id
    }
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
    #[must_use]
    pub const fn property_type(&self) -> PropertyType {
        self.property_type
    }
    #[must_use]
    pub const fn status(&self) -> PropertyStatus {
        self.status
    }
    #[must_use]
    pub const fn is_coliving(&self) -> bool {
        self.is_coliving
    }
    #[must_use]
    pub const fn address(&self) -> &Address {
        &self.address
    }
    #[must_use]
    pub const fn size_sqm(&self) -> Option<Decimal> {
        self.size_sqm
    }
    #[must_use]
    pub const fn rooms_count(&self) -> Option<i16> {
        self.rooms_count
    }
    #[must_use]
    pub const fn bathrooms_count(&self) -> Option<i16> {
        self.bathrooms_count
    }
    #[must_use]
    pub const fn floor(&self) -> Option<i16> {
        self.floor
    }
    #[must_use]
    pub const fn has_elevator(&self) -> Option<bool> {
        self.has_elevator
    }
    #[must_use]
    pub const fn energy_class(&self) -> Option<EnergyClass> {
        self.energy_class
    }
    #[must_use]
    pub const fn purchase_price(&self) -> Option<Money> {
        self.purchase_price
    }
    #[must_use]
    pub const fn current_value(&self) -> Option<Money> {
        self.current_value
    }
    #[must_use]
    pub const fn currency(&self) -> Currency {
        self.currency
    }
    #[must_use]
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
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

    /// Ricostruzione da persistenza. Non riesegue le invarianti: i dati sono già
    /// stati validati alla scrittura, e un aggregato salvato deve poter essere
    /// riletto anche se le regole sono cambiate da allora.
    #[must_use]
    pub fn from_persistence(parts: PropertyParts) -> Self {
        let PropertyParts {
            id,
            org_id,
            owner_id,
            code,
            label,
            property_type,
            status,
            is_coliving,
            address,
            size_sqm,
            rooms_count,
            bathrooms_count,
            floor,
            has_elevator,
            energy_class,
            purchase_price,
            current_value,
            currency,
            notes,
            created_at,
            updated_at,
            archived_at,
        } = parts;
        Self {
            id,
            org_id,
            owner_id,
            code,
            label,
            property_type,
            status,
            is_coliving,
            address,
            size_sqm,
            rooms_count,
            bathrooms_count,
            floor,
            has_elevator,
            energy_class,
            purchase_price,
            current_value,
            currency,
            notes,
            created_at,
            updated_at,
            archived_at,
        }
    }
}

/// Dati grezzi di un immobile letti dalla persistenza.
///
/// Una struct e non venti argomenti posizionali: con venti `Option` dello stesso
/// tipo, scambiarne due compila senza errori.
#[derive(Debug, Clone)]
pub struct PropertyParts {
    pub id: PropertyId,
    pub org_id: OrgId,
    pub owner_id: UserId,
    pub code: Option<String>,
    pub label: String,
    pub property_type: PropertyType,
    pub status: PropertyStatus,
    pub is_coliving: bool,
    pub address: Address,
    pub size_sqm: Option<Decimal>,
    pub rooms_count: Option<i16>,
    pub bathrooms_count: Option<i16>,
    pub floor: Option<i16>,
    pub has_elevator: Option<bool>,
    pub energy_class: Option<EnergyClass>,
    pub purchase_price: Option<Money>,
    pub current_value: Option<Money>,
    pub currency: Currency,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

fn validate_size(value: Decimal) -> DomainResult<Decimal> {
    if value <= Decimal::ZERO {
        return Err(DomainError::validation("size_sqm", "must be positive"));
    }
    if value.scale() > 2 {
        return Err(DomainError::validation(
            "size_sqm",
            "allows at most two decimal places",
        ));
    }
    Ok(value)
}

fn non_negative(field: &'static str, value: i16) -> DomainResult<i16> {
    if value < 0 {
        return Err(DomainError::validation(field, "cannot be negative"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::CountryCode;
    use std::str::FromStr;

    fn address() -> Address {
        Address::new(
            "Via Roma 1",
            None,
            "Milano",
            Some("MI".into()),
            None,
            CountryCode::IT,
        )
        .expect("indirizzo valido")
    }

    fn property() -> Property {
        Property::create(
            PropertyId::generate(),
            OrgId::generate(),
            UserId::generate(),
            "Bilocale centro",
            PropertyType::Apartment,
            address(),
            Utc::now(),
        )
        .expect("immobile valido")
    }

    #[test]
    fn new_property_starts_as_draft() {
        assert_eq!(property().status(), PropertyStatus::Draft);
    }

    #[test]
    fn changed_field_names_lists_only_mentioned_fields() {
        let changes = PropertyChanges {
            label: Some("nuovo".into()),
            notes: Some(None),
            ..Default::default()
        };
        assert_eq!(changes.changed_field_names(), vec!["label", "notes"]);
        assert!(PropertyChanges::default().changed_field_names().is_empty());
    }

    #[test]
    fn empty_patch_reports_no_change() {
        let mut property = property();
        let changed = property
            .apply(PropertyChanges::default(), Utc::now())
            .expect("nessun errore");
        assert!(!changed);
    }

    #[test]
    fn patch_can_clear_a_nullable_field() {
        let mut property = property();
        property
            .apply(
                PropertyChanges {
                    notes: Some(Some("da ristrutturare".into())),
                    ..Default::default()
                },
                Utc::now(),
            )
            .expect("valido");
        assert_eq!(property.notes(), Some("da ristrutturare"));

        property
            .apply(
                PropertyChanges {
                    notes: Some(None),
                    ..Default::default()
                },
                Utc::now(),
            )
            .expect("valido");
        assert_eq!(property.notes(), None);
    }

    #[test]
    fn archived_property_is_immutable() {
        let mut property = property();
        property.archive(Utc::now()).expect("archiviabile");
        let error = property
            .apply(
                PropertyChanges {
                    label: Some("nuovo nome".into()),
                    ..Default::default()
                },
                Utc::now(),
            )
            .expect_err("deve rifiutare");
        assert!(matches!(error, DomainError::Invariant(_)));
        assert!(property.archive(Utc::now()).is_err());
    }

    #[test]
    fn status_patch_cannot_archive() {
        let mut property = property();
        assert!(
            property
                .apply(
                    PropertyChanges {
                        status: Some(PropertyStatus::Archived),
                        ..Default::default()
                    },
                    Utc::now()
                )
                .is_err()
        );
    }

    #[test]
    fn size_must_be_positive_with_at_most_two_decimals() {
        let mut property = property();
        let bad = Decimal::from_str("-1").expect("decimale");
        assert!(
            property
                .apply(
                    PropertyChanges {
                        size_sqm: Some(Some(bad)),
                        ..Default::default()
                    },
                    Utc::now()
                )
                .is_err()
        );
        let too_precise = Decimal::from_str("55.555").expect("decimale");
        assert!(
            property
                .apply(
                    PropertyChanges {
                        size_sqm: Some(Some(too_precise)),
                        ..Default::default()
                    },
                    Utc::now()
                )
                .is_err()
        );
    }
}
