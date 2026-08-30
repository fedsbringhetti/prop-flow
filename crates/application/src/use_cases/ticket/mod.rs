//! Segnalazioni: apertura, thread, transizioni, costi.
//!
//! `OpenTicket`, `ListTickets`, `GetTicket`, `PostTicketMessage` e
//! `ListTicketMessages` usano `ensure_member` e non `ensure_staff`: è il modulo in
//! cui l'inquilino prende la parola, e *quali* segnalazioni siano sue lo decide
//! `app.can_access_ticket` attraverso le policy, non un ramo applicativo sul
//! ruolo.
//!
//! Le transizioni sono invece dello staff, con due eccezioni motivate: la
//! riapertura, che spetta a chi ha segnalato, e l'annullamento, che chi ha
//! segnalato può fare finché nessuno ci ha messo mano.
//!
//! Il perimetro della fetta è in `docs/09-stato-e-roadmap.md`.

mod costs;
mod open;
mod read;
mod thread;
mod transition;

pub use costs::{UpdateTicketCosts, UpdateTicketCostsCommand};
pub use open::{OpenTicket, OpenTicketCommand};
pub use read::{GetTicket, ListTickets, ListTicketsQuery, TicketDetail};
pub use thread::{
    ListTicketMessages, ListTicketMessagesCommand, PostTicketMessage, PostTicketMessageCommand,
};
pub use transition::{
    AssignTicket, AssignTicketCommand, CancelTicket, CancelTicketCommand, CloseTicket,
    CloseTicketCommand, ReopenTicket, ReopenTicketCommand, StartTicketWork, StartTicketWorkCommand,
    SuspendTicket, SuspendTicketCommand, TriageTicket, TriageTicketCommand,
};
