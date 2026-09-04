# Documentazione PropFlow

Mappa della documentazione di progetto. `CLAUDE.md` nella radice contiene le regole
operative in forma sintetica; questi documenti contengono il contesto e le
motivazioni.

| Documento | Contenuto |
|---|---|
| [01 — Prodotto](01-prodotto.md) | Visione, problema, utenti, perimetro dell'MVP, roadmap V2/V3, glossario |
| [02 — Architettura](02-architettura.md) | Struttura a crate, regola delle dipendenze, Unit of Work e contesto RLS, outbox, come si aggiunge una funzionalità |
| [03 — Modello dati](03-modello-dati.md) | Diagramma ER, convenzioni dello schema, Row Level Security, ruoli, audit, migrazioni |
| [04 — Sicurezza](04-sicurezza.md) | Autenticazione, token, password, isolamento multi-organizzazione, PII, logging, modello di minaccia |
| [05 — API](05-api.md) | Convenzioni HTTP, modello di errore, paginazione, semantica del PATCH, endpoint |
| [06 — Sviluppo](06-sviluppo.md) | Requisiti, avvio, strategia di test, ricette operative |
| [07 — Operazioni](07-operazioni.md) | Configurazione, deploy, migrazioni in produzione, osservabilità, CI |
| [08 — Decisioni](08-decisioni.md) | Registro delle decisioni architetturali con le motivazioni |
| [09 — Stato e roadmap](09-stato-e-roadmap.md) | Cosa è implementato, cosa manca, debiti tecnici noti, **il piano in corso** |
| [10 — Rilasci](10-rilasci.md) | Versionamento, changelog, flusso di rilascio, hotfix, rollback |
| [11 — Piani conclusi](11-piani-conclusi.md) | Archivio dei piani già eseguiti, con i loro esiti. Append-only |

## Come leggere questa documentazione

Se stai iniziando: [01](01-prodotto.md) → [02](02-architettura.md) →
[03](03-modello-dati.md). Sono i tre che danno il quadro.

Se devi mettere le mani su qualcosa: la tabella in `CLAUDE.md` §1 dice quali
documenti servono per quale compito.

Se una scelta ti sembra sbagliata: cercala in [08 — Decisioni](08-decisioni.md)
prima di cambiarla. Diverse decisioni derivano da un difetto osservato, e il
documento riporta il difetto.

Se la domanda è «perché questo modulo è fatto così»: l'esito del piano che lo ha
prodotto, in [11 — Piani conclusi](11-piani-conclusi.md). `09` e `11` si dividono per
**ciclo di vita**: il primo si riscrive a ogni fetta, il secondo non si riscrive mai.

## Convenzione

La documentazione descrive lo **stato attuale** e le sue motivazioni, non le
intenzioni. Ciò che non è ancora costruito sta in
[09 — Stato e roadmap](09-stato-e-roadmap.md) e non nei documenti descrittivi,
dove sarebbe indistinguibile da ciò che esiste.
