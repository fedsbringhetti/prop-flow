# 09 — Stato e roadmap

Stato al **5 agosto 2026**.

## Implementato

### Layer dati — completo per l'MVP

Le 16 migration coprono l'intero perimetro funzionale dell'MVP, non solo la parte
già esposta dall'API: 23 tabelle di business più `audit_log` partizionata, 35 enum,
4 domini, 169 indici, 59 policy RLS, 49 trigger, 4 viste, 25 funzioni.

Coperti a schema: utenti e organizzazioni, immobili e stanze, contratti e parti
contrattuali, documenti con allegato polimorfico, segnalazioni con thread e storico,
bollette con esito OCR, addebiti, pagamenti polimorfici, spese condivise con quote e
regolamenti, registro dei job AI, outbox degli eventi, audit con redazione.

### Backend — fondamento e una fetta verticale

| Componente | Stato |
|---|---|
| Workspace a 6 crate, vincoli verificati in CI | completo |
| Dominio: value object, identità, immobili, eventi, porte | completo per la fetta |
| Unit of Work con contesto RLS, due pool | completo |
| Repository: utenti, organizzazioni, membership, sessioni, immobili | completi |
| Outbox | completo |
| Argon2id, EdDSA, refresh opachi con rotazione | completo |
| Telemetria: log strutturati, OTLP | completo |
| Bordo HTTP: Problem Details, estrattori, OpenAPI | completo |
| Registrazione, login, refresh, logout | completo |
| CRUD immobili con PATCH parziale e archiviazione | completo |
| Migratore, dispatcher dell'outbox | completi |
| Immagine Docker, stack compose | completi |
| Pipeline CI | completa |

54 test: 28 unitari di dominio, 7 applicativi, 4 di infrastruttura, 5 di
integrazione sulla persistenza, 10 di contratto HTTP.

## Non implementato

### Moduli MVP che hanno lo schema ma non il codice

In ordine di dipendenza suggerito. Ognuno segue il pattern già stabilito: dominio →
porta → use case → repository → endpoint → test.

1. **Stanze** (`rooms`) — deliberatamente omesse dalla prima fetta per non
   introdurre codice senza endpoint. Parte dell'aggregato immobile.
2. **Contratti** (`leases`, `lease_parties`) — sblocca la visibilità
   dell'inquilino: oggi `can_access_property` per un inquilino passa dai contratti,
   che non si possono ancora creare dall'API. È il modulo con il maggior effetto
   sul resto.
3. **Inviti** (`org_invitations`) — onboarding dell'inquilino.
4. **Documenti** (`documents`) — richiede l'implementazione di `ObjectStorage`, già
   dichiarata come porta.
5. **Segnalazioni** (`tickets`) — thread, storico, assegnazione.
6. **Bollette e addebiti** (`utility_bills`, `charges`) — con la vista
   `v_charge_balances` già disponibile.
7. **Pagamenti** (`payments`) — provider `manual` per l'MVP.
8. **Spese co-living** (`expense_groups`, `expenses`, `expense_shares`,
   `settlements`) — la vista dei saldi esiste già.
9. **AI: OCR e triage** (`ai_jobs`) — i trait dei provider vanno definiti allora,
   verificando la reference del fornitore scelto in quel momento.
10. **Dashboard** — `v_owner_dashboard` esiste, l'endpoint no.

### Trasversali mancanti

| Ambito | Cosa manca |
|---|---|
| Sicurezza | rate limiting per IP; verifica email bloccante; MFA (colonna presente); CORS non montato; `kid` nei token per la rotazione delle chiavi |
| Operazioni | manifest Kubernetes; pubblicazione dell'immagine; backup e restore; metriche; alerting; cron per le partizioni di audit |
| Qualità | gate sulle licenze delle dipendenze; test di carico; benchmark |
| Prodotto | client mobile (Flutter o React Native), fuori perimetro backend |

## Debiti tecnici noti

Nessuno di questi blocca lo sviluppo; sono scelte consapevoli da rivedere quando
cambia il contesto.

| Debito | Dettaglio | Quando affrontarlo |
|---|---|---|
| Query non verificate a compile time | [ADR-006](08-decisioni.md#adr-006). La copertura dipende dai test di integrazione | Se compaiono errori SQL che i test non intercettano |
| Filtri di elenco con `($1 IS NULL OR col = $1)` | Il pattern può impedire al planner di usare l'indice sul filtro | Quando gli immobili per organizzazione passano l'ordine delle migliaia |
| `Cargo.lock` dipende dai crate presenti | Rende scomodo dividere il workspace fra branch: su un branch privo di `crates/api` un build `--locked` fallisce | Non si ripresenta se il workspace non viene più diviso |
| Chiamata di risoluzione del ruolo per richiesta con `X-Org-Id` | Una query in più quando il client cambia organizzazione | Se il cambio organizzazione diventa frequente |
| Nessuna cache | Ogni lettura va al database | Con dati reali sul profilo di carico |
| `audit_log` senza rotazione automatica | Le partizioni si creano con un cron da configurare | Prima del primo deploy in produzione |

## Prossimo passo suggerito

**Contratti** (punto 2). È il modulo che sblocca la visibilità dell'inquilino e
quindi la natura bidirezionale del prodotto: senza contratti creabili dall'API, il
ruolo `tenant` esiste nello schema ma non può fare nulla. Porta con sé stanze e
inviti, e apre la strada ad addebiti e bollette.

Le invarianti più delicate sono già nel database (vincoli di esclusione sulle
sovrapposizioni, coerenza fra tipo di contratto e stanza), quindi il lavoro è
soprattutto di dominio e di bordo.
