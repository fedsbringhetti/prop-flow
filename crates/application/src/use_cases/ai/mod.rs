//! Elaborazione automatica: l'accodamento e il ciclo che consuma la coda.
//!
//! Due pezzi con due posture diverse, ed è la cosa da capire per prima:
//!
//! | | Chi lo esegue | Come parla al database |
//! |---|---|---|
//! | [`enqueue_bill_ocr`] | l'API, dentro la transazione del caricamento | `propflow_app`, sotto RLS, con il contesto del chiamante |
//! | [`ProcessAiJob`] | il worker, in un ciclo suo | `propflow_worker`, `BYPASSRLS`, nessun contesto |
//!
//! La seconda riga è la ragione per cui esistono `WorkerUnitOfWork` e
//! `AiJobQueue`: un ciclo che non serve nessuna richiesta non ha un contesto di
//! sicurezza da installare, e `begin_privileged()` **non** è la risposta — la
//! regola 20 di `CLAUDE.md` la riserva ai flussi che avvengono prima
//! dell'autenticazione, e allargarla per comodità è precisamente ciò che quella
//! regola esiste per impedire.

mod enqueue;
mod process;
mod status;

pub(crate) use enqueue::enqueue_bill_ocr;
pub use process::{ProcessAiJob, Processed};
pub use status::{BillOcrStatus, GetBillOcrStatus};
