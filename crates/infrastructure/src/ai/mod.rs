//! Adattatori per l'elaborazione automatica.
//!
//! Oggi ce n'è uno solo ed è **deterministico**, per la stessa ragione per cui
//! `LoggingEventBus` è l'unica implementazione dell'event bus: il percorso deve
//! essere osservabile e verificabile prima che esista un fornitore vero. La
//! scelta del fornitore è rimandata di proposito e non per inerzia — è una
//! decisione sul trattamento dei dati personali, non una decisione tecnica, e la
//! motivazione completa sta in `docs/09-stato-e-roadmap.md` e in ADR-037.
//!
//! **Il momento di rileggere la porta è il primo adattatore vero.** È scritto qui
//! perché chi lo scriverà lo trovi.

mod deterministic;

pub use deterministic::DeterministicOcrProvider;
