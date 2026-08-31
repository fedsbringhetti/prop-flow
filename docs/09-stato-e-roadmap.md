# 09 — Stato e roadmap

Stato al **31 agosto 2026**.

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

Le 23 migration coprono l'intero perimetro funzionale dell'MVP, non solo la parte
già esposta dall'API: 24 tabelle più `audit_log` partizionata, 35 enum, 6 domini,
170 indici, 65 policy RLS, 50 trigger, 4 viste, 29 funzioni.

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
| Segnalazioni: apertura, elenco filtrato, dettaglio con storico, thread con note interne, sette transizioni, costi, allegati | completo per la fetta 1 |
| Bollette: registrazione manuale, correzione, validazione, annullamento, riaddebito, allegati | completo per la fetta 1 |
| Addebiti: emissione, elenco e dettaglio con il saldo dalla vista, annullamento, rinuncia all'incasso, allegati | completo per la fetta 1 |
| Storage oggetti: adattatore S3/MinIO, cifratura letta dal fornitore | completo |
| Migratore, dispatcher dell'outbox | completi |
| Immagine Docker, stack compose | completi |
| Pipeline CI | completa |

264 test: 133 unitari di dominio, 16 applicativi, 10 di infrastruttura, 6 unitari
del bordo HTTP, 38 di integrazione sulla persistenza, 3 sull'adattatore S3 contro
MinIO, 55 di contratto HTTP, 3 di guardia sulla documentazione della configurazione.

## Non implementato

### Moduli MVP che hanno lo schema ma non il codice

In ordine di dipendenza suggerito. Ognuno segue il pattern già stabilito: dominio →
porta → use case → repository → endpoint → test.

1. **Pagamenti** (`payments`) — provider `manual` per l'MVP. È il **prossimo passo
   suggerito**, e la fetta contabile gli ha già posato ciò che gli serve: la
   `GRANT UPDATE (status)` della migration `0023` basta a portare un addebito a
   `partially_paid` o `paid`, quindi il modulo si aggiunge con un metodo di dominio su
   `Charge` e uno use case, senza migration sullo schema degli addebiti. Finché non
   esiste, `paid_amount` vale sempre `0.00` in ogni risposta.
2. **Spese co-living** (`expense_groups`, `expenses`, `expense_shares`,
   `settlements`) — la vista dei saldi esiste già.
3. **AI: OCR e triage** (`ai_jobs`) — i trait dei provider vanno definiti allora,
   verificando la reference del fornitore scelto in quel momento. È il modulo che
   riempirà `ocr_extracted`, `ocr_confidence` e lo stato `pending_review`, che la
   fetta contabile ha deliberatamente lasciato ai loro default: lo schema tiene
   separato ciò che ha letto la macchina da ciò che ha confermato una persona
   (`validated_by`, `validated_at`) proprio perché l'accuratezza si possa misurare, e
   riempire le prime con valori umani distruggerebbe quel dato prima di raccoglierlo.
4. **Dashboard** — `v_owner_dashboard` esiste, l'endpoint no. La vista conta già
   `total_outstanding`, `overdue_charges` e `bills_pending_review`, e da questa fetta
   quei numeri hanno finalmente righe da contare.

