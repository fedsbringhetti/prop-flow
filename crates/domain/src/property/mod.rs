mod aggregate;
mod resident;
mod room;

pub use aggregate::{
    EnergyClass, Patch, Property, PropertyChanges, PropertyParts, PropertyStatus, PropertyType,
};
pub use resident::PropertyResident;
pub use room::{Room, RoomChanges, RoomParts, RoomStatus};
