# 09 — Stato e roadmap

Stato al **5 agosto 2026**.

## Implementato

### Layer dati — completo per l'MVP

Le 18 migration coprono l'intero perimetro funzionale dell'MVP, non solo la parte
già esposta dall'API: 23 tabelle di business più `audit_log` partizionata, 35 enum,
6 domini, 169 indici, 64 policy RLS, 49 trigger, 4 viste, 28 funzioni.

Coperti a schema: utenti e organizzazioni, immobili e stanze, contratti e parti
contrattuali, documenti con allegato polimorfico, segnalazioni con thread e storico,
bollette con esito OCR, addebiti, pagamenti polimorfici, spese condivise con quote e
regolamenti, registro dei job AI, outbox degli eventi, audit con redazione.

### Backend — fondamento e una fetta verticale

| Componente | Stato |
|---|---|
| Workspace a 6 crate, vincoli verificati in CI | completo |
| Dominio: value object, identità, immobili, stanze, contratti, inviti, eventi, porte | completo per la fetta |
| Unit of Work con contesto RLS, due pool | completo |
| Repository: utenti, organizzazioni, membership, sessioni, immobili, stanze, contratti, inviti | completi |
| Outbox | completo |
| Argon2id, EdDSA, refresh opachi con rotazione | completo |
| Telemetria: log strutturati, OTLP | completo |
| Bordo HTTP: Problem Details, estrattori, OpenAPI | completo |
| Registrazione, login, refresh, logout | completo |
| CRUD immobili con PATCH parziale e archiviazione | completo |
| CRUD stanze annidate sull'immobile | completo |
| Contratti: bozza, parti, attivazione, disdetta, stati derivati | completo |
| Inviti: emissione, revoca, accettazione senza pool privilegiato | completo |
| Migratore, dispatcher dell'outbox | completi |
| Immagine Docker, stack compose | completi |
| Pipeline CI | completa |

110 test: 63 unitari di dominio, 8 applicativi, 5 di infrastruttura, 11 di
integrazione sulla persistenza, 23 di contratto HTTP.

## Non implementato

### Moduli MVP che hanno lo schema ma non il codice

In ordine di dipendenza suggerito. Ognuno segue il pattern già stabilito: dominio →
porta → use case → repository → endpoint → test.

1. **Documenti** (`documents`) — richiede l'implementazione di `ObjectStorage`, già
   dichiarata come porta.
2. **Segnalazioni** (`tickets`) — thread, storico, assegnazione.
3. **Bollette e addebiti** (`utility_bills`, `charges`) — con la vista
   `v_charge_balances` già disponibile.
4. **Pagamenti** (`payments`) — provider `manual` per l'MVP.
5. **Spese co-living** (`expense_groups`, `expenses`, `expense_shares`,
   `settlements`) — la vista dei saldi esiste già.
6. **AI: OCR e triage** (`ai_jobs`) — i trait dei provider vanno definiti allora,
   verificando la reference del fornitore scelto in quel momento.
7. **Dashboard** — `v_owner_dashboard` esiste, l'endpoint no.

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

**Documenti** (punto 1). L'anello bidirezionale è chiuso: il proprietario crea
immobile, stanze e contratto, invita l'inquilino, e l'inquilino accetta e vede il
proprio contratto e il proprio immobile — coperto da un test di contratto HTTP
end-to-end. Da qui i moduli successivi non hanno più dipendenze bloccanti fra loro:
i documenti vengono per primi perché richiedono l'unica porta già dichiarata e non
implementata, `ObjectStorage`, e la stessa forma serve poi alle bollette.

Manca invece l'**invio delle notifiche**: la porta `Notifier` è dichiarata e senza
adattatore, quindi il token di invito torna nella risposta HTTP e lo consegna il
proprietario. Va implementata prima di considerare l'onboarding pronto per utenti
reali.
