# 09 — Stato e roadmap

Stato al **27 agosto 2026**.

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
| Dominio: value object, identità, immobili, stanze, contratti, inviti, documenti, eventi, porte | completo per la fetta |
| Unit of Work con contesto RLS, due pool | completo |
| Repository: utenti, organizzazioni, membership, sessioni, immobili, stanze, contratti, inviti, documenti | completi |
| Outbox | completo |
| Argon2id, EdDSA, refresh opachi con rotazione | completo |
| Telemetria: log strutturati, OTLP | completo |
| Bordo HTTP: Problem Details, estrattori, OpenAPI | completo |
| Registrazione, login, refresh, logout | completo |
| CRUD immobili con PATCH parziale e archiviazione | completo |
| CRUD stanze annidate sull'immobile | completo |
| Contratti: bozza, parti, attivazione, disdetta, stati derivati | completo |
| Inviti: emissione, revoca, accettazione senza pool privilegiato | completo |
| Documenti: caricamento multipart, elenco, download pre-firmato, archiviazione | completo per la fetta 1 |
| Storage oggetti: adattatore S3/MinIO, cifratura letta dal fornitore | completo |
| Migratore, dispatcher dell'outbox | completi |
| Immagine Docker, stack compose | completi |
| Pipeline CI | completa |

155 test: 75 unitari di dominio, 11 applicativi, 8 di infrastruttura, 6 unitari
del bordo HTTP, 18 di integrazione sulla persistenza, 3 sull'adattatore S3 contro
MinIO, 31 di contratto HTTP, 3 di guardia sulla documentazione della configurazione.

## Non implementato

### Moduli MVP che hanno lo schema ma non il codice

In ordine di dipendenza suggerito. Ognuno segue il pattern già stabilito: dominio →
porta → use case → repository → endpoint → test.

