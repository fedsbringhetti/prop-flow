//! Segnalazioni di manutenzione: apertura, thread, ciclo di vita.
//!
//! È la prima capacità che dà voce all'inquilino: fino a qui poteva solo leggere.
//! Il ciclo è `open → triaged → assigned → in_progress → closed`, con
//! `waiting_tenant` e `waiting_owner` come sospensioni della lavorazione e la
//! **riapertura** in mano a chi ha segnalato — è ciò che rende la segnalazione un
//! rapporto invece di una casella postale.
//!
//! Il perimetro di questa fetta è dichiarato in `docs/09-stato-e-roadmap.md`.
//! Restano fuori il triage AI, la ricerca full-text, gli SLA, la programmazione e
//! gli allegati sui singoli messaggi: le colonne esistono nello schema e restano
//! ai loro default.

mod aggregate;
mod thread;
mod value_objects;

pub use aggregate::{
    NewTicket, Ticket, TicketActor, TicketCategory, TicketCorrection, TicketCosts, TicketParts,
    TicketPriority, TicketStatus, WaitingOn,
};
pub use thread::{NewTicketMessage, TicketMessage, TicketStatusChange};
pub use value_objects::{MessageBody, TicketDescription, TicketReference, TicketTitle};
