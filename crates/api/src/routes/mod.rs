use serde::{Deserialize, Deserializer};

pub mod auth;
pub mod health;
pub mod properties;
pub mod rooms;

/// Distingue "campo assente" da "campo presente e null".
///
/// Con `Option<T>` semplice le due cose sono indistinguibili, e un PATCH non
/// potrebbe mai cancellare un valore. Vive qui e non nel modulo di una singola
/// risorsa perché è la semantica del PATCH di tutta l'API.
pub(crate) fn explicit_null<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}
