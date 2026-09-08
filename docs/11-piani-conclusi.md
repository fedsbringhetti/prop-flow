# 11 — Piani conclusi

L'archivio dei piani già eseguiti, ognuno con il suo esito. **Non si riscrive.**

Un piano entra qui quando il suo esito è scritto, e da quel momento resta com'era —
comprese le previsioni che l'esecuzione ha smentito, che sono la parte più utile da
rileggere. Correggerle a posteriori cancellerebbe l'unica prova di che cosa non si
sapeva prima di provare.

I piani ancora da eseguire, e quello in corso, stanno in
[09 — Stato e roadmap](09-stato-e-roadmap.md). **La divisione è per ciclo di vita e
non per argomento**, ed è deliberato: `09` si riscrive a ogni fetta, questo file mai.
Una divisione fra «vecchio» e «nuovo» sposterebbe il confine una volta e poi
marcirebbe, perché il piano nuovo diventa storia appena eseguito.

Il momento in cui un piano si sposta è **lo stesso commit in cui si scrive il suo
esito**: è l'unico in cui chi lavora ha già entrambi i file aperti, e una regola di
migrazione che chieda un passaggio a parte non viene rispettata. Sta anche in
`CLAUDE.md` §3.

## Che cosa c'è dentro un piano

Sempre le stesse sei parti, e servono a cose diverse:

| Sezione | A che serve |
|---|---|
| Obiettivo | il perimetro, e soprattutto **ciò che resta fuori con la ragione** |
| Decisioni prese, con il perché | le alternative scartate, che è ciò che impedisce di riproporle |
| Decomposizione | i passi eseguibili singolarmente, con il punto di taglio |
| Trappole | le vie che sembrano corrette e non lo sono: la conoscenza più cara, perché per riscoprirne una bisogna prima sbagliare |
| Come si saprà che è finita | il criterio di completamento, che non è «la suite è verde» |
| Esito | che cosa l'esecuzione ha **confermato, cambiato e trovato** |

Se la domanda è «cosa è successo», l'esito si legge per primo. Se la domanda è
«perché il codice è così», si legge per secondo.

## Indice

| Piano | Pianificato | Che cosa ha portato |
|---|---|---|
| [Documenti](#piano-documenti-fetta-1) · [esito](#esito-documenti-fetta-1) | 18 agosto 2026 | caricamento multipart, download pre-firmato, riferimento polimorfico |
| [Segnalazioni](#piano-segnalazioni-fetta-1) · [esito](#esito-segnalazioni-fetta-1) | 27 agosto 2026 | ciclo di vita a sette transizioni, thread, note interne |
| [Bollette e addebiti](#piano-bollette-e-addebiti-fetta-1) | 30 agosto 2026 | registrazione, validazione, riaddebito; `effective_status` derivato |
| [Pagamenti](#piano-pagamenti-fetta-1) · [esito](#esito-pagamenti-fetta-1) | 31 agosto 2026 | incasso e dichiarazione, idempotenza, saldi veri |
| [Spese co-living](#piano-spese-co-living-fetta-1) | 1 settembre 2026 | gruppi, quote che sommano all'importo, regolamenti a due tempi |
| [OCR delle bollette](#piano-ocr-bollette-fetta-1) · [esito](#esito-ocr-bollette-fetta-1) | 1 settembre 2026 | coda dei job, worker, proposta separata dai valori confermati |
| [I coinquilini si vedono](#piano-coinquilini-visibili-fetta-1) · [esito](#esito-coinquilini-visibili-fetta-1) | 2 settembre 2026 | perimetro dell'anagrafica ristretto alla casa (migration `0030`) |
| [Il client](#piano-client-fetta-1) · [esito](#esito-client-fetta-1) | 3 settembre 2026 | percorso del proprietario end-to-end, sessione in cookie, dashboard |
| [Completare il prodotto, fetta A](#piano-completare-il-prodotto) · [esito](#esito-completare-il-prodotto-fetta-a) | 4 settembre 2026 | il giro del denaro si chiude: incassi dall'interfaccia, coda delle dichiarazioni, le perdite in dashboard |

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

#### Una verifica che non verificava niente

Vale la pena registrarla perché il sintomo era un verde, non un rosso.

Il primo controllo della `0029` su database vero rispondeva `UPDATE 0` a ogni scrittura
— quella che doveva essere rifiutata e quella che doveva passare — e letto in fretta
sembrava che la policy funzionasse. Non c'erano righe: il setup aveva inserito i due
membri con **una `INSERT` multi-riga**, e quella fallisce.

La ragione è la stessa che vieta l'`INSERT … RETURNING` sui gruppi.
`expense_group_members_insert` chiede `can_access_expense_group(group_id)` per chi non è
il creatore, e quella funzione è `STABLE`: dentro lo stesso statement vede lo snapshot
iniziale, in cui la membership del creatore **non c'è ancora**. Il creatore entra per il
ramo della `0026`, il secondo membro no, e l'intero statement cade.

Due statement separati funzionano, perché in `READ COMMITTED` ognuno prende uno snapshot
nuovo. Nessun codice dell'applicazione fa una `INSERT` multi-riga di membership, quindi
non è un difetto — ma è una proprietà dello schema che va saputa prima di scrivere un
setup di prova, e il modo in cui si manifesta è un test che non prova niente invece di
un errore.

La regola generale: **una verifica che risponde «zero righe» va guardata due volte**,
perché zero righe è anche ciò che risponde una verifica che non ha trovato niente da
verificare.

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
tre sono registrati fra i [debiti noti](09-stato-e-roadmap.md#debiti-tecnici-noti) con la ragione e il
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
| La sequenza è globale, non per organizzazione | Sì. `TK-000001` sul database di test appena migrato, e la sequenza è una sola per il cluster. Registrato fra i [debiti](09-stato-e-roadmap.md#debiti-tecnici-noti) |
| Non esiste alcun vincolo fra `room_id`, `lease_id` e `property_id` | **Sì, riprodotto su database reale.** Un `INSERT` diretto con la stanza di un altro immobile è passato senza una parola. Il controllo sta in `OpenTicket`, una volta sola, e un test di persistenza più uno di contratto lo fissano |
| `ticket_messages` non ha una policy di `UPDATE` | Sì. Verificato interrogando `pg_policy`: due policy, `_select` e `_insert`. Il test `a_thread_message_can_never_be_rewritten` fallisce se qualcuno ne aggiunge una terza, perché quella è la proprietà su cui il thread si regge |
| La `WITH CHECK` di `tickets_update` vincola solo `org_id` | Sì. `pg_get_expr(polwithcheck, …)` restituisce esattamente `(org_id = current_org_id())`. La migration `0020` e [ADR-026](08-decisioni.md#adr-026) sono la risposta |
| L'onere di manutenzione non è un rischio silenzioso | Sì, e con una guardia in più: oltre ai test di integrazione che girano come `propflow_app`, il **controllo 12** delle invarianti dello schema verifica che le nove colonne strutturali restino non aggiornabili. Verificato concedendo `UPDATE (title)` a mano: il controllo fallisce con `colonna-strutturale-scrivibile / tickets.title` |
| Il service layer restringe comunque | Sì. `status` e i due costi restano concesse, e chi possa chiudere o valorizzare resta autorizzazione applicativa, transizione per transizione |
| Lo storico lo scrive un trigger, e scriverlo anche noi lo raddoppia | **Sì, riprodotto.** Un `INSERT` diretto produce da sé la riga `NULL → open`, con `changed_by` nullo fuori da un contesto di richiesta. La porta non ha un metodo di scrittura sullo storico, per non invitare a farlo |
| `closed_at` non si azzera da solo alla riapertura | Sì. Lo azzera `Ticket::reopen`, e due test lo fissano: uno unitario e uno di persistenza che rilegge la riga dal database, non l'aggregato in memoria |
| Solo chi ha segnalato può riaprire | Sì, e ha prodotto un debito: al coinquilino l'API risponde `org.staff_role_required`, che è vero a metà. Registrato fra i [debiti](09-stato-e-roadmap.md#debiti-tecnici-noti), insieme al gemello sui documenti |
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

#### Una verifica che non verificava niente

Vale la pena registrarla perché il sintomo era un verde, non un rosso.

Il primo controllo della `0029` su database vero rispondeva `UPDATE 0` a ogni scrittura
— quella che doveva essere rifiutata e quella che doveva passare — e letto in fretta
sembrava che la policy funzionasse. Non c'erano righe: il setup aveva inserito i due
membri con **una `INSERT` multi-riga**, e quella fallisce.

La ragione è la stessa che vieta l'`INSERT … RETURNING` sui gruppi.
`expense_group_members_insert` chiede `can_access_expense_group(group_id)` per chi non è
il creatore, e quella funzione è `STABLE`: dentro lo stesso statement vede lo snapshot
iniziale, in cui la membership del creatore **non c'è ancora**. Il creatore entra per il
ramo della `0026`, il secondo membro no, e l'intero statement cade.

Due statement separati funzionano, perché in `READ COMMITTED` ognuno prende uno snapshot
nuovo. Nessun codice dell'applicazione fa una `INSERT` multi-riga di membership, quindi
non è un difetto — ma è una proprietà dello schema che va saputa prima di scrivere un
setup di prova, e il modo in cui si manifesta è un test che non prova niente invece di
un errore.

La regola generale: **una verifica che risponde «zero righe» va guardata due volte**,
perché zero righe è anche ciò che risponde una verifica che non ha trovato niente da
verificare.

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

---

<a id="piano-pagamenti-fetta-1"></a>

## Piano — Pagamenti, fetta 1

Pianificato il **31 agosto 2026**, **implementato il 1 settembre 2026**. Il piano resta sotto come è stato
scritto: quello che segue serviva a eseguirlo, e serve ancora a capire perché il codice
è com'è. Ciò che l'esecuzione ha confermato, cambiato e scoperto è in fondo, in
[Esito](#esito-pagamenti-fetta-1).

### Obiettivo

Il denaro che entra. Il proprietario registra un incasso che ha visto sull'estratto
conto; l'inquilino può dichiarare di aver pagato e lo staff conferma. È il modulo che
fa muovere `paid_amount`, oggi sempre `0.00` in ogni risposta, e che rende veri i due
numeri di testa della dashboard.

Chiude anche il difetto delle viste registrato in cima ai debiti — quello per cui
`total_outstanding` conta come credito anche gli addebiti annullati — e lo chiude
**come primo passo**, perché la domanda «cosa significa insoluto» *è* la domanda di
questo modulo, e deciderla altrove significherebbe deciderla due volte.

**Dentro:** correzione di `app.v_charge_balances`; registrazione di un incasso da parte
dello staff; dichiarazione da parte dell'inquilino; conferma e rifiuto; elenco e
dettaglio dei pagamenti di un addebito; ricalcolo dello stato dell'addebito dai
pagamenti.

**Fuori, e non per dimenticanza:**

- **i rimborsi** (`refunded_amount`, e gli stati `refunded` e `partially_refunded`): la
  vista li conta già, ma un rimborso è un'operazione con la propria autorizzazione e la
  propria ragione, e aprirla qui allungherebbe la fetta senza che nessuno l'abbia
  chiesta. `refunded_amount` resta a zero, e `partially_refunded` resta uno stato che
  l'applicazione non scrive — pur essendo **contato** dalla vista, il che va saputo;
- **i provider veri** (`stripe`, `gocardless`): `provider` resta `manual`,
  `provider_payment_id` e `provider_payload` restano ai default. Il giorno in cui
  arriva un provider servono i webhook, e i webhook sono metà della ragione per cui
  `idempotency_key` esiste — l'altra metà la usiamo subito, vedi la decisione 4;
- **i pagamenti di quote e regolamenti** (`payable_type` in `expense_share`,
  `settlement`): è il modulo delle spese co-living. `payable_type` li contempla già;
- **il pagamento della bolletta al fornitore**: non è modellato affatto —
  `app.payable_type` non comprende `utility_bill` — ed è la stessa constatazione della
  fetta contabile;
- **la dashboard**: viene dopo, ed è questa fetta a darle numeri che dicono la verità.

### Decisioni prese, con il perché

#### 1. La vista azzera il residuo, e **solo per due stati**

`app.v_charge_balances` guadagna una condizione: quando lo stato è `void` o
`written_off`, `outstanding_amount` vale `0` e `is_past_due` è falso.

**`paid` non è nell'elenco, ed è la parte che conta.** Azzerare per stato dove
l'aritmetica è già significativa **inverte il rapporto** fra i due: lo stato
memorizzato scavalcherebbe il calcolo fatto sui pagamenti. Se un giorno
`status = 'paid'` e la somma dei pagamenti non torna, quello scarto va **visto**, non
nascosto — in contabilità la deriva fra ciò che una riga dichiara e ciò che i movimenti
dicono è precisamente il difetto che si vuole scoprire.

`void` e `written_off` sono diversi in natura: lì non è dovuto niente **a prescindere
dai pagamenti**, perché la pretesa è caduta. È l'unico caso in cui lo stato è
l'autorità.

**Conseguenza elegante: la dashboard si corregge da sé.** `v_owner_dashboard` filtra
già `outstanding_amount > 0` e conta su `is_past_due`, quindi una migration su **una
vista sola** rimette a posto entrambi i numeri. Non serve toccare la seconda.

**Alternativa scartata: azzerare tutto ciò che non è nell'insoluto**, `draft` e `paid`
compresi. Più simmetrico e peggiore, per la ragione detta sopra.

**Alternativa scartata: togliere le righe dalla vista.** È la via che sembra ovvia ed è
sbagliata: `find_balance_by_id` serve il dettaglio di un addebito **annullato**, che
l'inquilino deve poter vedere, e senza la riga riceverebbe `404` su qualcosa che gli
spetta. Un test della fetta contabile già lo fissa, e fallirebbe — il che è il modo
giusto di scoprirlo, ma meglio non scoprirlo affatto.

**Conseguenza sull'API, da dichiarare:** `outstanding_amount` di un addebito annullato
passa da `"120.00"` a `"0.00"`. Nessun client legge ancora quel campo, quindi il
cambiamento è additivo — ed è la finestra che i debiti dicevano di non lasciar chiudere.

#### 2. L'inquilino dichiara, lo staff conferma

Non lo si sta forzando sullo schema: **lo schema è costruito per questo**, e le tre
righe che lo dicono erano già lì dal `0014` e dal `0015`.

| | |
|---|---|
| `payments_insert` | `is_org_staff(org_id) OR payer_user_id = current_user_id()` |
| `payments_update` | staff soltanto |
| `v_charge_balances` | conta solo `succeeded` e `partially_refunded` |

Ne segue il flusso, senza inventare niente: l'inquilino inserisce una riga `pending`,
che **non muove il saldo**; lo staff la porta a `succeeded`, e il saldo si muove. Il
`pending` non è uno stato di comodo — è ciò che rende la dichiarazione visibile senza
renderla vera.

**Lo staff che registra un incasso visto sull'estratto conto inserisce direttamente
`succeeded`.** Non c'è un secondo osservatore da attendere, e un passo intermedio senza
differenza osservabile è lo stesso argomento per cui l'addebito non ha la bozza.

#### 3. Una dichiarazione rifiutata va a `cancelled`, e l'uscita si disegna adesso

Uno stato in cui si entra e da cui non si esce è peggio di nessuno stato: è
precisamente la ragione per cui `bill_status.disputed` è rimasto fuori dalla fetta
contabile. `pending` ha quindi **due uscite**, e nascono insieme a lui: `succeeded` se
confermato, `cancelled` se rifiutato.

`failed` resta non scritto: significa che il pagamento non è andato a buon fine
**presso il provider**, e con `manual` un provider non c'è.

**L'inquilino non può ritirare la propria dichiarazione**, e non è una scelta di
prodotto rivedibile nel codice: `payments_update` è solo staff. Va detto nell'API
perché è controintuitivo — si può creare qualcosa che poi non si può disfare.

#### 4. L'`Idempotency-Key` è obbligatoria e la fornisce il **client**

È la decisione più importante della fetta, e l'unica in cui sbagliare costa denaro.

`idempotency_key` è `NOT NULL` con un indice unico su `(org_id, idempotency_key)`. Se a
generarla fosse il server, ogni ritentativo produrrebbe una chiave nuova, quindi **un
secondo pagamento**, quindi un saldo che scende **due volte**. Un doppio invio — un tap
ripetuto, un retry di rete, un client che non ha visto la risposta — dimezzerebbe
l'insoluto senza che nessuno se ne accorga, e la traccia in `audit_log` mostrerebbe due
inserimenti perfettamente legittimi.

Header `Idempotency-Key` obbligatorio quindi, e una chiave ripetuta restituisce **`200`
con il pagamento già registrato**, non `409`. È la convenzione dell'idempotenza, e un
`409` costringerebbe il client a distinguere «l'ho già mandato io» da «qualcun altro ha
usato la mia chiave» — distinzione che non può fare.

**Alternativa scartata: chiave generata dal server.** Toglie ogni protezione proprio
dove il danno è denaro, e lo fa in silenzio.

#### 5. Un pagamento non si cancella, nemmeno soft

`app.payments` **non ha `deleted_at`** — verificato, non dedotto — e il ruolo
applicativo non ha `DELETE`. Un pagamento sbagliato si corregge muovendone lo stato,
non togliendo la riga. È coerente con l'audit: la tabella ha il proprio trigger, con la
redazione di `provider_payload`.

Ne segue una regola per le query: **non filtrare `deleted_at`**, che qui non esiste.
Copiare il pattern dalle altre tabelle darebbe un errore di colonna inesistente — il
che è fastidioso e innocuo, perché fallisce subito.

#### 6. `RecordPayment` rilegge l'addebito, per la stessa ragione della riga fantasma

`app.tg_check_polymorphic_ref` verifica che la riga referenziata **esista** —
`SELECT 1 FROM app.charges WHERE id = $1` — ed è `SECURITY DEFINER`, quindi la verifica
non dipende dalla visibilità RLS del chiamante. **Non guarda l'organizzazione.**

È lo stesso buco di `charges_insert`, in forma nuova: si potrebbe inserire un pagamento
con il proprio `org_id` che punta a un addebito di un'altra organizzazione, e il trigger
lo accetterebbe perché la riga esiste davvero. Il controllo sta nello use case, che
legge l'addebito dal proprio repository sotto la RLS del chiamante — è la terza volta
che questo progetto usa lo stesso rimedio, dopo `ensure_owner_is_visible` dei documenti
e `build_issued_charge` degli addebiti.

#### 7. Lo stato dell'addebito si **ricalcola** dai pagamenti, non si dichiara

`Charge::apply_payment(paid_total)`: da `issued` a `partially_paid` quando
`0 < paid < amount`, a `paid` quando `paid >= amount`. È la prima scrittura che la
`GRANT UPDATE (status)` della migration `0023` rende possibile, ed è l'unica di cui il
modulo ha bisogno — la fetta contabile l'aveva posata apposta.

**Il ricalcolo scatta solo quando un pagamento entra o esce da `succeeded`.** Una
dichiarazione rifiutata non lo tocca, perché `pending` non contava: è l'eleganza del
flusso in due tempi, e va detta perché altrimenti si è tentati di ricalcolare a ogni
transizione.

Registrare un pagamento su un addebito `void` o `written_off` è **rifiutato**: la
pretesa è caduta, e incassare su una pretesa caduta è un'incoerenza, non un caso d'uso.

#### 8. Il sovrapagamento è possibile e non lo si impedisce

Stessa forma del riaddebito che supera il totale della bolletta, e stessa risposta: lo
schema non ha un vincolo, un controllo applicativo sarebbe soggetto a race condition
fra due pagamenti concorrenti, e vale il criterio di
[ADR-015](08-decisioni.md#adr-015) per cui quel genere di invariante sta nel database o
non sta da nessuna parte. `outstanding_amount` andrebbe **negativo**, che è un credito
verso l'inquilino — rappresentabile, perché `app.money_amount` ammette i negativi.
Registrato qui invece che scoperto dopo.

### Decomposizione

**Il passo 1 viene prima di tutto**, ed è l'unico che si può mergiare da solo: è la
correzione che chiude la finestra sui due numeri della dashboard.

Il **punto di taglio** è il passo 7, se la fetta risulta troppo grande: senza i
privilegi di colonna resta un modulo completo, e la migration è additiva.

1. **Migration `0024`: `v_charge_balances` con il residuo azzerato.** `CREATE OR
   REPLACE VIEW`, con `outstanding_amount` e `is_past_due` a zero/falso per `void` e
   `written_off`. Verificare che la vista resti `security_invoker` — un `CREATE OR
   REPLACE` che perdesse quel flag bucherebbe la RLS in silenzio, ed è la trappola più
   cara del passo.
2. **Dominio: gli enum e i value object.** `PaymentStatus` (sette valori, tre scritti),
   `PaymentMethod`, `IdempotencyKey` come value object con il proprio tetto.
3. **Dominio: l'aggregato `Payment`.** `declare` (nasce `pending`), `record` (nasce
   `succeeded`, con `paid_at`), `confirm`, `reject`. Test sulla macchina: le transizioni
   ammesse **e** quelle rifiutate.
4. **Dominio: `Charge::apply_payment`.** Il primo metodo che scrive `status` dai
   pagamenti, con i test sulle tre soglie e sul rifiuto da `void` e `written_off`.
5. **Dominio: gli eventi.** `PaymentDeclared`, `PaymentConfirmed`, `PaymentRejected`.
   **Nessun importo viaggia**, e qui a maggior ragione che altrove: la coppia «chi» e
   «quanto ha pagato» è esattamente il dato che non deve conservarsi su un broker.
6. **Porta `PaymentRepository`:** `find_by_id`, `find_by_idempotency_key`,
   `list_for_charge`, `sum_succeeded_for_charge`, `insert`, `update_status`. La ricerca
   per chiave serve a restituire `200` invece di inserire, e va **prima** dell'insert.
7. **Migration `0025`: privilegi di colonna su `app.payments`.** La `WITH CHECK` di
   `payments_update` nomina il solo `org_id`: stesso difetto, stesso criterio di
   [ADR-026](08-decisioni.md#adr-026). Le colonne che l'applicazione scrive sono
   `status`, `paid_at` e `reconciled_at`; importo, pagante, metodo e chiave di
   idempotenza **non hanno motivo di cambiare** dopo l'inserimento. Estende il controllo
   12 delle invarianti, mantenendo il nome.
8. **Use case.** `RecordPayment` (staff, `succeeded`), `DeclarePayment` (membro,
   `pending`), `ConfirmPayment`, `RejectPayment`, `ListPayments`. Tutti rileggono
   l'addebito prima di scrivere, per la decisione 6.
9. **Infrastruttura: `PgPaymentRepository`.** I due stati che contano nel saldo come
   **letterale SQL**, allineato **alla vista e non all'indice** — vedi le trappole.
10. **Bordo HTTP.** Le rotte, l'header `Idempotency-Key` come estrattore obbligatorio,
    il `200` sulla chiave ripetuta.
11. **Test.** Unitari sulle due macchine; di persistenza con database reale — il saldo
    che si muove solo su `succeeded`, l'idempotenza, l'addebito di un'altra
    organizzazione rifiutato; di contratto HTTP sull'intero giro.
12. **Documentazione.** `docs/05-api.md`, l'esito del piano, e **due ADR**: la vista che
    azzera il residuo per gli stati terminali, e l'idempotenza fornita dal client.

### Trappole

Tutte lette nello schema prima di scriverle qui; quelle marcate **da riprodurre** vanno
verificate su database reale in esecuzione.

**`payments_payable_ix` e la vista NON elencano gli stessi stati.** L'indice è parziale
su `('authorized','succeeded','partially_refunded')`, la vista conta
`('succeeded','partially_refunded')`. Il predicato della query è più stretto di quello
dell'indice, quindi l'indice resta **utilizzabile** — nessun problema di prestazioni. Il
problema è un altro e più insidioso: **copiare l'elenco dall'indice conterebbe anche gli
`authorized`**, cioè pagamenti autorizzati e non incassati, facendo scendere il saldo su
denaro che non è arrivato. Il letterale da usare è quello della **vista**.

**`payments_succeeded_ck` vincola una sola direzione.** È
`status <> 'succeeded' OR paid_at IS NOT NULL`: obbliga ad avere la data quando si è
riusciti, e **non** obbliga a toglierla muovendosi altrove. È la **terza volta** che
questo schema presenta questa forma, dopo `tickets_closed_ck` e
`utility_bills_validated_ck`. Ormai è un pattern: **cercarla in ogni `CHECK` scritto
come `stato ≠ X OR colonna IS NOT NULL`**, e assumere che l'azzeramento tocchi al
dominio.

**`tg_check_polymorphic_ref` non guarda l'organizzazione.** Verifica solo che la riga
esista, ed è `SECURITY DEFINER`. **Da riprodurre**: inserire un pagamento verso un
addebito di un'altra organizzazione e vedere che il trigger lo accetta — è la prova che
il controllo applicativo serve.

**L'inquilino inserisce ma non aggiorna.** `payments_insert` lo ammette come pagante,
`payments_update` no. Una rotta che gli lasciasse ritirare la dichiarazione otterrebbe
zero righe aggiornate, cioè `403 auth.denied_by_policy` — il codice riservato ai nostri
difetti.

**`app.payments` non ha `deleted_at`.** Filtrarlo per abitudine dà un errore di colonna
inesistente. Fallisce subito, quindi è innocuo, ma fa perdere tempo a chi copia una
query da un altro repository.

**Il `CREATE OR REPLACE VIEW` può perdere `security_invoker`.** È l'unico modo in cui il
passo 1, che sembra innocuo, buca la RLS: senza quel flag la vista gira con i diritti
del proprietario e ogni organizzazione vede i saldi di tutte. Il controllo delle
invarianti **non** verifica quel flag oggi — vale la pena aggiungerlo mentre si è lì.

**La vista comincia davvero a fare il join.** Finora la `LEFT JOIN LATERAL` restituiva
sempre `NULL`, perché `app.payments` era vuota: qualunque prova di prestazioni fatta
prima di questa fetta non dice nulla su quella dopo.

**Il sovrapagamento porta `outstanding_amount` negativo**, e la dashboard lo somma. Con
`WHERE outstanding_amount > 0` la riga negativa viene esclusa dal totale, quindi il
credito **non compensa** gli altri insoluti — che è probabilmente il comportamento
giusto, ma è un caso da guardare quando qualcuno paga due volte.

**Il fuso orario di `paid_at`.** L'inquilino dichiara di aver pagato «ieri»: quel `TIMESTAMPTZ`
va interpretato, e `is_past_due` usa `CURRENT_DATE` del database. È lo stesso scarto già
registrato per gli addebiti, e qui si vede una seconda volta.

### Come si saprà che è finita

`./ops/ci/test-integration.sh` verde con i test nuovi, e il controllo delle invarianti
verde con le colonne del pagamento nell'elenco — verificato **anche al contrario**, come
per la fetta contabile.

**Il difetto delle viste verificato al contrario**, che è la prova che il passo 1 è
servito: riprodurre il caso già documentato — 800.00 emesso, 120.00 annullato, 40.00
scaduto — e vedere `total_outstanding` passare da `960.00` a **840.00**, con
`overdue_charges` che smette di contare l'annullato.

Un giro a mano contro il binario vero: il proprietario emette un addebito da 800; **l'inquilino
dichiara di aver pagato e il saldo NON si muove**, resta `800.00` insoluto e l'addebito
resta `issued`; lo staff conferma e il saldo scende a zero mentre l'addebito passa a
`paid`; **un secondo invio con la stessa `Idempotency-Key` restituisce `200` e non fa
scendere il saldo una seconda volta**; una seconda dichiarazione rifiutata lascia il
saldo dov'era; e un pagamento verso l'addebito di un'altra organizzazione riceve `404`.

<a id="esito-pagamenti-fetta-1"></a>

### Esito — che cosa l'esecuzione ha confermato, cambiato e trovato

Eseguita il **1 settembre 2026**, nell'ordine del piano. I dodici passi sono tutti
dentro: il punto di taglio al passo 7 non è servito.

`./ops/ci/test-integration.sh` verde, 293 test (erano 264): 147 unitari di dominio,
16 applicativi, 12 di infrastruttura, 6 unitari del bordo HTTP, 46 di integrazione
sulla persistenza, 3 sull'adattatore S3, 60 di contratto HTTP, 3 di guardia sulla
documentazione della configurazione. Invarianti dello schema verdi, e **verificate al
contrario**: riconcedendo `GRANT UPDATE ON app.payments` il controllo 12 riporta
tredici violazioni con i nomi delle colonne, una per una.

#### Le cinque cose che il piano non aveva previsto

**1. La trappola più temuta era già disinnescata.** Il piano diceva che il controllo
delle invarianti «non verifica `security_invoker` oggi — vale la pena aggiungerlo
mentre si è lì». Il controllo **c'era già**: è il numero 3,
`vista-senza-security-invoker`, e verifica ogni vista di `app`. La migration `0024` lo
ha attraversato verde al primo colpo. È il caso raro in cui una trappola prevista si
scopre disinnescata, e la lezione è banale e utile: **guardare l'elenco delle
invarianti prima di temere**, perché è cresciuto più della memoria di chi lo ha
scritto.

**2. Correggere la vista obbliga a correggere il dominio.** Il piano trattava il passo
1 come una migration sola. Non lo è: [ADR-028](08-decisioni.md#adr-028) aveva
dichiarato che `Charge::is_past_due` ripete la formula della vista **e non guarda lo
stato**, con un test che lo fissava. Cambiata la vista, quel test asseriva l'opposto
del vero. La correzione è di tre righe — `!self.status.is_claim_dropped() && …` — ma va
vista: una funzione che esiste **solo** per essere la stessa formula smette di avere
ragione di esistere nel momento in cui la formula diverge. Il test è stato riscritto
nella direzione nuova, e dice esplicitamente che prima asseriva l'opposto.

**3. `payer_user_id` non è anagrafica, ed è la scelta che il piano non aveva visto.**
Il piano descriveva chi può inserire, non **a chi la riga risulta visibile**.
`payments_select` mostra il pagamento a staff, pagante e incassante: un incasso
registrato dallo staff senza valorizzare `payer_user_id` è invisibile all'inquilino nel
proprio elenco. Il campo è quindi diventato opzionale nel comando di registrazione e
**assente** in quello di dichiarazione — lì il pagante è sempre chi chiama, perché la
policy lo esige. È registrato fra i debiti: il caso più facile, omettere il campo, è
anche quello che nasconde il pagamento a chi lo ha fatto.

**4. Due rotte di creazione e non una.** Il piano nominava cinque use case e non diceva
come si affacciassero sull'HTTP. Una rotta sola avrebbe richiesto all'handler di
scegliere fra `RecordPayment` e `DeclarePayment` guardando il ruolo di chi chiama, e
quel ramo è **autorizzazione**: la regola 3 di `CLAUDE.md` lo vieta, e non per
formalismo — l'autorizzazione duplicata in due strati diverge. Da qui
`POST /charges/{id}/payments` per lo staff e
`POST /charges/{id}/payment-declarations` per il membro, con la conseguenza gradita di
una specifica OpenAPI che dice la verità: due operazioni, due esiti.

**5. Il `409` residuo sull'idempotenza esiste, ed è irriducibile.** Il piano diceva
«una chiave ripetuta restituisce `200`». Vero, ma la rilettura gira sotto la RLS del
chiamante: se la chiave appartiene a un pagamento che il chiamante **non vede**, non la
trova e l'`INSERT` viola `payments_idempotency_uk`. Si aggiunge il caso della stessa
chiave verso un **altro** addebito, dove un `200` con un pagamento diverso da quello
chiesto sarebbe una bugia. Entrambi rispondono `payment.idempotency_key_reused`, ed
entrambi sono documentati: distinguerli richiederebbe di leggere una riga che al
chiamante non spetta. Vedi [ADR-033](08-decisioni.md#adr-033).

#### Le trappole, verificate una per una

| Trappola | Esito |
|---|---|
| L'elenco dell'indice **non** è quello della vista | **Confermata, e messa sotto test.** Il letterale del repository è quello della vista; due test lo tengono fermo, uno contro la costante di dominio e uno che asserisce che `authorized` **non** compaia. È l'errore che avrebbe fatto scendere il saldo su denaro non incassato |
| `payments_succeeded_ck` vincola una direzione sola | **Confermata**, e l'azzeramento è finito nel dominio: `Payment::reject` toglie `paid_at`. Terza occorrenza dello stesso pattern dopo `tickets_closed_ck` e `utility_bills_validated_ck` |
| `tg_check_polymorphic_ref` non guarda l'organizzazione | **Riprodotta.** Il test `the_polymorphic_trigger_accepts_a_charge_of_another_organization_and_the_use_case_does_not` inserisce a mano un pagamento di un'organizzazione verso l'addebito di un'altra e verifica che il trigger **lo accetti**; poi verifica che lo use case risponda `404`. È la prova che il controllo applicativo serve, non una supposizione |
| L'inquilino inserisce ma non aggiorna | **Confermata**, e diventata contratto dell'API: non esiste una rotta di ritiro, ed è detto nella documentazione perché è controintuitivo |
| `app.payments` non ha `deleted_at` | **Confermata.** Nessuna query lo filtra, ed è scritto sopra le macro delle colonne perché è la prima cosa che si copierebbe da un altro repository |
| Il `CREATE OR REPLACE VIEW` può perdere `security_invoker` | **Non si è presentata**: la clausola è ripetuta per esteso e il controllo 3 delle invarianti la verifica comunque. Vedi il punto 1 qui sopra |
| La vista comincia davvero a fare il join | **Confermata come fatto**, non misurata: nessuna prova di prestazioni è stata fatta né prima né dopo, e dirlo è più onesto che dedurne qualcosa |
| Il sovrapagamento porta `outstanding_amount` negativo | **Riprodotta**, con il test `an_overpayment_drives_the_outstanding_amount_negative`: 150.00 su un addebito da 100.00 dà `-50.00`, l'addebito va a `paid`, e `total_outstanding` della dashboard resta zero perché la vista filtra `> 0`. Il credito non compensa gli altri insoluti |
| Il fuso orario di `paid_at` | **Confermata e non corretta**, come per gli addebiti: correggerla significa decidere di chi è il fuso di riferimento, che è una domanda di prodotto. Detto nell'API, dove si vede |

#### Scelte prese eseguendo, che il piano lasciava aperte

**`reconciled_at` si scrive, e il piano non diceva quando.** Lo elencava fra le tre
colonne concesse dalla `0025` senza dire chi la valorizza. Scelta: la scrivono la
registrazione diretta e la conferma, perché in entrambe **una persona riconosce il
movimento** — che è la sola riconciliazione che questo MVP ha. Ne segue che
`payments_unreconciled_ix` resta vuoto oggi e comincerà a servire con i provider veri,
dove un webhook produce un `succeeded` che nessuno ha ancora guardato.

**`RejectPayment` non rilegge l'addebito.** Il piano diceva «tutti rileggono l'addebito
prima di scrivere, per la decisione 6». La decisione 6 riguarda però l'**inserimento**,
dove il trigger polimorfico è l'unico controllo: sul rifiuto il pagamento è stato
trovato sotto la RLS del chiamante, quindi l'organizzazione è già provata, e non c'è
niente da scrivere sull'addebito perché `pending` non contava. Una rilettura lì sarebbe
una query che non difende nulla. `ConfirmPayment` invece lo rilegge, perché gli serve
davvero.

**Le due vie convergono su `PaymentConfirmed`.** Il piano nominava tre eventi senza
dire quale emette la registrazione diretta, che nasce già `succeeded`. Emette
`PaymentConfirmed`: chi consuma reagisce al fatto che il denaro è diventato vero, non
alla porta da cui è passato, e a distinguere le due vie basta l'esistenza — o l'assenza
— del `PaymentDeclared` che porta lo stesso `payment_id`.

**L'addebito che si muove produce il proprio evento.** `billing.charge.status_changed`
viene emesso quando il ricalcolo cambia davvero lo stato, e non viene emesso quando non
lo cambia — `Charge::apply_payment` restituisce `false` e non si scrive né `UPDATE` né
evento né riga di audit. È la forma già stabilita dal riaddebito: due aggregati che si
muovono, due eventi.

**`Payable` ha una variante sola.** `app.payable_type` ne ha tre; `expense_share` e
`settlement` appartengono al modulo co-living e in lettura **danno errore** invece di
ricadere su `charge`. È la scelta di `AttachmentOwner`, e regge per la stessa ragione:
l'applicazione è l'unica a scrivere su quella tabella, quindi righe simili non possono
esistere.

**Il rifiuto non porta una motivazione.** `app.payments` non ha una colonna dove
metterla — `failure_message` descrive un fallimento presso un provider — e un evento
che trasportasse un testo libero scritto da una persona su un pagamento di un'altra
persona sarebbe il posto peggiore dove metterlo. Se servirà, è una colonna e una
migration.

#### Una verifica che non verificava niente

Vale la pena registrarla perché il sintomo era un verde, non un rosso.

Il primo controllo della `0029` su database vero rispondeva `UPDATE 0` a ogni scrittura
— quella che doveva essere rifiutata e quella che doveva passare — e letto in fretta
sembrava che la policy funzionasse. Non c'erano righe: il setup aveva inserito i due
membri con **una `INSERT` multi-riga**, e quella fallisce.

La ragione è la stessa che vieta l'`INSERT … RETURNING` sui gruppi.
`expense_group_members_insert` chiede `can_access_expense_group(group_id)` per chi non è
il creatore, e quella funzione è `STABLE`: dentro lo stesso statement vede lo snapshot
iniziale, in cui la membership del creatore **non c'è ancora**. Il creatore entra per il
ramo della `0026`, il secondo membro no, e l'intero statement cade.

Due statement separati funzionano, perché in `READ COMMITTED` ognuno prende uno snapshot
nuovo. Nessun codice dell'applicazione fa una `INSERT` multi-riga di membership, quindi
non è un difetto — ma è una proprietà dello schema che va saputa prima di scrivere un
setup di prova, e il modo in cui si manifesta è un test che non prova niente invece di
un errore.

La regola generale: **una verifica che risponde «zero righe» va guardata due volte**,
perché zero righe è anche ciò che risponde una verifica che non ha trovato niente da
verificare.

#### Come si è verificato

Il giro a mano che il piano chiedeva, tutto sotto test e non a mano:

- **la correzione delle viste al contrario** —
  `a_dropped_claim_no_longer_counts_as_outstanding` ricostruisce il caso documentato
  (800.00 emesso, 120.00 annullato, 40.00 scaduto) e legge
  `v_owner_dashboard.total_outstanding`: **840.00**, dove prima della `0024` erano
  960.00, con `overdue_charges` che smette di contare l'annullato. L'addebito annullato
  **resta leggibile**, con `outstanding_amount` a zero e `amount` intatto;
- **il flusso in due tempi** — `a_declaration_does_not_move_the_balance_and_the_confirmation_does`
  sulla persistenza e `a_declaration_becomes_real_only_when_the_staff_confirms_it` sul
  filo HTTP: l'inquilino dichiara e il saldo resta `800.00` insoluto con l'addebito
  `issued`; lo staff conferma e il saldo scende a zero con l'addebito `paid`;
- **l'idempotenza** — `the_same_idempotency_key_never_takes_the_money_twice` e il suo
  omologo HTTP: il secondo invio con la stessa chiave risponde `200`, restituisce lo
  stesso `id`, e il saldo è sceso **una volta sola**. Lo stesso test verifica il `409`
  sulla chiave riusata verso un altro addebito, e la versione HTTP verifica il `400`
  con l'header assente e il `422` con l'header malformato;
- **il rifiuto** — il saldo resta dov'era, l'addebito resta `issued`, `paid_at` torna
  nullo e una seconda conferma dà `409 payment.invalid_transition`;
- **l'isolamento** — un pagamento verso l'addebito di un'altra organizzazione riceve
  `404`, e il test dimostra prima che il trigger lo avrebbe accettato;
- **i privilegi di colonna** — sei `UPDATE` negate (`amount`, `payer_user_id`,
  `idempotency_key`, `refunded_amount`, `org_id`, `method`) e tre concesse (`status`,
  `paid_at`, `reconciled_at`), come letterali statici uno per uno: la regola 15 non
  ammette `format!` dentro `sqlx::query`, e il tipo lo impedisce anche in un test — il
  primo tentativo non compilava, il che è esattamente ciò che deve succedere.

#### Cosa resta fuori, e resta fuori davvero

I rimborsi (`refunded_amount` resta zero, `refunded` e `partially_refunded` non si
scrivono — pur essendo il secondo **contato** dalla vista); i provider veri e i loro
webhook (`provider` resta `manual`, e con lui `authorized` e `failed` restano non
scritti); i pagamenti di quote e regolamenti, che sono il modulo co-living; il pagamento
della bolletta al fornitore, che non è modellato affatto perché `app.payable_type` non
comprende `utility_bill`.

E la **dashboard**, che questa fetta ha reso *possibile* e non *prossima*. Fino a qui
due dei suoi numeri mentivano e adesso nessuno dei due mente più, ma la finestra su
quei numeri la chiude il primo client e non l'endpoint: sbloccata non è prioritaria, e
due delle sue semantiche restano da decidere. La distinzione è scritta in
[Prossimo passo suggerito](09-stato-e-roadmap.md#prossimo-passo-suggerito), perché è la conclusione sbagliata
che questa fetta invitava a trarre.

---

<a id="piano-spese-co-living-fetta-1"></a>

## Piano — Spese co-living, fetta 1

Pianificato ed eseguito il **1 settembre 2026**. Il piano resta sotto **come è stato
scritto**, senza correzioni a posteriori: serviva a eseguirlo, e serve a capire perché il
codice è com'è. L'esito è in fondo, come per documenti, segnalazioni, bollette e
pagamenti — ed è lì che si legge dove il piano aveva ragione, dove aveva ragione a metà,
e le tre cose che non aveva previsto.

### Obiettivo

Il denaro che i coinquilini si devono **fra loro**, che è una cosa diversa da tutto ciò
che il progetto ha fatto finora: non è una pretesa del proprietario verso l'inquilino,
è un conto fra pari. Il proprietario può guardarlo — è staff dell'organizzazione — ma
non ne è parte.

Uno compra la spesa per la casa, il sistema divide, ognuno vede cosa deve a chi, e chi
salda lo dichiara. È l'ultimo modulo dell'MVP che ha lo schema e non il codice, ed è
anche **l'unico che torna dentro codice già scritto**: `app.payable_type` ha tre valori
e il dominio dei pagamenti ne conosce uno.

**Dentro:** gruppi di spesa e composizione; registrazione di una spesa con ripartizione
`equal` e `weighted`; lettura dei saldi dalla vista; regolamenti fra due membri, con
conferma di chi riceve; correzione della vista dei saldi perché non perda chi è uscito
con un conto aperto; privilegi di colonna su `app.expenses`.

**Fuori, e non per dimenticanza:**

- **la semplificazione dei debiti** (il «chi paga chi» a numero minimo di bonifici): la
  vista dà il saldo netto per membro, e da quello un client sa già chi è in credito e
  chi in debito. L'algoritmo che minimizza i trasferimenti è una comodità di
  presentazione, non un fatto contabile, e nessuno l'ha chiesta;
- **`percentage`, `exact` e `by_usage`** come metodi di ripartizione: restano
  nell'enum non scritti, come `overdue` sugli addebiti. `by_usage` in più richiederebbe
  letture di contatori, che non esistono da nessuna parte;
- **`expense_shares.is_settled` e `settled_at`**: restano ai loro default. Vedi la
  decisione 7 — sono una seconda definizione di «saldato» che la vista non guarda;
- **il legame fra regolamento e pagamento**: `settlements.payment_id` resta nullo e
  `payments.payable_type = 'settlement'` resta non scritto. Vedi la decisione 6;
- **la ripartizione automatica di una bolletta** (`expenses.utility_bill_id`): il
  legame esiste a schema con il suo indice unico, ed è additivo. Richiede però di
  decidere *quale* bolletta di un immobile riguarda *quale* gruppo, che è una domanda a
  cui oggi non c'è modo di rispondere: `expense_groups.property_id` è opzionale;
- **gli allegati alle spese** (`expenses.document_id`): `AttachmentOwner` non comprende
  `expense`, e allargarlo è la stessa migration additiva già fatta per bollette e
  addebiti. Si fa quando serve, non adesso.

### Decisioni prese, con il perché

#### 1. Chi crea un gruppo ne diventa il primo membro, e serve una migration

Lo schema, letto com'è, produce un gruppo **inutilizzabile da chi lo ha creato**:

| | |
|---|---|
| `expense_groups_insert` | `org_id = current_org_id() AND is_org_member(org_id)` |
| `expense_group_members_insert` | `org_id = current_org_id() AND can_access_expense_group(group_id)` |
| `can_access_expense_group` | `is_org_staff(org_id) OR` esiste una membership attiva |

Un inquilino soddisfa la prima e **non** la seconda: appena creato il gruppo non ne è
membro, quindi non può aggiungere nessuno — nemmeno sé stesso. Il gruppo nasce vuoto e
resta vuoto, a meno che non passi lo staff.

La migration `0026` estende `expense_group_members_insert` con un secondo ramo: chi ha
creato il gruppo può iscrivere **sé stesso**, e nessun altro. `user_id =
app.current_user_id()` è la parte che non si può omettere — senza, chiunque crei un
gruppo potrebbe infilarci chiunque.

**Alternativa scartata: solo lo staff crea e popola.** Non richiede migration e dà
ragione allo schema come è, ma fa passare dal proprietario ogni coinquilino che arriva,
e un co-living che non può organizzarsi da sé non è il prodotto descritto in
[01 — Prodotto](01-prodotto.md).

**Alternativa scartata: allargare `can_access_expense_group` con `created_by`.** Una
riga sola e sbagliata: quella funzione governa anche la `SELECT`, quindi chi ha creato
un gruppo e ne è poi uscito continuerebbe a leggerne per sempre spese e saldi. Il
permesso che serve è stretto — iscrivere sé stessi, una volta — e va scritto stretto.

#### 2. Due metodi di ripartizione, e la colonna che li giustifica

`equal` divide fra i membri attivi. `weighted` usa `expense_group_members.weight`, che
lo schema ha già con il proprio commento — «peso per split `weighted` (es. stanza doppia
= 2)» — ed è il caso co-living vero: chi occupa la doppia paga di più.

Gli altri tre restano nell'enum e nessuno li scrive, per ragioni diverse fra loro:
`percentage` ed `exact` chiedono un input per membro, cioè una forma di richiesta
diversa dalle altre due, con la sua validazione e i suoi test, per un caso che nessuno
ha chiesto; `by_usage` chiede letture di contatori che non esistono.

`weight` è `NUMERIC(6,3)` mentre gli importi sono `NUMERIC(14,2)`: **il peso non è un
importo**, e passarcelo come `Money` è l'errore che il value object deve impedire — è la
stessa ragione per cui `Consumption` esiste separato nelle bollette.

#### 3. Il centesimo va ai primi in ordine, e l'ordine è per `user_id`

100,00 diviso tre fa 33,33 tre volte e avanza un centesimo. Le quote **devono** sommare
esattamente all'importo o il trigger `expense_shares_balanced` rifiuta la transazione,
quindi il resto non si può buttare: va assegnato a qualcuno.

La regola è quella di Splitwise: i centesimi di resto si distribuiscono **uno per uno ai
primi partecipanti**, in un ordine fisso. L'ordine è per `user_id` crescente — non per
data di iscrizione, non per nome — perché deve essere **riproducibile**: la stessa spesa
ricalcolata deve dare le stesse quote, e `joined_at` può cambiare se qualcuno esce e
rientra.

**Alternativa scartata: il resto a chi ha anticipato.** Più semplice da spiegare, e
concentra su una persona sola tutti i centesimi di tutte le spese che ha pagato lei —
che su un anno di spesa quotidiana non è più un arrotondamento.

**Alternativa scartata: a rotazione fra le spese.** La più equa e la peggiore da
implementare: richiede uno stato che ricordi a chi è toccato l'ultima volta, quindi una
colonna e una migration, e rende il calcolo di una quota non riproducibile guardando la
sola spesa.

**Vale per entrambi i metodi**, `weighted` compreso, dove la regola concorrente sarebbe
«il resto a chi ha la parte frazionaria più grande». Si sceglie la riproducibilità:
un'unica regola, un unico test, e nessuna sorpresa fra i due metodi.

#### 4. La vista non perde chi è uscito con un conto aperto

`v_expense_group_balances` parte da `app.expense_group_members` con
`WHERE m.left_at IS NULL`. Chi lascia il gruppo **sparisce dai saldi portandosi via il
debito**: il coinquilino che deve 200 euro esce, e quei 200 euro non risultano più a
nessuno.

È lo stesso difetto delle viste dei saldi degli addebiti — un numero che mente in
silenzio — e si corregge nello stesso posto e nello stesso modo: la migration `0027`
include i membri usciti **finché il loro saldo netto è diverso da zero**.

**Alternativa scartata: rifiutare l'uscita a saldo aperto.** Rende il difetto
irraggiungibile invece di correggerlo, non costa una migration, e ha un prezzo che si
paga sul caso reale: il coinquilino che se ne va davvero senza pagare resta membro per
sempre e il gruppo non può chiudersi. Il debito è del gruppo, non della membership.

**Alternativa scartata: accettare e documentare.** Sarebbe la seconda volta che questo
progetto lascia una vista mentire, e la prima è costata una migration correttiva e due
ADR. Si fa adesso, mentre nessun client legge quei numeri.

#### 5. Il regolamento è a due tempi, e conferma chi **riceve**

Chi paga dichiara di aver saldato — la riga nasce `pending` — e chi riceve conferma che
il denaro è arrivato. È la forma dei pagamenti, con una differenza che conta: là
confermava lo **staff**, qui conferma il **creditore**, perché è lui l'unico che sa.

Lo schema lo dice già, e non lo si sta forzando:

| | |
|---|---|
| `settlements_insert` | `from_user_id = current_user_id() OR is_org_staff(org_id)` |
| `settlements_update` | `to_user_id` **oppure** `from_user_id` **oppure** staff |
| `settlements.confirmed_by` | esiste, e la colonna dice a chi tocca |

`rejected` è l'uscita negativa e nasce insieme a `pending`, per la ragione di sempre:
uno stato in cui si entra e da cui non si esce è peggio di nessuno stato. `cancelled` è
l'uscita di chi ha dichiarato — la `USING` ammette anche `from_user_id`, quindi qui, a
differenza dei pagamenti, **chi dichiara può ritirare**. È l'asimmetria da dire ad alta
voce: fra pari il ritiro è lecito, verso il proprietario no.

**Solo il saldo `confirmed` muove i conti**: la vista somma `WHERE st.status =
'confirmed'`, e questo lo decide lei, non noi.

#### 6. Il regolamento **non** crea un pagamento, e le due strade restano una

`settlements.payment_id` è una foreign key verso `app.payments`, e `app.payable_type`
comprende `settlement`. Sono **due modi diversi** di legare la stessa coppia di righe, e
tenerli entrambi significa avere due risposte alla domanda «qual è il pagamento di
questo regolamento» — la stessa constatazione per cui `utility_bills.document_id` non è
popolata e la scansione passa dal riferimento polimorfico dei documenti.

In questa fetta **nessuno dei due si scrive**, e la ragione non è solo la simmetria: un
regolamento fra coinquilini non è denaro che l'organizzazione incassa, e farne una riga
in `app.payments` metterebbe nel registro degli incassi del proprietario un movimento
che non lo riguarda. Il giorno in cui servisse — perché il pagamento passa da un
provider vero — si sceglie **una** delle due strade e si scrive quella.

Ne segue che `Payable` resta a una variante sola. Il piano dei pagamenti prevedeva che
questo modulo la portasse a tre; la previsione era sbagliata, e va detto perché è
esattamente il genere di conclusione che si eredita senza riverificarla.

#### 7. `is_settled` non si scrive, e la fonte di verità resta la vista

`app.expense_shares` ha `is_settled` e `settled_at`, e
`v_expense_group_balances` **non li guarda**: il saldo netto è
`anticipato − dovuto + uscite − entrate`, tutto derivato dai movimenti.

Scriverli sarebbe una seconda definizione di «saldato» che nessuno riconcilia, cioè
esattamente il rapporto fra `charges.status` e `v_charge_balances` — con l'aggravante
che qui non esiste un'operazione che li produca: un regolamento è un **trasferimento
netto** fra due persone, e imputarlo alle singole quote richiederebbe un algoritmo di
allocazione che nessuno ha chiesto e che nessuna colonna registra.

Restano quindi a `false`, come `refunded_amount` sui pagamenti, e va saputo leggendoli.

#### 8. La spesa rilegge il gruppo, e il gruppo rilegge l'immobile

Terza e quarta occorrenza dello stesso rimedio, dopo `ensure_owner_is_visible` dei
documenti, `build_issued_charge` degli addebiti e `visible_charge` dei pagamenti.

`app.expense_groups` **non ha** il trigger `tg_assert_org_matches_property`, che invece
esiste su `rooms`, `leases`, `tickets` e `utility_bills` — verificato nella `0013`, non
dedotto. Quindi un `property_id` di un'altra organizzazione entra senza che nulla
protesti, e la riga resta a citare un immobile che chi l'ha scritta non vede.

Sulla spesa il problema è diverso e vale lo stesso rimedio: `expenses_insert` chiede
`can_access_expense_group(group_id)`, quindi il gruppo è già protetto dalla policy — ma
`paid_by_user_id` **no**, e nulla impedisce di dichiarare che ha anticipato una persona
che nel gruppo non c'è. Va verificato nello use case contro i membri attivi.

#### 9. Privilegi di colonna su `app.expenses`, e la ragione non è ADR-026

È una migration che nasce da un **buco vero**, non dall'applicazione di un criterio.

Il trigger `expense_shares_balanced` è `AFTER INSERT OR UPDATE OR DELETE ON
app.expense_shares`: guarda le **quote**. Aggiornare `app.expenses.amount` senza toccare
le quote **non lo fa scattare**, e lascia la somma delle quote diversa dall'importo
della spesa — l'invariante contabile che quel trigger esiste per difendere, aggirata
dalla porta accanto.

La `WITH CHECK` di `expenses_update` nomina il solo `org_id`, come sempre. La migration
`0028` revoca l'`UPDATE` e lo riconcede sulle colonne che una correzione legittima
tocca: `description`, `category`, `expense_date`, `notes`, `deleted_at`. Restano fuori
`amount`, `group_id`, `paid_by_user_id`, `split_method` e `currency`: cambiarli
significa descrivere un'altra spesa, e `amount` in particolare romperebbe l'invariante
in silenzio.

**Una spesa sbagliata si annulla e si riscrive**, come un addebito. La differenza con la
bolletta — dove ADR-026 ha dato «no» perché la GRANT avrebbe nominato quasi tutto — è
che qui l'elenco delle negate è corto e ognuna ha una ragione propria.

#### 10. Niente `INSERT … RETURNING` sul gruppo, e stavolta il muro c'è

`expense_groups_select` chiama `can_access_expense_group(id)`, cioè interroga **la riga
che si sta inserendo**, e la funzione è `STABLE`: vede lo snapshot di inizio statement,
in cui la riga non c'è. È il muro già incontrato sulle segnalazioni, e la regola
generale vale ancora: **il muro c'è solo quando la policy di `SELECT` dipende dalla riga
stessa**.

Sugli addebiti non c'era, perché `charges_select` interroga `app.leases`. Qui c'è.
L'identificatore lo assegna il dominio, come ovunque, quindi non se ne sente la
mancanza — ma va scritto, perché è la seconda volta e la prima è costata tempo.

### Decomposizione

**Il punto di taglio** è il passo 9, i regolamenti: senza di essi resta un registro di
spese con i saldi che si leggono e non si chiudono, che è una fetta coerente e
dichiarabile. Il passo 2 invece **non** si taglia: la vista corretta è ciò che rende i
saldi veri, ed è lo stesso argomento del passo 1 dei pagamenti.

1. **Migration `0026`: il creatore può iscrivere sé stesso.** Una funzione
   `app.created_expense_group(uuid)` `SECURITY DEFINER` con `SET search_path`, più
   `DROP`/`CREATE` di `expense_group_members_insert` con il secondo ramo. **La funzione
   non è un vezzo**: una sottoquery dentro una policy è soggetta alla RLS della tabella
   che legge, quindi un `EXISTS` su `app.expense_groups` scritto nella policy vedrebbe
   zero righe proprio per il creatore che deve autorizzare. Vedi le trappole.
2. **Migration `0027`: la vista che non perde chi è uscito con un conto aperto.**
   `CREATE OR REPLACE VIEW` conservando nomi, ordine e tipi delle colonne, con
   `security_invoker = true` ripetuto per esteso.
3. **Migration `0028`: privilegi di colonna su `app.expenses`.** Estende il controllo 12
   delle invarianti, mantenendo il nome. Verificare al contrario.
4. **Dominio: gli enum e i value object.** `ExpenseGroupKind`, `ExpenseCategory`,
   `SplitMethod` (cinque valori, due scritti), `SettlementStatus` (quattro, tutti
   scritti tranne `cancelled`… da decidere eseguendo), e `Weight` come value object con
   i tre decimali di `NUMERIC(6,3)`.
5. **Dominio: `ExpenseGroup` e la sua composizione.** `create`, `join`, `leave`,
   `archive`. `leave` valorizza `left_at` e **non** cancella la riga.
6. **Dominio: la ripartizione.** È il cuore aritmetico della fetta e va per primo fra le
   cose che calcolano: `Split::equal(amount, members)` e
   `Split::weighted(amount, members)`, entrambe che restituiscono quote la cui somma è
   **esattamente** l'importo. Test sulle soglie: 100,00 su 3; 0,01 su 3; 10,00 su 3 con
   pesi 1/1/2; un peso che porta una quota a 0,00.
7. **Dominio: `Expense`.** `record`, che costruisce spesa e quote **insieme** — non
   esiste una spesa senza quote, e il trigger lo impone al commit.
8. **Dominio: gli eventi.** `ExpenseRecorded`, `SettlementDeclared`,
   `SettlementConfirmed`. **Nessun importo viaggia**, come per i pagamenti.
9. **Dominio: `Settlement`** e la macchina a due tempi, con le transizioni ammesse e
   quelle rifiutate.
10. **Porte:** `ExpenseGroupRepository`, `ExpenseRepository` (con le quote, che si
    scrivono nella stessa transazione), `SettlementRepository`, e la lettura dei saldi
    dalla vista.
11. **Use case.** `CreateExpenseGroup` (e l'iscrizione del creatore nella stessa
    transazione), `AddGroupMember`, `LeaveGroup`, `RecordExpense`, `ListExpenses`,
    `GetGroupBalances`, `DeclareSettlement`, `ConfirmSettlement`, `RejectSettlement`.
12. **Infrastruttura.** I repository, con attenzione al `RETURNING` che qui non si può
    usare e al mapping dell'errore differito del trigger.
13. **Bordo HTTP.** Le rotte, i DTO, l'OpenAPI.
14. **Test.** Unitari sull'aritmetica della ripartizione e sulle due macchine; di
    persistenza con database reale — il trigger differito che rifiuta al commit, il
    creatore che si iscrive, il membro uscito che resta nei saldi se deve dei soldi, la
    spesa di un altro gruppo rifiutata; di contratto HTTP sull'intero giro.
15. **Documentazione.** `docs/05-api.md`, l'esito del piano, e **due ADR**: la regola di
    arrotondamento delle quote, e i privilegi di colonna che difendono un'invariante che
    il trigger non copre.

### Trappole

Tutte lette nello schema prima di scriverle qui; quelle marcate **da riprodurre** vanno
verificate su database reale in esecuzione.

**Una sottoquery dentro una policy subisce la RLS della tabella che legge.** È la
trappola del passo 1, ed è quella che farebbe perdere più tempo: scrivere
`EXISTS (SELECT 1 FROM app.expense_groups g WHERE g.id = group_id AND g.created_by =
app.current_user_id())` dentro `expense_group_members_insert` sembra corretto e non
funziona, perché quel `SELECT` passa da `expense_groups_select`, che chiede
`can_access_expense_group` — falsa proprio per il creatore che si sta cercando di
autorizzare. Serve una funzione `SECURITY DEFINER`, ed è la ragione per cui tutte le
`can_access_*` di questo progetto lo sono. **Da riprodurre**: la versione senza funzione
deve fallire.

**Il trigger `expense_shares_balanced` è `DEFERRABLE INITIALLY DEFERRED`: l'errore
arriva al `COMMIT`.** Non all'`INSERT`. In una Unit of Work questo significa che la
violazione esce da `uow.commit()` e non dalla query che l'ha causata, quindi il
`RepositoryError` va mappato **là**, e uno `unwrap` sul commit trasformerebbe un errore
di dominio in un `500`. **Da riprodurre**: quote che non sommano, e l'errore che compare
al commit.

**Il trigger è `FOR EACH ROW` su una tabella di quote.** Con quattro coinquilini scatta
quattro volte e ogni volta risomma tutte le quote. Irrilevante a queste dimensioni, e va
saputo prima di inserire quote a centinaia.

**Il trigger non guarda `app.expenses`.** Aggiornare `expenses.amount` da solo non lo fa
scattare e sbilancia l'invariante in silenzio. È il buco che la migration `0028` chiude,
ed è l'unica trappola di questo elenco che **non** si difende da sé.

**`INSERT … RETURNING` su `app.expense_groups` non restituisce la riga** al creatore non
staff. Il muro delle segnalazioni, seconda occorrenza.

**`expenses_bill_uk` è unico e parziale: una bolletta si ripartisce una volta sola.**
È l'asimmetria con il riaddebito degli addebiti, che è **ripetibile** apposta — la stessa
bolletta si riaddebita a più contratti. Qui no. Non ci tocca in questa fetta perché il
legame resta fuori, e va saputo dal giorno in cui entra.

**Tre tabelle su quattro non hanno `deleted_at`.** `expense_group_members` ha `left_at`,
`expense_shares` e `settlements` non hanno niente. Filtrare `deleted_at` per abitudine dà
un errore di colonna inesistente: fastidioso e innocuo, perché fallisce subito, ma è la
terza volta che questa nota serve dopo pagamenti e allegati.

**`settlements_confirmed_ck` vincola una sola direzione.** È
`status <> 'confirmed' OR confirmed_at IS NOT NULL`: **quarta occorrenza** dello stesso
pattern dopo `tickets_closed_ck`, `utility_bills_validated_ck` e `payments_succeeded_ck`.
Ormai non è più una trappola, è una convenzione dello schema: l'azzeramento tocca sempre
al dominio.

**`expense_group_members_group_user_uk` è unico su `(group_id, user_id)` e non è
parziale.** Chi esce e vuole rientrare **non** può avere una seconda riga: il rientro è
un `UPDATE` che rimette `left_at` a `NULL`, non un `INSERT`. Chi scrive `join` come un
inserimento incondizionato prende un conflitto sul vincolo.

**`weight` è `NUMERIC(6,3)` e gli importi sono `NUMERIC(14,2)`.** Un peso passato come
`Money` verrebbe rifiutato o troncato; è la stessa distinzione che `Consumption` fa
nelle bollette, e merita un value object per la stessa ragione.

**La somma delle quote deve tornare esattamente, e `Decimal` non arrotonda da sé.**
`rust_decimal` mantiene la scala che gli si dà: una divisione produce molte cifre, e una
quota costruita senza troncare a due decimali fa fallire il `CHECK` del dominio prima
ancora del trigger. La ripartizione va scritta come «tronca a due decimali, poi
distribuisci il resto», non come «dividi e arrotonda».

**`v_expense_group_balances` parte dai membri, non dalle spese.** Un utente che non è
mai stato membro non compare, anche se figura come `paid_by_user_id` di una spesa —
cosa che lo schema permette, perché su `expenses.paid_by_user_id` non c'è alcun vincolo
verso la membership. È la ragione della verifica applicativa della decisione 8.

### Come si saprà che è finita

`./ops/ci/test-integration.sh` verde con i test nuovi, e il controllo delle invarianti
verde con le colonne di `app.expenses` nell'elenco — verificato **anche al contrario**,
come per la fetta contabile e per quella dei pagamenti.

**L'aritmetica verificata sui casi che rompono**, e non solo su quelli che tornano:
100,00 fra tre dà 33,34 / 33,33 / 33,33 e somma 100,00; 0,01 fra tre dà 0,01 / 0,00 /
0,00; 10,00 fra tre con pesi 1/1/2 dà quote che sommano 10,00 esatti. Il criterio è uno
solo e vale per ogni caso: **la somma delle quote è l'importo**, altrimenti il trigger
rifiuta al commit.

**Il difetto della vista verificato al contrario**, come per la `0024`: un membro che
deve 200 euro esce dal gruppo, e il suo saldo **resta** nella vista; un membro a zero che
esce sparisce. Con la vista della `0015` il primo scompariva, e il test deve fallire su
quella definizione.

Un giro a mano contro il binario vero: un inquilino crea un gruppo e **si ritrova
dentro**; aggiunge due coinquilini; registra una spesa da 100,00 divisa in tre e i saldi
dicono +66,67 / −33,33 / −33,34; uno dichiara di aver saldato e **il saldo non si muove**
finché il creditore non conferma; una spesa con quote che non sommano viene **rifiutata
al commit**; e una spesa su un gruppo di un'altra organizzazione riceve `404`.

### Esito — che cosa l'esecuzione ha confermato, cambiato e trovato

Eseguito il **1 settembre 2026**, nell'ordine del piano. Le tre migration per prime, come
richiesto, e verificate su database vero **prima** di scrivere una riga di Rust.

La fetta è **completa**: nessun taglio al passo 9. I regolamenti sono dentro.

#### Le trappole del piano, verificate una per una

**«Una sottoquery dentro una policy subisce la RLS della tabella che legge»** — *vera, e
riprodotta*. Con un gruppo di cui il chiamante non è ancora membro:

```
sottoquery_in_policy | funzione_definer | can_access
---------------------+------------------+-----------
 f                   | t                | f
```

La versione senza funzione avrebbe prodotto un ramo che **non scatta mai**, e il sintomo
sarebbe stato identico a quello di prima della migration: gruppo creato, iscrizione
rifiutata. Il test `whoever_creates_a_group_is_already_in_it` esegue entrambe le versioni
e le confronta.

**Il trigger differito** — *vero*. `expense_shares_balanced` non fa fallire l'`INSERT`,
fa fallire il `COMMIT`. Riprodotto scrivendo a mano 50,00 di quote su una spesa da
100,00: l'inserimento riesce, e il commit risponde `does not match expense amount`. Da
qui `commit_expense` nel service layer, che mappa l'errore invece di lasciarlo diventare
un `500`.

**Il trigger non guarda `app.expenses`** — *vero, e peggio di come il piano lo
descriveva*. Riprodotto: `UPDATE app.expenses SET amount = 500.00` su una spesa con
100,00 di quote passa, **il commit riesce**, e il database resta con `importo_spesa =
500.00` e `somma_quote = 100.00`. Nessuna protesta. È la ragione della `0028`, ed è
diventato [ADR-035](08-decisioni.md#adr-035).

**`INSERT … RETURNING` sui gruppi** — *non riprodotta, e non serviva*. Nessuna query di
questa fetta ne aveva bisogno: gli identificatori li assegna il dominio. La nota resta
nel codice perché è la seconda occorrenza del muro e la prima era costata tempo.

**Il vincolo unico non parziale sulla membership** — *vera*. Il rientro è un `UPDATE`, e
il test lo fissa verificando che dopo un giro esci-e-rientra ci sia **una** sola riga.

**`settlements_confirmed_ck` vincola una sola direzione** — *vera, quarta occorrenza*.
L'azzeramento di `settled_at` sulle due uscite negative tocca al dominio.

**Tre tabelle su quattro senza `deleted_at`** — *vera*, e la nota è servita: le macro
`select_*` dei tre repository lo dicono in testa.

#### Le tre cose che il piano non aveva previsto

**1. La funzione della `0026` deve ancorare il gruppo all'organizzazione attiva, e il
piano non lo diceva.**

La decisione 1 indicava `user_id = app.current_user_id()` come «la parte che non si può
omettere». Ce n'è una **seconda**, e senza di essa la migration apre un buco fra
organizzazioni.

`app.current_org_id()` è una GUC impostata dall'applicazione e **non è una prova** — a
renderla un fatto sono le funzioni che rileggono la membership dal database, come dice
il commento di `app.set_request_context` dalla `0001`. Una funzione che verificasse il
solo `created_by` lascerebbe questo giro: chi ha creato un gruppo nell'organizzazione A
e ne è poi **uscito** può, con il contesto dell'organizzazione B di cui è ancora membro,
inserire una riga di membership verso il gruppo di A — e da quel momento
`can_access_expense_group` risponde vero per lui su un gruppo di un'organizzazione a cui
non appartiene più.

Verificato installando temporaneamente la versione ingenua:

```
--- Con la versione senza ancoraggio, la riga fra organizzazioni PASSA ---
INSERT 0 1
--- ...e da quel momento il gruppo di A gli è accessibile ---
 accesso_al_gruppo_di_a
------------------------
 t
```

Con `g.org_id = app.current_org_id() AND app.is_org_member(g.org_id)` dentro la
funzione, lo stesso inserimento è rifiutato. **È la cosa più cara imparata in questa
fetta**, e la regola generale che se ne ricava è: quando una funzione `SECURITY DEFINER`
legge una tabella per autorizzare una scrittura, deve verificare **anche** che la riga
letta appartenga all'organizzazione attiva — bypassare la RLS significa bypassare pure
l'isolamento, non solo la visibilità per ruolo.

**2. Un `RAISE EXCEPTION` di PL/pgSQL non ha un nome di vincolo, quindi il messaggio del
trigger non arriva al service layer.**

`map_sqlx_error` costruisce i dettagli di una `check_violation` dal **nome del
vincolo**, e un `RAISE EXCEPTION` non ne ha uno: `commit_expense` era stato scritto per
riconoscere l'errore dal testo (`details.contains("shares total")`), e non avrebbe mai
funzionato. Riconoscerlo dal messaggio era la via ovvia ed è sbagliata.

Non serve: `expense_shares_balanced` è **l'unico vincolo differito dell'intero schema**
— verificato, `DEFERRABLE` compare solo nella `0009` — e la transazione di
`RecordExpense` scrive una spesa, le sue quote e una riga di outbox. Un'integrità che si
manifesta al `COMMIT` di quella transazione è quella, e non può essere altro.

**3. Un rientro deve ridichiarare il peso, e il primo tentativo lo perdeva in silenzio.**

`AddGroupMember` sul ramo del rientro chiamava `member.rejoin(now)` e **ignorava** il
peso passato nel comando: la riga tornava attiva con il peso di prima. Il difetto è
uscito da un test — `rejoining_a_group_is_an_update_and_moves_the_joining_date` — e non
sarebbe uscito da nessun'altra parte, perché il repository *scriveva* correttamente il
peso dell'aggregato, che era quello vecchio.

La firma è diventata `rejoin(weight, now)`, con il parametro **obbligatorio**: con un
`Option` che ricade sul valore precedente, dimenticarlo sarebbe indistinguibile dal
volerlo. Il caso reale è concreto — chi rientra può prendere una stanza diversa, e
conservare il peso vecchio farebbe pagare la doppia a chi è tornato nella singola.

#### Scelte prese eseguendo, che il piano lasciava aperte

**1. Lo stato `cancelled` dei regolamenti si scrive.** Era l'unica domanda che il piano
lasciava aperta, e la risposta è **sì**.

La ragione non è di prodotto ed è una sola: la `USING` di `settlements_update` nomina
anche `from_user_id`, mentre `payments_update` è **staff soltanto**. Sui pagamenti «chi
dichiara non può ritirare» non era una scelta, era una conseguenza della policy,
documentata come controintuitiva. Qui la policy dice l'opposto, e importare il vincolo
dei pagamenti significherebbe copiarne la forma senza copiarne la ragione — che è
esattamente il genere di conclusione che la decisione 6 avverte di non ereditare.

Il fatto sostanziale è che **fra pari il ritiro è lecito, verso il proprietario no**:
chi ha sbagliato destinatario o importo non deve dover chiedere al creditore di
rifiutare una dichiarazione che sa già sbagliata. `pending` ha quindi **tre** uscite, e
tutte e quattro le varianti dell'enum si scrivono.

Verificato sul binario vero, non solo nel dominio: il test
`whoever_declared_a_settlement_can_withdraw_it_and_the_creditor_can_refuse_it` esegue le
tre transizioni contro PostgreSQL.

**2. Otto eventi invece dei tre del piano.** Il passo 8 elencava `ExpenseRecorded`,
`SettlementDeclared` e `SettlementConfirmed`. Ne sono stati scritti otto, e le aggiunte
hanno due ragioni diverse:

- **`SettlementRejected` e `SettlementCancelled`** chiudono un difetto: senza, chi
  consuma l'outbox vedrebbe la dichiarazione e mai un esito, restando convinto che il
  regolamento sia ancora pendente. È lo stesso argomento per cui l'uscita negativa nasce
  insieme allo stato da cui esce;
- **`ExpenseGroupCreated`, `ExpenseGroupMemberJoined` e `ExpenseGroupMemberLeft`** per
  coerenza: ogni altro use case che scrive in questo progetto accoda un evento, e
  `CreateExpenseGroup` sarebbe stato il primo a non farlo. La membership **non** è un
  aggregato a sé — gli eventi portano `aggregate_type = "ExpenseGroup"` — come già per
  `LeasePartyAdded` e `TicketMessagePosted`.

Nessuno degli otto porta importi, e nemmeno testo libero: la descrizione di una spesa
dice che cosa una persona ha comprato. Viaggiano categoria, metodo, numero di quote e
identificatori.

**3. `ArchiveExpenseGroup` e `ListSettlements` esistono, e il piano non li elencava.**
Non sono allargamenti di comodo, sono due buchi:

- senza una via per archiviare, `ExpenseGroup::ensure_open` non può mai fallire e la
  guardia è codice morto. Il passo 5 chiedeva `archive` nel dominio: senza lo use case
  quel metodo non sarebbe stato raggiungibile da nessuna parte;
- senza un elenco dei regolamenti, il creditore non ha **alcun modo** di trovare la
  dichiarazione da confermare — l'unico accesso sarebbe conoscerne l'identificatore,
  cioè averlo ricevuto per un canale che non esiste. `?status=pending` è servito
  dall'indice parziale `settlements_pending_ix`, che esiste dalla `0009` proprio per
  questa domanda.

**4. Si esce da un gruppo per sé stessi, e la restrizione è diventata un confine vero
con la `0029`.** Il piano elencava `LeaveGroup` senza dire chi può eseguirlo. La `USING`
di `expense_group_members_update` ammette **qualunque membro attivo del gruppo**, quindi
la RLS lasciava passare una query scritta a mano che toglie un altro.

La prima risposta è stata restringere nel service layer e **registrare il resto come
debito**, perché il passo 3 del piano nominava `app.expenses` e non le membership. Il
debito è stato poi chiuso dentro la stessa fetta, insieme ai privilegi di colonna, ed è
la `0029`.

**La restrizione ovvia avrebbe rotto il rientro**, ed è la trappola di quella migration.
Viene naturale stringere la `USING` con
`user_id = app.current_user_id() OR app.is_org_staff(org_id)`. Non si può: reintegrare
un coinquilino è un `UPDATE` sulla riga di **un altro** — il vincolo unico non è
parziale, quindi chi è uscito non può avere una seconda riga — e con quella `USING` un
membro non staff non potrebbe più far rientrare nessuno.

A stringere è la **`WITH CHECK`**, che vede la riga nuova: un membro non staff può
scriverla solo se il risultato ha `left_at IS NULL`, cioè può **reintegrare e non
togliere**. È il corollario aggiunto ad [ADR-035](08-decisioni.md#adr-035): un privilegio
di colonna dice quali colonne, una `WITH CHECK` dice in che verso.

Il test omonimo ha cambiato di segno: la seconda metà asseriva che la policy lasciasse
passare, e ora assicura che rifiuti.

**Resta ammesso, di proposito, che un membro cambi il peso di un altro membro attivo**:
il gruppo deve poter registrare che qualcuno si è spostato nella doppia, e farlo passare
dal proprietario renderebbe il co-living dipendente da lui per un fatto interno alla
casa. Oggi nessuna rotta lo espone — `AddGroupMember` su un membro attivo è rifiutato dal
dominio — e il giorno in cui una lo esponesse va deciso allora.

**5. Il destinatario di un regolamento non si verifica contro i membri, e la spesa sì.**
Sembra un'incoerenza e non lo è: si salda con chi è **uscito** dal gruppo, ed è
precisamente ciò che la `0027` esiste per rendere possibile — il debitore uscito resta
nei saldi finché non paga. Un controllo sui membri attivi renderebbe irraggiungibile la
chiusura di quel conto. Su `paid_by_user_id` il controllo invece serve, perché un
credito intestato a un estraneo non comparirebbe in nessun saldo.

**6. La correzione di una spesa non ha un endpoint.** La `0028` concede l'`UPDATE` su
cinque colonne, e nessuno use case le scrive: una spesa sbagliata si annulla e si
riscrive. Le concessioni restano perché la migration nasce per **negare** `amount`, e
l'elenco delle concesse è ciò che rende la revoca applicabile senza rompere le
correzioni legittime del giorno in cui arriveranno.

#### Una verifica che non verificava niente

Vale la pena registrarla perché il sintomo era un verde, non un rosso.

Il primo controllo della `0029` su database vero rispondeva `UPDATE 0` a ogni scrittura
— quella che doveva essere rifiutata e quella che doveva passare — e letto in fretta
sembrava che la policy funzionasse. Non c'erano righe: il setup aveva inserito i due
membri con **una `INSERT` multi-riga**, e quella fallisce.

La ragione è la stessa che vieta l'`INSERT … RETURNING` sui gruppi.
`expense_group_members_insert` chiede `can_access_expense_group(group_id)` per chi non è
il creatore, e quella funzione è `STABLE`: dentro lo stesso statement vede lo snapshot
iniziale, in cui la membership del creatore **non c'è ancora**. Il creatore entra per il
ramo della `0026`, il secondo membro no, e l'intero statement cade.

Due statement separati funzionano, perché in `READ COMMITTED` ognuno prende uno snapshot
nuovo. Nessun codice dell'applicazione fa una `INSERT` multi-riga di membership, quindi
non è un difetto — ma è una proprietà dello schema che va saputa prima di scrivere un
setup di prova, e il modo in cui si manifesta è un test che non prova niente invece di
un errore.

La regola generale: **una verifica che risponde «zero righe» va guardata due volte**,
perché zero righe è anche ciò che risponde una verifica che non ha trovato niente da
verificare.

#### Come si è verificato

`./ops/ci/test-integration.sh` verde: **343** test in tutto, 50 dei quali nuovi. **58**
di persistenza (12 nuovi), **64** di contratto HTTP (4 nuovi) e **181** unitari di
dominio (34 nuovi).

Il controllo delle invarianti dello schema verde con le otto colonne di `app.expenses` e
le tre di `app.expense_group_members` nell'elenco, **verificato al contrario**: con
`GRANT UPDATE (amount, group_id)` aggiunta a mano, il controllo 12 riporta

```
 colonna-strutturale-scrivibile | expenses.amount
 colonna-strutturale-scrivibile | expenses.group_id
```

e la pipeline si ferma.

**L'aritmetica verificata sui casi che rompono**, non solo su quelli che tornano: 100,00
fra tre dà 33,34 / 33,33 / 33,33; 0,01 fra tre dà 0,01 / 0,00 / 0,00; 10,00 fra tre con
pesi 1/1/2 dà 2,50 / 2,50 / 5,00. E un test esaustivo su **1 750 combinazioni** — ogni
importo da 0,01 a 2,50 per ogni numero di partecipanti da uno a sette, con pesi uniformi
e misti — verifica l'unico criterio che conta: **la somma delle quote è l'importo**.

**Il difetto della vista verificato al contrario**, come per la `0024`. Sullo stesso
gruppo, con un debitore da 33,33 uscito e uno in pari uscito:

| | vista `0015` | vista `0027` |
|---|---|---|
| righe | 1 | 2 |
| somma dei saldi | **33,33** | **0,00** |

Con la definizione vecchia il debito spariva e restava un credito senza contropartita.

**Il giro a mano, contro il router vero** (`a_coliving_group_splits_a_hundred_euros_and_closes_the_account`):
un inquilino crea un gruppo e si ritrova dentro; aggiunge due coinquilini; registra
100,00 divisi in tre e le quote sommano a 100,00; uno dichiara di aver saldato e **il
saldo non si muove**; chi paga prova a confermare a sé stesso e riceve `403
settlement.not_addressed_to_you`; il creditore trova la dichiarazione con
`?status=pending` e conferma; i due escono, il debitore resta nei saldi e chi era in
pari sparisce; il gruppo si archivia con un conto ancora aperto e rifiuta le spese
successive con `409 expense_group.archived`, mentre i saldi restano leggibili.

#### Cosa resta fuori, e resta fuori davvero

Tutto ciò che il perimetro del piano elencava, senza aggiunte: la **semplificazione dei
debiti**; i tre metodi di ripartizione oltre `equal` e `weighted`;
`expense_shares.is_settled` e `settled_at`, che restano ai loro default e che la vista
non guarda; il legame fra regolamento e pagamento, per cui `settlements.payment_id`
resta nullo e `payments.payable_type = 'settlement'` non si scrive; la ripartizione
automatica di una bolletta; gli allegati alle spese.

**`Payable` resta a una variante**, e vale la pena ripeterlo: il piano dei pagamenti
prevedeva che questo modulo la portasse a tre, e la previsione era sbagliata. Un
regolamento fra coinquilini non è denaro che l'organizzazione incassa.

Due cose **non** previste dal piano restano fuori e vanno segnate, perché si sono viste
eseguendo:

- **non esiste un endpoint che elenchi le persone di un'organizzazione**, e i corpi di
  questo modulo nominano le persone. È più grave di come è sembrato scrivendo la fetta, e
  la forma esatta si vede solo riproducendola: con un proprietario, due immobili e
  contratti **per stanza**, un inquilino vede un solo contratto e una sola parte
  contrattuale — sé stesso — quindi **non può popolare il gruppo**. I test di questa
  fetta non lo mostrano perché usano un contratto unico condiviso dai tre, che è l'altro
  caso di co-living;
- **e l'anagrafica, all'opposto, è visibile per organizzazione**: nello stesso scenario
  quell'inquilino vede la riga `app.users` anche degli inquilini dell'altro immobile,
  perché `shares_org_with` guarda l'organizzazione e non l'immobile. Nessun endpoint la
  espone, quindi oggi non è una fuga — ma è il perimetro che un elenco delle persone
  erediterebbe scrivendolo sulla RLS così com'è. **L'immobile non è il confine né in un
  senso né nell'altro**, ed è la cosa da sapere prima di disegnare quell'endpoint;
- **`app.expense_group_members` non ha privilegi di colonna**, quindi un membro attivo
  può riscrivere `user_id` o `weight` di chiunque nel gruppo. È lo stesso criterio di
  ADR-026 applicabile a una quarta tabella, e non è stato fatto perché fuori dal
  perimetro dichiarato del passo 3, che nominava `app.expenses`.

Entrambe sono registrate fra i debiti noti.

---

<a id="piano-ocr-bollette-fetta-1"></a>

## Piano — OCR delle bollette, fetta 1

Pianificato il **1 settembre 2026**. Il piano resta sotto come è stato scritto: serve a
eseguirlo, e servirà a capire perché il codice è com'è. L'esito va in fondo a esecuzione
conclusa, come per documenti, segnalazioni, bollette, pagamenti e spese co-living.

### Obiettivo

La lettura automatica di una bolletta. Lo staff registra la bolletta in bozza, ci allega
la scansione, e da lì una macchina legge il documento e propone i campi — che **nessuno
scrive al posto della persona**: la proposta finisce in `ocr_extracted`, la bolletta
passa a `pending_review`, e una persona corregge e valida.

È il primo modulo del progetto in cui **una macchina scrive nel database**, e quasi
tutte le decisioni qui sotto discendono da quella frase.

**Dentro:** la porta `OcrProvider` in `application::ports`; un adattatore deterministico
per i test; la tabella `app.ai_jobs` scritta per la prima volta; il ciclo di lavoro nel
worker con `FOR UPDATE SKIP LOCKED`; una migration per la policy di accodamento; lo
sblocco di `pending_review` nella macchina a stati della bolletta; la lettura dello
stato di un job dall'API.

**Fuori, e non per dimenticanza:**

- **il fornitore vero.** È la decisione 1, ed è la più discussa: si definisce la porta e
  si rimanda la scelta. Ne segue che questa fetta **non produce valore per un utente
  finale**, e va detto invece che lasciarlo scoprire;
- **il triage delle segnalazioni** (`ticket_triage`, `ticket_reply_draft`).
  `tickets.ai_job_id` e `ticket_messages.ai_job_id` restano nulli. Non è una seconda
  chiamata allo stesso adattatore: è decidere se una macchina possa scrivere categoria e
  priorità di una segnalazione e con quale soglia, che è una domanda di prodotto sua;
- **le capability `vision`, `llm` ed `embedding`**: restano nell'enum non scritte, come
  `percentage` fra i metodi di ripartizione;
- **la fotografia che crea la bolletta.** È la promessa del `README`, e richiede di
  caricare un documento prima di avere una bolletta a cui allegarlo, più una decisione
  su cosa fare quando l'estrazione non basta a costruire una riga valida — e
  `app.utility_bills` ha colonne `NOT NULL` che l'OCR può non trovare. Vedi la
  decisione 4: l'innesco scelto riusa tutto ciò che esiste;
- **un tetto di spesa per organizzazione.** `ai_jobs.cost_amount` e `ai_jobs_cost_ix`
  esistono perché quel controllo sia possibile, e questa fetta li **valorizza senza
  farne niente**: senza un fornitore vero il costo è zero, e un limite tarato su zero
  non si può provare;
- **il ritentativo automatico.** `attempts` esiste e questa fetta la incrementa; a
  decidere se e quando riprovare serve sapere quali errori un fornitore vero produce.

### Decisioni prese, con il perché

#### 1. Si definisce la porta e **non** si sceglie il fornitore

La roadmap diceva il contrario — «i trait dei provider vanno definiti allora,
verificando la reference del fornitore scelto in quel momento» — e la ragione per
scostarsene è che la scelta del fornitore **non è tecnica**.

Un LLM con visione leggerebbe la bolletta e ne estrarrebbe i campi strutturati in un
colpo solo, e servirebbe poi anche al triage da immagine. Ma l'immagine della bolletta
**esce dal perimetro**: fornitore, indirizzo, importi e spesso il codice cliente
finiscono presso terzi. Il commento della `0010` è esplicito nel tenere prompt e
immagine fuori da `ai_jobs`, e questa fetta lo rispetta — ma il dato al fornitore ci va
comunque, ed è una decisione di trattamento che va presa da chi risponde del
trattamento, non dedotta mentre si scrive un adattatore.

Tesseract in locale non ha quel problema e ne ha un altro: fa OCR e non estrazione
strutturata, quindi i campi andrebbero ricavati con euristiche nostre sul testo grezzo,
e sulle bollette italiane — impaginazione diversa per ogni fornitore — l'accuratezza
sarebbe bassa abbastanza da rendere il modulo un fastidio invece di un aiuto.

**Il rischio di questa scelta va nominato**, perché è reale: una porta disegnata senza
un fornitore davanti rischia di essere inventata, cioè di descrivere ciò che è comodo a
noi invece di ciò che i fornitori danno. Si mitiga in due modi, e nessuno dei due lo
elimina:

- **la porta chiede il minimo comune.** Un `Vec<u8>` con il suo MIME in ingresso; in
  uscita i campi trovati, ciascuno con la propria confidenza, più i contatori che
  `ai_jobs` ha già (token, costo, latenza, modello). Tutto ciò che un fornitore
  specifico offre in più — bounding box, pagine, alternative — resta fuori: aggiungerlo
  dopo è additivo, toglierlo no;
- **il primo adattatore vero è il momento di rileggerla**, ed è scritto qui perché chi
  lo scriverà lo trovi. Se la porta si rivelasse sbagliata, cambiarla allora costa un
  adattatore finto e nessun client.

**Alternativa scartata: aspettare la decisione sul fornitore e non fare la fetta.** È la
scelta onesta se il modulo servisse a un utente domani. Non serve: quello che questa
fetta compra è il **percorso** — coda, worker, job, stato, revisione — che è
indipendente dal fornitore e che è dove stanno le trappole.

#### 2. `app.ai_jobs` **non è scrivibile** dal ruolo applicativo, e serve una migration

È la scoperta che cambia la forma della fetta, e non si legge dai privilegi: si legge
dalle policy.

La `0015` concede a `propflow_app` `SELECT, INSERT, UPDATE` su **tutte** le tabelle
dello schema, `ai_jobs` compresa. Ma la `0014` crea per quella tabella **una sola
policy**, `ai_jobs_select`, con il commento «INSERT/UPDATE su ai_jobs: solo
propflow_worker (BYPASSRLS)». Con RLS attiva e nessuna policy per un comando, quel
comando è **negato**: il privilegio c'è e la policy no.

Ne segue che accodare un job dall'API fallisce, e fallisce con
`auth.denied_by_policy` — il codice riservato ai nostri difetti.

**Si estende la RLS, non si aggira.** È [ADR-020](08-decisioni.md#adr-020) alla lettera:
il pool privilegiato è per i flussi in cui un contesto di sicurezza **non può**
esistere, e qui esiste — chi carica la scansione è autenticato e staff. La migration
aggiunge `ai_jobs_enqueue`, una policy di `INSERT` per lo staff dell'organizzazione, con
`status = 'queued'` **dentro la `WITH CHECK`**.

`status = 'queued'` nella policy e non solo nel codice è la stessa forma di
`role = 'tenant'` in `org_members_accept_invitation` della `0018`: una richiesta HTTP non
deve poter inserire un job che si dichiara già `succeeded` con un `output` scritto a
mano, nemmeno con una query scritta a mano. L'aggiornamento resta del solo worker, che
ha `BYPASSRLS` e non passa da alcuna policy.

**Alternativa scartata: far accodare il worker, che sonda le bollette da leggere.**
Nessuna migration, e in cambio la coda diventa **derivata** — «le bollette in bozza con
un allegato e senza `ai_job_id`» — cioè un secondo stato da tenere allineato con
`ai_jobs`, che è la coda vera. È la stessa ragione per cui `is_settled` non si scrive.

**Alternativa scartata: passare dall'outbox.** Il caricamento emette un evento, il
worker lo consuma e crea il job. Architetturalmente è la più elegante — l'outbox esiste,
il worker gira già con `BYPASSRLS` — e costa molto di più di quanto sembri: oggi
`EventBus` ha una sola implementazione che **scrive nei log**, non un instradamento verso
consumatori. Costruirlo qui significherebbe fare il dispatcher degli eventi dentro la
fetta dell'OCR, e allora sarebbe quello il piano.

#### 3. `pending_review` sblocca due invarianti che oggi congelerebbero la bolletta

Il dominio, come è scritto, rende `pending_review` uno **stato senza uscite**:

| | |
|---|---|
| `BillStatus::is_editable` | `matches!(self, Draft \| Validated)` — `pending_review` **no** |
| `UtilityBill::validate` | `if !matches!(self.status, Draft)` — rifiuta |

Una bolletta portata a `pending_review` non si potrebbe quindi né correggere né
validare: `UpdateUtilityBill` e `ValidateUtilityBill` risponderebbero entrambi
`utility_bill.invalid_transition`, e l'unica uscita resterebbe l'annullamento. Sarebbe
il difetto che questo progetto rifiuta per principio — uno stato in cui si entra e da
cui non si esce — e lo introdurremmo noi, in un modulo il cui scopo è **proporre
qualcosa da correggere**.

Due modifiche di dominio, nessuna migration:

- `is_editable` comprende `PendingReview`. Correggere ciò che la macchina ha proposto è
  l'operazione per cui lo stato esiste;
- `validate` accetta `Draft` **o** `PendingReview`. La revisione umana è la stessa in
  entrambi i casi: una persona dice che i numeri sono quelli.

`utility_bills_validated_ck` non tocca `pending_review` — vincola `validated`,
`allocated` e `paid` — quindi non c'è niente da fare a schema.

**Nota di attenzione per chi esegue**: `apply` riporta indietro da `validated` a `draft`
(la correzione disfa la validazione, ADR-029). Va deciso e scritto **esplicitamente**
cosa fa `apply` da `pending_review`: resta `pending_review` finché non si valida, oppure
scende a `draft`? La risposta proposta è **resta**, perché `draft` significherebbe che
la proposta della macchina non c'è più mentre `ocr_extracted` è ancora piena; ma è una
riga di codice che va scritta guardandola, non ereditata dal ramo esistente.

#### 4. L'OCR **non scrive i campi della bolletta**

Scrive `ocr_extracted`, `ocr_confidence` e `ai_job_id`, e porta lo stato da `draft` a
`pending_review`. `supplier_name`, `total_amount`, `invoice_number` e gli altri restano
quelli che ha digitato la persona, finché una persona non li cambia.

È già la posizione della roadmap e va ripetuta qui perché eseguendo verrà la tentazione
opposta — riempire i campi è «più utile». Lo schema tiene separato ciò che ha letto la
macchina (`ocr_extracted`, `ocr_confidence`) da ciò che ha confermato una persona
(`validated_by`, `validated_at`) **proprio perché l'accuratezza si possa misurare**:
riempire le prime colonne con valori umani, o le seconde con valori macchina, distrugge
quel dato prima di raccoglierlo. Senza quella misura non c'è modo di rispondere alla
sola domanda che conta su un modulo di OCR — funziona abbastanza?

Ne segue anche che `utility_bills_dedup_uk` non è in gioco: l'OCR non scrive
`invoice_number`, quindi non può produrre il conflitto di deduplica.

#### 5. `ai_jobs.output` non duplica `ocr_extracted`

Sono due JSONB e la tentazione di scriverci la stessa cosa è forte. Il confine:

- **`utility_bills.ocr_extracted`** porta i **valori** proposti. È il dato della
  bolletta, sta sulla bolletta, ed eredita la RLS della bolletta — che è più stretta di
  quella di `ai_jobs`, perché comprende l'inquilino imputato;
- **`ai_jobs.output`** porta la **forma** dell'esito: quali campi sono stati trovati e
  con quale confidenza ciascuno, senza i valori. Serve a misurare l'accuratezza per
  campo senza rileggere le bollette, ed è ciò che l'indice `ai_jobs_output_ix` — `gin
  (output jsonb_path_ops)` — permette di interrogare.

Il commento della `0010` chiede che in `ai_jobs` vadano «SOLO parametri e riferimenti»,
mai il prompt con PII e mai l'immagine. I valori estratti sono il contenuto della
bolletta, quindi stanno dalla parte della bolletta.

#### 6. Il job gira nel **worker**, non nell'API

`ai_jobs_queue_ix` è parziale su `('queued','running')` e il commento della `0010` dice
per cosa esiste: `SELECT … FOR UPDATE SKIP LOCKED`. Lo schema è disegnato per un
consumatore che compete con altri, e non lo si sta forzando.

Una chiamata sincrona dall'handler terrebbe aperta una richiesta HTTP per i secondi di
un fornitore esterno, legherebbe l'esito del **caricamento** alla disponibilità del
fornitore, e moltiplicherebbe il costo per ogni ritentativo dell'utente. Il caricamento
deve riuscire anche quando l'OCR non è disponibile: la bolletta resta in bozza con il
suo allegato, che è esattamente ciò che si ha oggi.

`SKIP LOCKED` è ciò che rende il worker replicabile senza coordinamento, e va usato
anche con una replica sola: aggiungerlo dopo significa scoprire il difetto quando le
repliche diventano due, cioè sotto carico.

#### 7. La confidenza bassa **non è un fallimento**

`app.confidence` è `NUMERIC(5,4)` fra 0 e 1. Una lettura con confidenza bassa produce
comunque un job `succeeded` e una bolletta in `pending_review`: è un segnale per chi
rivede, non un errore. Il fallimento è il fornitore che non risponde o rifiuta il
documento, e allora il job va a `failed` con `error_code`/`error_message`, e **la
bolletta resta in `draft`** — dove era, con il suo allegato, correggibile a mano come se
l'OCR non esistesse.

**Nessuna soglia di rifiuto automatico in questa fetta.** Sceglierla richiede di aver
visto le confidenze di un fornitore vero su bollette vere; inventarla adesso significa
scartare letture buone o accettarne di cattive, e in entrambi i casi non lo si saprebbe.

#### 8. Nessun evento di dominio per il ciclo del job

È l'unica scrittura di questo progetto che **non** accoda in outbox, e va motivato
perché rompe una regolarità.

Un job dell'OCR non è un fatto di dominio: è un dettaglio di come una bolletta arriva a
`pending_review`. Il fatto che chi consuma vuole conoscere è **la bolletta che ha
cambiato stato**, e quello lo annuncia già `billing.utility_bill.status_changed`, che
esiste dalla fetta contabile. Accodare `ai.job.queued` e `ai.job.succeeded` darebbe a
un consumatore tre eventi per un fatto solo, due dei quali su un aggregato che nessuno
fuori da qui conosce.

`ai_jobs` ha invece la propria riga di `audit_log`, come ogni tabella di business, ed è
lì che si guarda per ricostruire cosa ha fatto la macchina.

> **Questa frase è sbagliata, e l'esecuzione l'ha trovata.** `app.ai_jobs` è
> **esclusa** dall'audit dalla `0013`, per volume, insieme a `ticket_messages`,
> `user_sessions`, `domain_events` e `ticket_status_history`. La conclusione della
> decisione — niente eventi di dominio per il ciclo del job — resta valida, ma il suo
> puntello no: la traccia di ciò che una macchina ha fatto **alla bolletta** sta nella
> riga di `audit_log` di `app.utility_bills`, con `ocr_extracted` fra le colonne
> redatte. Vedi [l'esito](#esito-ocr-bollette-fetta-1).

### Decisioni prese prima di eseguire, che il piano lasciava aperte

Decise il **2 settembre 2026**, chiudendo la fetta precedente. Stanno qui e non nella
sezione sopra perché quella è il piano com'era: queste sono le tre risposte alle domande
che il piano marcava «da decidere eseguendo», e vanno lette prima di aprire il codice.

#### A. `cancelled` resta nell'enum e il dominio non lo scrive

`app.ai_job_status` ha cinque valori; `AiJobStatus` ne muove **quattro**. È la stessa
scelta di `percentage` fra i metodi di ripartizione: il valore esiste nello schema,
l'applicazione non lo scrive mai, e lo si dichiara invece di lasciarlo scoprire.

Il precedente dei regolamenti — dove `cancelled` fu costruito insieme al resto — non si
applica: là annullare è un'azione che una persona compie su una dichiarazione che ha
fatto lei, qui un job si chiude in millisecondi e **non c'è nulla da annullare** finché
non esiste un fornitore vero che impieghi secondi. Una rotta di annullamento sarebbe
codice per un caso che non si può nemmeno provare.

C'è anche una domanda che scriverlo aprirebbe e che non conviene aprire adesso:
`ai_jobs_finished_ck` esige `finished_at` sui soli `succeeded` e `failed`, quindi un job
annullato non avrebbe una data di fine — e cosa significhi un terminale senza data di
fine è una decisione a sé. Il giorno in cui `cancelled` servirà, si deciderà anche
quella, e probabilmente insieme al ritentativo, che è l'altra cosa fuori perimetro.

#### B. Il passo 8 sta dentro: la fetta è intera

Tutti gli **11 passi**. Il punto di taglio esisteva ed è stato guardato: senza il passo 8
il percorso funziona e si osserva solo da database e log, quindi il giro a mano del
criterio di completamento andrebbe fatto con `psql`. È una rotta di **sola lettura** su
una risorsa che esiste già — costa poco — e senza di essa la fetta non è ispezionabile
da fuori, che per un modulo il cui valore è *il percorso* è precisamente la parte da
poter mostrare.

#### C. Un processo con due task, non due binari

Il piano propendeva per due binari e la ragione era buona: un job OCR lungo affamerebbe
il dispatcher dell'outbox. **Quel carico però non esiste in questa fetta**, perché
l'adattatore è deterministico e la decisione 1 rimanda il fornitore vero — e tararsi su
un'ipotesi costa un'immagine Docker, un servizio nello stack compose, una voce in
`docs/07-operazioni.md` e una modifica a `release.yml`.

Due `tokio::spawn` in `propflow-worker`, quindi: il dispatcher dell'outbox e il ciclo
dell'OCR, stesso binario, secondo compito.

**Separare dopo è un refactor e non un ridisegno**, ed è ciò che rende la scelta
reversibile: il ciclo sta dietro `AiJobRepository`, quindi spostarlo in un binario suo
non tocca né il dominio né gli use case. **Il momento di rifarsi la domanda è il primo
adattatore vero**, che è anche il momento in cui il piano dice di rileggere la porta —
sono la stessa conversazione, e vale la pena affrontarle insieme.

### Decomposizione

**Il punto di taglio** era il passo 8, la lettura dello stato del job dall'API, e la
decisione B lo tiene **dentro**: senza, resterebbe un percorso completo osservabile solo
dai log e dal database. Il passo 3 non si taglia in nessun caso: senza lo sblocco di
`pending_review` la fetta produce bollette congelate, cioè peggio di niente.

1. **Migration `0031`: la policy di accodamento.** `ai_jobs_enqueue` per l'`INSERT`,
   staff dell'organizzazione, con `status = 'queued'` nella `WITH CHECK`. Verificare **al
   contrario**: senza la policy l'inserimento dall'API è negato, e con la policy un
   `INSERT` che dichiara `succeeded` è comunque rifiutato.
2. **Dominio: `AiJob`.** `AiCapability`, `AiJobStatus` — l'enum del database ha cinque
   valori e il dominio ne scrive **quattro**, vedi la decisione A — `AiTask`, e la
   macchina `queued → running → succeeded | failed`. `ai_jobs_finished_ck` esige `finished_at`
   sui due terminali: è la **quinta** occorrenza del `CHECK` a una direzione sola, e
   ormai è una convenzione dello schema.
3. **Dominio: `pending_review` smette di essere un vicolo cieco.** `is_editable` e
   `validate` come dice la decisione 3, più la scelta esplicita su cosa fa `apply` da
   `pending_review`. Test sulle transizioni **rifiutate**, non solo su quelle ammesse.
4. **Porta `OcrProvider`** in `application::ports`, accanto a `ObjectStorage` e
   `Notifier`. Un metodo, `read`, con i byte e il MIME in ingresso e i campi trovati con
   la loro confidenza in uscita, più i contatori che `ai_jobs` registra. `StorageError`
   è il modello da seguire per l'errore.
5. **Adattatore deterministico** in `propflow-infrastructure`, accanto a
   `LoggingEventBus`. Non è un segnaposto: rende osservabile e testabile il percorso
   prima che esista un fornitore, che è la stessa ragione per cui `LoggingEventBus`
   esiste. Deve poter simulare le tre uscite — lettura buona, lettura a confidenza
   bassa, fallimento — perché i test le coprano tutte.
6. **Porta `AiJobRepository` e repository.** `enqueue`, `claim_next` con `FOR UPDATE
   SKIP LOCKED`, `mark_running`, `finish`. Attenzione: `claim_next` gira come worker e
   **non** passa da `UnitOfWorkFactory::begin`, che installa un contesto RLS che il
   worker non ha — va guardato come lo fa oggi il dispatcher dell'outbox.
7. **Use case e innesco.** `EnqueueBillOcr`, chiamato dal caricamento di un allegato a
   una bolletta; `ProcessAiJob`, che è il corpo del ciclo del worker e che al successo
   scrive `ocr_extracted`, `ocr_confidence`, `ai_job_id` e porta la bolletta a
   `pending_review`.
8. **Bordo HTTP:** lo stato del job sulla bolletta. Una rotta di lettura, e `null`
   finché nessuno ha caricato niente.
9. **Ciclo nel worker**, accanto al dispatcher dell'outbox: stesso binario, secondo
   compito, due `tokio::spawn` nello stesso processo. Vedi la decisione C.
10. **Test.** Unitari sulla macchina del job e sulle due invarianti sbloccate; di
    persistenza con database reale — la policy della `0031` al contrario, `SKIP LOCKED`
    con due worker simulati che non prendono lo stesso job, il fallimento che lascia la
    bolletta in `draft`; di contratto HTTP sul giro completo con l'adattatore
    deterministico.
11. **Documentazione.** `docs/05-api.md`, `docs/03-modello-dati.md`, l'esito, e **un
    ADR**: perché una porta si definisce prima di avere un fornitore, e a quali
    condizioni è onesto farlo.

### Trappole

Tutte lette nello schema e nel codice prima di scriverle qui; quelle marcate **da
riprodurre** vanno verificate su database reale in esecuzione.

**`ai_jobs` ha il privilegio e non la policy.** `propflow_app` può fare `INSERT` secondo
la `GRANT` della `0015` e **non** secondo la RLS, perché la `0014` crea per quella
tabella la sola `ai_jobs_select`. Con RLS attiva e nessuna policy per un comando, quel
comando è negato. È la trappola che farebbe perdere più tempo, perché guardando i
privilegi si conclude che si può. **Da riprodurre**: l'inserimento senza la `0031` deve
fallire.

**`pending_review` è un vicolo cieco nel dominio di oggi.** `is_editable` non lo
comprende e `validate` accetta il solo `draft`: una bolletta portata lì non si corregge
e non si valida. **Da riprodurre**: prima della modifica del passo 3, entrambe le
operazioni devono rispondere `utility_bill.invalid_transition`.

**Il worker non ha un contesto RLS, e `UnitOfWorkFactory::begin` ne pretende uno.**
`begin_privileged` esiste ed è riservata ai flussi pre-autenticazione dalla regola 20 di
`CLAUDE.md`. Il ciclo dell'OCR non è uno di quelli, e non deve diventarlo per comodità:
il worker gira già come `propflow_worker` con `BYPASSRLS` e con il proprio pool, e va
guardato come il dispatcher dell'outbox risolve la stessa cosa oggi. **Questa è la
trappola che rischia di allargare la superficie di `begin_privileged`**, che è la cosa
che la regola 20 esiste per impedire.

**`FOR UPDATE SKIP LOCKED` va nella transazione che poi aggiorna il job.** Prendere il
job in una transazione, chiuderla e lavorare fuori significa che due worker possono
prendere lo stesso job. Lavorare **dentro** la transazione significa tenerla aperta per
i secondi della chiamata al fornitore, che su un pool di connessioni è caro. La via è la
terza: transazione breve che porta il job a `running` — che lo toglie dall'indice
parziale, perché `ai_jobs_queue_ix` copre `queued` e `running` — e una seconda
transazione che lo chiude. **Da riprodurre**: due worker simulati non devono prendere lo
stesso job.

**Un job `running` che il processo abbandona resta `running` per sempre.** Non c'è
scadenza, e `attempts` da sola non lo rimette in coda. È fuori perimetro — il
ritentativo lo è — ma va **saputo**, perché il sintomo è una bolletta che non arriva mai
a `pending_review` senza che nulla sia in errore.

**`ai_jobs_finished_ck` vincola una sola direzione**: `status <> ANY('succeeded','failed')
OR finished_at IS NOT NULL`. **Quinta occorrenza** dopo `tickets_closed_ck`,
`utility_bills_validated_ck`, `payments_succeeded_ck` e `settlements_confirmed_ck`. Non
è più una trappola, è una convenzione: l'azzeramento tocca sempre al dominio.

**`cost_amount` è `NUMERIC(12,6)` e non `app.money_amount`.** È una delle due eccezioni
volute del controllo 8 delle invarianti — l'altra è la frequenza del canone — perché il
costo di una chiamata AI ha sei decimali. Passarlo come `Money` lo troncherebbe a due, e
un costo di 0,000012 diventerebbe zero. Merita un value object separato, per la stessa
ragione per cui `Weight` e `Consumption` esistono.

**`subject_type` è `app.attachment_owner_type`, non un enum suo.** Comprende undici
valori mentre `AttachmentOwner` nel dominio ne ha sei: leggere un job con un
`subject_type` fuori perimetro deve dare **errore**, come già fa
`attachment_owner_type_from_db`.

**`ai_jobs` non ha `deleted_at`.** Terza tabella dopo quelle del co-living e i
pagamenti; il ruolo applicativo non ha `DELETE`. Filtrare `deleted_at` per abitudine dà
un errore di colonna inesistente.

**Il caricamento apre già due transazioni** — è un debito noto — e accodare il job ne
aggiungerebbe una terza se lo si facesse fuori. Va accodato **nella stessa transazione
che registra il documento**: un job che punta a un documento la cui riga è stata
annullata è un job che fallirà leggendo, e l'avrebbe fatto dopo, nel worker, dove
l'errore non ha più niente a che vedere con la causa.

**`input_document_id` è `ON DELETE SET NULL` e i documenti hanno il soft delete.** Un
documento archiviato lascia il job a puntare a una riga che c'è e che l'API non mostra
più. Il worker lo leggerà comunque, perché gira con `BYPASSRLS`: va deciso se un job su
un documento archiviato debba fallire, e la risposta proposta è **sì**, perché
altrimenti la macchina legge ciò che una persona ha deciso di togliere.

### Come si saprà che è finita

`./ops/ci/test-integration.sh` verde con i test nuovi, e il controllo delle invarianti
verde.

**Le due trappole di partenza verificate al contrario**, che è l'unica prova che conta:
l'inserimento in `ai_jobs` dall'API deve fallire **senza** la `0031`, e la validazione
di una bolletta in `pending_review` deve fallire **prima** della modifica del passo 3.
Una suite verde dopo non prova nulla da sola.

**`SKIP LOCKED` verificato con due worker**, non con uno: due transazioni concorrenti
che chiamano `claim_next` devono ottenere due job diversi, e la seconda deve ottenere
`None` quando ce n'è uno solo.

**Il fallimento verificato quanto il successo**: un fornitore che rifiuta il documento
lascia il job `failed` con il suo `error_code` e la bolletta **in `draft`**, correggibile
a mano come se l'OCR non esistesse.

Un giro a mano contro il binario vero: lo staff registra una bolletta in bozza, ci
allega una scansione, il worker prende il job e lo porta a `succeeded`, la bolletta
compare in `pending_review` con `ocr_extracted` piena e `ocr_confidence` valorizzata,
una persona corregge un campo — e la bolletta **resta** in `pending_review` — poi valida,
e da lì il riaddebito funziona come nella fetta contabile.

<a id="esito-ocr-bollette-fetta-1"></a>

### Esito — che cosa l'esecuzione ha confermato, cambiato e trovato

Eseguita il **3 settembre 2026**, tutti e undici i passi. Suite verde: **402 test**
(erano 352), nessun `SALTATO`, invarianti dello schema pulite.

#### Le due trappole di partenza, riprodotte al contrario prima di scrivere il codice

Sono la sola prova che conta, e sono state prodotte **prima** della migration e prima
del passo 3, non dopo.

**`app.ai_jobs` ha il privilegio e non la policy.** Sullo schema alla `0030`, con il
contesto di sicurezza installato e l'utente owner dell'organizzazione:

```
 privilegio_insert            policyname   |  cmd
-------------------          ----------------+--------
 t                            ai_jobs_select | SELECT

ERROR: new row violates row-level security policy for table "ai_jobs"
```

Il privilegio dice di sì, la policy non c'è, l'inserimento è negato. Esattamente come
scritto, e la ragione per cui la trappola è cara è tutta in quella prima riga.

**`pending_review` era un vicolo cieco.** Sul dominio prima del passo 3, con una
bolletta costruita da persistenza in quello stato:

```
apply da pending_review    -> Err(Invariant(UtilityBillNotEditable))
validate da pending_review -> Err(Invariant(UtilityBillNotValidatable))
is_editable(PendingReview) = false
```

Due invarianti diverse, **un codice solo** — `utility_bill.invalid_transition` — quindi
un client avrebbe visto una bolletta che non si corregge e non si valida senza alcun
modo di distinguere il perché.

#### La cosa più utile imparata: `SKIP LOCKED`, tolto e misurato

Il piano diceva «due worker non devono prendere lo stesso job», e l'aspettativa
implicita era che senza `SKIP LOCKED` ne prendessero uno solo in due. **Non è quello
che succede.** Tolto `SKIP LOCKED` dalla query di claim e rieseguito il test:

```
secondo claim: Backend { details: "57014: canceling statement due to statement timeout" }
```

Il secondo worker **non** prende il job dell'altro — la riga è bloccata, PostgreSQL fa
il suo lavoro — ma si ferma ad aspettare, e resta fermo finché lo `statement_timeout`
non lo uccide. Sotto carico è **peggio** del doppio lavoro: le connessioni del pool si
esauriscono aspettando righe che qualcun altro sta già lavorando, e il sintomo che si
vede è «il worker è lento», non «il worker sbaglia». Il test resta scritto come il
piano lo chiedeva, perché il modo in cui fallisce non cambia ciò che deve garantire; il
commento nel repository ora dice quale fallimento aspettarsi.

#### Quattro cose che il piano non aveva previsto

**1. Il piano dice una cosa falsa su `audit_log`, e va corretta qui.** La decisione 8
afferma che «`ai_jobs` ha invece la propria riga di `audit_log`, come ogni tabella di
business». **Non ce l'ha**: la `0013` la esclude esplicitamente dall'audit per volume,
insieme a `ticket_messages`, `user_sessions`, `domain_events` e `ticket_status_history`,
e `docs/03-modello-dati.md` lo diceva già. La conclusione della decisione 8 — niente
eventi di dominio per il ciclo del job — **resta valida**, ma il suo puntello no: la
traccia di ciò che una macchina ha fatto **alla bolletta** non sta in `ai_jobs`, sta
nella riga di `audit_log` di `app.utility_bills`, che il trigger scrive con
`ocr_extracted` fra le colonne redatte. Il registro dei job resta in `ai_jobs` stessa, e
ciò che lo rende affidabile non è l'audit ma l'assenza di una policy di `UPDATE`.

**2. Il rifiuto della RLS su un `UPDATE` è muto, e va verificato apposta.** La `0031`
aggiunge una policy di `INSERT` e nessuna di `UPDATE`. Un `INSERT` rifiutato **solleva**
(`new row violates row level security policy`); un `UPDATE` senza policy no: la riga
semplicemente non è visibile all'aggiornamento, e il risultato è `UPDATE 0`. È la
quarta volta che questo progetto incontra la forma — è la stessa di
`ticket_reference_counters` — ed è la ragione per cui i repository trattano
`rows_affected() == 0` come `RepositoryError::RowLevelSecurity`. Un test che si fosse
limitato a «l'`UPDATE` fallisce» sarebbe passato senza che la difesa esistesse.

**3. Il worker ha bisogno dello storage, e questo è configurazione nuova.** Il piano
non lo nomina, e non è gratis: `propflow-worker` leggeva **una sola** variabile
d'ambiente e ora ne legge sei — le tre obbligatorie dello storage più tre con default.
Vanno aggiunte al deployment **prima** di rilasciare l'OCR, e devono puntare allo stesso
bucket dell'API: due configurazioni divergenti danno job `failed` con
`document_unavailable` su documenti che l'API mostra benissimo. Documentate in
[07 — Operazioni](07-operazioni.md#impostazioni-del-worker), aggiunte a
`docker-compose.yml` e a `.env.example`. Il worker le legge con `std::env::var` come il
migratore, e per la stessa ragione: `AppConfig` gli chiederebbe un bind e delle chiavi
di firma che non usa.

**4. La suite ha avuto bisogno di un lock, e la ragione è il codice, non i test.**
`claim_next` **non filtra per organizzazione** — il worker serve tutte, e deve — mentre
ogni test ha il proprio doppio dello storage. Due test dell'OCR in parallelo sullo
stesso database producono quindi il difetto peggiore: il worker dell'uno prende il job
dell'altro, non trova l'oggetto nel proprio bucket, e lo chiude `failed` con
`document_unavailable`. Non è un difetto del codice — in produzione lo storage è uno —
ma della suite, che simula una configurazione impossibile. La cura è un
`pg_advisory_xact_lock` condiviso fra i due binari di test, con l'invariante «accodare e
smaltire dentro la stessa sezione critica». `xact` e non di sessione: si rilascia con la
transazione, quindi anche se un test va in panico — un lock di sessione su una
connessione che torna al pool bloccherebbe la suite intera.

#### Il difetto che ha trovato il giro a mano e non la suite

Nella stessa risposta HTTP, la confidenza complessiva usciva come `"1.0000"` e quelle
per campo come `"1"`. Due grafie dello stesso numero, in un corpo solo.

La causa è che la scala di un `Decimal` dipende da come è stato costruito: quella
calcolata dall'adattatore ha scala zero, quella riletta da `NUMERIC(5,4)` ha scala
quattro. È **lo stesso difetto che `format_amount` risolve per gli importi**, e nessun
test lo vedeva perché tutti confrontavano `Confidence` con `Confidence` — cioè
`Decimal` con `Decimal`, dove `1` e `1.0000` sono uguali — invece che la stringa che
esce sul filo.

La cura sta nel tipo e non al bordo: `Display for Confidence` rende sempre quattro
decimali, e da lì passano sia il bordo HTTP sia il codificatore JSON. Metterla al bordo
avrebbe lasciato il codificatore libero di scrivere l'altra grafia in `ocr_extracted`,
cioè lo stesso difetto un livello più in basso, dove si vede ancora meno.

#### Scelte prese eseguendo, che il piano lasciava aperte

**Cosa fa `apply` da `pending_review`: la bolletta ci resta.** Il piano proponeva
questa risposta e chiedeva di guardarla invece di ereditarla. Guardata, e confermata,
per una ragione che il piano non dava per intero: da `validated` la correzione
**smentisce un'affermazione** — qualcuno ha detto «questi numeri sono quelli» — quindi
la validazione va disfatta; da `pending_review` non c'è nessuna affermazione da
smentire, c'è una proposta di una macchina, e **correggerla è la revisione**, non il suo
annullamento. Scendere a `draft` direbbe per giunta una cosa falsa — «non c'è niente da
rivedere» — mentre `ocr_extracted` è ancora piena.

Ne segue un'asimmetria che vale la pena avere scritta: `pending_review → validated →
apply` finisce in **`draft`** e non torna in revisione. È corretto — lì una persona ha
già rivisto la proposta — ed è coperto da un test suo.

**Un job su un documento archiviato fallisce.** Anche qui la risposta proposta era «sì»,
ed è stata confermata: il worker gira con `BYPASSRLS` e la riga la vedrebbe comunque, ma
`DocumentRepository::find_by_id` filtra `deleted_at IS NULL` e quindi torna `None` anche
a lui. Fallire è la scelta perché altrimenti la macchina legge ciò che una persona ha
deciso di togliere, **e glielo ripropone come se fosse ancora lì**. Il codice è
`document_archived`, distinto da `document_unavailable` — che è l'oggetto assente nel
bucket — perché i due rimedi sono opposti: il primo è una decisione di una persona, il
secondo un'incoerenza fra database e storage.

**La confidenza complessiva è il minimo, non la media.** Il piano non lo chiedeva e va
deciso, perché `ocr_confidence` è una colonna sola e i campi sono undici. Con la media,
dieci campi letti a `0.99` e il **totale** letto a `0.40` darebbero `0.94`: una bolletta
che sembra a posto e ha sbagliato l'unica cifra che qualcuno pagherà. Il minimo dice
«c'è qualcosa da guardare», che è ciò per cui il numero esiste; la confidenza per campo
resta comunque leggibile in `ocr_extracted`, quindi non si perde niente — si sceglie
solo cosa dice il riassunto.

**L'accodamento scatta solo su una bolletta in bozza, e il `kind` del documento non
conta.** La prima condizione evita di riportare a «da rivedere» una bolletta che una
persona ha già validato o riaddebitato. La seconda è la scelta opposta a quella che
verrebbe istintiva: vincolare a `DocumentKind::UtilityBill` significherebbe che l'OCR
non parte quando qualcuno classifica la scansione come `invoice` — altrettanto
ragionevole — e il sintomo sarebbe una funzione che «a volte non funziona» senza che
nulla lo dica. Un allegato a una bolletta *è* la scansione di quella bolletta.

**`WorkerUnitOfWork` è un tipo a sé, e non un terzo metodo di `UnitOfWorkFactory`.** È
la risposta alla trappola che il piano segnalava come la più pericolosa — quella che
allarga la superficie di `begin_privileged()`. Il worker ha il **proprio** pool, come il
dispatcher dell'outbox, e da quella unità di lavoro si ottengono **tre** repository e
non diciotto: la superficie senza RLS resta leggibile in venti righe invece di essere
«tutto, ma con un altro pool». `AiJobQueue` — `claim_next`, `update` — è raggiungibile
**solo** da lì, quindi una transazione applicativa non ha modo di far avanzare un job.
Il confine è nel sistema di tipi e non in una convenzione, che è ciò che la regola 4 di
`CLAUDE.md` chiede davvero.

#### Come si è verificato

| Cosa | Come |
|---|---|
| le due trappole di partenza | riprodotte **prima**, contro lo schema `0030` e contro il dominio non modificato |
| la policy `ai_jobs_enqueue` | tre direzioni: `queued` ammesso, `succeeded` rifiutato, altra organizzazione rifiutata |
| l'assenza di policy di `UPDATE` | `UPDATE 0` verificato esplicitamente, perché non solleva |
| `SKIP LOCKED` | **due worker concorrenti** con due job, più un terzo che deve ottenere `None`; e la query senza `SKIP LOCKED`, che va in timeout |
| l'indice parziale `ai_jobs_queue_ix` | `EXPLAIN`: `Bitmap Index Scan` con `Recheck Cond: status = ANY ('{queued,running}')` |
| il confine fra le due colonne JSONB | su righe vere: nessun valore della bolletta compare in `ai_jobs.output`, e `output @> '{"fields":["total_amount"]}'` trova il job |
| il fallimento | quanto il successo: job `failed` con il suo `error_code`, bolletta **in `draft`**, e la richiesta che ha caricato riuscita |
| il giro a mano | API e worker veri, con MinIO: bozza → scansione → `pending_review` → correzione (resta) → validazione → riaddebito |
| le tre uscite dell'adattatore | tutte contro il binario vero: lettura pulita (`1.0000`), scansione sporca (`0.8125`, job comunque `succeeded`), JPEG rifiutato (`unsupported_document`) |

Il giro a mano, per intero:

```text
bolletta in bozza          totale 100,00   fornitore assente     status draft
scansione allegata         job queued      la bolletta non si muove
worker                     job succeeded   status pending_review  confidence 1.0000
                           11 campi proposti, e i campi della bolletta intatti
correzione a mano          totale 184,20   fornitore "Enel Energia S.p.A."
                           status pending_review  ← resta, e la proposta è ancora lì
validazione                status validated
riaddebito                 addebito issued 184,20  status allocated
                           la proposta è leggibile anche da allocated
```

#### Cosa resta fuori, e resta fuori davvero

**Il fornitore vero.** È [ADR-037](08-decisioni.md#adr-037), ed è la decisione che
questa fetta ha deliberatamente non preso: mandare l'immagine di una bolletta a un
terzo è una decisione di trattamento dei dati, e spetta a chi risponde del trattamento.
Ne segue, e va detto senza girarci intorno, che **questa fetta non produce valore per un
utente finale**: quello che compra è il percorso.

**Il ritentativo automatico.** `attempts` esiste e questa fetta lo incrementa alla presa
in carico — così un processo morto a metà lascia traccia di averci provato — ma nessuno
riporta a `queued` un job abbandonato. Ne segue il debito sotto.

**Il tetto di spesa per organizzazione.** `ai_jobs.cost_amount` e `ai_jobs_cost_ix`
esistono perché sia possibile; l'adattatore deterministico non chiama nessuno, quindi il
costo è **nullo** — non zero, che sembrerebbe una misura — e un limite tarato sul nulla
non si può provare.

**Il triage delle segnalazioni.** `tickets.ai_job_id` e `ticket_messages.ai_job_id`
restano nulli. Non è una seconda chiamata allo stesso adattatore: è decidere se una
macchina possa scrivere categoria e priorità di una segnalazione e con quale soglia, che
è una domanda di prodotto sua.

**La fotografia che crea la bolletta.** È la promessa del `README` e richiede di
caricare un documento prima di avere una bolletta a cui allegarlo, più una decisione su
cosa fare quando l'estrazione non basta a costruire una riga valida —
`app.utility_bills` ha colonne `NOT NULL` che l'OCR può non trovare.

**Una soglia di rifiuto automatico.** Sceglierla richiede di aver visto le confidenze di
un fornitore vero su bollette vere; inventarla adesso significa scartare letture buone o
accettarne di cattive, e in entrambi i casi non lo si saprebbe.

**Una rotta che lanci l'OCR a comando.** Legherebbe una richiesta HTTP ai secondi di un
fornitore esterno, che è la decisione 6 rovesciata.


---

<a id="piano-coinquilini-visibili-fetta-1"></a>

## Piano — I coinquilini si vedono, fetta 1

Pianificato il **2 settembre 2026**. Il piano resta sotto come è stato scritto: serve a
eseguirlo, e servirà a capire perché il codice è com'è. L'esito va in fondo a esecuzione
conclusa, come per le fette precedenti.

### Obiettivo

**Chi abita la stessa casa si vede, e chi abita altrove no.** Sono due metà di una cosa
sola, e oggi sono sbagliate tutte e due in direzioni opposte.

È il pezzo che manca perché le spese co-living siano usabili nel caso reale: chi crea un
gruppo non può aggiungerci nessuno, perché non ha modo di ottenere l'identificatore dei
propri coinquilini.

**Viene prima dell'OCR** — [piano qui](#piano-ocr-bollette-fetta-1) — per una ragione di
ordine e non di dimensione: l'OCR aggiunge valore nuovo, questo rende utilizzabile valore
**già scritto**. Il modulo delle spese, così com'è, funziona solo con un contratto unico
condiviso.

**Dentro:** una funzione che restituisce i residenti di un immobile; la restrizione di
`users_select` dal perimetro dell'organizzazione a quello della casa; un endpoint;
il collegamento con l'aggiunta di un membro a un gruppo di spesa.

**Fuori, e non per dimenticanza:**

- **allargare `lease_parties_select` o `leases_select`.** Erano la prima idea ed è stata
  **misurata sbagliata**: vedi la decisione 1. Nessuna policy di lettura si allarga in
  questa fetta;
- **un elenco delle persone dell'organizzazione.** Il perimetro è la **casa**, ed è
  precisamente ciò che il piano decide;
- **l'invito diretto a un gruppo di spesa.** Il gruppo si popola scegliendo fra i
  residenti dell'immobile, che è chi ci abita davvero. Invitare nel gruppo qualcuno che
  non abita lì aprirebbe una via di ingresso parallela a quella dei contratti;
- **la notifica.** La porta `Notifier` resta senza adattatore.

### Che cosa è stato misurato prima di decidere

Tutto ciò che segue è stato **riprodotto su database vero**. Scrivendo la fetta co-living
il ragionamento a mente su questa RLS ha dato la risposta sbagliata due volte, e la prima
stesura di questo piano una terza.

Lo scenario: un proprietario, **due immobili**, cinque inquilini con contratti **per
stanza** (`lease_type = 'room'`, il co-living vero). X, Y, Z nella Casa 1; A, B nella
Casa 2. Tutti membri della stessa organizzazione, che è ciò che l'accettazione di un
invito produce.

| Sono… | Anagrafica: di chi leggo la riga `app.users` | Parti contrattuali |
|---|---|---|
| **X** | A, B, Proprietario, X, Y, Z | **solo X** |
| **A** | A, B, Proprietario, X, Y, Z | **solo A** |
| Proprietario | tutti e sei | tutti e cinque |

**L'immobile non è il confine, né in un senso né nell'altro.** Sui contratti la
visibilità è più stretta — il singolo contratto — mentre sull'anagrafica è più larga:
`shares_org_with` guarda l'organizzazione, quindi X legge la riga di A e B, che abitano
altrove e con cui non ha alcun rapporto.

### Decisioni prese, con il perché

#### 1. I residenti si leggono da una **funzione**, e nessuna policy si allarga

Era naturale pensare di estendere `lease_parties_select` al perimetro dell'immobile,
riusando `can_access_property`. **È stato provato e non funziona**, e la ragione per cui
non funziona è anche la ragione per cui è un bene che non funzioni.

Allargata la policy, questo è ciò che X vede:

| | X vede |
|---|---|
| `app.lease_parties` da sola | X, Y, Z ✓ |
| unita a `app.leases` | **solo X** ✗ |
| contratti visibili | **1** |

`leases_select` è `can_access_lease(id)`, cioè **per contratto**, e qualunque query che
unisca le parti ai contratti — e l'endpoint deve unirle, per filtrare l'immobile e i
contratti in essere — rifiltra tutto. Farlo funzionare avrebbe richiesto di allargare
**anche** `leases_select`, e da lì **X avrebbe visto il canone di Y**: `leases.rent_amount`
sta sulla riga. Un prezzo che la prima stesura di questo piano aveva accettato per
l'email dei coinquilini e nemmeno visto per il canone.

**Si fa invece una funzione `SECURITY DEFINER` che restituisce righe** invece di un
booleano — `app.property_residents(p_property_id)` → `(user_id, full_name, room_name)` —
con il confine **dentro**: risponde solo a chi `can_access_property` ammette già.

È lo stesso pattern delle `can_access_*`, che leggono tabelle senza riapplicare le
policy; cambia solo che questa restituisce un insieme. **Il confine resta nel database**,
perché la funzione è l'unica via e il controllo è nel suo corpo — non è una regola del
service layer.

Verificato, con `lease_parties_select` e `leases_select` **rimaste come sono**:

| Sono… | Residenti Casa 1 | Residenti Casa 2 | Contratti visibili |
|---|---|---|---|
| **X** | X, Y, Z | **nessuno** | 1 |
| **A** | **nessuno** | A, B | 1 |

Nessun canone e nessuna email escono, perché nessuna riga in più diventa leggibile: esce
solo ciò che la funzione proietta.

**Ne segue che la decisione sull'email non esiste più.** La prima stesura la trattava
come un compromesso da accettare e dichiarare; con questo disegno non c'è compromesso da
fare, ed è il segno che il disegno è migliore e non solo diverso.

#### 2. `users_select` si restringe alla casa, ed è metà dell'obiettivo

Che X veda A e B è un difetto, non un dettaglio: sono persone che abitano altrove, e la
riga `app.users` porta nome, **email** e **telefono**.

Oggi non lo espone nessun endpoint, quindi non trapela niente. Ma è un **pavimento**, e
questo progetto ha appena passato una migration — la `0029` — a smettere di fidarsi di
regole applicative su confini che il database lascia aperti. Applicare qui un metro
diverso sarebbe incoerente.

La policy diventa:

```
id = app.current_user_id()          io stesso
OR app.shares_home_with(id)         chi abita la mia stessa casa
OR app.is_staff_of_user(id)         lo staff vede le persone delle proprie organizzazioni
OR app.is_my_staff(id)              e l'inquilino vede lo staff della propria
OR app.is_platform_admin()
```

I due rami sullo staff non sono decorazione. Senza il terzo il proprietario perderebbe di
vista i propri inquilini; senza il quarto un inquilino non potrebbe **mai** sapere il nome
del proprio proprietario, e `properties.owner_id` punterebbe a una riga illeggibile.

**Restringere oggi non rompe niente, ed è misurato**: riducendo `users_select` al solo
`id = app.current_user_id() OR app.is_platform_admin()` la suite resta interamente verde
— **122 test** fra persistenza e contratto HTTP. Nessuna query legge `app.users` per
qualcuno che non sia il chiamante: `app.users` compare in tre query, tutte in `user.rs`,
tutte nei flussi di autenticazione.

**E quel verde non prova che restringere sia giusto**, che è la parte da non
fraintendere: prova che la capacità per cui `shares_org_with` esiste — «mostrare i nomi
delle controparti», dice il commento della `0014` — **non è mai stata costruita**. È lo
stesso equivoco già registrato nella fetta contabile, dove un test che passava da uno
strato non provava nulla sullo strato sotto. La finestra per restringere è aperta e la
chiude il primo client che legga un nome da lì.

**Perché i nomi non servono da `app.users`**: lo schema li **denormalizza dove si
mostrano** — `lease_parties.full_name` ed `email`, `expense_group_members.display_name` —
ed è *per questo* che nessuno la legge. La funzione della decisione 1 proietta
`full_name` da `lease_parties`, non da `app.users`, quindi le due decisioni sono
indipendenti: nessuna delle due ha bisogno dell'altra per funzionare.

#### 3. Le funzioni nuove sono `SECURITY DEFINER`, ed è la **terza** occorrenza della trappola

Tutte e quattro leggono tabelle protette per decidere un accesso, e farlo in linea in una
policy o in una query del chiamante significherebbe subire la RLS di quelle tabelle. È la
trappola della `0026`, e qui si ripresenta due volte: `property_residents` deve leggere
`app.leases`, che `leases_select` restringe al singolo contratto; `shares_home_with` deve
leggere `app.lease_parties` di **altri**, che oggi nessuno vede.

Vale la regola generale già scritta nell'esito della fetta co-living: quando una funzione
`SECURITY DEFINER` legge una tabella per autorizzare, deve verificare **anche**
l'organizzazione — bypassare la RLS significa bypassare l'isolamento, non solo la
visibilità per ruolo. Qui `can_access_property` lo fa già per conto suo.

#### 4. Il confine è la **casa**, e nello schema esiste già

`app.can_access_property` ammette lo staff **oppure** l'inquilino dell'immobile, e il
commento della `0014` lo dice per esteso: «l'inquilino vede l'immobile in cui abita
(anche solo per stanza)». La nozione di casa come perimetro non va inventata: va
**usata** dove oggi non lo è.

Ne segue che `shares_home_with` non deve reinventare «abitare qui»: due persone
condividono la casa se hanno entrambe una parte contrattuale su contratti dello stesso
immobile. Scrivere una seconda definizione sarebbe il difetto che ADR-019 e
`app.lease_in_force` esistono per impedire su «in essere».

#### 5. L'elenco è dei **residenti**, e usa `lease_in_force`

`GET /properties/{id}/residents`: chi ha una parte contrattuale su un contratto **in
essere** di quell'immobile. Il filtro passa da `app.lease_in_force`, che è l'unica
definizione di «in essere» del progetto (ADR-019): un inquilino uscito sei mesi fa non è
un residente, e confrontare le date a mano creerebbe la seconda definizione che quella
funzione esiste per impedire.

Restituisce `user_id`, `full_name` e la **stanza**, che è quanto basta ad aggiungere
qualcuno a un gruppo e a riconoscerlo. **Non restituisce l'email**: la funzione proietta,
quindi non mostrarla non costa niente — a differenza di una policy allargata, dove la
riga esce intera.

### Decomposizione

**Il punto di taglio** è il passo 5: senza il collegamento con i gruppi di spesa resta un
endpoint che elenca i residenti, utile di per sé e verificabile. Il passo 1 non si taglia
— è la capacità. Il passo 2 nemmeno: è la metà dell'obiettivo che riguarda A e B.

1. **Migration `0030`, prima parte:** `app.property_residents(uuid)`, `SECURITY DEFINER`
   con `SET search_path`, che restituisce `(user_id, full_name, room_name)` e filtra con
   `can_access_property` e `lease_in_force`. `GRANT EXECUTE`, che non arriva dal
   `GRANT … ON ALL FUNCTIONS` della `0015`.
2. **Migration `0030`, seconda parte:** `shares_home_with`, `is_staff_of_user`,
   `is_my_staff`, e l'`ALTER POLICY` di `users_select`. Verificare **al contrario**:
   prima della migration X vede A e B, dopo no.
3. **Porta e repository:** l'elenco dei residenti, letto dalla funzione. **Non** una
   query con i join, che è ciò che la decisione 1 ha escluso.
4. **Use case `ListPropertyResidents`.** `ensure_member`, e a decidere quali righe
   escono è la funzione. L'immobile si rilegge prima, per dare `404` invece di un elenco
   vuoto.
5. **Bordo HTTP:** la rotta, il DTO, l'OpenAPI.
6. **Il collegamento con i gruppi di spesa:** `docs/05-api.md` spiega come un client
   passa dai residenti all'aggiunta di un membro. Da valutare eseguendo se
   `AddGroupMember` debba **verificare** che la persona sia residente dell'immobile del
   gruppo: sembra ovvio e non lo è, perché `expense_groups.property_id` è opzionale e un
   gruppo senza immobile non avrebbe nulla contro cui verificare.
7. **Test.** Persistenza sullo scenario dei due immobili, in **entrambe** le direzioni —
   X vede Y e Z, X **non** vede A e B, A vede B e non vede gli altri — e le due parti
   della migration verificate al contrario; contratto HTTP sull'elenco e sul giro
   completo fino all'aggiunta al gruppo.
8. **Documentazione.** `docs/05-api.md`, `docs/03-modello-dati.md`,
   **`docs/04-sicurezza.md`** — dove il perimetro dei dati personali si **restringe**, il
   che va scritto perché è un cambiamento di garanzia — l'esito, e **un ADR**: il
   perimetro della casa, perché è un confine del database e non una regola dell'API, e
   perché una funzione che restituisce righe è preferibile a una policy allargata quando
   ciò che serve è una **proiezione**.

### Trappole

**Allargare una policy di lettura non basta se la query fa un join.** È la trappola che
ha riscritto questo piano: `lease_parties_select` allargata dà X, Y, Z, e l'unione con
`app.leases` li rifiltra a X, perché `leases_select` è rimasta per contratto. Il sintomo
è un elenco che sembra vuoto senza errori. **Da riprodurre**, perché è la ragione per cui
la fetta ha la forma che ha.

**Allargare `leases_select` avrebbe esposto il canone altrui.** `leases.rent_amount` sta
sulla riga, e una policy è per riga e non per colonna. Vale la pena saperlo anche se
questa fetta non lo fa: è il motivo per cui non lo fa.

**Una sottoquery dentro una policy subisce la RLS della tabella che legge.** Terza
occorrenza dopo la `0026` e le `can_access_*` originali. **Da riprodurre**: la versione
senza funzione deve fallire.

**`lease_parties.user_id` è annullabile.** È nullo finché l'invitato non accetta: un
residente «invitato e non ancora entrato» ha un nome e **nessun identificatore**, quindi
non è aggiungibile a un gruppo. L'elenco deve dirlo invece di ometterlo, altrimenti chi
guarda non capisce perché una persona che vede non si può aggiungere.

**Un contratto scaduto non toglie la parte contrattuale.** Senza il filtro su
`app.lease_in_force` un inquilino di due anni fa comparirebbe ancora fra i residenti.
**Da riprodurre**: un contratto terminato non produce residenti.

**Il proprietario non è un residente.** `can_access_property` lo ammette come staff, ma
non ha una parte contrattuale: nell'elenco non comparirà, ed è corretto — non abita lì.
Chi si aspetta di vederlo sta cercando un elenco diverso.

**Restringere `users_select` non fa fallire nessun test**, e questo è il pericolo. Il
verde non è la prova che sia sicuro: è la prova che nessuno usa quella visibilità. La
verifica vera è la matrice dello scenario, non la suite.

**`is_my_staff` e `is_staff_of_user` non sono la stessa funzione al contrario.** La prima
risponde «questa persona è staff di un'organizzazione a cui appartengo»; la seconda «sono
staff di un'organizzazione a cui questa persona appartiene». Servono entrambe, e
scambiarle produce una policy che sembra funzionare finché il chiamante non è un
inquilino.

### Come si saprà che è finita

`./ops/ci/test-integration.sh` verde con i test nuovi, e le invarianti verdi.

**La matrice dello scenario, verificata in entrambe le direzioni**, che è la sola prova
che risponde alla domanda per cui la fetta esiste:

| Sono… | Anagrafica | Residenti Casa 1 | Residenti Casa 2 |
|---|---|---|---|
| **X** | Proprietario, X, Y, Z | X, Y, Z | **nessuno** |
| **A** | A, B, Proprietario | **nessuno** | A, B |
| Proprietario | tutti e sei | tutti | tutti |

E le due direzioni contrarie: **prima** della migration X vede A e B nell'anagrafica e
non vede Y e Z fra i residenti; **dopo**, l'opposto. Una suite verde dopo non prova nulla
da sola.

**I contratti restano invisibili.** Nello stesso scenario X vede **un** contratto, il
proprio, prima e dopo: nessun canone altrui diventa leggibile. È l'assert che dice che
non si è pagato il prezzo che la prima stesura del piano aveva accettato.

Un giro a mano che chiude il buco della fetta precedente: il proprietario crea due stanze
e due contratti e invita X e Y; entrambi accettano; X elenca i residenti dell'immobile,
**trova Y con il suo identificatore**, crea un gruppo di spesa e **ce lo aggiunge** —
senza che il proprietario gli passi niente a mano. È il giro che oggi non si può fare.

<a id="esito-coinquilini-visibili-fetta-1"></a>

### Esito — che cosa l'esecuzione ha confermato, cambiato e trovato

Eseguito il **2 settembre 2026**. Il piano sopra resta com'era: serve a capire perché il
codice è così. Qui sotto c'è ciò che l'esecuzione ha aggiunto.

**La forma del piano ha tenuto per intero.** Migration `0030` in due parti, funzione che
proietta, `users_select` ristretta, endpoint, use case, nessuna policy di lettura
allargata. Non c'è stato nulla da rifare, e la ragione è che il piano era già stato
riscritto una volta *dopo* aver misurato — che è il motivo per cui misurare prima ha
pagato.

#### La trappola centrale ha una profondità in più di quella scritta

Il piano descriveva una trappola sola: allargare `lease_parties_select` dà X, Y, Z, e
l'unione con `app.leases` li rifiltra a X. **È vero, ma solo se si è già risolta una
trappola precedente**, e il piano non lo diceva perché nella misurazione originale quel
passo era stato fatto senza accorgersene.

Riprodotte tutte e due su database vero:

| Versione di `lease_parties_select` | `lease_parties` da sola | unita ad `app.leases` |
|---|---|---|
| `EXISTS (SELECT … FROM app.leases …)` in linea nella policy | **solo X** | solo X |
| lo stesso `EXISTS` dentro una funzione `SECURITY DEFINER` | X, Y, Z | **solo X** |

La versione ovvia — l'`EXISTS` scritto direttamente nella policy — **non fa comparire
nemmeno una riga in più**, perché quella sottoquery legge `app.leases` e subisce
`leases_select`. Cioè: si incontra prima la trappola della `0026`, e solo dopo averla
risolta si incontra quella del join.

**Perché conta:** chi riprova questa via parte dalla versione ovvia, vede «solo X», e ha
due spiegazioni disponibili — la sottoquery, oppure il join — che chiedono correzioni
diverse. Correggendo la prima si arriva a X, Y, Z e si crede di avere finito, finché il
join non ricompare. Sapere che sono due, in quest'ordine, è ciò che il piano non
diceva.

#### Il prezzo, misurato e non solo argomentato

Il terzo passo — allargare **anche** `leases_select` — è stato eseguito, e il risultato
è quello che il piano temeva, con i numeri:

```
 full_name   | rent_amount
-------------+-------------
 Inquilino X |      400.00
 Inquilino Y |      450.00     ← il canone del coinquilino
 Inquilino Z |      500.00     ← e dell'altro
```

Un test di integrazione tiene ferma la direzione opposta: nello scenario a due immobili
**ogni inquilino vede un solo contratto**, prima e dopo la migration. È l'assert che dice
che il prezzo non è stato pagato, e vale più della matrice dei residenti — quella dice
che la capacità c'è, questa dice che non è costata niente.

#### Quattro cose che il piano non aveva previsto

**1. Il `404` è più forte dell'elenco vuoto, e il piano prevedeva l'elenco vuoto.** La
matrice del piano dice «residenti Casa 2: nessuno» per X. La **funzione** risponde
davvero zero righe; lo **use case** risponde `404`, perché rilegge l'immobile prima e la
Casa 2 non è visibile a X. Sono due confini in fila e a parlare è il più esterno. La
distinzione è quella giusta e va detta: un elenco vuoto significa «questa casa esiste e
non ci abita nessuno», che è lo stato legittimo di un immobile sfitto — restituirlo per
casa altrui confermerebbe che l'identificatore indovinato esiste. Il test ha una
funzione dedicata, `residents_denied_to`, perché scriverlo come «elenco vuoto» lo faceva
fallire, ed è stato il primo segnale.

**2. `can_access_property` non basta a garantire l'organizzazione, e il piano diceva di
sì.** La decisione 3 affermava: «Qui `can_access_property` lo fa già per conto suo».
Verificato: **verifica un rapporto con l'organizzazione dell'immobile, non che sia
l'organizzazione attiva**. Chi appartiene a due organizzazioni potrebbe, con il contesto
della B, leggere i residenti di un immobile della A. Non è una fuga verso un estraneo —
sono i suoi coinquilini — ma è la stessa forma di buco che la `0026` documenta per esteso
e che `properties_select` chiude con la sua ancora `org_id = app.current_org_id()`.
Entrambe le funzioni nuove la portano, e il criterio della `0026` arriva così alla quarta
occorrenza.

**3. Il garante.** Il piano definiva i residenti come «chi ha una parte contrattuale su
un contratto in essere», e alla lettera comprende il **garante**, che non abita lì.
Escluso: è la stessa ragione per cui il proprietario non compare, e la distinzione esiste
già nel dominio come `LeasePartyRole::shares_rent()`. Ricavarla da un ruolo che lo schema
ha già evita di inventarne una seconda.

**4. `lease_parties.left_at`.** Il piano non lo nominava. La funzione filtra
`left_at IS NULL`, che è il filtro già usato da `LeaseParty::ensure_tenancy_complete`, e
**non** un confronto con `CURRENT_DATE`: `lease_parties_dates_ck` non dice se `left_at`
sia inclusiva o esclusiva, e oggi **nessun caso d'uso la valorizza**. Scegliere una
semantica del confine per una colonna che nessuno scrive sarebbe inventare la seconda
definizione che ADR-019 esiste per impedire, su una domanda che non è ancora stata posta.

#### Scelte prese eseguendo, che il piano lasciava aperte

**`AddGroupMember` non verifica la residenza, e non è una rinuncia.** Sembra il
complemento ovvio dell'elenco dei residenti, e a guardarlo da vicino è una regola che si
legge come un confine senza esserlo:

- `expense_groups.property_id` è **opzionale**, e per giunta `ON DELETE SET NULL`: un
  gruppo creato senza immobile non ha nulla contro cui verificare, e uno che ce l'aveva
  può perderlo da solo. Una regola che si aggira omettendo un campo facoltativo è un
  dosso, non un confine — ed è precisamente l'errore che la `0029` è servita a smettere
  di fare, questa volta commesso di proposito;
- **non sarebbe esprimibile nemmeno nella RLS**, per la stessa ragione: la policy
  `expense_group_members_insert` incontrerebbe lo stesso `NULL`. Se il confine non si può
  mettere dove i confini stanno, non va scritto altrove fingendo che sia lì;
- **romperebbe casi legittimi**. `AddGroupMember` è anche il **rientro**, e chi ha
  lasciato la casa il mese scorso non è più un residente ma ha ancora un conto aperto: è
  esattamente la persona che la migration `0027` esiste per non perdere di vista.
  Rifiutarne il reinserimento lavorerebbe contro la fetta precedente;
- **il gate che conta c'è già**, e riguarda chi aggiunge: `can_access_expense_group`
  ammette un membro attivo del gruppo o lo staff. Non ci si intrufola in un gruppo
  altrui; ci si porta dentro qualcuno di un gruppo di cui si fa parte, e quella è una
  decisione dei coinquilini.

**L'endpoint non restituisce l'email.** La decisione era già stata neutralizzata dalla
decisione 1 — con una funzione che proietta non c'è nulla da cui ometterla — e resta da
dire perché **non aggiungerla**: non serve a nessuno dei due usi dell'elenco (aggiungere
qualcuno a un gruppo vuole `user_id`, riconoscerlo vuole nome e stanza), ed è un dato
personale in una fetta che il perimetro dei dati personali lo **restringe**. Aggiungerla
qui mentre `users_select` si stringe si annullerebbe da sé: X perderebbe l'email di A e
guadagnerebbe quella di Y. L'asimmetria dei costi chiude: ometterla oggi non costa
niente, aggiungerla domani è additivo, toglierla domani è una rottura.

#### Una verifica che non verificava niente

`residents_seen_by(&env, x, home_two).await.is_empty()` — scritto così, sarebbe passato
per il motivo sbagliato il giorno in cui la funzione avesse smesso di filtrare, perché
`Vec::is_empty` è vero anche per un elenco che nessuno ha popolato. È diventato
`residents_denied_to`, che pretende un `AppError::NotFound`. È lo stesso genere di errore
già registrato nella fetta contabile: un'asserzione che non può fallire nella direzione
che interessa.

Lo stesso vale per la restrizione di `users_select`, e qui il piano avvertiva:
**restringere non fa fallire alcun test**. La verifica non è la suite verde, è
`the_anagraphic_perimeter_is_the_home_and_not_the_organization`, che valuta i due
predicati **fianco a fianco** sulla stessa persona nella stessa transazione:
`shares_org_with(A)` è ancora **vero** — la funzione della `0014` esiste e A è nella
stessa organizzazione — e `shares_home_with(A)` è **falso**. Poi legge la policy vera e
pretende che la riga di A non ci sia. Senza quel confronto, il test direbbe solo che
qualcosa non è visibile, senza dire che *prima* lo era.

#### Come si è verificato

- **Prima della migration**, su un database vero con lo scenario a due immobili: X vede
  sei righe di `app.users` — A e B compresi, con le loro email — un contratto, e fra le
  parti contrattuali **solo sé stesso**. È la fotografia del difetto.
- **Le tre versioni della via scartata**, eseguite in sequenza sullo stesso database:
  `EXISTS` in linea (nessuna riga in più), funzione `SECURITY DEFINER` (X, Y, Z che il
  join rifiltra), `leases_select` allargata (funziona, ed esce il canone). Ripristinato
  lo stato di partenza e riverificate le invarianti prima di proseguire.
- **Dopo la migration**, la matrice completa:

| Sono… | Anagrafica | Residenti Casa 1 | Residenti Casa 2 | Contratti |
|---|---|---|---|---|
| **X** | Proprietario, X, Y, Z | X, Y, Z | **`404`** | **1** |
| **A** | Proprietario, A, B | **`404`** | A, B | **1** |
| Proprietario | tutti e sei | X, Y, Z | A, B | 5 |

- **Sette test di persistenza** e **due di contratto HTTP**, fra cui il giro a mano del
  piano eseguito contro il router vero: il proprietario crea due stanze e due contratti
  **per stanza** e invita X e Y; entrambi accettano; X elenca i residenti, ci trova Y con
  il suo identificatore, crea un gruppo e ce lo aggiunge — **senza che il proprietario
  gli passi niente a mano**. Era il giro che non si poteva fare.
- `./ops/ci/test-integration.sh` verde: **352 test**, invarianti dello schema comprese.

#### Cosa resta fuori, e resta fuori davvero

- **Un elenco delle persone dell'organizzazione.** Non è rinviato: è il perimetro che
  questa fetta rifiuta, in entrambe le direzioni.
- **`app.shares_org_with` non è stata rimossa.** Non è più usata da alcuna policy.
  Toglierla è un cambiamento a sé, da fare quando si è certi che nessuna policy futura la
  voglia, e non c'era ragione di mescolarlo a questa migration.
- **Nessuna notifica.** La porta `Notifier` resta senza adattatore.
- **`shares_home_with` non filtra «in essere», e resta così.** Un ex coinquilino continua
  a leggere il nome di chi gli deve dei soldi, ed è voluto: `users_select` è un pavimento,
  non un elenco. L'elenco è `app.property_residents`, che «in essere» lo filtra.
- **Chi affitta due stanze compare due volte.** Una riga per parte contrattuale, con la
  stanza a distinguerle. Ridurlo a una riga sceglierebbe arbitrariamente quale stanza
  mostrare; se un client avrà bisogno del contrario, lo farà con l'informazione che ha.


---

<a id="piano-client-fetta-1"></a>

## Piano — Il client, fetta 1

Pianificato il **3 settembre 2026**. Il piano resta sotto come è stato scritto: serve a
eseguirlo, e servirà a capire perché il codice è com'è. L'esito va in fondo a esecuzione
conclusa, come per tutte le fette precedenti.

**Questo piano non è la voce «dashboard» della lista dei moduli.** Quella è un endpoint
di lettura su `v_owner_dashboard` e resta il passo 1 qui sotto. Questo piano apre un
fronte nuovo: **il client**, che `docs/09` elencava fra i trasversali mancanti come
«fuori perimetro backend» e che da qui in poi nel perimetro ci entra.

### Obiettivo

Un'interfaccia che un proprietario vero possa usare per amministrare i propri immobili:
entrare, vedere a colpo d'occhio cosa richiede attenzione, e da lì agire — registrare un
immobile, redigere un contratto, invitare un inquilino, registrare una bolletta, farla
leggere a una macchina, riaddebitarla.

**Non un prodotto da dimostrazione.** Il criterio è dichiarato perché orienta cento
decisioni piccole: accessibilità da tastiera, stati di caricamento e di errore
progettati invece che improvvisati, form che non perdono ciò che è stato digitato,
tabelle che reggono mille righe. Sono le cose che distinguono un'interfaccia che si
mostra da una che si usa, e che aggiunte dopo costano il triplo.

**Dentro:** la rotta della dashboard; la sessione in cookie con il refresh a singolo
volo; il guscio dell'applicazione — autenticazione, navigazione, selezione
dell'organizzazione, modello degli errori; e il percorso completo del **proprietario**:
dashboard, immobili e stanze, contratti e parti, bollette con revisione dell'OCR,
addebiti, inviti. Più la CI e l'immagine che li reggono.

**Fuori, e non per dimenticanza:**

- **il percorso dell'inquilino.** Non è una scelta di priorità ma una dipendenza: un
  inquilino non esiste finché un proprietario non crea l'organizzazione, l'immobile, il
  contratto e lo invita. Senza il percorso del proprietario, quello dell'inquilino non è
  nemmeno provabile con dati veri. È la fetta 2, e comprende segnalazioni, spese
  co-living e pagamenti dichiarati;
- **i pagamenti e le spese co-living dal lato proprietario.** Esistono nell'API e sono
  rimandati per tenere la fetta finita invece che larga: sono il primo candidato della
  fetta 2 insieme all'inquilino;
- **la traduzione in inglese.** L'interfaccia della fetta 1 è in italiano soltanto, ma
  **non è rimandabile l'impianto**: le stringhe stanno in un catalogo dal primo giorno,
  perché aggiungere una lingua a un'interfaccia con i testi nel JSX è una riscrittura.
  Vedi la decisione 6;
- **il mobile nativo.** `docs/09` ipotizzava Flutter o React Native. L'ipotesi è
  **rivista** — vedi la decisione 2 — e una web app responsive copre entrambi i ruoli
  con un codice solo. Il giorno in cui servisse una app vera, servirà per le notifiche
  push e la fotocamera, cioè per l'inquilino, e sarà quella la conversazione;
- **i manifest Kubernetes.** Restano il debito che erano. Questa fetta definisce il
  *contratto* del reverse proxy — `/` ai file statici, `/api/v1` all'API — e lo realizza
  in sviluppo e in `docker-compose.yml`; scriverlo per K8s è lavoro di esercizio, con il
  resto di quel debito;
- **le notifiche.** `Notifier` è una porta senza adattatore, quindi il token d'invito
  **torna nella risposta HTTP** e il proprietario lo consegna come preferisce. Il client
  non lo nasconde: lo presenta come qualcosa da copiare e mandare. È brutto ed è la
  verità, e mascherarlo con una schermata che dice «invito inviato» sarebbe una bugia
  nell'interfaccia.

### Che cosa è stato deciso prima di pianificare

Sei scelte, prese esplicitamente. Stanno qui perché il resto del piano le dà per
assodate.

| | Scelta | Alternativa principale scartata |
|---|---|---|
| Perimetro | guscio + proprietario end-to-end | le due home dei due ruoli, che sarebbe un guscio con poco dietro |
| Piattaforma | web responsive, React + TypeScript + Vite | Flutter, che `docs/09` ipotizzava |
| Collocazione | monorepo, `apps/web/` accanto a `crates/` | repository separato |
| Sessione | refresh in cookie `httpOnly`, access in memoria | entrambi in `localStorage` |
| Origine | **una sola**, con un reverse proxy davanti | `app.` e `api.` separati |
| Base UI | shadcn/ui — Radix + Tailwind | Mantine, MUI |

### Decisioni prese, con il perché

#### 1. Una sola origine, e ne segue che CORS non serve

È la decisione con più conseguenze, e quasi tutte sono cose che **non** si devono fare.

Un solo dominio: `/` serve i file statici, `/api/v1` inoltra all'API. In sviluppo lo fa
il proxy di Vite, in produzione un Ingress o un Caddy. Il browser vede una sola origine,
e ne segue:

- **CORS non va montato.** Era il primo lavoro che sembrava obbligatorio e sparisce: non
  c'è nessuna richiesta cross-origin da autorizzare. Il debito «CORS non montato» resta
  aperto per il giorno in cui un client di terze parti esisterà, e quel giorno sarà una
  decisione sua;
- **il cookie è di prima parte.** `SameSite=Strict` invece di `SameSite=None; Secure`,
  che è la differenza fra un cookie che il browser non manda mai altrove e uno che manda
  ovunque glielo si chieda;
- **nessun elenco di origini ammesse** da tenere allineato fra ambienti, che è una
  configurazione che si sbaglia in silenzio e si scopre in produzione.

**Alternativa scartata: origini separate.** Più comoda da distribuire in modo
indipendente, e costa tre pezzi di backend — CORS con credenziali, cookie
`SameSite=None`, difesa CSRF — ciascuno sbagliabile senza che nulla protesti. Si
comprerebbe indipendenza di deploy pagandola in superficie di sicurezza, su un prodotto
che tratta contratti di locazione e importi.

**Alternativa scartata: i file statici serviti dall'API Rust.** Un servizio solo e
nessun proxy da configurare, ed è tentante. Costa però che la build del frontend deve
finire nell'immagine dell'API, e che un processo HTTP fa due mestieri con profili di
carico e di cache opposti — file immutabili con hash nel nome da un lato, risposte
personali non memorizzabili dall'altro. Il proxy va scritto comunque per K8s.

#### 2. Web responsive e non mobile nativo, che è una revisione di `docs/09`

L'ipotesi scritta era «client mobile (Flutter o React Native)», ed è stata scritta prima
che il perimetro dell'API fosse noto. Rivista guardando **che cosa fa il proprietario**:
registra una bolletta con dodici campi, legge tabelle di addebiti, confronta una
proposta dell'OCR con i numeri che ha digitato, riaddebita a un contratto. È lavoro da
schermo grande e da tastiera.

L'inquilino invece è mobile per natura — fotografa un guasto, guarda quanto deve — e una
web app responsive lo copre bene, senza le due cose che richiederebbero davvero una app
nativa: **notifiche push** e accesso alla fotocamera più fluido di `<input capture>`.
Nessuna delle due è nella fetta 1, e la prima non è nemmeno possibile finché `Notifier`
non ha un adattatore.

**La decisione va rifatta quando l'inquilino sarà il fuoco**, ed è scritto qui perché
chi farà la fetta 2 lo trovi.

#### 3. La sessione: cookie per il refresh, memoria per l'access, e **nessun token CSRF**

La parte controintuitiva è l'ultima, quindi va spiegata per intero.

| | Dove vive | Chi lo manda |
|---|---|---|
| access token (15 min) | **solo in memoria** JavaScript | il client, come `Authorization: Bearer` |
| refresh token (30 giorni) | cookie `httpOnly`, `Secure`, `SameSite=Strict`, `Path=/api/v1/auth` | il browser, da solo, e solo verso le rotte di autenticazione |

L'access token in memoria muore al ricaricamento della pagina, e non è un problema: si
riottiene dal cookie. Non è leggibile da un XSS che arrivi *dopo* il caricamento, perché
non sta in nessuno storage persistente.

**Perché non serve una difesa CSRF**, che è la conclusione che va argomentata invece di
essere data per buona. Il CSRF è possibile quando il browser autentica una richiesta
**da solo**, cioè con un cookie. Qui:

- ogni rotta di business è autenticata dall'header `Authorization`, che un sito
  attaccante **non può impostare** su una richiesta cross-origin senza il permesso di
  CORS — che non concediamo, perché CORS non è montato affatto;
- le sole rotte autenticate dal cookie sono `/api/v1/auth/refresh` e `/logout`, e su
  quelle il cookie è `SameSite=Strict`: il browser **non lo manda** su una richiesta che
  parte da un altro sito. Non c'è niente da falsificare.

**Ne segue una condizione da non perdere di vista, ed è la ragione per cui questa nota è
lunga**: l'argomento regge finché *nessuna rotta di business è autenticata da un
cookie*. Il giorno in cui qualcuno aggiungesse una sessione a cookie per comodità, tutto
questo cadrebbe **in silenzio** — nessun test fallirebbe. Va scritto nell'ADR e va
messo un test che verifichi che l'unica rotta che accetta il cookie è quella di
refresh.

**Il `Path=/api/v1/auth` non è cosmetico**: limita l'invio del cookie alle sole rotte che
lo usano, quindi una richiesta a `/api/v1/properties` non se lo porta dietro nemmeno per
sbaglio. È difesa in profondità: se un giorno l'argomento sopra si incrinasse, il cookie
comunque non arriva dove non serve.

**Alternativa scartata: entrambi i token in `localStorage`.** Nessuna modifica al
backend, e in cambio un XSS legge un refresh token che vale trenta giorni. Su questo
prodotto è un rischio che si può accettare solo dicendolo, e non c'è ragione di
accettarlo.

#### 4. Il refresh token **sparisce dal corpo della risposta**, ed è una rottura

Se l'API mettesse il refresh token *sia* nel cookie *sia* nel JSON, il cookie non
proteggerebbe niente: un XSS lo leggerebbe dalla risposta del login. Metà misura, costo
pieno.

Quindi `SessionResponse` perde `refresh_token`, e l'API lo emette **solo** come cookie.
`RefreshRequest` e la richiesta di logout smettono di accettarlo nel corpo e lo leggono
dal cookie.

**È una rottura del contratto, e va annunciata**: sulla linea 0.x è ammessa, entra nel
commit che la causa e da lì nel changelog. Chi usa `curl` passa a un cookie jar
(`-c`/`-b`), che è una riga in più negli esempi di `docs/05-api.md` e va aggiornata lì.

**Alternativa scartata: un parametro che scelga fra cookie e corpo.** Un flag
sull'autenticazione è una superficie in più da sbagliare, e sposta su chi chiama una
decisione di sicurezza che deve essere del server.

#### 5. Il refresh a **singolo volo**, che è la trappola più cara del client

L'API ruota il refresh token e tratta il riuso come prova che qualcuno ne ha una copia:
`Invariant::RefreshTokenReused` dice «possible reuse, revoke the chain». È corretto e
severo, e in un browser si trasforma in un difetto se il client è ingenuo.

Lo scenario, che **accadrà**: la pagina fa tre richieste in parallelo, l'access token è
scaduto, tutte e tre ricevono `401`, tutte e tre chiamano `/auth/refresh` con lo stesso
cookie. La prima ruota il token; la seconda e la terza presentano un token **già
ruotato**; l'API revoca l'intera catena; l'utente viene buttato fuori. Con due schede
aperte è ancora più facile.

Il client deve quindi avere **una sola chiamata di refresh in volo**: una promessa
condivisa che tutte le richieste in attesa aspettano, e da cui ripartono con il token
nuovo. Non è un'ottimizzazione, è una condizione di correttezza.

Fra schede diverse il problema resta e non lo risolve una promessa in memoria: serve un
coordinamento (`BroadcastChannel` o un lock su `localStorage`). **Nella fetta 1 si
accetta**, si documenta, e si mette nei debiti — il sintomo è raro, e il rimedio è
ricaricare.

#### 6. Il monorepo, e che cosa significa per `CLAUDE.md`

`apps/web/` accanto a `crates/`. Il contratto dell'API e il client si muovono insieme: una
rotta che cambia e un client che non la segue si vedono **nella stessa PR**, che è
esattamente il difetto che un repository separato lascia scoprire tardi.

Ne segue una cosa che va detta: **`CLAUDE.md` è oggi interamente su Rust**. «Niente
`unwrap`», «errori con `thiserror`», «query come letterali statici» non hanno una
traduzione in TypeScript, e lasciare il frontend senza regole significa lasciarlo senza
criterio. Serve una sezione sua, corta, con le poche regole che valgono:

1. **la lingua dell'interfaccia è l'italiano**, e l'inglese arriverà. Gli utenti sono
   proprietari e inquilini italiani, e non contraddice la regola sull'inglese di
   `CLAUDE.md`, che riguarda ciò che un estraneo incontra dal di fuori del repository —
   commit, PR, README — non ciò che un utente legge nel prodotto.

   **Ne segue un vincolo che vale dal primo giorno, ed è la ragione per cui questa voce
   è più lunga delle altre: nessuna stringa visibile va scritta dentro un componente.**
   La traduzione in inglese è già decisa, solo rimandata, e una fetta costruita con i
   testi sparsi nel JSX la rende una riscrittura invece di un'aggiunta. Le stringhe
   stanno in un catalogo per lingua, i componenti ne usano le chiavi, e la lingua è una
   sola finché non ce ne sono due. Non serve una libreria di internazionalizzazione
   nella fetta 1 — serve **il livello di indirezione**, che è la parte impossibile da
   aggiungere dopo.

   Ne segue anche che le date, gli importi e i numeri non si formattano a mano:
   `Intl.DateTimeFormat` e `Intl.NumberFormat` con la lingua come parametro, altrimenti
   `31/08/2026` e `184,20 €` restano italiani anche in inglese;
2. **niente `any`**, e `strict` acceso in `tsconfig`. È l'equivalente di «zero warning»;
3. **gli importi restano stringhe**: vedi la trappola apposita;
4. **ogni stato di una schermata è progettato**: caricamento, vuoto, errore, e non solo
   il caso felice;
5. **nessuna chiamata `fetch` fuori dal livello dei dati.** È l'equivalente di «nessun
   SQL negli handler»: un componente che chiama l'API da sé aggira gestione degli
   errori, ritentativi e sessione.

#### 7. shadcn/ui, cioè componenti copiati e non una dipendenza

I componenti si copiano nel progetto. Il vantaggio non è filosofico: un gestionale
richiede tabelle con ordinamento e filtri, form con validazione per campo, date pickers
che capiscano il formato italiano, e con una libreria chiusa ognuno di questi si
personalizza combattendo. Radix sotto garantisce accessibilità e gestione della
tastiera, che è la parte che nessuno riscrive bene a mano.

**Alternativa scartata: Tailwind nudo.** Controllo totale, e accessibilità, focus, ARIA
su modali, menù e combobox da scrivere a mano. È il modo più rapido di costruire
un'interfaccia che *sembra* professionale e non lo è.

**Alternativa scartata: MUI.** Il `DataGrid` risolverebbe da solo metà delle schermate
del proprietario, e il prodotto somiglierebbe a un'app Google salvo un lavoro di tema
considerevole; le parti da gestionale più utili sono per giunta a pagamento.

### Decomposizione

**Undici passi.** I passi 1–5 sono la fondazione e non si tagliano: senza, non c'è
niente da mostrare. Il **punto di taglio** è dopo il passo 8: con dashboard, immobili e
contratti il proprietario ha un prodotto coerente, e bollette e inviti sono una fetta
1-bis. Va deciso guardando, non adesso.

1. **Backend: la rotta della dashboard.** `GET /api/v1/dashboard`, ruolo staff, che
   legge `app.v_owner_dashboard` — otto numeri: `properties_count`, `active_leases`,
   `monthly_rent_expected`, `total_outstanding`, `overdue_charges`, `open_tickets`,
   `leases_expiring_90d`, `bills_pending_review`. Use case di sola lettura, DTO, rotta,
   registrazione in `openapi.rs`, test di contratto. Nessuna migration: la vista esiste
   dalla `0015` ed è `security_invoker`, quindi applica già la RLS del chiamante.
2. **Backend: la sessione in cookie.** `Set-Cookie` su login e refresh con
   `httpOnly; Secure; SameSite=Strict; Path=/api/v1/auth`; lettura del cookie in refresh
   e logout; rimozione di `refresh_token` da `SessionResponse` e dalle richieste. Il
   `Secure` va reso condizionale per lo sviluppo su `http://localhost` — **è l'unica
   configurazione nuova**, e va nella tabella di `docs/07-operazioni.md`, che è l'unico
   documento capace di far fallire `cargo test`.
3. **Backend: il test che tiene in piedi l'argomento CSRF.** Verifica che il cookie sia
   accettato **solo** dalle rotte di autenticazione: una richiesta a una rotta di
   business con il solo cookie e senza `Authorization` deve rispondere `401`. Senza
   questo test la decisione 3 è una nota che nessuno rilegge.
4. **Frontend: la fondazione.** `apps/web/` con Vite, React, TypeScript in `strict`,
   Tailwind, shadcn/ui. Router (TanStack Router o React Router), livello dati (TanStack
   Query), form (React Hook Form + Zod). Il proxy di Vite su `/api/v1`. Nessuna
   schermata: solo che `npm run build` e `npm run typecheck` passino.
5. **Frontend: il client dell'API e la sessione.** Un modulo solo che fa le richieste,
   allega il `Bearer`, intercetta il `401`, esegue il **refresh a singolo volo** e
   riprova. Traduzione di `ProblemDetails` in un tipo di errore che porta il `code`
   stabile. È il pezzo su cui poggia tutto il resto e va scritto per primo e con i test.
6. **Frontend: autenticazione e guscio.** Registrazione, login, uscita; selezione
   dell'organizzazione con `X-Org-Id`; layout con navigazione; i componenti di stato —
   caricamento, vuoto, errore — come pezzi di prima classe e non come `if` sparsi.
7. **Frontend: la dashboard.** Gli otto numeri, ognuno **collegato al posto dove si
   agisce**: `overdue_charges` porta allo scadenzario filtrato, `bills_pending_review`
   alle bollette da rivedere. Un numero che non porta da nessuna parte è una decorazione.
8. **Frontend: immobili, stanze e contratti.** Elenco con paginazione a cursore,
   dettaglio, creazione, `PATCH` parziale, archiviazione. Contratti: bozza, parti
   contrattuali, attivazione, disdetta, e gli stati **derivati** `expiring`/`expired`,
   che non si possono filtrare (`lease.derived_status_filter`).
9. **Frontend: bollette, OCR e addebiti.** Registrazione di una bolletta; caricamento
   della scansione; **la schermata di revisione dell'OCR**, che è dove la fetta
   precedente diventa visibile — la proposta della macchina accanto ai campi digitati,
   con la confidenza per campo, e un gesto per accettare un valore; validazione;
   riaddebito a un contratto. Addebiti con `effective_status`.
10. **Frontend: inviti.** Invito di un inquilino e consegna del token, presentato per
    quello che è finché `Notifier` non esiste.
11. **CI, immagine e documentazione.** Un filtro di percorso per `apps/web/`, un job del
    frontend, l'estensione del `Dockerfile` e del `docker-compose.yml`; `CLAUDE.md` con
    la sezione del frontend; **due ADR** — la sessione in cookie senza CSRF, e la scelta
    di piattaforma che rivede `docs/09`; e l'esito.

### Trappole

Tutte lette nel codice e nei documenti prima di scriverle qui; quelle marcate **da
riprodurre** vanno verificate in esecuzione.

**Il refresh concorrente revoca la sessione.** È la decisione 5 vista dal lato del
difetto: senza il singolo volo, tre richieste parallele che scadono insieme fanno uscire
l'utente. **Da riprodurre**: tre chiamate in parallelo con un access token scaduto
devono produrre **un solo** `POST /auth/refresh` e nessuna revoca.

**L'argomento CSRF cade in silenzio se una rotta di business accetta il cookie.** Non
c'è nulla che lo impedisca strutturalmente, ed è la ragione del passo 3.

**Gli importi sono stringhe decimali, e `parseFloat` li rovina.** L'API li manda come
`"184.20"` apposta: un `float` binario non rappresenta `0.10`, e in contabilità la
differenza si accumula. Sommare o confrontare importi nel client va fatto con una
libreria decimale o in centesimi interi, e **mai** con l'aritmetica nativa. Vale anche
per `confidence`, che è a quattro decimali.

**`?status=overdue` sugli addebiti risponde `422`**, con `charge.derived_status_filter`;
lo stesso vale per `expiring`/`expired` sui contratti. Un client che costruisca i filtri
dall'elenco dei valori dell'enum ci finisce dentro. Gli stati derivati si mostrano e non
si filtrano — si filtra con `?outstanding=true`.

**Il limite del corpo è doppio**: 1 MiB in generale, 25 MiB sulle rotte di caricamento
dei documenti. Un client che lasci scegliere una fotografia da 30 MB ottiene un errore
che non spiega niente. La dimensione va controllata **prima** di inviare.

**L'inquilino registrato da sé deve passare `X-Org-Id` per sempre.** È un debito già
registrato: `pick_default_membership` preferisce il ruolo più alto e la registrazione
crea sempre un'organizzazione personale di cui si è `owner`. Il sintomo è `200` con zero
elementi — **una schermata vuota senza alcun errore**, che è il modo peggiore di
sbagliare. Il client deve trattare l'organizzazione attiva come stato esplicito dal primo
giorno, non aggiungerla quando qualcuno se ne accorge.

**Il filtro dei percorsi della CI considera «codice» tutto ciò che non è `docs/*.md`.**
Una modifica al solo frontend farebbe quindi girare l'intera CI Rust: sette minuti di
test su PostgreSQL per un cambio di CSS. Va aggiunto un secondo output al job `changes`,
e il job di riepilogo deve elencare il job nuovo fra i propri `needs`. **Il job di
riepilogo si chiama `CI` ed è il check richiesto su `main` e `dev`**: rinominarlo blocca
ogni merge finché non si aggiorna anche la protezione via `gh api`.

**Il `Secure` sul cookie impedisce di entrare in sviluppo.** Su `http://localhost` il
browser non manda un cookie `Secure`, quindi il refresh fallisce e la sessione dura
quindici minuti. Va reso condizionale, ed è la sola configurazione nuova che questa fetta
introduce.

**La proposta dell'OCR non va scritta sopra i campi della bolletta senza chiedere.**
Lo schema tiene separato ciò che ha letto la macchina da ciò che ha confermato una
persona **perché l'accuratezza si possa misurare**. Un client che «applichi tutto» con
un bottone e basta distrugge quel dato prima che lo si raccolga: l'accettazione va fatta
campo per campo, o almeno registrata come un gesto della persona.

**Il token d'invito è una credenziale e non va nei log né nelle tracce.** Torna nella
risposta HTTP perché `Notifier` non esiste, e da lì va mostrato e dimenticato.

### Come si saprà che è finita

`./ops/ci/test-integration.sh` verde con i test nuovi del backend, e il job del frontend
verde: `typecheck`, lint, test, build.

**Le tre verifiche che contano, e nessuna è una suite verde:**

1. **il refresh a singolo volo, riprodotto al contrario**: senza la promessa condivisa,
   tre richieste parallele con l'access token scaduto devono far uscire l'utente. Con la
   promessa, una sola chiamata di refresh e nessuna revoca;
2. **il cookie rifiutato dalle rotte di business**: una richiesta con il solo cookie e
   senza `Authorization` deve rispondere `401`;
3. **un giro a mano dal browser, con il binario vero**: registrazione, login, creazione
   di un immobile, di un contratto e di una parte, attivazione, invito di un inquilino,
   registrazione di una bolletta, caricamento della scansione, comparsa della proposta
   dell'OCR, correzione, validazione, riaddebito — e il numero della dashboard che
   cambia di conseguenza.

**Due prove di qualità che non sono funzionali** e che il criterio «prodotto reale»
rende obbligatorie:

- **navigazione da sola tastiera** su un percorso completo, e nessuna trappola di focus
  nelle modali;
- **la schermata stretta**: 375 px di larghezza su dashboard, elenchi e almeno un form,
  perché l'inquilino della fetta 2 arriverà da lì e le scelte di impianto si prendono
  adesso.

<a id="esito-client-fetta-1"></a>

### Esito — che cosa l'esecuzione ha confermato, cambiato e trovato

Eseguito il **3 settembre 2026**. Il piano è rimasto sopra com'era scritto; questo è
il resoconto di ciò che è successo mettendolo in pratica.

**Tutti gli undici passi sono stati fatti.** Il punto di taglio dopo il passo 8 non
è stato usato: bollette, OCR e inviti sono dentro, e il criterio per non tagliare è
stato che senza la revisione dell'OCR la fetta precedente sarebbe rimasta invisibile
— cioè che il taglio avrebbe risparmiato lavoro e cancellato la ragione per cui
quella fetta era stata fatta.

#### Le tre verifiche che contano, e come sono andate

**1. Il refresh a singolo volo, riprodotto al contrario.** È in
`apps/web/src/api/session.test.ts`, e la parte che conta è il doppio dell'API:
`RotatingApi` ruota il token a ogni uso e revoca la catena al riuso, come fa
`RefreshSession::ensure_usable`. Con `singleFlight: false` tre richieste parallele
con l'access token scaduto producono **tre** chiamate a `/auth/refresh` e almeno una
rifiutata; con la promessa condivisa ne producono **una** e la sessione resta viva.

Il test del difetto non è decorazione: senza di lui il test del rimedio passerebbe
anche con il singolo volo tolto, perché un doppio indulgente restituirebbe `200` a
tutte e tre. È il test che dimostra che l'altro test verifica qualcosa.

**2. Il cookie rifiutato dalle rotte di business.** È
`the_session_cookie_is_refused_by_business_routes` in `crates/api/tests/http_contract.rs`:
presenta un cookie **valido** a cinque rotte — la dashboard, tre elenchi e
un'archiviazione — senza `Authorization`, e pretende `401 auth.token_missing` da
tutte. Verificato anche il codice e non solo lo stato, perché un `401` per un'altra
ragione non proverebbe la stessa cosa.

**3. Il giro a mano dal browser**, con i binari veri: API, worker, PostgreSQL e
MinIO. Registrazione → accesso → immobile → contratto → parte contrattuale →
attivazione → invito con il token consegnato → bolletta → caricamento della
scansione → comparsa della proposta → accettazione di due campi → applicazione →
validazione → riaddebito. **Il numero della dashboard è cambiato di conseguenza**:
insoluto da `0,00 €` a `191,40 €`, cioè esattamente la quota riaddebitata, e
«bollette da rivedere» da `1` a `0` quando la bolletta è passata ad `allocated`.

Un dettaglio che vale la pena registrare perché è il contratto che si vede
funzionare: il contratto attivato con scadenza fra due mesi ha mostrato **«In
scadenza (Attivo)»** — `effective_status` divergente da `status`, calcolato dal
database e non dal client.

#### Le due prove di qualità

**La schermata a 375 px.** Dashboard, elenco degli addebiti e il form della bolletta
da dodici campi. La tabella degli addebiti scorre **dentro il proprio contenitore** e
la pagina no, che è la differenza fra un elenco consultabile e uno inutilizzabile;
il form si impila senza perdere le spiegazioni sotto i campi; il menù di navigazione
diventa a scomparsa e `aria-expanded` lo segue.

**La navigazione da sola tastiera**, ed è qui che il metodo di verifica è cambiato.
Il browser di automazione **non riesce a dispatchare `Invio`**: il suo evento arriva
alla pagina con `key` vuoto, quindi il browser non lo riconosce come `Enter` e non
esegue l'attivazione predefinita. `Tab` funziona, l'anello di focus si vede, ma
premere invio su un bottone non fa niente — e la cosa non riguarda l'applicazione.

Verificarlo a mano era quindi **impossibile con lo strumento a disposizione**, e
dirlo è più utile che dichiararlo fatto. La verifica è diventata un test —
`apps/web/src/features/auth/AuthPage.test.tsx` — con `@testing-library/user-event`,
che produce eventi di tastiera veri: tabulazione fra i campi in ordine, `Invio` in un
campo di testo che invia il form, barra spaziatrice che attiva il bottone. È una
verifica **migliore** di quella prevista, perché si ripete a ogni esecuzione della
suite invece di una volta sola; ed è anche il caso in cui un limite dello strumento
ha prodotto una copertura che non era in piano.

#### Che cosa il piano aveva previsto bene

**Le trappole erano tutte reali, e tre si sono presentate esattamente come scritte.**

Gli **stati derivati che non si filtrano** — `overdue` sugli addebiti, `expiring` sui
contratti — hanno costretto a scrivere a mano gli elenchi delle opzioni dei filtri
invece di derivarli dall'enum. La tentazione di derivarli c'era, ed è precisamente il
modo in cui si arriva a un `422` in produzione.

**Gli importi come stringhe** hanno retto senza eccezioni: `parseFloat` non compare
in nessun file, e `apps/web/src/lib/decimal.ts` fa la somma su `bigint`. Il test che
lo giustifica è quello che mostra `0.1 + 0.2 !== 0.3` accanto a `sumAmounts(['0.10',
'0.20']) === '0.30'`.

**L'organizzazione attiva come stato esplicito** ha evitato il sintomo previsto: si
manda `X-Org-Id` sempre, anche quando coincide con quella del token, e il selettore
la mostra anche quando ce n'è una sola — perché è ciò che rende diagnosticabile in
due secondi un elenco vuoto che altrimenti non direbbe niente.

**Il filtro dei percorsi della CI** è stato esteso con la seconda uscita, e il job di
riepilogo si chiama ancora `CI`: la protezione dei branch non va toccata.

#### Che cosa il piano non aveva previsto

**I nomi dei campi dell'OCR erano stati inventati.** Il piano nominava
`consumption_value`, `consumption_unit` ed `energy_amount`; `BillField::as_str` emette
`consumption` — **una coppia valore-unità in un campo solo** — e non ha mai avuto un
`energy_amount`. La prima versione della schermata mostrava quindi `CONSUMPTION` non
tradotto e un «valore attuale» vuoto, e il difetto si è visto **soltanto** eseguendo
il giro a mano con il worker acceso: nessun tipo lo intercettava, perché il nome di
un campo è una stringa da entrambe le parti.

Ne segue una cosa da sapere per la fetta 2: **i nomi dei campi dell'OCR sono un
contratto non tipizzato fra il dominio Rust e il catalogo del client**, e l'unico
modo di verificarlo è farlo girare. Un campo che il catalogo non conosce compare
grezzo, che è deliberato — rumoroso invece che assente, come per gli enum letti dal
database.

**Non esiste un endpoint che elenchi le organizzazioni di un utente.** Il selettore
è stato costruito su quelle che *questo browser* ha conosciuto, tenute in
`localStorage`. Per il proprietario è indistinguibile da un elenco vero; per
l'inquilino della fetta 2 non lo sarà. È il gemello esatto del debito già registrato
sulle **persone** di un'organizzazione, ed è emerso allo stesso modo: costruendo il
client. La sezione «cosa un client incontrerà per prime» ne prevedeva tre; erano
quattro.

**`Intl.NumberFormat` accetta una stringa**, e questo ha risolto un problema che il
piano lasciava aperto: mostrare un importo senza passare da un `Number`. Un importo
da dodici cifre intere più due decimali sta al limite della precisione di un double,
e convertirlo per formattarlo avrebbe reintrodotto proprio l'errore che le stringhe
decimali esistono per evitare. C'è comunque un ramo di riserva che mostra il valore
grezzo: un numero senza simbolo di valuta è brutto, uno sbagliato è peggio.

**Le date vanno spezzate a mano.** `new Date('2026-08-31')` è mezzanotte **UTC**, e a
ovest di Greenwich la data mostrata diventa il 30. È un difetto che nessuno
attribuisce al fuso orario, ed è coperto da un test che gira con `TZ` spostato.

#### Le due domande sulla dashboard restano aperte, e adesso costano di più

Il piano le aveva lasciate fuori di proposito: se `written_off` vada distinto da
`void` come **perdita**, e se il credito da sovrapagamento vada mostrato.

Restano aperte, e **la finestra in cui cambiarle era additivo si è chiusa**: adesso
un client legge quegli otto numeri. Aggiungere un nono campo è ancora additivo;
cambiare che cosa significa `total_outstanding` non lo è più. Le due domande sono
scritte nel tipo di dominio (`OwnerDashboard`) e nel DTO, così chi le deciderà le
trova dove guarderà.

#### Quello che resta fuori, e che va saputo

Il **percorso dell'inquilino** è la fetta 2, e con esso segnalazioni, spese co-living
e pagamenti dichiarati — che nell'API esistono e nel client no. La ragione è la
dipendenza dichiarata nel piano: un inquilino non esiste finché un proprietario non
lo invita, quindi il percorso dell'inquilino non era nemmeno provabile con dati veri
prima di questa fetta. Adesso lo è.

Il **numero delle segnalazioni aperte** è in dashboard e porta a una schermata che
non esiste ancora: mostra il vero e rimanda ai contratti. Fingere che il numero non
esistesse sarebbe stato peggio che mostrarlo senza destinazione.

Il **coordinamento del refresh fra schede**, il **bundle in un pezzo solo** e il
**download degli allegati senza `Authorization`** sono i tre debiti nuovi, tutti
registrati sopra con il perché e il quando.

---

<a id="piano-completare-il-prodotto"></a>

## Piano — Completare il prodotto (fetta A)

Pianificato il **4 settembre 2026**, eseguito lo stesso giorno fino al punto di
taglio dichiarato.

**Il piano è riportato qui per intero, comprese le fette B e C che non sono state
eseguite.** Non è un errore di archiviazione: il piano *è* il documento che spiega
perché la fetta A è stata fatta per prima e che cosa è stato deliberatamente lasciato
fuori, e le decisioni 5, 6 e 8 sono state prese *qui* anche se il codice che le
realizza non c'è ancora. Riscriverlo per togliere ciò che non è stato eseguito
distruggerebbe esattamente la parte per cui l'archivio esiste.

**Ciò che resta da fare vive in [09 — Stato e roadmap](09-stato-e-roadmap.md)**, con
la sua decomposizione: la fetta A-bis (le notifiche, passi 9 e 10), la fetta B
(l'inquilino) e la fetta C. Le due liste si sovrappongono, ed è previsto: questa non
si riscrive mai, quella si riscrive a ogni fetta.

L'esito è in fondo: [Esito](#esito-completare-il-prodotto-fetta-a).

### Obiettivo

Chiudere la distanza fra l'API e il prodotto. Misurata, non stimata: delle **87 rotte
documentate** il client ne chiama 50, e **37 non le chiama nessuno**. Più quattro
debiti trasversali del client e l'ultima porta senza adattatore.

Non è una fetta sola, e dichiararlo è metà del piano: 37 rotte su tre moduli in un
colpo è precisamente il tipo di lavoro che si abbandona a metà, e che poi si rilegge
come «quasi finito» per mesi. Sono **due fette con un punto di taglio ciascuna**, più
una terza che si nomina e non si pianifica.

**Fetta A — il giro del denaro si chiude.** Pagamenti dal lato del proprietario, lo
storno esteso agli addebiti parzialmente pagati, il nono numero della dashboard, il
credito da sovrapagamento visibile, e l'adattatore delle notifiche.

**Fetta B — l'inquilino.** L'endpoint delle organizzazioni, segnalazioni, spese
co-living, dichiarazione di pagamento, allegati sugli owner rimasti, e i due debiti
del client che l'inquilino incontrerebbe per primo.

**Fetta C — prima degli utenti veri.** Nominata e **non pianificata**: rate limiting
per IP, verifica dell'email bloccante, manifest Kubernetes, backup e restore,
metriche, alerting, cron per le partizioni di `audit_log`. Non è dimenticanza ed è la
ragione per cui ha un nome: dipendono da **dove** il prodotto viene esercito, che non
è deciso, e pianificarle adesso significherebbe scrivere un piano su un'ipotesi.

**Fuori, e non per dimenticanza:**

- **il fornitore di OCR.** Resta la decisione che blocca più valore di ogni riga di
  codice, ed è una decisione sul trattamento dei dati personali: non è un compito di
  sviluppo e non va messa in coda con quelli. Vedi
  [ADR-037](08-decisioni.md#adr-037);
- **il triage AI delle segnalazioni**, che dipende dalla stessa scelta e in più ha
  una domanda di prodotto sua — con quale soglia di confidenza una macchina possa
  scrivere la categoria di una segnalazione;
- **la traduzione in inglese.** Il catalogo esiste dal primo giorno, quindi aggiungere
  una lingua è un file in più e non una riscrittura: è una decisione di mercato, non
  di completezza, e non blocca nulla;
- **il mobile nativo**, che [ADR-039](08-decisioni.md#adr-039) rimanda alla
  conversazione sulle notifiche push — cioè a dopo che `Notifier` esiste, non prima;
- **la ricerca full-text**, che è V2 e va alimentata dall'outbox.

### Che cosa è stato deciso prima di pianificare

Sei scelte, prese esplicitamente. Le ultime due sono nate **leggendo il codice** dopo
la prima stesura, e hanno cambiato la decomposizione: vedi le decisioni 7 e 8.

| | Scelta | Alternativa scartata |
|---|---|---|
| Ordine | pagamenti, poi inquilino | l'inquilino per intero subito, pagamenti compresi |
| Perdite | nono numero in dashboard, **dentro la fetta A** | lasciare `written_off` indistinguibile da `void` |
| Sovrapagamenti | visibili per addebito, **non** in dashboard | un decimo riquadro, oppure compensare l'insoluto |
| Porte senza adattatore | `Notifier` dentro, OCR fuori | entrambe, o nessuna |
| Posta in sviluppo | raccoglitore SMTP locale nel compose | aspettare un server vero, o un fornitore con API |
| Coda dei pagamenti | `GET /api/v1/payments`, **dentro la fetta A** | solo per addebito, senza coda |

### Decisioni prese, con il perché

#### 1. I pagamenti prima dell'inquilino, ed è una scelta contro l'istinto

L'istinto dice l'inquilino: è metà del prodotto, è la parte che manca, ed è appena
diventata provabile con dati veri. È un buon argomento e non è quello decisivo.

Il decisivo è che **il giro del denaro è rotto adesso, per l'utente che già esiste**.
Dall'interfaccia un proprietario emette un addebito e poi può solo annullarlo o
stornarlo: **non può registrare un incasso**. Ne segue che l'insoluto in dashboard
sale e non scende mai, e che l'unico modo di farlo scendere è rinunciare ai soldi.
Il numero più in vista del prodotto è oggi un numero che mente per omissione.

**Alternativa scartata: l'inquilino per intero, pagamenti compresi.** Racconto di
prodotto più coerente, fetta molto più grande, e il buco dei pagamenti resta aperto
per tutto il tempo in cui la si costruisce.

#### 2. Il nono numero e lo storno parziale nella **stessa** fetta

`written_off_total` da solo sarebbe un numero contabile che conta **metà** delle
perdite, e per una ragione strutturale: oggi `Charge::write_off` è ammesso dal solo
`issued`, quindi un addebito su cui è entrato del denaro non è stornabile. Il caso
non è di scuola — l'inquilino che versa trecento euro su ottocento e sparisce è
precisamente il caso in cui una perdita si materializza.

Un numero che dice meno del proprio nome è peggio di un numero assente: chi lo legge
non ha modo di sapere che è parziale. Quindi le due cose stanno insieme, e stanno
nella fetta A perché estendere lo storno richiede di rispondere a «che ne è del
denaro già incassato», che è una domanda del modulo dei pagamenti.

**Lo storno sì, l'annullamento no.** `void` su un addebito parzialmente pagato
significherebbe che del denaro è stato ricevuto contro una pretesa che non è mai
esistita, e quella è una domanda a cui questo piano non risponde. Lo storno non ce
l'ha: il denaro incassato resta incassato, e si rinuncia al resto. La distinzione è
già scritta nel dominio, in `Charge::void`, e va rispettata invece che appianata.

#### 3. La perdita è il **residuo**, non l'importo

Ne segue dalla decisione 2 e va scritta perché è il modo più facile di sbagliare
questo numero. Un addebito da 800 € con 300 € incassati e poi stornato è una perdita
di **500**, non di 800: gli altri trecento sono entrati.

Quindi `written_off_total` somma `amount - paid_amount` sugli addebiti `written_off`,
e non `amount`. La vista sa già calcolare `paid_amount`; quello che non si può usare
è `outstanding_amount`, che dalla migration `0024` vale `0.00` proprio su
`written_off`. È la stessa colonna che rende il numero necessario e che non può
produrlo.

#### 4. Il credito da sovrapagamento si **evidenzia**, e non si filtra

Un `outstanding_amount` negativo è un credito verso l'inquilino, e oggi non compare
da nessuna parte perché la vista somma con `WHERE outstanding_amount > 0`.

Resta fuori dalla dashboard — è un cruscotto di ciò che è dovuto, e un credito non è
un insoluto negativo — e nell'elenco degli addebiti **si evidenzia**, come già fa.

**Filtrarlo non si può, e la prima stesura di questo piano lo dava per fatto.** Un
sovrapagamento porta lo stato ricalcolato a `paid`, quindi `?outstanding=true` lo
esclude per costruzione; e `ChargeQuery` non ha nulla sul segno del residuo. Un
filtro vero richiederebbe un parametro nuovo e un indice parziale che lo regga, cioè
il lavoro che `charges_outstanding_ix` è costato per l'insoluto. È rinviato **con la
sua ragione**, e non è una dimenticanza: il caso d'uso — «mostrami tutti i crediti» —
non è ancora nato, e costruire l'indice prima del caso è il modo di costruire quello
sbagliato.

**Alternativa scartata: compensare `total_outstanding`.** Cambierebbe il significato
di un numero che un client legge già, quindi sarebbe una rottura, e nasconderebbe
esattamente lo scarto che la contabilità esiste per scoprire.

#### 5. `Notifier` dentro, adattatore SMTP, e un raccoglitore locale in sviluppo

È l'ultima porta dichiarata e senza implementazione, e ha **due** consumatori in
attesa: l'invito, che oggi passa da «copia questo codice e mandalo tu», e — dalla
fetta A — la dichiarazione di pagamento in `pending`.

**SMTP e non un fornitore con la sua API**, per la stessa ragione per cui
`ObjectStorage` ha un adattatore solo che parla anche con MinIO: con SMTP il
fornitore diventa un **dato di configurazione** invece di un ramo del codice, e
cambiarlo è una variabile d'ambiente. Un adattatore per l'API di un fornitore
specifico si scrive quando quel fornitore offre qualcosa che SMTP non ha —
tracciamento delle aperture, template remoti — che è roba di V2.

**In sviluppo l'adattatore parla con un raccoglitore SMTP locale nel compose**, come
MinIO sta al posto di S3: si scrive e si verifica senza credenziali di nessuno, e
server e mittente veri restano configurazione di deploy. Ne segue che questa fetta
**non è bloccata** da una scelta di fornitore, che era il rischio della prima stesura.

**Il token dell'invito continua a tornare nella risposta HTTP.** Toglierlo sarebbe una
rottura, e soprattutto lascerebbe senza rimedio il caso in cui l'email non arriva. Il
client mostra «invito inviato a …» e tiene il codice sotto un dettaglio richiudibile,
che è la verità: l'email è partita, e se non arriva c'è ancora una via.

#### 6. L'endpoint delle organizzazioni **prima** della fetta B, non dopo

Il selettore dell'organizzazione lavora oggi su ciò che *quel browser* ha conosciuto,
perché nessun endpoint elenca le organizzazioni di un utente. Per il proprietario, che
ne ha una, è indistinguibile da un elenco vero. **Per l'inquilino non lo è**: su un
browser mai usato non vedrebbe l'organizzazione del proprietario finché non riaccetta
un invito, e il sintomo sarebbe una schermata vuota senza errore — il modo peggiore di
sbagliare, e quello che la fetta del client ha già speso una decisione per evitare.

Quindi è il passo **B0** e non un debito da chiudere dopo: costruirci sopra le
schermate dell'inquilino significherebbe costruirle su un difetto noto.

La rotta è `GET /api/v1/organizations`, senza prefisso `me`: `organizations_select` è
già `is_org_member`, quindi la Row Level Security restringe da sé alle proprie, e il
percorso dice il vero senza aggiungere una parola.

#### 7. `GET /api/v1/payments`, la rotta che manca perché la coda esista

**Trovata leggendo il codice, e non c'era nella prima stesura di questo piano.**
`PaymentQuery` ha `charge_id` **obbligatorio**, non esiste una rotta di collezione, e
un filtro sullo stato non c'è — con la ragione scritta nel dominio: «i pagamenti di un
addebito sono pochi, e un filtro su un elenco che sta in una pagina non compra
niente». Vero per l'elenco *di un addebito*, e falso per la domanda che la fetta A
pone: **quali dichiarazioni aspettano una conferma, in tutta l'organizzazione**.

Senza quella rotta la coda non esiste, il proprietario scopre una dichiarazione solo
aprendo gli addebiti a uno a uno, e la notifica della decisione 5 avviserebbe di
qualcosa che poi non si trova.

`GET /api/v1/payments`, paginata a cursore come le altre, con `?status=` e
`?lease_id=`. L'organizzazione la restringe la Row Level Security, come per ogni altra
collezione: non serve un parametro. `PaymentQuery` diventa quindi `charge_id:
Option<ChargeId>` più i filtri nuovi, e la rotta annidata esistente resta com'è —
passa lo stesso oggetto con l'addebito valorizzato.

**Ne segue un indice.** Il filtro sullo stato su una collezione che cresce con ogni
incasso vuole un indice parziale su `pending`, che è la stessa forma di
`charges_outstanding_ix` e di `tickets_open_ix`. Va nella stessa migration del nono
numero.

#### 8. L'invito **non può** passare dall'outbox, e l'istinto dice il contrario

È la trappola più cara di questa fetta e va scritta come decisione, perché la via
sbagliata è quella che sembra giusta.

L'istinto — corretto ovunque altrove in questo progetto — è: lo use case emette un
evento nella stessa transazione, il worker lo consuma e manda l'email. È il pattern
dell'outbox, ed è la ragione per cui la fetta dell'OCR dice «l'esito di un caricamento
non dipende dalla disponibilità del fornitore».

**Qui non si può, e la ragione è nel dominio.** Il token d'invito in chiaro esiste
**solo** nella risposta HTTP: in database ne resta lo hash, e gli eventi di dominio
non trasportano valori né credenziali — «identificatori e nomi di campo, non valori».
Un worker che leggesse `InvitationIssued` non avrebbe niente da spedire. Metterci il
token dentro significherebbe scrivere una credenziale in una tabella che esiste per
essere pubblicata su un broker, che è il contrario di ciò per cui il token è hashato.

Quindi l'invio dell'invito è **dopo il commit, best-effort**: se il fornitore di posta
non risponde, l'invito resta valido e il token resta nella risposta. Il fallimento va
nei log senza il destinatario e senza il token, e **non** diventa un errore HTTP —
l'invito è stato creato, dirlo fallito sarebbe falso.

**La dichiarazione di pagamento non ha questo vincolo**: non c'è alcuna credenziale
da recapitare, quindi potrebbe passare dall'outbox. In questa fetta segue comunque la
stessa via — dopo il commit, best-effort — per non avere **due** meccanismi di invio
appena nato il primo. Il giorno in cui le notifiche diventeranno molte, l'outbox è la
strada, e questa nota è scritta perché chi ci arriverà sappia che per l'invito quella
strada resta chiusa.

### Decomposizione

#### Fetta A — il giro del denaro si chiude

Undici passi. Il **punto di taglio è dopo A8**: lì il proprietario ha un modulo
contabile completo con quello che l'API già sa fare, e le notifiche sono una fetta
A-bis.

1. **Dominio: lo storno da `partially_paid`.** `Charge::write_off` ammette anche quello
   stato; `void` no, e il commento che lo spiega resta. Test unitari sulle due
   transizioni e sul rifiuto di `void`. Nessuna migration: la `0023` concede già
   l'`UPDATE` sulla sola `status`.
2. **Migration: il nono numero e l'indice della coda.** `CREATE OR REPLACE VIEW
   app.v_owner_dashboard` in una migration nuova — lo schema è append-only, quindi la
   `0015` non si tocca — con `written_off_total` calcolato come
   `SUM(amount - paid_amount)` sugli addebiti `written_off`. Nella stessa migration
   l'indice parziale su `app.payments` per `status = 'pending'` (decisione 7).
   Invarianti dello schema, e il controllo che la vista resti `security_invoker`.
3. **Backend: il nono numero esce.** `OwnerDashboard`, il repository, il DTO, il test
   di contratto che lo verifica su un addebito **parzialmente pagato** e poi stornato.
4. **Backend: `GET /api/v1/payments`.** `PaymentQuery` con `charge_id` opzionale più
   `status` e `lease_id`, repository, use case, rotta, registrazione in `openapi.rs`,
   test di contratto — compreso che un inquilino veda i **propri** pagamenti e non
   quelli altrui, che è la policy e non un ramo sul ruolo.
5. **Client: il livello dati dei pagamenti.** Le sei rotte più quella nuova, con la
   chiave di idempotenza generata **una volta per intento** e riusata a ogni
   ritentativo.
6. **Client: registrare un incasso.** Dal dettaglio di un addebito. `payer_user_id` è
   una scelta esplicita fra le parti del contratto — o un «non attribuito» dichiarato
   — e non un campo che si dimentica.
7. **Client: la coda delle dichiarazioni.** Elenco dei pagamenti `pending`
   dell'organizzazione, conferma e rifiuto, con la spiegazione di che cosa muove il
   saldo e che cosa no.
8. **Client: i due numeri.** Il nono riquadro in dashboard e il credito evidenziato
   nell'elenco addebiti.
9. **`Notifier`: la porta e l'adattatore.** La variante `PaymentDeclared` in
   `NotificationContent` — oggi ce n'è **una sola**, `TenantInvitation` —
   l'adattatore SMTP in `infrastructure`, il raccoglitore locale nel
   `docker-compose.yml`, la configurazione in `AppConfig` e nelle tabelle di
   [07 — Operazioni](07-operazioni.md).
10. **Gli invii.** Iniezione della porta in `InviteTenant` e in `DeclarePayment`:
    firma degli use case, `AppState`, `Dependencies`, `main.rs` e l'harness dei test.
    Invio **dopo il commit e best-effort** (decisione 8). Il client mostra «inviato» e
    tiene il codice sotto un dettaglio.
11. **Documentazione ed esito.** ADR per l'adattatore SMTP e per il nono numero;
    aggiornamento di [05 — API](05-api.md); esito, e il piano si sposta nell'archivio.

#### Fetta B — l'inquilino

Dieci passi. Il **punto di taglio è dopo B3**: con le segnalazioni complete
l'inquilino ha un prodotto che sta in piedi, e le spese co-living sono una fetta
B-bis.

0. **Backend: `GET /api/v1/organizations`.** Use case di lettura, DTO, rotta, test di
   contratto. Chiude il debito che il primo client ha trovato.
1. **Client: il selettore letto dall'API**, e non più dal `localStorage`. Quello resta
   solo per ricordare **quale** era attiva, che è una preferenza e non un elenco.
2. **Client: segnalazioni, la lettura.** Apertura su un immobile, elenco con
   `?open=true`, dettaglio con lo storico degli stati, thread dei messaggi paginato a
   parte.
3. **Client: segnalazioni, le transizioni.** Le sette sotto-risorse e il `PATCH` dei
   soli costi, con l'instradamento sui codici e non sul ruolo.
4. **Client: spese co-living, i gruppi.** Creazione, composizione, ingresso e uscita,
   archiviazione.
5. **Client: spese e saldi.** Ripartizione `equal` e `weighted`, quote mostrate,
   saldi con il netto.
6. **Client: i regolamenti a due tempi.** Dichiarazione, conferma di chi riceve,
   rifiuto, ritiro di chi ha dichiarato.
7. **Client: la dichiarazione di pagamento** dell'inquilino, con la chiave di
   idempotenza della fetta A.
8. **Client: gli allegati sugli owner rimasti** — immobile, stanza, contratto,
   segnalazione, addebito — e l'archiviazione di un documento.
9. **Client: i due debiti che l'inquilino incontra.** Divisione del bundle per rotta,
   con i `Suspense` progettati insieme agli stati di caricamento che già esistono; e
   il coordinamento del refresh fra schede.
10. **Documentazione ed esito.**

### Trappole

Tutte lette nel codice e nei documenti prima di scriverle qui. Le prime quattro sono
state trovate **rileggendo il codice dopo la prima stesura del piano**, e ognuna
avrebbe cambiato il lavoro a metà strada.

**L'invito non può passare dall'outbox.** È la decisione 8, vista dal lato del
difetto: il worker non ha il token in chiaro, e metterglielo in un evento
significherebbe scrivere una credenziale in una tabella pensata per essere pubblicata.
Chi arriva qui con l'abitudine dell'outbox — cioè chiunque abbia letto il resto del
progetto — prende la strada sbagliata per la ragione giusta.

**`Notifier` non è iniettata da nessuna parte.** `InviteTenant::new` prende tre
argomenti e nessuno è un `Notifier`, e `NotificationContent` ha **una sola variante**.
Il passo 9 non è «scrivere un adattatore»: tocca la porta, lo use case, `AppState`,
`Dependencies`, `main.rs` e l'harness dei test.

**`PaymentQuery` è per addebito, non per organizzazione.** `charge_id` è obbligatorio
e non c'è filtro sullo stato, con una ragione scritta che vale per l'elenco di un
addebito e non per la coda. Renderlo opzionale non deve cambiare il comportamento
della rotta annidata, che continua a passare l'addebito valorizzato.

**Il credito non è filtrabile lato server**, perché un sovrapagamento porta lo stato a
`paid` e `?outstanding=true` lo esclude. Si evidenzia; filtrarlo è un parametro nuovo
e un indice, ed è rinviato con la ragione (decisione 4).

**La chiave di idempotenza non si rigenera al ritentativo.** La genera il client, ed è
deliberato: se la generasse il server ogni ritentativo produrrebbe una chiave nuova,
quindi un secondo pagamento, quindi un saldo che scende **due volte**, e `audit_log`
mostrerebbe due inserimenti perfettamente legittimi. Ne segue una condizione precisa
lato client: la chiave si genera **all'intento** — l'invio del form — e si riusa per
ogni tentativo successivo. Generarla dentro `mutationFn` la rigenererebbe a ogni
ritentativo, che è esattamente il difetto che l'header esiste per impedire.

**La perdita è il residuo e non l'importo.** Vedi la decisione 3. Sommare `amount`
gonfia le perdite di tutto ciò che è stato incassato. E `outstanding_amount` non serve
a calcolarle, perché la `0024` lo azzera proprio su `written_off`: è la stessa colonna
che rende il numero necessario e che non può produrlo.

**Una chiave ripetuta risponde `200`, non `201` e non `409`.** Il client deve trattare
i due stati come lo stesso esito riuscito. Il `409` esiste per due casi che la
rilettura non assorbe — la stessa chiave verso un **altro** addebito, e una chiave
usata da qualcuno di cui il chiamante non vede i pagamenti — e nessuno dei due si
risolve riprovando.

**Chi dichiara non può ritirare, e il client non deve offrirglielo.**
`payments_update` è staff soltanto, quindi un bottone «ritira la dichiarazione»
otterrebbe zero righe aggiornate, cioè `auth.denied_by_policy` — il codice riservato
ai nostri difetti. L'uscita da `pending` è il rifiuto, ed è dello staff.

**`payer_user_id` omesso rende l'incasso invisibile a chi ha pagato.** È lecito — un
bonifico da un conto che non si sa a chi ricondurre esiste — ma è anche il
comportamento di chi semplicemente dimentica il campo, che è il caso più facile. Il
debito dice che con il primo client si vede se debba diventare obbligatorio: **questa
è quella occasione**, e la risposta va scritta nell'esito.

**Il rifiuto azzera `paid_at` e non muove il saldo**, perché un pagamento `pending`
non lo aveva mosso. Un client che mostrasse «saldo aggiornato» dopo un rifiuto direbbe
il falso.

**Un raccoglitore SMTP locale non è un fornitore vero.** Accetta tutto: non verifica
SPF, DKIM né i rimbalzi, e un indirizzo inesistente gli va benissimo. Verifica che il
messaggio parta e che cosa contiene, **non** che sarebbe recapitato. È lo stesso
limite di MinIO rispetto a S3, ed è il motivo per cui la prima spedizione verso un
server vero va guardata.

**L'adattatore SMTP è il posto più facile in cui far uscire dati personali.**
`docs/04` vieta email, token e corpi nei log, e un client SMTP che registri la
richiesta li registra tutti e tre insieme. Il token dell'invito in particolare è una
**credenziale**: va nel corpo del messaggio e in nessun log. La porta ha già un `Debug`
redatto, e l'adattatore non deve annullarlo.

**`?open=true` non è zucchero per `status <> 'closed'`.** L'indice parziale
`tickets_open_ix` è definito su esattamente sei valori, e una disuguaglianza non lo
userebbe. Lo stesso vale per `?outstanding=true` sugli addebiti.

**`resolved` esiste nell'enum e l'API non lo produce mai.** Un client che costruisse i
filtri dai valori dell'enum offrirebbe uno stato che non esiste — è la stessa forma
della trappola sugli stati derivati, che la fetta del client ha già incontrato due
volte.

**La riapertura è di chi ha segnalato, e il codice lo dice.** A un coinquilino che ci
provi l'API risponde `403 ticket.not_yours` e **non** `org.staff_role_required`: non
gli manca un ruolo — nessun ruolo gli darebbe quell'operazione. Il client instrada sul
codice, e mostra due messaggi diversi.

**`estimated_cost` è `null` per chi non è staff anche quando un valore c'è.** Mostrare
`0,00 €` sarebbe una bugia; mostrare `—` è la verità.

**Le note interne non arrivano proprio.** Le filtra la policy di SELECT, non un campo
della risposta: il client non deve nasconderle, non le riceve. E a un inquilino che le
chieda esplicitamente l'API risponde `403`.

**Tre metodi di ripartizione su cinque danno `422`.** `percentage`, `exact` e
`by_usage` compaiono nella specifica e sono rifiutati. Ancora una volta: costruire il
menù dall'enum ci finisce dentro.

**Il peso ha tre decimali, gli importi due.** Un quarto decimale su `weight` dà `422`
sul campo. La doppia si esprime con `"2"`.

**Chi è uscito da un gruppo non vede i propri saldi.** La vista è `security_invoker` e
passa da una policy che chiede una membership **attiva**: il debitore uscito compare
nei saldi che leggono gli altri e non nei propri. È corretto rispetto all'isolamento
ed è deliberatamente fuori perimetro — il client non deve presentarlo come un errore.

**La somma dei `net_balance` fa zero, e va verificata con il modulo decimale.**
Sommarli con l'aritmetica nativa produrrebbe uno zero che a volte non è zero, cioè un
controllo che fallisce a caso.

**Il bundle diviso per rotta senza `Suspense` progettati produce una pagina bianca**
dove prima c'era uno scheletro. I tre stati esistono già come componenti: i confini
vanno messi dove quelli sono, non dove capita.

### Come si saprà che è finita

`./ops/ci/test-integration.sh` verde e il job del client verde sono il minimo, non il
criterio.

**Fetta A, le quattro verifiche che contano:**

1. **L'insoluto scende.** Un giro a mano dal browser: addebito da 800 €, incasso di
   300 € registrato, la dashboard passa da 800 a 500 e lo stato dell'addebito diventa
   `partially_paid`. È il giro che oggi non si può fare.
2. **La stessa chiave inviata due volte lascia il saldo dov'è.** Non è la suite: è il
   doppio clic su un bottone, fatto a mano, con il saldo guardato prima e dopo.
3. **Il nono numero dice il residuo.** Sull'addebito del punto 1, stornato dopo
   l'incasso parziale: `written_off_total` deve valere **500** e non 800.
4. **La coda trova la dichiarazione.** Un inquilino dichiara di aver pagato — dal
   `curl`, finché la fetta B non esiste — e il proprietario la trova nella coda senza
   sapere quale addebito guardare. È la ragione per cui la rotta della decisione 7
   esiste.

Più, se i passi 9 e 10 sono dentro: **un'email di invito che arriva davvero** nel
raccoglitore locale, con il token nel corpo e in **nessuna** riga di log.

**Fetta B, le tre verifiche che contano:**

1. **Un inquilino entra da un browser mai usato e vede l'organizzazione del
   proprietario**, senza incollare niente. È il debito B0 visto dal lato di chi lo
   subiva.
2. **Una segnalazione fa il giro completo**: l'inquilino la apre, il proprietario la
   classifica, l'assegna, la sospende, la chiude; l'inquilino la riapre. E il numero
   `open_tickets` in dashboard cambia di conseguenza.
3. **I saldi di un gruppo di spesa sommano a zero**, con un membro uscito che ha ancora
   un conto aperto — che è il caso per cui la migration `0027` esiste.

Più le due prove di qualità che il criterio «prodotto reale» rende obbligatorie, come
per la fetta precedente: **navigazione da sola tastiera** su un percorso completo, e
**375 px** sulle schermate nuove.

<a id="esito-completare-il-prodotto-fetta-a"></a>

### Esito — che cosa l'esecuzione ha confermato, cambiato e trovato

Eseguita il **4 settembre 2026**, gli **otto passi** fino al punto di taglio
dichiarato. I passi 9 e 10 — `Notifier`, l'adattatore SMTP e gli invii — **non sono
stati eseguiti**: erano dichiarati come fetta A-bis nel piano, e lo restano.

Il giro del denaro dal lato del proprietario è chiuso: **l'insoluto adesso scende.**

#### Che cosa è entrato

| Passo | Esito |
|---|---|
| A1 — storno da `partially_paid` | `Charge::write_off` ammette `Issued` e `PartiallyPaid`; `void` resta al solo `Issued`. Due test unitari nuovi. Nessuna migration |
| A2 — migration `0032` | `written_off_total` su `v_owner_dashboard`, e `payments_pending_ix` parziale su `status = 'pending'` |
| A3 — il nono numero | `OwnerDashboard.written_off_total`, repository, DTO, tre test di contratto |
| A4 — `GET /api/v1/payments` | `PaymentQuery` con `charge_id` opzionale più `status` e `lease_id`; rotta di collezione, `openapi.rs`, tre test di contratto e uno di persistenza |
| A5 — livello dati del client | Le sette rotte, i tipi, e `@/api/idempotency` con i suoi cinque test |
| A6 — registrare un incasso | `ChargeDetailPage` — che prima non esisteva — e `RecordPaymentDialog`, con cinque test |
| A7 — la coda | `PaymentsPage`: elenco `pending` dell'organizzazione, conferma e rifiuto |
| A8 — i due numeri | Nono riquadro in dashboard con la sua destinazione, e il credito evidenziato **con la parola accanto al numero** |

423 test dal lato Rust — 10 in più — e 37 dal lato client, 10 in più.

#### Le quattro verifiche che contano, fatte a mano dal browser

Sono il criterio di completamento dichiarato nel piano, e valgono più del verde.

1. **L'insoluto scende.** Addebito da 800 €, incasso di 300 € registrato
   dall'interfaccia: la dashboard è passata da **1300 a 1000**, e lo stato
   dell'addebito da `issued` a `partially_paid`. È il giro che prima non si poteva
   fare, e che rendeva il numero più in vista del prodotto un numero che mentiva per
   omissione.
2. **La stessa chiave inviata due volte lascia il saldo dov'è.** Due `POST` identici
   con la stessa `Idempotency-Key`: **`201` poi `200`**, una sola riga in
   `app.payments`, saldo mosso una volta sola.
3. **Il nono numero dice il residuo.** Sull'addebito del punto 1, stornato dopo
   l'incasso parziale: `written_off_total` vale **500,00 €** e non 800,00 €. E
   `outstanding_amount` sulla stessa riga vale `0.00`, che è il motivo per cui il
   numero non si può calcolare da lì.
4. **La coda trova la dichiarazione.** L'inquilino ha dichiarato da `curl`, e il
   proprietario l'ha trovata in `/pagamenti` **senza sapere quale addebito
   guardare** — con il link che glielo dice.

Più le due prove di qualità: **375 px** sulle tre schermate nuove (le tabelle
scorrono dentro il proprio contenitore, la pagina no) e la **navigazione da sola
tastiera**, coperta da un test con eventi veri e non con click sintetici.

#### La trappola che il piano non aveva previsto, e che ha morso

**`CREATE OR REPLACE VIEW` riscrive la vista per intero, e il corpo si riprende
dall'ultima migration che l'ha toccata — non dalla prima che l'ha creata.**

La `0032` è partita dal corpo della `0015`, che è il primo che si trova cercando
`CREATE VIEW app.v_owner_dashboard`. Così facendo ha **rimesso in piedi il difetto
che la `0017` aveva corretto**: `status = 'active'` al posto di
`app.lease_in_force(...)` in tre punti, cioè contratti scaduti di nuovo contati fra
quelli in essere e i loro canoni di nuovo sommati.

**La migrazione non ha protestato**, e non aveva motivo: la vista era valida, le
colonne erano quelle giuste, il tipo pure. Il sintomo sarebbe stato due numeri gonfi
in dashboard, cioè esattamente il genere di errore che nessuno attribuisce a una
migration.

L'ha vista un test di persistenza che la `0017` si era lasciata dietro apposta —
`an_expired_lease_keeps_its_stored_state_but_leaves_the_dashboard`. Vale la pena
dirlo perché è il ritorno di un investimento fatto una fetta fa da qualcun altro: il
test non serviva a proteggere questa migration, e l'ha protetta.

La regola è ora scritta nella `0032` e in [ADR-040](08-decisioni.md#adr-040).

#### La cosa misurata invece che dedotta

**Un filtro parametrico su un enum non usa un indice parziale, e il pattern di
questo progetto ci cadeva dentro.**

Il piano prevedeva `payments_pending_ix` e dava per scontato che
`($1::text IS NULL OR status = $1::app.payment_status)` — il pattern che *ogni* altro
elenco del progetto usa — lo avrebbe usato. Verificato con `EXPLAIN` su un banco di
80 000 pagamenti: **non lo usa.** Produce un `Bitmap Index Scan` su
`payments_org_status_ix` seguito da un `Sort`.

La causa non è la selettività: il cast da `text` a un enum passa da `enum_in`, che
PostgreSQL dichiara `STABLE` e non `IMMUTABLE`, quindi
`'pending'::text::app.payment_status` non viene ridotto a una costante nemmeno nel
piano *custom*, e senza costante l'implicazione con il predicato dell'indice non è
dimostrabile.

Il rimedio è una condizione ridondante che nomina il letterale, accesa da un booleano
che vale `true` esattamente quando il filtro chiede `pending`. Con quella il piano
diventa `Index Scan using payments_pending_ix`, e **ci resta anche dopo le sei
esecuzioni che portano al piano generico**. Su ogni altro stato il ramo è spento e il
risultato non cambia, quindi `?status=` è rimasto un filtro generale invece di
diventare un secondo flag booleano.

La prima stesura del commento nella migration diceva il contrario — che il piano
custom sarebbe bastato — ed era sbagliata. È stata corretta *dopo* la misura, e la
misura è nel commento perché chi la rileggerà non debba rifarla.

#### La decisione che la fetta doveva prendere: `payer_user_id`

Il debito diceva «con il primo client si vede se l'attribuzione debba diventare
obbligatoria, o se convenga dedurla dalle parti del contratto». **Nessuna delle
due**, e il debito è chiuso.

**Obbligatoria nell'API sarebbe sbagliato**: un bonifico da un conto che non si sa a
chi ricondurre esiste, e non c'è un valore sentinella per «deliberatamente non
attribuito» che non sia `NULL`. Toglierlo lascerebbe l'API senza il modo di dire una
cosa vera.

**Dedurla dalle parti del contratto non si può**: un contratto ha spesso più
inquilini con le loro quote, quindi «il» pagante non è determinato; e una parte senza
`user_id` è un invitato che non ha ancora accettato, a cui l'incasso **non è
attribuibile affatto** perché la policy confronta un identificatore che non esiste.
Quest'ultimo caso non era nel piano ed è emerso costruendo il menù.

**Il difetto vero era che ometterla fosse il default.** Il rimedio sta nel form:
nessun vuoto inviabile, l'elenco di chi ha accettato, e una voce «non attribuito» che
porta scritta la sua conseguenza — che chi ha pagato non vedrà quella riga nel
proprio elenco, pur vedendone l'effetto sul residuo. Un campo dimenticato diventa una
scelta dichiarata, senza togliere una capacità legittima.

La colonna resta com'è, il DTO pure. Vedi [ADR-040](08-decisioni.md#adr-040).

#### Che cosa il piano aveva previsto bene

**Le quattro trappole in cima all'elenco hanno tutte tenuto**, e tre delle quattro
avrebbero fatto perdere tempo:

- **`PaymentQuery` con `charge_id` obbligatorio**: renderlo opzionale ha toccato la
  porta, il repository, lo use case, due rotte e due test già scritti. Il piano
  chiedeva che la rotta annidata non cambiasse comportamento, e non è cambiato — un
  test di contratto verifica che continui a rispondere `404` e non una pagina vuota;
- **la chiave di idempotenza non si rigenera al ritentativo**: il test che lo fissa è
  stato verificato *contro il difetto*, sostituendo `intent.current()` con
  `crypto.randomUUID()` dentro `mutationFn`. Il test è fallito, il che è l'unica prova
  che serva a qualcosa. Un test che non fallisce quando il codice è sbagliato è un
  verde che non prova nulla;
- **la perdita è il residuo**: scritta tre volte — nel dominio, nella migration e nel
  DTO — perché è il modo più facile di sbagliare quel numero;
- **`GET /api/v1/payments` senza la quale la coda non esiste**: confermata dal punto
  4 delle verifiche a mano, che è precisamente il giro che quella rotta rende
  possibile.

**Il nono numero e lo storno parziale nella stessa fetta** era la decisione giusta, e
lo si vede dal caso di prova: senza A1, il giro delle verifiche 1 e 3 non si sarebbe
potuto fare affatto — l'addebito con 300 € dentro non sarebbe stato stornabile, e il
numero avrebbe risposto `0.00` su una perdita che c'era.

#### Che cosa è cambiato rispetto al piano, e perché

**È stata aggiunta una schermata che il piano non nominava**: `ChargeDetailPage`.
Il passo A6 diceva «dal dettaglio di un addebito», e quel dettaglio **non
esisteva** — il client aveva solo l'elenco, e `GET /charges/{id}` non era chiamato da
nessuno. Non è un allargamento di perimetro: è la superficie su cui il passo A6 era
scritto, e senza di essa l'incasso si sarebbe dovuto registrare da una modale
appesa a una riga di tabella, dove non c'è spazio per dire quanto è il residuo.

**È stato aggiunto un filtro sullo stato nell'elenco degli addebiti**, che il piano
non chiedeva. È la destinazione del riquadro *Perdite*: la dashboard di questo
progetto ha la regola che ogni numero porti dove si agisce, e senza quel filtro il
nono riquadro sarebbe stato l'unico a non portare da nessuna parte — cioè una
decorazione, che è il difetto che quella regola esiste per impedire.

**Il bottone «Annulla» sparisce su un addebito parzialmente pagato**, invece di
esserci e dare `409`. Il piano non lo diceva; è la conseguenza diretta della decisione
2, e un bottone che produce sempre un errore è peggio di un bottone assente. Al suo
posto c'è la frase che dice perché e indica la via — la rinuncia all'incasso.

**La coda ha due colonne di data e non una.** La prima stesura mostrava `paid_at`,
che su una dichiarazione **può mancare** — chi dichiara non è obbligato a dire quando
ha pagato — e che il rifiuto azzera. Il risultato era una coda di lavoro la cui
colonna di ordinamento era vuota nel caso comune, cosa che si è vista solo
guardandola con dati veri. Ora ci sono *Ricevuto il* (`created_at`, che c'è sempre) e
*Pagato il* (`paid_at`, con il trattino quando manca).

#### Debiti nuovi, e uno che resta aperto per una ragione operativa

**Nessun debito tecnico nuovo di rilievo.** Le tre cose da sapere sono:

- **la coda non è filtrabile per immobile**, solo per contratto e per stato. Nasce
  dal fatto che `app.payments` non ha `property_id` e il legame passerebbe da due
  salti (`payment → charge → lease → property`). Non serve finché un proprietario ha
  pochi immobili; con decine, la coda diventa lunga e il filtro va aggiunto;
- **il bundle è cresciuto a 614 kB** (185 kB compressi) da 597. Il debito della
  divisione per rotta era già registrato ed è ancora della fetta B;
- **`.env.example` ha quattro variabili di storage che un `.env` più vecchio non
  ha.** Chi ha copiato il file prima che lo storage esistesse trova l'API che non
  parte con `missing field access_key_id`. Non è un difetto del codice — la
  configurazione è validata all'avvio apposta — ma il messaggio non dice da dove
  arriva la variabile. Trovato avviando l'API a mano per il giro dal browser.

**Il `docker compose up` che «si era piantato» non era piantato.** Era la prima
compilazione `--release` dell'intero albero delle dipendenze dentro l'immagine, che
BuildKit mostra come un singolo passo fermo per molti minuti senza output. Verificato:
`docker compose build api` completa, e lo stack intero sale e serve il nono numero
dal reverse proxy su `http://localhost:8081`. Non c'è niente da correggere; c'è da
saperlo, e ora è scritto.


---

<a id="piano-inquilino-fetta-b"></a>

## Piano — L'inquilino (fetta B)

Pianificata il **4 settembre 2026**, dopo la fetta A e leggendo il codice che tocca.
**Dodici passi**, `B0`–`B11`. Il **punto di taglio è dopo `B5`**: lì l'inquilino ha un
prodotto che sta in piedi — entra, vede che cosa deve, **dichiara di aver pagato**,
segnala un guasto e ne segue il corso — e le spese co-living sono una fetta B-bis.

### Obiettivo

L'altra metà del prodotto. Oggi il client copre il proprietario e basta: un inquilino
che entrasse **atterrerebbe su un errore**, perché la pagina iniziale è la dashboard e
la dashboard è staff soltanto.

Il perimetro è la differenza fra le due metà, non il modulo: le segnalazioni sono
dell'inquilino *e* del proprietario, e vanno costruite per entrambi in una volta —
sono la stessa schermata vista da due ruoli, e farne due sarebbe farne due da tenere
allineate.

### Che cosa è stato deciso prima di pianificare

Sei scelte. Le prime tre sono nate **leggendo il codice**, e due di esse hanno tolto
lavoro dal piano invece di aggiungerne.

| | Scelta | Alternativa scartata |
|---|---|---|
| Autorizzazione di `B0` | `ensure_authenticated`, non `ensure_member` | chiedere un'organizzazione attiva per sapere quali si hanno |
| Guscio | un passo suo, **prima** delle segnalazioni | adattare il guscio strada facendo, schermata per schermata |
| Assegnazione | **solo a sé stessi**, nessun endpoint delle persone | `GET /members` dentro questa fetta |
| Pagina iniziale dell'inquilino | lo scadenzario, nessuna rotta nuova | una dashboard dell'inquilino |
| Composizione dei gruppi di spesa | dai residenti dell'immobile, che esistono già | un elenco di persone dell'organizzazione |
| Navigazione dell'inquilino | addebiti, segnalazioni, contratto, bollette | tutto ciò che l'API gli concede, o il solo minimo |
| Punto di taglio | dopo `B5`, **dichiarazione di pagamento compresa** | dopo le sole segnalazioni, lasciando il giro del denaro a metà |

### Decisioni prese, con il perché

#### 1. `B0` è `ensure_authenticated`, e non `ensure_member`

«Quali organizzazioni sono mie» **non appartiene a nessuna organizzazione**: chiederne
una attiva per poterlo domandare è circolare, e rompe proprio il caso che l'endpoint
esiste per servire — chi non ne ha una selezionata, o non ne ha più.

**Verificato prima di deciderlo, perché è ciò che lo rende possibile:**
`app.member_role(p_org_id)` interroga `app.org_members` filtrando su
`app.current_user_id()` e sull'organizzazione **passata come argomento**;
`app.current_org_id()` non compare. `organizations_select` è `is_org_member(id)` e
`org_members_select` è `user_id = current_user_id() OR is_org_member(org_id)`: **la
query funziona con il contesto RLS privo di organizzazione**, e
`app.set_request_context` accetta già un `org_id` nullo.

*Alternativa scartata: `ensure_member`.* Funzionerebbe quasi sempre — il token porta
l'organizzazione predefinita — e fallirebbe esattamente quando serve.

#### 2. La risposta porta il **nome**, ed è il punto dell'endpoint

Non è una comodità. Oggi il selettore mostra `t.org.role[...]` e **l'UUID grezzo**,
perché nessun endpoint restituisce il nome di un'organizzazione a chi non ne è staff.
Per il proprietario, che ne ha una, è indistinguibile da un elenco vero. **Per
l'inquilino sono due UUID fra cui indovinare.**

`organizations_select` è già `is_org_member(id)`, quindi il nome è leggibile da un
membro: non serve né una migration né un allargamento di policy. Il debito «nessun
endpoint elenca le organizzazioni» si chiude qui.

La forma: `GET /api/v1/organizations` → `{ items: [{ id, name, slug, role }] }`.
Nessun cursore — le organizzazioni di una persona sono poche per costruzione, e una
paginazione su un elenco che sta in una riga è cerimonia.

**Senza prefisso `me`**: la Row Level Security restringe già alle proprie, e il
percorso dice il vero senza aggiungere una parola.

#### 3. L'assegnazione è **solo a sé stessi**, e non è una semplificazione

`AssignTicketCommand` vuole un `UserId`, e non esiste un endpoint che elenchi le
persone di un'organizzazione — è il debito che la roadmap dice che «un client
incontrerà per primo». Sembra quindi che `B4` sia bloccato.

**Non lo è, e la ragione è che quel menù non avrebbe nessuno dentro.** Verificato:
l'unico modo di diventare staff di un'organizzazione è `RegisterOwner`, che crea
l'organizzazione personale con ruolo `Owner`; `InviteTenant` scrive
`role: OrgRole::Tenant` fisso, e non esiste altra via per creare una membership.
**`OrgRole::Manager` è nell'enum e nessuno lo scrive mai.** Un'organizzazione ha
quindi esattamente uno staff, il suo owner.

L'azione è dunque «**prendi in carico**», che assegna a chi chiama, ed è la verità
completa del modello e non un ripiego. Il giorno in cui esisterà un invito che crea un
gestore, quell'azione diventa un menù — e sarà quella fetta a doverne discutere.

*Alternativa scartata: `GET /api/v1/members` dentro questa fetta.* Non è una rotta in
più: è decidere **quali colonne** un membro possa vedere di un'altra persona, cioè la
stessa domanda di [ADR-036](08-decisioni.md#adr-036), presa senza un caso d'uso che la
guidi. Costruirla ora significa costruire quella sbagliata.

#### 4. L'inquilino atterra sullo scadenzario, e non su una dashboard sua

Serve una pagina iniziale, e la dashboard attuale gli risponde
`403 org.staff_role_required` — non per la Row Level Security ma per `ensure_staff`,
che la **stringe** apposta: la vista passa da `organizations_select`, che è
`is_org_member`, quindi senza quel controllo un inquilino leggerebbe i numeri
dell'intera organizzazione.

Atterra su `/addebiti`, che è la domanda che si fa davvero — *quanto devo* — e che
`ensure_member` gli concede già, ristretta dalle policy ai suoi.

*Alternativa scartata: una dashboard dell'inquilino.* È una proiezione nuova, quindi
un endpoint nuovo, quindi backend dentro una fetta di client — e soprattutto è una
domanda di prodotto che nessuno ha ancora posto: *quali* numeri. È **rinviata con la
sua ragione**, ed è già annotata nel tipo `GetOwnerDashboard` («il suo è un altro
elenco, e sarà un'altra proiezione con la sua rotta»).

**Le voci che vede sono quattro**: *Addebiti*, *Segnalazioni*, *Contratto*, *Bollette*
— più *Spese* da `B6`. **Riepilogo esce** perché gli risponde `403`.

**Immobili esce**, e la ragione non è la sicurezza: `ListProperties` è `ensure_member` e
gliene mostrerebbe uno. Una voce di navigazione che porta a un elenco di un elemento è
rumore, e la scheda dell'immobile resta raggiungibile dal contratto — che è anche il
modo in cui ci si arriva pensando.

**Le bollette restano**, ed è la voce su cui si esita: sono lette e non agite. Ma sono
la ragione di metà dei suoi addebiti, e un riaddebito che non si può risalire a un
documento è una cifra da accettare sulla fiducia.

#### 5. Il guscio si adatta al ruolo, ed è un passo suo

È la cosa che l'abbozzo precedente di questa fetta non nominava, e che si vede
provando: **oggi un inquilino non incontra una schermata mancante, ne incontra sei
sbagliate.**

Le letture in gran parte funzionano — `ListProperties`, `ListLeases`, `ListCharges`,
`ListUtilityBills` sono tutte `ensure_member`, e le policy restringono ai suoi — ma la
navigazione è quella di chi amministra, e ogni azione di scrittura che gli si mostra
risponde `403`: «Registra un immobile», «Emetti un addebito», «Registra un incasso».

Farlo **prima** delle segnalazioni e non strada facendo è deliberato: è il passo che
rende l'inquilino una cosa provabile: finché atterra su un errore, ogni schermata
successiva si collauda solo con l'account del proprietario.

*Alternativa scartata: nascondere le azioni schermata per schermata mentre si
costruisce.* Il ramo sul ruolo finirebbe scritto dodici volte, e la tredicesima
verrebbe dimenticata — che è precisamente come si costruisce un bottone che risponde
sempre `403`.

#### 6. La composizione dei gruppi passa dai **residenti dell'immobile**

`AddGroupMemberCommand` vuole un `UserId`, e questa volta l'elenco esiste:
`GET /api/v1/properties/{id}/residents` restituisce `user_id` e `full_name` per la
**casa**, ed è esattamente ciò per cui la fetta «i coinquilini si vedono» e la
migration `0030` esistono. È il ritorno di un investimento fatto due fette fa.

Ne segue la sua trappola, ed è la stessa forma di `payer_user_id` nella fetta A: un
residente **senza `user_id` non ha ancora accettato l'invito**, e non è aggiungibile a
un gruppo — la policy confronterebbe un identificatore che non esiste. Va mostrato
come non selezionabile con la ragione, non nascosto: nasconderlo farebbe cercare una
persona che sullo schermo c'è.

### Decomposizione

#### `B0` — Backend: `GET /api/v1/organizations`

Use case di lettura (`ensure_authenticated`, decisione 1), DTO con `id`, `name`,
`slug` e `role`, rotta, registrazione in `openapi.rs`, riga in
[05 — API](05-api.md).

`OrgMemberRepository::list_for_user` **esiste già** ed è usata da `Login`; là gira sul
pool privilegiato perché il login è pre-autenticazione, qui gira sotto la Row Level
Security del chiamante. Serve in più una lettura delle organizzazioni per nome:
`OrganizationRepository` ha `find_by_id` e `insert`, quindi o un `find_many` o un join
nel repository delle membership — la seconda evita `N+1` ed è una query sola.

Test di contratto: due organizzazioni per lo stesso utente, un'organizzazione altrui
che non compare, e **la chiamata senza `X-Org-Id` che risponde `200`** — che è la
decisione 1 verificata invece che dichiarata.

#### `B1` — Client: il selettore letto dall'API

`getOrganizations` nel livello dati, la query nel guscio, i nomi al posto degli UUID.

Il `localStorage` **resta**, ma per una cosa sola: ricordare **quale** era attiva. È
una preferenza di quel browser, non un elenco — e la distinzione è ciò che il commento
in `session.tsx` già dichiara e che questo passo rende vera.

I tre stati vanno progettati anche qui: un selettore che sta caricando non deve
mostrare un elenco vuoto, perché un elenco vuoto è indistinguibile da «non appartieni
a niente».

#### `B2` — Client: il guscio si adatta al ruolo

La navigazione diventa una funzione del ruolo, e la pagina iniziale con essa
(decisioni 4 e 5). Per l'inquilino: *Addebiti*, *Segnalazioni*, *Contratto*,
*Bollette*, e atterraggio su `/addebiti`. Le azioni di scrittura riservate allo staff
spariscono per chi non lo è.

**Il ramo sta in un posto solo.** Un `useIsStaff()` letto dalla sessione, e i punti in
cui si usa contati sulle dita: la navigazione, la rotta iniziale, e i bottoni delle
schermate esistenti. Non un `if` per componente.

**Non è un confine di sicurezza e non va scambiato per tale**: a negare è l'API, e la
policy dietro. Qui si evita di *offrire* ciò che verrebbe rifiutato — che è
un'altra cosa, e va scritto perché la differenza si perde.

#### `B3` — Client: segnalazioni, la lettura

Apertura su un immobile (`POST /properties/{id}/tickets`, `ensure_member` — è **il**
punto in cui l'inquilino prende la parola), elenco con `?open=true`, dettaglio con lo
storico degli stati, thread dei messaggi **paginato a parte** dal dettaglio.

#### `B4` — Client: segnalazioni, le transizioni

Le sette sotto-risorse, con i percorsi esatti perché uno di essi non si chiama come il
suo use case:

`/triage` · `/assign` · `/start` · **`/wait`** · `/close` · `/reopen` · `/cancel`

Più il `PATCH /tickets/{id}`, che è **solo i costi** ed è staff.

L'instradamento è **sui codici e non sul ruolo** — vedi le trappole.

#### `B5` — Client: la dichiarazione di pagamento ← *punto di taglio*

`POST /charges/{id}/payment-declarations`, dal dettaglio dell'addebito che **esiste
già** dalla fetta A.

**Sta prima del taglio e non dopo**, ed è un cambiamento rispetto alla prima stesura di
questo piano: è il passo che chiude il giro che la fetta A ha aperto. Là la coda delle
dichiarazioni è stata costruita e collaudata **da `curl`**, perché un inquilino non
aveva un'interfaccia; qui quella coda diventa una cosa che accade da sé. Lasciarlo
dopo il taglio avrebbe reso il criterio di completamento incoerente con la
decomposizione — la verifica 3 lo richiede.

**La chiave di idempotenza c'è già**: `@/api/idempotency` dalla fetta A, con i suoi
cinque test. Va **riusata e non riscritta**, e la condizione è la stessa: si genera
all'intento e si riusa a ogni tentativo.

Il corpo **non ha `payer_user_id`**: chi dichiara è sempre chi chiama, perché
`payments_insert` lo esige a chi non è staff. E **non c'è un bottone per ritirarla**:
vedi le trappole.

#### `B6` — Client: spese co-living, i gruppi

Creazione, composizione dai residenti (decisione 6), ingresso e uscita,
archiviazione. Da qui compare la voce *Spese* nella navigazione.

#### `B7` — Client: spese e saldi

Ripartizione `equal` e `weighted`, quote mostrate, saldi con il netto.

#### `B8` — Client: i regolamenti a due tempi

Dichiarazione, conferma di **chi riceve**, rifiuto, e il **ritiro di chi ha
dichiarato** — che fra i pagamenti non esiste. Vedi le trappole: le due cose si
somigliano e non sono la stessa.

#### `B9` — Client: gli allegati sugli owner rimasti

Immobile, stanza, contratto, segnalazione, addebito — e l'archiviazione di un
documento, che è l'unica rotta dei documenti che nessuno chiama.

#### `B10` — Client: i due debiti che l'inquilino incontra

Divisione del bundle per rotta con i `Suspense` progettati, e il coordinamento del
refresh fra schede. Entrambi registrati fra i debiti noti con il loro perché.

#### `B11` — Documentazione ed esito

ADR per ciò che sopravvive alla fetta, aggiornamento di [05 — API](05-api.md), esito,
e **il piano si sposta in [11 — Piani conclusi](11-piani-conclusi.md) nello stesso
commit**.

### Trappole

Le prime tre sono state trovate leggendo il codice per scrivere questo piano; le
altre erano già note e sono qui perché il piano che le ha prodotte è archiviato e
nessuno lo aprirà mentre esegue.

**Un inquilino non trova schermate mancanti, ne trova sbagliate.** È la decisione 5
vista dal lato del difetto, ed è la ragione per cui `B2` viene prima: le letture
funzionano già, quindi il sintomo non è una pagina vuota ma sei pagine che sembrano
funzionare finché non si preme qualcosa.

**Il menù delle persone non esiste, e non serve.** `AssignTicketCommand` vuole un
`UserId` e sembra chiedere un endpoint nuovo. Un'organizzazione ha un solo staff:
l'azione è «prendi in carico». Chi parte a scrivere `GET /members` costruisce la cosa
sbagliata prima di accorgersi che sarebbe vuota.

**Un residente senza `user_id` non è aggiungibile a un gruppo di spesa.** Non ha
accettato l'invito, e la policy confronterebbe un identificatore che non esiste. Va
mostrato non selezionabile **con la ragione**: è la stessa forma della trappola di
`payer_user_id` nella fetta A, dove il rimedio è stato non offrire un vuoto inviabile.

**La transizione si chiama `/wait` e lo use case `SuspendTicket`.** Cercare
`/suspend` non trova niente.

**`?open=true` non è zucchero per `status <> 'closed'`.** L'indice parziale
`tickets_open_ix` è definito su esattamente sei valori, e una disuguaglianza non lo
userebbe. **La fetta A ha aggiunto una ragione in più per crederci**: lì si è misurato
che perfino un filtro parametrico su un enum non usa un indice parziale, perché il
cast `text → enum` passa da `enum_in`, che è `STABLE` e non si riduce a costante. Vedi
[ADR-040](08-decisioni.md#adr-040).

**`resolved` esiste nell'enum e l'API non lo produce mai.** Un client che costruisse i
filtri dai valori dell'enum offrirebbe uno stato che non esiste. La fetta A ci è
passata una terza volta, e il rimedio è quello: il menù si scrive a mano.

**La riapertura è di chi ha segnalato, e il codice lo dice.** A un coinquilino che ci
provi l'API risponde `403 ticket.not_yours` e **non** `org.staff_role_required`: non
gli manca un ruolo — nessun ruolo gli darebbe quell'operazione. Il client instrada sul
codice e mostra due messaggi diversi. Vale anche per l'annullamento, che chi ha
segnalato può fare **finché nessuno ci ha messo mano**.

**`estimated_cost` è `null` per chi non è staff anche quando un valore c'è.** Mostrare
`0,00 €` sarebbe una bugia; mostrare `—` è la verità. E **non è un confine di
sicurezza** ma una regola del contratto dell'API, dichiarata in
`authorization::discloses_estimated_cost`: la Row Level Security lascia comunque
leggere la riga.

**Le note interne non arrivano proprio.** Le filtra la policy di `SELECT`, non un
campo della risposta: il client non deve nasconderle, **non le riceve**. E a un
inquilino che le chieda esplicitamente l'API risponde `403`.

**Tre metodi di ripartizione su cinque danno `422`.** `percentage`, `exact` e
`by_usage` compaiono nella specifica e sono rifiutati.

**Il peso ha tre decimali, gli importi due.** Un quarto decimale su `weight` dà `422`
sul campo. La doppia si esprime con `"2"`.

**Chi è uscito da un gruppo non vede i propri saldi.** La vista è `security_invoker` e
passa da una policy che chiede una membership **attiva**: il debitore uscito compare
nei saldi che leggono gli altri e non nei propri. È corretto rispetto all'isolamento
ed è deliberatamente fuori perimetro — il client non deve presentarlo come un errore.

**La somma dei `net_balance` fa zero, e va verificata con il modulo decimale.**
Sommarli con l'aritmetica nativa produrrebbe uno zero che a volte non è zero.

**Confermare un regolamento non ricalcola niente**, a differenza di `ConfirmPayment`:
il saldo del gruppo non è memorizzato da nessuna parte, si legge dalla vista. Chi
arriva dal modulo dei pagamenti si aspetta un `recalculate_*` che qui non serve e non
deve esistere.

**Il ritiro esiste fra i regolamenti e non fra i pagamenti.** `settlements_update`
ammette `from_user_id`, `payments_update` è staff soltanto. Le due cose si somigliano,
e offrire il ritiro di una dichiarazione di pagamento darebbe `auth.denied_by_policy`
— il codice riservato ai nostri difetti.

**Il bundle diviso per rotta senza `Suspense` progettati produce una pagina bianca**
dove prima c'era uno scheletro. I tre stati esistono già come componenti: i confini
vanno messi dove quelli sono, non dove capita.

### Come si saprà che è finita

`./ops/ci/test-integration.sh` verde e il job del client verde sono il minimo, non il
criterio. Le quattro verifiche che contano, tutte da fare **a mano e con due account**:

1. **Un inquilino entra da un browser mai usato e vede l'organizzazione del
   proprietario per nome**, senza incollare niente e senza atterrare su un errore. È
   il debito `B0` e la decisione 4 visti dal lato di chi li subiva;
2. **Una segnalazione fa il giro completo**: l'inquilino la apre, il proprietario la
   classifica, la prende in carico, la mette in attesa, la chiude; l'inquilino la
   riapre. E `open_tickets` in dashboard cambia di conseguenza — che è anche il
   momento in cui quel riquadro smette di portare ai contratti per mancanza di una
   destinazione propria;
3. **L'inquilino dichiara di aver pagato dall'interfaccia**, e il proprietario la
   conferma dalla coda della fetta A: il residuo dell'addebito scende. È il giro che
   nella fetta A si era potuto provare solo da `curl`, ed è la ragione per cui `B5`
   sta **dentro** il punto di taglio;
4. **I saldi di un gruppo di spesa sommano a zero**, con un membro uscito che ha
   ancora un conto aperto — il caso per cui la migration `0027` esiste.

Più le due prove di qualità che il criterio «prodotto reale» rende obbligatorie:
**navigazione da sola tastiera** su un percorso completo dell'inquilino, e **375 px**
sulle schermate nuove — che per l'inquilino non è un di più, perché è il ruolo che sta
davvero su un telefono ([ADR-039](08-decisioni.md#adr-039)).

<a id="esito-inquilino-fetta-b"></a>

### Esito — che cosa l'esecuzione ha confermato, cambiato e trovato

**Eseguita il 4 settembre 2026**, fino al **punto di taglio dichiarato**: `B0`–`B5`.
`B6`–`B8` (spese co-living), `B9` (allegati sugli owner rimasti) e `B10` (i due
debiti) **non** sono stati eseguiti e restano in
[09 — Stato e roadmap](09-stato-e-roadmap.md) come la fetta successiva. Il piano si
sposta qui perché la parte che dichiarava di eseguire è eseguita; ciò che resta è
ripreso di là con il suo dettaglio, non cancellato.

#### Il piano ha retto, e le tre trappole trovate leggendo il codice erano vere

Le sei decisioni prese prima di pianificare sono state tutte confermate
dall'esecuzione, e nessuna è stata rivista. Le tre trappole nate dalla lettura del
codice si sono presentate **esattamente nella forma prevista**:

1. **«Un inquilino non trova schermate mancanti, ne trova sei sbagliate»** è la
   ragione per cui `B2` stava prima delle segnalazioni, e provandolo si è visto che
   era la decisione giusta: accedendo come inquilino **prima** di `B2`, il client
   atterrava sul riepilogo — `403` — e mostrava «Registra un immobile», «Emetti un
   addebito», «Registra un incasso». Dopo `B2` la stessa sessione atterra su
   `/addebiti` con quattro voci, e ogni schermata successiva si è potuta collaudare
   **con l'account dell'inquilino** invece che con quello del proprietario. Se `B2`
   fosse stato fatto per ultimo, `B3`–`B5` sarebbero stati costruiti alla cieca;
2. **il menù delle persone non serviva**, e non è stato costruito. «Prendi in carico»
   assegna a chi chiama, e nella prova a mano è l'unica azione sensata: l'unico staff
   dell'organizzazione è il proprietario che l'ha creata;
3. **`estimated_cost` è `null` per chi non è staff anche quando un valore c'è**,
   verificato a mano e non solo per lettura: il proprietario ha scritto `120,00 €`,
   e la stessa segnalazione aperta dall'inquilino mostra `—` sul costo stimato e
   `95,50 €` su quello sostenuto. Mostrare `0,00 €` sarebbe stata una bugia.

Anche le trappole ereditate hanno tenuto: la transizione si chiama `/wait` mentre lo
use case è `SuspendTicket` (cercare `/suspend` non trova niente, ed è stato scritto nel
livello dei dati perché il prossimo non lo ricerchi); `resolved` è nell'enum e il menù
dei filtri è scritto a mano; la riapertura è di chi ha segnalato e l'instradamento è
sui codici.

#### Che cosa è cambiato rispetto al piano

**Tre aggiunte, tutte piccole e tutte trovate provando.**

**Il riquadro «Segnalazioni aperte» ora porta alle segnalazioni.** Il piano lo
prevedeva come conseguenza della verifica 2 e così è stato: fino a questa fetta quel
riquadro portava ai **contratti** per mancanza di una destinazione propria — il numero
era vero e non c'era una schermata dove agire. Ora c'è.

**La correzione dell'organizzazione ricordata non era nel piano, e serve.** `B1`
diceva «il `localStorage` resta, ma per ricordare *quale* era attiva». Costruendolo si
è visto che manca un pezzo: se l'organizzazione ricordata **non è più fra le proprie**
— un invito revocato, un'appartenenza chiusa — il client continua a mandare quell'
`X-Org-Id` e **ogni** richiesta risponde `403 org.not_a_member`. Non è un caso di
laboratorio: è ciò che accade a chiunque smetta di essere inquilino di qualcuno. Il
rimedio è che l'elenco vero **corregge il ricordo** quando arriva, e c'è un test che lo
riproduce. Lo stesso vale per il ruolo, che si memorizza per non far lampeggiare la
navigazione al ricaricamento e non è autorevole.

**`PageHeader` non sapeva andare a capo, e si è visto solo a 375 px.** Le azioni
stavano in un contenitore `shrink-0`: prendeva la propria larghezza a contenuto pieno e
usciva dallo schermo, e il suo `flex-wrap` non poteva farci niente perché non aveva un
limite entro cui avvolgere. Nessuna schermata lo aveva mostrato prima perché nessuna
aveva più di tre azioni; **una segnalazione in stato `open` ne ha cinque**, e a 375 px
faceva scorrere in orizzontale la *pagina* — che è precisamente ciò che la regola sulle
tabelle vieta. Corretto in `page.tsx`, e il rimedio migliora anche le schermate della
fetta A.

#### Quello che il punto di taglio ha comprato

`B5` era stato spostato **dentro** il taglio in una revisione del piano, e
l'esecuzione dice che era la decisione giusta. Nella fetta A la coda delle
dichiarazioni era stata costruita e collaudata **da `curl`**, perché un inquilino non
aveva un'interfaccia. Qui il giro si è chiuso da sé: l'inquilino ha dichiarato
`500,00 €` di acconto su un canone da `800,00 €`, il residuo è **rimasto** a
`800,00 €` — una dichiarazione `pending` non muove il saldo, ed è tutto il suo punto —
il proprietario l'ha confermata dalla coda della fetta A, e il residuo è sceso a
`300,00 €` con lo stato passato a `partially_paid`. Nessun `curl`.

Lasciare `B5` fuori avrebbe reso il criterio di completamento incoerente con la
decomposizione, e avrebbe consegnato una fetta in cui l'inquilino vede quanto deve e
non ha modo di dire che l'ha pagato.

#### Le quattro verifiche a mano

Tutte con **due account veri**, in un browser senza sessione, con l'API e il client
sul percorso nativo.

1. **Passata.** L'inquilino entra e vede due organizzazioni **per nome** — la propria
   («Giulia Bianchi», *Proprietario*) e quella di chi gli affitta casa («Immobili di
   Mario Rossi», *Inquilino*) — senza incollare niente. Scegliendo la seconda, la
   navigazione diventa *Addebiti · Segnalazioni · Contratto · Bollette* e atterra
   sullo scadenzario. Prima di questa fetta avrebbe visto due UUID e un `403`;
2. **Passata.** La segnalazione ha fatto il giro completo: l'inquilino l'ha aperta
   (`TK-000001`), il proprietario l'ha classificata, presa in carico, avviata, messa in
   attesa dell'inquilino, ripresa e chiusa con la nota; l'inquilino l'ha riaperta. Lo
   storico porta **otto** righe, dalla prima scritta dal trigger (`Aperta`) fino a
   `Chiusa → Aperta`. `open_tickets` in riepilogo è andato `1 → 0 → 1`. La riapertura
   ha **cancellato** `closed_at` e la nota di risoluzione, come il dominio prescrive:
   una `open` che dichiara come è stata risolta direbbe il falso sul proprio stato;
3. **Passata.** Vedi sopra: dichiarazione, conferma, residuo che scende;
4. **Non eseguita, ed è fuori dal punto di taglio.** I saldi dei gruppi di spesa sono
   `B6`–`B8`, cioè la fetta successiva. Il criterio del piano elencava quattro
   verifiche perché copriva dodici passi; le prime tre sono quelle dei passi eseguiti.

Le due prove di qualità: **navigazione da sola tastiera** verificata sul percorso
dell'inquilino — accesso, apertura di una segnalazione, invio — con l'ordine di
tabulazione che parte dal salto al contenuto, attraversa le azioni nell'ordine in cui
compaiono e finisce sul thread, senza trappole e senza `tabindex` positivi; **375 px**
verificata su tutte le schermate nuove, con `documentElement.scrollWidth` uguale a
`clientWidth` — cioè nessuno scorrimento orizzontale della pagina — dopo la correzione
di `PageHeader`.

#### Numeri

Il bundle del client è passato da **614 kB a 648** (185 → 193 compressi): sei
schermate nuove in un pezzo solo, e il debito della divisione per rotta è `B10`, che è
oltre il taglio. È la terza fetta che lo rimanda, e la terza volta è annotata.

**424 test Rust** (erano 423: uno di contratto in più, `GET /organizations` con le tre
condizioni che la decisione 1 dichiarava) e **48 del client** (erano 37: sei sulla
dichiarazione di pagamento, cinque sul guscio). Suite completa verde, nessun test
dichiarato `SALTATO`.

Il test sull'idempotenza della dichiarazione è stato **verificato contro il difetto**,
come quello della fetta A: sostituendo `intent.current()` con `crypto.randomUUID()`
dentro `mutationFn`, fallisce. È la seconda volta che quella verifica si fa, ed è
diventata la condizione con cui si accetta un test sull'idempotenza.

#### Debiti tecnici

**Nessuno nuovo di rilievo**, e due dei tre già registrati **restano**: la divisione
del bundle per rotta e il coordinamento del refresh fra schede erano `B10`, che è oltre
il taglio.

Due cose da sapere, trovate eseguendo:

- **il ramo sul ruolo non è un confine di sicurezza**, ed è scritto tre volte — nel
  codice, nell'ADR e qui — perché è la cosa più facile da fraintendere di questa fetta.
  A negare sono l'API e la policy di riga; il client evita di *offrire* ciò che
  verrebbe rifiutato. Chi in futuro togliesse un controllo dall'API perché «tanto il
  client non mostra il bottone» avrebbe capito il contrario;
- **`PaymentsPage` risponde a due domande diverse sulla stessa rotta.** Per lo staff è
  la coda delle dichiarazioni; per chi abita sono i **propri** movimenti, perché a
  scegliere le righe è `payments_select` e non un ramo sul ruolo. La schermata lo dice
  con titolo e sottotitolo diversi: chiamare «dichiarazioni da verificare» l'elenco di
  chi non può verificare niente sarebbe stata una schermata che dice il falso. Non è in
  navigazione per l'inquilino — l'ha già sull'addebito — ma è raggiungibile per URL, e
  una schermata raggiungibile va progettata.

---

<a id="piano-spese-co-living-fetta-b-bis"></a>

## Piano — Le spese co-living (fetta B-bis)

Pianificata il **7 settembre 2026**, dopo la fetta B e **leggendo il codice che tocca**:
`crates/api/src/routes/expenses.rs` (1179 righe, **quindici** operazioni), i suoi DTO, la vista
`app.v_expense_group_balances` e la funzione `app.can_access_expense_group`. Due delle
decisioni qui sotto sono nate da quella lettura, e sono le due che costano di più se le
si scopre implementando.

**Tre passi**, `B6`–`B8`. Il **punto di taglio è dopo `B8`**, ed è tutta la fetta: le
spese co-living sono una capacità sola, e finisce quando i saldi di un gruppo sommano a
zero con un membro uscito che ha ancora un conto aperto.

`B9` (allegati sugli owner rimasti) e `B10` (bundle diviso per rotta, refresh fra
schede) **escono da questa fetta** e diventano una fetta di manutenzione a sé — vedi
[la fetta C](#piano-fetta-c), che li ha assorbiti come `C3` e `C5`. Non è un rinvio per
stanchezza: sono lavoro di manutenzione e non una capacità, il loro criterio di
completamento non ha niente a che vedere con quello delle spese, e tenerli insieme
rifarebbe l'errore che la fetta B ha dovuto correggere a metà dichiarando un taglio.

### Obiettivo

L'ultimo modulo dell'MVP che ha lo schema, l'API e nessuna interfaccia. È anche il primo
in cui il denaro **non passa dal proprietario**: gli addebiti sono ciò che l'inquilino
deve all'organizzazione, le spese co-living sono ciò che i coinquilini si devono fra
loro. Il proprietario ne è estraneo per costruzione.

Il backend è completo dalla fetta 1: gruppi con il creatore già dentro (migration
`0026`), composizione con uscita e rientro, spese con ripartizione `equal` e `weighted`,
saldi letti dalla vista, regolamenti a due tempi confermati da chi riceve, e chi esce
con un conto aperto che resta nei saldi (migration `0027`). **Questa fetta non tocca il
database e non aggiunge rotte**: è client, e se si scopre di aver bisogno di un endpoint
nuovo è il segnale di aver capito male qualcosa — si rilegga prima questa sezione.

### Che cosa è stato deciso prima di pianificare

Sette scelte. Le prime due sono nate **leggendo il codice**, e sono quelle che il piano
esiste per non far riscoprire.

| | Scelta | Alternativa scartata |
|---|---|---|
| I nomi | il client scrive **sempre** `display_name` all'ingresso | leggerli dai residenti quando servono |
| Uscire dal gruppo | è **irreversibile dal client**, e va detto prima | trattarlo come un'archiviazione qualunque |
| Voce *Spese* | **solo per l'inquilino** | anche per lo staff, che l'API gli concede |
| Come si scopre un debito | il **netto in cima** a *Spese* | un contatore sulla voce di navigazione |
| Chi paga chi | si mostrano i netti, **non si calcola una minimizzazione** | proporre il giro di bonifici più corto |
| Ripartizione | si sceglie **per spesa**, con il default del gruppo proposto | fissarla sul gruppo |
| Punto di taglio | dopo `B8`, le sole spese | fino a `B10`, portandosi dietro la manutenzione |

### Decisioni prese, con il perché

#### 1. I nomi si scrivono all'ingresso, perché dopo non si recuperano

**È la decisione della fetta, ed è nata leggendo i DTO.** Nessuna risposta di questo
modulo porta un nome: `GroupBalanceResponse`, `ExpenseShareResponse`,
`SettlementResponse` e `ExpenseResponse.paid_by_user_id` identificano le persone con il
solo `user_id`. L'unico nome che esiste è `display_name` su
`ExpenseGroupMemberResponse`, ed è **facoltativo** — lo scrive chi chiama
`POST /expense-groups/{id}/members`, o resta assente.

Una schermata dei saldi è fatta di nomi e numeri e di nient'altro. Senza `display_name`
mostrerebbe UUID, che è esattamente la condizione da cui `B0` ha liberato il selettore
delle organizzazioni una fetta fa.

**E non c'è un ripiego.** `GET /properties/{id}/residents` dà `user_id` e `full_name`,
ma copre **chi abita ora**: la persona uscita di casa che sta ancora saldando il suo
conto — cioè precisamente il caso per cui la migration `0027` esiste, e il soggetto
della verifica che chiude questa fetta — non compare più fra i residenti. Nessun altro
endpoint proietta un utente per identificatore, e non deve esistere
([ADR-036](08-decisioni.md#adr-036)).

Ne segue la regola, e vale **senza eccezioni**: ogni volta che il client aggiunge un
membro manda `display_name`, preso dal `full_name` dei residenti; e alla creazione del
gruppo manda il proprio. Il nome così vive nella riga della membership, che non sparisce
mai — `left_at` valorizzato, riga conservata — ed è leggibile anche quando la persona
non abita più lì.

*Alternativa scartata: risolvere i nomi al momento di mostrarli, incrociando i
`user_id` con i residenti dell'immobile.* Non funziona nel caso che conta, funziona in
tutti gli altri, e quindi **si romperebbe solo sui gruppi vecchi con qualcuno uscito** —
il difetto più difficile da notare fra quelli possibili qui. In più `property_id` sul
gruppo è facoltativo: un gruppo creato senza immobile non ha nemmeno un elenco di
residenti da interrogare.

**Che cosa mostrare quando `display_name` manca lo stesso** — un gruppo creato da
`curl`, o da una versione futura che dimentica la regola: il nome dei residenti se
l'identificatore è ancora fra loro, altrimenti un'etichetta neutra dal catalogo
(«Membro del gruppo»), **mai l'UUID**. Un identificatore in faccia a chi legge non è
informazione, è una resa.

#### 2. Uscire da un gruppo lo fa sparire, e va detto **prima**

`app.can_access_expense_group` ammette lo staff, oppure un membro con
**`left_at IS NULL`**. Verificato nella `0014`. Ne segue una cosa che nessun documento
diceva e che l'interfaccia deve dire: **chi esce da un gruppo smette di vederlo**, saldi
propri compresi, anche se ha ancora un conto aperto. Il suo debito continua a comparire
nei saldi **che leggono gli altri** — è la migration `0027` — e non nei suoi.

È corretto rispetto all'isolamento e resta fuori perimetro; ma è anche un'uscita da cui
non si rientra da soli, perché rientrare richiede che **qualcun altro** ti riaggiunga.
La conferma dell'uscita deve quindi dire che cosa accade, con la stessa forma delle
altre conferme del progetto — che spiegano che cosa cambia invece di chiedere «sei
sicuro?».

*Alternativa scartata: trattarla come l'archiviazione di un immobile.* Là la risorsa
resta visibile a chi l'ha archiviata; qui sparisce, e presentare le due cose allo stesso
modo insegnerebbe una regola falsa.

#### 3. La voce *Spese* è solo dell'inquilino

L'API la concederebbe anche allo staff — quasi tutto il modulo è `ensure_member`, e il
proprietario è staff dell'organizzazione — ma le spese co-living sono il denaro che i
coinquilini si devono **fra loro**, e il proprietario ne è estraneo per costruzione. Una
voce che per lui è sempre vuota è il rumore che [ADR-042](08-decisioni.md#adr-042) ha
appena tolto dalla sua navigazione con *Immobili*.

L'inquilino passa quindi a **cinque voci**: *Addebiti*, *Spese*, *Segnalazioni*,
*Contratto*, *Bollette*. Lo staff resta a sette.

**Le schermate restano progettate**, perché restano raggiungibili per URL e perché un
`owner` che abiti in una delle proprie case può essere in un gruppo come chiunque altro:
in quel caso `useIsStaff()` è vero e la voce non c'è, ma `/spese` risponde. È la stessa
scelta di `PaymentsPage` nella fetta B — fuori dalla navigazione di un ruolo, e onesta
per chiunque ci arrivi.

*Alternativa scartata: una scheda dei gruppi dentro l'immobile, per il proprietario.*
`property_id` sul gruppo è **facoltativo**, quindi un gruppo senza immobile non
comparirebbe da nessuna parte per lui: una vista che ne mostra alcuni e tace sugli altri
è peggio di una che non c'è.

#### 4. Il netto in cima a *Spese*, e non un contatore in navigazione

Oggi `/addebiti` risponde a «quanto devo al proprietario». Il debito verso i coinquilini
è un'altra tasca, e nessuna schermata lo annuncia: senza un indicatore si scopre solo
entrando.

L'indicatore è il **proprio netto complessivo**, in cima all'elenco dei gruppi. Costa
`1 + N` richieste — l'elenco dei gruppi, più i saldi di ognuno — e con uno o due gruppi
è niente. Non servono i nomi: il netto è il proprio, e il proprio `user_id` il client ce
l'ha già dalla sessione.

*Alternativa scartata: un contatore sulla voce di navigazione.* Si vedrebbe da ogni
schermata, e per calcolarlo servirebbe la stessa `1 + N` **a ogni caricamento del
guscio**, non solo su *Spese* — perché non esiste una rotta che dia il netto complessivo
di una persona. Un numero utile pagato con N richieste su ogni pagina è il tipo di costo
che si nota quando è tardi.

*Alternativa scartata: portarlo anche su `/addebiti`.* Sono debiti verso persone diverse
con rimedi diversi — uno si paga al proprietario e lo conferma lui, l'altro si paga a un
coinquilino e lo conferma lui — e affiancarli invita a sommarli.

#### 5. Si mostrano i netti, e non si calcola chi paga chi

`GET /expense-groups/{id}/balances` dà il netto **per persona**, i creditori per primi.
Non dice chi debba pagare chi: quella è una minimizzazione dei trasferimenti, e l'API
non la fa.

**Il client non la fa nemmeno.** Non è pigrizia: due client che la calcolassero con
euristiche diverse proporrebbero giri di bonifici diversi sugli stessi saldi, e il
prodotto direbbe due cose. Chi dichiara un regolamento sceglie **a chi** dall'elenco dei
creditori, e l'importo lo propone il minimo fra il proprio debito e il credito di quella
persona — un suggerimento visibile nel campo, non un algoritmo nascosto.

*Alternativa scartata: proporre il giro più corto.* È la funzione che rende un'app di
spese piacevole, ed è anche una decisione di prodotto con più di una risposta ragionevole
(minimizzare i bonifici? pagare prima chi ha anticipato di più?). Va presa quando
qualcuno la chiede, e allora va presa **nell'API**, dove è una sola.

#### 6. La ripartizione si sceglie per spesa

`default_split_method` sul gruppo è **una consuetudine, non una regola**: il DTO lo dice,
e ogni spesa può dichiararne una diversa. Il form della spesa propone quindi il default
del gruppo e lo lascia cambiare, invece di fissarlo alla creazione.

`equal` e `weighted` sono le due ammesse. **`percentage`, `exact` e `by_usage` compaiono
nell'enum e danno `422`**: il menù si scrive a mano con due voci, come i filtri di stato
delle segnalazioni si scrivono a mano per non offrire `resolved`.

#### 7. Il punto di taglio è dopo `B8`

Le spese sono una capacità e finiscono quando i saldi tornano a zero. `B9` e `B10` sono
manutenzione: allegati su owner che già esistono, e due debiti registrati da tre fette.
Il loro criterio di completamento — «un allegato si carica anche da qui», «il bundle è
diviso» — non ha niente in comune con «i saldi sommano a zero», e una fetta con due
criteri scollegati è una fetta che si taglia a metà mentre la si esegue. È successo
nella fetta B ed è stato gestito dichiarando il taglio prima; qui il taglio è dichiarato
non facendo entrare le due cose.

### Decomposizione

#### `B6` — Client: i gruppi, e i nomi che ci entrano dentro

**Livello dati.** `listExpenseGroups`, `getExpenseGroup`, `createExpenseGroup`,
`addGroupMember`, `leaveGroup`, `archiveExpenseGroup` in `@/api/endpoints`, più i tipi
sul filo in `@/api/types` — `ExpenseGroup`, `ExpenseGroupMember`, `ExpenseGroupKind`,
`SplitMethod` — trascritti a mano dai DTO come tutti gli altri.

**Attenzione a una differenza fra elenco e dettaglio**, verificata nel codice:
`GET /expense-groups` restituisce `members: None`, `GET /expense-groups/{id}` restituisce
`members: Some(...)`. L'elenco **non ha i nomi**, quindi non può mostrare chi c'è
dentro: mostra il nome del gruppo, e il resto sta nel dettaglio.

**Schermate.** `/spese` — elenco dei gruppi con il netto complessivo in cima (decisione
4) — e `/spese/:id`, la scheda del gruppo con la composizione. La creazione è una modale
sull'elenco, come le stanze e non come gli immobili: un gruppo ha un nome e poco altro.

**La composizione passa dai residenti** (`listResidents`, che esiste dalla fetta 1), e
il form manda **sempre** `display_name` preso dal `full_name` del residente scelto
(decisione 1). Un residente **senza `user_id`** non è aggiungibile — non ha accettato
l'invito — e va mostrato **non selezionabile con la ragione**, non nascosto: è la terza
volta che questa forma si presenta, dopo `payer_user_id` nella fetta A e `lease_id`
nella fetta B.

**Il gruppo può non avere un immobile.** `property_id` è facoltativo: quando manca non
c'è un elenco di residenti da cui pescare, e il form deve dirlo invece di mostrare un
menù vuoto. Chi crea il gruppo dall'interfaccia lo crea **con** l'immobile, perché è il
caso reale; il ramo senza immobile serve a non rompersi sui gruppi creati altrimenti.

**L'uscita** è `DELETE /expense-groups/{id}/members/{user_id}` e risponde `200` con la
membership aggiornata, non `204`: la riga resta con `left_at`. La conferma dice che il
gruppo sparirà dalla propria vista (decisione 2). Togliere **qualcun altro** richiede lo
staff, e a un membro l'API risponde `403 expense_group.membership_not_yours`: il client
offre l'uscita solo per sé.

**L'archiviazione** chiude il conto — niente più spese né regolamenti — e **non è una
cancellazione**: saldi e spese restano leggibili, e si può archiviare anche con dei conti
aperti. Va scritto nella conferma, perché «archivia» in questo progetto ha già due
significati diversi e questo è il terzo.

Da qui compare la voce *Spese* nella navigazione dell'inquilino (decisione 3).

#### `B7` — Client: le spese e i saldi

**Livello dati.** `listExpenses`, `recordExpense`, `getExpense`, `getGroupBalances`.

**La scheda del gruppo cresce di due sezioni**: le spese, paginate a cursore, e i saldi.

**Le spese.** Il form ha descrizione, importo, categoria, chi ha anticipato (`assente
significa «io»`, e il menù sono i membri attivi), la data (`assente significa «oggi»`, ed
è una data e non un istante, quindi nessun fuso) e il metodo di ripartizione, proposto
dal default del gruppo (decisione 6). Il peso ha **tre** decimali mentre gli importi ne
hanno due: un quarto decimale su `weight` dà `422` sul campo, e la doppia si esprime con
`"2"`.

**Chi ha anticipato deve essere un membro attivo**, e l'API risponde `409
expense.payer_not_in_group` a chi non lo è: il menù si costruisce dai membri con
`left_at` nullo, e chi è uscito non ci compare — pur restando nei saldi.

**Il dettaglio di una spesa porta le quote**, e la loro somma è **esattamente**
l'importo: lo garantisce il trigger `expense_shares_balanced`. Il client le mostra e non
le ricalcola. `share_weight` è **nullo su una divisione in parti uguali**, dove il peso
non ha partecipato al calcolo — mostrarlo come `1` sarebbe inventare un dato.

**I saldi.** `net_balance` positivo è **in credito**, negativo è in debito, e la somma
del gruppo fa zero. Il client la verifica **con `@/lib/decimal`** e non con l'aritmetica
nativa, che produrrebbe uno zero che a volte non è zero. I saldi arrivano già ordinati —
i creditori per primi — e il client non li riordina.

**Comprendono chi è uscito con un conto aperto**, e quelle righe vanno etichettate come
tali: un nome nei saldi senza spiegazione, che non è fra i membri attivi, è la domanda
che chi guarda si farà per primo.

#### `B8` — Client: i regolamenti a due tempi ← *punto di taglio*

**Livello dati.** `listSettlements`, `declareSettlement`, `confirmSettlement`,
`rejectSettlement`, `cancelSettlement`.

**Quattro stati e tre uscite da `pending`**, che è l'unico da cui si esce: `confirmed`,
`rejected`, `cancelled`. È la differenza con i pagamenti, dove le uscite sono due.

**Chi dichiara è sempre chi chiama**: `DeclareSettlementRequest` **non ha
`from_user_id`**, perché `settlements_insert` esige
`from_user_id = current_user_id()` a chi non è staff. Il corpo ha `to_user_id`, che si
sceglie dall'elenco dei creditori (decisione 5).

**A confermare e a rifiutare è chi riceve**, non lo staff e non chi ha dichiarato: è
l'unico che sa se il bonifico è arrivato. A chiunque altro l'API risponde `403
settlement.not_addressed_to_you`. **A ritirare è chi ha dichiarato**, e a chi riceve
l'API risponde `403 settlement.not_declared_by_you`. Il client instrada sui codici e
mostra i tre casi come tre cose diverse.

**Il ritiro esiste qui e non fra i pagamenti**, e l'asimmetria va spiegata al contrario
rispetto alla fetta B: là il form dice «una volta inviata non la puoi ritirare» perché
`payments_update` è staff soltanto; qui `settlements_update` ammette `from_user_id`,
quindi il ritiro c'è. Chi ha letto l'altra schermata si aspetta il contrario.

**Confermare non ricalcola niente**, a differenza di `ConfirmPayment`: il saldo del
gruppo non è memorizzato da nessuna parte, si legge dalla vista. Il client invalida la
chiave dei saldi e basta — chi arriva dal modulo dei pagamenti cerca un
`recalculate_*` che qui non serve e non deve esistere.

**Nessuna chiave di idempotenza**: `POST /expense-groups/{id}/settlements` non la
richiede, a differenza delle due rotte che creano un pagamento. Non è una svista
dell'API ed è da non replicare per simmetria.

### Trappole

Le prime due sono state trovate leggendo il codice per scrivere questo piano; le altre
erano già note e sono qui perché il piano che le ha prodotte è archiviato e nessuno lo
aprirà mentre esegue.

**Nessuna risposta di questo modulo porta un nome.** Saldi, quote, spese e regolamenti
identificano le persone con il solo `user_id`, e `display_name` sulla membership è
facoltativo. Chi non lo scrive all'ingresso costruisce una schermata di saldi fatta di
UUID, e per la persona uscita di casa **non c'è modo di recuperarlo**. Vedi la
decisione 1.

**Chi esce da un gruppo smette di vederlo.** `can_access_expense_group` esige
`left_at IS NULL`. Il suo debito resta nei saldi degli altri e sparisce dai suoi, e per
rientrare serve che qualcun altro lo riaggiunga. Va detto nella conferma.

**Un residente senza `user_id` non è aggiungibile a un gruppo.** Non ha accettato
l'invito, e la policy confronterebbe un identificatore che non esiste. Va mostrato come
**non selezionabile con la ragione**, non nascosto: nasconderlo farebbe cercare una
persona che sullo schermo c'è. Terza occorrenza della stessa forma —
[ADR-040](08-decisioni.md#adr-040) per la prima.

**Tre metodi di ripartizione su cinque danno `422`.** `percentage`, `exact` e `by_usage`
compaiono nella specifica e sono rifiutati.

**Il peso ha tre decimali, gli importi due.** Un quarto decimale su `weight` dà `422` sul
campo. La doppia si esprime con `"2"`.

**La somma dei `net_balance` fa zero, e va verificata con il modulo decimale.** Sommarli
con l'aritmetica nativa produrrebbe uno zero che a volte non è zero. `parseFloat` è
vietato.

**Confermare un regolamento non ricalcola niente**, a differenza di `ConfirmPayment`.

**Il ritiro esiste fra i regolamenti e non fra i pagamenti.** Le due cose si somigliano
e non sono la stessa, e la fetta B ha già scritto l'asimmetria nell'altro verso.

**`GET /expense-groups` non porta i membri**, `GET /expense-groups/{id}` sì. Un elenco
che provi a mostrare chi c'è dentro mostrerà `undefined`.

**Aggiungere a un gruppo qualcuno che non abita lì è possibile ed è voluto.** L'API non
verifica la residenza, perché `property_id` è facoltativo e una regola che si aggira
omettendo un campo facoltativo si legge come una garanzia senza esserlo. Il client non
deve reintrodurre quel controllo: chi ha lasciato la casa il mese scorso e sta ancora
saldando è un caso legittimo.

**Le azioni di una schermata avvolgono, e `PageHeader` ora lo sa fare.** La fetta B ha
corretto un `shrink-0` che a 375 px faceva scorrere la pagina in orizzontale. Chi
aggiunge una schermata con molte azioni non deve reintrodurlo.

**Il bundle non è ancora diviso per rotta.** Sei schermate in più lo faranno crescere
ancora, ed è `B10`, che è fuori da questa fetta. Va annotato nell'esito, non risolto qui.

### Come si saprà che è finita

`./ops/ci/test-integration.sh` verde e il job del client verde sono il minimo, non il
criterio — e la suite Rust **non deve cambiare di un test**, perché questa fetta non
tocca il backend: se cambia, qualcosa è stato capito male.

La verifica che conta è la quarta del piano della fetta B, l'unica rimasta, e va fatta
**a mano e con due account** perché è un fatto fra due persone e con un account solo non
si distingue chi deve da chi riceve:

**I saldi di un gruppo sommano a zero, con un membro uscito che ha ancora un conto
aperto.** Il giro completo: due coinquilini creano un gruppo, uno anticipa una spesa
divisa in parti uguali, il secondo esce dal gruppo **senza saldare**, e il suo debito
continua a comparire nei saldi che legge il primo — con il suo **nome**, non con il suo
identificatore, che è la decisione 1 vista dal lato di chi la subirebbe. Poi il primo
verifica che la somma dei netti sia esattamente zero. È il caso per cui la migration
`0027` esiste, e nessuno lo ha ancora visto da un'interfaccia.

Più il giro del regolamento, che è la ragione per cui `B8` sta dentro il taglio: chi
deve dichiara di aver pagato, **il saldo non si muove**, chi riceve conferma, e allora
sì. E il ritiro, che qui esiste.

Più le due prove di qualità che il criterio «prodotto reale» rende obbligatorie:
**navigazione da sola tastiera** su un percorso completo, e **375 px** sulle schermate
nuove ([ADR-039](08-decisioni.md#adr-039)).

### Esito — che cosa l'esecuzione ha confermato, cambiato e trovato

**Eseguita il 7 settembre 2026**, per intero e fino al punto di taglio dichiarato:
`B6`–`B8`. `B9` e `B10` non sono entrati, ed è la decisione 7 applicata: restano in
[09 — Stato e roadmap](09-stato-e-roadmap.md) come la fetta di manutenzione.

**Il backend non è stato toccato**, come il piano prevedeva: nessuna migration, nessuna
rotta, nessun DTO. La suite Rust è rimasta a **424 test esatti**, che era il modo
dichiarato di accorgersi di aver capito male qualcosa.

#### Le due decisioni nate leggendo il codice erano le due che contavano

**La decisione 1 — il nome si scrive all'ingresso — è la fetta.** Non è stata rivista, ed
è diventata [ADR-043](08-decisioni.md#adr-043) perché vale per chiunque tocchi quel
modulo in futuro e non solo per queste schermate. Verificarla ha richiesto di costruire
il caso apposta, e vale la pena di riportarlo: due coinquilini, un gruppo, una bolletta
anticipata da una e divisa in parti uguali; poi il secondo esce dal gruppo senza saldare;
poi **il proprietario disdice il suo contratto**, che è ciò che lo toglie dai residenti
della casa. Da quel momento non esiste, in tutta l'API, nessuna risposta che leghi il suo
identificatore al suo nome — tranne la riga della sua membership. La schermata dei saldi
continua a dire «Marco Neri», e i netti continuano a sommare a zero.

Senza la regola avrebbe detto `01a07c16-0044-70c1-9d96-beab10de6616`, e **solo lì**: in
ogni altra schermata il ripiego dai residenti avrebbe funzionato. È il difetto che il
piano prevedeva — «si romperebbe solo sui gruppi vecchi con qualcuno uscito» — visto per
davvero.

**La decisione 2 — uscire è irreversibile dal client — si è confermata nel modo più
diretto.** Chi esce viene rimandato all'elenco, e da lì il gruppo non c'è più: nemmeno i
propri saldi. La conferma lo dice prima, ed è servita: il gesto è a un clic da una
schermata che mostra un debito aperto, e chi lo compie senza saperlo perderebbe di vista
proprio il conto che stava guardando.

#### Che cosa è cambiato rispetto al piano

**Tre cose, tutte trovate guardando le schermate e nessuna nel codice.**

1. **Le quote di una spesa non sono una tabella, sono un elenco.** `TableFrame` tiene
   `min-w-[36rem]` perché regge gli elenchi del proprietario; dentro una modale larga la
   metà, quella larghezza minima spinge fuori **la colonna degli importi** — cioè la sola
   ragione per cui si apre quel dettaglio. Con due fatti per riga un `<ul>` li tiene
   accanto a qualunque larghezza. Stessa correzione, e per la stessa ragione, sull'elenco
   dei gruppi: là fuori usciva **il saldo**, sulla schermata che esiste per mostrarlo;
2. **il netto complessivo si mostra senza il segno.** «Devi −61,40 €» è un doppio
   negativo, e chi lo legge si ferma a chiedersi in che verso vada. La parola porta il
   verso, il numero porta la quantità; nelle tabelle il segno resta, perché lì è la
   colonna a dire che cosa significa e non una frase. `NetAmount` prende quindi un
   `display` separato dal valore: il **tono** resta quello del valore vero;
3. **i rimborsi hanno due colonne di data e non una.** È la stessa lezione della fetta A
   sui pagamenti, ritrovata da capo: finché un rimborso è in attesa `settled_at` è nullo,
   quindi una colonna sola sarebbe vuota **proprio nel caso comune**. Si mostrano
   *Dichiarato il* (`created_at`, che c'è sempre) e *Pagato il* (`settled_at`).

E una correzione al piano: il piano diceva che `settled_at` assente significa «adesso».
**Non è così, ed è meglio.** `Settlement::confirm` lo fa ricadere sul momento della
**conferma**, non su quello della dichiarazione — «confermare significa dire che il
denaro si è mosso» — e una data dichiarata non viene riscritta. Il testo del form lo dice
com'è.

#### La verifica che chiude la fetta, fatta con due account

Il percorso nativo — `cargo run -p propflow-api` con le quattro `PROPFLOW__STORAGE__*`
sulla riga di comando, `npm run dev` — e due inquilini della stessa casa, ognuno con il
proprio contratto per stanza. **Il co-living per stanza è l'unico modo di avere due
residenti**: due contratti sull'immobile intero si sovrappongono e il secondo non si
attiva, il che è anche la conferma pratica del perché la migration `0030` esisteva.

Il giro completo, e ogni passaggio ha detto la verità:

- **la spesa e le quote**: 122,80 € anticipati da Giulia, divisi in parti uguali;
  61,40 € a testa, calcolati dal server e mostrati senza ricalcolarli. I saldi diventano
  `+61,40` e `−61,40`, e la riga sotto la tabella dice che la somma fa zero;
- **il rimborso non muove niente finché non lo conferma chi riceve**: Marco dichiara
  61,40 €, e i saldi restano dov'erano. La riga è *In attesa* e per Giulia porta «Aspetta
  la tua conferma»;
- **il ritiro esiste, e qui sì**: Marco la ritira, la riga passa a *Ritirato*, i saldi non
  si muovono — non li aveva mai mossi. È l'asimmetria con le dichiarazioni di pagamento,
  vista dal lato in cui il bottone c'è;
- **la conferma muove i saldi**: Marco dichiara 30,00 €, Giulia conferma, e i netti
  diventano `+31,40` e `−31,40`. *Pagato il* si riempie con il momento della conferma,
  come il dominio prescrive;
- **l'uscita senza saldare**: Marco esce con 31,40 € di debito. Viene rimandato a
  `/spese`, dove non ha più nessun gruppo. Giulia continua a vedere «Marco Neri», la sua
  riga marcata *uscito, conto ancora aperto*, e la somma dei netti a zero;
- **l'archiviazione**: chiude le scritture e non la lettura. Spariscono i due bottoni che
  otterrebbero `409 expense_group.archived`; **resta quello che aggiunge un membro**, ed è
  voluto — riportare dentro chi è uscito con un debito è il modo in cui quella persona
  torna a vederlo, e su un conto chiuso è esattamente il caso che capita.

Più le due prove di qualità: **375 px** su entrambe le schermate nuove, dove la pagina
non scorre in orizzontale (le tabelle scorrono nel loro contenitore) e le azioni
dell'intestazione avvolgono senza reintrodurre lo `shrink-0` che la fetta B ha tolto; e
la **tastiera**, dove l'ordine di tabulazione va dal salto al contenuto alle voci alle
azioni, il fuoco resta dentro una modale aperta, `Escape` la chiude e lo restituisce al
bottone che l'ha aperta.

#### Una cosa vista per caso, e che vale come misura

**Il debito sul refresh non coordinato fra schede si è manifestato da sé**, e non
cercandolo: due schede aperte sulla stessa origine, entrambe hanno tentato il rinnovo al
caricamento, e l'API ha revocato la catena — `riuso di un refresh token già ruotato:
revocate tutte le sessioni` nei log, e la sessione fuori senza che nessuno avesse fatto
niente. Era registrato come «sintomo raro»; **non è così raro**, ed è la prima volta che
lo si vede accadere invece di dedurlo. Resta `B10`, con una nota in più su quanto costa.

#### Numeri

**424 test Rust**, esattamente come prima: questa fetta non ha toccato il backend, e il
numero è la prova. **61 del client**, erano 48: tredici nuovi, e ognuno copre una cosa che
si romperebbe in silenzio — sei sulla risoluzione dei nomi (`display_name`, il ripiego dai
residenti, l'etichetta neutra, **il caso di chi non è più residente**), quattro sulla
somma dei netti verificata sugli interi — fra cui una bolletta di 122,80 € divisa fra
quattro, dove l'aritmetica nativa **non** dà zero — e tre sulla scheda del gruppo, che
rendono la verifica di chiusura ripetibile in CI: il nome di chi è uscito, la nota che
spiega perché è nei saldi, e la riga che dice che i conti tornano.

Il bundle è passato da **648 kB a 690 kB** (da 193 a 202 compressi). Sei schermate in più
sono sei schermate in più, ed è `B10`: annotato, non risolto, com'era previsto.

#### Debiti tecnici

**Nessuno nuovo.** Tre note:

- il `.env` più vecchio dello storage ha morso per la **terza** volta. Il rimedio resta
  quello registrato — un messaggio che nomini `.env.example` — e la terza occorrenza dice
  che vale la pena di farlo;
- **il netto complessivo costa `1 + N` richieste**, ed è dichiarato: l'elenco dei gruppi,
  più i saldi di ognuno. Con uno o due gruppi è niente; il costo è nel numero dei gruppi,
  non nel numero degli utenti, quindi non cresce con l'adozione. Il totale si mostra
  **solo quando tutte le letture sono arrivate**: un netto calcolato su una parte dei
  gruppi non è un'approssimazione, è un numero sbagliato che sembra giusto;
- **`can_access_expense_group` rende invisibile il proprio debito a chi esce**, e resta
  fuori perimetro. È corretto rispetto all'isolamento e il client lo dice prima; farglielo
  vedere sarebbe un allargamento della visibilità, cioè una decisione di prodotto con la
  sua policy.

<a id="piano-fetta-c"></a>

## Piano — Chiudere i buchi che un collaudo incontrerebbe (fetta C)

Pianificata il **7 settembre 2026**, dopo la B-bis e **facendo l'audit che nessuna fetta
aveva fatto**: quali operazioni dell'API non si raggiungono da nessuna schermata. Ne sono
uscite cinque, e **una di esse rompe il giro del prodotto**: nessuna schermata chiama
`POST /auth/accept-invitation`, quindi un invitato non può entrare senza `curl`.

Assorbe `B9` e `B10` della fetta B — restano nominati così dove l'archivio vi rimanda —
e li mette accanto alle altre cose minime, perché hanno lo stesso criterio: **che il
prodotto si possa percorrere per intero da due persone con un browser**, senza che chi
prova debba conoscere l'API. È la condizione per la fase successiva, non un abbellimento:
finché il giro si chiude solo con `curl`, «funziona» è un'affermazione su cui nessuno può
essere smentito.

**Sei passi**, `C1`–`C6`, e il **punto di taglio è dopo `C6`**. Il criterio è il
[collaudo completo](#il-collaudo-completo-che-chiude-la-fetta), che si fa **dopo**, si fa
**a mano** e lo fa **chi non ha scritto il codice**.

### L'audit, che è il fatto da cui la fetta nasce

Cinque funzioni del livello dati esistono e **nessuna schermata le chiama**, più una rotta
che il livello dati non ha affatto:

| Cosa | Conseguenza | Dove va |
|---|---|---|
| `acceptInvitation` | **un invitato non può accettare**: l'unico modo è `curl`. Il proprietario genera il codice e lo consegna, e lì il giro si ferma | `C1` |
| `revokeInvitation` | un invito mandato all'indirizzo sbagliato non si ritira | `C1` |
| `updateRoom`, `archiveRoom` | una stanza si crea e poi **non si corregge**: un nome sbagliato resta, e il nome è unico per immobile | `C4` |
| `DELETE /documents/{id}` — **non c'è nemmeno la funzione** | un allegato caricato per sbaglio non si toglie | `C3` |
| `getPayment` | nessuna, **ed è voluto**: vedi sotto | in nessun passo |

**`getPayment` resta non chiamata apposta.** Il dettaglio di un pagamento non porta niente
che la riga del suo elenco non abbia già, e una schermata in più che ripete gli stessi sei
campi è lavoro che si paga due volte — a scriverla e a tenerla allineata. Sta scritto qui
perché la prossima persona che rifà questo audit la troverà di nuovo, e senza questa riga
la «completerebbe».

**L'audit va rifatto**, ed è la parte riusabile di questa fetta: si fa confrontando le
funzioni esportate da `@/api/endpoints` con chi le importa. Trovare una funzione mai
chiamata non è di per sé un difetto — `getPayment` lo dimostra — ma è **sempre** una
domanda da porsi, e per due fette non se l'era posta nessuno.

### Che cosa è stato deciso prima di pianificare

| | Scelta | Alternativa scartata |
|---|---|---|
| Il codice d'invito | si **incolla in un campo**, e non sta nell'URL | un link `/invito/<token>` da mandare |
| Dopo l'accettazione | l'organizzazione appena accettata **diventa quella attiva** | lasciare che il default la scelga come sempre |
| Il refresh fra schede | si **serializza**, non si condivide il token | trasmettere la sessione con `BroadcastChannel` |
| L'ordine dei passi | il buco dell'invito **per primo**, il bundle per ultimo | il più facile per primo |
| Il collaudo | **a mano, da chi non ha scritto il codice**, con un percorso scritto | una suite end-to-end automatica |
| Il perimetro | sei passi, e la fase «utenti veri» **resta fuori** | portarsi dietro le notifiche, che sono lì accanto |

### Decisioni prese, con il perché

#### 1. Il codice d'invito si incolla, e non viaggia in un URL

`accept_token` è **una credenziale**: chi ce l'ha entra in un'organizzazione. Un link
`/invito/<token>` sarebbe più comodo e lo scriverebbe in tre posti che nessuno ripulisce —
la cronologia del browser, il `Referer` verso qualunque risorsa esterna, e i log di
qualunque proxy davanti al client. Il catalogo lo chiama già «codice», e la modale che lo
consegna dice che vale sette giorni e si vede una volta sola: la schermata che lo riceve
resta coerente con quella, ed è **un campo in cui incollare**.

*Alternativa scartata: il link con il token nel percorso.* Comprerebbe un clic e
venderebbe la credenziale a tre archivi che nessuno considera parte del prodotto.

**Ne segue una conseguenza da progettare, non da subire**: accettare richiede una
sessione, perché la rotta è autenticata. L'invitato deve quindi **prima avere un
account**. La schermata lo dice e lo indirizza, invece di rispondere `401` a chi ha
appena incollato un codice giusto.

#### 2. Accettare sceglie l'organizzazione, ed è l'unico momento in cui si può sapere quale

C'è un debito registrato da due fette: `pick_default_membership` preferisce il ruolo più
alto — owner, poi manager, poi tenant — e la registrazione crea sempre un'organizzazione
personale di cui si è owner. Un inquilino che si registra da sé atterra quindi **sempre**
nella propria organizzazione vuota, e deve cambiare a mano. Lo si è visto durante il
collaudo della B-bis: la prima schermata di un inquilino appena registrato è «Comincia
registrando il tuo primo immobile».

**Accettare un invito è il momento in cui l'ambiguità non c'è**: quella persona ha appena
detto in quale organizzazione vuole entrare. La risposta porta `organization_id` e `role`,
e il client chiama `selectOrganization` con quelli.

*Alternativa scartata: cambiare `pick_default_membership` nel backend.* Vorrebbe dire
decidere **per tutti** quale appartenenza vinca, e il caso legittimo esiste — chi affitta
una casa e ne abita un'altra è owner della propria e tenant di un'altra, e nessuna delle
due è ovvia. La scelta del client è locale, è reversibile dal selettore, e non richiede
una migration.

**Il debito non si chiude del tutto**, e va detto: chi entra da un browser nuovo, mesi
dopo, riparte dal default di sempre. Si chiude a metà, e la metà che si chiude è la prima
volta — che è quella che fa dire «non funziona».

#### 3. Il refresh fra schede si **serializza**, e il token non lascia la memoria

Il difetto è documentato da tre fette e nella B-bis **è accaduto**: due schede aperte, il
rinnovo tentato da entrambe, `riuso di un refresh token già ruotato: revocate tutte le
sessioni`, e la sessione fuori senza che nessuno avesse fatto niente. Con due account e
due finestre — cioè il collaudo che questa fetta prepara — non è raro: è certo.

La via che viene in mente per prima è **trasmettere la sessione**: la scheda che vince
rinnova e manda l'access token alle altre con `BroadcastChannel`. Funziona, e **rompe una
proprietà che il progetto ha scelto apposta**: l'access token vive in memoria e non è
leggibile da uno script che arrivi dopo il caricamento della pagina. Metterlo su un canale
o, peggio, in `localStorage` come ripiego, lo espone esattamente a ciò da cui era stato
tenuto fuori.

**Non serve condividerlo.** Ciò che le schede condividono è **il cookie**, e l'API ruota a
ogni uso: il riuso lo rileva solo su *due usi dello stesso token*. Se la seconda scheda
**aspetta** che la prima abbia finito e poi rinnova a sua volta, presenta il cookie
**nuovo**, che è una rotazione legittima. Serve quindi una coda, non una trasmissione:
`navigator.locks` — che è esattamente un mutex fra contesti della stessa origine — con
ripiego su un lock in `localStorage` con scadenza, dove **si scrive un marcatore e non un
segreto**.

Costa `N` rinnovi per `N` schede invece di uno. È il prezzo giusto: succede una volta ogni
quindici minuti per scheda, e compra il fatto che il token non esce dalla memoria.

*Alternativa scartata: un `SharedWorker` che tenga la sessione.* È il disegno corretto per
un'applicazione che ne avesse bisogno, e qui sarebbe un terzo contesto da far vivere,
morire e ricollegare per risolvere un problema che un lock risolve in venti righe.

#### 4. L'ordine: il buco dell'invito per primo, il bundle per ultimo

`C1` è l'unico passo senza il quale **il collaudo non può cominciare**: il secondo account
non esiste finché qualcuno non lo può far entrare. `C2` è il secondo perché è ciò che
interrompe il collaudo a metà, e scoprirlo mentre si prova costa il doppio — si perde la
sessione **e** si dubita di ciò che si stava guardando.

`C5`, la divisione del bundle, va per ultimo e non perché sia difficile: **tocca ogni
rotta**, quindi è il passo che più facilmente rompe qualcosa d'altro, ed è quello che si
vuole avere davanti quando tutto il resto è già verde.

#### 5. Il collaudo è a mano, e non una suite automatica

La tentazione è scrivere Playwright e chiamarlo collaudo. Sono due cose diverse, e questa
fetta vuole la prima: **una persona che non ha scritto il codice attraversa il prodotto e
dice se sta in piedi**. Una suite end-to-end verifica che ciò che qualcuno ha già capito
continui a valere; non trova la schermata che risponde a una domanda che nessuno si era
posto, che è precisamente ciò che il collaudo deve trovare adesso.

*Alternativa scartata: Playwright ora.* Va bene **dopo**, quando i percorsi sono stabili e
il valore è la regressione. Prima, congela ipotesi.

#### 6. Il perimetro si ferma qui, e le notifiche restano fuori

Sono lì accanto — sono la fase successiva, e sono già pianificate — e prenderle dentro
cambierebbe il criterio della fetta: «il prodotto si percorre per intero» non ha niente a
che vedere con «un utente vero riceve un'email». Vale la ragione della decisione 7 della
B-bis: due criteri scollegati in una fetta sola sono una fetta che si taglia a metà mentre
la si esegue.

### Decomposizione

#### `C1` — L'invitato entra dall'interfaccia

**È il passo per cui la fetta esiste.** Oggi il proprietario genera il codice, lo
consegna, e lì il prodotto finisce: chi lo riceve non ha una schermata in cui usarlo.

**Schermata `/invito`**, un campo e un bottone. Raggiungibile **con la sessione**, perché
`POST /auth/accept-invitation` è autenticata: chi arriva senza account trova scritto che
deve prima registrarsi, con il link, e che poi torna qui — non un `401`.

**La chiamata va senza `X-Org-Id`**, ed è già così in `acceptInvitation`: è l'operazione
che **crea** l'appartenenza, e dichiarare un'organizzazione di cui non si è ancora membri
fa rispondere `403` a una richiesta legittima. Chi tocca quel codice non deve
«uniformarlo» agli altri.

**Alla riuscita si sceglie l'organizzazione appena accettata** (decisione 2) e si va alla
pagina iniziale del ruolo che la risposta dichiara — che per un inquilino è `/addebiti` e
non la radice.

**I tre codici che questa rotta produce si traducono in tre messaggi diversi**:
`invitation.not_found` (codice sbagliato o scaduto), `invitation.already_accepted` (qualcuno
l'ha già usato, e non è un errore di chi legge), e `auth.required` se la sessione è caduta
mentre si compilava.

**La revoca sta nella modale che ha creato l'invito**, e non altrove: `revokeInvitation`
vuole l'id, e **non esiste una rotta che elenchi gli inviti** — quindi l'unico momento in
cui il client ha quell'id è subito dopo averlo creato. Serve al caso vero — «ho sbagliato
indirizzo, lo rifaccio» — e va offerto lì, dicendo che dopo si potrà solo lasciarlo
scadere. **Non inventare un elenco**: sarebbe una rotta nuova, cioè backend dentro una
fetta di client.

#### `C2` — Il refresh smette di buttare fuori chi ha due schede

`SessionStore.refresh` ha già il singolo volo **dentro** una scheda. Si aggiunge il lock
**fra** schede (decisione 3): chi entra mentre un'altra sta rinnovando aspetta, e poi
rinnova a sua volta con il cookie ruotato.

`navigator.locks.request` con un nome per origine, e ripiego su `localStorage` con un
marcatore e una scadenza — mai il token. Il ripiego serve dove i Web Locks non ci sono, e
deve **scadere**: una scheda chiusa a metà rinnovo non può bloccare le altre per sempre.

**Il test si scrive al contrario**, come quello del singolo volo: due `SessionStore` che
condividono il finto cookie, e la prova che senza il lock il secondo presenta il token già
ruotato. È il modo in cui il singolo volo è stato verificato nella fetta 1, e vale la
stessa condizione — un test che non fallisce quando il codice è sbagliato è un verde che
non prova nulla.

#### `C3` — Gli allegati sugli owner rimasti

Immobile, stanza, contratto, segnalazione, addebito. `ScansCard` di `BillDetailPage` è il
modello, e va estratto in un componente condiviso invece di essere copiato cinque volte.

**`tickets` manca dall'unione degli owner** in `uploadDocument` e `listDocuments`, e la
rotta esiste: va aggiunto, altrimenti il typecheck rifiuta la segnalazione.

**L'archiviazione di un documento non ha nemmeno la funzione**: `DELETE /documents/{id}`.
Il codice che restituisce a chi non può è `org.staff_role_required`, ed è **vero solo a
metà** — la policy ammette lo staff *oppure* chi ha caricato — quindi il messaggio non deve
promettere che serva un ruolo. È un debito registrato, e qui si evita di ripeterlo
all'utente.

**Il controllo della dimensione sta prima dell'invio** (`UPLOAD_LIMIT_BYTES`, 25 MiB):
oltre il limite la risposta è un `413` che non dice quale limite né su quale file.

Vale la trappola registrata: **il link di download non porta l'`Authorization`**, e
funziona perché il browser lo segue nel contesto della scheda.

#### `C4` — Le stanze si correggono e si archiviano

`updateRoom` e `archiveRoom` esistono nel livello dati e non le chiama nessuno: una stanza
si crea e resta com'è. Il nome è **unico per immobile**, quindi un refuso non è cosmetico —
occupa un nome che poi non si può riusare.

Il `PATCH` è parziale con la semantica di sempre: assente = invariato, `null` = azzerato.
L'archiviazione **libera il nome**, e la conferma lo dice — la stringa esiste già nel
catalogo (`t.room.archiveExplain`) ed è stata scritta per questo momento.

#### `C5` — Il bundle diviso per rotta ← *passo che tocca tutto*

`React.lazy` per rotta, con i confini di `Suspense` messi **dove stanno i tre stati**: un
confine senza il suo stato produce una pagina bianca dove prima c'era uno scheletro.

Il bundle è a **690 kB (202 compressi)**: 597 alla fetta 1, 614 alla A, 648 alla B, 690
alla B-bis. Su desktop non si nota, e il proprietario è su desktop; **l'inquilino è su rete
mobile**, e adesso esiste davvero.

**La misura si scrive nell'esito**, prima e dopo, per rotta iniziale. Un «abbiamo diviso il
bundle» senza numeri non è verificabile da chi legge fra un anno.

#### `C6` — Il messaggio che ha morso tre volte

L'unico passo che tocca il backend, ed è di dieci righe: un `.env` copiato prima della
fetta dei documenti non ha le quattro `PROPFLOW__STORAGE__*`, e l'API si ferma con
`missing field access_key_id for key "default.storage"` — che non dice **da dove** debba
arrivare quella variabile né che `.env.example` la porti.

Tre occorrenze in tre fette, sempre allo stesso punto: avviare l'API per il giro a mano.
**Il rimedio non è un default** — uno storage configurato a caso è peggio di un avvio
fallito — ma un messaggio che nomini il file da cui copiare.

### Trappole

Le prime tre vengono dall'audit e dalla lettura del codice; le altre sono note e sono qui
perché il piano che le ha prodotte è archiviato e nessuno lo apre mentre esegue.

**Accettare un invito va fatto senza `X-Org-Id`.** È l'operazione che crea
l'appartenenza: dichiarare l'organizzazione di cui non si è ancora membri fa rispondere
`403` a una richiesta legittima. `acceptInvitation` lo fa già, e va **non toccato**.

**Non esiste una rotta che elenchi gli inviti.** La revoca è possibile solo con l'id che
la creazione ha restituito, cioè nella stessa modale. Costruire un elenco vorrebbe dire
aggiungere una rotta, cioè backend in una fetta di client.

**`getPayment` resta non chiamata apposta**, e non è un pezzo mancante.

**Il token dell'invito è una credenziale**: non nell'URL, non nei log, non conservato. Il
client lo mostra una volta perché il proprietario lo consegni, e lo dimentica.

**Il link di download non porta l'`Authorization`**, e funziona perché il browser lo segue
nel contesto della scheda.

**Il limite del corpo sulle rotte di caricamento è 25 MiB** e va controllato prima:
oltre, la risposta è un `413` che non dice né quale limite né su quale file.

**`org.staff_role_required` sull'archiviazione di un documento è vero a metà**: la policy
ammette anche chi ha caricato. Il messaggio non deve promettere un ruolo.

**Il nome di una stanza è unico per immobile**, e l'archiviazione lo libera.

**Un confine di `Suspense` senza il suo stato è una pagina bianca** dove prima c'era uno
scheletro.

**Il lock fra schede non deve trasmettere il token**, e il ripiego su `localStorage` scrive
un marcatore con una scadenza. Una scheda chiusa a metà rinnovo non può bloccare le altre.

**Le azioni di una schermata avvolgono**, e `PageHeader` lo sa fare dalla fetta B: chi
aggiunge azioni non reintroduca lo `shrink-0`.

<a id="il-collaudo-completo-che-chiude-la-fetta"></a>

### Il collaudo completo, che chiude la fetta

`./ops/ci/test-integration.sh` verde e il job del client verde sono il minimo, non il
criterio. **Il criterio è che una persona attraversi il prodotto per intero da un browser,
con due account, senza mai aprire `curl`.**

Va fatto **dopo `C6`**, sul percorso nativo (`cargo run -p propflow-api`, `npm run dev`) e
in quest'ordine, perché ogni passo prepara il successivo:

1. **il proprietario si registra**, e atterra sul riepilogo vuoto che dice da dove
   cominciare;
2. **registra un immobile**, ci aggiunge **due stanze**, e ne **corregge una** — che è `C4`
   e che oggi non si può;
3. **redige due contratti per stanza**, ci mette le parti, li attiva;
4. **invita i due inquilini**, e consegna i due codici. Ne **revoca uno** e lo rifà, che è
   l'altra metà di `C1`;
5. **i due inquilini si registrano e accettano** — `C1`, il passo senza il quale il
   collaudo si ferma qui — e si ritrovano **nell'organizzazione giusta**, non nella
   propria vuota;
6. **il proprietario registra una bolletta**, ne carica la scansione, ne rivede la lettura
   della macchina se il fornitore è configurato, la valida e la **riaddebita** a un
   contratto;
7. **un inquilino apre una segnalazione** con una **foto allegata** — `C3` — e il
   proprietario la porta per il ciclo: triage, presa in carico, sospensione, ripresa,
   chiusura;
8. **l'inquilino dichiara di aver pagato** un addebito; il proprietario la trova nella coda
   e la conferma; il residuo scende;
9. **i due inquilini creano un gruppo di spesa**, ci mettono una spesa a testa, uno rimborsa
   l'altro, il creditore conferma, e i saldi tornano a zero;
10. **da due finestre contemporaneamente** — il proprietario in una, un inquilino
    nell'altra — si lascia scadere l'access token e si ricarica: nessuno dei due deve
    ritrovarsi fuori. È `C2`, e prima di questa fetta è **certo** che succeda;
11. **a 375 px**, il percorso dell'inquilino per intero.

Ciò che il collaudo trova si scrive nell'esito **anche se non si corregge**: una fetta che
chiude con «tutto bene» dopo un giro come questo è una fetta che il giro non l'ha fatto.

### Che cosa **non** entra, e con quale ragione

- **le notifiche**, che sono la fase successiva e hanno un criterio diverso (decisione 6);
- **una suite end-to-end**, che va scritta quando i percorsi sono stabili (decisione 5);
- **un elenco degli inviti**, che è una rotta nuova;
- **il dettaglio di un pagamento**, che non porta niente in più della sua riga;
- **il resto del debito sull'organizzazione predefinita**: si chiude la prima volta, non
  ogni volta (decisione 2).

<a id="esito-fetta-c"></a>

## Esito — Fetta C, chiudere i buchi che un collaudo incontrerebbe

**Eseguita l'8 settembre 2026**, per intero e fino al punto di taglio dichiarato:
`C1`–`C6`, e **le notifiche sono restate fuori** come la decisione 6 prevedeva.

**Il collaudo in undici punti non è ancora stato percorso**, ed è la ragione per cui
questo esito è **aperto**: il criterio della fetta non è la suite, è che una persona che
non ha scritto il codice attraversi il prodotto con due account e un browser. Ciò che
quel giro troverà si scrive qui sotto, **anche se non si corregge**. Un esito che si
chiudesse con «tutto bene» prima del giro sarebbe un esito che il giro non l'ha fatto.

### Che cosa è entrato

| Passo | Esito |
|---|---|
| `C1` | schermata `/invito`, un campo e un bottone. L'organizzazione appena accettata **diventa quella attiva**, e si atterra sulla pagina iniziale del ruolo. La revoca vive nella modale che ha creato l'invito, e da nessun'altra parte |
| `C2` | lock per origine sopra il singolo volo: `navigator.locks`, con ripiego su un marcatore in `localStorage` che scade. Il token non lascia la memoria |
| `C3` | `AttachmentsCard` estratta da `ScansCard` e usata su **sei** owner. `tickets` aggiunto all'unione, `archiveDocument` scritta |
| `C4` | la stanza si corregge e si archivia, da una modale che è la sua unica superficie |
| `C5` | bundle diviso per rotta. I numeri sono qui sotto |
| `C6` | il messaggio della configurazione nomina `.env.example` e dice come si carica |

**Il backend è stato toccato una volta sola**, in `C6`: `AppConfig::load` restituisce un
`ConfigError` tipizzato invece di un `figment::Error` incapsulato, e `main.rs` perde il
`.context` che ora contraddirebbe. **424 test Rust prima e dopo** — nessuno di essi
asserisce su quel messaggio, ed è coerente: è prosa per un umano al primo avvio.

Il client passa da **61 a 69 test**: quattro sull'accettazione dell'invito e quattro sul
rinnovo fra schede.

### I numeri del bundle, per rotta iniziale

Misurati sulla chiusura transitiva degli import di ogni entry, gzip incluso. Prima era
**un pezzo solo**: ogni rotta pagava tutto.

| Rotta iniziale | Prima | Dopo | |
|---|---|---|---|
| `/` — riepilogo, chi amministra | 690,6 kB · 201,7 gz | **438,7 kB · 141,6 gz** | −36 % · −30 % |
| `/addebiti` — chi abita | 690,6 kB · 201,7 gz | **455,7 kB · 148,3 gz** | −34 % · −26 % |
| `/invito` | 690,6 kB · 201,7 gz | **440,8 kB · 142,8 gz** | −36 % · −29 % |
| `/accesso` — il login | 690,6 kB · 201,7 gz | **543,4 kB · 172,4 gz** | −21 % · −15 % |
| tutte le rotte insieme | 690,6 kB · 201,7 gz | 706,8 kB · 226,2 gz | +2 % · +12 % |

**Due righe vanno lette e non passate sopra.**

La prima: **il totale cresce**. Dividere costa un preambolo per chunk, e chi visitasse
ogni schermata scaricherebbe più di prima. È il baratto giusto — nessuno visita ventuno
schermate in una sessione, e l'inquilino ne visita tre — ma va saputo, perché il giorno
in cui qualcuno misurasse «il bundle» senza dire *di quale rotta* leggerebbe un
peggioramento.

La seconda: **il login è la rotta più pesante**, e non è un caso. `AuthPage` usa `zod`
per gli schemi dei form, e `zod` è un chunk da 107,9 kB (32,5 gz) che nessuna altra rotta
iniziale tira dentro. È **la prima schermata che un utente nuovo vede**, cioè il posto
peggiore in cui avere il picco. Non si tocca in questa fetta — sarebbe riscrivere la
validazione di due form, e il perimetro è chiuso — ma è un debito nuovo, registrato in
`09` con il suo rimedio.

### Che cosa il piano non aveva previsto

**`invitation.not_found` non è il codice che si riceve per un codice sbagliato.** Il piano
ne prevedeva tre — `invitation.not_found`, `invitation.already_accepted`, `auth.required`
— e il primo è sbagliato: l'API risponde **`401 auth.invalid_credentials`** a un token
sconosciuto, scaduto **o intestato a un'altra email**, e li rende indistinguibili apposta,
perché distinguerli sarebbe un oracolo su quali inviti esistono
(`http_contract.rs::an_unknown_token_is_indistinguishable_from_someone_elses`).

Ne segue una cosa che il piano non poteva sapere: nel catalogo quel codice significa
«email o password non corrette», che su questa schermata è una frase su una password che
nessuno ha digitato. Il messaggio è quindi **della schermata e non del codice**, ed è
l'unico posto del client dove un `code` si traduce fuori dal catalogo. C'è un test che lo
tiene, perché è esattamente il genere di cosa che qualcuno «semplifica».

**Lo `state.from` era una promessa che nessuno manteneva.** Il commento in `App.tsx`
diceva che chi arriva su una pagina interna senza sessione «non perde dove stava
andando», e lo `state` veniva scritto — ma dopo il login si andava sempre alla radice,
perché nessuno lo leggeva. È stato scoperto progettando `C1`, dove serve davvero: chi si
registra per accettare un invito deve tornare su `/invito`. Ora lo legge, accettando solo
un percorso interno — uno `state` di `history` lo fabbrica chiunque, e un valore che
cominci con `//` sarebbe una redirezione aperta regalata da tre righe di comodità.

**Il lock fra schede maschera il difetto che il singolo volo evita.** Non era previsto e
si è visto subito: il test della fetta 1 che riproduceva il difetto ha **smesso di
fallire**, perché il lock serializza anche dentro una scheda. Il test è stato corretto per
disattivare entrambi i meccanismi, con la ragione scritta accanto. È il caso da manuale
del criterio del progetto — un test che non fallisce quando il codice è sbagliato è un
verde che non prova nulla — e stavolta si è manifestato al contrario: un verde nuovo che
copriva la dimostrazione di un difetto vecchio.

**La stanza non aveva una superficie propria.** `GET /rooms/{id}` non esiste — l'elenco
passa dall'immobile — quindi `C3` e `C4` si sono uniti: la modale che corregge la stanza
è anche l'unico posto in cui i suoi allegati possono stare. L'alternativa sarebbe stata
una rotta di dettaglio che legge filtrando una lista, cioè una schermata in più per un
oggetto che non la chiede.

### Le trappole, e quali hanno morso

| Trappola | Esito |
|---|---|
| accept-invitation senza `X-Org-Id` | **non ha morso**, ed è merito del piano: `acceptInvitation` era già giusta, e il piano diceva di non toccarla. C'è ora un test che guarda l'header vero |
| nessuna rotta elenca gli inviti | rispettata: la revoca sta nella modale, e nessun elenco è stato inventato |
| `getPayment` resta non chiamata | rispettata. L'audit rifatto a fine fetta la ritrova sola, ed è corretto |
| il limite di 25 MiB prima di inviare | nella scheda condivisa, quindi in tutti e sei i posti invece che in uno |
| `org.staff_role_required` vero a metà | il bottone di archiviazione si offre a chi è staff **oppure** ha caricato, cioè come dice la `USING` di `documents_update`: il modo migliore di non ripetere una mezza verità è non farla incontrare |
| un `Suspense` senza il suo stato | **ha morso in scrittura e non in esecuzione**: il primo confine è stato scritto attorno al guscio, il che avrebbe smontato navigazione e selettore a ogni cambio di rotta. Corretto prima di provarlo, mettendolo dentro `<main>` |
| le azioni avvolgono, niente `shrink-0` | rispettata anche nella scheda nuova |

### Che cosa resta aperto

- **il collaudo in undici punti**, che è il criterio della fetta;
- **`zod` sul login**, debito nuovo di `C5` (sopra);
- **il debito sull'organizzazione predefinita si chiude a metà**, come la decisione 2
  prevedeva: chi entra da un browser nuovo mesi dopo riparte dal default di sempre;
- **il ripiego del lock non ha un confronta-e-scambia**: resta una finestra teorica in cui
  due schede scrivono insieme, e in quel caso si torna al difetto — non a una regressione.
  Vedi [ADR-044](08-decisioni.md#adr-044).

### Il collaudo — da percorrere

*Questa sezione si riempie percorrendolo, ed è ciò che chiude la fetta.*

| # | Punto | Esito |
|---|---|---|
| 1 | il proprietario si registra e atterra sul riepilogo vuoto | — |
| 2 | immobile, due stanze, **una corretta** (`C4`) | — |
| 3 | due contratti per stanza, parti, attivazione | — |
| 4 | due inviti, **uno revocato e rifatto** (`C1`) | — |
| 5 | i due inquilini si registrano e **accettano** (`C1`), nell'organizzazione giusta | — |
| 6 | bolletta, scansione, revisione OCR, validazione, riaddebito | — |
| 7 | segnalazione con **foto allegata** (`C3`), e il ciclo completo | — |
| 8 | dichiarazione di pagamento, conferma, residuo che scende | — |
| 9 | gruppo di spesa, una spesa a testa, rimborso, saldi a zero | — |
| 10 | **due finestre insieme**, token scaduto, nessuno buttato fuori (`C2`) | — |
| 11 | il percorso dell'inquilino **a 375 px** | — |

Ciò che il giro trova va scritto qui **anche se non si corregge**.

---

<a id="piano-sezione-1"></a>

## Piano — Sezione 1: ritocchi visibili

Pianificata l'**8 settembre 2026**. È la prima delle nove sezioni dell'
[ordine di esecuzione](12-da-fare.md#lordine-di-esecuzione), e chiude **sei** voci:
`R5`, `R4`, `R6`, `R10`, `R20` e il bottone del thread.

**È la sola sezione che non richiede di decidere niente**, ed è per questo che va per
prima: nessuna migration, nessuna rotta nuova, nessun cambiamento di dominio. Tutto
`apps/web/`, e il backend non si tocca — **424 test Rust prima e dopo**.

Il criterio di completamento è d'uso e non di suite: il prodotto va provato meglio dopo,
perché è provandolo che stanno uscendo le voci di [12](12-da-fare.md).

### Che cosa è stato deciso prima di pianificare

| | Scelta | Alternativa scartata |
|---|---|---|
| La validazione di accesso e registrazione | le regole native di **react-hook-form** | tenere `zod` e accettare il chunk sulla prima schermata |
| La creazione di una stanza | **tutti** i campi che il `POST` accetta, meno `status` | lasciarla minima e rimandare alla modifica |
| La navigazione | altezza piena e selettore **ancorato in fondo** | solo spostare il blocco, senza toccare l'altezza |
| Il thread | un bottone **Aggiorna** premuto da chi vuole | un `refetchInterval`, o `SSE` |

### Decisioni, con il perché

#### 1. `zod` esce da `AuthPage`, e resta ovunque altro

`/accesso` è la rotta iniziale più pesante — **543,4 kB contro i 438,7 del riepilogo** —
perché `AuthPage` è l'unica entry iniziale che tira `zod`, un chunk da 107,9 kB (32,5
gz). È il posto peggiore in cui avere il picco: chi arriva la prima volta non ha nulla in
cache.

Le regole di quei due form sono **tre**: un'email formalmente valida, dodici caratteri di
password, campi obbligatori. `react-hook-form` le esprime da sé con `register(...,
{ required, pattern, minLength })` — è già una dipendenza, e non se ne aggiunge nessuna.

*Alternativa scartata: tenere `zod`.* Ha un argomento vero — **la coerenza**:
`NewLeasePage`, `NewPropertyPage` e `NewBillPage` usano `zod` con `zodResolver`, e
`AuthPage` diventerà l'unico form scritto in un altro modo. Si accetta, e il commento nel
file deve dirlo, perché altrimenti qualcuno lo «uniformerà» rimettendo il chunk sulla
prima schermata. **Le altre tre restano con `zod`**: sono rotte pigre, quindi il loro
peso non è sulla prima visita.

**La regola del server resta l'autorità**: il controllo a dodici caratteri qui evita un
giro di rete per dire una cosa che si sa già, e non sostituisce niente. È già scritto
così nel file e va conservato.

#### 2. La stanza si crea con tutti i suoi campi, tranne lo stato

`POST /properties/{id}/rooms` accetta **otto** campi: `name`, `status`, `size_sqm`,
`floor`, `has_private_bathroom`, `is_furnished`, `monthly_rent`,
`utility_share_percent`. Il form ne offre **quattro**, e la modale di modifica cinque:
chi crea non ha modo di sapere che il resto esiste.

Entrano `floor`, `is_furnished` e `utility_share_percent`. Quest'ultimo con il suo
significato scritto — *assente vale «in parti uguali»* — perché un campo numerico vuoto
in una schermata di co-living è ambiguo fra «zero» e «non specificato».

**`status` resta fuori, ed è una scelta e non una dimenticanza.** Che stato abbia una
stanza è la domanda di [`R11`](12-da-fare.md#lelenco), che è la sezione 4: oggi
`Occupied` non lo scrive nessuno, e il dominio dichiara una transizione che non esiste.
Offrire una tendina adesso **anticiperebbe quella decisione** nel posto sbagliato — e se
la sezione 4 concludesse che lo stato va derivato dai contratti, quella tendina andrebbe
tolta subito dopo averla aggiunta.

**Va detto che si può correggere dopo**, ed è metà della voce: il nome è unico per
immobile, quindi «tanto lo sistemo dopo» è esattamente la frase che serve. La stringa non
esiste nel catalogo e va aggiunta.

#### 3. La navigazione: il problema non è dove sta il blocco, è l'altezza

Il sintomo è che su alcune schermate la barra laterale «si vede solo a metà». La causa è
in `shell.tsx` e **non è il blocco dell'organizzazione**:

```
<div className="min-h-screen">                    ← il vincolo d'altezza è QUI
  <div className="... flex ... lg:flex-row">      ← e questo ha altezza automatica
    <nav className="shrink-0 ... lg:w-60">
```

`min-height` su un genitore **non allunga** un figlio di blocco: il contenitore flex è
alto quanto il suo contenuto, e la barra — che è `stretch` dentro di esso — finisce dove
finisce il contenuto. Su una schermata con poche righe si ferma a metà schermo, e sembra
tagliata.

Quindi servono **due** cose, e farne una sola non basta: il contenitore flex deve essere
alto almeno quanto la finestra, e dentro la barra il blocco del selettore va spinto in
fondo (`mt-auto` in una colonna flex). Spostare il blocco senza l'altezza lo lascerebbe
attaccato all'elenco come adesso.

**Solo da `lg` in su.** Sotto quella soglia la navigazione è un menù a scomparsa e il
selettore sta già nella barra superiore: un `mt-auto` incondizionato spingerebbe in fondo
qualcosa che lì non deve stare.

L'elenco delle voci va reso scorrevole a sé (`overflow-y-auto`), altrimenti su una
finestra bassa il selettore ancorato in fondo spinge fuori le ultime voci — che è lo
stesso difetto, spostato.

#### 4. «Prendi in carico» sparisce solo a chi ce l'ha

`assignable = open || triaged || assigned`, e `assigned` è nell'elenco **apposta**: il
bottone serve anche a **riassegnare** a qualcun altro. Non si toglie quindi lo stato dalla
condizione — si aggiunge che chi guarda non sia già l'assegnatario.

*Alternativa scartata: nasconderlo su `assigned` in generale.* Toglierebbe la
riassegnazione, che è una funzione vera anche se oggi in organizzazione c'è un solo staff.

#### 5. Il thread si aggiorna con un bottone

Deciso: né tempo reale né intervallo automatico. Le ragioni per esteso, comprese le due
vie scartate e il motivo di sicurezza che esclude `SSE` oggi, stanno
[in fondo a questo piano](#thread-aggiorna) — sono state scritte in
[12](12-da-fare.md) quando la decisione è stata presa, e si sono spostate qui con la voce
che chiudevano.

Qui basta la parte esecutiva: il thread ha già una chiave propria
(`keys.ticketMessages`), quindi il bottone è la sua `refetch`, e va **disabilitato mentre
rilegge** — un bottone che non dà segno di aver fatto qualcosa viene premuto tre volte.

### Decomposizione

I passi sono **indipendenti** e si possono eseguire e verificare uno per uno. L'ordine
suggerito mette per primo ciò che si vede subito.

##### `S1` — Mostrare la password

`AuthPage.tsx`, i due campi `type="password"`. Un bottone dentro il campo che alterna
`password` e `text`.

Va **raggiungibile da tastiera** e deve dire che cosa fa: `aria-label` che cambia con lo
stato, e `aria-pressed`. Le due etichette vanno nel catalogo, non nel JSX. Il campo non
deve perdere il focus quando si preme.

##### `S2` — `zod` fuori da `AuthPage`

Sostituire `zodResolver` con le regole native di `react-hook-form` sui due form, togliere
gli import di `zod` **da quel file soltanto**, e lasciare un commento che dica perché
questo form è diverso dagli altri tre.

**La misura si rifà e si scrive**: la rotta `/accesso` va rimisurata sulla chiusura
transitiva degli import, come nella fetta C, e il numero va nell'esito. Senza numero non
si sa se è servito.

##### `S3` — La navigazione a tutta altezza, con il selettore in fondo

`shell.tsx`. Portare il vincolo d'altezza sul contenitore flex, rendere la barra una
colonna flex da `lg` in su, spingere in fondo il blocco del selettore, e rendere
scorrevole l'elenco.

**Da provare a tre altezze**: una schermata con poche righe — dove il difetto si vede —
una lunga, e una finestra bassa, dove si verifica che il selettore non spinga fuori le
voci. E a 375 px, dove non deve cambiare niente.

##### `S4` — La stanza si crea con tutti i campi

`NewRoomDialog` in `PropertyDetailPage.tsx`: aggiungere `floor`, `is_furnished` e
`utility_share_percent`, con il suo significato. Aggiungere la frase che dice che si può
correggere dopo, e la stringa nel catalogo.

**I due form vanno allineati**, non solo ampliati: `NewRoomDialog` e `RoomDialog`
chiedono le stesse cose, e sono già due elenchi di campi che divergono. Se diventano tre
divergeranno di nuovo — vale la pena estrarre i campi condivisi in un componente solo, che
è anche ciò che impedisce alla prossima aggiunta di finire in uno solo dei due.

##### `S5` — «Prendi in carico» sparisce a chi ce l'ha

`TicketActions.tsx`: alla condizione si aggiunge che `ticket.assigned_to` non sia chi
guarda. Serve l'identificatore dalla sessione, che il file già può leggere.

##### `S6` — Il bottone «Aggiorna» nel thread

`TicketDetailPage.tsx`, nella `ThreadCard`. Un bottone nell'intestazione del riquadro che
chiama la `refetch` della query paginata, disabilitato mentre è in volo. Stringa nel
catalogo.

### Trappole

**Il catalogo, sempre.** Nessuna stringa visibile dentro un componente: le nuove vanno in
`i18n/it.ts`. È la regola che una sezione di soli ritocchi viola più facilmente, perché
ogni singola stringa sembra troppo piccola per meritare una chiave.

**`min-height` su un genitore non allunga un figlio di blocco.** È la causa di `R6`, ed è
la cosa che si sbaglia riprovandoci: spostare il blocco senza toccare l'altezza non
risolve niente.

**`mt-auto` va condizionato a `lg`.** Sotto, la navigazione è un menù a scomparsa e non
una colonna alta quanto lo schermo.

**Il bottone della password non deve inviare il form.** Dentro un `<form>` il default di
`<button>` è `submit`: `Button` del progetto mette già `type="button"` da sé, e questo va
**conservato** se si scrive un elemento a mano invece di usarlo.

**`assigned` non esce dalla condizione di `assignable`.** Serve alla riassegnazione: si
aggiunge un termine, non si toglie uno stato.

**Le altre tre schermate con `zod` non si toccano.** Sono rotte pigre e il loro peso non
è sulla prima visita; «uniformare» toglierebbe validazione dove non costa niente.

**Le azioni avvolgono.** Chi aggiunge il bottone del thread o quello della password non
reintroduca `shrink-0`: è la trappola registrata dalla fetta B, e a 375 px fa scorrere in
orizzontale la pagina intera.

### Come si saprà che è finita

- i due form di accesso mostrano la password, da mouse e da tastiera;
- `/accesso` **misurata di nuovo**, con il numero scritto nell'esito;
- la barra laterale arriva in fondo su una schermata corta, e il selettore sta in basso;
- una stanza si crea con tutti i suoi dati, e la schermata dice che si correggono dopo;
- «Prendi in carico» non compare su ciò che si ha già in carico, e compare su ciò che ha
  qualcun altro;
- il thread ha il suo bottone, che si disabilita mentre lavora;
- `npm run typecheck`, `lint`, `test`, `build` verdi, e **424 test Rust invariati**.

### Che cosa **non** entra

- **`R1`, `R11`, `R25`** — gli stati che nessuno aggiorna: sono la sezione 4, e hanno una
  decisione comune da prendere una volta sola;
- **il collaudo in undici punti**, che è la sezione 2;
- **qualunque cosa tocchi l'API.** Se durante l'esecuzione sembra necessaria una rotta,
  la conclusione giusta è che quella cosa appartiene a un'altra sezione.

<a id="esito-sezione-1"></a>

## Esito — Sezione 1, ritocchi visibili

**Eseguita l'8 settembre 2026**, per intero: `S1`–`S6`, cioè `R5`, `R4`, `R6`, `R10`,
`R20` e il bottone del thread. Le sei voci escono da [12](12-da-fare.md).

**Il backend non è stato toccato**, come il piano prevedeva: **424 test Rust prima e
dopo**, e nessuna migration, nessuna rotta, nessun DTO. Il client passa da **69 a 73
test**: uno sul bottone della password, tre su «Prendi in carico».

### Che cosa è entrato

| Passo | Esito |
|---|---|
| `S1` | un bottone dentro i due campi password: `aria-label` che cambia, `aria-pressed`, e il `mousedown` annullato perché scoprire la password non tolga il fuoco a chi sta digitando |
| `S2` | `zodResolver` sostituito dalle regole native di `react-hook-form` **su quel file soltanto**. I numeri sotto |
| `S3` | l'altezza sul contenitore flex, la barra come colonna `lg`, il selettore in fondo con `mt-auto`, l'elenco che scorre a sé — **più una terza cosa che il piano non aveva previsto**, sotto |
| `S4` | `NewRoomDialog` e `RoomDialog` condividono un solo `RoomFields`: sette campi identici invece di quattro e cinque divergenti. `status` è restato fuori |
| `S5` | «Prendi in carico» sparisce a chi ce l'ha, e resta su ciò che è di qualcun altro |
| `S6` | il bottone «Aggiorna» nell'intestazione del thread, disabilitato mentre rilegge |

### La misura di `/accesso`, rifatta

Rifatta **sullo stesso albero, prima e dopo**, con uno script che somma la chiusura
transitiva degli import di ogni entry — il guscio più il chunk della rotta — e ne
comprime il risultato. È lo stesso criterio della fetta C.

| Rotta iniziale | Prima | Dopo | |
|---|---|---|---|
| `/accesso` — il login | 556,5 kB · 176,6 gz | **480,1 kB · 156,0 gz** | −13,7 % · −11,7 % |
| `/` — riepilogo | 449,3 kB · 145,0 gz | 449,8 kB · 145,2 gz | +0,5 kB, le stringhe nuove |
| `/addebiti` — chi abita | 466,6 kB · 151,8 gz | 467,1 kB · 152,0 gz | idem |
| `/invito` | 451,4 kB · 146,3 gz | 451,9 kB · 146,5 gz | idem |

**Due cose vanno lette e non passate sopra.**

La prima: **i numeri assoluti non coincidono con quelli della fetta C** — che dava 543,4
kB per `/accesso` e 438,7 per il riepilogo — e la ragione è che quella misura è di un
altro albero e forse di un'altra compressione. È per questo che il confronto qui sopra è
*prima e dopo con lo stesso script*, e non «543,4 → 480,1»: due misure prese con due
metodi non si sottraggono. Il **delta** è la cosa vera, e vale −76,4 kB (−20,6 gz).

La seconda: **`/accesso` non torna al livello del riepilogo**, e restano 30 kB di
distanza. Non è `zod`, che è sparito dalla sua chiusura: è **`react-hook-form`**, che i
due form usano e che il riepilogo non tira. Prima era dentro lo stesso chunk di `zod` e
la divisione lo ha reso un pezzo a sé. Trenta kB per la validazione dei due form è il
prezzo della cosa stessa, non un debito: toglierlo vorrebbe dire scrivere quei form a
mano.

### Che cosa il piano non aveva previsto

**`R6` richiedeva tre modifiche e non due.** Il piano diceva l'altezza sul contenitore
flex e `mt-auto` sul blocco, ed entrambe servono; ma con **quelle sole**, su una
schermata **lunga**, la barra si allunga quanto il *documento* e il selettore ancorato in
fondo finisce a mille pixel di distanza — cioè il difetto di partenza al contrario:
prima era troppo corta, così sarebbe stata troppo lunga. Il piano chiedeva di provare
anche una schermata lunga, ed è quella prova ad averlo trovato.

Il rimedio è che «tutta altezza» significhi **la finestra e non il documento**:
`lg:sticky lg:top-0 lg:h-screen` sulla barra. Con `h-screen` la colonna è alta quanto la
finestra qualunque cosa faccia il contenuto, e `sticky` la tiene ferma mentre il resto
scorre. Il `min-h-screen` sul contenitore flex **resta** e non è ridondante: è ciò che
regge il caso opposto, la schermata corta, dove senza di esso lo sfondo della colonna si
fermerebbe a metà.

**Le quattro geometrie, misurate nel browser e non a occhio:** finestra 1024×400 con un
documento da 834 px — barra alta 400, elenco scorrevole a sé, selettore in fondo;
scorrendo a 400, la barra resta a `top: 0`; a 375 px la barra torna `display: block`,
`position: static`, l'elenco non scorre e il blocco del selettore è `display: none`,
perché lì il selettore sta nella barra in alto. Nessuno scorrimento orizzontale.

**Il bottone della password ha reso ambiguo `getByLabelText(/password/i)`**, che ora
trova due controlli: il campo e il bottone, che si chiama «Mostra la password» perché è
ciò che fa. Quattro test esistenti lo interrogavano così. Si è scelto di **dire quale dei
due si vuole** (`{ selector: 'input' }`) invece di rendere il bottone più muto: due
controlli distinti devono restare distinguibili anche per chi li ascolta.

**Il giro di tabulazione è cambiato**, ed è una conseguenza voluta: fra il campo password
e «Entra» c'è ora il bottone che scopre la password. Il test che lo percorre lo dice
esplicitamente.

### Che cosa è entrato senza essere nel piano

- **la virgola decimale nei campi della stanza.** `420,50` digitato in un'interfaccia
  italiana produceva un `422`, e il catalogo su quel codice dice «controlla i campi
  segnalati» **senza segnalarne nessuno**. Vale ora per canone, superficie e quota, in
  entrambi i form, perché passano dallo stesso componente;
- **la quota delle utenze è verificata prima di partire**, 0–100 con al più due decimali
  come `app.percentage`, e **solo dopo un tentativo di invio** — un errore che compare a
  ogni tasto mentre si scrive `33,33` è rumore. Senza questo, l'unico messaggio sarebbe
  stato quel «controlla i campi segnalati» che non segnala niente;
- **la creazione di una stanza invalida anche l'immobile**: il conteggio dei locali sta
  lì, e prima non si aggiornava;
- **tre test su «Prendi in carico»**, che è la voce dove una regressione sarebbe muta:
  togliere `assigned` dalla condizione sembra la semplificazione ovvia e toglierebbe la
  **riassegnazione**. Il test è stato provato al contrario — con la condizione vecchia
  fallisce.

### Come è stato verificato

`just ci` — `fmt`, `clippy -D warnings`, purezza del dominio, i test del workspace, il
client per intero, e la suite con il database: **424 test Rust, 73 del client**.

E poi **usandolo**, che per una sezione di soli ritocchi è il criterio vero: accesso con
la password mostrata e nascosta, la barra alle tre altezze e a 375 px, una stanza creata
con tutti i campi — quota a `150` rifiutata dal client, `33,33` accettata, piano e
arredata riletti dalla modale di modifica — e poi archiviata. Il bottone del thread
provato in volo: una richiesta sola, disabilitato mentre rilegge.

**Un pezzo non è stato provato a mano, e va detto:** «Prendi in carico» che **sparisce**
richiede una segnalazione assegnata, cioè una transizione irreversibile su dati veri.
Coperto dai tre test invece che dal giro a mano; l'altra metà — il bottone che *c'è* su
una segnalazione aperta di nessuno — è stata vista.

### Che cosa resta aperto

- **la sezione 2**, il collaudo in undici punti della fetta C, che resta il primo lavoro;
- **`R11` e `R1`**, gli stati che nessuno aggiorna: `status` è restato fuori dal form
  della stanza apposta, ed è la sezione 4;
- **il thread non è live**, e la scelta è registrata: il bottone dice la verità, cioè che
  i messaggi arrivano quando li si va a prendere.

<a id="thread-aggiorna"></a>

### Il thread non diventa live: un bottone «Aggiorna»

La domanda era se si potesse rendere la conversazione in tempo reale. **Si è deciso di
no**, e nemmeno con un intervallo automatico: nel riquadro del thread ci va un bottone
che rilegge, premuto da chi vuole.

È **solo client** e non aggiunge niente all'architettura: il thread ha già una chiave
propria (`keys.ticketMessages`), scelta perché invalidarlo non rilegga anche il
dettaglio, quindi il bottone è la sua `refetch`. Va progettato come tutto il resto —
lo stato di caricamento c'è già, e il bottone si disabilita mentre rilegge.

**Perché sono state scartate le altre due**, e vale la pena lasciarlo scritto perché la
domanda tornerà:

- **l'intervallo automatico** (`refetchInterval`) costa poco ma fa richieste che
  nessuno ha chiesto, per una conversazione che nel prodotto vero procede a ore di
  distanza. Un bottone dice la verità: i messaggi arrivano quando li si va a prendere;
- **il tempo reale** (`SSE`, più adatto di WebSocket perché il flusso è a senso unico)
  ha una complicazione che non è di trasporto ma di sicurezza: `EventSource` non può
  mandare intestazioni, quindi niente `Authorization`. Il token **nell'URL** è escluso —
  finirebbe in cronologia, `Referer` e log dei proxy, che è l'argomento per cui il
  codice d'invito non viaggia in un percorso ([ADR-045](08-decisioni.md#adr-045)), e per
  giunta si tratterebbe dell'access token. Autenticare con il cookie significherebbe
  usare quello del *refresh* per altro, cioè una decisione di sicurezza a sé.

Se un giorno servisse davvero, il pezzo che manca non è la sorgente: **l'outbox degli
eventi esiste** e il worker lo drena. Manca la decisione sull'autenticazione del flusso,
e conviene prenderla dopo [`R9`](12-da-fare.md#lelenco), che ridisegna comunque chi vede
che cosa.