1. **Segnalazioni** (`tickets`) — thread, storico, assegnazione. Il piano della
   prima fetta è in fondo a questo documento:
   [Piano — Segnalazioni, fetta 1](#piano-segnalazioni-fetta-1).
2. **Bollette e addebiti** (`utility_bills`, `charges`) — con la vista
   `v_charge_balances` già disponibile.
3. **Pagamenti** (`payments`) — provider `manual` per l'MVP.
4. **Spese co-living** (`expense_groups`, `expenses`, `expense_shares`,
   `settlements`) — la vista dei saldi esiste già.
5. **AI: OCR e triage** (`ai_jobs`) — i trait dei provider vanno definiti allora,
   verificando la reference del fornitore scelto in quel momento.
6. **Dashboard** — `v_owner_dashboard` esiste, l'endpoint no.

**Documenti** è implementato nella fetta 1: caricamento, elenco, download e
archiviazione su immobili, stanze e contratti. Restano fuori, e non per
dimenticanza, il versionamento (V2) e l'OCR (punto 5); allargare gli owner ad
avatar, ticket, bollette e spese è additivo e si fa quando quei moduli esistono.
Il resoconto è in fondo: [Piano — Documenti, fetta 1](#piano-documenti-fetta-1).

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
| `org.staff_role_required` è vero solo a metà sull'archiviazione di un documento | `DELETE /api/v1/documents/{id}` lo restituisce a chi non è staff e non ha caricato il documento. Ricalca la `USING` di `documents_update` — staff **oppure** chi ha caricato — ed è più onesto di lasciar produrre `auth.denied_by_policy` alla RLS, che significa "difetto nostro". Ma il nome dice al chiamante che gli serve un ruolo, e all'uploader quel ruolo non serve: chi legge il codice senza la policy davanti conclude una cosa falsa. Servirebbe un codice proprio, del tipo `document.not_yours` | **Insieme al debito qui sopra**, di cui è la stessa conversazione: entrambi riguardano quale codice meriti un rifiuto al bordo, e deciderli separatamente vorrebbe dire toccare la tassonomia degli errori due volte. Finché nessun client instrada su quel codice, cambiarlo è additivo e non è una rottura — è **quella** la finestra, e si chiude con il primo client |
| L'organizzazione dell'invito non diventa mai predefinita | `pick_default_membership` preferisce il ruolo più alto (owner → manager → tenant) e la registrazione crea sempre un'organizzazione personale di cui l'utente è owner. Un inquilino registrato da sé deve quindi passare `X-Org-Id` per sempre; senza, gli elenchi rispondono `200` con zero elementi. Osservato percorrendo l'MVP a mano | Con il primo client: è lì che il carico si vede, ed è lì che si decide se l'accettazione debba spostare la predefinita |
| Il pacchetto su GHCR non è raggiungibile | `docker pull ghcr.io/fedsbringhetti/prop-flow:0.2.0` risponde `403` senza `read:packages`, e le note di rilascio invitano a eseguirlo. Coerente finché il repository è privato — il pacchetto ne eredita la visibilità — e rotto per tutti il giorno in cui non lo sarà. Si cambia nelle impostazioni del pacchetto, non nel workflow | Prima di rendere pubblico il repository, o al primo utente esterno |
| L'oggetto nel bucket sopravvive alla riga archiviata | La cancellazione è soft e il ruolo applicativo non ha `DELETE`; `ObjectStorage` non espone una rimozione, di proposito. Un documento archiviato lascia i suoi byte nello storage a tempo indeterminato | Insieme al purge di `retention_until`: sono lo stesso lavoro — un job applicativo che cancella righe e oggetti, che lo schema prevede e che nessuno ha ancora scritto |
| `retention_until` non viene applicato da nessuno | La colonna esiste e nessuna riga la valorizza; anche valorizzata, nessun job la legge. Lo schema la prevede come job applicativo e non come trigger, deliberatamente | Prima del primo dato reale soggetto a un obbligo di conservazione |
| Il caricamento tiene il file intero in memoria | `UploadDocument` riceve `Vec<u8>` e lo passa allo storage. Con il limite di 25 MiB e poche richieste concorrenti è sostenibile; non lo è con caricamenti grandi e concorrenza alta | Quando i caricamenti superano le decine di MiB, o quando il profilo di memoria dell'API lo mostra. Lo streaming verso lo storage richiede di cambiare la firma della porta |
| Il caricamento apre due transazioni | La prima verifica che l'owner sia visibile prima di scrivere l'oggetto, la seconda registra la riga: l'oggetto va scritto fuori transazione, e senza la verifica preliminare chiunque potrebbe riempire il bucket puntando a identificatori a caso | Se il costo si vede sul pool di connessioni. La verifica non si toglie: è ciò che impedisce di scrivere byte per una richiesta che finirà `404` |
| `actions/checkout@v4` gira su Node 20 | I runner lo forzano già su Node 24 con un avviso di deprecazione. Non rompe oggi, romperà quando GitHub toglierà il ponte | Al prossimo intervento sui workflow, o quando l'avviso diventa errore |

## Prossimo passo suggerito

**Segnalazioni** (`tickets`). I documenti sono chiusi per la fetta 1, e con loro la
porta `ObjectStorage`: da qui i moduli rimasti non hanno più dipendenze bloccanti fra
loro. I ticket vengono per primi perché sono l'altra metà del rapporto bidirezionale —
finora l'inquilino può solo leggere — e perché `can_access_attachment` copre già
`ticket` e `ticket_message`, quindi allegare un documento a una segnalazione sarà
additivo e non richiederà di ripensare nulla.

Il piano della prima fetta è in fondo:
[Piano — Segnalazioni, fetta 1](#piano-segnalazioni-fetta-1).

Manca ancora l'**invio delle notifiche**: la porta `Notifier` è dichiarata e senza
adattatore, quindi il token di invito torna nella risposta HTTP e lo consegna il
proprietario. Va implementata prima di considerare l'onboarding pronto per utenti
reali, ed è oggi l'ultima porta senza implementazione.

---

<a id="piano-documenti-fetta-1"></a>

## Piano — Documenti, fetta 1

Pianificato il **18 agosto 2026**, **implementato il 27 agosto 2026**. Il piano resta
sotto come è stato scritto: quello che segue serviva a eseguirlo, e serve ancora a
capire perché il codice è com'è. Ciò che l'esecuzione ha confermato, cambiato e
scoperto è in fondo, in [Esito](#esito-documenti-fetta-1) — letto per primo se la
domanda è «cosa è successo», per secondo se la domanda è «perché».

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


<a id="esito-documenti-fetta-1"></a>

### Esito — che cosa l'esecuzione ha confermato, cambiato e trovato

Scritto il **27 agosto 2026**, a fetta conclusa. I dieci passi sono stati eseguiti
tutti; le sette trappole si sono presentate come descritte, tranne due, e ne sono
emerse quattro nuove.

#### Le trappole, verificate una per una

| Trappola del piano | Si è presentata? |
|---|---|
| Il limite del corpo è globale | **Sì, e più grande.** I limiti sono due, non uno: vedi la trappola nuova qui sotto e [ADR-025](08-decisioni.md#adr-025) |
| `documents_update` non lascia toccare una riga archiviata | Sì. L'invariante di dominio `Invariant::DocumentArchived` ferma la richiesta prima che diventi un update a zero righe, e un test di persistenza lo fissa |
| L'autorizzazione va scritta due volte | Sì. `ensure_owner_is_visible` la ripete leggendo l'aggregato dal proprio repository, che gira sotto le stesse policy: riscriverla come condizione applicativa la farebbe divergere dalla policy alla prima modifica di una delle due |
| Il nome del file è un dato personale | Sì. `FileName` ha un `Debug` che stampa `<redatto>`, non entra negli eventi, e la chiave è `{org_id}/{document_id}`. Tre test lo verificano, in tre livelli diversi |
| Il ruolo applicativo non può cancellare | Sì. L'oggetto resta nel bucket dopo l'archiviazione: registrato fra i debiti, insieme al purge di `retention_until` |
| L'indice per owner filtra `is_current` | Sì. `list_by_owner` filtra `deleted_at IS NULL AND is_current` anche se il versionamento è fuori perimetro, con il commento che spiega perché |
| `reqwest` è solo transitivo | **In parte.** Vedi sotto: la versione non era il problema |

#### Le due trappole che non si sono presentate come descritte

**`reqwest` transitivo: la versione non era il problema, il provider TLS sì.**
Il piano temeva che dichiarare `reqwest` come dipendenza diretta facesse fallire
`cargo deny check bans` sulle versioni duplicate. Dichiararlo alla stessa versione
(`0.13`) non duplica nulla — e comunque `deny.toml` ha `multiple-versions = "warn"`,
quindi un duplicato **non farebbe fallire** il controllo: lo segnalerebbe fra gli
altri sedici che già ci sono. La trappola era mal calibrata in due modi
contemporaneamente.

Il problema vero era un altro, ed era più grave: la feature `rustls` di reqwest 0.13
implica `aws-lc-rs`, cioè il toolchain C e cmake nell'immagine di build — esattamente
ciò che [ADR-012](08-decisioni.md#adr-012) evita per `jsonwebtoken`. La via è
`rustls-no-provider` più il provider `ring` installato all'avvio, e senza quel
`install_default` reqwest **va in panico** costruendo il client, non restituisce un
errore. Motivato in [ADR-024](08-decisioni.md#adr-024).

Il solo effetto sulle dipendenze è **un** avviso di duplicato in più rispetto a
prima — `core-foundation`, tirato da `rustls-platform-verifier` e limitato al target
macOS. `cargo deny check advisories bans sources` passa.

**`is_encrypted` non nasce `true`.** Il piano diceva che la colonna nasce `true` e che
con SSE dice il vero «al costo di una configurazione». Il costo di quella
configurazione è precisamente il problema: sta nel provisioning del bucket, che non è
in questo repository, e se qualcuno lo dimentica la colonna mente senza che nulla lo
riveli. L'adattatore legge invece l'header `x-amz-server-side-encryption` dalla
risposta della `PUT` e registra ciò che il fornitore dichiara di aver fatto: su un
bucket non configurato `is_encrypted` vale `false`. Costa zero — l'header è già lì — e
trasforma una speranza in un fatto interrogabile. [ADR-024](08-decisioni.md#adr-024).

#### Le quattro trappole nuove

Sono le cose che costano una giornata e che il piano non poteva sapere.

**1. I limiti del corpo sono due, e il secondo non è un layer.** Oltre a
`RequestBodyLimitLayer`, `Multipart` di axum applica `DefaultBodyLimit`, che vale
**2 MiB** e non compare fra i layer del router. Alzando solo il primo, un file da
3 MiB fallisce lo stesso, e l'errore arriva dal parser multipart mentre legge il
corpo: cercarlo fra i layer non lo trova. Le rotte di caricamento montano entrambi.
Il test che lo tiene fermo carica **3 MiB** proprio perché è sopra tutti e due:
alzarne uno solo lo farebbe fallire. [ADR-025](08-decisioni.md#adr-025).

**2. Il marcatore `SALTATO` e la CI si contraddicevano.** Il piano chiedeva un test
contro MinIO «solo se la variabile d'ambiente c'è, con il marcatore `SALTATO` come gli
altri». Ma la pipeline **fallisce** se trova quella parola nell'output (`.github/workflows/ci.yml`,
passo `Suite`): il marcatore esiste perché un verde non possa significare «non ho
verificato niente». Un test che si salta sempre in CI avrebbe rotto la CI, oppure —
peggio — avrebbe costretto a indebolire il grep.

La via presa è la terza: rendere lo storage **disponibile** invece di rendere il
marcatore accettabile. `ops/ci/prepare-object-storage.sh` avvia MinIO e crea il
bucket, ed è chiamato sia da `ops/ci/test-integration.sh` sia dal job `tests`, come
già fa `prepare-database.sh` per il database e per la stessa ragione: due procedure di
preparazione divergono al primo cambiamento. Il marcatore resta, per chi esegue
`cargo test` a mano senza Docker.

MinIO **non** è un service container del runner: la sua immagine vuole `server /data`
come comando, e i service di GitHub Actions non permettono di sovrascriverlo. Un
`docker run` esplicito è la stessa riga in locale e in CI.

**3. Il caricamento vuole due transazioni, e la prima non è ridondante.** L'oggetto va
scritto fuori dalla transazione — tenerla aperta durante una chiamata di rete occupa
una connessione del pool per tutta la durata dell'upload — ma l'owner va verificato
**prima** di scrivere l'oggetto: senza, un membro qualsiasi potrebbe riempire il
bucket puntando a identificatori a caso, e il `404` arriverebbe dopo aver già scritto
i byte. Quindi: transazione breve per verificare, `put`, transazione per registrare —
con la verifica ripetuta dentro, perché fra le due l'owner può essere archiviato. Un
test di contratto verifica che un caricamento rifiutato non lasci **nessun** oggetto
orfano.

**4. `content-type` deve stare fra gli header firmati.** Se resta fuori dalla firma
SigV4, il fornitore lo tratta come un header qualsiasi e può ignorarlo: il documento
torna come `application/octet-stream` al download, e la causa è a due giorni di
distanza dal sintomo. `rusty_s3` lo firma se lo si mette in `headers_mut()`, e allora
la richiesta HTTP **deve** portarlo con lo stesso identico valore, altrimenti la firma
non torna.

#### Scelte prese eseguendo, che il piano lasciava aperte

- **Il percorso è `/api/v1/{collezione}/{id}/documents`** con tre rotte concrete
  (`properties`, `rooms`, `leases`) invece di una sola con `{owner_type}` come
  parametro. Costa tre handler sottili e tre annotazioni `utoipa`, e in cambio non
  c'è ambiguità di routing con `/api/v1/documents/{id}/content` né un parametro di
  percorso da validare a mano; la specifica OpenAPI nomina il parametro vero
  (`lease_id`) invece di un generico `owner_id`.
- **L'elenco è paginato con cursore keyset**, come immobili e contratti e non come le
  stanze: i documenti di un contratto non sono limitati per costruzione, e un solo
  `GET` non deve poter esaurire memoria e connessione.
- **`DocumentArchived` esiste**, benché il piano nominasse solo `DocumentUploaded`.
  Archiviare senza evento avrebbe rotto la proprietà per cui non esiste una modifica
  senza l'evento che la descrive ([02 — Architettura](02-architettura.md)), e reso i
  documenti l'unico aggregato con un'archiviazione muta.
- **`status` ed `extracted_data` non si leggono nemmeno.** Il piano diceva che restano
  ai default; la query non li seleziona affatto, perché il dominio non li modella e
  leggere una colonna che nessuno sa interpretare significa doverla mappare su un tipo
  inventato per l'occasione.
- **`PROPFLOW__SERVER__UPLOAD_BODY_LIMIT_BYTES` è un'impostazione in più** rispetto
  alle cinque che il piano elencava per `[storage]`. Sta in `[server]` e non in
  `[storage]` perché è un limite di trasporto: riguarda quanti byte l'API accetta, non
  dove finiscono.
- **`AttachmentOwnerType` nel dominio ha tre valori su undici.** Un documento allegato
  a un ticket dà **errore** in lettura invece di sparire dall'elenco: è la regola degli
  enum di `docs/06`, e qui vale a maggior ragione — un documento che l'API non sa
  mostrare non deve svanire in silenzio.

#### Come si è verificato

`./ops/ci/test-integration.sh` verde con 155 test (erano 126), di cui 3 contro un
MinIO vero. Riprodotto anche il controllo della pipeline — `cargo test --nocapture`
più il grep di `SALTATO` — che non trova nulla: nessun test si salta.

A mano, contro PostgreSQL e MinIO reali: caricato un PDF su un contratto, riletto
nell'elenco, scaricato seguendo il `307` e confrontato byte a byte con l'originale;
verificato che la chiave nel bucket sia `{org_id}/{document_id}` e non contenga il
nome del file; che un utente di un'altra organizzazione riceva `404 document.not_found`;
che dopo il `DELETE` l'elenco sia vuoto e l'oggetto sia ancora nel bucket, che è il
debito dichiarato e non un difetto.

#### Cosa resta fuori, e resta fuori apposta

Versionamento (V2), OCR (modulo AI), owner diversi da immobile/stanza/contratto,
cancellazione dei byte dallo storage, purge di `retention_until`, streaming del
caricamento. I primi tre erano già dichiarati fuori perimetro dal piano; gli ultimi
tre sono registrati fra i [debiti noti](#debiti-tecnici-noti) con la ragione e il
momento in cui affrontarli.

---

<a id="piano-segnalazioni-fetta-1"></a>

## Piano — Segnalazioni, fetta 1

Pianificato il **27 agosto 2026**, non ancora implementato. Scritto qui prima di
scrivere codice, come chiede `CLAUDE.md` §3: quello che segue è la decomposizione,
non un riassunto.

### Obiettivo

Chiudere l'anello nell'altro verso. Oggi l'inquilino accetta un invito e **legge**:
il proprio contratto, il proprio immobile, i documenti che lo riguardano. Non ha un
modo di dire che il boiler perde. Le segnalazioni sono la prima capacità che gli dà
voce, ed è la ragione per cui vengono prima di bollette, pagamenti e spese, che sono
tutti flussi dal proprietario verso il basso.

**Dentro:** apertura di una segnalazione da parte dell'inquilino o dello staff,
elenco filtrato, dettaglio con il thread e lo storico degli stati, messaggi pubblici
e note interne allo staff, il ciclo di vita degli stati, assegnazione, costi
stimati e sostenuti, allegati sul ticket.

**Fuori, e non per dimenticanza:**

- **il triage AI** (`ai_job_id`, `ai_suggested_category`, `ai_suggested_priority`,
  `ai_confidence`, `ai_analysis`): è il punto 5 della lista dei moduli, e i trait dei
  provider vanno definiti allora, verificando la reference del fornitore scelto in
  quel momento. Le colonne restano ai loro default. Lo schema tiene i suggerimenti
  del modello separati dai valori decisi da una persona proprio perché si possa
  misurare l'accuratezza dopo: riempirle con valori umani distruggerebbe quel dato
  prima ancora di raccoglierlo;
- **la ricerca full-text**: `search_vector` esiste con il suo indice GIN, ma
  `docs/01-prodotto.md` colloca Meilisearch in V2 e la colonna è pensata per essere
  alimentata dall'outbox. Esporre una `?q=` che interroga `tsvector` ora significa
  costruire un contratto che poi va rotto;
- **gli SLA** (`sla_due_at`): la colonna c'è e l'indice pure, ma non esiste una
  politica che dica quanto tempo ha una categoria per essere risolta, e nessuno
  scheduler che guardi le scadenze. Una data di scadenza che nessuno controlla è
  peggio di nessuna data;
- **la programmazione** (`scheduled_at`): il calendario è V2;
- **gli allegati sui messaggi** (`ticket_message` come owner): il ticket sì, il
  singolo messaggio no. Allargare dopo è additivo, e nella fetta 1 la foto del
  guasto sta sulla segnalazione, dove chi la cerca la trova;
- **il ruolo `manager`**: `docs/01-prodotto.md` lo dà come predisposto e non ancora
  esposto. Le segnalazioni non lo cambiano: `is_org_staff` copre owner e manager
  insieme, come ovunque.

### Decisioni prese, con il perché

**Il ciclo di vita: `open → triaged → assigned → in_progress → closed`, con la
riapertura in mano a chi ha segnalato.**

```
open ──► triaged ──► assigned ──► in_progress ──► closed
  │         │           │              │             │
  │         └───────────┴──────────────┤             │
  │                          waiting_tenant          │
  │                          waiting_owner           │
  │                                                  │
  └──────────── cancelled ◄──────────────────────────┘
                                                     │
                            open ◄───── riapertura ──┘
```

Lo staff guida la lavorazione; l'inquilino apre, commenta, e **riapre** se il guasto
si ripresenta. La riapertura è ciò che rende la segnalazione un rapporto invece di
una casella postale: senza, un guasto ricorrente diventa una segnalazione nuova ogni
volta e lo storico si frantuma proprio dove servirebbe intero.

`waiting_tenant` e `waiting_owner` non sono una fase a sé ma una sospensione della
lavorazione: si entra e si esce da `in_progress`, e servono a dire perché un ticket
è fermo — «aspetto che tu sia in casa» è lo stato più comune di una manutenzione
reale, e senza un nome resta un ticket che sembra abbandonato.

**Alternativa scartata: `resolved` come passo separato, con conferma dell'inquilino.**
È ciò che la coppia `resolved`/`closed` distinta nello schema lascia intendere, ed è
il flusso più fedele alla bidirezionalità. Scartata perché introduce un'attesa che
senza scheduler non si chiude mai da sé: un ticket risolto e mai confermato resta
appeso per sempre, e la chiusura automatica dopo N giorni è precisamente lo scheduler
che non abbiamo. La riapertura ottiene lo stesso risultato — l'inquilino ha voce
sulla risoluzione — senza richiedere un orologio.

**Conseguenza da dire ad alta voce: `resolved` non lo scrive mai l'applicazione.**
Resta un valore dell'enum del database che il codice sa leggere e non produce, come
`expiring` ed `expired` per i contratti ([ADR-019](08-decisioni.md#adr-019)). Va
comunque nella tabella di conversione di `string_enum!`, perché una riga scritta a
mano potrebbe contenerlo e un valore sconosciuto in lettura deve dare errore.
Aggiungerlo poi come transizione è additivo, non è una rottura.

**Le transizioni sono sotto-risorse, non un `PATCH` sullo stato.** Stessa scelta di
attivazione e disdetta dei contratti e dell'archiviazione degli immobili, e per la
stessa ragione: ognuna ha effetti propri — `closed_at`, una riga di storico, un
evento — e un `PATCH` che li produce di soppiatto rende invisibile che siano
avvenuti. Il `PATCH` resta per ciò che è davvero un attributo: i costi.

**Il `PATCH` è solo per i costi, e solo allo staff.** Titolo e descrizione non si
modificano: sono il testo con cui una persona ha descritto un guasto in un momento
preciso, e il thread esiste apposta per correggere il tiro. Un titolo riscritto dopo
tre messaggi rende incomprensibili i tre messaggi.

**Categoria e priorità si impostano con il triage, non con il `PATCH`.** Sono ciò che
`triaged` *significa* — qualcuno ha guardato la segnalazione e l'ha classificata — e
lasciarle modificabili da un `PATCH` renderebbe lo stato `triaged` una dichiarazione
non verificabile.

**L'inquilino può annullare solo finché nessuno ci ha messo mano.** `cancelled` da
`open` se è chi ha segnalato; da qualunque stato non terminale se è staff. Un
inquilino che annulla un ticket già in lavorazione butterebbe via il lavoro di
qualcun altro.

**Il thread è paginato a parte dal dettaglio.** `GET /tickets/{id}` restituisce la
segnalazione e lo storico degli stati — che è corto e limitato per costruzione — e i
messaggi stanno su `GET /tickets/{id}/messages` con cursore keyset. Una conversazione
non ha un tetto: incorporarla nel dettaglio significa che il giorno in cui qualcuno
scrive duecento messaggi il dettaglio smette di rispondere.

**Le note interne sono un campo del messaggio, non una risorsa separata.**
`is_internal` esiste già e la policy di SELECT lo filtra per l'inquilino. Una tabella
separata avrebbe due thread da fondere in lettura e due punti dove sbagliare l'ordine.

**Gli allegati riusano `AttachmentOwner`, senza migration.** L'enum
`app.attachment_owner_type` ha già `ticket`, e `can_access_attachment` lo risolve su
`can_access_ticket`. Aggiungere la variante al dominio e le due rotte è tutto ciò che
serve: è il dividendo di aver fatto i documenti per primi, ed è la ragione per cui
erano primi.

**Nessuna migration.** Lo schema copre l'intero modulo dalla `0007_tickets.sql`, e
questa fetta si limita a usarlo. In particolare **non** si aggiungono privilegi di
colonna alla `tickets` — vedi la trappola sulla `WITH CHECK`, dove è spiegato perché
[ADR-021](08-decisioni.md#adr-021) non si applica qui.

### Decomposizione

Ogni passo è eseguibile e verificabile da solo, nell'ordine di `docs/06`
§Aggiungere un caso d'uso. I passi 9 e 10 sono additivi: se la fetta risulta troppo
grande in esecuzione, **il punto di taglio è dopo il passo 8** — costi e allegati
diventano una fetta 1b e il modulo resta utile senza.

1. **Dominio: gli enum e i value object.** `TicketCategory`, `TicketPriority`,
   `TicketStatus`, `TicketActor` come enum di dominio, con le righe nella macro
   `string_enum!` (`docs/06` §Aggiungere un enum). Value object per titolo e
   descrizione con i limiti dello schema (`title` fra 1 e 200 caratteri dopo il trim,
   `description` obbligatoria e non vuota). `TicketReference` come tipo che il
   dominio sa **leggere** e non costruire — vedi la trappola.
2. **Dominio: l'aggregato e la macchina a stati.** `Ticket` con i campi privati e i
   metodi di transizione (`triage`, `assign`, `start`, `wait_for`, `close`, `reopen`,
   `cancel`), ognuno che rifiuta le transizioni non ammesse con una variante di
   `Invariant`. `TicketMessage` come entità del thread. Test unitari puri sulla
   macchina: è il punto in cui una tabella delle transizioni si verifica senza
   database, e va coperta per intero — le transizioni ammesse **e** quelle rifiutate.
3. **Dominio: gli eventi.** `TicketOpened`, `TicketStatusChanged`, `TicketAssigned`,
   `TicketMessagePosted`. Nessuno trasporta titolo, descrizione o corpo del
   messaggio: è testo libero scritto da una persona su un guasto in casa propria, e
   un evento pubblicato su un broker è un dato che viaggia e si conserva (regola 18,
   stessa ragione per cui `LeaseTerminated` non porta `termination_reason`).
4. **Porte di persistenza.** `TicketRepository` (`insert`, `find_by_id`, `list`,
   `update`), `TicketMessageRepository` (`insert`, `list_for_ticket`) e la lettura
   dello storico (`list_status_history`). Nessun metodo di scrittura sullo storico:
   lo scrive un trigger, e una porta che lasciasse scriverlo inviterebbe a farlo.
5. **Use case: apertura ed elenco.** `open_ticket`, `list_tickets`, `get_ticket`, con
   la sequenza canonica. L'apertura risolve immobile, stanza e contratto attraverso i
   rispettivi repository — è lo stesso controllo che i documenti fanno con
   `ensure_owner_is_visible`, e per la stessa ragione: senza, il rifiuto arriva dalla
   RLS come `403 auth.denied_by_policy`.
6. **Use case: il thread.** `post_message`, `list_messages`. `is_internal` richiede
   `ensure_staff`; la policy lo ripete, e il service layer deve rifiutare prima.
7. **Use case: le transizioni.** Uno use case per transizione, ognuno che autorizza,
   carica l'aggregato, chiama il metodo di dominio, persiste ed emette l'evento.
   `reopen` è l'unico che un inquilino può eseguire, e solo se è chi ha segnalato.
8. **Infrastruttura.** `PgTicketRepository` e la lettura dello storico. La `INSERT`
   usa `RETURNING reference`: è la prima del progetto a farlo, e la trappola spiega
   perché non c'è alternativa. Query come letterali statici con `concat!`, cast
   espliciti sugli enum.
9. **Bordo HTTP.** Le rotte, i DTO, le annotazioni `utoipa`, la registrazione in
   `openapi.rs` e nel router. `POST /api/v1/properties/{property_id}/tickets` per
   l'apertura — annidata come le stanze, perché una segnalazione nasce sempre su un
   immobile — e tutto il resto per id.
10. **Costi e allegati.** `PATCH /api/v1/tickets/{id}` per `estimated_cost` e
    `actual_cost`, con la semantica del `Patch<T>` già in uso. `AttachmentOwner::Ticket`
    più `POST`/`GET /api/v1/tickets/{id}/documents`, che riusano gli use case dei
    documenti senza modificarli.
11. **Test.** Unitari di dominio sulla macchina a stati; di persistenza sul
    repository con database reale, compreso che lo storico si popoli da sé; di
    contratto HTTP sull'intero giro, con l'inquilino che apre, allega una foto,
    commenta, non vede le note interne, e riapre dopo la chiusura.
12. **Documentazione.** Endpoint in `docs/05-api.md`, il modulo spostato da «non
    implementato» a «implementato» in questa roadmap, un ADR se una di queste
    decisioni viene cambiata strada facendo, e l'esito del piano come per i documenti.

### Trappole

Sono le vie che sembrano corrette e non lo sono. Tutte verificate contro lo schema
prima di scriverle qui.

**`reference` la assegna il database, e nessun repository sa ancora farlo.**
`tickets.reference` ha come `DEFAULT` `'TK-' || to_char(nextval('app.ticket_reference_seq'), 'FM000000')`.
È il primo campo del progetto che l'aggregato **non** può produrre: ogni altro
identificatore nasce nel dominio con `generate()`. Ne segue che la `INSERT` deve
usare `RETURNING reference` e che `Ticket::open` non può restituire un aggregato
completo prima di aver scritto — o si accetta un `Option<TicketReference>` che si
riempie alla persistenza, o si legge la riga dopo l'inserimento. Nessun repository
del progetto usa `RETURNING` oggi, quindi non c'è un esempio da copiare.

Generare la reference nell'applicazione **non** è la scorciatoia che sembra: si
metterebbe in competizione con la sequenza, e il primo `INSERT` che non la specifica
la farebbe avanzare comunque, producendo collisioni sull'indice unico
`tickets_reference_uk` a distanza di mesi dal cambiamento.

**La sequenza è globale, non per organizzazione.** `TK-000042` dice a chiunque quante
segnalazioni esistono sull'intera piattaforma, non nella propria organizzazione. Non
è un dato sensibile ma è un dato che non intendevamo dare, e cambiarlo dopo significa
rinumerare o convivere con due formati. Da registrare fra i debiti alla fine della
fetta, non da risolvere dentro.

**Non esiste alcun vincolo fra `room_id`, `lease_id` e `property_id`.** Il trigger
`assert_org_matches_property` verifica soltanto che l'`org_id` del ticket coincida
con quello dell'immobile. Una stanza di **un altro immobile** o un contratto di un
altro immobile sono accettati dallo schema senza protestare, e il risultato è una
segnalazione che nei report appare sotto un immobile e nell'interfaccia sotto un
altro. Il service layer deve verificarlo, come già fa `CreateRoom` con l'immobile
padre: leggerli dai rispettivi repository e confrontare, non fidarsi.

**`ticket_messages` non ha una policy di `UPDATE`.** Ce ne sono due, `_select` e
`_insert`, e basta. Ne segue che un messaggio **non si modifica e non si cancella**,
nemmeno in modo soft: la colonna `deleted_at` esiste sulla tabella e nessuno può
valorizzarla. Non è una svista da correggere di passaggio — un thread di conversazione
in cui le righe non si riscrivono è una proprietà, non un limite. Ma qualunque cosa
prometta «cancella il messaggio» sta promettendo ciò che il database rifiuta, e
servirebbe una migration nuova sulla policy.

**La `WITH CHECK` di `tickets_update` vincola solo `org_id`.** La `USING` lascia
passare staff, chi ha segnalato **e** chi è assegnato; la `WITH CHECK` non nomina
nessun'altra colonna. La Row Level Security quindi **non** impedisce a un inquilino
di impostare `actual_cost`, `assigned_to`, `priority` o `property_id` sulla
segnalazione che ha aperto lui. Deve farlo il service layer, transizione per
transizione.

E qui la via che sembra corretta e non lo è: **non si aggiungono privilegi di
colonna** come ha fatto la `0019` per inviti e parti contrattuali.
[ADR-021](08-decisioni.md#adr-021) dichiara il proprio criterio, ed è preciso —
quello strumento serve alle policy che concedono scrittura a un **non membro**,
attraversando il confine dell'organizzazione. Qui chi scrive è già membro e il danno
resta dentro il perimetro di chi ha già accesso: muovere la propria segnalazione fra
due immobili della propria organizzazione è un fastidio, non una violazione.
Applicare lo strumento fuori dal suo criterio significa pagarne il costo — ogni
colonna nuova da aggiungere alla `GRANT` — senza il pericolo che lo giustifica.

**Lo storico lo scrive un trigger, e scriverlo anche noi lo raddoppia.**
`tg_ticket_status_history` scatta `AFTER INSERT OR UPDATE OF status` e inserisce da
sé. Due conseguenze: l'inserimento di una segnalazione produce già la riga
`NULL → open`, quindi il caso d'uso di apertura non deve aggiungerne una; e
`ticket_status_history` **non ha una policy di INSERT**, quindi una scrittura
applicativa verrebbe rifiutata — funziona solo perché il trigger è `SECURITY DEFINER`
e gira come proprietario della tabella. Il campo `changed_by` lo riempie
`app.current_user_id()`, che fuori da un contesto di richiesta è nullo.

**`closed_at` non si azzera da solo alla riapertura.** Il vincolo
`tickets_closed_ck` è `status <> 'closed' OR closed_at IS NOT NULL`: constringe una
sola direzione. Riaprire lascia quindi un `closed_at` nel passato su una segnalazione
che risulta `open`, e il database lo accetta senza dire niente. Deve azzerarlo il
dominio, nel metodo `reopen`. Lo stesso varrebbe per `resolved_at` il giorno in cui
`resolved` entrasse in uso.

**Solo chi ha segnalato può riaprire, e non è una scelta di prodotto.** La `USING` di
`tickets_update` è staff **oppure** `reported_by` **oppure** `assigned_to`: non passa
dal contratto. Un coinquilino sullo stesso contratto *vede* la segnalazione — glielo
concede `can_access_ticket` attraverso `lease_id` — ma non può aggiornarla. Se il
prodotto volesse che chiunque sul contratto possa riaprire, è una migration nuova
sulla policy, non un ramo applicativo.

**Una segnalazione senza `lease_id` è invisibile all'inquilino che ci abita.**
`can_access_ticket` concede l'accesso a staff, a chi ha segnalato, a chi è assegnato,
e solo poi — se `lease_id` non è nullo — a chi è parte di quel contratto. Una
segnalazione aperta dallo staff su un immobile, senza contratto indicato, non la vede
nessun inquilino. Quando si apre per conto di qualcuno, `lease_id` va valorizzato, o
la persona che dovrebbe seguirla non sa che esiste.

**`search_vector` è `GENERATED ALWAYS`.** Non va mai nominata in una `INSERT` o in
una `UPDATE`: PostgreSQL rifiuta la scrittura. È il genere di errore che non compare
finché qualcuno non aggiunge una colonna elencando i campi «tutti quanti».

**`tickets_open_ix` è parziale sui sei stati di lavorazione.** L'elenco della coda —
quello che la dashboard del proprietario legge — usa l'indice solo se filtra
`status` dentro `('open','triaged','assigned','in_progress','waiting_tenant','waiting_owner')`.
Un elenco «tutti i ticket non chiusi» scritto come `status <> 'closed'` non lo usa.

### Come si saprà che è finita

`./ops/ci/test-integration.sh` verde con i test nuovi. Un giro a mano che percorre il
rapporto nei due versi: l'inquilino apre una segnalazione sul proprio immobile con
una foto allegata, il proprietario la classifica e la assegna, si scambiano due
messaggi, il proprietario scrive una nota interna che **l'inquilino non vede**,
registra il costo sostenuto e chiude; l'inquilino riapre perché il guasto torna, e lo
storico degli stati mostra la catena intera compresa la riapertura. Un utente di
un'altra organizzazione non vede nulla di tutto questo, e un inquilino di un altro
contratto nemmeno.

`docs/05-api.md` che descrive gli endpoint, e la guardia della configurazione verde —
questa fetta non aggiunge impostazioni, quindi deve restare verde senza toccarla.
