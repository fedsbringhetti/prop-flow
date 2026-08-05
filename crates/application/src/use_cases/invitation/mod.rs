//! Inviti: emissione, revoca e accettazione.
//!
//! L'accettazione è il punto in cui un utente qualunque diventa inquilino di
//! un'organizzazione, e avviene **senza** pool privilegiato: vedi
//! [`AcceptInvitation`] e ADR-020.

mod accept;
mod invite_tenant;
mod revoke;

pub use accept::{AcceptInvitation, AcceptInvitationCommand, AcceptedInvitation};
pub use invite_tenant::{InviteTenant, InviteTenantCommand, IssuedInvitation};
pub use revoke::{RevokeInvitation, RevokeInvitationCommand};