**Documenti** è implementato nella fetta 1: caricamento, elenco, download e
archiviazione su immobili, stanze, contratti, segnalazioni e — dal modulo contabile —
bollette e addebiti. Restano fuori, e non per dimenticanza, il versionamento (V2) e
l'OCR (punto 3); allargare gli owner ad avatar, spese e pagamenti è additivo e si fa
quando quei moduli esistono. Il resoconto è in fondo:
[Piano — Documenti, fetta 1](#piano-documenti-fetta-1).

**Segnalazioni** è implementato nella fetta 1: apertura, elenco filtrato, dettaglio
con lo storico degli stati, thread con note interne, le sette transizioni del ciclo di
vita, costi e allegati. Restano fuori, e non per dimenticanza, il triage AI (punto 3),
la ricerca full-text (V2), gli SLA, la programmazione, gli allegati sui singoli
messaggi e le note di risoluzione. Il resoconto è in fondo:
[Piano — Segnalazioni, fetta 1](#piano-segnalazioni-fetta-1).

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
| **Le viste dei saldi non filtrano sullo stato** | `v_charge_balances` ha il solo `WHERE c.deleted_at IS NULL`, e `v_owner_dashboard` somma da lì senza filtrare a sua volta: `total_outstanding` conta anche gli addebiti `void` e `written_off`, e `overdue_charges` fa lo stesso con `is_past_due`. Riprodotto su database vero — 800.00 emesso + 120.00 **annullato** + 40.00 scaduto hanno dato `total_outstanding = 960.00`, dove il vero è 840.00. L'inquilino si è visto dire che quei 120 euro non li deve più. Non nasce con la fetta contabile — la vista è così dalla `0015` — ma è quella fetta a renderlo raggiungibile. **L'elenco dell'API è corretto**: `?outstanding=true` enumera i tre stati come letterale. Vedi [ADR-028](08-decisioni.md#adr-028) | **Prima della dashboard.** È una riga di `WHERE` per vista, ma cambia il significato di due numeri: finché nessun client li legge è additivo, dopo è una rottura. Richiede prima una decisione di prodotto — *outstanding* significa `issued` + `partially_paid`, e `written_off` si riporta a parte come perdita? |
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

**Prima di tutto, due righe di `WHERE`.** Le viste dei saldi non filtrano sullo stato,
quindi `total_outstanding` conta come credito anche gli addebiti annullati e stornati —
riprodotto, 960.00 dove il vero è 840.00, ed è il primo debito della tabella qui sopra.
Va corretto **prima della dashboard** e prima che qualcuno legga quei numeri: finché
nessun client li consuma cambiarne il significato è additivo, dopo è una rottura. È
poco lavoro e una decisione di prodotto, ed è l'unico debito del progetto la cui
finestra si chiude da sola.

Poi, il modulo:

**Pagamenti** (`app.payments`), provider `manual`. La fetta contabile lo ha reso il
passo più corto che resti: la `GRANT UPDATE (status)` della migration `0023` è già
tutto ciò che serve a portare un addebito a `partially_paid` o `paid`, quindi il modulo
si aggiunge con un metodo di dominio su `Charge` — che oggi **non** ha un `apply`,
apposta — e uno use case che, in una transazione, inserisce il pagamento, rilegge
l'addebito e ne scrive lo stato. **Nessuna migration sullo schema degli addebiti.**

È anche il modulo che rende vero un numero che oggi è sempre lo stesso:
`paid_amount` vale `0.00` in ogni risposta perché `app.payments` è vuota, non perché
il calcolo sbagli. Finché resta così, `outstanding_amount` coincide con `amount` e la
dashboard non ha nulla da distinguere.

Ciò che quel modulo dovrà decidere, e che questa fetta ha lasciato aperto: chi può
registrare un incasso (solo staff, o anche l'inquilino che dichiara di aver pagato),
come si gestisce l'`idempotency_key` che lo schema richiede, e se `void` e
`write_off` restino ammessi da `partially_paid` — oggi partono dal solo `issued`,
perché annullare un addebito su cui è già entrato del denaro chiede prima di dire cosa
ne è di quel denaro.

**La fonte di verità resta la vista**, anche dopo. `outstanding_amount` continuerà a
venire da `app.v_charge_balances` e non da `status`: lo stato memorizzato è una
comodità interrogabile — l'indice `charges_outstanding_ix` è parziale su quei tre
valori — che il modulo dei pagamenti tiene allineata come effetto secondario. Le due
definizioni non devono diventare due.

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

Pianificato il **27 agosto 2026**, **implementato il 29 agosto 2026**. Il piano resta
sotto come è stato scritto: quello che segue serviva a eseguirlo, e serve ancora a
capire perché il codice è com'è. Ciò che l'esecuzione ha confermato, cambiato e
scoperto è in fondo, in [Esito](#esito-segnalazioni-fetta-1) — letto per primo se la
domanda è «cosa è successo», per secondo se la domanda è «perché».

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

**Una migration sola, e sono privilegi di colonna.** Lo schema copre l'intero modulo
dalla `0007_tickets.sql` e questa fetta si limita a usarlo, con un'eccezione: la
`0020` toglie a `propflow_app` l'`UPDATE` sull'intera tabella `tickets` e gliela
riconcede sulle sole colonne che i repository scrivono. Stessa forma della `0019`.

La ragione sta nella trappola sulla `WITH CHECK`, insieme a ciò che si è visto
riproducendola. Ne discende la conseguenza che rende la migration quasi gratuita:
`property_id`, `room_id`, `lease_id`, `reported_by`, `title`, `description` e
`reference` restano **fuori** dalla `GRANT`, cioè non sono più scrivibili
dall'applicazione dopo l'inserimento. È accettabile perché una segnalazione aperta
sull'immobile sbagliato **si annulla e si riapre** — `cancelled` esiste per questo — e
non si corregge spostandola. Il giorno in cui servisse correggerla, è una migration
nuova, ed è il punto in cui ci si ferma a chiedersi se quella colonna debba davvero
essere scrivibile.

### Decomposizione

Ogni passo è eseguibile e verificabile da solo, nell'ordine di `docs/06`
§Aggiungere un caso d'uso.

**Il punto di taglio è il passo 11**, se la fetta risulta troppo grande in
esecuzione: è l'unico additivo, e tolto lascia un modulo completo — apertura, thread,
transizioni, endpoint, test. Tagliare più in alto non è un'opzione, perché il passo 10
è il bordo HTTP e senza quello non c'è niente da usare.

Dentro il passo 11 ci sono però **due cose di natura diversa**, e se si taglia
conviene tagliarle separatamente. Gli allegati sono additivi e chiusi: una variante
d'enum e due rotte che riusano gli use case dei documenti senza toccarli. I costi no:
`estimated_cost` e `actual_cost` sono il primo denaro su un aggregato che non è
contabile, e prima o poi qualcuno chiederà se il costo di una riparazione diventi un
addebito all'inquilino — domanda che appartiene al modulo delle bollette e degli
addebiti, non a questo. Se una delle due deve scivolare, scivolano i costi.

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
8. **Migration `0020`: i privilegi di colonna.** `REVOKE UPDATE ON app.tickets FROM
   propflow_app`, poi `GRANT UPDATE (…)` sulle sole colonne che i repository
   scrivono. Stessa forma della `0019`, guardia compresa per l'ambiente in cui il
   ruolo non esiste. Viene **prima** dell'infrastruttura, non dopo: le colonne
   concesse e quelle scritte dalle query sono la stessa lista, e scriverla due volte
   in ordine inverso è il modo di farle divergere. Un test di persistenza fissa il
   rifiuto, come già fa quello sulle parti contrattuali.
9. **Infrastruttura.** `PgTicketRepository` e la lettura dello storico. La `INSERT`
   usa `RETURNING reference`: è la prima del progetto a farlo, e la trappola spiega
   perché non c'è alternativa. Query come letterali statici con `concat!`, cast
   espliciti sugli enum. Ogni colonna che compare in una `UPDATE` deve comparire
   nella `GRANT` del passo 8.
10. **Bordo HTTP.** Le rotte, i DTO, le annotazioni `utoipa`, la registrazione in
    `openapi.rs` e nel router. `POST /api/v1/properties/{property_id}/tickets` per
    l'apertura — annidata come le stanze, perché una segnalazione nasce sempre su un
    immobile — e tutto il resto per id.
11. **Costi e allegati.** `PATCH /api/v1/tickets/{id}` per `estimated_cost` e
    `actual_cost`, con la semantica del `Patch<T>` già in uso. `AttachmentOwner::Ticket`
    più `POST`/`GET /api/v1/tickets/{id}/documents`, che riusano gli use case dei
    documenti senza modificarli.
12. **Test.** Unitari di dominio sulla macchina a stati; di persistenza sul
    repository con database reale, compreso che lo storico si popoli da sé e che le
    colonne strutturali diano `permission denied`; di contratto HTTP sull'intero
    giro, con l'inquilino che apre, allega una foto, commenta, non vede le note
    interne, e riapre dopo la chiusura.
13. **Documentazione.** Endpoint in `docs/05-api.md`, il modulo spostato da «non
    implementato» a «implementato» in questa roadmap, l'esito del piano come per i
    documenti, e **un ADR per la `0020`**: estende a un caso nuovo lo strumento di
    [ADR-021](08-decisioni.md#adr-021) con un criterio diverso da quello che
    quell'ADR dichiara, e una scelta del genere non può restare in una riga di
    migration.

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

**Osservato, non ipotizzato:** riproducendo la trappola sulla `WITH CHECK` si è
ottenuta esattamente questa riga — una segnalazione spostata su un immobile, con il
`lease_id` che continuava a puntare al contratto del primo. Lo schema l'ha accettata
senza una parola.

La `0020` cambia dove va fatto il controllo, non se: `property_id`, `room_id` e
`lease_id` restano fuori dalla `GRANT`, quindi dopo l'inserimento non sono più
scrivibili e l'incoerenza può nascere **solo all'apertura**. Il controllo va lì, una
volta sola, e non su ogni transizione.

**`ticket_messages` non ha una policy di `UPDATE`.** Ce ne sono due, `_select` e
`_insert`, e basta. Ne segue che un messaggio **non si modifica e non si cancella**,
nemmeno in modo soft: la colonna `deleted_at` esiste sulla tabella e nessuno può
valorizzarla. Non è una svista da correggere di passaggio — un thread di conversazione
in cui le righe non si riscrivono è una proprietà, non un limite. Ma qualunque cosa
prometta «cancella il messaggio» sta promettendo ciò che il database rifiuta, e
servirebbe una migration nuova sulla policy.

**La `WITH CHECK` di `tickets_update` vincola solo `org_id`.** La `USING` lascia
passare staff, chi ha segnalato **e** chi è assegnato; la `WITH CHECK` non nomina
nessun'altra colonna. Ogni colonna diversa da `org_id` è quindi libera per chiunque
la `USING` ammetta — e ammette l'inquilino che ha aperto la segnalazione.

**Riprodotto su database reale**, come ha fatto [ADR-021](08-decisioni.md#adr-021)
prima di esistere. Con `UPDATE` scritte a mano, come inquilino sotto RLS, sulla
segnalazione che ha aperto lui:

| Tentativo | Esito |
|---|---|
| `actual_cost = 9999.00` | riuscito |
| `assigned_to` = un altro utente | riuscito |
| `status = 'closed'`, saltando il flusso | riuscito |
| `priority = 'urgent'` | riuscito |
| spostarlo su un immobile in cui non abita | riuscito |
| toccare la segnalazione di un altro | rifiutato dalla `USING` |
| spostarlo in un'altra organizzazione | rifiutato dal trigger `assert_org_matches_property` |

Due cose che l'esperimento ha stabilito e che il ragionamento da solo non dava.
**Il confine di isolamento tiene:** dopo tutte le scritture l'inquilino vede ancora
una sola segnalazione, non ha guadagnato visibilità su niente. **Il danno è
registrato:** lo storico annota `open → closed` con lui come autore, e `tickets` è
nella lista dei trigger di audit, quindi ogni `UPDATE` finisce in `audit_log`.

Non è quindi una violazione dell'isolamento come quella che ha prodotto ADR-021 — è
**corruzione di dati dentro la propria organizzazione**, tracciata.

**E si è deciso lo stesso di usare i privilegi di colonna** (la migration `0020`, vedi
le decisioni). Il criterio di ADR-021 — «policy che concedono scrittura a un non
membro attraversando il confine» — descrive perché *quelle due* ne avevano bisogno,
non esaurisce i casi in cui lo strumento conviene. Qui conviene per una ragione
diversa e più modesta: non proteggere un confine, ma togliere di mezzo una classe di
incoerenze che nessuna policy può esprimere, perché **una policy non può confrontare
la riga nuova con quella vecchia** — è la stessa constatazione che sta in ADR-021 e
che vale qui identica.

Ciò che rende la scelta quasi gratuita è che le colonne strutturali non hanno un
motivo legittimo di cambiare: un ticket mal classificato si annulla e si riapre.

**L'onere di manutenzione non è un rischio silenzioso.** Ogni colonna aggiunta in
futuro a una `UPDATE` va aggiunta anche alla `GRANT`, altrimenti la scrittura fallisce
con `permission denied` — ma fallisce **nei test di integrazione**, che girano come
`propflow_app` con le stesse concessioni. Il precedente esiste ed è verde:
`the_application_role_cannot_repoint_a_lease_party_to_another_lease` in
`crates/infrastructure/tests/persistence.rs`.

**Il service layer restringe comunque.** I privilegi rendono impossibile l'incoerenza
strutturale; non dicono nulla su chi possa chiudere un ticket o valorizzarne il costo,
perché `status` e `actual_cost` restano concesse — devono esserlo, è il lavoro
dell'applicazione. Quella parte resta autorizzazione applicativa, transizione per
transizione.

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


<a id="esito-segnalazioni-fetta-1"></a>

### Esito — che cosa l'esecuzione ha confermato, cambiato e trovato

Scritto il **29 agosto 2026**, a fetta conclusa. I tredici passi sono stati eseguiti
tutti, **compreso il passo 11**: la fetta non si è rivelata troppo grande e il punto di
taglio non è servito. Le trappole si sono presentate come descritte tranne una, che si
è rivelata **più grave** di come il piano la immaginava; ne sono emerse tre nuove.

#### Le trappole, verificate una per una

| Trappola del piano | Si è presentata? |
|---|---|
| `reference` la assegna il database, e nessun repository sa ancora farlo | **Sì, e peggio.** La via indicata — `RETURNING reference` — **non funziona sotto RLS**. Vedi la trappola nuova 1 |
| La sequenza è globale, non per organizzazione | Sì. `TK-000001` sul database di test appena migrato, e la sequenza è una sola per il cluster. Registrato fra i [debiti](#debiti-tecnici-noti) |
| Non esiste alcun vincolo fra `room_id`, `lease_id` e `property_id` | **Sì, riprodotto su database reale.** Un `INSERT` diretto con la stanza di un altro immobile è passato senza una parola. Il controllo sta in `OpenTicket`, una volta sola, e un test di persistenza più uno di contratto lo fissano |
| `ticket_messages` non ha una policy di `UPDATE` | Sì. Verificato interrogando `pg_policy`: due policy, `_select` e `_insert`. Il test `a_thread_message_can_never_be_rewritten` fallisce se qualcuno ne aggiunge una terza, perché quella è la proprietà su cui il thread si regge |
| La `WITH CHECK` di `tickets_update` vincola solo `org_id` | Sì. `pg_get_expr(polwithcheck, …)` restituisce esattamente `(org_id = current_org_id())`. La migration `0020` e [ADR-026](08-decisioni.md#adr-026) sono la risposta |
| L'onere di manutenzione non è un rischio silenzioso | Sì, e con una guardia in più: oltre ai test di integrazione che girano come `propflow_app`, il **controllo 12** delle invarianti dello schema verifica che le nove colonne strutturali restino non aggiornabili. Verificato concedendo `UPDATE (title)` a mano: il controllo fallisce con `colonna-strutturale-scrivibile / tickets.title` |
| Il service layer restringe comunque | Sì. `status` e i due costi restano concesse, e chi possa chiudere o valorizzare resta autorizzazione applicativa, transizione per transizione |
| Lo storico lo scrive un trigger, e scriverlo anche noi lo raddoppia | **Sì, riprodotto.** Un `INSERT` diretto produce da sé la riga `NULL → open`, con `changed_by` nullo fuori da un contesto di richiesta. La porta non ha un metodo di scrittura sullo storico, per non invitare a farlo |
| `closed_at` non si azzera da solo alla riapertura | Sì. Lo azzera `Ticket::reopen`, e due test lo fissano: uno unitario e uno di persistenza che rilegge la riga dal database, non l'aggregato in memoria |
| Solo chi ha segnalato può riaprire | Sì, e ha prodotto un debito: al coinquilino l'API risponde `org.staff_role_required`, che è vero a metà. Registrato fra i [debiti](#debiti-tecnici-noti), insieme al gemello sui documenti |
| Una segnalazione senza `lease_id` è invisibile all'inquilino che ci abita | Non riprodotta a runtime, ma **letta nello schema** e confermata: `can_access_ticket` arriva al contratto solo se `lease_id` non è nullo. Non è codice ma un avviso a chi userà l'API, ed è finito nella documentazione del campo — nel DTO, nel comando e in `docs/05-api.md` |
| `search_vector` è `GENERATED ALWAYS` | **Non si è presentata, e non poteva.** La colonna non compare in nessuna query, nemmeno in lettura: la trappola scatta solo per chi elenca «tutti i campi», e il commento sulla macro `select_ticket!` è lì per quella persona |
| `tickets_open_ix` è parziale sui sei stati di lavorazione | Sì, e ha deciso la forma della query. Vedi la scelta sul letterale SQL più sotto |

#### La trappola che si è rivelata più grave: `RETURNING` sotto Row Level Security

Il piano diceva che «la `INSERT` deve usare `RETURNING reference`» e lasciava aperta,
come alternativa, «leggere la riga dopo l'inserimento». **La prima via non esiste.**

Un `INSERT … RETURNING` applica alla riga restituita anche la **policy di `SELECT`**,
oltre alla `WITH CHECK` dell'`INSERT`. `tickets_select` chiama
`app.can_access_ticket(id)`, che è dichiarata `STABLE`: vede lo snapshot preso
all'inizio dello **statement**, cioè prima che la riga esistesse. La funzione risponde
`false`, la policy di SELECT non passa, e PostgreSQL rifiuta con

```
ERROR: new row violates row-level security policy for table "tickets"
```

che è il messaggio della `WITH CHECK` e manda a cercare nel posto sbagliato: si
controlla la policy di `INSERT`, la si trova corretta, e non si sospetta la `SELECT`.
Isolato con due `INSERT` identici nella stessa transazione, uno con `RETURNING` e uno
senza: il secondo passa.

La rilettura in uno **statement separato** della stessa transazione funziona, perché lì
`can_access_ticket` prende uno snapshot nuovo che comprende la riga appena scritta.
Verificato: `SELECT reference FROM app.tickets WHERE id = $1` subito dopo l'`INSERT`
restituisce `TK-000011`.

Il repository fa così. Costa un round-trip in più per apertura, ed è l'unica via che la
policy lascia aperta senza toccarla. **Nessuna decisione del piano cambia:** il
riferimento lo assegna comunque il database, `Ticket::open` restituisce comunque un
aggregato con `reference` a `None`, e il porto restituisce comunque il valore a chi
chiama. Cambia il meccanismo, che era la seconda delle due alternative che il piano
stesso elencava.

**Conseguenza generale, buona da ricordare:** ogni futura `RETURNING` su una tabella
con una policy di `SELECT` che dipende dalla riga stessa incontrerà lo stesso muro. È
il caso di tutte le tabelle il cui `can_access_*` interroga la tabella filtrata.

#### Le altre due trappole nuove

**1. Lo storico non si ordina per `id`.** `ticket_status_history.id` è
`gen_random_uuid()`, cioè UUID **v4**: a differenza di ogni altro identificatore del
progetto non porta con sé il tempo, e ordinare per id darebbe un ordine casuale. La
query ordina per `created_at`. È il genere di dettaglio che non fallisce mai in un test
con tre righe scritte a un millisecondo di distanza, e che si nota quando la catena
degli stati appare mescolata in produzione.

**2. `updated_at` non va nella `GRANT`, e non va nella `UPDATE`.** Ogni tabella con
quella colonna ha un trigger `set_updated_at` `BEFORE UPDATE` (migration `0013`), e un
trigger non è soggetto ai privilegi di colonna del chiamante. Nominare `updated_at`
nella `SET` avrebbe richiesto di concederla, allungando la lista senza motivo — e
avrebbe reso meno evidente che l'elenco concesso è *esattamente* ciò che
l'applicazione decide.

#### Scelte prese eseguendo, che il piano lasciava aperte

- **`close` e `cancel` sono ammesse da tutti e sei gli stati di lavorazione**, non
  dal solo `in_progress` che il diagramma suggerisce. Una segnalazione che si rivela
  un falso allarme si chiude senza inventare un avanzamento che non c'è stato, e
  `cancelled` resta per il caso diverso in cui la richiesta viene ritirata. L'insieme
  ammesso coincide con `TicketStatus::WORKING`, cioè con il predicato di
  `tickets_open_ix`: la coda di lavorazione e «ciò da cui si può ancora uscire» sono
  la stessa cosa, ed è una coerenza che vale la pena avere.
- **`assign` è ammessa anche da `open` e da `assigned`.** Da `open` perché una
  segnalazione ovvia si assegna senza passare dal triage, e obbligare a classificarla
  prima produrrebbe classificazioni finte; da `assigned` perché è una riassegnazione,
  che è un fatto normale.
- **`TicketCostsUpdated` è un quinto evento**, che il piano non nominava. Senza, il
  `PATCH` sui costi sarebbe l'unica modifica del progetto priva dell'evento che la
  descrive, e romperebbe la proprietà su cui l'outbox si regge. Porta i **nomi dei
  campi** e non i valori, come `PropertyUpdated`.
- **L'assegnazione emette due eventi**, `TicketStatusChanged` e `TicketAssigned`. Il
  primo da solo non direbbe a chi il lavoro è finito in mano; il secondo da solo
  farebbe perdere l'assegnazione a chi consuma le transizioni. La funzione condivisa
  delle transizioni emette sempre il primo, così nessuna può dimenticarlo.
- **Le sette transizioni sono sette casi d'uso con una sequenza sola.** Ciò che le
  distingue è l'autorizzazione e la chiamata al dominio, non l'ordine dei passi:
  replicare sette volte «autorizza → carica → applica → persisti → evento → commit»
  darebbe sette occasioni di sbagliarne uno, e la sequenza è precisamente ciò che non
  deve variare. Vivono in `transition.rs` attorno a una funzione privata `run`.
- **`resolution_notes` resta fuori perimetro.** Il piano non la nominava né fra ciò
  che è dentro né fra ciò che è fuori. Sta fuori perché la chiusura non ha un corpo, e
  aggiungerla avrebbe voluto dire decidere chi può scriverla e se sia visibile
  all'inquilino — due domande che nessuno aveva posto.
- **La valuta è fissata a `EUR` alla creazione e non è modificabile.** L'`INSERT` non
  nomina `currency`, che resta al default dello schema, e la colonna non è nella
  `GRANT`. Entrambi i costi la condividono, perché la colonna è una sola: due importi
  in valute diverse non sarebbero rappresentabili.
- **`ticket.cancelled` è un codice a sé**, distinto da `ticket.invalid_transition`.
  Non è una transizione rifiutata ma una modifica su un aggregato che non ne accetta
  più, e il rimedio per il client è diverso — non «riprova da un altro stato» ma
  «questa segnalazione è finita».
- **I sei stati della coda esistono come letterale SQL**, e non come parametro. Il
  planner può usare un indice parziale solo se dimostra che il predicato della query
  implica quello dell'indice, e con un array passato come parametro non può: al
  momento del piano non ne conosce il contenuto. Il prezzo è che l'elenco vive in due
  posti — `TicketStatus::WORKING` e la macro `working_statuses!` — e il test
  `the_sql_literal_and_the_domain_constant_agree` li tiene insieme.
- **Il thread è paginato per `id`** e non per `created_at`, così il cursore resta un
  solo UUID come in ogni altro elenco dell'API. Gli id sono UUIDv7, quindi l'ordine
  coincide con quello di creazione; il prezzo è che `ticket_messages_thread_ix` serve
  l'uguaglianza su `ticket_id` ma non l'ordinamento, che resta un sort su un insieme
  limitato da quanto due persone si scrivono.
- **Il controllo 12 delle invarianti ha un nome proprio**,
  `colonna-strutturale-scrivibile`, distinto dal `colonna-di-confine-scrivibile` del
  controllo 11. Il criterio è diverso — qui non c'è un confine da difendere — e
  confonderli renderebbe illeggibile la ragione di entrambi.
- **`AttachmentOwnerType` nel dominio passa da tre valori a quattro.** Il test degli
  enum che verificava «un `ticket` in lettura dà errore» ora verifica lo stesso su
  `utility_bill`: la regola non è cambiata, si è spostato il confine del perimetro.

#### Come si è verificato

`./ops/ci/test-integration.sh` verde con **204 test** (erano 155), contando anche le
quattro decisioni prese subito dopo: 23 unitari di dominio in più sulla macchina a
stati e sui value object, 5 applicativi sull'autorizzazione delle transizioni e sulla
divulgazione dei costi, 1 di infrastruttura sul letterale SQL, 10 di persistenza e 10
di contratto HTTP.

Il test `the_relationship_runs_in_both_directions` percorre il giro che il piano
descriveva a parole: l'inquilino apre una segnalazione sul proprio immobile con una
foto allegata, il proprietario la classifica e la assegna, si scambiano due messaggi,
il proprietario scrive una nota interna che **l'inquilino non vede** — mentre lui la
vede, ed è lo stesso thread — registra il costo sostenuto e chiude; l'inquilino riapre
perché il guasto torna, e lo storico mostra la catena intera compresa la riapertura,
con la prima riga senza stato di partenza.

Su database reale, oltre alla suite: riprodotto che lo schema accetta una segnalazione
con la stanza di un altro immobile; che il trigger scrive `NULL → open` da sé; che
`propflow_app` non ha più `UPDATE` a livello di tabella su `app.tickets` e ne ha dieci
a livello di colonna, esattamente quelle che il repository scrive; che il controllo 12
fallisce se una colonna strutturale torna concessa; e che `INSERT … RETURNING` è
rifiutato mentre lo stesso `INSERT` senza `RETURNING` passa.

**E a mano, contro il binario vero, PostgreSQL vero e MinIO vero** — che è la parte che
i test di contratto non coprono, perché montano il router in-process con un doppio in
memoria dello storage. Quarantasette richieste, nessun `5xx`, nessun panic, nessun
`WARN`; i soli quattro `404` sono gli accessi dell'estraneo, che devono darlo.

Il giro: l'inquilino apre la segnalazione — `TK-000001`, la prima della sua
organizzazione — e vi allega una foto JPEG di 6822 byte. `checksum_sha256` nella
risposta coincide con lo `sha256sum` dell'originale, perché lo calcola l'API sui byte
ricevuti e non lo dichiara il client. Il download segue il `307` fino a MinIO e torna
identico byte a byte. Nel bucket la chiave è `{org_id}/{document_id}`: `mc ls` non
trova né `boiler` né `.jpg`, cioè il nome del file non è finito nei log del fornitore
di storage.

Poi il rapporto nei due versi: triage, assegnazione, avvio, sospensione
`waiting_tenant`, ripresa, due messaggi, una nota interna che nel thread del
proprietario compare e in quello dell'inquilino no. Costi: il proprietario vede
`stima=200.00 sostenuto=180.00`, l'inquilino `stima=None sostenuto=180.00` sulla
**stessa** segnalazione. Chiusura con la nota di risoluzione, che l'inquilino legge;
riapertura fatta da lui, dopo la quale `closed_at` e la nota sono spariti entrambi. Lo
storico mostra le otto righe della catena, la prima senza stato di partenza.

Un'organizzazione nuova riparte da `TK-000001` mentre la prima è ferma a uno: due
contatori indipendenti, verificati anche in tabella. L'estraneo riceve `404` su
dettaglio, thread, allegati e contenuto del documento, e l'elenco vuoto. L'outbox ha
ricevuto 7 `ticket.status_changed` — uno per transizione — più `ticket.opened`,
`ticket.assigned`, `ticket.costs_updated` e 3 `ticket.message.posted`, e **nessun
payload contiene** «boiler», «gonfiato», «valvola» o «idraulico»: il testo libero non
viaggia. `audit_log` ha registrato un `insert` e otto `update` sulla segnalazione.

Infine, sul database vivo: `UPDATE app.tickets SET property_id = …` e
`SET title = …` come `propflow_app` danno entrambi `permission denied for table
tickets`, e l'inserimento nel contatore dà `new row violates row-level security
policy`.

#### Cosa resta fuori, e resta fuori apposta

Triage AI, ricerca full-text, SLA, programmazione, allegati sui singoli messaggi, ruolo
`manager` esposto, note di risoluzione. I primi sei erano già dichiarati fuori
perimetro dal piano; l'ultima è una scelta presa eseguendo, ed è motivata qui sopra.

Le due cose che la fetta aveva lasciato come debiti — la sequenza dei riferimenti
globale e il `403` che un coinquilino riceve provando a riaprire — **non sono più
debiti**: sono state decise e chiuse subito dopo, e il resoconto è qui sotto.


<a id="decisioni-dopo-la-fetta-segnalazioni"></a>

### Decisioni prese subito dopo la fetta

Prese il **29 agosto 2026**, a fetta consegnata, sulle domande che l'esecuzione aveva
lasciato aperte e sui due debiti che aveva creato. Implementate sullo stesso ramo, con
la migration `0021`.

#### 1. Il riferimento si numera per organizzazione

Era un debito, ed è stato chiuso subito per la forma del suo costo e non per la sua
gravità: finché nessuno ha dati reali è una migration su tabelle vuote, dopo sarebbe
rinumerare o convivere con due formati per sempre. Un contatore per `org_id` riempito
da un trigger, e `tickets_reference_uk` che passa da `(reference)` a
`(org_id, reference)`. Motivazione completa, alternative scartate e conseguenza sulla
concorrenza in [ADR-027](08-decisioni.md#adr-027).

**Conseguenza sul contratto pubblico:** il riferimento non è più un identificatore
globale. Serve l'`id` per raggiungere una segnalazione, e un client che indicizzasse
per `reference` senza l'organizzazione fonderebbe righe diverse.

#### 2. L'inquilino vede il costo sostenuto, non la stima

`actual_cost` sì: chi ha subito il guasto sa quanto è costato ripararlo.
`estimated_cost` no, ed è `null` per chi non è staff anche quando un valore c'è — è la
cifra di una trattativa con un fornitore e non riguarda chi abita la casa.

Prima di questa decisione l'inquilino vedeva **entrambi**, e non per scelta: i campi
erano nel DTO e la Row Level Security lascia leggere la riga a chi ha segnalato.
Verificato interrogando l'API prima di cambiarla, non dedotto dal codice.

La regola vive in `authorization::discloses_estimated_cost`, cioè nel service layer,
dove sta l'autorizzazione; il bordo HTTP la applica alla proiezione e non la decide.
**Non è un confine di sicurezza:** la riga resta leggibile, e renderlo un confine vero
richiederebbe privilegi di colonna in `SELECT` o una policy separata. La direzione è
deliberata — aggiungere un campo a una risposta è additivo, toglierlo è una rottura —
ed è la ragione per cui si è partiti dalla scelta più chiusa delle due praticabili.

#### 3. Un coinquilino non riapre, e il codice glielo dice per quello che è

Il comportamento non cambia: riaprire spetta a chi ha segnalato, perché la `USING` di
`tickets_update` non passa dal contratto, e allargarlo sarebbe una migration sulla
policy e non un ramo applicativo.

Cambia il codice. `ticket.not_yours` al posto di `org.staff_role_required`: al
coinquilino non manca un ruolo — nessun ruolo gli darebbe questa operazione — gli manca
l'essere stato lui a segnalare, e i due rimedi sono diversi. Aggiungere un codice è
additivo.

`org.staff_role_required` resta invece **corretto** in un caso vicino e diverso: chi ha
segnalato prova ad annullare una segnalazione già in lavorazione. Lì il ruolo di staff
serve davvero, perché annullarla butterebbe via il lavoro di qualcun altro.

Il debito gemello sui documenti (`org.staff_role_required` all'archiviazione da parte
di chi non ha caricato) **resta aperto**: è la stessa conversazione, ma toccarlo qui
avrebbe voluto dire cambiare la tassonomia di un modulo chiuso per un'occasione.

#### 4. Chiudere registra come è stata risolta

`POST /tickets/{id}/close` accetta `resolution_notes`, facoltativo, visibile a chiunque
veda la segnalazione. Senza, l'unica traccia di cosa fosse stato fatto era il thread —
una conversazione, non una conclusione: a distanza di mesi la risposta a «com'è stato
riparato il boiler» andava dedotta rileggendo venti messaggi.

`resolution_notes` è ora nella `GRANT` dei privilegi di colonna (migration `0021`) e
**non** è fra le colonne strutturali del controllo 12: è deliberatamente scrivibile.

**Una riapertura la cancella**, insieme a `closed_at` e `resolved_at`. La regola è una
sola e si spiega in una riga — riaprire disfa la chiusura, e la nota fa parte della
chiusura — e l'alternativa lascerebbe una segnalazione `open` che dichiara come è stata
risolta, cioè una riga che dice il falso sul proprio stato in una risposta che
l'inquilino legge. Se quel testo deve sopravvivere alla riapertura, il posto è il
thread, che non si riscrive mai. È una perdita di informazione **scelta**, non subita.

#### 5. Le tre conferme, che non hanno cambiato codice

- **Si chiude da tutti e sei gli stati di lavorazione.** Confermato: un falso allarme
  si chiude senza inventare un avanzamento che non c'è stato, e l'insieme ammesso
  coincide con il predicato di `tickets_open_ix`.
- **La valuta resta `EUR` e non è dichiarabile.** Rinviato al modulo delle bollette e
  degli addebiti, dove il problema si pone davvero — non è rimasto indietro, è stato
  spostato dove appartiene.
- **Nessun filtro «assegnate a me».** Rinviato a quando esisterà un client che lo
  consumi: senza, sarebbe un filtro da mantenere e non da usare. L'indice
  `tickets_assignee_ix` esiste già e aspetta, e aggiungerlo dopo è additivo.

---

<a id="piano-bollette-e-addebiti-fetta-1"></a>

## Piano — Bollette e addebiti, fetta 1

Pianificato il **30 agosto 2026**, **implementato il 31 agosto 2026**. Il piano resta
sotto come è stato scritto: serve a eseguirlo, e serve a capire perché il codice è
com'è. L'esito è in fondo, dopo le trappole.

### Obiettivo

Il flusso che il proprietario ha già in mano ogni mese: arriva una bolletta, la
registra, la controlla, ne riaddebita all'inquilino la parte che gli spetta, e vede
chi è indietro con i pagamenti. È il primo modulo **contabile** del progetto, e la
prima volta che l'applicazione scrive una cifra che qualcuno dovrà versare a
qualcun altro.

Chiude anche le due domande che il modulo delle segnalazioni ha rinviato qui per
nome: la valuta dichiarabile e il costo di una riparazione che diventa un addebito.
Entrambe hanno una risposta, ed entrambe le risposte sono «no», con la ragione
scritta — vedi le decisioni 6 e 7.

**Dentro:** registrazione manuale di una bolletta su un immobile, correzione finché
non è validata, validazione, riaddebito a un contratto (che crea l'addebito),
annullamento; creazione diretta di un addebito su un contratto, elenco e dettaglio
con il **saldo** letto da `app.v_charge_balances`, annullamento e rinuncia
all'incasso; allegati su bollette e addebiti, che riusano gli use case dei documenti.

**Fuori, e non per dimenticanza:**

- **l'OCR** (`ocr_extracted`, `ocr_confidence`, `ai_job_id`, `document_id`,
  `pending_review`): è il punto 4 della lista dei moduli, e i trait dei provider
  vanno definiti allora. In questa fetta la bolletta la digita una persona, le
  colonne restano ai loro default e lo stato `pending_review` non lo scrive nessuno.
  Lo schema tiene separato ciò che ha letto la macchina da ciò che ha confermato una
  persona (`validated_by`, `validated_at`) proprio perché si possa misurare
  l'accuratezza dopo: riempire le prime con valori umani distruggerebbe quel dato
  prima di raccoglierlo;
- **i pagamenti** (`app.payments`): sono il modulo successivo, ed è la ragione per
  cui `paid` e `partially_paid` restano stati che l'applicazione **non scrive mai**.
  Ne segue una conseguenza da dire ad alta voce e da documentare nell'API:
  `paid_amount` vale sempre `0.00` in questa fetta, e `outstanding_amount` coincide
  sempre con `amount`. Non è un difetto della vista, è l'assenza di righe in
  `app.payments`;
- **il pagamento della bolletta al fornitore** (`bill_status = 'paid'`): non è
  modellato affatto — `app.payable_type` è `('charge','expense_share','settlement')`
  e **non comprende `utility_bill`**, quindi non esiste un posto dove registrare che
  il proprietario ha pagato l'utenza. Lo stato resta nell'enum e nessuno lo scrive;
- **la contestazione al fornitore** (`disputed`): manca la via di ritorno. Uno stato
  in cui si entra e da cui non si esce è peggio di nessuno stato, e disegnare
  l'uscita significa decidere cosa succede alle bollette già riaddebitate. Nessuno
  ha posto la domanda;
- **la generazione ricorrente dei canoni** (`generation_key`): richiede uno
  scheduler, che [ADR-019](08-decisioni.md#adr-019) ha deliberatamente evitato di
  introdurre. La colonna resta nulla; l'indice unico `charges_generation_uk` è ciò
  che rende additivo aggiungerla il giorno in cui un client o uno scheduler la
  useranno;
- **la ripartizione fra coinquilini** di una stessa bolletta: è il modulo delle spese
  co-living (`expenses`, `expense_shares`), che ha il vincolo differito a garantire
  che le quote sommino esattamente. Qui una bolletta si riaddebita **a un contratto
  per volta**, e riaddebitarla a più contratti in parti è possibile ripetendo
  l'operazione — senza che nulla verifichi che le parti sommino al totale, vedi la
  decisione 5;
- **la dashboard** (`v_owner_dashboard`): è il punto 5 della lista. La vista esiste e
  già conta `total_outstanding`, `overdue_charges` e `bills_pending_review`; nessun
  endpoint la legge.

### Decisioni prese, con il perché

#### 1. L'addebito nasce **emesso**, e `draft` non si scrive mai

`app.charge_status` ha `draft`, e la tentazione è il flusso in due passi — si prepara
l'addebito, poi lo si emette. Non regge, per due ragioni che si sommano.

La prima è che **una bozza sarebbe già visibile all'inquilino**. La policy
`charges_select` è `org_id = current_org_id() AND app.can_access_lease(lease_id)`: non
guarda lo stato. Un addebito in bozza comparirebbe nell'elenco di chi deve pagarlo
esattamente come uno emesso, e sarebbe una «bozza» solo nel nome.

La seconda è che **emettere non manda niente a nessuno**: la porta `Notifier` è
dichiarata e senza adattatore. Fra i due stati non ci sarebbe alcuna differenza
osservabile — nessun invio, nessuna visibilità diversa, nessun effetto sul saldo, che
la vista calcola su `amount` senza guardare `status`.

Quindi: l'addebito nasce `issued` con `issued_at` valorizzato, e un addebito
sbagliato **si annulla** — `void` esiste per questo. È la stessa forma della
segnalazione aperta sull'immobile sbagliato, che si annulla e si riapre invece di
essere spostata.

**Cosa costerebbe cambiare idea.** Introdurre una bozza vera richiede *due*
migration, non una: restringere `charges_select` con `status <> 'draft' OR
app.is_org_staff(org_id)` **e** correggere `app.can_access_attachment`, che per
`charge` risolve su `can_access_lease` e lascerebbe quindi vedere all'inquilino gli
allegati di un addebito che non può vedere. È additivo — l'inquilino vedrebbe meno
righe, non di più — ma va fatto in due punti, e questo è il posto dove sta scritto
quali sono.

#### 2. Lo stato **scaduto si deriva**, non si memorizza

`app.charge_status` ha `overdue`, e nessuno lo scriverà. Un addebito scaduto è un
addebito `issued` la cui `due_date` è passata e il cui residuo è positivo: è una
**conseguenza**, e le conseguenze si derivano. È esattamente
[ADR-019](08-decisioni.md#adr-019), che ha già stabilito la regola per i contratti
dopo averla pagata: lo stato memorizzato invecchia da solo perché nessuno lo
aggiorna, e la dashboard finisce per contare cose finite mesi prima.

La definizione vive **in un posto solo**, e quel posto è la vista:
`v_charge_balances.is_past_due` è `due_date < CURRENT_DATE AND amount > paid_amount`.
La risposta dell'API porta due stati, come i contratti:

- `status`, quello memorizzato, che cambia solo su azioni esplicite;
- `effective_status`, che vale `overdue` quando `is_past_due` è vero mentre `status`
  resta `issued`.

Il dominio ripete la formula in `Charge::is_past_due(today)` per poterla verificare
senza database — stessa coppia di `app.lease_in_force` e `Lease::is_in_force`, e
stesso obbligo di rispondere la stessa cosa.

Conseguenza sul filtro dell'elenco: `?status=overdue` risponde **422** indicando
`?outstanding=true`, come `?status=expiring` sui contratti. Restituire una pagina
vuota sarebbe il modo peggiore di dire che quello stato in database non c'è.

#### 3. Un addebito emesso **non si riscrive**: privilegi di colonna su `app.charges`

Un addebito è una **pretesa verso una persona**. Riscriverne l'importo dopo averla
comunicata non è una correzione, è cambiare le carte in tavola: la traccia resta in
`audit_log`, ma la riga che l'inquilino rilegge dice un'altra cifra senza che nulla,
nella riga stessa, ricordi la prima.

Lo schema non lo impedisce. `charges_update` ha `USING` e `WITH CHECK` identici —
`org_id = app.current_org_id() AND app.is_org_staff(org_id)` — e una `WITH CHECK` che
nomina il solo `org_id` **lascia libera ogni altra colonna**: è la stessa
constatazione di [ADR-021](08-decisioni.md#adr-021) e di
[ADR-026](08-decisioni.md#adr-026), e vale qui identica, perché una policy non può
confrontare la riga nuova con quella vecchia.

Rientra nel **secondo criterio** dichiarato da ADR-026 — «una colonna non ha un motivo
legittimo di cambiare e lo schema non ha un vincolo che ne difenda la coerenza» — e
qui il criterio produce una lista corta come nessun'altra:

```sql
REVOKE UPDATE ON app.charges FROM propflow_app;
GRANT  UPDATE (status) ON app.charges TO propflow_app;
```

**Una colonna sola.** Tutto il resto — `lease_id`, `charge_type`, `amount`,
`currency`, `due_date`, `period_start`, `period_end`, `description`,
`utility_bill_id`, `generation_key`, `issued_at`, `document_id`, `notes`,
`deleted_at` — descrive la pretesa nel momento in cui è stata fatta, e cambiare
significa farne un'altra: si annulla e si riemette. `updated_at` non compare perché
la scrive il trigger `set_updated_at`, che è `BEFORE UPDATE` e non è soggetto ai
privilegi di colonna del chiamante.

Il modulo dei pagamenti non avrà bisogno di altro: riconciliare un incasso porta
l'addebito a `partially_paid` o `paid`, cioè scrive `status`, che è concessa.

**Conseguenza da dire ad alta voce: non esiste un `PATCH` sugli addebiti.** Prorogare
una scadenza — «ti do una settimana in più» — oggi si fa annullando e riemettendo. È
scomodo ed è voluto: è il punto in cui ci si ferma a chiedersi se `due_date` debba
davvero essere scrivibile, e il giorno in cui la risposta sarà sì è una migration
nuova, una riga.

#### 4. Sulle bollette lo strumento **non** si usa, e la ragione è la lunghezza della lista

Sarebbe l'automatismo naturale — la tabella accanto, lo stesso difetto nella
`WITH CHECK` — e sarebbe sbagliato. Una bolletta si **digita**, dodici campi alla
volta, e finché non è validata correggerne uno è il lavoro previsto, non un'anomalia.
La `GRANT` dovrebbe quindi nominare quasi ogni colonna, e **un elenco che contiene
quasi tutto non difende niente**: si paga l'onere di manutenzione — ogni colonna
aggiunta a una `UPDATE` va aggiunta anche là — senza comprare la proprietà per cui
l'onere esisteva.

L'immutabilità della bolletta è quindi un'invariante di **dominio** e non un
privilegio: `UtilityBill::apply` rifiuta le modifiche da `allocated` in poi. Ed è la
prima volta che ADR-026 viene applicato e dà «no»: il criterio serve a decidere, non
a estendersi da sé.

#### 5. Correggere una bolletta validata **disfa la validazione**

Il `PATCH` è ammesso da `draft` e da `validated`; su una bolletta validata riporta lo
stato a `draft` e azzera `validated_at` e `validated_by`. Non è un effetto
collaterale: validare è l'affermazione «questi numeri sono quelli», e cambiare i
numeri la smentisce — chi la valida di nuovo deve rifarlo apposta. È la stessa regola
per cui riaprire una segnalazione cancella `closed_at` e la nota di risoluzione.

Da `allocated` in poi la bolletta è **ferma**: quei numeri sono la base di una pretesa
già fatta a qualcuno, e cambiarli sotto la pretesa è precisamente ciò che non deve
succedere.

**Alternativa scartata: `PATCH` solo in bozza, e correzione per annullamento e
reinserimento.** Sembra più semplice e sbatte contro `utility_bills_dedup_uk`, che è
unico su `(org_id, supplier_vat, invoice_number)` e — attenzione — è parziale su
`deleted_at IS NULL`, **non** su `status <> 'void'`. Una bolletta annullata
continuerebbe quindi a occupare la chiave, e la stessa fattura non si potrebbe
reinserire mai più. Vedi la trappola dedicata.

#### 6. Annullare una bolletta la **cancella**, annullare un addebito no

L'asimmetria è deliberata e ha una ragione sola.

Un addebito annullato **resta visibile**: qualcuno si è visto chiedere quella somma, e
deve poter vedere che la richiesta è caduta. `status = 'void'`, `deleted_at` intatto.

Una bolletta annullata **sparisce**: `status = 'void'` **e** `deleted_at = now()`. Non
c'è nessuno a cui dirlo — è un documento nostro inserito male — e c'è invece una
ragione tecnica per liberare la riga: `utility_bills_dedup_uk` è parziale su
`deleted_at IS NULL`, quindi senza il soft delete la stessa fattura non sarebbe più
reinseribile. La traccia di ciò che è stato annullato resta in `audit_log`, che è
append-only.

#### 7. Il riaddebito è una **sotto-risorsa della bolletta**

`POST /api/v1/utility-bills/{id}/recharge` crea l'addebito, lo collega
(`charges.utility_bill_id`) e porta la bolletta ad `allocated`, tutto nella stessa
transazione e con due eventi. L'alternativa — accettare `utility_bill_id` nel corpo di
`POST /api/v1/leases/{id}/charges` — nasconderebbe in una creazione un effetto su un
altro aggregato, che è lo stesso motivo per cui le transizioni delle segnalazioni sono
sotto-risorse e non un `PATCH` sullo stato. `POST /leases/{id}/charges` quindi **non
accetta** `utility_bill_id`: c'è una via sola perché una bolletta e un addebito si
leghino, ed è quella che tiene onesto lo stato della bolletta.

Il riaddebito è ammesso da `validated` **e da `allocated`**: riaddebitare una bolletta
in parti a più contratti — il caso del co-living con contratti per stanza — è una
ripetizione dell'operazione, come riassegnare una segnalazione già assegnata.

**Nessuno verifica che le parti sommino al totale**, e non è una dimenticanza. Lo
schema non ha un vincolo; un controllo applicativo sarebbe soggetto a race condition
fra due riaddebiti concorrenti, che è esattamente il criterio di
[ADR-015](08-decisioni.md#adr-015) per cui quel genere di invariante sta nel database o
non sta da nessuna parte; e il posto in cui «le parti sommano» è già espresso
correttamente è il `CONSTRAINT TRIGGER … DEFERRABLE` di `expense_shares`, cioè il
modulo co-living. Riaddebitare più del totale è oggi possibile, ed è registrato qui
invece che scoperto dopo.

#### 8. La valuta resta **una sola**, e la ragione non è l'importo: è la vista dei saldi

È la domanda che i ticket hanno rinviato qui per nome, e la risposta è no — con una
motivazione che nel modulo delle segnalazioni non si poteva ancora vedere.

`v_charge_balances` calcola `amount - SUM(payments.amount - refunded_amount)` **senza
confrontare `currency`**, e `v_owner_dashboard.total_outstanding` somma
`outstanding_amount` di tutta l'organizzazione allo stesso modo. Bastano due valute
perché quei due numeri diventino silenziosamente falsi: non un errore di
arrotondamento, una somma di grandezze diverse. `Money` non implementa `PartialOrd`
proprio per rendere impossibile quel confronto nel dominio ([ADR-002](08-decisioni.md#adr-002));
le viste lo fanno lo stesso, perché il SQL non ha quel tipo.

Il costo di rendere la valuta dichiarabile non sta quindi nella colonna — c'è già, con
il suo default — ma nelle **due viste** e nella decisione su cosa significhi un totale
insoluto multi-valuta. È una migration e una scelta di prodotto, non un campo in un
DTO. Fino ad allora `currency` non compare in nessuna `INSERT` né in nessuna `UPDATE`,
resta al default `EUR`, ed è fra le colonne che il controllo delle invarianti verifica
non essere scrivibili.

#### 9. Il costo di una riparazione **non** diventa un addebito

L'altra domanda rinviata dai ticket. La risposta è no, e la ragione sta nello schema
prima che nel prodotto: **`app.charges` non ha una colonna per la segnalazione**.
`utility_bill_id` c'è, con il suo indice; `ticket_id` no. Un addebito generato da una
riparazione arriverebbe all'inquilino come una riga di testo libero, e la domanda «per
cosa sono questi 180 euro» resterebbe senza risposta interrogabile — esattamente dove
serve, perché è la riga che qualcuno contesterà.

C'è anche una ragione che non è nostra: in una locazione italiana la manutenzione
ordinaria è a carico dell'inquilino e la straordinaria del proprietario, e quella
distinzione il modello non la rappresenta. Deciderla di striscio, dentro un endpoint,
significherebbe deciderla per sempre.

Quindi: `charge_type = 'maintenance'` esiste, e un proprietario che voglia riaddebitare
una riparazione **crea l'addebito a mano** scrivendo nella descrizione a cosa si
riferisce. Il giorno in cui il legame debba diventare di prima classe è una migration
che aggiunge `charges.ticket_id` e il suo indice, e quello è il momento in cui si
decide anche la questione ordinaria/straordinaria.

#### 10. Il saldo si **legge** dalla vista, non si ricalcola

Le letture — elenco e dettaglio — passano da `app.v_charge_balances`, che è
`security_invoker` e quindi applica la RLS del chiamante. La definizione di «pagato»
resta una sola: il giorno in cui il modulo dei pagamenti aggiungerà uno stato
all'array `('succeeded','partially_refunded')`, cambierà in un posto e non in due.

Copiare la `LEFT JOIN LATERAL` della vista dentro il repository sarebbe la seconda
definizione, e divergerebbe alla prima modifica. Il prezzo pagato è che `app.charges`
viene attraversata due volte quando la lettura serve anche le colonne che la vista non
porta (`description`, `period_*`, `issued_at`, `utility_bill_id`, `notes`): la porta ha
quindi **due metodi di lettura distinti** — `find_by_id`, che legge la sola tabella ed
è ciò che usano le transizioni, e `find_balance_by_id` / `list_balances`, che uniscono
la vista e sono ciò che usa il bordo HTTP. Le scritture non pagano un join che non
useranno.

#### 11. Gli allegati riusano `AttachmentOwner`, e chiedono **una correzione** alla RLS

`app.attachment_owner_type` ha già `utility_bill` e `charge`, e
`app.can_access_attachment` li risolve già: è il dividendo di aver fatto i documenti
per primi, come lo è stato per le segnalazioni. Due varianti d'enum di dominio e
quattro rotte, senza toccare gli use case dei documenti.

Ma **`can_access_attachment` per `utility_bill` è più largo della policy della
bolletta**, e finché nessuna bolletta aveva allegati la cosa non si vedeva. Risolve su
`app.can_access_property(b.property_id)`, mentre `utility_bills_select` chiede staff
**oppure** parte del contratto indicato. In un co-living con contratti per stanza,
`can_access_property` è vera per **ogni** inquilino dell'immobile: aprendo le rotte
degli allegati, il coinquilino A potrebbe scaricare la scansione della bolletta
imputata al contratto di B — la bolletta con il nome, l'indirizzo e i consumi di B —
senza poter vedere la riga della bolletta stessa.

La correzione è una migration che riallinea la funzione alla policy («il documento di
una bolletta si vede se si vede la bolletta»), ed è parte di questa fetta perché è
questa fetta ad aprire la porta. La conseguenza sui documenti già esistenti è nulla:
nessun documento ha oggi `owner_type = 'utility_bill'`.

#### 12. Gli stati che l'enum ha e l'applicazione non scrive

Riepilogo, perché è lungo e sparso: `bill_status.pending_review` (OCR),
`bill_status.paid` (pagamento al fornitore, non modellato),
`bill_status.disputed` (manca l'uscita), `charge_status.draft` (decisione 1),
`charge_status.overdue` (decisione 2), `charge_status.paid` e
`charge_status.partially_paid` (modulo pagamenti). Tutti vanno **comunque** nella
tabella di conversione di `string_enum!`: una riga scritta a mano potrebbe
contenerli, e un valore sconosciuto in lettura deve dare errore e non un default. È
la regola di `docs/06` §Aggiungere un enum, e qui riguarda sette valori su quindici.

### Decomposizione

Ogni passo è eseguibile e verificabile da solo, nell'ordine di `docs/06` §Aggiungere
un caso d'uso.

**Il punto di taglio è il passo 12**, se la fetta risulta troppo grande in esecuzione:
gli allegati sono additivi e chiusi — due varianti d'enum, una migration di
riallineamento e quattro rotte che riusano gli use case dei documenti senza toccarli.
Tagliato, resta un modulo completo. Tagliare più in alto non è un'opzione: il passo 11
è il bordo HTTP e senza quello non c'è niente da usare.

1. **Dominio: gli enum e i value object.** `UtilityType`, `BillStatus`, `ChargeType`,
   `ChargeStatus` come enum di dominio con le righe in `string_enum!`. Value object
   per ciò che ha limiti propri: `InvoiceNumber`, `SupplierVat` (testo libero a
   schema — il vincolo è nostro), `Consumption` — **non** un `Money`, perché
   `NUMERIC(14,4)` ha quattro decimali e `Money::new` ne rifiuta più di due. Il tetto
   di `description` e `notes` è nostro, come per la descrizione delle segnalazioni.
2. **Dominio: l'aggregato `UtilityBill`.** Campi privati, `register`, `apply`
   (rifiutata da `allocated` in poi, e da `validated` disfa la validazione),
   `validate`, `mark_allocated`, `void` (che valorizza anche `archived_at`/soft
   delete). Test unitari sulla macchina: le transizioni ammesse **e** quelle
   rifiutate.
3. **Dominio: l'aggregato `Charge`.** `issue` come unico costruttore — nasce `issued`
   con `issued_at` — più `void`, `write_off`, e `is_past_due(today)` che ripete la
   formula della vista. Nessun `apply`: un addebito emesso non si riscrive.
4. **Dominio: gli eventi.** `UtilityBillRegistered`, `UtilityBillUpdated` (nomi dei
   campi, non valori), `UtilityBillStatusChanged`, `ChargeIssued`,
   `ChargeStatusChanged`. **Nessun importo viaggia**: è la stessa regola di
   `TicketCostsUpdated` e di `RoomUpdated`, che non porta il canone. Nemmeno
   `supplier_name` e `supplier_vat`, che per un fornitore individuale nominano una
   persona.
5. **Porte di persistenza.** `UtilityBillRepository` (`find_by_id`, `list`, `insert`,
   `update`) e `ChargeRepository` (`find_by_id`, `find_balance_by_id`,
   `list_balances`, `insert`, `update`), con le due letture distinte della decisione
   10. `UtilityBillQuery` e `ChargeQuery` con cursore keyset. Nessun `soft_delete`
   sugli addebiti: si annullano, e l'annullamento è uno stato.
6. **Use case: le bollette.** `RegisterUtilityBill`, `UpdateUtilityBill`,
   `ValidateUtilityBill`, `VoidUtilityBill`, `ListUtilityBills`, `GetUtilityBill`.
   `ensure_staff` su tutto ciò che scrive, `ensure_member` sulle letture — la policy
   mostra all'inquilino la bolletta imputata al suo contratto, e *quali* siano sue lo
   decide lei, non un ramo applicativo sul ruolo. La registrazione verifica che
   `room_id` e `lease_id` appartengano all'immobile, leggendoli dai rispettivi
   repository: lo schema non lo verifica, vedi le trappole.
7. **Use case: gli addebiti.** `IssueCharge`, `VoidCharge`, `WriteOffCharge`,
   `ListCharges`, `GetCharge`. `IssueCharge` **deve** leggere il contratto dal proprio
   repository prima di inserire: la `WITH CHECK` di `charges_insert` non chiama
   `can_access_lease`, e senza il controllo si scrive una riga che nessuno potrà mai
   rileggere. Vedi le trappole.
8. **Use case: il riaddebito.** `RechargeUtilityBill`, che in una transazione sola
   verifica lo stato della bolletta, risolve il contratto, costruisce l'addebito, porta
   la bolletta ad `allocated` ed emette i due eventi.
9. **Migration `0022`: `can_access_attachment` riallineata.** `CREATE OR REPLACE
   FUNCTION` che per `utility_bill` chiede staff **oppure** parte del contratto
   indicato, come `utility_bills_select`. Viene **prima** del bordo HTTP che apre le
   rotte degli allegati, non dopo.
10. **Migration `0023`: i privilegi di colonna su `app.charges`.** `REVOKE UPDATE`,
    `GRANT UPDATE (status)`, con la guardia per l'ambiente in cui il ruolo non esiste.
    Estende l'elenco del **controllo 12** delle invarianti con le colonne
    dell'addebito, mantenendo il nome `colonna-strutturale-scrivibile`: il criterio è
    quello di ADR-026, non uno nuovo, e inventargli un terzo nome renderebbe
    illeggibile la ragione di tutti e tre. Viene **prima** dell'infrastruttura: le
    colonne concesse e quelle scritte dalle query sono la stessa lista, e scriverla
    due volte in ordine inverso è il modo di farle divergere.
11. **Infrastruttura.** `PgUtilityBillRepository` e `PgChargeRepository`. Query come
    letterali statici con `concat!`, cast espliciti sugli enum e sui domini. I tre
    stati dell'insoluto come **letterale SQL** in una macro `outstanding_statuses!`,
    per la stessa ragione di `working_statuses!`: `charges_outstanding_ix` è parziale
    su quell'elenco esatto. Mappatura di `utility_bills_dedup_uk` su un `Conflict`
    nominato.
12. **Bordo HTTP.** Le rotte, i DTO, `utoipa`, la registrazione in `openapi.rs` e nel
    router. Importi come stringhe decimali, `effective_status` accanto a `status`,
    `paid_amount` e `outstanding_amount` nella risposta.
13. **Allegati.** `AttachmentOwner::UtilityBill` e `::Charge`, più
    `POST`/`GET /api/v1/utility-bills/{id}/documents` e
    `POST`/`GET /api/v1/charges/{id}/documents`, che riusano gli use case dei documenti
    senza modificarli. Il test degli enum che oggi verifica «un `utility_bill` in
    lettura dà errore» si sposta sul primo owner rimasto fuori: la regola non cambia,
    si sposta il confine del perimetro.
14. **Test.** Unitari di dominio sulle due macchine a stati e sui value object; di
    persistenza con database reale — RLS dell'inquilino su addebiti e bollette, le
    colonne dell'addebito che danno `permission denied`, la deduplica che dà
    conflitto, il saldo letto dalla vista, la riga fantasma impedita; di contratto HTTP
    sull'intero giro, compreso il `422` su `?status=overdue` e il coinquilino che non
    scarica la bolletta dell'altro.
15. **Documentazione.** Endpoint in `docs/05-api.md`, il modulo spostato da «non
    implementato» a «implementato» in questa roadmap, l'esito del piano, e **quattro
    ADR**: lo stato scaduto derivato (estende ADR-019 agli addebiti), l'addebito che
    nasce emesso e non si riscrive (applica ADR-026 e dice perché non alle bollette),
    la valuta unica motivata dalla vista, e il documento della bolletta che si vede se
    si vede la bolletta.

### Trappole

Sono le vie che sembrano corrette e non lo sono. Tutte lette nello schema prima di
scriverle qui; quelle marcate **da riprodurre** vanno verificate su database reale in
esecuzione, come si è fatto per la `WITH CHECK` delle segnalazioni.

**`charges_insert` non chiama `can_access_lease`, e la riga fantasma è dietro
l'angolo.** La `WITH CHECK` è `org_id = app.current_org_id() AND
app.is_org_staff(org_id)`: nomina l'organizzazione e **non** il contratto. Uno staff
può quindi inserire un addebito con il **proprio** `org_id` e il `lease_id` di un
contratto di un'altra organizzazione, se ne conosce l'UUID — la foreign key verso
`app.leases` non guarda l'organizzazione, e su `app.charges` **non c'è** un trigger
`assert_org_matches_*`, che invece esiste su `rooms`, `leases`, `tickets` e
`utility_bills`. La riga entra e poi è invisibile a tutti: `charges_select` chiede
`can_access_lease(lease_id)`, che è falsa per chi l'ha scritta (non è staff dell'altra
organizzazione) e per l'altra organizzazione (l'`org_id` non è il suo). Non si può
rileggere, non si può cancellare — il ruolo applicativo non ha `DELETE` — e non
comparirà mai in nessun saldo. **Da riprodurre.** Il controllo sta in `IssueCharge`,
che legge il contratto dal proprio repository prima di scrivere: è lo stesso
`ensure_owner_is_visible` dei documenti, e per la stessa ragione.

**`INSERT … RETURNING` qui funziona, e sapere perché conta più del fatto.** Sulle
segnalazioni falliva perché `tickets_select` chiama `can_access_ticket(id)`, che
interroga **la riga che si sta inserendo**, e la funzione è `STABLE`: vede lo snapshot
di inizio statement, in cui la riga non c'è. `charges_select` chiama invece
`can_access_lease(lease_id)`, che interroga `app.leases` — una tabella che la `INSERT`
non tocca — quindi lo snapshot vecchio va benissimo. La regola generale, buona da
ricordare: **il muro c'è solo quando la policy di `SELECT` dipende dalla riga stessa.**
Non ce ne serviamo comunque, perché gli identificatori nascono nel dominio e nessuna
colonna è assegnata dal database.

**`utility_bills_dedup_uk` è parziale su `deleted_at IS NULL`, non su `status`.**
`(org_id, supplier_vat, invoice_number)`, unico, quando entrambi i campi sono
valorizzati. Una bolletta **annullata ma non cancellata** continuerebbe a occupare la
chiave, e la stessa fattura non sarebbe reinseribile mai più: è la ragione per cui
`void` sulle bollette valorizza anche `deleted_at` (decisione 6). Il vincolo scatta
solo se `supplier_vat` **e** `invoice_number` sono entrambi presenti, quindi due
bollette senza numero di fattura non danno conflitto — il che è corretto e va detto,
perché sembra che la deduplica non funzioni.

**`utility_bills_validated_ck` vincola una sola direzione.** È
`status NOT IN ('validated','allocated','paid') OR validated_at IS NOT NULL`: obbliga
ad avere una data quando si è validati, e **non** obbliga a toglierla tornando in
bozza. Il database accetta senza dire niente una bolletta `draft` con
`validated_at` valorizzato — cioè una bolletta che dichiara di essere stata controllata
e risulta da controllare. Deve azzerarlo il dominio, in `apply`. È esattamente la forma
di `tickets_closed_ck` e di `closed_at` alla riapertura, ed è la seconda volta che
questo schema la presenta: la si cerchi in ogni `CHECK` scritto come `stato ≠ X OR
colonna IS NOT NULL`.

**`charges_issued_ck` fa lo stesso.** `status = 'draft' OR issued_at IS NOT NULL`.
Poiché `draft` non si scrive mai, ogni addebito deve nascere con `issued_at`: dimenticarlo
non dà una riga strana, dà un rifiuto all'inserimento. È la traduzione a schema della
decisione 1, ed è la ragione per cui quella decisione è quasi gratuita.

**`charges_outstanding_ix` è parziale sui tre stati dell'insoluto.**
`WHERE deleted_at IS NULL AND status IN ('issued','partially_paid','overdue')`.
L'elenco dello scadenzario — quello che la dashboard leggerà — usa l'indice solo se
filtra `status` dentro quell'elenco **come letterale**: un array passato come parametro
non permette al planner di dimostrare che il predicato della query implica quello
dell'indice, perché al momento del piano non ne conosce il contenuto. Stessa
situazione di `tickets_open_ix` e stessa soluzione, macro compresa, con un test che
tiene insieme il letterale e la costante di dominio. Nota che l'elenco contiene
`overdue`, che l'applicazione non scrive: l'indice resta corretto lo stesso — copre
più di quanto serva — ma chi lo legge deve sapere che il terzo valore è lì per righe
che non esistono.

**Non esiste alcun vincolo fra `room_id`, `lease_id` e `property_id` di una
bolletta.** È la trappola delle segnalazioni, identica: il trigger
`assert_org_matches_property` su `app.utility_bills` verifica soltanto che l'`org_id`
coincida con quello dell'immobile. Una stanza di un altro immobile o il contratto di un
altro immobile passano senza una parola, e il risultato è una bolletta che nei report
sta sotto un immobile e nell'interfaccia sotto un altro. Il controllo sta nel service
layer, e — a differenza delle segnalazioni — **va ripetuto anche nel `PATCH`**, perché
lì `room_id` e `lease_id` restano scrivibili: sulla bolletta dicono *chi deve pagare*,
che è una decisione che cambia legittimamente finché la bolletta è una bozza, mentre
sulla segnalazione dicono *dov'è il guasto*, che è un fatto fissato al momento in cui
è stato segnalato. È il prezzo della decisione 4, ed è la ragione per cui il controllo
vive in una funzione sola chiamata da due punti.

**`can_access_attachment` per `utility_bill` è più largo della policy della
bolletta.** Vedi la decisione 11: risolve su `can_access_property`, e in un co-living
con contratti per stanza questo significa *ogni* inquilino dell'immobile. **Da
riprodurre** prima e dopo la migration `0022`: è l'unico modo di sapere che la
migration serve davvero e che ha funzionato.

**Il saldo non guarda la valuta.** `v_charge_balances` sottrae
`SUM(payments.amount - refunded_amount)` da `charges.amount` senza confrontare
`currency`, e le due colonne esistono su entrambe le tabelle. Finché la valuta è una
sola non fa danni; è la ragione per cui resta una sola (decisione 8), e va riletto il
giorno in cui qualcuno vorrà dichiararla.

**`is_past_due` usa `CURRENT_DATE`, che dipende dal fuso della sessione.** Un addebito
in scadenza oggi diventa scaduto alla mezzanotte **del database**, non a quella di chi
guarda. Con il container in UTC e un utente in Italia lo scarto è di una o due ore, una
volta al giorno, sul solo giorno della scadenza. Non si corregge in questa fetta —
correggerlo significa decidere di chi è il fuso orario di riferimento, che è una
domanda di prodotto — ma va scritto, perché il sintomo (un addebito che risulta scaduto
«troppo presto») è di quelli che si cercano nel posto sbagliato.

**`taxable_amount + vat_amount` non deve fare `total_amount`, e nessuno lo verifichi.**
Una bolletta di utenza italiana porta abitualmente più aliquote IVA, quote fisse e voci
non imponibili: un controllo di uguaglianza rifiuterebbe bollette valide. Lo schema
infatti non lo impone — i tre `CHECK` chiedono solo che siano non negativi — e il
dominio non deve aggiungerlo per simmetria.

**`consumption` è `NUMERIC(14,4)` e non è denaro.** Passarlo da `Money` lo farebbe
rifiutare a `Money::new`, che ammette due decimali: serve un value object proprio.
Sembra un dettaglio e costa mezza giornata quando un consumo di `123.4567 kWh`
restituisce un `422` che parla di importi.

**Due modi di allegare un documento a una bolletta, e uno solo va usato.**
`utility_bills.document_id` è una colonna con FK verso `app.documents` — la
«scansione» — e i documenti hanno già il riferimento polimorfico
`(owner_type, owner_id)`. Si usa il secondo, e `document_id` **resta non popolata**,
come `leases.contract_document_id` e `users.avatar_document_id` che l'API già non
tocca. Popolarle entrambe significherebbe avere due risposte alla domanda «qual è il
documento di questa bolletta» e doverle tenere allineate.

**`ocr_extracted` è fra le colonne redatte nell'audit**, come `extracted_data` dei
documenti. Non c'è nulla da fare in questa fetta — nessuno la scrive — ma è la prova
che il modulo OCR è già atteso da quella parte dello schema, e la redazione non va
tolta credendola inutile.

**`app.charges` non è nella lista dei trigger di audit?** Lo è: compare nell'array della
`0013` insieme a `settlements` e `tickets`, e `utility_bills` ha il proprio trigger con
la redazione di `ocr_extracted`. Nessuna azione, ma è il primo posto da guardare quando
si vuole sapere se una modifica lascia traccia — e per gli addebiti la risposta è sì,
ogni `UPDATE` finisce in `audit_log`.

**Gli identificatori nascono in Rust anche qui.** `utility_bills.id` e `charges.id`
hanno `DEFAULT gen_random_uuid()`, cioè UUIDv4: se una riga fosse inserita senza
specificare l'id, il cursore keyset per `id` restituirebbe un ordine casuale. Come per
ogni altra tabella del progetto, l'`INSERT` nomina sempre l'id e lo genera il dominio
con `generate()`, che è UUIDv7. Il `DEFAULT` resta lì per le righe che nessuno di noi
scrive.

### Come si saprà che è finita

`./ops/ci/test-integration.sh` verde con i test nuovi, e il controllo 12 delle
invarianti verde con le colonne dell'addebito nell'elenco — verificato **anche al
contrario**, concedendo `UPDATE (amount)` a mano e vedendolo fallire con
`colonna-strutturale-scrivibile / charges.amount`.

Un giro a mano contro il binario vero, PostgreSQL vero e MinIO vero, che percorra il
mese del proprietario: registra una bolletta del gas sull'immobile, vi allega la
scansione, si accorge di aver sbagliato il numero di fattura e la corregge, la valida,
la riaddebita al contratto per 120 euro con scadenza fra dieci giorni; l'inquilino vede
**l'addebito e la bolletta**, con `paid_amount` a `0.00` e `outstanding_amount` uguale
all'importo; il proprietario emette anche il canone del mese; un addebito con scadenza
nel passato risulta `effective_status: overdue` mentre `status` resta `issued`;
annullare l'addebito lo lascia visibile all'inquilino come `void`, annullare la bolletta
la fa sparire da entrambi e **libera il numero di fattura**, verificato reinserendola.

E le prove che riguardano chi non deve vedere: un utente di un'altra organizzazione
riceve `404` su bolletta, addebito, allegati e contenuto del documento, e l'elenco
vuoto; in un immobile in co-living con due contratti per stanza, il coinquilino che non
è parte del contratto della bolletta **non ne scarica la scansione** — la prova che la
migration `0022` serviva; `?status=overdue` risponde `422` indicando `?outstanding=true`;
e come `propflow_app`, sul database vivo, `UPDATE app.charges SET amount = …` dà
`permission denied for table charges`.

`docs/05-api.md` che descrive gli endpoint, e la guardia della configurazione verde
senza toccarla: questa fetta non aggiunge impostazioni.

### Esito — che cosa l'esecuzione ha confermato, cambiato e trovato

Eseguito il **31 agosto 2026**, tutti e quindici i passi, senza usare il punto di
taglio: gli allegati (passo 13) sono entrati nella fetta. Suite completa verde, **264
test** (erano 204): 35 nuovi unitari di dominio, 10 di persistenza, 14 di contratto
HTTP, più l'estensione delle tabelle di conversione degli enum.

Il piano ha retto quasi per intero. Ciò che segue è **solo ciò che è cambiato**, perché
è l'unica parte che il piano da solo non racconta.

#### Quattro cose che il piano non aveva previsto

**1. `Charge::is_past_due` ha due parametri, non uno.** Il piano scriveva
`is_past_due(today)`. Non poteva funzionare: la formula della vista è
`due_date < CURRENT_DATE AND amount > COALESCE(paid_amount, 0)`, e `paid_amount` **non
è un dato dell'aggregato** — vive in `app.payments`. Una funzione con un argomento solo
non sarebbe *la stessa* formula, sarebbe una che le somiglia finché non esistono
pagamenti, cioè fino al modulo successivo. La firma è
`is_past_due(today, paid_amount: Decimal)`, e in questa fetta il chiamante passa sempre
zero. Registrato in [ADR-028](08-decisioni.md#adr-028).

**2. La vista non guarda lo stato, e questo si è scoperto leggendola.** Il piano citava
`is_past_due` come se filtrasse gli addebiti insoluti. Non lo fa: `v_charge_balances`
non ha `status` nel predicato, quindi un addebito **annullato** con una scadenza nel
passato ha `is_past_due` vero. Il dominio ripete la formula così com'è — deve — e a
escludere quel caso è `effective_status`, che deriva `overdue` solo dagli stati
dell'insoluto. Ne segue una conseguenza per la dashboard, che il piano non poteva
vedere: `v_owner_dashboard.overdue_charges` conta con la definizione **più larga**, e
conterà anche gli addebiti annullati scaduti. Va saputo prima di scoprirlo.

**3. `void` sulle bollette non è ammesso da `allocated`.** Il piano non diceva da quali
stati si annulla una bolletta. La sua stessa logica lo impone: da `allocated` la
bolletta è la base di un addebito già emesso, e farla sparire — perché `void`
valorizza `deleted_at` — lascerebbe quell'addebito a citare (`charges.utility_bill_id`)
una riga che non si vede più. `void` è quindi ammesso da `draft` e `validated`, gli
stessi stati in cui la bolletta è correggibile, e l'ordine è **annullare prima gli
addebiti**.

**4. `Consumption` porta anche l'unità.** Il piano chiedeva un value object per il
numero, per via dei quattro decimali. Tenerci dentro anche l'unità è costato nulla e
rende inesprimibile una riga che lo schema accetta: `consumption` valorizzata e
`consumption_unit` nulla, cioè un numero che non dice niente. Stessa forma di `Money`,
che tiene insieme importo e valuta.

#### La cosa più cara imparata: un test che passa da uno strato non prova nulla sullo strato sotto

Il piano marcava «**da riprodurre**» la trappola di `can_access_attachment`. La prima
riproduzione è **fallita nel senso peggiore**: il test passava.

Scritto attraverso lo use case (`ListDocuments`), restava verde **anche rimettendo la
definizione sbagliata della `0014`** — verificato davvero, riportando la funzione
indietro su un database vero. La ragione è che `ensure_owner_is_visible` rilegge la
bolletta sotto `utility_bills_select` e restituisce `404` prima che la RLS entri in
gioco: lo strato applicativo **mascherava il difetto invece di dimostrarlo**.

La prova richiede di interrogare `can_access_attachment` **direttamente, sotto il
contesto RLS del coinquilino** — installando le GUC come fa l'unità di lavoro — che è
ciò che fa `documents_select` quando l'applicazione non c'è di mezzo. Fatto così, il
test fallisce con la definizione vecchia e passa con la `0022`. Riprodotto in entrambe
le direzioni; è la parte del lavoro che il ragionamento da solo non avrebbe dato, ed è
in [ADR-031](08-decisioni.md#adr-031).

La regola generale, che vale oltre questo caso: **la difesa in profondità si verifica
solo interrogando lo strato che si vuole verificare.** Un test che passa dal bordo
prova il bordo.

#### Le trappole confermate

Tutte quelle marcate nel piano si sono presentate come descritto, e nessuna ha
richiesto di cambiare rotta:

- `charges_insert` non chiama `can_access_lease`: il controllo in `IssueCharge` è ciò
  che impedisce la riga fantasma, e un test di persistenza prova che dopo il rifiuto
  `app.charges` è vuota — cioè che il controllo precede la scrittura;
- `utility_bills_dedup_uk` è parziale su `deleted_at IS NULL`: un test reinserisce la
  stessa fattura dopo l'annullamento e la vede passare;
- `utility_bills_validated_ck` vincola una sola direzione: un test rilegge la riga dal
  database dopo una correzione e verifica che `validated_at` sia `NULL`, non solo che
  l'aggregato in memoria lo sia;
- i privilegi di colonna della `0023`: verificati **anche al contrario**, concedendo
  `UPDATE (amount)` a mano e vedendo il controllo 12 fallire con
  `colonna-strutturale-scrivibile / charges.amount`, poi revocando e vedendolo tornare
  verde;
- `charges_issued_ck`: un addebito senza `issued_at` non dà una riga strana, dà un
  rifiuto all'inserimento — ed è la ragione per cui «niente bozza» è quasi gratuito;
- il vincolo assente fra `room_id`, `lease_id` e `property_id`: il controllo vive in
  `ensure_belong_to_property`, **una funzione chiamata da due punti**, e un test di
  contratto lo prova sia sulla registrazione sia sul `PATCH`.

Una sola trappola si è rivelata più stretta del previsto, e riguarda i **test**: la
regola «le stringhe SQL sono letterali statici» (`CLAUDE.md` §2.15) vale anche lì.
Comporre con `format!` l'elenco delle colonne da provare non compila, e le cinque
`UPDATE` del test sui privilegi sono cinque letterali.

#### Cosa resta fuori, e resta fuori davvero

Il perimetro dichiarato non si è allargato di un campo. Restano ai loro default e
**nessuno li scrive**: `ocr_extracted`, `ocr_confidence`, `ai_job_id`,
`document_id` (su entrambe le tabelle), `generation_key`, `metadata`, e gli stati
`bill_status.pending_review`, `bill_status.paid`, `bill_status.disputed`,
`charge_status.draft`, `charge_status.overdue`, `charge_status.partially_paid`,
`charge_status.paid`.

Sette valori d'enum su quindici sono nella tabella di conversione di `string_enum!` e
non escono mai da una scrittura. È deliberato: una riga scritta a mano potrebbe
contenerli, e un valore sconosciuto in lettura deve dare errore invece di ricadere su
un default plausibile.

#### Il giro a mano

Eseguito il **31 agosto 2026** contro il binario vero, PostgreSQL vero e MinIO vero,
percorrendo il mese del proprietario per intero. Tutto è tornato. Ciò che vale la pena
registrare, perché è la parte che i test automatici non dicono:

- la **normalizzazione della partita IVA** funziona sul filo: `it 123.456.789.01`
  entra e diventa `IT12345678901`, che è ciò che rende utile l'indice di deduplica;
- il **consumo** attraversa l'intera pila con i suoi quattro decimali
  (`123.4567 Smc`), che era la ragione del value object;
- la **scansione** ha fatto il giro completo dello storage: caricata, `307` verso
  l'URL pre-firmato, scaricata da MinIO, e i byte coincidono con quelli caricati;
- la **correzione disfa la validazione**, verificata due volte di seguito: validata,
  corretta con una nota, tornata `draft` con `validated_at` e `validated_by` a `null`;
- il riaddebito ha portato la bolletta ad `allocated` e l'addebito è nato `issued` con
  `paid_amount: "0.00"` e `outstanding_amount` uguale all'importo, **e il periodo
  copiato dalla bolletta** e non dal comando;
- un addebito con scadenza nel passato risponde `effective_status: overdue` mentre
  `status` resta `issued`; `?status=overdue` dà `422` con il rimedio nel messaggio, e
  `?outstanding=true` restituisce lo scadenzario;
- annullare l'addebito lo lascia **visibile all'inquilino** come `void`; annullare una
  bolletta non riaddebitata la fa sparire (`404`) e **libera il numero di fattura**,
  verificato reinserendo la stessa fattura con successo; una bolletta già riaddebitata
  dà invece `409`;
- un utente di un'altra organizzazione riceve `404` su bolletta, addebito, entrambi gli
  elenchi di allegati e il contenuto del documento, e i due elenchi rispondono `200`
  con zero elementi;
- come `propflow_app` sul database vivo, `UPDATE app.charges SET amount = …` e
  `SET due_date = …` danno `permission denied for table charges`, mentre
  `SET status = …` è accettata.

**La prova del co-living, quella per cui esiste la migration `0022`:** immobile con due
contratti per stanza, bolletta imputata al contratto di B con la sua scansione.

| | riga bolletta | elenco allegati | scansione |
|---|---|---|---|
| **B**, parte del contratto | `200` | `200` | `307` |
| **A**, coinquilino di un altro contratto | `404` | `404` | **`404`** |

Senza la `0022` l'ultima cella sarebbe stata un `307`, cioè la fattura di B — con il
suo nome, il suo indirizzo e i suoi consumi — scaricabile da A.

**Una cosa trovata dal giro e non dai test:** `.env.example` ha guadagnato la sezione
`PROPFLOW__STORAGE__*` quando è arrivato lo storage oggetti, e un `.env` locale creato
prima non ce l'ha. L'API non parte e dice
`missing field access_key_id for key "default.storage"`, che non nomina la variabile
d'ambiente da impostare. Non è un difetto di questa fetta e non si corregge qui, ma è
il primo intoppo che incontra chi torna su un ambiente vecchio.
