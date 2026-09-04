//! Letture che attraversano più moduli insieme.
//!
//! Nessun comando qui dentro, e non è una fase: una proiezione non ha nulla da
//! decidere. Se un giorno comparisse una scrittura in questo modulo sarebbe il
//! segnale che il numero mostrato è diventato un numero memorizzato — cioè che
//! esistono due definizioni dello stesso valore, che è il difetto che
//! [ADR-032](../../../../docs/08-decisioni.md) esiste per evitare.

mod dashboard;

pub use dashboard::GetOwnerDashboard;
