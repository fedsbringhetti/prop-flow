//! Elaborazione automatica: il registro dei job affidati a una macchina.
//!
//! È il primo modulo del progetto in cui **una macchina scrive nel database**, e
//! quasi tutto ciò che segue discende da quella frase.
//!
//! ```text
//! AiJob:  queued ──start──► running ──succeed──► succeeded
//!                              └─────fail─────► failed
//!
//!         cancelled: esiste nell'enum del database e nessuno lo scrive
//! ```
//!
//! **Il fornitore non è scelto.** La porta `OcrProvider` vive in
//! `application::ports`, l'unico adattatore è deterministico, e la ragione sta in
//! `docs/09-stato-e-roadmap.md`: mandare l'immagine di una bolletta a un terzo è
//! una decisione di trattamento dei dati, che va presa da chi risponde del
//! trattamento e non dedotta mentre si scrive un adattatore. Ne segue che questa
//! fetta **non produce valore per un utente finale**: quello che compra è il
//! percorso — coda, worker, job, stato, revisione — che è indipendente dal
//! fornitore ed è dove stanno le trappole.
//!
//! **Il confine fra le due colonne JSONB** è la cosa più facile da sbagliare:
//!
//! | | |
//! |---|---|
//! | `utility_bills.ocr_extracted` | i **valori** proposti. Sono dati della bolletta, stanno sulla bolletta, ed ereditano la sua RLS — che comprende l'inquilino imputato |
//! | `ai_jobs.output` | la sola **forma**: quali campi con quale confidenza, senza i valori. È ciò che `ai_jobs_output_ix` permette di interrogare per misurare l'accuratezza |
//!
//! **L'OCR non scrive i campi della bolletta.** `supplier_name`, `total_amount` e
//! gli altri restano quelli che ha digitato la persona, finché una persona non li
//! cambia: lo schema tiene separato ciò che ha letto la macchina
//! (`ocr_extracted`, `ocr_confidence`) da ciò che ha confermato una persona
//! (`validated_by`, `validated_at`) **proprio perché l'accuratezza si possa
//! misurare**, e riempire le une con i valori delle altre distrugge quel dato
//! prima di raccoglierlo.

mod extraction;
mod job;
mod value_objects;

pub use extraction::{BillField, ExtractedField, ExtractedValue, OcrReading};
pub use job::{
    AiCapability, AiJob, AiJobErrorCode, AiJobFailure, AiJobParts, AiJobStatus, AiTask, NewAiJob,
};
pub use value_objects::{AiCost, Confidence, ProviderUsage};
