# 09 — Stato e roadmap

Stato al **18 agosto 2026**.

## Come si usa questo documento

Qui atterrano i piani **prima** di essere implementati, con il dettaglio con cui
sono stati pensati: la scomposizione in passi, le decisioni con il perché, le
alternative scartate e le trappole trovate. Un piano che resta nella conversazione
in cui è nato non esiste per la sessione successiva, che riparte da zero.

Il documento può essere lungo, ed è previsto che lo sia: accorciarlo per ordine
toglie esattamente ciò per cui esiste. Ciò che è rinviato va segnato come rinviato,
**con la ragione** — senza, si rilegge come lavoro pendente e qualcuno lo prende in
mano credendo che sia solo rimasto indietro.

La regola sta anche in `CLAUDE.md` §3.

## Implementato

### Layer dati — completo per l'MVP

Le 19 migration coprono l'intero perimetro funzionale dell'MVP, non solo la parte
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

125 test: 65 unitari di dominio, 11 applicativi, 5 di infrastruttura, 4 unitari
del bordo HTTP, 13 di integrazione sulla persistenza, 24 di contratto HTTP, 3 di
guardia sulla documentazione della configurazione.

## Non implementato

### Moduli MVP che hanno lo schema ma non il codice

In ordine di dipendenza suggerito. Ognuno segue il pattern già stabilito: dominio →
porta → use case → repository → endpoint → test.

