//! Letture che attraversano più moduli insieme.
//!
//! Qui non ci sono aggregati: ci sono **proiezioni**. Un aggregato ha invarianti
//! da difendere e un ciclo di vita; una proiezione è una fotografia che qualcuno
//! guarda e nessuno modifica, e metterla accanto agli aggregati farebbe cercare a
//! chi legge un costruttore che applica le regole — che qui non esiste e non deve
//! esistere.
//!
//! È la stessa ragione per cui `PropertyResident` sta accanto all'immobile e non
//! è un aggregato: la fonte è una vista o una funzione del database, e il dominio
//! ne trasporta il risultato senza ricalcolarlo.

mod dashboard;

pub use dashboard::OwnerDashboard;
