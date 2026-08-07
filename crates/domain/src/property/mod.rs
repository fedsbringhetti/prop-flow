mod aggregate;
mod room;

pub use aggregate::{
    EnergyClass, Patch, Property, PropertyChanges, PropertyParts, PropertyStatus, PropertyType,
};
pub use room::{Room, RoomChanges, RoomParts, RoomStatus};