1. **Documenti** (`documents`) — richiede l'implementazione di `ObjectStorage`, già
   dichiarata come porta. Il piano è scritto in fondo a questo documento:
   [Piano — Documenti, fetta 1](#piano-documenti-fetta-1).
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
| Operazioni | manifest Kubernetes; backup e restore; metriche; alerting; cron per le partizioni di audit |
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
| `auth.denied_by_policy` risponde `403` | Rinviato in [ADR-022](08-decisioni.md#adr-022). Se la RLS rifiuta una scrittura che il service layer aveva già autorizzato è un difetto nostro, non un errore del chiamante: `500` sarebbe più onesto. Fuori da quel cambiamento perché è una decisione sul comportamento, non sulla tassonomia | Quando si tocca il comportamento degli errori al bordo, o se il codice compare nei log |

## Prossimo passo suggerito

**Documenti** (punto 1). L'anello bidirezionale è chiuso: il proprietario crea
immobile, stanze e contratto, invita l'inquilino, e l'inquilino accetta e vede il
proprio contratto e il proprio immobile — coperto da un test di contratto HTTP
end-to-end. Da qui i moduli successivi non hanno più dipendenze bloccanti fra loro:
i documenti vengono per primi perché richiedono l'unica porta già dichiarata e non
implementata, `ObjectStorage`, e la stessa forma serve poi alle bollette.

Il piano della prima fetta è in fondo: [Piano — Documenti, fetta 1](#piano-documenti-fetta-1).

Manca invece l'**invio delle notifiche**: la porta `Notifier` è dichiarata e senza
adattatore, quindi il token di invito torna nella risposta HTTP e lo consegna il
proprietario. Va implementata prima di considerare l'onboarding pronto per utenti
reali.

---

<a id="piano-documenti-fetta-1"></a>

## Piano — Documenti, fetta 1

Pianificato il **18 agosto 2026**, non ancora implementato. Scritto qui prima di
scrivere codice, come chiede `CLAUDE.md` §3: quello che segue è la decomposizione,
non un riassunto.

### Obiettivo

Allegare file a immobili, stanze e contratti, e riscaricarli. Chiude la porta
`ObjectStorage`, dichiarata dal primo giorno e senza adattatore, e la stessa forma
servirà poi alle bollette.

**Dentro:** upload, elenco per owner, metadati, download, archiviazione.
**Fuori, e non per dimenticanza:**

- **il versionamento** (`version`, `supersedes_document_id`, `is_current`): le
  colonne esistono, ma [01 — Prodotto](01-prodotto.md) lo colloca in V2. Lo schema
  è avanti al perimetro, e vedere le colonne non è una ragione per riempirle;
- **l'OCR** (`extracted_data`, `status`): è il punto 6 della lista, e i trait dei
  provider vanno definiti allora. In questa fetta un documento resta `uploaded` e
  `extracted_data` resta `{}`;
- **avatar utente, ticket, bollette, spese**: `can_access_attachment` copre undici
  tipi di owner, ma qui se ne espongono tre — quelli i cui aggregati hanno già
  dominio, use case ed endpoint. Allargare dopo è additivo. Conseguenza da dire ad
  alta voce: `users.avatar_document_id` e `leases.contract_document_id` restano
  non popolati dall'API.

### Decisioni prese, con il perché

**Storage: MinIO nel compose, adattatore S3 con `rusty-s3` sopra `reqwest`.**
`rusty-s3` fa solo la firma SigV4 e non porta un client HTTP proprio; il presigning
è esattamente il suo scopo, ed è ciò che serve a `presigned_get_url`. In produzione
lo stesso adattatore punta a S3 cambiando endpoint. Scartato `aws-sdk-s3`: corretto
per costruzione ma decine di crate in più e un rischio concreto sul controllo `bans`
di cargo-deny, che fallisce sulle versioni duplicate. Scartato un adattatore su
filesystem: `presigned_get_url` non ha senso lì, e la porta — già decisa e motivata —
andrebbe riscritta per accomodare un ripiego.

**Upload multipart attraverso l'API, download pre-firmato.** L'asimmetria è
deliberata: `checksum_sha256` è `NOT NULL` nello schema e un checksum dichiarato dal
client non è un checksum, quindi il contenuto deve passare da noi almeno una volta.
In download il contenuto non ha bisogno di transitare, e un URL pre-firmato toglie
dall'API il traffico più pesante. Scartato il `PUT` pre-firmato in upload: scala
meglio ma rende MIME e checksum dichiarazioni del client, e impone uno stato
intermedio con oggetti orfani da ripulire.

**Cifratura a riposo lato bucket (SSE), non applicativa.** `is_encrypted` nasce
`true` e con SSE la colonna dice il vero al costo di una configurazione;
`encryption_key_ref` registra la modalità, non una chiave. Scartata la cifratura a
busta: è il livello che la colonna lascia immaginare, ma porta con sé gestione e
rotazione delle chiavi ed è un modulo a sé. **Limite da non nascondere:** con SSE la
chiave è del provider, quindi non protegge da un accesso amministrativo al bucket.

### Decomposizione

Ogni passo è eseguibile e verificabile da solo, nell'ordine di `docs/06` §Aggiungere
un caso d'uso.

1. **Dipendenze.** `axum` con la feature `multipart`, `rusty-s3` e `reqwest` in
   `[workspace.dependencies]`. Verificare l'API di `rusty-s3` sulla sua reference
   corrente invece di assumerla: è un crate piccolo e la firma dei metodi è cambiata
   fra versioni minori.
2. **Dominio.** Entità `Document` con i value object per MIME, dimensione e
   checksum, e `DocumentKind`/`AttachmentOwner` come enum di dominio. La riga nella
   macro `string_enum!` per i due enum del database (`docs/06` §Aggiungere un enum).
   Evento `DocumentUploaded` che trasporta identificatori e non il nome del file.
3. **Porta di persistenza.** `DocumentRepository` in `domain::ports`: `insert`,
   `find_by_id`, `list_by_owner`, `archive`. Nessun metodo che restituisca il
   contenuto: quello è `ObjectStorage`.
4. **Use case.** `upload_document`, `list_documents`, `download_document`,
   `archive_document` in `crates/application/src/use_cases/document/`, con la
   sequenza canonica: autorizza → costruisci → transazione → persisti → evento →
   commit. L'oggetto va scritto **prima** della transazione: se il commit fallisce
   resta un oggetto orfano, che è recuperabile; l'ordine inverso lascerebbe una riga
   che punta al nulla, che non lo è.
5. **Adattatore S3.** `S3ObjectStorage` in `crates/infrastructure/src/storage/`, con
   `put`, `get` e `presigned_get_url`. Chiave dell'oggetto derivata:
   `{org_id}/{document_id}`, mai il nome originale — vedi le trappole.
6. **Configurazione.** Sezione `[storage]` in `AppConfig` e in `config/default.toml`:
   endpoint, regione, bucket, credenziali dall'ambiente, TTL dell'URL pre-firmato.
   Documentarla in `docs/07-operazioni.md` **nello stesso commit**, altrimenti
   `crates/api/tests/docs_config_reference.rs` fa fallire la suite.
7. **Bordo HTTP.** `POST /api/v1/{owner_type}/{owner_id}/documents` (multipart),
   `GET .../documents`, `GET /api/v1/documents/{id}/content` che risponde `307` verso
   l'URL pre-firmato, `DELETE /api/v1/documents/{id}` che archivia. DTO e OpenAPI
   secondo `docs/05-api.md`.
8. **Limite del corpo per rotta.** Vedi le trappole: non è una riga di
   configurazione, è una modifica alla struttura del router.
9. **Test.** Unitari di dominio sulle validazioni; un doppio in memoria di
   `ObjectStorage` per i test di contratto; test di persistenza sul repository con
   database reale; un test che verifichi il rifiuto quando l'owner non è accessibile.
   Un test contro MinIO vero solo se la variabile d'ambiente c'è, con il marcatore
   `SALTATO` come gli altri.
10. **Documentazione.** Endpoint in `docs/05-api.md`, il modulo in questa roadmap
    spostato da "non implementato" a "implementato", e un ADR se una di queste
    decisioni viene cambiata strada facendo.

### Trappole

Sono le vie che sembrano corrette e non lo sono. Costano una giornata ciascuna se le
si scopre sbagliando.

**Il limite del corpo è globale e sta fuori da tutto.**
`RequestBodyLimitLayer::new(body_limit_bytes)` è montato sull'intero router in
`crates/api/src/lib.rs:60`, con default 1 MiB. Aggiungere un limite più largo sulla
rotta di upload **non funziona**: il layer esterno rifiuta per primo, e il sintomo è
un `413` che nessun limite per-rotta sembra spiegare. Alzare quello globale
funzionerebbe, ed è la trappola vera: indebolisce ogni endpoint per servirne uno. La
via corretta è montare il limite globale sul gruppo di rotte che non comprende
l'upload, e dare a quella il proprio.

**`documents_update` non lascia toccare una riga archiviata.** La policy ha
`deleted_at IS NULL` nella `USING`: archiviare funziona, **ripristinare no**, e un
update su una riga già archiviata non dà errore — aggiorna zero righe. Se serve un
ripristino, è una migration nuova sulla policy, non un `WHERE` diverso nella query.

**L'autorizzazione va scritta due volte, ed è voluto.** `documents_insert` pretende
in `WITH CHECK` che il chiamante sia staff oppure passi `can_access_attachment`. Lo
use case deve autorizzare con lo stesso criterio: se lo salta, il rifiuto arriva
dalla RLS come `403 auth.denied_by_policy`, che è il codice che significa "difetto
nostro" ([ADR-016](08-decisioni.md#adr-016), e il debito registrato sopra).

**Il nome del file è un dato personale.** `original_filename` finisce spesso per
essere `contratto-mario-rossi.pdf`. Non va nei log, non va negli eventi di dominio
(regola 18) e **non va nella chiave dell'oggetto**: la chiave si deriva da
`{org_id}/{document_id}`, che è anche il modo di non violare mai l'indice unico
`documents_object_uk`.

**Il ruolo applicativo non può cancellare.** Ha `SELECT, INSERT, UPDATE` e basta,
quindi la cancellazione è solo soft — ma l'oggetto nel bucket sopravvive alla riga.
In questa fetta nessuno lo rimuove: è un debito da registrare, insieme al purge di
`retention_until`, che lo schema prevede come job applicativo e non come trigger.

**L'indice per owner filtra `is_current`.** `documents_owner_ix` è parziale su
`deleted_at IS NULL AND is_current`: una query di elenco che non filtra entrambi non
lo usa. Vale anche se il versionamento resta fuori perimetro, perché `is_current`
nasce `true` per tutti.

**`reqwest` oggi è solo transitivo.** Arriva da `opentelemetry-otlp` alla versione
`0.13.4`. Dichiararlo diretto a una versione diversa fa fallire
`cargo deny check bans` sulle versioni duplicate — che è esattamente il controllo che
oggi passa e che nessuno guarda finché non diventa rosso.

### Come si saprà che è finita

`./ops/ci/test-integration.sh` verde con i test nuovi; un `curl` che carica un PDF su
un contratto, lo rilegge nell'elenco e lo scarica seguendo il `307`; il documento
invisibile a un utente di un'altra organizzazione e a un inquilino senza accesso a
quel contratto; `docs/05-api.md` che descrive i quattro endpoint e la guardia della
configurazione verde con la sezione `[storage]` documentata.
