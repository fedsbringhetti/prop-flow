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
[Perché la dashboard doveva restare ultima](#perche-la-dashboard-doveva-restare-ultima), perché è la conclusione sbagliata
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

**Il collaudo in undici punti è stato percorso il 9 e il 10 settembre 2026**, e con esso
questo esito **si chiude**. È restato aperto per due giorni e cinque sezioni, ed è
l'attesa più lunga che questo archivio registri: il criterio della fetta non era la suite,
era che una persona che non ha scritto il codice attraversasse il prodotto con due account
e un browser. È successo, e ciò che ha trovato sta nella tabella qui sotto e in
[12](12-da-fare.md) come `R30`–`R35`.

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

### Il collaudo — percorso

**Percorso il 9 e il 10 settembre 2026**, per intero, **da chi non aveva scritto il
codice**. Rinviato una volta — il 9 settembre, dopo i primi due punti, con la ragione
nell'[esito della sezione 2](#esito-sezione-2) — e poi ripreso e finito. I punti 4 e 10
erano **stati corretti** prima di percorrerli, ed è scritto sotto la tabella.

**Nessuno degli undici punti si è fermato.** Il prodotto regge un giro completo con due
account e un browser, e questa è la prima volta che qualcuno può dirlo. Le sei cose
trovate sono difetti *dentro* percorsi che funzionano, non percorsi interrotti — che è una
distinzione che conta, perché dice che ciò che manca è rifinitura e non fondamenta.

| # | Punto | Esito |
|---|---|---|
| 1 | il proprietario si registra e atterra sul riepilogo vuoto | percorso, nessun difetto riportato |
| 2 | immobile, due stanze, **una corretta** (`C4`) | percorso. È il punto in cui si vede `R30`: una stanza occupata dice «Occupata (Disponibile)». La divergenza vera — «Non disponibile · occupata» su una stanza in ristrutturazione e abitata — è stata **verificata e funziona**, ed è la ragione per cui `R30` corregge la regola invece di toglierla |
| 3 | due contratti per stanza, parti, attivazione | percorso. Qui si vede l'altra faccia di `R30`: un contratto scaduto dice «Scaduto (Attivo)» |
| 4 | due inviti; il primo **revocato, e il codice revocato presentato davvero** — che è l'unico modo di provare la revoca — poi rifatto (`C1`) | percorso, e la revoca rifiuta: `C1` regge nella forma corretta da `T1` |
| 5 | i due inquilini si registrano e **accettano** (`C1`), nell'organizzazione giusta | percorso. **È il punto che ha prodotto `R31`**, che il giro ha incontrato al secondo contratto e non al primo: un inquilino già membro non può essere collegato a un contratto nuovo |
| 6 | bolletta, scansione, revisione OCR, validazione, riaddebito | percorso. Ha prodotto `R33` — il riaddebito non dice a quale contratto assegna, ed elenca i contratti come *tipo · data* — e metà di `R32` |
| 7 | segnalazione con **foto allegata** (`C3`), e il ciclo completo | percorso. Ha prodotto metà di `R32` — il collegamento all'immobile dice «Immobile» — e ha mostrato che il ciclo delle transizioni **non si capisce dall'interfaccia**, che è `R8` e non un difetto nuovo |
| 8 | dichiarazione di pagamento, conferma, residuo che scende | percorso, e i numeri tornano. Ha prodotto `R34`, le date che non propongono oggi |
| 9 | gruppo di spesa, una spesa a testa, rimborso, saldi a zero | percorso, nessun difetto riportato |
| 10 | **due schede dello stesso account**, stesso profilo, token scaduto, ricaricate insieme: nessuna delle due viene buttata fuori (`C2`) | percorso, nessuno buttato fuori. **`C2` regge**, ed era il punto che prima della fetta C era *certo* che fallisse |
| 11 | il percorso dell'inquilino **a 375 px** | percorso, nessun difetto riportato |

**Il dettaglio di queste righe è più magro di quanto `T7` chiedesse, e va detto invece di
lasciarlo sembrare completo.** Il metodo voleva «che cosa si è visto e non un segno di
spunta»; il resoconto è arrivato consolidato — *tutto andava tranne quello che ho scritto*
— quindi per i punti 1, 9, 10 e 11 qui resta l'affermazione che sono stati percorsi e che
non hanno prodotto difetti, senza l'osservazione che il metodo voleva. È una perdita
reale: fra sei mesi nessuno saprà se al punto 9 i saldi tornavano a zero **da entrambe le
parti** o solo da una. Il rimedio, la prossima volta, non è ricordarsi meglio: è scrivere
la riga **mentre** si percorre il punto, non alla fine del giro.

Ciò che il giro ha trovato è entrato in [12](12-da-fare.md) con identificatori nuovi —
`R30`–`R35`, ripartendo da dove `R29` si era fermata — e **non è stato corretto durante il
giro**, come la regola prevede.

**Due punti sono stati corretti prima di percorrerli**, e ciò che dicevano resta scritto
qui sotto perché è quello che si credeva. Nessuno dei due era una previsione smentita:
erano istruzioni che **non provavano ciò che dichiaravano**, e percorrerle com'erano
scritte avrebbe prodotto due righe verdi che non significano niente.

- il **10** diceva «*due finestre insieme*, token scaduto, nessuno buttato fuori». Due
  finestre di **due account** sono due contenitori di cookie, quindi due catene di refresh
  distinte: non c'è contesa, e il punto passerebbe anche con il lock di `C2` rimosso. Ciò
  che `C2` protegge è la corsa di due schede della **stessa** sessione;
- il **4** diceva «due inviti, *uno revocato e rifatto*», che si percorre premendo
  «revoca» e creando un invito nuovo — **senza mai presentare il codice revocato**. La
  revoca si prova solo provando a usarlo: deve essere rifiutato.

La rilettura che li ha trovati è il passo `T1` della
[sezione 2](#piano-sezione-2), e la ragione per cui una correzione
entra nell'archivio è la sua decisione 1.

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

---

<a id="piano-sezione-2"></a>

## Piano — Sezione 2: le pendenze della fetta C

Pianificata l'**8 settembre 2026**, subito dopo la [sezione 1](11-piani-conclusi.md#esito-sezione-1).
È la seconda delle nove dell'[ordine di esecuzione](12-da-fare.md#lordine-di-esecuzione), e
chiude `R2`, `R3` e **il collaudo in undici punti** — cioè la fetta C, che alla
pianificazione aveva un [esito aperto](11-piani-conclusi.md#esito-fetta-c) e che il
collaudo ha poi chiuso il 10 settembre 2026, dalla [sezione 5-bis](#esito-sezione-5-bis).

**Qui il codice è due stringhe.** Tutto il resto è prosa e cammino, ed è il motivo per cui
questa sezione esiste: il criterio della fetta C non è la suite — che è verde da un giorno
— ma che **una persona attraversi il prodotto per intero con due account e un browser,
senza mai aprire `curl`**. Finché quel giro non è stato fatto, «funziona» è
un'affermazione che nessuno ha verificato.

### Stato di avanzamento, all'8 settembre 2026

**`T1`, `T2` e `T3` sono fatti; il giro no.** Chi riprende parte da `T4`, e l'ambiente è
già quello che `T3` chiede.

| Passo | Stato |
|---|---|
| `T1` | **fatto.** Il punto 10 corretto, e la rilettura ha trovato **il punto 4** con lo stesso difetto: «uno revocato e rifatto» si percorre senza mai presentare il codice revocato. Corretti entrambi in luogo, con le formulazioni vecchie conservate |
| `T2` | **fatto.** Le otto occorrenze e la nota ad [ADR-023](08-decisioni.md#adr-023); il messaggio riletto davvero, con un ambiente incompleto |
| `T3` | **fatto.** Volumi cancellati, database ricreato — 32 migration, invarianti tutte rispettate — bucket nuovo, `SECURE_COOKIES=false` già a posto. Che cosa è stato cancellato è annotato qui sotto |
| `T4`–`T6` | **aperti.** Il giro non è stato percorso |
| `T7` | **aperto**, perché è l'esito del giro |

**Che cosa il `down-hard` ha portato via**, come la decisione 5 chiede di annotare:
un'organizzazione (`AuraFact`), un utente, un immobile in bozza con due stanze, un
contratto in bozza senza parti, una segnalazione aperta. Nessun documento, quindi nel
bucket non c'era niente. L'evidenza di `R18` non c'era già più.

**Il giro è cominciato e si è fermato al punto 2**, che ha già trovato un difetto —
[`R26`](12-da-fare.md#lelenco), il filtro «Archiviato» che non può restituire niente — e
questa è la dimostrazione più corta del perché la sezione esiste: il difetto è saltato
fuori al secondo punto, camminando.

**Che cosa serve per riprendere:** due account inquilino registrati, e le credenziali dei
tre. La registrazione è il solo gesto che chi guida non può fare al posto di chi cammina,
ed è anche il punto 1 e la prima metà del punto 5.

### Che cosa è stato deciso prima di pianificare

| | Scelta | Alternativa scartata |
|---|---|---|
| Il testo del punto 10, che vive nell'archivio | si corregge **in luogo e in modo visibile**, lasciando leggibile ciò che diceva | spostare il collaudo in `09`, o correggerlo solo qui nel piano |
| Il messaggio della configurazione | nomina **`just`** come via normale e **conserva** `set -a; source .env; set +a` | dire solo `just`, o togliere la frase |
| [ADR-023](08-decisioni.md#adr-023) | una **nota di aggiornamento**: la decisione non è cambiata | un ADR nuovo sul `justfile` |
| Lo stato del database su cui si cammina | **vuoto**, ma guardando prima che cosa si cancella | camminare sopra i dati che ci sono |
| Chi cammina | **chi non ha scritto il codice**, cioè non chi lo ha scritto | far percorrere il giro a chi ha fatto la fetta |

### Decisioni, con il perché

#### 1. Un testo dell'archivio si corregge in luogo, e la correzione si vede

`11` **non si riscrive mai** — nemmeno per aggiustare una previsione che l'esecuzione ha
smentito, che è la parte più utile da rileggere. Ma la tabella del collaudo è
un'eccezione dichiarata **dentro** l'archivio stesso: dice «questa sezione si riempie
percorrendolo, ed è ciò che chiude la fetta». Riempirla è già una scrittura in `11`.

Il punto 10 però non è una previsione sbagliata: è un'**istruzione che non prova ciò che
dichiara**, e percorrerla com'è scritta produrrebbe una riga verde che non significa
niente. Si corregge quindi in luogo, **conservando la formulazione vecchia** — barrata, o
in una riga sotto la tabella — perché il valore dell'archivio è anche che si veda che cosa
si credeva.

*Alternativa scartata: spostare il collaudo in `09`.* Separerebbe il criterio dalla fetta
che lo ha prodotto, e `09` si riscrive a ogni fetta: i risultati del giro sparirebbero
alla successiva.

*Alternativa scartata: correggere solo qui nel piano.* Chi percorre il collaudo legge la
tabella in `11`, non il piano di una sezione che nel frattempo è finito in archivio.

#### 2. Il punto 10 prova `C2` solo con **due schede dello stesso account**

Com'è scritto — «il proprietario in una finestra, un inquilino nell'altra» — non esercita
niente: due account sono due contenitori di cookie, quindi due catene di refresh distinte,
e **non c'è contesa**. Il punto passerebbe anche con il lock rimosso, cioè è un test che
non può fallire.

Quello che `C2` protegge è la corsa di **due schede della stessa sessione** con l'access
token scaduto insieme: la seconda presenta un refresh token già ruotato, e l'API **revoca
la catena** — entrambe fuori. La versione che lo esercita è quindi: *stesso account,
stesso profilo del browser, due schede, ricaricate insieme dopo la scadenza*.

Va scritto anche **che cosa si guarda**, o il punto resta ambiguo: il criterio è che
nessuna delle due schede venga buttata fuori, e prima della fetta C era **certo** che
fallisse.

#### 3. Il messaggio della configurazione nomina `just`, e non toglie la shell

`R3` sembra una riga e sono **otto occorrenze**: la stessa affermazione — «`.env` lo
carica la shell» — è ripetuta in `crates/api/src/config.rs` (il messaggio d'errore e il
commento di `AppConfig`), `CLAUDE.md` §4, `README.md`, `.env.example`, `docs/06-sviluppo.md`
(due volte: i comandi e la tabella dei sintomi), `docs/07-operazioni.md` (due volte) e
[ADR-023](08-decisioni.md#adr-023). Dal `justfile` con `set dotenv-load` non è più tutta
la verità.

Il messaggio d'errore lo legge **chi ha appena visto l'API non partire**, e quella persona
può averla avviata in due modi. Dice quindi entrambi: `just api` lo carica da sé; chi
lancia `cargo run` a mano fa `set -a; source .env; set +a`.

*Alternativa scartata: dire solo `just`.* Sarebbe falso per chi lancia il binario, ed è
esattamente la persona che quel messaggio incontra.

*Alternativa scartata: togliere la frase.* È il rimedio, cioè la ragione per cui il
messaggio esiste: senza, torna a essere la validazione di figment che dice che manca un
campo e non da dove debba arrivare.

#### 4. `ADR-023` prende una nota, non un successore

La decisione è «`.env` è un file per la shell, **non una sorgente di configurazione**», e
**non è cambiata**: nessun crate dipende da `dotenvy`, il processo continua a non leggere
quel file, e `just` lo carica **fuori** dal processo esattamente come faceva la shell. È
cambiata l'ergonomia, non la proprietà.

Quindi una nota di aggiornamento in coda all'ADR, datata. Un ADR nuovo direbbe che si è
deciso qualcosa, e non si è deciso niente: si è aggiunto un modo di digitare meno.

#### 5. Si cammina su un database vuoto, e prima si guarda che cosa si cancella

Il punto 1 è «il proprietario si registra e atterra sul **riepilogo vuoto**»: su un
database con dei dati quel punto non è osservabile, e i nove successivi partirebbero da uno
stato che nessuno ha scritto.

Ma cancellare non è gratis, e va fatto **guardando**: `just down-hard` porta via anche i
volumi di **MinIO**, quindi i documenti caricati, e quello che c'è nel database di sviluppo
è l'unica prova osservata di alcune voci di [12](12-da-fare.md). Il passo `T3` chiede
perciò di guardare prima e di annotare ciò che sparisce.

**Un caso è già successo**, ed è la ragione per cui questa decisione è scritta: il
contratto attivo con le quote `100 / 0` che documenta [`R18`](12-da-fare.md#lelenco)
**non è più** nel database di sviluppo. La voce resta valida — è descritta in prosa, con
il meccanismo che la produce — ma la sezione 3 non deve contare di trovarne l'esemplare.

#### 6. Il giro lo percorre chi non ha scritto il codice

È metà del criterio, non un dettaglio organizzativo: chi ha scritto una schermata la
attraversa come la pensava, non come la incontra chi non sa dov'è il bottone.

Ne segue una divisione pratica del lavoro, e va detta perché altrimenti la sezione si
ferma: **la registrazione dei due account e i gesti del giro sono di chi collauda**;
preparare l'ambiente, il testo del punto 10, il messaggio di `R3` e la scrittura dell'esito
si possono fare prima e dopo. Il piano descrive quindi ogni punto con **che cosa si
osserva**, non solo che cosa si fa.

### Decomposizione

`T1` e `T2` sono indipendenti da tutto il resto e si possono fare subito. `T3` prepara,
`T4`–`T6` sono il giro, `T7` lo chiude.

##### `T1` — Il punto 10, e gli altri dieci riletti con lo stesso metro

Correggere il punto 10 in `11-piani-conclusi.md`, conservando la formulazione vecchia.

**E rileggere gli altri dieci con la stessa domanda**: *questo punto, come è scritto,
prova ciò che dichiara?* Costa dieci minuti ed è l'unico momento in cui conviene farlo —
dopo, ogni punto sarà stato percorso e la domanda non si porrà più. `R2` è nato così, da
un solo punto guardato con quella domanda.

##### `T2` — `R3`, le otto occorrenze e la nota all'ADR

Il messaggio in `crates/api/src/config.rs` e il commento di `AppConfig`; poi `CLAUDE.md`
§4, `README.md`, `.env.example`, `docs/06-sviluppo.md`, `docs/07-operazioni.md`; infine la
nota in coda ad [ADR-023](08-decisioni.md#adr-023).

**Nessun test asserisce su quel messaggio** — la fetta C lo ha verificato — quindi la
suite non protegge questo passo: la verifica è rileggerlo dopo averlo cambiato, con
`cargo run -p propflow-api` e un `.env` incompleto.

##### `T3` — Preparare il giro

Guardare che cosa c'è nel database e annotarlo; poi `just down-hard`, `just up`, `just db`,
e i tre processi: API, worker, client. **Il worker serve davvero** — vedi le trappole.

Preparare anche le due cose che il giro chiede e che non si improvvisano:

- `PROPFLOW__SERVER__SECURE_COOKIES=false`, o su HTTP il browser non manda il cookie e la
  sessione dura i quindici minuti dell'access token — e il punto 10 fallirebbe per la
  ragione sbagliata;
- **la finta bolletta** che l'OCR sa leggere, che è un file di testo (sotto, nelle
  trappole, con il contenuto pronto).

##### `T4` — Punti 1–5: dal riepilogo vuoto ai due inquilini dentro

Registrazione e atterraggio; immobile, due stanze **e una corretta**; due contratti per
stanza con le parti e l'attivazione; due inviti, **uno revocato e rifatto**; i due
inquilini che si registrano e accettano, **nell'organizzazione giusta**.

Che cosa si osserva, oltre al fatto che non ci siano errori: che l'organizzazione attiva
dopo l'accettazione sia quella dell'invito e non quella personale (`C1`, ADR-045), e che
l'invito revocato **non funzioni più**.

##### `T5` — Punti 6–9: il denaro e i guasti

Bolletta con scansione, revisione dell'OCR, validazione e riaddebito; segnalazione **con
foto** e ciclo completo; dichiarazione di pagamento, conferma, residuo che scende; gruppo
di spesa, una spesa a testa, rimborso, saldi a zero.

Qui si guarda soprattutto **il verso dei numeri**: l'insoluto che scende della cifra
giusta, e i saldi co-living che tornano a zero da entrambe le parti.

##### `T6` — Punti 10 e 11: la corsa fra schede, e i 375 px

Il punto 10 nella forma corretta da `T1`. Il punto 11 è il percorso dell'inquilino a
375 px, **dalla parte sua**: entra, vede quanto deve, dichiara di aver pagato, segnala un
guasto, guarda le spese.

##### `T7` — L'esito, e la fetta C chiusa

Riempire le undici righe **anche dove è andato tutto bene**, con che cosa si è visto e non
con un segno di spunta. Ciò che il giro ha trovato entra in [12](12-da-fare.md) con
identificatori nuovi — **si continua da `R26`**, i numeri chiusi non si riusano — e non si
corregge qui. Poi l'esito della fetta C smette di essere aperto, e `R2` e `R3` escono da
`12`.

### Trappole

**Il punto 10 vuole una sola origine.** Il lock del rinnovo è **per origine**: una scheda
su `http://localhost:5173` e una su `http://localhost:8081` — lo stack dietro il proxy —
sono due origini diverse, `navigator.locks` non le coordina e nemmeno il ripiego in
`localStorage`, che è per origine anch'esso. Due schede così proverebbero il **contrario**
di ciò che si vuole, e in silenzio.

**Senza il worker, la bolletta non arriva mai in revisione, e il sintomo è muto.** Niente
va in errore: semplicemente non succede niente. È già scritto nel `justfile` e in
`CLAUDE.md` §4, ed è la trappola che costa di più al punto 6.

**L'OCR di sviluppo non legge né PDF né immagini, e non è un difetto.**
`DeterministicOcrProvider` tratta il documento come **testo** e cerca righe
`Etichetta: valore`; un PDF o un JPEG li rifiuta con `UnsupportedDocument`, che è la
verità — non sa leggerli. Chi carica una scansione vera al punto 6 conclude che il
prodotto è rotto. La finta bolletta da caricare, con le etichette che riconosce:

```
Fornitore: Enel Energia
Partita IVA: 06655971007
Numero fattura: 2026-000123
Data emissione: 01/09/2026
Scadenza: 30/09/2026
Periodo dal: 01/07/2026
Periodo al: 31/08/2026
Imponibile: 96,00
IVA: 21,12
Totale: 117,12
Consumo: 412,5 kWh
```

**`just down-hard` porta via anche MinIO.** I documenti caricati sono oggetti nel bucket,
non righe: si perdono con i volumi, e la riga che li cita resta senza byte.

**Non si corregge niente a metà giro.** Ciò che si trova si **annota** e si continua:
correggere cambia il sistema sotto i piedi, e i punti successivi non provano più lo stato
che si stava attraversando. Le correzioni sono le sezioni 3–9.

**L'esito si scrive anche se è andato tutto bene.** Una fetta che chiude con «tutto bene»
dopo un giro come questo è, quasi sempre, una fetta il cui giro non è stato fatto.

**Gli identificatori non si riusano.** `R4`, `R5`, `R6`, `R10` e `R20` sono chiusi: ciò
che esce dal giro parte da `R26`.

### Come si saprà che è finita

- le **undici righe** hanno un esito scritto, con che cosa si è visto;
- il punto 10 è stato percorso **nella forma che esercita `C2`**, e nessuna delle due
  schede è stata buttata fuori;
- ciò che il giro ha trovato è in [12](12-da-fare.md), con identificatori nuovi e senza
  essere stato corretto;
- `R2` e `R3` sono chiusi ed escono da `12`; l'esito della fetta C non è più aperto;
- `just ci` verde: `R3` tocca una stringa in Rust, e il resto è prosa.

### Che cosa **non** entra

- **correggere i difetti che il giro trova.** Vanno in `12` e nelle sezioni 3–9: una
  sezione che si allarga a ciò che incontra non finisce mai, e questa deve chiudere la
  fetta C;
- **le notifiche**, che restano dopo il collaudo per decisione della fetta C;
- **il fornitore di OCR vero**: il punto 6 si percorre con l'adattatore deterministico, che
  esiste apposta per rendere osservabile il percorso prima che un fornitore sia scelto;
- **qualunque cosa richieda una decisione di prodotto.** Se un punto del giro non si può
  percorrere perché manca una decisione, quella è una voce di `12`, non un lavoro da fare
  qui.

<a id="esito-sezione-2"></a>

## Esito — Sezione 2: le pendenze della fetta C

Eseguita l'**8 e il 9 settembre 2026**. Chiude `R2` e `R3`. **Il collaudo in undici punti
non è stato percorso**, ed è una decisione presa a metà strada, non una cosa rimasta
indietro: sta nella sezione [5-bis](12-da-fare.md#lordine-di-esecuzione).

### Che cosa è entrato

| Passo | Esito |
|---|---|
| `T1` | Il punto 10 corretto — vuole **due schede dello stesso account** — e la rilettura degli altri dieci ha trovato **il punto 4** con lo stesso difetto |
| `T2` | `R3` in **otto** occorrenze, non due, più la nota di aggiornamento ad [ADR-023](08-decisioni.md#adr-023) |
| `T3` | Ambiente preparato: volumi cancellati, database ricreato con 32 migration e invarianti verdi, bucket nuovo, `SECURE_COOKIES=false` verificato, e la finta bolletta che l'OCR deterministico sa leggere |
| `T4`–`T7` | **non eseguiti**, vedi sotto |

### Che cosa il piano non aveva previsto

**Il punto 4 aveva lo stesso difetto del punto 10, e nessuno lo sapeva.** Diceva «due
inviti, uno revocato e rifatto», che si percorre premendo «revoca» e creando un invito
nuovo — **senza mai presentare il codice revocato**, che è l'unico modo di provare la
revoca. Non era in elenco: è uscito da `T1`, che chiedeva di rileggere gli altri dieci
punti con la stessa domanda del 10. È la dimostrazione più corta che quel passo serviva.

**`R3` non era «due righe in `config.rs`».** La stessa affermazione — «`.env` lo carica la
shell» — viveva in otto posti: il messaggio d'errore, il commento di `AppConfig`,
`CLAUDE.md` §4, `README.md`, `.env.example`, due righe di `06`, due di `07` e l'ADR-023
stesso. Sono invecchiate tutte insieme il giorno in cui il `justfile` è entrato, ed è ciò
che succede a un fatto ripetuto in otto punti.

### Il collaudo è rinviato, e questa è la ragione

Il giro è stato **cominciato e fermato al punto 3**: i punti 1 e 2 sono stati percorsi, e
il 2 ha trovato [`R26`](12-da-fare.md#lelenco) — un immobile archiviato che sparisce anche
dal filtro «Archiviato». Cinque minuti d'uso, un difetto che nessuna lettura del codice
aveva trovato, e che ha poi cambiato il piano della sezione 3.

Poi ci si è accorti del costo: **sei punti su undici attraversano codice che le sezioni
rimanenti riscrivono.** Il 3 e il 4 li ha appena riscritti la sezione 3; il 2 e il 6 li
tocca la 4 (`R1`, `R11`, `R25`); il 7 lo riscrive tutta la 5; l'1 e il 5 dipendono da `R9`,
che è la sezione 8. Percorrerli ora vuol dire percorrerli due volte.

**Rinviato quindi a dopo la sezione 5**, dove nove punti su undici sono su codice fermo, e
non «alla fine», che non è una data. Restano scoperti l'1 e il 5 fino alla sezione 8, ed è
un rischio accettato: aspettare anche quella significherebbe non provare niente per cinque
sezioni.

**Il costo del rinvio va detto**, perché è reale e non teorico: il collaudo non serve solo
a verificare, **alimenta i piani**. `R26` è entrata così, e la trappola del `LEFT JOIN`
della sezione 3 — quella che avrebbe fatto sparire dall'elenco i contratti di ogni immobile
archiviato — è stata scritta *perché* quel difetto era appena stato trovato. Le sezioni 4 e
5 si pianificheranno leggendo il codice soltanto, e la 4 parla proprio di stati che si
vedono usando.

### Che cosa resta pronto per chi riprende

- ~~nel database di sviluppo c'è il giro interrotto~~ — **non c'è più**: la sezione 4 ha
  eseguito `just down-hard` per provare la migration `0033` su un database pulito, e i
  volumi se ne sono andati con esso. Era un utente, due immobili di cui uno archiviato, due
  stanze e un contratto in bozza con due parti **entrambe al 100%** — fermo esattamente sul
  vicolo cieco che la sezione 3 ha poi aperto. **La lezione è che una migration si prova
  sul container effimero di `ops/ci/test-integration.sh`**, che esiste apposta, e non sul
  database di sviluppo di qualcun altro;
- il piano del giro è quello qui sopra, passi `T3`–`T7`, con le sette trappole e la finta
  bolletta;
- **l'API in esecuzione va riavviata** prima di riprendere: il binario è precedente alle
  rotte della sezione 3, e il client mostrerebbe bottoni che rispondono `404`.

### Che cosa ne è uscito, oltre alle due voci chiuse

`R26` — i filtri che offrono stati che non possono comparire, in tre elenchi con tre cause
diverse — e `R27`, il primo avvio a mano che `README.md` e `06` insegnano ancora mentre
`just setup` fa tutto in un comando. Nessuna delle due è stata corretta: è la regola del
giro, si annota e si va avanti.

---

<a id="piano-sezione-3"></a>

## Piano — Sezione 3: contratti, correggere gli errori

Pianificata il **9 settembre 2026**. È la terza dell'
[ordine di esecuzione](12-da-fare.md#lordine-di-esecuzione) e chiude `R13`, `R17`, `R18`,
`R12` e `R14`.

**È il danno peggiore che esiste adesso**, e non è un'ipotesi: in database ci sono stati
contratti attivi con le quote sbagliate, e non c'è modo di correggerli. La sezione esiste
per togliere un **vicolo cieco**, non per aggiungere una funzione.

### Il vicolo cieco, in quattro passi

Va letto prima di stimare qualunque cosa, perché è il motivo per cui queste voci stanno
insieme e non si prendono una alla volta:

1. `AddPartyDialog` propone `share_percent: '100'` e `is_primary: true`. Chi aggiunge la
   **prima** parte si prende tutto, e nel co-living la prima parte non è l'unica;
2. l'invito successivo riceve `remaining_share`, che a quel punto è **zero**, e nessuno lo
   dice;
3. l'attivazione verifica **la somma, non il senso**: `100 + 0 = 100` passa;
4. niente di tutto questo si corregge dopo — non si cambia una quota, non si toglie una
   parte, e il contratto non si elimina: l'unica uscita sarebbe attivarlo e disdirlo, che
   è precisamente ciò che non riesce.

**Il contratto resta lì per sempre**, non attivabile e non rimovibile.

C'è una prova che il codice stesso conosce il buco: `remaining_share`
(`crates/application/src/use_cases/invitation/invite_tenant.rs`) porta scritto che una
quota a zero va bene perché «chi amministra la correggerà». **Quella capacità non esiste**,
ed è `R17`. La funzione non va riscritta: va data al progetto l'operazione che il suo
commento presuppone.

### Che cosa è stato deciso prima di pianificare

| | Scelta | Alternativa scartata |
|---|---|---|
| Disfare un contratto sbagliato | **annullarlo**, esponendo `Lease::cancel` che esiste già | una `DELETE` con cancellazione soft |
| Correggere una parte | `PATCH /lease-parties/{id}`, come `rooms` | una rotta annidata sotto il contratto |
| Togliere una parte | soft delete **solo se non è mai stata collegata** a un utente | togliere anche una parte con un inquilino vero dietro |
| L'attivazione | rifiuta **una quota a zero**; non pretende che l'intestatario abbia accettato | pretendere l'accettazione |
| Aggiungere una parte a un contratto **attivo** | si rifiuta, con la ragione scritta | ammetterlo e ricontrollare la somma |
| Dare un nome ai contratti | **due campi in più nella risposta**: nome dell'immobile e della stanza | una colonna `label`, o comporre nel client |

### Decisioni, con il perché

#### 1. Un contratto sbagliato si **annulla**, e la transizione esiste già

`Lease::cancel()` è nel dominio da sempre: ammette `draft` e `pending_signature`, rifiuta
il resto con `LeaseNotCancellable`, e **non la chiama nessuno** — nessuno use case, nessuna
rotta. È stata trovata dalla sezione 2 mentre si guardava perché il filtro «Annullato»
fosse sempre vuoto ([`R26`](12-da-fare.md#lelenco)).

Quindi `R13` non è «manca la cancellazione»: è **una transizione scritta e non esposta**.
Serve uno use case, una rotta `POST /leases/{id}/cancel` e un bottone.

*Alternativa scartata: una `DELETE` con cancellazione soft.* Nasconderebbe la riga da ogni
elenco — `leases_select` filtra `deleted_at IS NULL` — cioè riprodurrebbe esattamente la
forma di `R26`: un'opzione «Annullato» che continua a non poter corrispondere a niente. E
**perderebbe l'informazione**: che un contratto sia stato annullato è un fatto da leggere,
non da nascondere. La cancellazione soft resta la via giusta per gli *immobili*, dove
l'archiviazione è un'altra cosa.

**Ne segue un effetto collaterale buono**: il filtro «Annullato» dei contratti smette di
essere morto, cioè **un terzo di `R26` si chiude qui**. Gli altri due terzi — immobili
archiviati e bollette pagate — restano aperti, e vanno detti come restanti invece di
segnare la voce come fatta.

#### 2. Le parti si correggono da una rotta di primo livello, e **non serve una migration**

La forma è quella delle stanze, che è il precedente più vicino: si creano annidate
(`POST /leases/{id}/parties`), si correggono e si tolgono da una rotta propria
(`PATCH` e `DELETE /lease-parties/{id}`). Il kebab-case è quello di `expense-groups` e
`utility-bills`.

**Il livello dati regge già tutto**, ed è la cosa da sapere prima di aprire l'editor:
`app.lease_parties` ha `deleted_at` e `left_at`, e la policy `lease_parties_update` della
`0014` concede l'`UPDATE` allo staff dell'organizzazione. **Chi scrive una migration qui
sta risolvendo il problema sbagliato.**

Quali campi si correggono: `share_percent`, `full_name`, `phone`, `role`, `is_primary`.
**L'email no, se la parte è già collegata a un utente**: l'email è ciò a cui l'invito è
intestato e ciò che l'accettazione confronta, e cambiarla sotto i piedi a un inquilino
registrato sposterebbe un rapporto già stabilito. Finché `user_id IS NULL` invece è
correggibile, ed è il caso vero — un refuso nell'indirizzo a cui si è mandato l'invito.

#### 3. Togliere una parte e far uscire un inquilino sono **due cose diverse**

`DELETE /lease-parties/{id}` è per l'**errore di inserimento**: una parte aggiunta per
sbaglio, mai collegata a nessuno. È una cancellazione soft — `deleted_at` — e gli indici
unici parziali fanno il resto: `lease_parties_primary_uk` e `lease_parties_lease_user_uk`
escludono le righe cancellate, quindi togliere una parte **libera** l'intestatario
principale e il legame con l'utente. È voluto e va saputo.

Su una parte **già collegata a un utente** la `DELETE` rifiuta. Quella non è una
correzione: è una persona che se ne va, e il modello ha un campo apposta — `left_at`, che
`ensure_tenancy_complete` già esclude dal conteggio delle quote. Ma «far uscire un
inquilino» porta con sé le domande di [`R16`](12-da-fare.md#lelenco) — che cosa vede dopo,
che ne è dei suoi addebiti — ed è la sezione 6. **Qui si rifiuta con un codice che dice
perché**, invece di far finta che sia la stessa operazione.

#### 4. L'attivazione rifiuta una quota a zero, e **non** pretende l'accettazione

Sono due domande diverse e hanno risposte diverse.

**Una quota a zero è un errore in ogni caso che sappiamo nominare**: un inquilino che non
deve niente non è un inquilino, è una riga rimasta indietro. L'attivazione la rifiuta, con
un invariante nuovo accanto a `LeaseSharesNotHundred`.

**L'accettazione dell'intestatario no.** Un contratto si firma sulla carta prima che
l'inquilino apra un account, e pretenderla bloccherebbe un giro legittimo — quello che il
collaudo stesso percorre ai punti 3–5, dove si attiva prima di invitare.

È un **cambiamento di regola di dominio**, quindi una voce in
[08 — Decisioni](08-decisioni.md), scritta prima del codice: dice che cosa l'attivazione
garantisce, che cosa deliberatamente non garantisce, e che i contratti già rotti in
database **non vengono sanati** da questa regola — la correggerà `R17` a mano, e va detto
o qualcuno penserà che l'invariante nuova valga anche all'indietro.

#### 5. Su un contratto attivo non si aggiungono parti

**Buco trovato pianificando, e non era in elenco**: `AddLeaseParty` fa `ensure_staff`,
carica il contratto e inserisce. **Non guarda lo stato.** Quindi si può aggiungere una
parte a un contratto **attivo**, e da quel momento le quote non sommano più a 100 senza
che niente protesti — l'invariante è verificata solo *al momento* dell'attivazione.

Si rifiuta. Il caso legittimo — un coinquilino che subentra a metà contratto — non è
un'aggiunta: è un cambio di composizione che deve tenere la somma a 100, cioè
un'operazione a sé che tocca due righe insieme e che nessuno ha ancora progettato.

*Alternativa scartata: ammetterlo e ricontrollare la somma dopo l'inserimento.* Non si può
fare in una richiesta sola — per aggiungere il terzo inquilino bisogna prima abbassare le
quote degli altri due — quindi o si rifiuta sempre, o si inventa una richiesta che porta
tutte le parti insieme. La seconda è il subentro, e va progettata quando serve davvero.

Va aperta una voce in [12](12-da-fare.md) per il subentro, con questo ragionamento.

#### 6. Il nome del contratto si compone nella **risposta**, non nel client e non in una colonna

`app.leases` non ha una colonna nome: ci sono `notes` e `registration_number`, e l'elenco
mostra tipo più date, da cui righe identiche. Le vie sono tre e non due.

**Scelta: due campi in più in `LeaseResponse`** — `property_label` e `room_name` — che è un
join nella query dell'elenco.

*Alternativa scartata: una colonna `label`.* È l'unica che permette di chiamarlo «*Rossi,
secondo piano*», e resta aperta come voce; ma è migration, dominio, DTO e form per
risolvere un problema che oggi è «sono tutti uguali», e quello lo risolve il join.

*Alternativa scartata: comporre nel client.* Sembra gratis — l'elenco degli immobili è una
richiesta sola — ma le stanze sono una richiesta **per immobile**, e il client
reimplementerebbe una giunzione che appartiene al modello di lettura.

**Il join va `LEFT`**, ed è la trappola: la stanza è opzionale per costruzione, e un
`INNER JOIN` farebbe sparire dall'elenco tutti i contratti sull'immobile intero.

#### 7. La sovrapposizione si **avverte**, non si vieta

`R12` è di solo client, ed è la scoperta che rende la voce piccola. I due vincoli di
esclusione della `0005` coprono `pending_signature`, `active` ed `expiring` — **non
`draft`** — e le bozze sono escluse *apposta*: due ipotesi alternative sullo stesso periodo
sono legittime. Il `409 lease.period_overlap` all'attivazione è quindi corretto e **resta**.

Quello che manca è dirlo prima: alla creazione il client chiede `listLeases` filtrato
sull'immobile e confronta i periodi. Se ne trova uno che si sovrappone, «Crea» chiede una
conferma in più.

**Non si toccano i vincoli**, e il confronto del client deve replicarli esattamente: stessi
tre stati, e la stessa dimensione — per immobile quando `room_id` è nullo, per stanza
quando c'è. Un client che confrontasse tutto per immobile avviserebbe su due stanze diverse
della stessa casa, cioè sul caso normale del co-living.

### Decomposizione

L'ordine è quello del danno: prima si apre il vicolo cieco, poi si smette di scavarlo.

##### `U1` — Annullare un contratto (`R13`)

Use case `CancelLease`, rotta `POST /leases/{id}/cancel`, bottone nella scheda del
contratto, offerto solo su `draft` e `pending_signature`. Il dominio c'è già: `cancel()` e
`LeaseNotCancellable`.

Il filtro «Annullato» dell'elenco, che oggi non può corrispondere a niente, comincia a
funzionare: **si verifica che sia così**, ed è la prova che `U1` è servito a qualcosa.

##### `U2` — Correggere una parte (`R17`)

`PATCH /lease-parties/{id}` con la semantica di sempre — assente = invariato, `null` =
azzerato — sui campi decisi. L'email solo se `user_id IS NULL`, con un codice proprio per
il rifiuto. Nel client: la riga delle parti diventa modificabile dalla scheda del
contratto.

**Nessuna migration.** La policy `lease_parties_update` esiste già.

##### `U3` — Togliere una parte aggiunta per sbaglio (`R17`)

`DELETE /lease-parties/{id}`: cancellazione soft, e rifiuto se la parte è collegata a un
utente. Un test deve provare **entrambi** i rami, e uno deve provare che togliere
l'intestatario principale libera davvero il posto — è l'indice unico parziale a garantirlo,
ed è la cosa che nessuno verificherebbe guardando.

##### `U4` — Le due regole che impediscono di rifare il danno (`R18`, e il buco della decisione 5)

Nel dominio: l'attivazione rifiuta una quota a zero fra gli inquilini; l'aggiunta di una
parte rifiuta su un contratto non più in bozza. Due invarianti nuove, due codici nuovi, due
messaggi nel catalogo.

**Prima il documento**: la voce in `08-decisioni.md` si scrive prima del codice, perché
dice anche che cosa l'attivazione **non** garantisce.

##### `U5` — Il form smette di produrre quote sbagliate (`R18`)

`AddPartyDialog`: la quota proposta è il **residuo** — la stessa aritmetica di
`remaining_share`, che il client può fare perché ha già l'elenco delle parti —
e `is_primary` si propone solo se un intestatario non c'è già. Nella modale dell'invito, la
quota che la nuova parte riceverà va **mostrata**, perché oggi la si scopre dopo.

Da qui in avanti una quota a zero si vede **mentre** la si crea, che è l'unico momento in
cui costa poco.

##### `U6` — L'elenco dice quale contratto è quale (`R14`)

`property_label` e `room_name` in `LeaseResponse`, con il `LEFT JOIN` nella query
dell'elenco e del dettaglio; la colonna nell'elenco del client diventa *immobile · stanza*
invece del tipo.

##### `U7` — L'avviso di sovrapposizione (`R12`)

Solo client, in `NewLeasePage`: `listLeases` sull'immobile, confronto dei periodi sui tre
stati che i vincoli coprono, conferma in più se si sovrappone.

### Trappole

**Non si toccano i vincoli di esclusione.** Allargarli a `draft` vieterebbe di abbozzare
due ipotesi alternative sullo stesso periodo, che è lavoro normale.

**`R17` non ha una migration**, e chi ne scrive una sta risolvendo il problema sbagliato:
`deleted_at`, `left_at` e la policy `lease_parties_update` ci sono già dalla `0014`.

**Gli indici unici sono parziali su `deleted_at IS NULL`.** Togliere una parte libera
l'intestatario principale e il legame con l'utente: è voluto, e va saputo prima di
stupirsene.

**Il join dei nomi è `LEFT`.** Con un `INNER` spariscono dall'elenco tutti i contratti
sull'immobile intero, che non hanno una stanza.

**L'email di una parte collegata non si tocca**: è ciò a cui l'invito è intestato.

**`remaining_share` non va riscritta.** Ha già la forma giusta, e il suo commento assume
`R17`: quello che manca è la capacità che presuppone.

**L'invariante nuova non sana i contratti già rotti.** Quelli si correggono a mano con
`U2`, e va scritto — o qualcuno cercherà una migration di bonifica che non esiste.

**`R26` si chiude solo per un terzo.** Il filtro dei contratti smette di essere morto; gli
immobili archiviati e le bollette pagate restano, e la voce va aggiornata invece di essere
tolta.

### Come si saprà che è finita

- **il vicolo cieco non esiste più**, e si prova percorrendolo: si crea un contratto, si
  sbaglia la quota, si corregge, si attiva. Un test lo fa da capo a fondo;
- una bozza sbagliata si **annulla**, e il filtro «Annullato» la trova;
- l'attivazione rifiuta una quota a zero, con un messaggio italiano e non un codice;
- una parte aggiunta per sbaglio si toglie; una collegata a un inquilino no, e il rifiuto
  dice perché;
- l'elenco dei contratti dice **quale casa e quale stanza**;
- la creazione avverte prima, e il `409` all'attivazione resta com'è;
- `just ci` verde, con i test nuovi sui due rami di ogni rifiuto.

### Che cosa **non** entra

- **il subentro**: cambiare la composizione di un contratto attivo. Va aperto come voce
  nuova in [12](12-da-fare.md), con il ragionamento della decisione 5;
- **far uscire un inquilino** (`left_at`): è parente di [`R16`](12-da-fare.md#lelenco), che
  è la sezione 6;
- **la colonna `label`** dei contratti: resta aperta, ed è l'unica via per «*Rossi, secondo
  piano*»;
- **le altre due facce di `R26`**: immobili archiviati e bollette pagate;
- **una bonifica dei dati già rotti.** Si correggono a mano, ed è precisamente la capacità
  che questa sezione consegna.

<a id="esito-sezione-3"></a>

## Esito — Sezione 3: contratti, correggere gli errori

Eseguita il **9 settembre 2026**, subito dopo la [sezione 1](#esito-sezione-1) e con il
collaudo della [sezione 2](#esito-sezione-2) rinviato — la sezione 3 è una delle ragioni
per cui è stato rinviato, perché riscrive i punti che quel giro attraversa. Chiude `R13`, `R17`, `R18`, `R12` e `R14`.

**Il vicolo cieco non esiste più**, e la prova non è una descrizione: è un test che lo
percorre — si crea un contratto, si sbaglia la quota, l'attivazione rifiuta, si toglie la
parte di troppo, si corregge quella buona, si attiva.

### Che cosa è entrato

| Passo | Esito |
|---|---|
| `U1` | `POST /leases/{id}/cancel`. La transizione **c'era già** — `Lease::cancel`, con `LeaseNotCancellable` a guardia — e non la chiamava nessuno |
| `U2` | `PATCH /lease-parties/{id}`: nome, telefono, ruolo, quota, intestatario. L'email **solo finché la parte non ha un utente** |
| `U3` | `DELETE /lease-parties/{id}`: cancellazione soft, e rifiuto se dietro c'è una persona registrata |
| `U4` | Due invarianti nuove: l'attivazione rifiuta **una quota a zero**, e su un contratto non più in bozza **non si aggiungono parti**. Prima il documento: [ADR-046](08-decisioni.md#adr-046) |
| `U5` | Il form propone **il residuo** e non cento, offre l'intestatario solo se il posto è libero, e l'invito **dice quale quota** riceverà chi accetta |
| `U6` | `property_label` e `room_name` nelle due letture, con un `LEFT JOIN` e **una query per pagina**; l'elenco mostra la casa e la stanza al posto del tipo |
| `U7` | L'avviso di sovrapposizione alla creazione, con una conferma in più. Solo client: il `409` all'attivazione resta l'autorità |

**Il backend cresce di tre rotte e nessuna migration**, che era la previsione del piano e
si è rivelata esatta: `deleted_at`, `left_at` e la policy `lease_parties_update` erano già
nella `0014`.

I test Rust passano da **424 a 428**, il client da **73 a 86**.

### Che cosa il piano non aveva previsto

**`LeaseResponse` non aveva un posto dove mettere i nomi, e l'aggregato non deve
averlo.** Il piano diceva «due campi in più nella risposta» come se fosse un join dentro
la query dell'elenco: non lo è, perché quella query costruisce l'aggregato `Lease`, e il
nome dell'immobile appartiene all'immobile. È diventato un **modello di lettura** —
`LeaseLabels` nella porta, una query sola per l'intera pagina, e la composizione nello use
case, che è la forma che `LeaseDetail` usava già per le parti. Costo reale: una porta, una
query, un tipo applicativo e due campi opzionali nel DTO.

**Il `LEFT JOIN` serviva più di quanto il piano dicesse.** La trappola parlava dei
contratti sull'immobile intero, che non hanno una stanza. Ce n'è una seconda, e l'ha
trovata `R26` della sezione 2: un contratto il cui **immobile è archiviato** — la riga ha
`deleted_at`, e la query dei nomi la filtra come impone la regola 10 di `CLAUDE.md`. Con
un `INNER JOIN` sarebbero spariti dall'elenco *tutti* i contratti di ogni immobile
archiviato. Il campo resta assente, e il client mostra «Immobile archiviato» invece di uno
spazio vuoto.

**La quota tornava con due scale diverse.** `PATCH` rispondeva `"100"` e la rilettura
`"100.00"`: la scala di un `Decimal` dipende da come è stato costruito. `format_amount`
esisteva già con questo problema scritto nel suo commento, e la quota non lo usava — ora
sì, e vale anche per la risposta all'aggiunta, che aveva lo stesso difetto da prima di
questa sezione.

**Il client faceva già la cosa giusta, e non bastava.** `AddPartyDialog` era offerto solo
sulle bozze, ma l'API accettava una parte su un contratto attivo: è la stessa distinzione
di [ADR-042](08-decisioni.md#adr-042) — che cosa si offre non è un confine — e questa volta
il confine mancava davvero.

### Decisioni prese eseguendo, che il piano non aveva sciolto

**La correzione di una parte non ricontrolla la somma delle quote.** È deliberato e senza
sarebbe impossibile rimediare: per portare due parti da `100 / 0` a `50 / 50` bisogna
passare da uno stato in cui la somma non fa 100. Sta in [ADR-046](08-decisioni.md#adr-046)
fra ciò che la regola **non** garantisce.

**La correzione non guarda lo stato del contratto.** I contratti rotti che esistono in
database sono **attivi**: rifiutare lì lascerebbe irreparabili proprio quelli, cioè tutti
quelli per cui la sezione esiste.

**La query dei nomi filtra `deleted_at IS NULL` sulle righe collegate**, come impone la
regola 10, e quindi un immobile archiviato lascia il contratto senza nome. L'alternativa
— leggere il nome anche di una riga archiviata — è più comoda e infrange una regola
vincolante: si è scelta la regola, e il client dice che cosa è successo.

### Che cosa resta aperto

- **`R26` si chiude per un terzo.** Il filtro «Annullato» dei contratti adesso trova
  qualcosa, e il test lo verifica. Gli immobili archiviati e le bollette pagate restano;
  la voce è aggiornata, non tolta;
- **il subentro** — cambiare la composizione di un contratto attivo — è la conseguenza
  diretta di `U4`, ed è una voce nuova in [12](12-da-fare.md);
- **la colonna `label`** dei contratti resta aperta: è l'unica via per chiamarne uno
  «*Rossi, secondo piano*», e il join risolve «sono tutti uguali», che era la lamentela;
- **far uscire un inquilino** (`left_at`) non è entrato, ed è parente di `R16`;
- **niente bonifica dei dati già rotti**: si correggono a mano, ed è la capacità che
  questa sezione consegna.

### Che cosa non è stato provato a mano

Tutto: questa sezione è stata scritta e verificata **solo con i test**. Il giro in undici
punti attraversa esattamente queste schermate — i punti 3, 4 e 5 — ed è il momento in cui
il lavoro qui si vede davvero: è rinviato alla sezione **5-bis**, dopo la 4 e la 5, e la
ragione sta nell'[esito della sezione 2](#esito-sezione-2).

---

<a id="piano-sezione-4"></a>

## Piano — Sezione 4: gli stati che nessuno aggiorna

Pianificata il **9 settembre 2026**. È la quarta dell'
[ordine di esecuzione](12-da-fare.md#lordine-di-esecuzione) e chiude `R1`, `R11` e `R25`.

Le tre voci sono **la stessa domanda posta tre volte** — *derivato o scritto a mano?* — e
stanno insieme per questo. Ma le tre risposte **non sono uguali**, e il piano esiste
soprattutto per dire perché.

### Il criterio c'è già, e non va inventato

[ADR-019](08-decisioni.md#adr-019) ha risposto a questa domanda una volta, per i
contratti, e la formula che usa è il criterio di tutta la sezione:

> La scadenza non è una decisione: è una conseguenza, e le conseguenze si derivano.

Lo stato memorizzato cambia **solo su azioni esplicite**; ciò che discende da altri fatti
si calcola al momento della lettura. `expiring` ed `expired` restano nell'enum — una riga
scritta a mano potrebbe contenerli — ma l'applicazione non li scrive mai, e il filtro li
rifiuta con `422` invece di restituire zero righe.

Applicare quel criterio alle tre voci le separa da sole:

| Valore | Che cos'è | Risposta |
|---|---|---|
| stanza `unavailable` | una stanza in ristrutturazione: **un giudizio** | memorizzato |
| stanza `occupied` | c'è un contratto in essere su di essa: **una conseguenza** | derivato |
| immobile `draft`, `available`, `maintenance` | **giudizi** di chi amministra | memorizzati |
| immobile `rented`, `partially_rented` | quante stanze sono occupate: **una conseguenza** | derivati |
| bolletta `paid` | «l'ho pagata al fornitore»: un fatto **fuori dal sistema** | memorizzato, e la capacità manca |
| bolletta `disputed` | l'inizio di un **processo**, non uno stato | resta fuori |

### Il fatto che il piano ha trovato, e che nessuno aveva in elenco

**`v_property_occupancy` conta una colonna che nessuno scrive, quindi dice sempre zero.**

```sql
count(r.id) FILTER (WHERE r.status = 'occupied')  AS rooms_occupied,
CASE WHEN count(r.id) > 0
     THEN round(100.0 * count(r.id) FILTER (WHERE r.status = 'occupied') / count(r.id), 2)
END                                               AS occupancy_percent,
```

`RoomStatus::Occupied` non lo scrive nessun use case e nessun trigger — è `R11` — quindi
`rooms_occupied` è **0** e `occupancy_percent` è **0,00** per ogni immobile del sistema,
sempre. È esattamente il difetto che [ADR-019](08-decisioni.md#adr-019) ha corretto sui
contratti — una vista che conta uno stato che mente — nella direzione opposta: là lo stato
era stantio, qui non viene scritto affatto.

**Nessuno se n'era accorto perché quella vista la legge solo un test**, e legge
`active_leases`, che è calcolata da `lease_in_force` e quindi è giusta. Le due colonne
sbagliate non le guarda nessuno: è un numero falso che aspetta il primo che lo mostri.

### Che cosa è stato deciso prima di pianificare

| | Scelta | Alternativa scartata |
|---|---|---|
| Come si sa se una stanza è occupata | **derivata alla lettura**, da `app.lease_in_force` | una colonna aggiornata da un trigger |
| Dove vive la derivazione | nel **modello di lettura**, in SQL | in un metodo dell'aggregato, come `Lease::effective_status` |
| Lo stato dell'immobile | `status` memorizzato **più** `effective_status` derivato | un solo campo, con il derivato che vince |
| Filtrare per uno stato derivato | `422`, come per i contratti, più un filtro booleano | far finta che si possa, restituendo zero righe |
| La bolletta pagata | una **transizione a mano**: `POST /utility-bills/{id}/pay` | una riga in `app.payments` |
| `disputed` | **non si espone**, e l'opzione esce dal filtro | esporlo come secondo flag |

### Decisioni, con il perché

#### 1. L'occupazione è derivata, e un trigger andrebbe stantio a mezzanotte

È la decisione che regge tutta la sezione, e l'argomento non è di eleganza.

Un trigger su attivazione e disdetta sembra la via ovvia: due eventi, due `UPDATE`, e la
colonna resta vera. Non resta vera, perché **l'occupazione dipende dal calendario**: un
contratto attivato oggi che comincia il primo del mese prossimo non occupa niente adesso,
e un contratto in essere fino a domani smette di occupare **senza che nessuno tocchi
niente**. Un trigger non scatta al passare del tempo. Servirebbe uno scheduler notturno —
che è precisamente ciò che [ADR-019](08-decisioni.md#adr-019) ha già scartato: un
componente da far girare, monitorare e recuperare dopo un fermo, che apre comunque una
finestra in cui il database dice una cosa e il calendario un'altra.

**La definizione di «in essere» è già una sola**, `app.lease_in_force`, e la `0030` porta
già scritto perché non se ne scrivono altre. La derivazione dell'occupazione la usa e non
la riscrive.

#### 2. Questa derivazione **non è pura**, e quindi non sta nell'aggregato

`Lease::effective_status` si calcola dalle date che il contratto ha addosso: è una funzione
pura dell'aggregato, e per questo vive nel dominio.

L'occupazione di una stanza no: dipende da **altre righe** — i contratti che la citano — e
lo stato di un immobile dipende dalle sue stanze. Un metodo sull'aggregato dovrebbe
ricevere in ingresso ciò che l'aggregato non ha, e chi lo chiama dovrebbe ricordarsi di
caricarlo: è il modo in cui una regola diventa vera in un punto e falsa in un altro.

Vive quindi nel **modello di lettura**, in SQL accanto a `lease_in_force` e
`property_residents`, ed esce dal repository insieme alla riga — come i nomi dei contratti
della sezione 3, che hanno già questa forma.

*Alternativa scartata: copiare `effective_status` di `Lease` alla lettera.* Sembra
coerenza, e sarebbe una firma che mente: `Room::effective_status(&self)` non può rispondere
senza i contratti.

#### 3. Due campi sul filo, non uno

`status` resta ciò che qualcuno ha deciso; `effective_status` è ciò che è vero adesso. È la
forma che i contratti hanno già, e chi legge l'API la riconosce.

*Alternativa scartata: un campo solo, con il derivato che copre il memorizzato.* Perde
l'informazione che serve a chi amministra: una stanza `unavailable` **e** occupata è un
errore da vedere — qualcuno l'ha messa in ristrutturazione mentre ci abita una persona — e
con un campo solo diventa invisibile.

#### 4. Un filtro su uno stato derivato risponde `422`, e accanto c'è un booleano

Identico ai contratti: `?status=rented` non può funzionare, perché in database quel valore
non c'è, e **restituire zero righe è il modo peggiore di dirlo**. Il codice esiste già —
`lease.derived_status_filter` — e ne serve uno per famiglia.

Accanto si offre il filtro che serve davvero, come `in_force` fa per i contratti:
`?occupied=true`. Quello è esprimibile in SQL, perché la derivazione è in SQL.

**Ne segue che due terzi di [`R26`](12-da-fare.md#lelenco) si chiudono qui**: le quattro
opzioni morte del filtro degli immobili diventano tre valide e due rifiutate con un
messaggio, e la voce va aggiornata invece di essere tolta — resta il terzo delle bollette,
che questa sezione chiude a sua volta togliendo `disputed` dal filtro.

#### 5. La bolletta pagata è una transizione a mano, non un pagamento

`app.payments` non è il posto, e ci sono tre ragioni concrete:

- `app.payable_type` è un **enum** con tre valori — `charge`, `expense_share`,
  `settlement` — e aggiungerne un quarto è una migration più il trigger polimorfico;
- un pagamento ha `payer_user_id` e `payee_user_id`: **un fornitore non è un utente**, e
  metterlo a `NULL` significherebbe una riga di pagamento che non dice a chi sono andati i
  soldi;
- tutto quel modulo è **denaro che entra** — un addebito che viene saldato — e questo è
  denaro che esce. È registrato da tre fette fra ciò che resta fuori, e non è questa
  sezione a deciderlo.

Quindi: `POST /utility-bills/{id}/pay`, che scrive lo stato e la data. È la stessa forma di
`validate` e `void`, che sono già sotto-risorse e non un `PATCH` sullo stato.

**Da `allocated` e da `validated`**, non da qualunque stato: una bolletta in bozza o in
revisione non è ancora una bolletta, e una annullata non si paga. Se una bolletta si può
pagare *prima* di riaddebitarla è la domanda da porsi eseguendo — la risposta probabile è
sì, perché il fornitore non aspetta il riaddebito.

#### 6. `disputed` non si espone, e l'opzione esce dal filtro

Una contestazione non è uno stato: è un **processo** con una ragione, una data di apertura,
un esito e — se va bene — una nota di credito. Esporla come flag darebbe un'etichetta senza
nessuna delle cose che la rendono utile, e la prima persona che la usa si aspetta di poter
dire *perché*.

Il valore resta nell'enum, come `expiring` per i contratti. Ciò che si toglie è l'opzione
dal filtro del client, che oggi non può corrispondere a niente.

### Decomposizione

##### `V1` — La migration: la vista che mente, e la derivazione che manca

Una migration nuova — **append-only**, la vista si sostituisce con `CREATE OR REPLACE` in
un file nuovo — che:

- corregge `v_property_occupancy`, che oggi conta `r.status = 'occupied'` e risponde
  sempre zero;
- introduce la derivazione dell'occupazione di una stanza, accanto a `lease_in_force` e con
  la stessa forma: una funzione `STABLE`, con il confine dentro come fa `property_residents`.

**Prima di scrivere**: rileggere la `0030`, che è il precedente più vicino e spiega perché
la funzione proietta invece di esporre una vista, e perché `security_invoker` e
`SECURITY DEFINER` non sono intercambiabili.

##### `V2` — Le due letture portano lo stato effettivo

Stanza e immobile escono dal repository con il derivato accanto al memorizzato, e i DTO lo
espongono come `effective_status` — stessa parola dei contratti, che è già nel vocabolario
di chi legge l'API.

##### `V3` — I filtri: `422` sul derivato, booleano accanto

Un codice per famiglia — `property.derived_status_filter`, `room.derived_status_filter` —
e i filtri `?occupied=true` su stanze e immobili. Il messaggio dice **quale filtro usare**,
come fa già quello dei contratti.

##### `V4` — Il `<Select>` dello stato, dove è un giudizio

`EditPropertyDialog` offre `draft`, `available`, `maintenance` — e **non** `archived`, che
ha la sua operazione e che il dominio rifiuta via `PATCH`
(`PropertyArchiveRequiresDedicatedOperation`), né i due derivati.

La stanza prende `available` e `unavailable` nella **sola** modale di modifica: nessuno
crea una stanza già in ristrutturazione, ed è la ragione per cui la sezione 1 aveva
lasciato `status` fuori dal form di creazione — quella scelta si conferma, e adesso ha una
ragione scritta invece di un rinvio.

##### `V5` — La bolletta si dichiara pagata

`POST /utility-bills/{id}/pay`, con la data. Il client lo offre dalla scheda, e l'elenco
perde l'opzione `disputed` dal filtro.

##### `V6` — Il client mostra il derivato

I badge di stanza e immobile mostrano `effective_status` e, quando i due divergono,
**dicono entrambi**: «Non disponibile · occupata» è un'anomalia che chi amministra deve
vedere, non un dettaglio da nascondere dietro la precedenza di un campo sull'altro.

### Trappole

**Il trigger sembra la via ovvia e va stantio con il calendario.** È la trappola numero
uno di questa sezione: due `UPDATE` su attivazione e disdetta sembrano bastare, e non
bastano perché nessuno scatta a mezzanotte quando un contratto comincia.

**`app.lease_in_force` è l'unica definizione di «in essere».** Confrontare le date a mano
crea la seconda definizione che quella funzione esiste per impedire — la `0030` lo dice
già, e vale identico qui.

**Le migration sono append-only.** La vista non si modifica dove sta: si scrive un file
nuovo con `CREATE OR REPLACE VIEW`.

**Lo stato derivato non si scrive mai.** `occupied`, `rented` e `partially_rented` restano
nell'enum e nessuno li mette in una `UPDATE`: è la regola di ADR-019, e il modo in cui si
viola è «tanto lo aggiorno anche qui, per sicurezza».

**`R26` si chiude per due terzi qui**, non del tutto: la voce va aggiornata, e ciò che
resta è il filtro «Archiviato» degli immobili — che è vuoto per una ragione diversa da
tutte queste, cioè perché archiviare mette `deleted_at`.

**Il conteggio delle stanze occupate cambia dei numeri che qualcuno potrebbe già
guardare.** Oggi `occupancy_percent` è zero ovunque: quando comincia a dire la verità, un
grafico che lo mostrasse cambierebbe di colpo. Nessuno lo mostra — va verificato prima di
correggere la vista, non dopo.

### Come si saprà che è finita

- una stanza con un contratto in essere si legge **occupata**, e senza nessuno torna
  disponibile **senza che nessuno tocchi niente** — un test con due date lo prova;
- una stanza in ristrutturazione resta `unavailable` anche se occupata, e l'interfaccia
  mostra tutte e due le cose;
- un immobile esce da `draft`, e i due stati che dipendono dai contratti non si scrivono a
  mano;
- `?status=rented` risponde `422` dicendo quale filtro usare, e `?occupied=true` funziona;
- `v_property_occupancy` non dice più zero: un test la interroga su un immobile con una
  stanza occupata;
- una bolletta si dichiara pagata, e `disputed` non compare più fra i filtri;
- `just ci` verde, con i test nuovi sulle **date** e non solo sugli stati.

### Che cosa **non** entra

- **il pagamento vero al fornitore**: denaro in uscita è un modulo che non esiste, e
  `app.payments` è costruito per il denaro che entra;
- **la contestazione come processo**: ragione, esito, nota di credito. Se serve, è una
  voce nuova;
- **il terzo di `R26` sugli immobili archiviati**, che ha una causa sua;
- **lo stato del contratto**, che è già derivato e non si tocca.

<a id="esito-sezione-4"></a>

## Esito — Sezione 4: gli stati che nessuno aggiorna

Eseguita il **9 settembre 2026**. Chiude `R1`, `R11` e `R25`.

Il criterio non è stato inventato: [ADR-019](08-decisioni.md#adr-019) l'aveva già dato —
*le conseguenze si derivano, le decisioni si memorizzano* — e applicarlo ha separato le
tre voci da solo.

### Che cosa è entrato

| Passo | Esito |
|---|---|
| `V1` | Migration `0033`: `app.room_is_occupied`, `app.property_occupancy_status`, e `v_property_occupancy` che **smette di dire zero a tutti** |
| `V2` | Stanze e immobili escono dalle letture con `effective_status` accanto a `status` |
| `V3` | `?status=rented` risponde `422` con `property.derived_status_filter`, e accanto c'è `?occupied=true` |
| `V4` | Le due tendine, dove lo stato è un giudizio: immobile `draft`/`available`/`maintenance`, stanza `available`/`unavailable` |
| `V5` | Migration `0034` e `POST /utility-bills/{id}/pay`: la bolletta si dichiara pagata, con la data |
| `V6` | I badge mostrano il derivato **e** il dichiarato quando divergono, come già fanno i contratti |

**Due migration e non una**, ed è la divisione giusta: la `0033` deriva ciò che non si può
memorizzare, la `0034` memorizza ciò che non si può derivare — una bolletta pagata al
fornitore è un fatto che accade fuori dal prodotto, e nessuna query lo sa.

I test Rust passano da **428 a 431**.

### Il numero falso che nessuno guardava

`v_property_occupancy` contava `r.status = 'occupied'`, che è la colonna che `R11` dice non
essere scritta mai: `rooms_occupied` e `occupancy_percent` erano **zero per ogni immobile,
sempre**, da quando la vista esiste. Nessuno se n'era accorto perché quella vista la legge
un test solo, e leggeva `active_leases` — l'unica colonna giusta, perché passa da
`lease_in_force`.

È lo stesso difetto che ADR-019 ha corretto sui contratti, rovesciato: là lo stato era
stantio, qui non veniva scritto affatto. Adesso un test lo guarda su un immobile con una
stanza davvero occupata.

### La prova che un trigger sarebbe stato sbagliato

È un test, e vale più di un ragionamento: due contratti **entrambi attivi**, uno in essere
da dieci giorni e uno che comincia **fra un mese**. La prima stanza risulta occupata, la
seconda no. Un `UPDATE` all'attivazione le avrebbe marcate tutte e due, e nessun trigger
sarebbe scattato la notte in cui il secondo contratto comincia davvero.

### Che cosa il piano non aveva previsto

**Il confine dentro la funzione rompe la lettura dal pool privilegiato.**
`app.room_is_occupied` è `SECURITY DEFINER` e porta `can_access_property` nel corpo, come
`app.property_residents` della `0030`. Sul pool privilegiato — che non ha contesto — quel
confine risponde «no» e la vista conta zero: il test che la interroga ha dovuto installare
`app.current_org_id` **e** `app.current_user_id`, perché `can_access_property` passa da
`is_org_staff`, che chiede il ruolo di *quella* persona in *quella* organizzazione. Non è
un difetto: è la ragione per cui una funzione che legge senza policy deve controllare da sé
chi la chiama, e ora è scritto in un test invece che in un commento.

**Una stanza è occupata anche da un contratto sull'immobile intero.** Non era nel piano e
sarebbe stato un secondo zero: chi affitta tutto l'appartamento occupa ogni stanza che c'è
dentro, e contare solo i contratti che nominano la stanza avrebbe detto che un
appartamento interamente affittato ha zero stanze occupate.

**`app.utility_bills` non aveva una colonna per la data del pagamento.** Il piano diceva
«scrive lo stato e la data» dandola per esistente. Da qui la seconda migration, con il
vincolo `utility_bills_paid_ck` che ricalca `utility_bills_validated_ck`: uno stato che
dichiara un fatto senza la data del fatto non dice quando è successo.

### Decisioni prese eseguendo

**Chi ha dichiarato pagata una bolletta non diventa una colonna.** `validated_by` esiste
perché la validazione è il momento in cui una persona si prende la responsabilità dei
numeri che una macchina ha letto — un fatto sul dato. Dichiarare pagata è un fatto sul
denaro, e chi l'ha fatto sta in `audit_log` come ogni altra scrittura.

**La dichiarazione non si disfa.** Se il pagamento non è mai partito, ciò che va corretto è
la dichiarazione, e correggere una dichiarazione è un'operazione che questo modulo non ha —
come non ha la contestazione. Meglio non offrirla che offrirla a metà.

**`archived` resta fra le opzioni del filtro degli immobili**, e continua a non poter
corrispondere a niente: è il terzo di `R26` che questa sezione non chiude, ed è scritto nel
codice del client accanto all'elenco delle opzioni perché chi lo legge non lo prenda per
una svista.

### Che cosa resta aperto

- **`R26` si chiude per due terzi**: le opzioni derivate degli immobili e `disputed` sulle
  bollette escono dai filtri. Resta «Archiviato», che è vuoto per una ragione tutta sua;
- **la contestazione come processo** — ragione, esito, nota di credito — non è entrata, e
  se serve è una voce nuova;
- **il pagamento vero al fornitore**: denaro in uscita resta un modulo che non esiste;
- **una data scelta da chi dichiara**: oggi il pagamento è «adesso». Una bolletta pagata la
  settimana scorsa e registrata oggi è un caso che questo modulo non tratta, e con la data
  scelta arriverebbe la domanda su quanto indietro si possa andare.

### Che cosa non è stato provato a mano

Tutto, come per la sezione 3: solo test. E qui pesa di più, perché due schermate cambiano
sotto gli occhi — la scheda dell'immobile e quella della bolletta — e i punti **2** e **6**
del collaudo le attraversano. Sono anche i due punti che la sezione 5-bis rimetterà in
piedi.

---

<a id="piano-sezione-5"></a>

## Piano — Sezione 5: le segnalazioni

Pianificata il **9 settembre 2026**. È la quinta dell'
[ordine di esecuzione](12-da-fare.md#lordine-di-esecuzione) e chiude `R19`, `R19-bis`,
`R21`, `R22`, `R23` e `R24`.

Tre voci su sei erano già **decise** in [12](12-da-fare.md) e non si ridiscutono: la
visibilità diventa una scelta di chi apre (`R19`), chi ha segnalato modifica finché
nessuno ha preso in carico (`R19-bis`), e `R20` è chiusa dalla sezione 1. Il piano
esiste per le tre che restano aperte e per **come** le tre decise si eseguono, che è la
parte in cui questa sezione può sbagliare in silenzio.

Le sei voci non sono la stessa domanda posta sei volte, come lo erano nella
[sezione 4](11-piani-conclusi.md#piano-sezione-4). Sono **tre coppie**:

| Coppia | Voci | Che cosa hanno in comune |
|---|---|---|
| chi vede, e chi tocca | `R19`, `R19-bis` | portano **due migration**, e la seconda **revoca una decisione già presa** |
| chi è | `R21`, `R22` | è il problema dei nomi, e la risposta l'ha già data la sezione 3 |
| che cosa si può ancora fare | `R23`, `R24` | due regole di dominio che mancano, e la seconda esiste **perché** la prima chiude una porta |

### Tre cose che pianificare ha trovato, e che il backlog non diceva

Sono la ragione per cui questo piano è più lungo delle sue sei righe di partenza.

#### 1. `R19-bis` non è «una rotta in più»: contraddice tre cose che la CI verifica

`PATCH /tickets/{id}` è `update_costs`, e questo il backlog lo dice. Quello che non dice
è che **titolo e descrizione non sono modificabili per privilegio di colonna**, non per
scelta applicativa, e che quel privilegio è verificato in tre posti diversi:

- la migration [`0020`](../migrations/0020_ticket_column_privileges.sql) toglie a
  `propflow_app` l'`UPDATE` sull'intera tabella e la riconcede su nove colonne.
  `title` e `description` restano deliberatamente fuori, con la ragione scritta:
  *«una segnalazione aperta sull'immobile sbagliato si annulla e si riapre»*;
- il controllo `colonna-strutturale-scrivibile` di
  `ops/sql/checks/schema_invariants.sql` **fallisce** se `propflow_app` acquista
  l'`UPDATE` su `tickets.title` o `tickets.description`. È fra le invarianti che
  `just ci` esegue;
- il test `the_application_role_cannot_move_a_ticket_to_another_property`
  (`crates/infrastructure/tests/persistence.rs`) **asserisce** che
  `UPDATE app.tickets SET title = 'riscritto'` sia rifiutato con `permission denied`.

E la ragione è scritta anche in [ADR-026](08-decisioni.md#adr-026), in
`docs/05-api.md`, nel commento di `Ticket::apply_costs`, nel commento di
`UpdateTicketCostsRequest` e nella stringa `t.ticket.onlyCostsArePatchable`.

Ne segue che `R19-bis` **non si può eseguire senza revocare una decisione documentata**,
e che il modo giusto di revocarla è una voce in [08](08-decisioni.md) *prima* del codice,
non una riga tolta di soppiatto da un elenco di controllo. Chi arrivasse qui aspettandosi
«uno use case e una rotta» scoprirebbe il vincolo al primo `just ci`, cioè dopo aver già
scritto tutto.

#### 2. La metà economica di `R22` **è già fatta**, e quel che resta è tutto l'altra

Il backlog dice che il ruolo si deduce da chi è staff e che «da solo risolve gran parte
della domanda». Il ruolo **c'è già sul filo e già a schermo**: `TicketMessageResponse`
porta `author_kind`, e `MessageRow` in `TicketDetailPage.tsx` stampa
`t.ticket.actors[message.author_kind]` — *Inquilino*, *Proprietario*, *Gestore*. Lo
stesso vale per «Aperta da», che mostra `reported_by_kind`.

Quindi la separazione che il backlog proponeva — prima il ruolo, poi il nome — non ha
una prima metà da fare. E il caso che resta scoperto è **esattamente quello del
co-living**: due coinquilini nel thread compaiono tutti e due come *Inquilino*, e la
domanda «me lo dice il proprietario o un coinquilino?» ha già risposta mentre «quale
coinquilino?» non ne ha nessuna. `R22` è quindi solo il nome, e va eseguita con `R21`
invece che prima.

#### 3. `ticket_status_history.reason` è una colonna che nessuno scrive, e il client la mostra già

È lo stesso difetto che la [sezione 4](11-piani-conclusi.md#esito-sezione-4) ha trovato
in `v_property_occupancy`, in forma più piccola. La colonna esiste dalla `0007`,
`TicketStatusChange::reason()` la legge, `TicketStatusChangeResponse` la serializza e
`HistoryRow` la renderizza — ed è **sempre `null`**, perché la scrive il trigger
`tg_ticket_status_history`, che non ha modo di riceverla: `app.ticket_status_history`
non ha nemmeno una policy di `INSERT`.

Conta qui perché è il posto in cui verrebbe naturale mettere il motivo di `R24`. Non ci
va, e la decisione 6 dice perché.

### Che cosa è stato deciso prima di pianificare

| | Scelta | Alternativa scartata |
|---|---|---|
| Dove va il ramo di visibilità (`R19`) | dentro **`app.can_access_ticket`**, sostituita con `CREATE OR REPLACE` | un `ALTER POLICY tickets_select`, che mostrerebbe la segnalazione e non il suo thread |
| Il predicato «chi guarda abita qui» | **`app.property_residents(property_id)`**, già introdotta dalla `0030` | `app.shares_home_with`, che è utente↔utente e non filtra `lease_in_force` |
| La forma di `R19-bis` | una **rotta a sé**, `PATCH /tickets/{id}/report` | allargare `PATCH /tickets/{id}`, che avrebbe due autorizzazioni secondo i campi presenti |
| Chi corregge | **lo staff sempre, chi ha segnalato finché è `open`** | solo chi ha segnalato, che lascerebbe un refuso incorreggibile |
| I nomi (`R21`, `R22`) | un **modello di lettura** che risolve gli identificatori dalle righe `app.users` che la RLS già concede | una rotta che risolva le persone, che [ADR-043](08-decisioni.md#adr-043) ha già scartato |
| Quando manca il nome | **etichetta neutra dal catalogo**, mai l'UUID | mostrare l'identificatore, che è una resa |
| Messaggi dopo la chiusura (`R23`) | **vietati in tutti e tre gli stati terminali** | ammessi su `closed`, che renderebbe la riapertura facoltativa |
| Il motivo della riapertura (`R24`) | **obbligatorio**, e diventa il primo messaggio del giro nuovo | una colonna sulla segnalazione, o `ticket_status_history.reason` |

### Decisioni, con il perché

#### 1. Il ramo di visibilità sta nella **funzione**, non nella policy

Il backlog dice «un `ALTER POLICY` con un ramo in più». È la via che viene in mente e
produce un difetto preciso: `tickets_select` non è l'unica policy che decide chi vede una
segnalazione. `app.can_access_ticket` è chiamata da **quattro** posti:

| Chiamante | Che cosa concede |
|---|---|
| `tickets_select` | la riga della segnalazione |
| `ticket_messages_select` | il thread (meno le note interne) |
| `ticket_status_history_select` | lo storico degli stati |
| `can_access_attachment('ticket', …)` | **le fotografie del guasto** |

Allargare la sola `tickets_select` darebbe a un coinquilino una segnalazione che si apre
e non si legge: niente thread, niente storico, niente foto, e nessun errore che spieghi
perché. Il ramo va quindi dentro `app.can_access_ticket`, con
`CREATE OR REPLACE FUNCTION` e la **stessa firma** — le policy la referenziano per oid,
quindi non vanno né toccate né ricreate.

**La conseguenza va detta ad alta voce e non è un effetto collaterale da scoprire:**
condividere una segnalazione condivide *anche* la sua conversazione e le sue fotografie,
e `ticket_messages_insert` passa dalla stessa funzione — quindi un residente può anche
**scrivere** nel thread. È voluto: la caldaia condominiale interessa a tutti, e una
visibilità che permetta di leggere «hanno chiamato il tecnico» ma non di rispondere
«da me non funziona ancora» sarebbe una bacheca, non una segnalazione condivisa.

**Che cosa la condivisione non dà**: `tickets_update` ha un predicato **in linea** —
staff, `reported_by`, `assigned_to` — e non passa da `can_access_ticket`. Un residente
non riapre, non annulla, non corregge e non chiude. Il confine di scrittura resta dov'è,
ed è la ragione per cui questa migration è più piccola di quanto sembri.

#### 2. Il predicato è `app.property_residents`, e la scelta non è di gusto

Il vincolo dice di **scegliere** fra ciò che la `0030` ha già introdotto invece di
scriverne un terzo. Le due candidate non sono equivalenti:

`app.shares_home_with(p_user_id)` è **utente↔utente**: risponde «questa persona e chi
chiama abitano la stessa casa». Per usarla servirebbe una persona da confrontare — chi ha
segnalato — e ne verrebbero due difetti. Il primo: `reported_by` è
`ON DELETE SET NULL`, quindi su una segnalazione il cui autore è stato cancellato il ramo
sparirebbe e la condivisione smetterebbe di valere **senza che nessuno abbia cambiato
niente**. Il secondo, peggiore: `shares_home_with` **non filtra `app.lease_in_force`**, e
il suo commento dice che è deliberato — è un *pavimento* per la leggibilità dei nomi, non
un elenco di chi ci abita adesso. Usarla qui darebbe a un inquilino uscito due anni fa la
visibilità sulle segnalazioni nuove della casa in cui non vive più.

`app.property_residents(p_property_id)` è **ancorata all'immobile**, che è la dimensione
giusta: la segnalazione ha un `property_id` obbligatorio. Filtra `lease_in_force`,
filtra `left_at IS NULL`, esclude i garanti, e ha già dentro `can_access_property` come
confine. Il ramo diventa:

```sql
OR (t.visible_to_residents
    AND EXISTS (SELECT 1 FROM app.property_residents(t.property_id) pr
                 WHERE pr.user_id = app.current_user_id()))
```

**`t.visible_to_residents` va scritto per primo e il ramo va per ultimo nell'`OR`.**
Non è formattazione: `can_access_ticket` è `COST 500` e viene valutata **per riga**
nell'elenco delle segnalazioni. Con il booleano davanti, la funzione che restituisce
righe non viene nemmeno chiamata sulle segnalazioni private, che sono il default e
saranno la maggioranza.

*Alternativa scartata: una terza funzione, `app.lives_in_property(p_property_id)`.*
Sarebbe più leggibile di un `EXISTS` su una funzione tabellare e sarebbe la **seconda
definizione** di «chi abita qui» che la `0030` esiste per impedire — la stessa trappola
che quel file documenta alla sua terza occorrenza.

#### 3. La colonna si chiama `visible_to_residents`, e il default è il comportamento di oggi

`ALTER TABLE app.tickets ADD COLUMN visible_to_residents BOOLEAN NOT NULL DEFAULT false`.

**Il default `false` è la decisione, non un ripiego**: `R19` dice che oggi non è un
difetto ma la policy, e che quel comportamento resta. Una segnalazione nasce privata; chi
la apre può dichiararla di interesse comune.

*Perché non `visible_to_all_tenants`, che è la parola del backlog*: «tutti gli inquilini»
si legge come «tutti gli inquilini dell'organizzazione», che sarebbe un confine diverso e
più largo — un inquilino di un'altra casa dello stesso proprietario non c'entra niente.
Il confine è **l'immobile**, e il nome della colonna deve dirlo, perché è la colonna che
qualcuno leggerà fra un anno senza questo documento davanti. La casella nel form invece
può e deve usare la parola comune: *«Visibile a chi abita in questo immobile»*.

La colonna entra nella `GRANT UPDATE` — è modificabile, vedi la decisione 4 — e **non**
entra fra le colonne strutturali del controllo 12.

#### 4. `R19-bis` è una rotta a sé, e si chiama `/report`

Il backlog nomina il pericolo esatto: *«una rotta con due autorizzazioni diverse a
seconda dei campi presenti — costi allo staff, testo a chi ha segnalato — che è il genere
di cosa che si sbaglia in silenzio»*. Il modo in cui si sbaglia in silenzio è concreto e
vale la pena scriverlo: un `PATCH` con la semantica del progetto ha campi **assenti**,
quindi l'autorizzazione dipenderebbe da quali chiavi ci sono nel JSON, e un campo
aggiunto un anno dopo senza toccare quel ramo sarebbe scrivibile da chi non deve.

Quindi: `PATCH /api/v1/tickets/{id}/report`, con **una sola** regola di autorizzazione
per l'intera rotta. Il corpo porta `title`, `description` e `visible_to_residents`, tutti
facoltativi con la semantica generale (assente = invariato); nessuno dei tre è
azzerabile, quindi non serve `explicit_null`.

**Perché un sostantivo e non un verbo.** Le sotto-risorse di questo modulo sono
transizioni — `/triage`, `/assign`, `/close` — e sono `POST` perché hanno effetti
propri. Questa non è una transizione: è la correzione di un attributo, e il metodo
giusto resta `PATCH`. `report` è ciò che si corregge — la segnalazione così come è stata
fatta — ed è la stessa cosa che la casella di visibilità decide. Tenere i tre campi
insieme non è comodità: è che sono **tutti e tre quello che chi ha segnalato ha
dichiarato**, e hanno per questo la stessa autorizzazione.

*Alternativa scartata: `PUT /tickets/{id}/report` con tutti e tre i campi obbligatori.*
Toglierebbe il problema dei campi assenti alla radice, e in cambio obbligherebbe chi
cambia solo la casella a rimandare indietro un testo che non ha toccato — cioè a
sovrascriverlo con una copia possibilmente vecchia. Il progetto non usa `PUT` da nessuna
parte, e non è questo il caso per introdurlo.

*Alternativa scartata: nessuna rotta, e la correzione passa dall'annullamento.* È
letteralmente ciò che la `0020` prescrive oggi — «si annulla e si riapre» — e su un
refuso nel titolo costa a chi segnala di rifare tutto e di perdere il riferimento
`TK-000042` che ha già comunicato.

**Chi corregge: lo staff sempre, chi ha segnalato finché è `open`.** È la forma di
`canceller` in `transition.rs`, e non va **copiata**: va estratta e condivisa, perché due
copie della stessa regola sono due regole che divergeranno alla prima modifica di una
delle due. Ne segue anche che i codici di rifiuto restano quelli già in catalogo —
`ticket.not_yours` a chi non ha segnalato, `org.staff_role_required` a chi ha segnalato
ma la segnalazione è già in lavorazione — con i due rimedi diversi che quella
distinzione esiste per dire.

Il dominio rifiuta su `cancelled` con `ticket.cancelled`, come fa già `apply_costs`: una
segnalazione annullata non si tocca più. Su una **chiusa** invece si corregge, e serve
davvero: la si rilegge mesi dopo, e un titolo sbagliato lì è un titolo sbagliato per
sempre.

#### 5. Il privilegio di colonna si allarga di tre colonne, e questo **emenda ADR-026**

`title`, `description` e `visible_to_residents` entrano nella `GRANT UPDATE` di
`propflow_app`. Le prime due erano fuori con una ragione scritta, e la ragione cambia
adesso perché **cambia il fatto**: la `0020` le teneva fuori dicendo che una segnalazione
sbagliata si annulla e si riapre, cioè che non esisteva un modo legittimo di correggerle.
`R19-bis` dice che quel modo deve esistere.

Il criterio di [ADR-026](08-decisioni.md#adr-026) **non cambia** e va riletto per
intero: lo strumento serve *«quando una colonna non ha un motivo legittimo di cambiare»*.
Quello che cambia è che ora ce l'ha. `property_id`, `room_id`, `lease_id`,
`reported_by`, `reference` e `currency` **restano fuori**, e la frase «si annulla e si
riapre» resta vera per loro: una segnalazione aperta sull'immobile sbagliato non si
sposta.

Ne segue con precisione che cosa il privilegio smette di garantire, e va detto perché è
il pezzo che si perde: dal momento del `GRANT`, un inquilino che sia `reported_by` può
riscrivere titolo e descrizione **da qualunque stato** con una `UPDATE` scritta a mano
sotto RLS, perché il limite «finché è `open`» è applicativo e una policy non può
esprimerlo — non può confrontare la riga nuova con quella vecchia, che è la constatazione
di ADR-021 e ADR-026. Resta vero ciò che la `0020` aveva già misurato: il confine di
isolamento tiene, e ogni `UPDATE` finisce in `audit_log`. È esattamente lo stesso
compromesso che vale oggi per `status` e `actual_cost`, che sono concesse e la cui
disciplina è applicativa.

**Due migration e non una**, come nella sezione 4 e per la stessa ragione: la prima
(`0035`) allarga chi *vede*, la seconda (`0036`) allarga chi *scrive*. Sono due decisioni
diverse, con due ADR diverse, e un solo file le farebbe rileggere come una cosa sola.

#### 6. Il motivo della riapertura è **il primo messaggio del giro nuovo**

`R23` e `R24` sono legate, e il verso del legame è quello che il backlog indica: se un
thread chiuso non riceve più messaggi, riaprire diventa l'unico modo di dire «non è
risolto», e allora il motivo non è un di più.

Da qui due conseguenze.

**Il motivo è obbligatorio.** `POST /tickets/{id}/reopen` oggi non accetta corpo; ne
accetterà uno con `reason` non vuoto. È l'asimmetria con `resolution_notes`, che è
facoltativo, e ha una ragione: la chiusura lascia comunque `closed_at` e una riga di
storico, mentre la riapertura senza motivo non lascia **niente** che spieghi perché la
segnalazione è tornata viva — e `R23` ha appena tolto l'unico canale alternativo.

**Il motivo va nel thread.** `ReopenTicket` scrive, nella stessa transazione, un
messaggio pubblico con quel testo, di chi ha riaperto. Tre ragioni:

- **il thread non si riscrive mai** — `app.ticket_messages` ha due sole policy — quindi
  il motivo sopravvive alla chiusura successiva, mentre `resolution_notes` viene
  cancellato dalla riapertura stessa e una colonna simmetrica lo sarebbe a sua volta;
- una segnalazione si riapre **più volte**, e tre riaperture hanno tre motivi. Una
  colonna ne tiene uno;
- chiude il cerchio di `R23` invece di lasciarlo aperto: chi ha segnalato e vuole dire
  «non è risolto» lo dice riaprendo, e le sue parole finiscono dove finiscono tutte le
  altre parole.

*Alternativa scartata: `ticket_status_history.reason`.* È la colonna fatta apposta, il
client la mostra già, e riempirla chiuderebbe il terzo fatto trovato più sopra. Costa
però un **secondo canale verso il trigger**: la riga la scrive
`tg_ticket_status_history`, che è `SECURITY DEFINER` perché quella tabella non ha policy
di `INSERT`, quindi il motivo dovrebbe arrivargli da una `set_config` transazionale —
il meccanismo di `app.current_user_id()`, che esiste e funziona. Si scarta per due
ragioni. La prima è una trappola vera: una `set_config` locale vale per **tutta** la
transazione, quindi una seconda transizione nella stessa richiesta erediterebbe il
motivo della prima, e il rimedio (un trigger che consuma il valore azzerandolo) è
macchina nuova per un campo di testo. La seconda è che `TicketRepository::update`
prende `&Ticket` e il motivo **non è un attributo dell'aggregato**: farcelo passare
vorrebbe dire o un campo che non appartiene alla segnalazione, o una firma diversa per
una transizione sola.

**`ticket_status_history.reason` resta quindi non scritta, e va detto invece di
lasciarlo scoprire.** Non è una svista ed è la differenza con `v_property_occupancy`: là
un numero *falso* veniva mostrato, qui un campo *vuoto* non compare. `HistoryRow` lo
renderizza già condizionalmente, quindi non c'è niente da togliere.

#### 7. Un thread chiuso non riceve messaggi, e il confine è il dominio

`PostTicketMessage` fa `ensure_member` e **non guarda lo stato**: il thread di una
segnalazione chiusa o annullata accetta scritture. È una regola di dominio che manca, e
nasconderla nel client lascerebbe la rotta aperta.

La regola: si scrive **esattamente nei sei stati di lavorazione**, cioè
`TicketStatus::is_working()`. È lo stesso predicato che `close` e `cancel` usano già come
guardia, e riusarlo invece di enumerare «non `closed` e non `cancelled`» copre da sé il
terzo caso — `resolved`, che l'applicazione non scrive mai ma che una riga scritta a mano
può contenere ([ADR-019](08-decisioni.md#adr-019) e la nota su `TicketStatus`).

Un invariante nuovo, un codice nuovo, un messaggio nel catalogo. **Leggere resta
permesso**: `ListTicketMessages` non cambia, e un thread chiuso si rilegge per intero.

*Alternativa scartata: ammettere le repliche su `closed` e vietarle solo su `cancelled` e
`resolved`.* Lascerebbe la riapertura facoltativa, e con essa il caso che il modulo è
fatto per evitare — una conversazione che continua sotto uno stato che dice «finita».

### Decomposizione

L'ordine è quello del rischio: prima le due migration, che sono la parte che non si
disfa, poi i nomi, poi le due regole di dominio. **Ogni passo che porta una decisione ha
la sua voce in [08](08-decisioni.md) scritta prima del codice**, non dopo.

##### `W1` — La visibilità diventa una scelta di chi apre (`R19`) — **ADR-047 prima**

Migration `0035_a_ticket_can_be_shared_with_residents.sql`:

- `ALTER TABLE app.tickets ADD COLUMN visible_to_residents BOOLEAN NOT NULL DEFAULT false`;
- `CREATE OR REPLACE FUNCTION app.can_access_ticket(uuid)` con il quinto ramo, **ultimo
  nell'`OR`** e con il booleano davanti all'`EXISTS`. Firma identica: le policy non si
  toccano;
- `GRANT UPDATE (visible_to_residents) ON app.tickets TO propflow_app`, nello stesso
  blocco `DO $$` con la guardia sul ruolo che le altre migration usano.

**Prima di scrivere**: rileggere la `0030`, che spiega perché una funzione che proietta
non è intercambiabile con una policy allargata, e la testa della `0020`, che dice quali
colonne restano fuori e perché.

Poi: il campo nell'aggregato e in `TicketParts`, la colonna in `select_ticket!()` e
nell'`INSERT`, `visible_to_residents` in `OpenTicketRequest` e in `TicketResponse`, la
casella nel form di apertura accanto a quella del contratto — che è dove la conseguenza
si spiega, come già fa `leaseNoneHint`.

##### `W2` — Correggere ciò che si è scritto (`R19-bis`) — **ADR-048 prima**

Migration `0036_a_report_can_be_corrected.sql`: `GRANT UPDATE (title, description)`, con
in testa il perché la `0020` le teneva fuori e che cosa è cambiato.

Nello **stesso passo**, e non in un momento successivo, perché sono le tre facce della
stessa revoca:

- `ops/sql/checks/schema_invariants.sql`: via le due righe `('tickets','title')` e
  `('tickets','description')`, e il commento del controllo 12 aggiornato — resta a dire
  perché le altre sei ci sono ancora;
- `crates/infrastructure/tests/persistence.rs`: le due `UPDATE` passano dal blocco dei
  rifiuti a quello delle concessioni, e il commento sopra cambia con esse. Il test
  continua a provare che `property_id`, `room_id`, `lease_id`, `reference` e
  `reported_by` restano negate;
- `docs/05-api.md`, il commento di `Ticket::apply_costs`, quello di
  `UpdateTicketCostsRequest` e la stringa `t.ticket.onlyCostsArePatchable`.

Poi il codice: `Ticket::correct(...)` nel dominio con il rifiuto su `cancelled`;
l'estrazione della guardia condivisa con `cancel`; lo use case `CorrectTicketReport`;
`PATCH /api/v1/tickets/{id}/report`; nel client il bottone «Correggi» sulla scheda, con
la stessa condizione dell'annullamento.

##### `W3` — Dire chi (`R21`, `R22`) — **ADR-049 prima**

Un modello di lettura, con la forma che la [sezione 3](11-piani-conclusi.md#esito-sezione-3)
ha già stabilito per i nomi degli immobili: una porta, **una query per pagina**, la
composizione nello use case, campi opzionali nel DTO.

- porta: risoluzione di un insieme di `UserId` in nomi, letta da `app.users` sotto RLS.
  Il precedente è `list_organizations_for_user` ([ADR-041](08-decisioni.md#adr-041)),
  che si affida alla policy invece di ripetere il confine nel `WHERE`;
- `TicketResponse` guadagna `reported_by_name` e `assigned_to_name`;
  `TicketMessageResponse` guadagna `author_name`. Tutti e tre opzionali;
- nel client: «In carico a» mostra **il nome e poi la data**, e non più la sola data —
  che è il difetto letterale di `R21`; il thread mostra *Proprietario · Mario Rossi*;
  «Aperta da» fa lo stesso.

**`reported_by_name` entra pur non essendo nella lettera di `R21` né di `R22`**: è lo
stesso identificatore risolto dalla stessa query e mostrato nella stessa scheda, tre
righe più su di «In carico a». Lasciarlo fuori significherebbe consegnare una schermata
in cui un nome compare e l'altro no.

Quando il nome non torna — la riga `app.users` non è leggibile da chi guarda — il campo
è **assente** e il client mette l'etichetta neutra del catalogo, mai l'UUID. È la regola
di [ADR-043](08-decisioni.md#adr-043) e la stessa forma di «Immobile archiviato» della
sezione 3.

##### `W4` — Su una segnalazione chiusa non si scrive più (`R23`) — **ADR-050 prima**

Un invariante nel dominio legato a `TicketStatus::is_working()`, la guardia in
`PostTicketMessage` dopo la lettura della segnalazione, il codice e il messaggio nel
catalogo. Nel client la casella di scrittura sparisce sugli stati terminali e **dice
perché**, indicando la riapertura a chi ha segnalato.

##### `W5` — Riaprire dice perché (`R24`) — stessa ADR-050

`ReopenTicketCommand` porta un `reason` obbligatorio, validato dal value object del
thread. `ReopenTicket` scrive il messaggio nella stessa transazione della transizione.

La sequenza comune delle transizioni vive in `run()` in `transition.rs`, e **non va
duplicata per questa**: il commento di quella funzione dice che è lì perché nessuna
transizione dimentichi `TicketStatusChanged`, e una copia per la riapertura è
esattamente il modo in cui una transizione lo dimentica. `run()` acquista quindi un
punto in cui inserire un messaggio dentro la stessa unità di lavoro, e le altre sei lo
lasciano vuoto.

Nel client: il dialogo di riapertura passa da conferma a form con un campo obbligatorio,
e il testo dice che quello che si scrive finisce nella conversazione.

### Trappole

**Il ramo di visibilità nella policy invece che nella funzione** è la trappola numero uno,
e il sintomo è una segnalazione che si apre e non si legge: niente thread, niente storico,
niente fotografie, nessun errore.

**Il ramo va per ultimo e dietro il booleano.** `can_access_ticket` è `COST 500` e viene
valutata per riga: `visible_to_residents AND EXISTS(...)` evita di chiamare una funzione
tabellare su ogni segnalazione privata dell'elenco.

**Una funzione con il confine dentro risponde «no» al pool privilegiato**, ed è la
lezione della [sezione 4](11-piani-conclusi.md#esito-sezione-4) alla lettera:
`app.property_residents` porta `can_access_property` nel corpo, che passa da
`is_org_staff`, che chiede il ruolo di *quella* persona in *quella* organizzazione. Un
test che verifichi la condivisione deve installare **`app.current_org_id` e
`app.current_user_id`**, e vederlo fallire senza è il modo di scoprirlo tardi.

**Non si scrive una terza definizione di «chi abita qui».** La `0030` documenta la
trappola alla sua terza occorrenza; una `app.lives_in_property` sarebbe la quarta.

**`R19-bis` fa fallire `just ci` in due posti prima ancora di sbagliare qualcosa**: il
controllo `colonna-strutturale-scrivibile` e
`the_application_role_cannot_move_a_ticket_to_another_property`. Vanno emendati **nello
stesso passo** della migration, e l'emendamento è il documento della revoca — non un
fastidio da togliere di mezzo.

**Il privilegio di colonna non esprime «finché è `open`».** Una policy non può
confrontare la riga nuova con quella vecchia. Chi cercasse di mettere quel limite nella
`USING` di `tickets_update` si accorgerebbe tardi che restringerla romperebbe la
**riapertura**, che parte proprio da `closed`.

**Non copiare `canceller`.** La regola «staff sempre, chi ha segnalato finché è `open`»
deve esistere una volta sola: due copie sono due regole, e la seconda smetterà di
somigliare alla prima appena una delle due cambia.

**La metà «ruolo» di `R22` è già in produzione.** Rifarla significherebbe riscrivere
`MessageRow` per ottenere ciò che già mostra, e concludere la voce senza aver toccato il
caso vero, che è il co-living.

**Il nome che non c'è non è un errore.** Se `users_select` non concede la riga, il campo
è assente: il client mette l'etichetta neutra e non l'UUID, e nessuno va a cercare un
difetto nella query.

**Questa sezione è il primo lettore di `users_select`**, e la `0030` lo aveva previsto
per iscritto: *«la finestra per restringere è aperta e la chiude il primo client che
legga un nome da lì»*. Dopo `W3` quella policy diventa portante, e restringerla non sarà
più gratuito. Va scritto nell'ADR, non qui soltanto.

**La guardia di `R23` non va nel costruttore del messaggio**, o la riapertura di `W5`
inciamperebbe nella propria regola. Sta nello use case che *pubblica*, e la riapertura
scrive il suo messaggio dopo aver riportato lo stato a `open`.

**`run()` non si duplica per la riapertura.** È la funzione che garantisce che nessuna
transizione dimentichi il proprio evento; una copia è il modo in cui una lo dimentica.

**Le migration sono append-only**, e la funzione si sostituisce con
`CREATE OR REPLACE FUNCTION` in un file nuovo, con la firma identica.

**Non si prova nulla sul database di sviluppo.** Le migration si provano sul container
effimero di `ops/ci/test-integration.sh`, che esiste per questo: `just down-hard`
cancella i volumi, ed è già successo una volta — sta scritto
nell'[esito della sezione 2](11-piani-conclusi.md#esito-sezione-2).

### Come si saprà che è finita

- una segnalazione dichiarata visibile la **legge un coinquilino che non è parte del suo
  contratto**, e con essa il thread, lo storico e le fotografie: un test la percorre
  tutta, non solo la riga;
- lo stesso coinquilino **non** la riapre, non la annulla e non la corregge: la
  `USING` di `tickets_update` non è cambiata, e un test lo prova;
- una segnalazione **non** dichiarata visibile resta invisibile a quello stesso
  coinquilino — è il default, ed è la metà che si dimentica di verificare;
- chi ha segnalato corregge titolo e descrizione finché è `open`, e da `assigned` in poi
  riceve `org.staff_role_required`; lo staff corregge sempre, e su una `cancelled` nessuno
  dei due;
- `just ci` è verde **con** il controllo 12 e il test dei privilegi emendati, e i due
  continuano a rifiutare `property_id`, `room_id`, `lease_id`, `reference` e
  `reported_by`;
- «In carico a» dice **chi**, il thread distingue due coinquilini per nome, e un nome non
  risolvibile mostra l'etichetta neutra e mai un UUID;
- su una segnalazione chiusa il thread si **legge** e non si **scrive**, e il rifiuto ha
  un messaggio italiano e non un codice;
- riaprire senza motivo è rifiutato, e riaprire con un motivo lo lascia scritto nella
  conversazione, dove sopravvive alla chiusura successiva;
- `just ci` verde, con i test nuovi sui **due rami** di ogni rifiuto.

### Che cosa **non** entra

- **la modifica di categoria e priorità da parte di chi segnala**: sono il lavoro del
  triage, che è dello staff, e lasciarle correggere altrove renderebbe `triaged` una
  dichiarazione non verificabile — è la ragione per cui esistono come transizione;
- **spostare una segnalazione** su un altro immobile, stanza o contratto. `property_id`,
  `room_id` e `lease_id` restano non scrivibili, e la frase della `0020` resta vera per
  loro;
- **modificare o cancellare un messaggio del thread**: `app.ticket_messages` ha due sole
  policy, e cambiarlo è una migration sulla policy, non una rotta;
- **`ticket_status_history.reason`**, che resta non scritta con la ragione della
  decisione 6;
- **una rotta che elenchi le persone di un'organizzazione**: il debito è aperto apposta,
  e [ADR-036](08-decisioni.md#adr-036) e [ADR-043](08-decisioni.md#adr-043) hanno già
  scartato l'idea di proiettare un utente per identificatore. Qui i nomi viaggiano
  **dentro** la risposta della segnalazione, che è la via che la sezione 3 ha già
  percorso;
- **`R7`**, il proprio account, che è la sezione 6 e che di questa sezione erediterà il
  modo di risolvere un nome;
- **il collaudo in undici punti**, che è la [5-bis](12-da-fare.md#lordine-di-esecuzione)
  e viene subito dopo: sarà la prima volta che le sezioni 3, 4 e 5 si vedono con gli
  occhi.

<a id="esito-sezione-5"></a>

## Esito — Sezione 5: le segnalazioni

Eseguita il **9 settembre 2026**. Chiude `R19`, `R19-bis`, `R21`, `R22`, `R23` e `R24`.

**Due migration, quattro ADR, una rotta nuova, e una decisione documentata revocata.** È
la prima sezione che *toglie* qualcosa a un controllo di CI invece di aggiungerlo, e il
modo in cui lo fa — emendando il controllo e il test **nello stesso passo** della
migration — è la parte da ricordare.

I test Rust passano da **431 a 438**, il client da **86 a 89**.

### Che cosa è entrato

| Passo | Esito |
|---|---|
| `W1` | Migration `0035`: `visible_to_residents`, e il **quinto ramo dentro `app.can_access_ticket`** — non in `tickets_select` |
| `W2` | Migration `0036` e `PATCH /tickets/{id}/report`, con l'invariante di schema e il test dei privilegi emendati nello stesso passo |
| `W3` | `reported_by_name`, `assigned_to_name`, `author_name`: una query per pagina, e l'etichetta neutra quando il nome non torna |
| `W4` | Un thread terminato si **legge** e non si scrive: `ticket.thread_closed` |
| `W5` | Riaprire pretende un motivo, che diventa il primo messaggio del giro nuovo |

### Il piano aveva ragione sulla trappola, e la trappola era più grande

Il piano diceva che `R19-bis` contraddiceva tre cose che la CI verifica. Eseguendola sono
state **quattro**, e la quarta è quella che sarebbe stata scoperta tardi: oltre alla
migration `0020`, al controllo `colonna-strutturale-scrivibile` e al test
`the_application_role_cannot_move_a_ticket_to_another_property`, la frase «di una
segnalazione si modificano solo i costi» viveva anche in `docs/05-api.md`, nel commento di
`Ticket::apply_costs`, in quello di `UpdateTicketCostsRequest` e nella stringa
`t.ticket.onlyCostsArePatchable` del catalogo. Sette posti per un fatto solo — la stessa
forma di `R3`, che ne aveva otto.

**Il test dei privilegi non è stato accorciato, è stato riequilibrato.** Le due `UPDATE`
su `title` e `description` sono passate dal blocco dei rifiuti a quello delle concessioni,
e nel blocco dei rifiuti sono entrate `reported_by_kind` e `currency`, che erano fuori
dalla `GRANT` e che nessun test nominava. Un test che perde due assert è un test più
debole; questo ne ha quattro invece di due.

### Che cosa il piano non aveva previsto

**`TicketResponse` doveva portare i nomi *sempre*, transizioni comprese.** Il piano
diceva «campi opzionali nel DTO» pensando alle sole letture. Ma se le transizioni non
risolvessero i nomi, la stessa risposta li avrebbe a volte sì e a volte no, e un client non
potrebbe distinguere «non leggibile» da «non risolto»: mostrerebbe l'etichetta neutra
subito dopo un'azione riuscita. Da qui `TicketView` — segnalazione più nomi — restituita da
**ogni** caso d'uso che restituisce una segnalazione, al prezzo di una query su al più due
identificatori prima del commit.

**`run()` ha dovuto crescere, e la forma scelta non è quella ovvia.** Il piano diceva «un
punto in cui inserire un messaggio». Aggiungerlo come ottavo parametro avrebbe reso quella
funzione una che si chiama leggendo la firma, e `extra_events` era già il settimo: le due
cose sono diventate una struct sola, `Aftermath`, che porta gli eventi *e* il messaggio
facoltativo. `TicketMessagePosted` lo emette `run`, non chi costruisce il messaggio — è la
stessa proprietà per cui quella funzione esiste.

**L'ordine dentro la riapertura conta più di quanto sembri.** Il messaggio si scrive
**dopo** l'aggiornamento dello stato: al contrario, la regola «un thread terminato non
riceve messaggi» colpirebbe proprio il messaggio che esiste per superarla. È scritto nel
codice accanto alla riga, perché invertire due `await` sembra innocuo.

**La metà «ruolo» di `R22` era già in produzione**, come il piano aveva trovato — e
`reported_by_kind` lo era anche in «Aperta da». Il lavoro vero è stato il nome, e la
schermata ora mostra le due cose **insieme**: il ruolo dice da che parte del rapporto sta
chi scrive, il nome dice chi è.

### Decisioni prese eseguendo

**Il nome della colonna dice «residenti» e non «tutti gli inquilini».** La seconda formula
è quella del backlog e si legge come «dell'organizzazione», che sarebbe un confine più
largo e sbagliato: un inquilino di un'altra casa dello stesso proprietario non c'entra.
La casella nel form usa invece la parola comune, dove il confine è chiaro dal contesto.

**`app.property_residents` e non `app.shares_home_with`**, ed è la scelta che il vincolo
chiedeva di fare invece di scrivere un terzo predicato. La seconda è utente↔utente e
avrebbe avuto due difetti: `reported_by` è `ON DELETE SET NULL`, quindi la condivisione
sarebbe sparita da sé alla cancellazione dell'autore; e non filtra `lease_in_force` —
deliberatamente, è un pavimento e non un elenco — quindi un inquilino uscito due anni fa
avrebbe visto le segnalazioni nuove della casa in cui non vive più.

**Il ramo va per ultimo nell'`OR` e dietro il booleano.** `can_access_ticket` è `COST 500`
e viene valutata per riga nell'elenco: con `visible_to_residents` davanti, la funzione che
restituisce righe non viene chiamata affatto sulle segnalazioni private, che sono il
default.

**La correzione non guarda se la segnalazione è chiusa**, solo se è annullata — come
`apply_costs`. Una chiusa si rilegge mesi dopo, e un titolo sbagliato lì è sbagliato per
sempre.

**`reported_by_name` è entrato pur non essendo nella lettera di `R21` né di `R22`**: è lo
stesso identificatore, risolto dalla stessa query, mostrato nella stessa scheda tre righe
più su di «In carico a». Lasciarlo fuori avrebbe consegnato una schermata in cui un nome
compare e l'altro no.

### Che cosa questa sezione ha reso portante

**`users_select` non si restringe più gratis.** La `0030` lo aveva scritto per esteso — *«la
finestra per restringere è aperta e la chiude il primo client che legga un nome da lì»* — e
questa sezione è quel primo client. I due rami su cui il modulo ora poggia sono
`shares_home_with`, che fa leggere al coinquilino il nome dell'altro nel thread, e
`is_my_staff`, che fa leggere all'inquilino il nome di chi ha in carico la sua
segnalazione. Un test li interroga tutti e tre insieme al ramo dello staff.

Il fatto che `shares_home_with` **non** filtri `lease_in_force` era una scelta scritta nel
suo commento; da adesso è una dipendenza, perché è ciò che rende leggibile il nome
dell'autore di un messaggio vecchio su una segnalazione di chi si è nel frattempo
trasferito.

### Che cosa resta aperto

- **`ticket_status_history.reason` resta non scritta**, e ora ha una ragione scritta invece
  di essere solo vuota: riempirla richiederebbe un secondo canale verso il trigger — una
  `set_config` transazionale che una seconda transizione erediterebbe — per un campo di
  testo che il thread già tiene meglio. La differenza con il numero falso della sezione 4 è
  che là un valore sbagliato **si mostrava**, qui un campo vuoto non compare;
- **i nomi nello storico degli stati** non ci sono: `changed_by` resta un identificatore, e
  la riga dello storico dice già ciò che serve — che cosa è cambiato e quando. È un'altra
  domanda, e la si fa quando qualcuno la pone;
- **`R26`** resta ferma dov'era: nessuna delle sei voci la tocca;
- **la correzione non ha uno storico.** Chi ha riscritto il titolo sta in `audit_log` e
  nell'evento `ticket.report_corrected`, che porta i **nomi dei campi** e non i valori —
  perché il testo di una persona non finisce su un broker. Il testo di prima non è
  recuperabile dal prodotto;
- **il subentro (`R28`)** e **il perimetro dei ruoli (`R9`)** restano dove erano.

### Che cosa non è stato provato a mano

Tutto, come per la 3 e la 4: solo test. E qui pesa quanto nelle due precedenti, perché la
schermata della segnalazione cambia in **quattro** punti — la casella all'apertura, i nomi,
il bottone «Correggi», la conversazione che si chiude — e il punto **7** del collaudo la
attraversa per intero.

**È il momento della [5-bis](12-da-fare.md#lordine-di-esecuzione)**: nove punti su undici
sono ora su codice fermo, ed è la prima volta che le sezioni 3, 4 e 5 si vedono con gli
occhi invece che da una suite verde.


---

<a id="esito-sezione-5-bis"></a>

## Esito — Sezione 5-bis: il collaudo in undici punti

Percorso il **9 e il 10 settembre 2026**, per intero, **da chi non aveva scritto il
codice**. Non chiude nessuna voce di [12](12-da-fare.md) — non era il suo compito — e ne
apre **sei**: `R30`, `R31`, `R32`, `R33`, `R34` e `R35`, che nello stesso giorno si è
dissolta dentro `R31`.

Le undici righe stanno nell'[esito della fetta C](#esito-fetta-c), che con questo giro
**smette di essere aperto** dopo due giorni e cinque sezioni di attesa.

### Il rendimento, e perché il rinvio era costato

L'[esito della sezione 2](#esito-sezione-2) aveva scritto il costo del rinvio in anticipo,
e vale la pena rileggerlo adesso che c'è la misura:

> Il collaudo non serve solo a verificare, **alimenta i piani**. […] Le sezioni 4 e 5 si
> pianificheranno leggendo il codice soltanto.

**Mezza giornata di giro ha prodotto sei voci; cinque sezioni di lettura del codice ne
avevano prodotte quattro.** E una delle sei — `R31` — è un buco nel **ciclo di vita
normale** del prodotto, non un dettaglio: un inquilino il cui contratto scade e che ne
firma un altro non può essere ricollegato, perché l'accettazione dell'invito è l'unica
cosa che scrive `lease_parties.user_id` e rifiuta chi è già membro.

**Nessun test poteva trovarlo, e la ragione è la cosa più utile di questo esito**: ogni
test di integrazione costruisce un inquilino **nuovo**. Il difetto vive esattamente nello
spazio che una suite verde non attraversa — la seconda volta che qualcosa succede alla
stessa persona. Vale la pena portarselo dietro come domanda: *quali altri percorsi il
prodotto attraversa solo la seconda volta?*

### Che cosa ha retto

Va scritto, perché un esito che elenca solo i difetti fa credere che il resto non sia
stato guardato:

- **nessuno degli undici punti si è fermato.** Le sei cose trovate sono difetti dentro
  percorsi che funzionano, non percorsi interrotti;
- **`C2` regge** — due schede dello stesso account, token scaduto, ricaricate insieme, e
  nessuna buttata fuori — ed era il punto che prima della fetta C era *certo* che
  fallisse;
- **`C1` regge nella forma corretta da `T1`**: il codice revocato, presentato davvero,
  viene rifiutato. La correzione del punto 4 è servita a qualcosa;
- **la divergenza vera degli stati funziona**: una stanza in ristrutturazione e abitata
  dice «Non disponibile · occupata», che è l'anomalia che la sezione 4 voleva far vedere.
  È la ragione per cui `R30` **corregge la regola invece di toglierla**.

### Una voce entrata e dissolta nello stesso giorno

`R35` — che cosa vede l'inquilino di un contratto non ancora attivo — era stata scritta
come una scelta di prodotto con tre letture possibili, e tenuta **fuori** dal piano della
5-ter in attesa di una risposta. La risposta è arrivata da una riproduzione: un inquilino
invitato su un contratto in bozza accetta e **lo vede**. Non esiste nessuna regola sulle
bozze; il contratto che non compariva non compariva perché la persona non era collegata.

**Era `R31` visto dall'altro lato.** Vale la pena tenerne il ricordo perché è il modo in
cui un difetto si traveste: da lontano sembrava una domanda di prodotto, e chiedere
«quale delle tre?» invece di riprodurre avrebbe fatto progettare una regola per un
comportamento che non esiste.

### Che cosa non ha funzionato di questo esito

**Il dettaglio delle undici righe.** `T7` chiedeva di riempirle «con che cosa si è visto e
non con un segno di spunta»; il resoconto è arrivato consolidato — *tutto andava tranne
quello che ho scritto* — e per i punti 1, 9, 10 e 11 resta l'affermazione che sono stati
percorsi, senza l'osservazione.

È una perdita reale e non una formalità: fra sei mesi nessuno saprà se al punto 9 i saldi
tornavano a zero **da entrambe le parti** o solo da una, che è precisamente la cosa che la
fetta B-bis aveva costruito. **Il rimedio non è ricordarsi meglio**: è scrivere la riga
mentre si percorre il punto, non alla fine del giro. Chi rifarà un collaudo tenga il
documento aperto accanto al browser.

### Che cosa viene dopo

La [sezione 5-ter](#piano-sezione-5-ter) corregge le cinque voci che
restano, e la [5-quater](#piano-utenti-veri) aggiunge le quattro cose
senza cui una persona esterna non può cominciare. L'obiettivo che le lega è dichiarato:
**un proprietario vero, che non ha scritto il codice, usa il prodotto per un mese con i
suoi dati.**

---

<a id="piano-sezione-5-ter"></a>

## Piano — Sezione 5-ter: quello che il collaudo ha trovato

Pianificata il **9 settembre 2026**, subito dopo la prima mezza giornata di
[collaudo](12-da-fare.md#lordine-di-esecuzione) percorsa **da chi non aveva scritto il
codice**. Chiude `R30`, `R31`, `R32`, `R33` e `R34`. `R35` resta fuori e la ragione è
scritta in fondo.

**È la sezione che giustifica cinque rinvii.** Il costo del rinvio era scritto
nell'[esito della sezione 2](11-piani-conclusi.md#esito-sezione-2) — *«il collaudo non
serve solo a verificare, alimenta i piani»* — e mezza giornata di giro vero ha prodotto
**sei voci**, di cui una (`R31`) è un buco nel ciclo di vita normale del prodotto che
nessuna delle cinque sezioni precedenti aveva visto, e che nessun test avrebbe trovato
perché tutti i test costruiscono un inquilino nuovo ogni volta.

### Le cinque voci non sono cinque problemi

Sono **tre**, e vedere quali si accorpano è la parte del piano che conviene non perdere:

| Voci | Che cosa hanno in comune |
|---|---|
| `R30` | una regola di presentazione scritta per l'eccezione e applicata alla norma |
| `R32`, `R33` | **il problema dei nomi, terza occorrenza**: `R14` sui contratti, `R21`/`R22` sulle segnalazioni, e adesso su segnalazioni, addebiti, bollette e sul riaddebito. La risposta è già decisa due volte |
| `R31`, `R34` | due difetti isolati, che condividono solo l'essere stati trovati usando |

### Che cosa il collaudo ha trovato, e la causa di ciascuno

Le cause sono **verificate nel codice**, non dedotte dal sintomo: è la differenza fra un
piano che si può eseguire e un elenco di lamentele.

#### `R30` — la parentesi che compare sempre

`status.tsx` mostra i due stati **ogni volta che differiscono**. La regola era per
l'anomalia — una stanza in ristrutturazione e occupata — ma per
[ADR-019](08-decisioni.md#adr-019) e per la [sezione 4](11-piani-conclusi.md#esito-sezione-4)
la divergenza è **garantita** nel caso normale: nessuno riscrive `active` quando un
contratto scade, `occupied` non lo scrive nessuno mai, `rented` nemmeno. La parentesi
compare quindi su quasi ogni riga.

**Ma il commento di `LeaseStatusBadge` porta una ragione vera**, e buttarla via
sarebbe il modo di reintrodurre il problema al contrario: chi filtra per «attivo» e si
vede tornare un contratto marcato «Scaduto» deve poter capire perché. L'informazione
serve; è il *posto* a essere sbagliato.

#### `R31` — l'inquilino che torna, e non può rientrare

Il ciclo di vita normale: il contratto scade o si disdice, con la stessa persona se ne fa
un altro. Oggi **non si può**, e il modo in cui non si può è il peggiore:

1. `InviteTenant` non guarda niente: crea la parte contrattuale ed emette l'invito. Il
   proprietario vede un codice buono e lo consegna;
2. `AcceptInvitation` fa **sempre** `uow.members().insert(...)`, che sbatte contro
   `org_members_org_user_uk`;
3. `map_membership_conflict` lo traduce in `409 membership.already_exists`.

L'inquilino riceve un errore su un invito che il proprietario ha appena emesso senza
un avviso. E `lease_parties.user_id` **lo scrive solo quel percorso**: nessun'altra rotta
collega una persona a una parte contrattuale — `PATCH /lease-parties/{id}` tocca nome,
telefono, ruolo, quota e intestatario, non l'utente.

**Nessun test lo copre**, e la ragione è istruttiva: ogni test costruisce un inquilino
**nuovo**. Il difetto vive esattamente nello spazio che una suite verde non attraversa.

#### `R32` e `R33` — i nomi, terza occorrenza

Tre collegamenti mostrano un'etichetta generica: `TicketDetailPage.tsx:93`,
`ChargeDetailPage.tsx:164`, `BillDetailPage.tsx:154`. E il `<Select>` del riaddebito
elenca i contratti come *tipo · data*, che è **il difetto che `R14` ha corretto
nell'elenco dei contratti** — mentre `LeaseResponse` porta ormai `property_label` e
`room_name` e quel dialogo non li guarda.

È il terzo giro sulla stessa radice, e la risposta è decisa da
[ADR-049](08-decisioni.md#adr-049): allargare la risposta con un modello di lettura, mai
una rotta che risolva le entità.

#### `R34` — la data che non propone oggi

`paid_at: ''` in `DeclarePaymentDialog` e `RecordPaymentDialog`, mentre `todayIso()`
esiste ed è già usato per le scadenze. **La trappola sta nel rimedio**, non nel difetto:
oggi il campo vuoto significa «adesso» e il server mette l'istante vero; precompilarlo
lo trasformerebbe in **oggi a mezzanotte**.

### Che cosa è stato deciso prima di pianificare

| | Scelta | Alternativa scartata |
|---|---|---|
| Quando si mostra lo stato dichiarato | **solo quando è un giudizio che il fatto smentisce** | ogni volta che i due differiscono, che è oggi |
| Dove va lo stato dichiarato quando non è un'anomalia | **un campo suo nella scheda** | toglierlo del tutto, perdendo la ragione del filtro |
| Come rientra un inquilino (`R31`) | **`AcceptInvitation` tollera la membership che c'è già** | una rotta nuova che colleghi un utente a una parte |
| I nomi mancanti (`R32`) | **tre campi in più su tre risposte**, con il modello di lettura di sempre | comporli nel client con richieste in più |
| La bolletta dell'immobile intero (`R33`) | **dire quanto resta da riaddebitare**, una chiamata per volta | una richiesta che ne porti diverse insieme |
| La data di oggi (`R34`) | **mostrata e non mandata** finché non la si cambia | mandarla, perdendo l'ora vera |

### Decisioni, con il perché

#### 1. Il badge mostra il fatto; il dichiarato compare solo se è un giudizio smentito

La regola nuova, in una riga: **lo stato memorizzato compare accanto al derivato solo
quando qualcuno ha deciso qualcosa che il fatto contraddice.**

| Caso | Oggi | Da qui |
|---|---|---|
| contratto `active` + derivato `expired`/`expiring` | «Scaduto (Attivo)» | **«Scaduto»** |
| stanza `available` + derivato `occupied` | «Occupata (Disponibile)» | **«Occupata»** |
| immobile `available` + derivato `rented` | «Affittato (Disponibile)» | **«Affittato»** |
| stanza **`unavailable`** + derivato `occupied` | «Occupata (Non disponibile)» | **resta doppio**: qualcuno l'ha messa in ristrutturazione mentre ci abita una persona |
| immobile **`maintenance`** o **`draft`** + derivato `rented` | doppio | **resta doppio** |

Il criterio non è un elenco di casi da imparare a memoria: `available` su una stanza e
`active` su un contratto sono **il default che la derivazione esiste per superare**, non
una decisione di qualcuno. `unavailable` e `maintenance` sì.

**L'informazione che il commento difendeva non si perde, cambia posto.** Lo stato
dichiarato diventa un campo della scheda — accanto a quello effettivo — dove chi
amministra lo trova quando lo cerca invece di leggerlo su ogni riga. E chi filtra per
«attivo» e riceve un contratto marcato «Scaduto» trova la spiegazione **dove nasce la
domanda**: nell'elenco, accanto al filtro attivo, non ripetuta su venticinque righe.

*Alternativa scartata: togliere del tutto lo stato dichiarato.* È il rimedio che verrebbe
in mente leggendo solo la lamentela, e perderebbe la ragione scritta in
`LeaseStatusBadge`: quel campo è l'unica cosa che spiega il risultato di un filtro.

#### 2. Un inquilino che torna passa dalla stessa porta

`AcceptInvitation` smette di fallire quando la membership esiste già: se chi accetta è
**già membro attivo di quella organizzazione**, non la si inserisce e si va avanti a
collegare la parte contrattuale.

**Una via sola e non due**, ed è la ragione della scelta: l'invito è già il gesto che il
proprietario compie — lo ha appena compiuto, e ha ricevuto un codice buono. Aggiungere
una rotta «collega una persona già dentro» significherebbe due meccanismi per lo stesso
fatto, e il proprietario dovrebbe sapere in anticipo quale dei due gli tocca — cioè
ricordarsi se quella persona è già stata sua inquilina.

**Il ruolo non si tocca.** Chi è già `owner` o `manager` e viene invitato come `tenant`
resta quello che è: l'invito serve a collegare la parte contrattuale, non a retrocedere
qualcuno. È anche il caso vero di [01](01-prodotto.md) — chi affitta una casa e ne abita
un'altra.

**Chi è uscito dall'organizzazione resta fuori da questa decisione.** `org_members` ha
`status` e `deleted_at`, ma nessuno li valorizza finché [`R16`](12-da-fare.md#uscire-da-unorganizzazione)
non esiste: oggi non c'è modo di uscire, quindi non c'è nessuno da far rientrare. La
guardia va scritta lo stesso — membership **attiva**, non una qualunque — perché il
giorno in cui `R16` arriva questo punto diventerebbe la porta di servizio.

*Alternativa scartata: `PATCH /lease-parties/{id}` che accetta `user_id`.* Sposta la
scelta della persona su chi amministra, che dovrebbe cercarla in un elenco — e l'elenco
delle persone di un'organizzazione **non esiste ed è aperto apposta**
([ADR-036](08-decisioni.md#adr-036), [ADR-043](08-decisioni.md#adr-043)). Costruirlo per
questo sarebbe riaprire quella domanda per un problema di comodità.

#### 3. I nomi: terza occorrenza, stessa risposta

`property_label` e `room_name` su `TicketResponse`; le stesse due su `ChargeResponse` e
`UtilityBillResponse`, risolte attraverso il contratto. Modello di lettura, **una query
per pagina**, campi opzionali, e l'etichetta neutra quando mancano.

**Non è una decisione nuova e non serve un ADR**: [ADR-049](08-decisioni.md#adr-049) l'ha
presa per le segnalazioni tre giorni fa, e questa ne è l'applicazione. Vale invece la pena
scrivere che è la **terza** volta — `R14`, poi `R21`/`R22`, ora `R32` — perché la
regolarità dice qualcosa: **ogni risposta che porta un identificatore verso una schermata
è un difetto che aspetta di essere trovato**, e conviene guardare le rimanenti prima che
le trovi qualcun altro.

#### 4. Il riaddebito dice a chi, e quanto resta

Due metà, e la seconda è più piccola di quanto sembri.

**Chi**: il `<Select>` mostra *immobile · stanza* e il nome dell'intestatario, non
*tipo · data*. I dati ci sono già tutti in `LeaseResponse` dalla sezione 3. E l'elenco
**si restringe ai contratti che possono ricevere un addebito**: una bozza o un contratto
disdetto non sono candidati, e offrirli è la stessa classe di difetto di
[`R26`](12-da-fare.md#lelenco) — un'opzione che non può portare a niente di buono.

**Quanto resta**: la bolletta conosce il proprio totale, e la somma di ciò che è già stato
riaddebitato è leggibile. Il dialogo lo mostra e propone il residuo, così riaddebitare una
bolletta d'immobile a tre contratti è tre volte lo stesso gesto guidato invece di tre
sottrazioni a mente.

*Alternativa scartata: una richiesta che porti più contratti insieme.* È la strada giusta
il giorno in cui serve davvero, e oggi porterebbe **l'esito parziale** che
[`R15`](12-da-fare.md#cancellare-in-generale) ha già isolato come decisione a sé: N
scritture, una fallisce, e non è chiaro né che cosa dire né che cosa disfare. Non si apre
qui un fronte che ha già una voce sua.

**Non si tocca la regola contabile**: nessuno verifica che le quote riaddebitate sommino
al totale, e riaddebitare più del totale resta possibile. È scritto nel commento di
`RechargeDialog` e non è un difetto da correggere di passaggio: lo schema non ha il
vincolo e un controllo applicativo sarebbe soggetto a race condition fra due riaddebiti
concorrenti.

#### 5. La data si mostra e non si manda

`paid_at` parte da oggi **nel campo**, e la richiesta continua a non portarlo finché la
data resta quella di oggi.

Sembra un cavillo ed è la sostanza: oggi il campo vuoto significa «adesso» e il server
mette l'istante vero. Precompilare e mandare trasformerebbe un pagamento delle 18 in uno
delle **00:00**, cioè si perderebbe l'ora per guadagnare una comodità — e in un elenco
ordinato per data due dichiarazioni dello stesso giorno diventerebbero indistinguibili.

Vale per la dichiarazione dell'inquilino e per l'incasso registrato dal proprietario. Per
il secondo la data conta di più — un bonifico si vede sull'estratto conto giorni dopo — e
lì cambiarla è il gesto normale, non l'eccezione.

### Decomposizione

L'ordine è quello del danno: prima il buco nel ciclo di vita, poi le tre cose che si
leggono male.

##### `Y1` — L'inquilino che torna (`R31`)

`AcceptInvitation` verifica la membership prima di inserirla: se ne esiste una **attiva**
in quella organizzazione, salta l'inserimento e prosegue. Il ruolo non si tocca.

Due test, e provano i due lati: una persona già inquilina di un contratto scaduto accetta
l'invito al contratto nuovo e **vede tutti e due**; e chi non è membro continua a
diventarlo, che è il caso che non deve rompersi.

**La riproduzione esiste già** ed è il criterio di accettazione, misurato prima di
pianificare:

```
CASO 1 — inquilino nuovo invitato su un contratto in bozza
  invito 201 · accettazione 200 · l'inquilino vede 1 contratto ('draft')
CASO 2 — lo stesso inquilino invitato su un secondo contratto
  invito 201  ← il proprietario riceve un codice che sembra buono
  accettazione 409 membership.already_exists
  l'inquilino vede ancora 1 contratto, non 2
```

Il caso 2 deve finire `200` e con **due** contratti; il caso 1 deve restare identico.

**Prima di scrivere**: rileggere il commento in testa a `AcceptInvitation` sull'ordine
delle tre scritture — `app.has_pending_invitation` abilita le prime due e diventa falso
appena l'invito è marcato accettato. Saltare la prima non cambia l'ordine delle altre, e
va verificato che non cambi nemmeno il permesso.

##### `Y2` — Il badge smette di gridare (`R30`)

`status.tsx`: i tre badge mostrano il derivato da solo, e il dichiarato accanto **solo**
nei casi di anomalia della tabella della decisione 1. Nelle tre schede — contratto,
stanza, immobile — lo stato dichiarato diventa un campo della `DescriptionList`.

Un test per famiglia sul caso che regredisce: **la stanza in ristrutturazione e occupata
continua a mostrare entrambi**. È l'assert che qualcuno toglierà semplificando.

##### `Y3` — I nomi nelle tre risposte (`R32`)

`property_label` e `room_name` su `TicketResponse`, `ChargeResponse` e
`UtilityBillResponse`; nel client i tre collegamenti generici diventano il nome, con
l'etichetta neutra quando il campo manca.

Il modello di lettura dei contratti esiste già (`LeaseRepository::labels`): per addebiti e
bollette si risolve **attraverso il contratto**, e per le segnalazioni serve la coppia
sull'immobile, che è una query nuova della stessa forma.

##### `Y4` — Il riaddebito (`R33`)

Il `<Select>` dice *immobile · stanza · intestatario* e offre i soli contratti che possono
ricevere un addebito; il dialogo mostra il **residuo da riaddebitare** e lo propone come
importo.

##### `Y5` — Le date (`R34`)

`paid_at` precompilato a oggi in entrambi i dialoghi, e mandato **solo se diverso da
oggi**. Un test sul caso che si romperebbe in silenzio: aprire e inviare senza toccare la
data non deve mettere `paid_at` nel corpo.

### Trappole

**Il caso che regredisce di `Y2` è l'anomalia, non la norma.** Chi legge la nuova regola
e la semplifica in «mostra sempre il derivato» toglie l'unica cosa che quel doppio badge
esisteva per dire. Il test va scritto su quello.

**`Y2` non è solo estetica: cambia ciò che si legge dopo un filtro.** Il commento di
`LeaseStatusBadge` va **spostato**, non cancellato: la ragione che porta resta vera, e
sopravvive nel campo della scheda e nell'avviso accanto al filtro.

**`Y1` tocca un flusso che gira sotto una policy fatta apposta.** La `0018` estende la RLS
all'accettazione, e `app.has_pending_invitation` è ciò che abilita le scritture. Un ramo
in più prima dell'inserimento non deve spostare l'ordine delle altre due.

**`Y1` guarda la membership *attiva*.** Una qualunque aprirebbe la porta a chi è uscito, il
giorno in cui `R16` renderà possibile uscire — e sarebbe un difetto trovato mesi dopo, in
un'altra sezione.

**`Y3` è la terza volta**, e le trappole sono già state pagate: `LEFT JOIN` e
`deleted_at IS NULL`, quindi un immobile archiviato lascia il campo assente e il client
dice che cosa è successo; **una query per pagina** e non una per riga.

**`Y5` sembra una riga e ne vale due.** Precompilare *e* mandare perde l'ora vera. Il campo
mostra oggi, la richiesta tace.

**Non si tocca la regola del riaddebito.** Che le quote possano superare il totale è
scritto e voluto: qui si aggiunge un numero che aiuta, non un vincolo.

### Come si saprà che è finita

- un inquilino con un contratto scaduto accetta l'invito a un contratto nuovo e **li vede
  entrambi**, senza che nessuno tocchi il database;
- un contratto scaduto dice **«Scaduto»** e basta; una stanza occupata dice **«Occupata»**
  e basta; una stanza in ristrutturazione e occupata continua a dire **tutte e due**;
- la scheda di un contratto dice ancora qual è lo stato **dichiarato**, per chi lo cerca;
- dalla scheda di una segnalazione, di un addebito e di una bolletta si legge **il nome**
  di ciò a cui portano, e un immobile archiviato lo dice invece di lasciare uno spazio;
- il riaddebito dice a quale stanza e a chi, e quanto **resta** da riaddebitare;
- una dichiarazione di pagamento inviata senza toccare la data conserva **l'ora**;
- `just ci` verde, con i test nuovi sui casi che regrediscono in silenzio.

### Che cosa **non** entra

- **`R35` non entra perché non esiste più.** Posta la domanda, la risposta è arrivata da
  una riproduzione e non da un'opinione: un inquilino invitato su un contratto **in bozza**
  accetta e **lo vede**. Non c'è nessuna regola che nasconda le bozze; il contratto che non
  compariva non compariva perché la persona non era collegata, che è `R31` visto
  dall'altro lato. Ne resta una domanda di sola presentazione — se «Bozza» sia la parola
  giusta per chi abiterà lì — che va con [`R8`](12-da-fare.md#interfaccia-e-prodotto).
  **Vale la pena tenerne il ricordo**: la voce era stata scritta come una scelta di
  prodotto con tre letture, ed era il sintomo di un difetto già in elenco;
- **una richiesta che riaddebiti a più contratti insieme**: è l'esito parziale di `R15`,
  che ha una voce sua;
- **il vincolo sulla somma dei riaddebiti**, che è scritto come deliberato;
- **le altre risposte che portano identificatori verso una schermata.** La regolarità
  vista al punto 3 dice che ce ne sono altre; guardarle tutte è una voce nuova, non un
  allargamento di questa;
- **`R26`, `R27`, `R28`, `R29`**, che restano dove sono.

---

<a id="esito-sezione-5-ter"></a>

## Esito — Sezione 5-ter: quello che il collaudo ha trovato

Eseguita il **10 settembre 2026**. Chiude `R30`, `R31`, `R32`, `R33` e `R34`, e ne apre
una: `R36`.

**Un ADR, nessuna migration, cinque test Rust e sedici del client.** I test Rust passano
da **438 a 443**, quelli del client da **86 a 102** — la sproporzione è il ritratto della
sezione: quattro voci su cinque vivono a schermo.

### Che cosa è entrato

| Passo | Esito |
|---|---|
| `Y1` | `AcceptInvitation` salta l'inserimento quando la membership c'è già **ed è attiva**. [ADR-051](08-decisioni.md#adr-051), nessuna migration |
| `Y2` | `isAnomaly` in `status.tsx`: il dichiarato compare solo se è un giudizio che il fatto smentisce. Lo stato dichiarato passa nella scheda e accanto al filtro |
| `Y3` | `property_label` e `room_name` su `TicketResponse`, `ChargeResponse` e `UtilityBillResponse` — su **ogni** risposta, transizioni comprese |
| `Y4` | Il `<Select>` dice *immobile · stanza · intestatario* e offre i soli contratti in corso; il dialogo mostra il **residuo** e lo propone |
| `Y5` | `paid_at` parte da oggi **nel campo** e non entra nel corpo finché la data non cambia |

### La premessa sbagliata di `Y4`, e che cosa è costata

Il piano diceva, del `<Select>` del riaddebito: *«il nome dell'intestatario. **I dati ci
sono già tutti in `LeaseResponse` dalla sezione 3.»* Non è vero, e la metà sbagliata è
proprio l'intestatario: la sezione 3 ha messo `property_label` e `room_name` su
`LeaseResponse`, **non il nome di chi ci abita**. Le parti stanno in `LeaseDetail`, che è
la lettura di un contratto per volta.

La decisione era giusta e la premessa no, quindi si è emendata la premessa invece di
rinunciare alla decisione: `LeaseLabels` guadagna un terzo campo, `primary_tenant`,
risolto **dalla stessa query** con un `LEFT JOIN` su `lease_parties`. Costo reale: una
colonna in una query che già esisteva, zero round trip in più, e la trappola del piano —
*una query per pagina e non una per riga* — rispettata invece che aggirata. L'alternativa
sarebbe stata caricare il dettaglio di ogni contratto candidato dal client, cioè
esattamente l'`N+1` che quella trappola vieta.

Il join non può moltiplicare le righe, e non per fortuna: `lease_parties_primary_uk` è
unico su `(lease_id) WHERE is_primary AND deleted_at IS NULL`. Senza quel vincolo sarebbe
servito un `LATERAL … LIMIT 1`, che avrebbe nascosto il problema invece di appoggiarsi
alla garanzia che c'è.

### La stanza non ha una scheda, e il piano dava per scontato che ce l'avesse

`Y2` diceva: *«Nelle tre schede — contratto, stanza, immobile — lo stato dichiarato
diventa un campo della `DescriptionList`»*. Le schede sono **due**. Una stanza non ha una
pagina propria — `GET /rooms/{id}` non esiste, ed è scritto nel codice del client — e vive
come riga di una tabella dentro la scheda dell'immobile.

Non si è aggiunta una colonna «Stato dichiarato» a quella tabella, perché sarebbe stata
`R30` riscritta in un'altra forma: la stessa informazione ripetuta su ogni riga. Lo stato
dichiarato di una stanza si legge dov'è **già**, cioè nella tendina a due valori di
`RoomDialog`, che è anche il posto in cui si cambia — e chi lo cerca lo cerca per
cambiarlo. Il commento di `RoomFields` che spiegava perché lo stato *non* fosse offerto è
stato riscritto: era rimasto alla domanda aperta prima della sezione 4, e diceva il
contrario di quello che il codice fa da allora.

### `Y3` è costato più del previsto, e la ragione è la lezione della sezione 5

Il piano diceva «tre campi in più su tre risposte». Il lavoro vero è stato **dove** metterli.

L'[esito della sezione 5](#esito-sezione-5) aveva scritto che `TicketResponse` deve
portare i nomi *sempre*, transizioni comprese, perché una risposta che li porti a volte sì
e a volte no non permette di distinguere «non leggibile» da «non risolto». Quella
proprietà vale identica per gli immobili archiviati, e vale identica su addebiti e
bollette: ne segue che **nove casi d'uso di scrittura** hanno cambiato tipo di ritorno, non
solo i quattro di lettura.

Da qui due cose che il piano non nominava.

**`view_of`, un punto unico per le segnalazioni.** `TicketView` si costruiva in dodici
posti con la forma «risolvi e poi costruisci», ripetuta. Con due modelli di lettura invece
di uno, la tredicesima ne dimentica uno — e il sintomo sarebbe un'etichetta neutra subito
dopo un'azione riuscita, cioè indistinguibile da un immobile archiviato. Adesso c'è una
funzione sola che li risolve entrambi **prima del commit**, e le dodici la chiamano.

**`Placed<T>` per bollette e addebiti**, con `Deref` verso `T`. È la forma di
`axum::Json<T>`: senza, ogni chiamante che vuole l'aggregato scriverebbe `.value`, e il
rumore finirebbe per far preferire il tipo nudo — cioè per rimettere i nomi solo sulle
letture, che è il difetto da cui si partiva. `places` resta esplicito, perché chi lo usa lo
sta usando apposta.

Il campo `recharged` di `Placed<T>` è **vuoto** su ciò che non cita una bolletta, ed è la
scelta meno pulita della sezione. Un involucro per aggregato sarebbe sembrato più ordinato
e non regge il terzo caso: il riaddebito restituisce **una bolletta e un addebito
insieme**, quindi cita entrambi i modelli e li vuole tutti e due. Con due involucri
distinti quel caso avrebbe avuto bisogno di un terzo che li contiene — un tipo in più per
non ammettere che i modelli di lettura di questo modulo sono due e non uno per aggregato.

### `recharged_amount` è per lo staff, e la ragione è più forte di quella di `estimated_cost`

`discloses_estimated_cost` esisteva già e la sua ragione è di **contratto**: la stima è la
cifra di una trattativa con un fornitore, e la Row Level Security lascia comunque leggere
la riga a chi ha segnalato.

Qui la ragione è di **correttezza**. La somma dei riaddebiti si legge sotto RLS, e
`charges_select` mostra all'inquilino le sole quote sue: il numero che riceverebbe sarebbe
**parziale e indistinguibile da un totale** — «riaddebitato 92,10» su una bolletta
riaddebitata per intero a due coinquilini. Un numero sbagliato è peggio di un numero
assente, ed è la stessa forma del difetto che la sezione 4 ha trovato in
`v_property_occupancy`: una vista che mostrava uno zero non vero.

**È anche una domanda che `R29` ha già in elenco**, vista da un'altra angolazione:
`app.v_charge_balances` è `security_invoker` e somma i pagamenti sotto la RLS di chi
legge. Là il numero parziale **si mostra**; qui si è scelto di non mostrarlo affatto, che
è la terza delle tre strade che `R29` elenca. Chi risolverà `R29` trovi qui un precedente,
non un vincolo.

### Decisioni prese eseguendo

**`ChargeStatusBadge` non è stato toccato, ed è `R36`.** Il piano parla di «i tre badge» e
la tabella della decisione 1 elenca contratti, stanze e immobili. Il badge degli addebiti
ha la stessa forma e **non lo stesso difetto**: `Charge::effective_status` diverge solo da
uno stato `is_outstanding()`, cioè da `issued` *o* da `partially_paid`, e «parzialmente
pagato» non si ricostruisce da «scaduto». Accorparlo al volo avrebbe cancellato un fatto
contabile; lasciarlo com'è lascia «Scaduto (Emesso)», che è `R30`. Il rimedio è
asimmetrico e va deciso, non applicato di passaggio: è una voce nuova.

**L'etichetta neutra è una sola stringa e sta in `common`.** Era `t.lease.unknownProperty`,
usata da due schermate; con segnalazioni, addebiti e bollette sarebbero diventate cinque
copie della stessa frase sotto quattro moduli diversi. È diventata
`t.common.archivedProperty`, e le due chiamate esistenti sono state spostate.

**`subtractAmounts` non esisteva.** `@/lib/decimal` aveva la somma e il confronto, non la
differenza — e il residuo da riaddebitare è una differenza. Scriverla a mano con un `-`
sarebbe stato il punto esatto in cui `parseFloat` rientra dalla finestra: `184.20 - 90` in
aritmetica nativa dà `94.19999999999999`, e quel valore sarebbe finito **nel campo di un
form** e da lì nella richiesta. Due test lo fissano, uno dei quali sul residuo negativo,
che è ammesso.

**L'avviso sul superamento del totale è un `hint`, non un errore.** Riaddebitare più del
totale resta possibile ed è scritto come deliberato: un errore che blocca l'invio avrebbe
trasformato un aiuto in un cancello, cioè avrebbe introdotto dal client il vincolo che il
database non ha e che ADR-015 spiega perché non debba avere.

**I contratti candidati si filtrano sullo stato memorizzato, non su quello derivato.** È
la parte che si sbaglia volendo essere precisi: un contratto scaduto ieri ha
`effective_status` `expired` e `status` ancora `active`, e riaddebitargli l'ultima bolletta
del periodo in cui quella persona ci abitava è il caso normale. Filtrare sul derivato lo
escluderebbe proprio quando serve.

### Il metodo non è stato rispettato su un punto, e va scritto

**[ADR-051](08-decisioni.md#adr-051) è stata scritta dopo il codice di `Y1`, non prima.**
`CLAUDE.md` §2 dice il contrario, e la ragione della regola è vera: una decisione scritta
dopo tende a giustificare ciò che si è fatto invece di scegliere. In questo caso il testo
della decisione era già nel piano — *«Un inquilino che torna passa dalla stessa porta»*,
con l'alternativa scartata e la guardia sulla membership attiva — quindi la scelta non è
stata riderivata a posteriori; ma l'ordine è stato quello sbagliato, e chi rifà questo
passo lo faccia nell'ordine giusto.

**La numerazione ha anche prodotto una collisione.** Il piano della
[5-quater](#piano-utenti-veri) riservava `ADR-051` al codice di
recupero, dando per scontato che la 5-ter non producesse ADR. La 5 ne aveva già consumato
uno — `ADR-050`, il thread chiuso — quindi questa voce è la `051` e quella della 5-quater
è diventata la **`052`**, corretta nel piano nello stesso commit.

### Che cosa resta aperto

- **`R36`**, il badge degli addebiti, per la ragione scritta sopra;
- **le altre risposte che portano un identificatore verso una schermata.** La regolarità
  che il piano ha nominato al punto 3 — *ogni risposta che porta un identificatore verso
  una schermata è un difetto che aspetta di essere trovato* — non è stata verificata a
  tappeto: si sono sistemate le tre che il collaudo ha visto. Guardarle tutte resta una
  voce nuova;
- **`primary_tenant` non compare nell'elenco dei contratti.** Il campo c'è su
  `LeaseResponse` e lo usa solo la tendina del riaddebito. Mostrarlo anche in `LeasesPage`
  è probabilmente giusto e non è stato deciso qui;
- **la bolletta non nomina il proprio immobile.** `property_label` su
  `UtilityBillResponse` è quello del **contratto imputato**, che è ciò a cui il
  collegamento porta; l'immobile della bolletta resta un `property_id` senza nome, e nella
  scheda non compare affatto. Nessuno se n'è lamentato, ed è la ragione per cui non entra;
- **`R29` resta aperta**, con il precedente scritto sopra;
- **il residuo non si mostra sulla scheda della bolletta**, solo dentro il dialogo del
  riaddebito. È dove serve a decidere; metterlo anche nella `DescriptionList` è una riga e
  non è stata scritta perché nessuno l'ha chiesta.

### Che cosa non è stato provato a mano

Tutto, di nuovo, e qui pesa in modo diverso dalle sezioni 3, 4 e 5: **quattro voci su
cinque cambiano ciò che si vede**, e tre di esse — il badge, i nomi nei collegamenti, il
dialogo del riaddebito — si giudicano guardandole. I test coprono il comportamento e non
l'impressione: che «Occupata» da sola si legga meglio di «Occupata (Disponibile)» è
un'affermazione che solo un giro vero conferma.

**`Y5` è l'eccezione, e va detta al contrario**: è la sola voce il cui difetto un giro a
mano **non** troverebbe. Le due schermate sono identiche prima e dopo — il campo mostra
oggi in entrambi i casi — e a distinguerle è il corpo della richiesta. Lì il test non è un
sostituto del collaudo: è l'unico strumento che vede.

---

<a id="piano-utenti-veri"></a>

## Piano — Prima degli utenti veri: le quattro cose che mancano a una persona esterna

Pianificata il **10 settembre 2026**. Non chiude nessuna voce di
[12](12-da-fare.md), e la ragione è che **non sono difetti**: è lavoro mai cominciato,
che il criterio d'ordine di questo documento teneva in fondo *fino al momento in cui
qualcuno che non sia chi sviluppa apre il prodotto*. Quel momento è arrivato — il
collaudo l'ha aperto — e l'obiettivo dichiarato è più preciso di «utenti veri»:

> **un proprietario vero, che non ha scritto il codice, lo usa per un mese con i suoi
> dati.**

Da lì discende tutto ciò che entra e tutto ciò che resta fuori. Non serve ciò che serve a
mille sconosciuti; serve ciò senza cui **una** persona esterna si blocca e non torna.

**Emendata il 10 settembre 2026**, subito dopo l'esecuzione della
[5-ter](11-piani-conclusi.md#esito-sezione-5-ter) e rileggendo questo piano **contro il
codice** invece che contro sé stesso. È il controllo che alla 5-ter è mancato e le è
costato un passo: `Y4` poggiava su una premessa falsa — *«i dati ci sono già tutti in
`LeaseResponse`»* — che nessuno aveva verificato prima di eseguire. Qui le premesse sono
state verificate una per una e **tengono tutte**: `Notifier` è una porta senza adattatore e
non è iniettata da nessuna parte, `revoke_all_for_user` esiste, le rotte di autenticazione
sono quattro, `ops/web/Caddyfile` realizza già l'origine unica, `secure_cookies` è già
`true` per default, e la guardia della configurazione deriva davvero l'elenco da
`AppConfig`.

Quello che il controllo ha trovato sono **quattro cose che il piano non diceva**, e sono
già dentro: la posta che cade in silenzio (decisione 1), il tempo di risposta che
distingue un indirizzo noto da uno sconosciuto (decisione 5), il perimetro di `Z5`, e il
cambio password da autenticati, che **non** entra. Sono segnate dove valgono.

### La fase era già nominata, e per metà già pianificata

[Prima degli utenti veri](#prima-degli-utenti-veri) porta già i passi 9 e 10 del piano
archiviato — la porta `Notifier` e i due invii — con le loro decisioni e le loro trappole.
**Questo piano non le riscrive**: le assorbe, e aggiunge le tre cose che quella prosa non
prevedeva perché nate da una domanda diversa. Vanno rilette prima di `Z1`.

### Quattro pezzi, e uno dipende da un altro

L'ordine non è una preferenza:

```
Z1  la posta          ──┐
                        ├──► Z2  il recupero della password
                        │        (senza un canale, non c'è modo di recapitare niente)
Z3  il deploy         ──┴──► Z4  il backup
Z5  la privacy — piccola, e non rimandabile
```

**`Z2` è il blocco duro.** Le rotte di autenticazione sono quattro — `register`, `login`,
`refresh`, `logout` — e non c'è **nessun** modo di recuperare una password. Il primo «non
mi ricordo la password» del tuo proprietario finisce in una `UPDATE` scritta a mano da te,
di notte, sul database di produzione. E per recapitare un codice serve `Z1`.

### Che cosa è stato deciso prima di pianificare

| | Scelta | Alternativa scartata |
|---|---|---|
| Il canale | **SMTP**, il fornitore come dato di configurazione | l'API di un fornitore, che diventa un ramo di codice |
| La posta è obbligatoria? | **sì, l'avvio la pretende** | facoltativa, con un utente chiuso fuori e nessun modo di farlo rientrare |
| La forma del codice di recupero | **un codice da incollare**, come l'invito | un link cliccabile, con la credenziale nell'URL |
| Che cosa fa il recupero alle sessioni | **le revoca tutte** | lasciarle vive, e chi ha rubato l'account resta dentro |
| Che cosa risponde chi chiede il recupero | **la stessa cosa sempre** | dire se l'email esiste, cioè un oracolo sugli account |
| Il limite agli invii | **per email, nel modello** | per IP, che vuole uno stato condiviso e un servizio in più |
| La forma del deploy | **compose dietro Caddy su una VPS** | Kubernetes, che è la scala che non hai |
| Quando un backup è un backup | **quando è stato ripristinato una volta** | quando il cron non dà errori |

### Decisioni, con il perché

#### 1. La posta diventa obbligatoria all'avvio, e il recupero è la ragione

Oggi la configurazione della posta non esiste e non serve: il token d'invito torna nella
risposta HTTP e il client lo mostra con un bottone per copiarlo. Da `Z2` in poi **la posta
è l'unico canale** verso una persona chiusa fuori dal proprio account.

Quindi le impostazioni della posta entrano in `AppConfig` come **obbligatorie**, allo
stesso titolo di quelle dello storage. È la lezione già pagata tre volte e registrata
nella fetta C: un'impostazione mancante deve far fallire **l'avvio**, con un messaggio che
dice che cosa manca — non il primo uso, dove somiglia a un problema di rete.

*Alternativa scartata: un `Notifier` che non fa niente quando la posta non è configurata.*
Sembra prudente e produce il difetto peggiore di tutti: il recupero password che risponde
`202` e non manda niente, su un utente che non può più entrare.

**E la configurazione obbligatoria da sola non basta**, che è la contraddizione che
rileggere il piano ha trovato. I passi 9 e 10 che `Z1` assorbe prescrivono l'invio **dopo
il commit e best-effort**: con SMTP configurato ma *giù*, il recupero risponde `202` e non
manda niente lo stesso — stesso sintomo, causa diversa. Per l'invito non importa, perché il
token è **anche** nella risposta HTTP e lo consegna il proprietario a mano; per il recupero
non c'è un secondo canale, ed è tutta la ragione per cui `Z1` viene prima di `Z2`.

Il rimedio **non è dirlo al chiamante**: si tenta l'invio solo se l'indirizzo ha un
account, quindi una risposta che distingua un invio riuscito da uno fallito è l'oracolo
sugli account che la decisione 4 esiste per chiudere. E non è nemmeno l'invio sincrono, che
farebbe durare la richiesta quanto il server SMTP e finirebbe comunque in un `202` al primo
timeout.

**La raggiungibilità del server SMTP si verifica all'avvio e vive nel controllo di
salute.** L'invio resta best-effort dopo il commit, come per l'invito; ciò che cambia è che
un'indisponibilità è **nota prima** che qualcuno la incontri, invece di essere scoperta da
una persona chiusa fuori. La configurazione è già obbligatoria per la decisione qui sopra,
quindi il collegamento all'avvio non aggiunge un concetto: aggiunge una riga a `/health`.

Il limite va detto: il controllo di salute prova che il server risponde, non che il
messaggio verrebbe **recapitato** — quello lo dicono SPF, DKIM e DMARC, ed è la trappola
pratica più cara di questa fase.

#### 2. Il codice di recupero si incolla, non si clicca — ADR-052

[ADR-045](08-decisioni.md#adr-045) ha già deciso questa domanda per l'invito, e
l'argomento vale identico qui: **il token è una credenziale**, e un link
`/nuova-password/<token>` la scrive in tre archivi che nessuno ripulisce — la cronologia
del browser, il `Referer` verso qualunque risorsa esterna, e i log di ogni proxy davanti
al client.

Si fa quindi **come l'invito**: l'email porta un codice, la schermata ha un campo in cui
incollarlo. Coerenza con una decisione già presa, e nessun meccanismo nuovo — il codice
riusa `OpaqueTokenFactory` e `TokenHash`, che esistono per l'invito.

**Il costo va detto, perché è reale**: incollare un codice è più scomodo di un clic, e
succede nel momento in cui una persona è già frustrata. L'alternativa — il link, con la
pagina che ripulisce l'URL dalla cronologia e `Referrer-Policy: no-referrer` — chiuderebbe
due dei tre archivi e non il terzo, e introdurrebbe **due forme diverse** per due
credenziali che si somigliano. Si sceglie la coerenza; se il collaudo dirà che l'attrito è
troppo, sarà una voce nuova con una misura dietro invece di un'ipotesi.

#### 3. Cambiare la password revoca tutte le sessioni

È la parte che si dimentica, ed è quella che rende il recupero una difesa invece di una
comodità: chi ha ottenuto l'accesso a un account ha in mano un refresh token che vive
**trenta giorni**, e cambiare la password senza revocarlo non lo butta fuori.

`SessionRepository::revoke_all_for_user` **esiste già** — la usa `RefreshSession` quando
rileva il riuso di un token ruotato — quindi è una chiamata, non un meccanismo.

#### 4. Chiedere il recupero non dice se l'email esiste

Stessa risposta per un indirizzo noto e per uno sconosciuto. È la regola di
[ADR-017](08-decisioni.md#adr-017), che il modulo degli inviti applica già: distinguere
sarebbe un oracolo per capire chi ha un account.

Ne segue una cosa da scrivere nel client e non da lasciare intuire: la schermata dice «se
quell'indirizzo ha un account, riceverà un codice», che è la verità e non una scusa.

#### 5. Il limite agli invii è per email, e sta nel modello

Un recupero senza limiti è un modo per far arrivare cinquanta email a un indirizzo
altrui. Il rimedio **non è il rate limiting per IP** — che è nell'elenco dei trasversali
mancanti, vuole uno stato condiviso e non serve con una sola istanza: è un limite per
**email**, cioè una riga che dice quando è stato emesso l'ultimo codice e un rifiuto
silenzioso se è troppo recente.

Costa una colonna e nessun servizio in più. Il rate limiting per IP resta dov'è.

**Ma il limite per email apre una differenza che la decisione 4 chiude solo a metà**, e
va chiusa qui. Per un indirizzo **senza** account non si crea nessuna riga: la risposta è
identica — è la decisione 4 — mentre il costo no. Un ramo fa una lettura; l'altro fa una
lettura, calcola uno hash e inserisce. [ADR-017](08-decisioni.md#adr-017) governa il
**contenuto** della risposta, non la sua durata, e un tempo che distingue è un oracolo come
un codice di errore che distingue.

**Il percorso è quindi a tempo costante**: anche per un indirizzo sconosciuto si calcola
lo hash di un valore fittizio, così i due rami costano uguale. Poche righe, e chiudono la
differenza dove nasce invece di lasciarla scritta come rischio accettato — che con una
persona sola sarebbe stato tollerabile e sarebbe rimasto lì per sempre, come le finestre
che la `0030` aveva lasciato aperte per iscritto.

#### 6. Il deploy è `compose` dietro Caddy, e la parte difficile è già fatta

`ops/web/Caddyfile` realizza già il contratto **una sola origine** — `/` ai file statici,
`/api/v1` all'API — che è la metà dell'argomento per cui non c'è CORS da montare né una
difesa CSRF da scrivere ([ADR-038](08-decisioni.md#adr-038)). In produzione cambia una
riga: `:8080` diventa il dominio, e Caddy prende il certificato da sé.

*Alternativa scartata: Kubernetes.* I manifest sono nell'elenco di ciò che manca, e
restano lì: sono la scala che non hai, e ogni ora spesa lì è un'ora non spesa a scoprire
se il prodotto serve a qualcuno.

**`secure_cookies` torna a `true`**, che è già il default del file di configurazione: su
HTTPS il cookie del refresh è `Secure`, e il `false` resta l'eccezione dello sviluppo su
`http://localhost`.

#### 7. Un backup è un backup solo dopo essere stato ripristinato una volta

Un `pg_dump` che gira e non dà errori non prova niente: prova che il comando esiste. La
prova è **ripristinarlo in un database vuoto e far partire il prodotto sopra**, e va fatta
una volta prima che dentro ci siano dati di qualcun altro.

Due cose e non una: il database **e il bucket**. I documenti sono oggetti in MinIO, non
righe: un dump del database li lascia fuori, e la riga che li cita resta senza byte — è la
stessa constatazione che `just down-hard` ha già prodotto una volta.

E la copia **esce dalla macchina**. Un backup sullo stesso disco del database non è un
backup, è una seconda copia dello stesso guasto.

#### 8. La privacy è piccola adesso e cara dopo

Appena i dati dei suoi inquilini entrano nella tua VPS — nome, email, telefono — stai
trattando dati personali di terzi. Per un test fra conoscenti non serve un impianto: serve
dire per iscritto che cosa raccogli, perché, dove sta e che lo cancelli su richiesta.

Sta nel piano e non fuori perché è l'unica voce che **cresce di costo con il tempo**:
scriverla adesso è un pomeriggio, scriverla dopo aver raccolto dati per sei mesi è
un'operazione su dati che esistono già.

**La cancellazione è un runbook e non una rotta, e la ragione è nello schema.** La prima
stesura di questo passo diceva *«deve esistere una risposta che non sia una `DELETE` a
mano»*, e chiedeva una capacità che il progetto **vieta**: il ruolo applicativo non ha il
privilegio `DELETE` ([ADR-003](08-decisioni.md#adr-003), regola 7), tutto passa da
`deleted_at`, e `audit_log` e gli eventi di dominio continuano a citare le righe. Una
cancellazione vera non si soddisfa con un soft delete, e farla diventare una capacità di
prodotto qui vorrebbe dire decidere che cosa significa il verbo su righe che altre tabelle
nominano — cioè una migration, un ADR e la stessa domanda che `R15` ha già isolata.

`Z5` produce quindi **la pagina e un runbook**, eseguito con il ruolo del migratore e
percorso una volta per intero, come il ripristino di `Z4` e per la stessa ragione: una
procedura mai eseguita non è una procedura. La pagina dichiara un **tempo di risposta**
invece di promettere un bottone, che è la verità e non una scusa. La capacità di prodotto
diventa una voce di [12](12-da-fare.md) e si pianifica quando qualcuno la chiede davvero.

*Alternativa scartata: solo la pagina, e la procedura da inventare alla prima richiesta.*
È più piccola del runbook di mezza giornata e sposta il lavoro nel momento peggiore — con
una persona che aspetta, e su dati che nel frattempo sono cresciuti.

### Decomposizione

##### `Z1` — La posta

I passi 9 e 10 di [Prima degli utenti veri](#prima-degli-utenti-veri), da rileggere per
intero: la porta esiste, l'adattatore no, e `Notifier` **non è iniettata da nessuna
parte**.

Qui si aggiungono le tre cose che quella prosa non aveva:

- **le impostazioni sono obbligatorie** in `AppConfig`, con il messaggio di avvio che dice
  che cosa manca. Un'impostazione nuova tocca **quattro** posti e non due: oltre a
  `AppConfig` e alle tabelle di [07](07-operazioni.md) — che è la guardia che fa fallire
  `cargo test` — servono `.env.example`, senza cui `just env` smette di dire che cosa
  manca a chi ha un `.env` più vecchio, e il servizio `api` del `docker-compose.yml`;
- **un raccoglitore locale nel `docker-compose.yml`** per lo sviluppo, e la configurazione
  di un mittente vero per la produzione, che è cosa diversa — vedi le trappole;
- **la raggiungibilità del server SMTP nel controllo di salute**, per la decisione 1: è
  ciò che rende visibile una posta caduta **prima** che la incontri una persona chiusa
  fuori dal proprio account.

##### `Z2` — Il recupero della password — **ADR-052 prima**

Migration nuova: la tabella dei codici di recupero — hash e non il codice, scadenza breve,
uso singolo, e la colonna che regge il limite per email.

Due rotte, `POST /api/v1/auth/password-reset` e
`POST /api/v1/auth/password-reset/confirm`; due schermate nel client; il codice consegnato
per email.

**Il pool privilegiato è legittimo qui, ed è l'unico posto nuovo in cui lo diventa.** La
regola 27 lo riserva ai flussi in cui un contesto di sicurezza **non può** esistere:
registrazione, login, rotazione del refresh. Chi ha dimenticato la password non è
autenticato per definizione. Va scritto nel commento del caso d'uso, o alla prossima
lettura sembrerà una violazione.

**Il percorso è a tempo costante** (decisione 5): per un indirizzo sconosciuto si calcola
comunque lo hash di un valore fittizio, così i due rami non si distinguono per durata.

**Il cambio password da autenticati non entra**, ed è la cosa che il titolo di questo passo
invita a dare per fatta. Dopo `Z2` chi *è* dentro continua a non poter cambiare la propria
password: quella rotta sta in [`R7`](12-da-fare.md#interfaccia-e-prodotto), sezione 6.
Coerente con il perimetro dichiarato — questa fase serve a chi **non può entrare**, non a
chi è già dentro — e va scritto perché condivide con il recupero la revoca delle sessioni e
la validazione, quindi sembra incluso.

Quattro test, e sono i quattro che contano: la conferma **revoca le sessioni**; un
indirizzo sconosciuto riceve la stessa risposta di uno noto; un codice già usato non vale
due volte; e un secondo codice chiesto troppo presto viene rifiutato in silenzio.

##### `Z3` — Il deploy

VPS, dominio, il `Caddyfile` che punta al dominio, i segreti fuori dal repository, il
migratore prima dell'API, e la politica di riavvio.

**`Strict-Transport-Security` va aggiunto a mano**: Caddy prende il certificato da sé e
reindirizza a HTTPS, ma quell'intestazione non la mette. Su un sito che tratta dati
personali di terzi è la differenza fra «reindirizza» e «non ci si arriva più in chiaro».

Il criterio non è «i container sono su»: è **aprire il prodotto dal telefono, in 4G, e
fare l'accesso**. Finché non è successo, non è deployato.

##### `Z4` — Il backup, e il ripristino provato

`pg_dump` a orario e copia del bucket, cifrati, **fuori dalla macchina**, con una
ritenzione dichiarata. Poi il ripristino, una volta, su un database vuoto.

##### `Z5` — La privacy

Una pagina raggiungibile dal prodotto: che cosa si raccoglie, perché, dove sta, per quanto
e come si chiede la cancellazione, **con un tempo di risposta dichiarato** invece di un
bottone promesso.

E il **runbook** della cancellazione, per la decisione 8: i passi da eseguire con il ruolo
del migratore, percorsi una volta per intero prima che dentro ci siano dati di qualcun
altro — come il ripristino di `Z4`, e per la stessa ragione. Una procedura mai eseguita non
è una procedura.

La cancellazione **come capacità del prodotto** non entra: richiede di decidere che cosa
significhi il verbo su righe che `audit_log` e gli eventi continuano a citare, ed è la
stessa domanda di [`R15`](12-da-fare.md#cancellare-in-generale). Entra invece in
[12](12-da-fare.md) come voce sua.

### Trappole

**La posta da una VPS nuova finisce nello spam, ed è la trappola pratica più cara di
questa fase.** Un IP appena assegnato non ha reputazione, e senza SPF, DKIM e DMARC sul
dominio l'email di recupero non arriva — e il sintomo è **muto**: nessun errore, il codice
non compare. Non è un difetto del codice ed è il motivo per cui l'adattatore parla SMTP e
non con l'API di qualcuno: il mittente vero è un dato di configurazione. Va provato **con
un indirizzo vero**, non solo con il raccoglitore locale.

**Un raccoglitore locale non è un fornitore.** Accetta tutto: non verifica SPF, DKIM né i
rimbalzi. Prova che il messaggio parte e che cosa contiene, non che sarebbe recapitato. E
**il controllo di salute prova ancora meno**: dice che il server risponde, non che il
messaggio arriverebbe.

**Il recupero non può dire se l'invio è riuscito, e nemmeno metterci più tempo.** Le due
metà si dimenticano insieme: si tenta l'invio solo per un indirizzo che ha un account,
quindi *qualunque* cosa distingua i due rami — un codice di risposta, un messaggio, una
durata — è l'oracolo che la decisione 4 esiste per chiudere. La risposta è uniforme per
decisione 4, il costo lo è per decisione 5, e chi tocca uno dei due rami deve guardare
l'altro.

**Aggiungere un'impostazione senza la riga in [07](07-operazioni.md) fa fallire
`cargo test`.** `crates/api/tests/docs_config_reference.rs` deriva l'elenco dai campi di
`AppConfig` e lo confronta con quelle tabelle. Non è un fastidio: è la ragione per cui
`PROPFLOW__TELEMETRY__FILTER` non è più documentato con il default di due versioni fa.

**L'invito non può passare dall'outbox**, e vale identico per il codice di recupero. Il
token in chiaro esiste **solo** nella risposta e nell'email: in database c'è lo hash, e gli
eventi di dominio non trasportano valori né credenziali. Un worker che leggesse l'evento
non avrebbe niente da spedire, e metterci il token dentro significherebbe scrivere una
credenziale in una tabella fatta per essere pubblicata su un broker.

**L'adattatore SMTP è il posto più facile in cui far uscire dati personali.**
[04](04-sicurezza.md) vieta email, token e corpi nei log, e un client SMTP che registri la
richiesta li registra tutti e tre insieme. La porta ha già un `Debug` redatto e
l'adattatore non deve annullarlo.

**Il recupero senza revoca delle sessioni non è un recupero.** Un refresh token vive
trenta giorni: cambiare la password e lasciarlo vivo lascia dentro chi c'era.

**`just down-hard` sul server porta via tutto**, MinIO compreso. Sulla macchina di
produzione quel comando non deve esistere nella memoria muscolare.

**MinIO e PostgreSQL sullo stesso disco cadono insieme.** Il backup deve coprire entrambi,
e stare altrove.

**Non si aggiunge Redis.** Il limite agli invii è per email e sta nel modello; il rate
limiting per IP resta fra i trasversali mancanti. Un servizio in più è un servizio da
avviare, mettere in sicurezza, monitorare e ripristinare.

### Come si saprà che è finita

- un'email di invito arriva davvero nel raccoglitore locale, con il token nel corpo e in
  **nessuna** riga di log;
- un'email arriva a un indirizzo **vero**, e non nella cartella dello spam;
- con il server SMTP **fermo**, il controllo di salute lo dice — invece di lasciarlo
  scoprire a chi è chiuso fuori dal proprio account;
- una password si recupera da capo a fondo, e le sessioni aperte prima **non funzionano
  più**;
- chiedere il recupero per un indirizzo che non esiste dà la stessa risposta di uno che
  esiste, e il client lo dice invece di lasciarlo intuire;
- il prodotto risponde su un dominio in HTTPS, aperto **dal telefono in 4G**;
- un dump viene ripristinato in un database vuoto e il prodotto ci parte sopra;
- esiste una pagina che dice che cosa si raccoglie, come si chiede la cancellazione e in
  quanto tempo, e il **runbook** che la soddisfa è stato percorso una volta per intero;
- `just ci` verde.

### Che cosa **non** entra

- **il rate limiting per IP**, la verifica dell'email bloccante e l'MFA. La prima non serve
  con una sola istanza; la seconda cambia il percorso di registrazione, che ogni collaudo
  attraversa, e pagarne il costo mentre si prova da soli è il caso più netto del criterio
  d'ordine; la terza è una colonna e nient'altro;
- **Kubernetes, metriche, alerting, il cron delle partizioni di audit.** Dipendono da dove
  il prodotto verrà esercito, che dopo questa fase sarà deciso — una VPS — e allora
  saranno pianificabili con una misura invece che con un'ipotesi;
- **`R9`, il perimetro dei ruoli.** Serve prima degli **sconosciuti**, non prima di un
  proprietario che conosci: oggi chiunque si registri è owner di una propria
  organizzazione, e con una persona sola è tollerabile e va saputo;
- **la sezione 5-ter**, che è lavoro suo e viene prima di questa fase: `R31` in particolare
  romperebbe il test appena il tuo proprietario rinnovasse un contratto;
- **il cambio password da autenticati.** Sta in [`R7`](12-da-fare.md#interfaccia-e-prodotto),
  sezione 6, e non qui: questa fase serve a chi **non può entrare**. Va detto perché
  condivide con `Z2` la revoca delle sessioni e la validazione, quindi sembra incluso;
- **la cancellazione come capacità del prodotto**, che `Z5` risolve con un runbook e che
  diventa una voce di [12](12-da-fare.md): decidere che cosa significhi cancellare su righe
  che `audit_log` e gli eventi continuano a citare è la domanda di
  [`R15`](12-da-fare.md#cancellare-in-generale), non un dettaglio di una pagina;
- **il modello di ricavo.** Questa fase esiste per poter fare la domanda, non per
  rispondere.

---

<a id="esito-utenti-veri"></a>

## Esito — Prima degli utenti veri: le quattro cose che mancano a una persona esterna

Eseguita il **10 settembre 2026**. Non chiude nessuna voce di [12](12-da-fare.md), come
il piano prevedeva: non erano difetti, era lavoro mai cominciato. Ne apre una, `R37`.

**Due sezioni su quattro sono fatte; due sono preparate e non eseguite**, e la
distinzione è la cosa più importante di questo esito. `Z1` e `Z2` sono codice, girano e
hanno i loro test. `Z3`, `Z4` e `Z5` hanno prodotto **artefatti e un runbook**: il
Caddyfile di produzione, il compose, gli script di copia e di ripristino, la pagina della
privacy. Il `docker compose config` passa e gli script passano `bash -n`; **la macchina,
il dominio e le credenziali non esistono**, quindi nulla di quel percorso è stato
percorso. Non è deployato.

Un ADR, una migration, i test Rust da **443 a 449**, quelli del client da 102 a **104**.

### Che cosa è entrato

| Passo | Esito |
|---|---|
| `Z1` | Adattatore SMTP, configurazione **obbligatoria** all'avvio, `mailpit` nel compose, e la posta dentro `/ready` |
| `Z2` | Migration `0037`, due rotte, una schermata, [ADR-052](08-decisioni.md#adr-052) scritta **prima** del codice |
| `Z3` | `ops/deploy/Caddyfile` e `docker-compose.prod.yml`, **validati e non eseguiti** |
| `Z4` | `ops/deploy/backup.sh` e `restore.sh`, **scritti e non eseguiti** |
| `Z5` | La pagina della privacy, raggiungibile senza sessione, e il runbook della cancellazione |

### Il test che asseriva la cosa sbagliata, e che cosa ha trovato

La decisione 2 di ADR-052 dice «confermare revoca **tutte** le sessioni». Il test lo
verificava nel modo ovvio: la vecchia sessione non deve più leggere niente. **Fallisce**,
e non per un difetto del codice.

`revoke_all_for_user` revoca i **refresh token**. L'access token è un JWT firmato e **non
è revocabile** ([ADR-012](08-decisioni.md#adr-012)): chi ne ha uno in mano resta dentro
fino alla scadenza, quindici minuti. La catena si chiude — nessun rinnovo, e da lì in poi
è fuori — ma la finestra esiste.

È il baratto che ADR-012 aveva già scelto, e il recupero della password non lo cambia;
quello che cambia è che adesso è **scritto**, perché «revoca tutte le sessioni» si legge
come «lo butta fuori adesso» e non è vero. La voce di ADR-052 lo dice, e il test asserisce
la cosa giusta: il refresh non rinnova più. **Un test scritto per confermare una decisione
ne ha trovato il confine**, che è il rendimento che si spera da un test e che quasi mai
si vede.

### Il `202` che il client non sapeva leggere

`ApiClient.decode` trattava come «senza corpo» il solo `204`. La richiesta di recupero
risponde `202` e **niente**, perché deve rispondere lo stesso a chiunque: `json()` su un
corpo vuoto sollevava, e l'errore parlava di JSON malformato su una richiesta riuscita.

Il rimedio non è aggiungere `202` all'elenco — un elenco va aggiornato al prossimo codice,
e la volta dopo ce se ne dimentica — ma guardare il **tipo di contenuto**. È un difetto
che esisteva da prima e che nessuna rotta aveva ancora incontrato.

### Che cosa il piano non aveva previsto

**`Notifier::probe` sta sulla porta e non sull'adattatore.** Il piano diceva «SMTP nel
controllo di salute» senza dire dove vivesse il metodo. Sull'adattatore avrebbe obbligato
`AppState` a tenere un `Arc<SmtpNotifier>` concreto, cioè a far conoscere
`propflow-infrastructure` al bordo HTTP fuori da `main.rs` — la regola delle dipendenze,
rotta per una sonda. «Il canale è raggiungibile?» è una domanda che appartiene alla porta.

**Il limite agli invii non ha avuto bisogno di una colonna.** Il piano ne prevedeva una,
«quando è stato emesso l'ultimo codice»: è il `created_at` della riga più recente, che
l'indice su `(user_id, created_at DESC)` rende una lettura sola. Una colonna che
duplicasse quel valore avrebbe due fonti di verità per lo stesso fatto.

**`PasswordResetCode::mark_used` restituisce un `bool` e non un errore tipizzato.** La
forma del progetto sarebbe una variante di `Denial` con il suo codice; qui non arriverebbe
mai a nessuno, perché sconosciuto, scaduto e già usato danno tutti `401` per non fare da
oracolo. Un codice d'errore da documentare per un caso che non si osserva è peso senza
contropartita.

**`Entity` non ha una variante `Organization`**, e serviva per leggere il nome
dell'organizzazione da mettere nell'email dell'invito. Non si è aggiunta: chi invita è
staff di quella organizzazione e `organizations_select` è `is_org_member(id)`, quindi un
`None` lì è un difetto nostro e non una riga che il chiamante non vede.
`AppError::internal`, come già fa `AccepInvitation` per la parte contrattuale mancante.

**`${var:?parola}` e l'apostrofo.** In bash l'apostrofo dentro quella espansione apre una
stringa: `${1:?serve il percorso dell'archivio}` rompe lo script prima che venga eseguito,
e `bash -n` lo dice con un messaggio che punta cinquanta righe più in basso. Il controllo
è diventato un `if`, con il perché scritto accanto.

### Decisioni prese eseguendo

**Il testo delle email sta nell'adattatore, in italiano, e in testo semplice.** Nel
service layer sarebbe un template dentro un caso d'uso, che è precisamente ciò che
`NotificationContent` esiste per evitare. In italiano perché è ciò che una persona legge,
e la regola dell'inglese di `CLAUDE.md` §3 riguarda ciò che si incontra **fuori** dal
repository. Testo semplice perché un'email che porta un codice da incollare non guadagna
niente da una resa ricca, e ogni client di posta la mostra allo stesso modo.

**Il recupero è una schermata sola per due tempi.** Si chiede il codice, si incolla: fra i
due passaggi non c'è niente da ricordare tranne l'indirizzo, e mandare su una seconda
pagina una persona già frustrata è attrito che non compra niente. L'indirizzo resta a
schermo, così chi lo ha digitato male se ne accorge lì invece di aspettare un'email che
non arriverà.

**La conseguenza della revoca si legge prima di premere.** Cambiare la password chiude le
sessioni ovunque, telefono compreso, e scoprirlo dopo sembra un guasto.

**Swagger UI non si espone in produzione**, e il Caddyfile risponde `404` su `/docs`. Non
era nel piano: in sviluppo è comodo, in produzione è l'elenco completo delle rotte servito
a chiunque passi.

**`PROPFLOW__SERVER__SECURE_COOKIES` non compare nel compose di produzione.** Il default è
già `true`; dichiararlo sarebbe un posto in più da cui qualcuno può metterlo a `false` per
far funzionare una prova.

**Il dump lo fa il ruolo del migratore.** Con quello applicativo `pg_dump` gira sotto Row
Level Security e produce un backup con **zero righe di business**, senza dare errori — che
è il modo peggiore di sbagliare un backup.

### Che cosa resta aperto

- **il deploy non è avvenuto.** Il runbook di [07](07-operazioni.md) è in sei passi e
  nessuno è stato eseguito. La riga che conta è la quarta: *finché non si è aperto il
  prodotto dal telefono, in 4G, non è deployato*;
- **il ripristino non è stato provato**, e finché non lo è non c'è un backup: c'è un file.
  Lo script dice da sé i tre controlli da fare dopo, e il terzo — scaricare un documento —
  è quello che si scopre mancante mesi dopo;
- **la posta non è mai arrivata a un indirizzo vero.** `mailpit` prova che il messaggio
  parte e che cosa contiene, non che sarebbe recapitato: SPF, DKIM e DMARC non si
  verificano da qui, e su un dominio nuovo il sintomo è muto;
- **`R37`, la cancellazione come capacità del prodotto.** `Z5` la risolve con un runbook
  eseguito a mano dal ruolo del migratore, e la pagina lo dichiara invece di promettere un
  bottone;
- **il cambio password da autenticati** resta in `R7`, sezione 6;
- **i codici di recupero scaduti restano in tabella**, inerti: il ruolo applicativo non ha
  `DELETE`. La potatura è un'operazione del runbook;
- **nessun alerting**, compreso quello sul `/ready` che dice «posta non raggiungibile»:
  oggi lo vede solo chi guarda.

### Che cosa non è stato provato a mano

`Z1` e `Z2` solo con i test, come le sezioni 3, 4, 5 e 5-ter. `Z3`, `Z4` e `Z5` **non
sono stati provati affatto**, e non per scelta: richiedono una macchina, un dominio e
delle credenziali. Gli artefatti sono validati per quello che si può validare senza —
`docker compose config`, `bash -n` — e questo non è la stessa cosa di funzionare.

**È la prima volta che questo progetto consegna qualcosa che non ha visto girare**, e
vale la pena scriverlo: le sezioni precedenti finivano con una suite verde, questa
finisce con un elenco di comandi da eseguire. Il criterio di fine non è «gli artefatti
esistono»: è il punto 4 del runbook.

---

<a id="piano-sezione-6"></a>

## Piano — Sezione 6: le persone

Pianificata il **10 settembre 2026**, dopo il rinvio del deploy. Chiude `R7`, `R16` e
`R38`. Le prime due erano in elenco; la terza è **uscita verificando le premesse** di
questo piano contro il codice, ed è il motivo per cui la sezione ha una migration.

**È la prima sezione pianificata senza il collaudo davanti.** Le tre precedenti sono nate
da un giro vero; questa nasce da una lettura, e va saputo: il rendimento del giro era sei
voci in mezza giornata contro le quattro di cinque sezioni di lettura. Ciò che segue è un
ragionamento, non una misura.

### Che cosa esiste davvero, verificato e non dedotto

| Premessa | Verifica |
|---|---|
| Nessuna rotta legge o scrive l'utente corrente | vero: `crates/api/src/routes/` non ha `/users/me` né equivalenti |
| `users_update_self` esiste già | vero, `0014`: `USING (id = current_user_id() AND deleted_at IS NULL)`, `WITH CHECK (id = current_user_id())` |
| `UserRepository::update` è chiamato solo dal pool privilegiato | vero: `Login` e `ConfirmPasswordReset`, entrambi `begin_privileged()` |
| Un membro **non** può toccare la propria membership | vero: `org_members_update` è `is_org_staff(org_id)` su USING **e** WITH CHECK |
| L'uscita è rappresentabile senza migration | **falso a metà**: `status` e `deleted_at` ci sono, ma la policy non la ammette e `role` resterebbe scrivibile |
| `app.users` è coperta dalle invarianti dei privilegi di colonna | **falso**: né `colonna-di-confine-scrivibile` né `colonna-strutturale-scrivibile` la nominano |

### Il difetto che pianificare ha trovato — `R38`

`users_update_self` ha una `WITH CHECK` che nomina **la sola identità della riga**. È
letteralmente la constatazione della [`0029`](08-decisioni.md#adr-026) su una quinta
tabella: ciò che il `WITH CHECK` non nomina resta libero, e sul ruolo applicativo non c'è
nessuna `GRANT UPDATE` di colonna su `app.users`.

La più cara è **`platform_role`**, e la catena va scritta perché non si vede da una riga
sola:

```
UPDATE app.users SET platform_role = 'platform_admin' WHERE id = app.current_user_id()
   → Login legge users.platform_role e lo mette nei claim
   → il middleware chiama with_platform_admin(...)
   → l'unità di lavoro installa app.platform_admin = 'on'
   → app.is_platform_admin() è vera
   → is_org_member e is_org_staff sono vere su OGNI organizzazione
```

Seguono `status`, con cui ci si toglie una sospensione, e `failed_login_count` con
`locked_until`, che sono la difesa dal blocco dopo dieci tentativi descritta in
[04](04-sicurezza.md#blocco-dopo-tentativi-falliti).

**Non è sfruttabile dall'API**, e va detto con la stessa precisione: nessuna rotta scrive
quelle colonne, e chi non possiede le credenziali del ruolo applicativo non può emettere
quella query. È una difesa in profondità mancante, esattamente della classe che ADR-021 e
ADR-026 hanno già chiuso quattro volte — *una regola applicativa non è un confine*.

**Entra in questa sezione e non in una sua** per una ragione di tempi: `R7` aggiunge la
**prima rotta che scrive `app.users` dal pool applicativo**. Finché non esiste, la porta è
chiusa da un privilegio che nessuno usa; da quando esiste, è chiusa solo da un `UPDATE`
scritto bene.

### Che cosa è stato deciso prima di pianificare

| | Scelta | Alternativa scartata |
|---|---|---|
| Come si raggiunge il proprio account | **`/api/v1/me`, che non prende mai un identificatore** | `PATCH /users/{id}`, che riaprirebbe ADR-036 |
| Che cosa si può correggere | **nome, telefono, lingua** | anche l'email, che è la credenziale d'accesso |
| Cambiare la password da dentro | **serve quella vecchia**, e revoca **tutte** le sessioni | tenere viva la corrente, che sono due regole invece di una |
| Che cosa protegge `app.users` | **privilegi di colonna**, più le invarianti | una `WITH CHECK` più larga, che non può confrontare vecchio e nuovo |
| Come si esce da un'organizzazione | **`status = 'left'` sulla propria riga**, negato con un contratto in essere | un soft delete della membership |
| Chi garantisce il divieto | **una policy**, non un `if` | il solo service layer |

### Decisioni, con il perché

#### 1. `/api/v1/me`, e mai un identificatore

`GET` e `PATCH` su `/api/v1/me`. Il percorso non prende mai un `user_id`, ed è la forma
che tiene chiusa la domanda di [ADR-036](08-decisioni.md#adr-036): **nessuna rotta proietta
un utente per identificatore**. Una `PATCH /users/{id}` che accettasse solo il proprio
sarebbe la stessa cosa con una guardia applicativa davanti — cioè la porta aperta e un
cartello.

`GET /me` restituisce ciò che serve a una schermata del proprio profilo: identificatore,
email, nome, telefono, lingua, e se l'email risulta verificata. **Non** il ruolo di
piattaforma, che non è affare di chi guarda.

#### 2. L'email non si cambia in questa sezione, e la ragione è scritta

È la scelta che sembra una mancanza e non lo è. `users.email` non è un dato anagrafico: è
**la credenziale d'accesso**, ed è la chiave con cui tre cose si ritrovano.

- `app.has_pending_invitation` fa `JOIN app.users u ON u.email = i.email`: cambiare
  l'email **rompe in silenzio** ogni invito pendente per quell'indirizzo;
- `app.current_user_email()` è ciò su cui poggiano quattro policy della `0018`;
- `lease_parties.email` è come l'accettazione trova la propria riga.

Cambiarla bene richiede un giro di verifica sul nuovo indirizzo — cioè la verifica
dell'email, che è **deliberatamente fuori perimetro** — e una decisione su che cosa
succede agli inviti pendenti. Farla male significa un account che non riceve più niente e
un invito che non si può più accettare, con un sintomo muto in entrambi i casi.

*Alternativa scartata: permetterla e basta.* È mezza sezione di lavoro e una classe di
guasti che si manifesta giorni dopo, su una funzione che nessuno ha chiesto.

#### 3. Cambiare la password da dentro **pretende quella vecchia**

`POST /api/v1/me/password`, con `current_password` e `new_password`. La vecchia serve
perché il contesto è diverso da quello del recupero: là la prova di identità è il controllo
della casella di posta, qui è una sessione — e una sessione può essere una scheda lasciata
aperta. Senza la vecchia, chiunque si sieda davanti a un browser non bloccato si prende
l'account per sempre.

**Revoca tutte le sessioni, compresa la propria**, esattamente come
[ADR-052](08-decisioni.md#adr-052) fa per il recupero. La scelta è l'uniformità, ed è stata
presa contro l'alternativa che sembra più gentile — tenere viva la sessione da cui si sta
cambiando, riconoscendola dal cookie di refresh che la richiesta porta già.

Il ragionamento, perché è la decisione meno ovvia della sezione: *«cambiare la password
chiude tutto»* è **una frase sola**, e questa frase va scritta in almeno quattro posti — la
schermata del recupero, quella del proprio account, `docs/04-sicurezza.md` e il corpo
dell'API. Due regole diverse per due percorsi che una persona non distingue — *«ho cambiato
la password»* è la stessa cosa dai due lati — sono due frasi da tenere allineate, e la
seconda diverge alla prima modifica. L'attrito che si paga è piccolo e visibile: chi cambia
rientra subito con la password che ha appena scelto.

**Ne segue che questa sezione non ha bisogno di un ADR nuovo.** Con la revoca selettiva
sarebbe servito — una decisione che diverge da una documentata è una voce in
`docs/08-decisioni.md`, scritta prima del codice — e con l'uniformità ADR-052 basta a
spiegare entrambi i percorsi.

Vale anche qui la nota di ADR-052 che si dimentica: si revocano i **refresh token**, non
gli access token, che sono JWT e vivono i loro quindici minuti. Chi cambia la password non
è fuori nell'istante in cui la cambia; è fuori quando quel token scade.

#### 4. `app.users` si difende con i privilegi di colonna

Una migration che revoca `UPDATE` su `app.users` al ruolo applicativo e la riconcede sulle
sole colonne che il prodotto scrive **da una richiesta autenticata**:

| Concesse | Negate |
|---|---|
| `full_name`, `phone`, `locale`, `password_hash` | `platform_role`, `status`, `email`, `email_verified_at`, `last_login_at`, `failed_login_count`, `locked_until`, `deleted_at` |

Le negate non sono irraggiungibili: le scrive il pool **privilegiato**, che ha `BYPASSRLS`
e i privilegi pieni — `Login` aggiorna `last_login_at` e il contatore dei tentativi, il
recupero password scrive `password_hash`. È la stessa divisione delle quattro migration
che l'hanno preceduta: ciò che una richiesta autenticata può scrivere è un elenco corto e
dichiarato, tutto il resto passa da un flusso che il contesto di sicurezza non ha.

Le colonne entrano nelle invarianti di `ops/sql/checks`: `platform_role` e `status` in
`colonna-di-confine-scrivibile`, perché definiscono che cosa si vede; le altre in
`colonna-strutturale-scrivibile`. **Le invarianti si aggiornano nello stesso passo della
migration**, che è la forma che la sezione 5 ha stabilito emendando un controllo di CI.

#### 5. Uscire è uno **stato**, non un soft delete

`status = 'left'` su `app.org_members`, non `deleted_at`. Le due cose non sono
intercambiabili: `OrgMemberRepository::find` filtra `deleted_at IS NULL`, quindi una
membership cancellata **non si vede più** — e `org_members_org_user_uk` è unico e non
parziale, quindi chi è uscito così non potrebbe più rientrare né con un `INSERT` né con un
`UPDATE` su una riga che non si trova. È esattamente il difetto che la `0029` ha già
pagato sulle spese co-living.

Con lo stato la riga resta, e qui il piano deve dire una cosa che non si vede dal nome
della voce: **così com'è, chi esce non rientra lo stesso.**
[ADR-051](08-decisioni.md#adr-051) fa cercare a `AcceptInvitation` una membership
**attiva**; su una riga `left` non la trova e prova a inserirne una, che sbatte contro
`org_members_org_user_uk` — unico e non parziale — e diventa `409
membership.already_exists`. È esattamente il `409` che quella sezione aveva chiuso, che
ricompare da un'altra porta.

ADR-051 lo aveva previsto per iscritto — *«il giorno in cui `R16` arriva questo punto
diventerebbe la porta di servizio»* — e quel giorno è questo. **`AcceptInvitation` va
esteso in questa sezione**: una membership non attiva si **riattiva** invece di inserirne
una nuova. Senza quel ramo, `R16` produce persone che escono e non possono più tornare, e
il sintomo arriverebbe mesi dopo su un invito che sembra buono.

#### 6. Il divieto è una policy, non un `if`

«Non si esce con un contratto in essere» va **verificato e garantito**, ed è la lezione
della `0029`: là rendere l'uscita un confine vero ha richiesto una policy, perché una
regola applicativa si legge come una garanzia proprio quando è scritta in tre posti.

Serve quindi una funzione — `app.has_lease_in_force(p_org_id, p_user_id)`, `SECURITY
DEFINER` con `search_path` come tutte le altre — che passi da `lease_parties.user_id` e da
`app.lease_in_force`, cioè dalla **stessa** definizione di «in essere» del resto del
progetto e non da una seconda.

E la `WITH CHECK` della policy nuova deve nominare **`role`**. Oggi `org_members_update` è
staff soltanto, quindi il ruolo è al sicuro per costruzione; dal momento in cui un membro
può toccare la propria riga, `role` è la colonna con cui si promuoverebbe a `owner`. Una
policy non può confrontare la riga vecchia con la nuova, quindi la difesa è un
**privilegio di colonna**, come al punto 4.

#### 7. Chi esce dall'unica organizzazione in cui abita

Resta con la propria, personale e vuota: è il caso di
[ADR-045](08-decisioni.md#adr-045) al contrario. Il client deve **dirlo prima**, non
lasciarlo scoprire dopo con una dashboard vuota e nessuna spiegazione — è la stessa forma
dell'avviso sulla revoca delle sessioni della 5-quater.

### Decomposizione

##### `A1` — I privilegi di colonna su `app.users` (`R38`)

Migration: `REVOKE UPDATE ON app.users` dal ruolo applicativo, `GRANT UPDATE (full_name,
phone, locale, password_hash)`. Le colonne entrano nelle due invarianti di
`ops/sql/checks/schema_invariants.sql` **nello stesso passo**.

`UserRepository` guadagna un `update_profile` che nomina le sole colonne concesse, sulla
forma di `ChargeRepository::update_status`. `update` resta e resta privilegiato: un test
che lo chiamasse dal pool applicativo fallirebbe con `permission denied`, ed è la difesa
che si vuole.

Un test dei privilegi che prova le negate una per una, come quello dei ticket: la sola
`platform_role` non basta, perché è l'elenco a essere il contratto.

##### `A2` — `/api/v1/me`, lettura e correzione (`R7`)

`GET` e `PATCH /api/v1/me`: nome, telefono, lingua. `PATCH` parziale con la semantica di
sempre — campo omesso significa invariato, `null` esplicito cancella. Nessun `user_id` nel
percorso.

Un test che prova che l'email **non** è modificabile e che il tentativo lo dice.

##### `A3` — La password si cambia da dentro (`R7`)

`POST /api/v1/me/password` con la vecchia e la nuova. Sul pool **applicativo**: chi cambia
la password è autenticato, quindi la regola 27 non ammette il privilegiato.

Due test: la vecchia password sbagliata dà `401` e non cambia niente; dopo il cambio
**nessuna** sessione rinnova più, compresa quella da cui si è cambiata. Il secondo è quello
che si scrive sbagliato, ed è lo stesso inciampo della 5-quater: asserire che la vecchia
sessione smetta di *leggere* fallisce, perché l'access token è un JWT e vive i suoi quindici
minuti. Ciò che muore è il **rinnovo**.

Il client deve dire la conseguenza **prima** del bottone, con la stessa frase che usa il
recupero: chi cambia la password si ritrova disconnesso anche sul telefono, e scoprirlo dopo
sembra un guasto.

##### `A4` — Uscire da un'organizzazione (`R16`)

Migration: `app.has_lease_in_force`, la policy che ammette `user_id =
app.current_user_id()` con la `WITH CHECK` che vincola `org_id`, `user_id` e `role`, e i
privilegi di colonna corrispondenti.

`POST /api/v1/organizations/{id}/leave`, e `AcceptInvitation` che riattiva una membership
non attiva invece di inserirne una.

Tre test: si esce; **non** si esce con un contratto in essere; e chi è uscito **rientra**
accettando un invito nuovo — che è la porta di servizio di ADR-051, ora percorribile.

##### `A5` — Il client

Una schermata del proprio account con i tre campi e il cambio password, raggiungibile dal
menù dell'organizzazione. L'uscita sta lì, con l'avviso che dice che cosa resta dopo.

### Trappole

**`UserRepository::update` nomina tutte le colonne.** Dopo `A1`, chiamarlo dal pool
applicativo fallisce con `permission denied for table users`. Oggi non lo chiama nessuno da
lì — solo `Login` e il recupero password, entrambi privilegiati — quindi la migration non
rompe niente **e non lo dimostra**: è `A2` che deve usare il metodo nuovo.

**`role` è la colonna che si dimentica in `A4`.** Aprire `org_members_update` a un membro
senza negargli `role` è dargli la promozione a `owner`. Una policy non può confrontare
vecchio e nuovo: la difesa è il privilegio.

**Uscire non è cancellare.** Con `deleted_at` la riga sparisce da `find`, e
`org_members_org_user_uk` è unico e non parziale: chi esce così non rientra più. È il
difetto che la `0029` ha già pagato.

**ADR-051 va esteso, non riletto e basta.** La sua guardia sulla membership *attiva* è
stata scritta prevedendo questo giorno; ciò che non è stato scritto è il ramo di
riattivazione, perché non serviva. Adesso serve, e senza di esso `R16` produce persone che
non possono rientrare.

**L'email non si cambia, e va detto nel client.** Un campo disabilitato senza una
spiegazione si legge come un difetto.

**«Revoca tutte le sessioni» non butta fuori nessuno all'istante.** Si revocano i refresh
token; l'access token già emesso è un JWT e non è revocabile, quindi vive i suoi quindici
minuti. È la nota di ADR-052, ed è la stessa su cui un test della 5-quater ha asserito la
cosa sbagliata: il criterio giusto è che il **rinnovo** non funzioni più.

### Come si saprà che è finita

- si aprono i propri dati, si corregge il telefono, si ricarica e il telefono è quello;
- si cambia la password dando quella vecchia, e **nessuna** sessione rinnova più — compresa
  quella da cui si è cambiata, che è la decisione 3;
- la password vecchia sbagliata non cambia niente e lo dice;
- si esce da un'organizzazione in cui non si ha un contratto in essere; con un contratto in
  essere **non** si esce, e il rifiuto arriva dal database e non da un `if`;
- chi è uscito **rientra** accettando un invito nuovo;
- una `UPDATE` a mano su `users.platform_role` dal ruolo applicativo fallisce con
  `permission denied`, e un test lo prova su tutte le colonne negate;
- `just ci` verde.

### Che cosa **non** entra

- **il cambio dell'email**, per la decisione 2: richiede la verifica dell'indirizzo, che è
  fuori perimetro, e una decisione sugli inviti pendenti. Il client mostra il campo e non lo
  rende modificabile, **con una riga che dice perché**: un campo disabilitato senza
  spiegazione si legge come un difetto. Entra in [12](12-da-fare.md) insieme alla verifica
  dell'email, che è la sua precondizione;
- **togliere qualcuno dalla propria organizzazione**, che è il gemello di `R16` e non `R16`.
  La policy lo ammetterebbe già — `org_members_update` è `is_org_staff` — e manca solo la
  rotta, ma è una decisione a sé: che ne è dei suoi contratti, dei suoi addebiti e di ciò
  che ha scritto, e lo stesso divieto va garantito su una persona che **non è chi chiama**.
  Entra in [12](12-da-fare.md), riconosciuta e non istruita;
- **la verifica dell'email alla registrazione**, che resta dov'è: cambia il percorso che
  ogni collaudo attraversa;
- **la cancellazione del proprio account.** È `R37` e la stessa domanda di `R15`: che cosa
  significhi il verbo su righe che `audit_log` e gli eventi continuano a citare;
- **`R9`**, il perimetro dei ruoli: serve prima degli sconosciuti, non prima di un
  proprietario che conosci;
- **l'elenco delle persone di un'organizzazione**, che resta chiuso per
  [ADR-036](08-decisioni.md#adr-036) e [ADR-043](08-decisioni.md#adr-043).

---

<a id="esito-sezione-6"></a>

## Esito — Sezione 6: le persone

Eseguita l'**11 settembre 2026**. Chiude `R7`, `R16` e `R38`.

**Due migration, nessun ADR nuovo, tre eventi di dominio.** I test Rust passano da
**449 a 459**, quelli del client da 104 a **107**. `just ci` verde, zero `SALTATO`:
il database c'era, quindi la suite è girata per intero.

Il **nessun ADR** era una conclusione del piano e non una svista: la decisione sulle
sessioni è stata resa uniforme a [ADR-052](08-decisioni.md#adr-052) apposta per non
doverne scrivere un secondo, e nulla di ciò che è emerso eseguendo ha cambiato quella
scelta. Le decisioni nuove sono **applicazioni** di criteri già registrati —
ADR-021/ADR-026 sui privilegi di colonna, ADR-036 sul non proiettare un utente per
identificatore, ADR-051 sul rientro — e stanno nei commenti delle migration e in
[03](03-modello-dati.md), [04](04-sicurezza.md) e [05](05-api.md).

### Che cosa è entrato

| Passo | Esito |
|---|---|
| `A1` | Migration `0038`, `UserRepository::update_profile`, le colonne nelle **due** invarianti, e un test dei privilegi che prova otto negate e quattro concesse |
| `A2` | `GET` e `PATCH /api/v1/me`. Nessun identificatore nel percorso, e `email` nel corpo **rifiutata** invece che ignorata |
| `A3` | `POST /api/v1/me/password`, sul pool applicativo. Il test asserisce che muore il **rinnovo**, non la lettura |
| `A4` | Migration `0039`: `app.has_lease_in_force`, `org_members_update_self`, i privilegi di colonna — e `AcceptInvitation` che **riattiva** |
| `A5` | `/account`: i tre campi, l'email spiegata, il cambio password con l'avviso, l'uscita con ciò che resta dopo |

### Il piano aveva ragione su tutte e cinque le trappole

È la prima volta che succede, e vale la pena dirlo perché è la prova che serviva:
questa sezione è stata pianificata **leggendo il codice** e non usando il prodotto, e
il timore scritto in testa al piano — *«è un ragionamento, non una misura»* — non si è
avverato sulle trappole. `UserRepository::update` ha fallito con `permission denied`
esattamente dove previsto, `role` era la colonna da negare, l'uscita come soft delete
avrebbe chiuso il rientro, `AcceptInvitation` andava esteso e non riletto, e il test
della revoca si scrive sbagliato.

Quello che il piano **non** aveva previsto è tutto altrove, ed è l'elenco qui sotto.

### Quattro cose che il piano non aveva previsto

**Il divieto sarebbe valso solo per chi non amministra.** È la più importante, ed è
un difetto che il piano conteneva: le policy permissive si combinano in **OR**, quindi
la `WITH CHECK` di `org_members_update` — `is_org_staff(org_id)` — avrebbe lasciato
uscire un proprietario con un contratto in essere, cioè proprio il caso che la
decisione 6 esiste per chiudere, dalla porta accanto. Il criterio di fine del piano
(*«il rifiuto arriva dal database e non da un `if`»*) sarebbe stato vero per un
inquilino e falso per chi amministra, e nessuno se ne sarebbe accorto perché il test
sarebbe stato scritto con un inquilino.

Il rimedio è una `ALTER POLICY` nella stessa migration, con lo stesso ramo, e non
allarga niente: `org_members_update` non aveva **nessun** chiamante — a
`OrgMemberRepository` mancava un `update` — quindi non rompe nulla e chiude in
anticipo la porta che `R39` percorrerà.

**L'evento va scritto prima dell'`UPDATE` della membership**, ed è un ordine che
sembra indifferente e non lo è. `domain_events_insert` ammette
`org_id IS NULL OR org_id = current_org_id() OR is_org_member(org_id)`; la rotta non
pretende `X-Org-Id` — si esce anche da un'organizzazione che non è quella attiva —
quindi a reggere è l'ultimo ramo, e `is_org_member` passa da `member_role`, che esige
`status = 'active'`. Appena la riga diventa `left`, quel ramo diventa falso e
l'inserimento dell'evento sbatte contro la policy: un `auth.denied_by_policy` — cioè
«difetto nostro» — su un'operazione riuscita. È la stessa forma dell'ordine di
`AcceptInvitation`, dove `has_pending_invitation` regge finché l'invito non è marcato.

**L'ultimo proprietario non se ne va, e nessuna policy lo impedisce.** Il piano non lo
nomina: la decisione 7 dice che chi esce dall'unica organizzazione in cui abita resta
con la propria, e non contempla che si possa uscire dalla **propria**. Si può, ed è
`POST /api/v1/organizations/{la mia}/leave`. Lo schema dichiara «almeno un owner per
org» come invariante del **service layer** — un trigger cross-row sarebbe workflow nel
database — quindi è l'unico dei tre rifiuti che vive per intero nel caso d'uso. Senza,
immobili, contratti e addebiti resterebbero in una riga che `organizations_select`
(`is_org_member(id)`) non mostra più a nessuno, e non esiste nessuna operazione del
prodotto per rientrarci.

**`revoke_all_for_user` scriveva «refresh token reuse detected» anche per il recupero
della password.** Un letterale, in una colonna — `user_sessions.revoked_reason` — che
si legge mesi dopo per capire perché una sessione è caduta. Tre percorsi, tre ragioni,
una frase, falsa per due di essi. È diventato un enum di dominio, `RevocationReason`,
passato dal chiamante. La revoca di una **singola** sessione resta dov'era: la deduce
`SessionRepository::update` dallo stato della riga, dove la ragione è leggibile dalla
riga stessa.

### Decisioni prese eseguendo

**`email` nel corpo di un `PATCH` si rifiuta, non si ignora.** È l'eccezione alla
tolleranza dell'API — un campo sconosciuto si ignora ovunque, ed è ciò che permette a
un client vecchio di parlare con un'API nuova — e la ragione è che l'email **non è un
campo sconosciuto**: è quello che chiunque proverà a mandare per primo. Ignorarla
darebbe `200` con l'indirizzo di prima, cioè esattamente il sintomo muto che la
decisione 2 esiste per non produrre. Il `422` nomina il campo e dice perché.

**Telefono e lingua si validano nel dominio, e non solo nello schema.**
`app.users.phone` ha un `CHECK` con una forma precisa: senza la stessa regola nel
dominio, un numero malformato sarebbe morto sul vincolo, cioè con un `409
conflict.constraint_violated` dove il chiamante merita un `422` che nomina il campo.
Per `locale` non c'è nemmeno un `CHECK`, e la difesa è più concreta di quanto sembri:
il client formatta date e importi con `Intl` passando quel valore (regola 19), e
`Intl.DateTimeFormat` **solleva** su un'etichetta malformata — un `locale` di
spazzatura accettato qui diventa una schermata che non si apre più, e il rimedio
sarebbe un `UPDATE` a mano.

**Cambiare la password da dentro non sblocca l'account; il recupero sì.** Due metodi
di dominio invece di uno: `set_password` azzera il contatore dei tentativi e il
blocco, `change_password` no. La ragione è doppia e le due si sorreggono. Di
privilegi: la `0038` non concede `failed_login_count` né `locked_until` al pool
applicativo, quindi un metodo che li azzerasse mentirebbe al chiamante — la scrittura
non arriverebbe in database. Di significato: chi recupera ha superato una prova
**diversa** da quella che il blocco difende, chi cambia da dentro sta rifacendo la
stessa prova di sempre.

**La password corrente errata non incrementa il contatore dei tentativi.** Il blocco
dopo dieci tentativi difende il *login*, dove chi prova non è autenticato; qui chi
prova ha già una sessione, e incrementarlo darebbe a chiunque rubi una scheda il modo
di bloccare fuori il legittimo proprietario — cioè trasformerebbe una difesa in
un'arma. Il privilegio di colonna lo impedirebbe comunque, il che è la conferma che
l'elenco della `0038` è quello giusto.

**Il `204` del cambio password non ripulisce il cookie di refresh.** È una scelta e
non una dimenticanza: il cookie è limitato a `Path=/api/v1/auth`, quindi quella rotta
non lo raggiunge nemmeno; e il refresh che porta è ormai revocato, quindi il prossimo
rinnovo fallisce e il client torna all'accesso da sé.

**`app.has_lease_in_force` non filtra il ruolo della parte contrattuale**, al
contrario di `app.property_residents`, che esclude i garanti. Le due rispondono a due
domande diverse: là «chi abita qui», qui «questa persona ha ancora un legame con un
contratto in corso» — e un garante ce l'ha. `left_at IS NULL` invece vale in entrambe:
chi ha lasciato il contratto non ne è più parte.

**Tre eventi nuovi, nessuno dei quali porta un valore.** `identity.user.profile_updated`
porta i **nomi dei campi** e non i valori, perché un nome e un telefono sono dati
personali e un evento pubblicato viaggia e si conserva — è la forma di
`PropertyUpdated` e `TicketReportCorrected`. `identity.user.password_changed` porta
solo il fatto e la data; nemmeno il numero di sessioni revocate, che si leggerebbe
come una misura e non serve a nessun consumatore. `identity.organization.member_left`
porta il ruolo che si aveva, che non è un dato personale ed è ciò che dice a chi
consuma se se n'è andato un inquilino o chi amministrava.

### Una svista del piano, sul valore di un enum

Il piano scrive la catena di `platform_role` come
`SET platform_role = 'platform_admin'`. Quel valore **non esiste**: `app.platform_role`
è `('user', 'support', 'admin')`, e il letterale giusto è `'admin'`. La stessa svista
sta in `R38`. Non cambia niente della decisione né della catena — `app.is_platform_admin()`
legge un'impostazione di sessione che il contesto installa dai claim, e i claim vengono
da quella colonna — ma un test scritto copiando la riga del piano sarebbe fallito con un
errore di enum invece che con `permission denied`, cioè per la ragione sbagliata.

### Che cosa resta aperto

- **il cambio dell'email**, con la verifica dell'indirizzo che ne è la precondizione.
  Resta in [12](12-da-fare.md), e il client mostra il campo con una riga che dice
  perché non si tocca;
- **`R39`, togliere qualcuno dalla propria organizzazione.** Metà del lavoro adesso
  esiste: `app.has_lease_in_force` c'è, `org_members_update` porta già il divieto anche
  sul ramo dello staff, e `OrgMemberRepository::update` scrive le due colonne concesse.
  Quello che manca è la **decisione**: che ne è dei suoi contratti, dei suoi addebiti e
  di ciò che ha scritto;
- **non si può chiedere di rientrare.** Chi esce rientra accettando un invito nuovo, e
  l'invito lo emette chi amministra: non c'è nessuna via per cui una persona torni di
  propria iniziativa, né per cui segnali di volerlo. È voluto ed è la stessa via sola di
  ADR-051. L'avviso prima dell'uscita lo dice — *«Se qualcuno ti reinvita potrai
  rientrare»* — e ciò che resta scoperto è **dopo**: chi ci ripensa non ha niente da
  guardare, e `membership.already_left` su un secondo tentativo non spiega la via. È
  `R41`, ed è una riga di testo prima che una rotta;
- **chi esce non perde niente di ciò che ha lasciato.** Addebiti, pagamenti e
  segnalazioni restano, e la Row Level Security smette di mostrarglieli perché
  `member_role` esige `status = 'active'`. Non è stato verificato a mano che nessuna
  schermata mostri un elenco vuoto invece di dire che si è usciti;
- **`R9`**, il perimetro dei ruoli, e **`R37`**, la cancellazione: dove erano.

### Che cosa non è stato provato a mano

Tutto, di nuovo, e qui pesa in modo particolare: `A5` è una schermata **nuova e
intera**, non un ritocco a una che esiste, ed è la prima volta dalla fetta C che se ne
consegna una senza guardarla. I test del client coprono le tre cose che il piano
chiedeva di vedere — l'email disabilitata con la sua riga, l'avviso della revoca prima
del bottone, l'uscita offerta solo dove ha senso — e non coprono l'impressione.

Il punto da percorrere in un giro vero è il **rientro**: uscire, farsi reinvitare,
accettare. È il cammino più lungo di questa sezione, attraversa due account e la posta,
e i suoi test lo provano a pezzi.

---

<a id="poscritto-sezione-6"></a>

### Poscritto — il giro a mano, l'11 settembre, e ciò che ha trovato in dieci minuti

**Questo esito si emenda in coda e non si riscrive**, come ADR-048 fa con ADR-026:
la previsione sbagliata resta scritta sopra, perché rileggerla è la parte utile.

La frase da emendare è nella tabella qui sopra, e diceva che `A4` era entrato.
**Era vero per il server e falso per la schermata dopo.** Il primo giro a mano —
quello che questo stesso esito raccomandava — ha percorso l'uscita e ha trovato in
dieci minuti un vicolo cieco che i test non vedevano.

#### La catena, e perché ogni anello sembrava innocuo

1. si esce, e il server fa tutto bene: `status = 'left'`, evento
   `identity.organization.member_left` scritto e dispatchato, nessun errore in
   `last_error`. **Questa metà l'esito la descriveva correttamente**;
2. il client tiene l'organizzazione lasciata come **attiva**: sta in `localStorage`,
   e fin lì è giusto — nessuno gliel'ha ancora tolta;
3. `GET /api/v1/organizations` mandava `X-Org-Id`, con la ragione scritta che
   *«l'eccezione sta nell'endpoint e non nel client»*. Sembrava innocuo, perché la
   risposta non cambia. **Ma è la richiesta che ripara l'organizzazione attiva
   quando è sbagliata**, e `Authenticated` risolve il ruolo per qualunque richiesta
   che porti quell'header: `403 org.not_a_member`, **prima** dell'handler;
4. React Query tiene i dati dell'ultimo successo quando un refetch fallisce, quindi
   l'elenco continuava a contenere l'organizzazione lasciata, il riallineamento del
   guscio la *trovava*, e non correggeva niente.

**Sopravviveva al ricaricamento**, che è ciò che lo rendeva definitivo: al reload il
ricordo viene riselezionato **prima** che l'elenco sia chiesto, quindi anche la prima
richiesta dopo il reload portava l'identificatore morto. Né uscire e rientrare
bastava: la preferenza è per utente e resta.

E prima del `403` c'era una finestra peggiore: per quindici minuti il token porta
ancora quell'organizzazione, header e token coincidono, e il ruolo **non viene
riletto** — le richieste passano, ma `app.member_role` pretende `status = 'active'`,
quindi le policy non restituiscono niente. **Schermate vuote senza errore**, che è il
modo peggiore di sbagliare e quello che il progetto cita più spesso.

#### La parte che brucia

Il commento nel guscio **descriveva questo scenario** come ciò che quel codice esiste
per impedire — *«un'appartenenza chiusa, e da quel momento ogni richiesta porta un
`X-Org-Id` che l'API rifiuta, cioè un'applicazione intera che risponde 403»*. La
guardia era scritta, ragionata, e non reggeva: **il correttore è a sua volta una
richiesta, e portava addosso il motivo del rifiuto**.

Non era mai scattata perché fino a questa sezione **nessuna appartenenza poteva
diventare inattiva dal prodotto**. `R16` non ha introdotto il difetto: lo ha reso
raggiungibile. È la stessa forma di `R31`, che viveva nella *seconda* volta che
qualcosa succede alla stessa persona — qui vive nella prima volta che una membership
smette di essere attiva.

#### La correzione

- `getOrganizations` usa `withoutOrganization: true`. È **la** radice: l'unica
  chiamata che deve poter rispondere quando il ricordo è sbagliato non deve
  dichiarare un'organizzazione;
- l'uscita **toglie l'organizzazione dall'elenco in cache** prima di rileggerlo, così
  il riallineamento è sincrono con la risposta che lo autorizza invece di aspettare
  un giro di rete;
- un test del client che riproduce lo stallo, con il finto server che **rifiuta come
  rifiuta il vero**: senza quel ramo passerebbe anche con il difetto. Verificato al
  contrario — rimesso l'header, il test fallisce.

Non si è aggiunta la gestione dell'elenco *in errore* nel riallineamento, ed è una
scelta: con la radice chiusa quell'elenco non può più fallire per questa ragione, e un
secondo meccanismo per lo stesso caso è un secondo posto in cui sbagliare.

#### Un test asseriva il difetto come se fosse il comportamento

Correggendo, `shell.test.tsx` ha smesso di passare — e la riga che si è rotta è
quella che conta: *«corregge l'organizzazione ricordata quando non è più fra le
proprie»* pretendeva che la **prima** richiesta portasse l'identificatore orfano.

Cioè il test che esisteva per provare la correzione **asseriva la metà rotta del
meccanismo**: che l'elenco delle organizzazioni dichiarasse l'organizzazione
sbagliata era scritto nero su bianco come atteso, e il commento accanto spiegava che
senza la correzione *«ogni richiesta risponderebbe 403»* — senza accorgersi che a
rispondere `403` sarebbe stata anche quella. Adesso asserisce la proprietà giusta e
più forte: **l'identificatore orfano non esce mai**.

È la stessa specie del test della 5-quater sulla revoca delle sessioni, e la seconda
volta in due sezioni: un test scritto per confermare una decisione può fissare
l'errore accanto ad essa, e passa per mesi.

#### Che cosa portarsi dietro

**Il rendimento del giro a mano è di nuovo quello promesso**, e stavolta contro una
sezione i cui test erano tutti verdi: dieci minuti contro una suite da 459.

E la lezione sta nella forma del difetto, non nel difetto: **una richiesta che ripara
uno stato non deve dipendere da quello stato**. Vale per l'elenco delle
organizzazioni e varrebbe per qualunque altra cosa che si scriva per rimettere in
sesto un client sbagliato.

#### Il giro, completato

Percorso per intero il **12 settembre 2026**, da chi non aveva scritto il codice:
i propri dati, il cambio password con la seconda scheda aperta, l'uscita con e senza
un contratto in essere, e il **rientro** — uscire, farsi reinvitare, accettare. Oltre
al vicolo cieco qui sopra **non ha trovato altro nella sezione**, ed è un esito da
scrivere quanto l'altro: una fetta che chiude con «tutto bene» dopo un giro fatto
davvero è una fetta collaudata, non una fetta che il giro non l'ha fatto.

Ha trovato invece due cose **fuori** dalla sezione, usando il prodotto per
arrivarci: `R43`, che l'inquilino non ha modo di contattare chi gli affitta casa, e
`R44`, che un immobile nasce in bozza, ci resta, e in bozza si affitta lo stesso.
Entrambe in [12](12-da-fare.md).


---


<a id="piano-sezione-7"></a>

## Piano — Sezione 7: cancellare, in generale

Pianificata il **12 settembre 2026**, subito dopo l'esito della 6 e il giro a mano che
l'ha collaudata — è la prima sezione pianificata sotto la regola che una sezione
conclusa pianifica **subito** la successiva. Chiude `R15`. Come la 6, nasce da una
lettura del codice e non da un collaudo; a differenza della 6, il collaudo che
l'avrebbe potuta trovare **c'è stato** e non l'ha nominata: nessuno ha sentito la
mancanza di una cancellazione in blocco con dieci righe in tabella. È un'indicazione
sul peso della voce, e va detta prima delle decisioni.

### Che cosa esiste davvero, verificato e non dedotto

| Premessa | Verifica |
|---|---|
| «Elimina» non è un'operazione sola | vero, e le operazioni sono **quattordici** rotte su undici entità: si **archivia** (immobile, stanza, documento, gruppo di spesa), si **annulla** (contratto in bozza, addebito, bolletta, segnalazione, regolamento), si **disdice** (contratto attivo), si **revoca** (invito), si **rifiuta** (pagamento, regolamento), si **toglie** (parte contrattuale, membro di un gruppo), si **esce** (organizzazione) |
| Nessuna rotta accetta più identificatori | vero: tutte prendono `{id}` nel percorso, e nessuna risposta ha la forma «per identificatore» |
| Le tabelle passano da `@/components/ui/table` | vero, e la primitiva ha `TableFrame`, `Tr`, `Td`, `Th` e **nessuna selezione**: niente checkbox, niente stato |
| Gli elenchi hanno azioni per riga | **falso per cinque su sette**: solo addebiti e pagamenti agiscono dall'elenco. Immobili, contratti, segnalazioni, bollette e gruppi di spesa agiscono **solo dalla scheda** — «cancellare una sola riga» dall'elenco oggi non esiste per loro |
| Una spesa co-living si annulla | **falso, ed è la trovata di questa verifica**: la `0028` concede `expenses.deleted_at` al ruolo applicativo *«per l'annullamento»*, scritto nel suo commento, e la vista dei saldi filtra `deleted_at IS NULL`. **Nessun caso d'uso e nessuna rotta lo fanno.** La porta è stata preparata e mai aperta: una spesa sbagliata oggi resta nei saldi per sempre |
| Ripetere un'operazione già fatta è un errore | falso: archiviare un archiviato dà `409 property.archived`, annullare un annullato `409 *.invalid_transition`. Sono codici che dicono «era già così», e un'azione in blocco deve leggerli come tali |
| `ConfirmAction` esiste | vero: `trigger`, `title`, `explanation`, `confirmLabel`, `variant` |

### Il difetto che pianificare ha trovato — la spesa che non si annulla

Non ha ancora un identificatore in [12](12-da-fare.md) perché **entra qui**: è
letteralmente «cancellare, in generale» sull'unica entità in cui la cancellazione è
stata preparata dallo schema — la `GRANT` della `0028` nomina `deleted_at` con la
ragione scritta — e mai costruita. Una spesa registrata per sbaglio, o con l'importo
sbagliato, resta nei saldi del gruppo, e l'unico rimedio è una spesa opposta, che è
un trucco contabile e non un'operazione.

È l'unico passo di questa sezione con del **backend**, e la sezione lo dichiara:
il perimetro scritto in [12](12-da-fare.md) era «client trasversale», ed è vero per
tutto il resto.

### Che cosa è stato deciso prima di pianificare

| | Scelta | Alternativa scartata |
|---|---|---|
| Come si chiama il bottone | **il verbo dell'entità**: Archivia, Annulla, Disdici | «Elimina», che direbbe il falso su almeno tre entità |
| Come si esegue un'azione su N righe | **N chiamate dal client, in sequenza**, con un esito per riga | una rotta in blocco per entità |
| Che cosa vede chi ha selezionato dieci righe e sette sono passate | **le sette spariscono, le tre restano selezionate con il motivo accanto**; nessun ritentativo automatico | tutto-o-niente; oppure ritentare da soli |
| «Era già così» | **non è un errore**: si mostra come fatto | rosso su una riga che è già nello stato chiesto |
| Dove vive la selezione | **nella primitiva della tabella**, una volta | in ogni elenco, sette volte |
| Quali elenchi entrano | immobili, stanze, documenti, addebiti, bollette, segnalazioni, spese | contratti e pagamenti, per le ragioni sotto |
| «Seleziona tutto» | **la pagina**, non l'intero elenco | tutte le 340 righe di un elenco a cursore |

### Decisioni, con il perché

#### 1. Il bottone dice il verbo dell'entità, e mai «Elimina»

`R15` chiede di «cancellare», e la parola è quella che un utente usa; ma il prodotto
non cancella niente — il ruolo applicativo non ha il privilegio `DELETE`
([ADR-003](08-decisioni.md#adr-003)), e ogni entità ha **il suo** modo di uscire di
scena, che non è lo stesso. Un immobile archiviato resta leggibile con la sua storia;
un addebito annullato resta **visibile a chi se l'era visto chiedere** (la `0023` nega
`deleted_at` apposta); una bolletta annullata libera il numero di fattura. Tre cose
diverse con tre conseguenze diverse, e un bottone unico che dicesse «Elimina» le
nasconderebbe tutte e tre proprio nel momento in cui una persona sta decidendo.

Ogni tabella è di **una** entità, quindi ogni tabella ha **un** verbo — e la
conferma dice la conseguenza di *quel* verbo, riusando la frase che la scheda
mostra già per l'azione singola. Non si scrive una seconda frase per il blocco: si
cambia il numero.

#### 2. N chiamate dal client, e nessuna rotta in blocco

*Alternativa scartata: `POST /charges/void` con un elenco di identificatori,* e
così per ogni entità. Sono sei rotte nuove, una forma di risposta nuova — un esito
per identificatore — e una domanda di semantica transazionale che non ha una risposta
buona: tutto-o-niente fa fallire nove annullamenti per un addebito già annullato;
riga-per-riga in una transazione sola è N transazioni travestite da una.

Le rotte singole fanno già tutto ciò che serve: autorizzano sotto Row Level Security
riga per riga, rispondono con un codice che dice perché, e sono idempotenti nel senso
che conta — ripeterle su una riga già fatta risponde `409` con un codice
riconoscibile. Il client le chiama **in sequenza**, non in parallelo: una pagina sono
al più venticinque righe, la sequenza rende l'esito leggibile nell'ordine della
tabella, e soprattutto evita che venticinque `401` simultanei facciano venticinque
rinnovi — che il singolo volo di [ADR-044](08-decisioni.md#adr-044) gestirebbe, ma
che non c'è motivo di provocare.

**Il costo è dichiarato**: venticinque round trip invece di uno. Su una rete mobile
sono secondi, non minuti, e la barra di avanzamento li mostra.

#### 3. L'esito parziale è per riga, e non si ritenta da solo

Dopo il giro, tre cose possono essere successe a ogni riga:

| Esito | Come si riconosce | Che cosa si mostra |
|---|---|---|
| **fatta** | `2xx` | la riga sparisce con la rilettura |
| **era già così** | `409 property.archived`, `409 *.invalid_transition`, `409 document.archived` | la riga sparisce come le altre — non è un errore, è la conferma che lo stato chiesto c'è |
| **non fatta** | `403`, `404`, `409` di altro tipo, rete | la riga **resta selezionata**, con il motivo accanto, tradotto dal `code` |

La terza è quella che va progettata. La riga resta selezionata perché chi ha appena
premuto «Annulla 10» e ne vede 3 restare vuole **riprovare quelle 3**, non
riselezionarle a mano; e il motivo sta *accanto alla riga* e non in un avviso in cima,
perché con tre motivi diversi su tre righe un avviso cumulativo non dice quale.

**Nessun ritentativo automatico.** Un `403` non cambia riprovando, un `404` nemmeno,
e su un errore di rete la persona ha già il bottone davanti. Ritentare da soli
nasconderebbe il motivo e, sulle rotte che non sono idempotenti nel senso pieno,
raddoppierebbe un'azione.

**Il giro non si interrompe al primo errore.** Fermarsi lascerebbe le righe dopo in
uno stato ignoto — «le ho annullate o no?» — che è peggio di un errore per riga.

#### 4. La selezione vive nella primitiva della tabella

`@/components/ui/table` guadagna una colonna di selezione — checkbox per riga,
checkbox di intestazione per la pagina — e una **barra di selezione** che compare
quando c'è almeno una riga selezionata, con il conteggio e il verbo. Sette elenchi
che la scrivessero da sé sarebbero sette occasioni per una checkbox che non si
raggiunge da tastiera, o per un conteggio che non torna dopo un refetch.

**La selezione è un insieme di identificatori, non di righe**: dopo una rilettura, gli
identificatori che non sono più nella pagina **escono dalla selezione**. Senza questa
regola, «3 selezionati» resta scritto sopra una tabella in cui ne restano 2, e il
bottone agisce su una riga che non c'è.

**«Seleziona tutto» è la pagina.** Gli elenchi sono a cursore (keyset), quindi «tutte
le 340 righe» non è un'operazione che il client possa fare senza scaricarle tutte, e
scaricarle tutte per annullarle è il caso in cui una persona dovrebbe fermarsi a
pensare. Il conteggio dice «25 su questa pagina», mai «tutti».

#### 5. Quali elenchi entrano, e perché due no

Entrano quelli in cui il verbo è **uno** e non richiede niente per riga: immobili,
stanze e documenti (*archivia*), addebiti, bollette e segnalazioni (*annulla*), e
le spese co-living (*annulla*, che è il passo `E5`).

**I contratti no.** Una selezione conterrebbe bozze — che si *annullano* — e attivi —
che si *disdicono*, con una data e un motivo **per contratto**. Due verbi in una
tabella, e uno dei due vuole un dialogo per riga: un blocco che disdicesse cinque
contratti con la stessa data e lo stesso motivo sarebbe quasi sempre sbagliato.
Restano azioni dalla scheda, come sono.

**I pagamenti no.** Rifiutare una dichiarazione è un giudizio su quella dichiarazione
— «non è arrivato niente» — e rifiutarne dieci insieme è dieci giudizi con un clic.
L'elenco ha già le azioni per riga dalla fetta A, e basta.

#### 6. La spesa co-living si annulla, e il backend è questo solo

`POST /api/v1/expenses/{id}/cancel`, che scrive `deleted_at` — la `0028` l'ha
concessa per questo e il commento lo dice. La vista dei saldi filtra già
`e.deleted_at IS NULL` (verificato nella `0027`), quindi una spesa annullata **esce
dai saldi da sola**: nessuna vista da toccare, nessuna migration.

Chi può: chi l'ha registrata, o lo staff — è la `USING` di `expenses_update`, e va
verificata eseguendo. Fino a quando: finché nessun regolamento la cita? **No**: un
regolamento salda un saldo, non una spesa, e togliere una spesa dopo un regolamento
sposta il saldo — che è esattamente ciò che una persona vuole quando la spesa era
sbagliata. Il saldo si riequilibra, ed è il trigger `expense_shares_balanced` a
garantire che le quote restino coerenti. Va scritto nella scheda: «annullare una spesa
già regolata cambia i saldi».

#### 7. `R37` resta fuori, e la 7 non lo tocca

Cancellare i dati di una persona è un'altra domanda — che cosa significhi il verbo su
righe che `audit_log` e gli eventi continuano a citare — e ha un runbook nel
frattempo. Questa sezione riguarda **le righe di lavoro**, non le persone.

### Decomposizione

##### `E1` — La selezione, nella primitiva

`@/components/ui/table`: colonna di selezione, checkbox di intestazione per la
pagina, barra con conteggio e azione. La selezione è un `Set` di identificatori che
si **pota** a ogni cambio di dati. Test: la potatura dopo un refetch; tastiera; il
conteggio dice «su questa pagina».

##### `E2` — Il giro, nel livello dei dati

Una funzione in `@/api` (o `@/lib`) che riceve N identificatori e una mutazione
singola, li esegue **in sequenza**, classifica ogni esito in *fatta / era già così /
non fatta* leggendo il `code`, e restituisce l'elenco. Invalida **una volta** alla
fine, non N. Test: l'esito parziale; la classificazione del `409` «già fatto»; il
giro che non si ferma al primo errore; nessun ritentativo.

##### `E3` — Archiviare: immobili, stanze, documenti

I tre elenchi con `E1` + `E2` e il verbo *Archivia*, e l'azione **per riga** dove
manca (immobili). La conferma riusa la frase della scheda con il numero.

##### `E4` — Annullare: addebiti, bollette, segnalazioni

Come `E3` con il verbo *Annulla*. Gli addebiti hanno già l'azione per riga: si
allinea alla primitiva invece di tenerne due.

##### `E5` — La spesa si annulla

Dominio (`Expense::cancel`), caso d'uso, rotta, test che prova che **i saldi
scendono**. Poi il client, per riga e in blocco. È il solo passo con backend, e va
per ultimo fra quelli di codice perché è il solo che può trovare qualcosa nello schema.

##### `E6` — Le stringhe, i documenti, l'esito

Le frasi di conferma in `i18n`, `05-api.md` per `E5`, l'esito in `11` — e **il piano
della sezione 8 scritto subito dopo**, che è la regola nuova.

### Trappole

**«Era già così» sembra un errore e non lo è.** Il `409` che torna su una riga già
archiviata o già annullata è la conferma dello stato chiesto. Trattarlo come
fallimento lascia righe rosse selezionate che non si possono «sistemare», e la
persona riprova all'infinito.

**La selezione sopravvive alla rilettura se non la si pota.** Dopo il giro le righe
fatte spariscono dalla pagina; se la selezione è di righe e non di identificatori, o
se non si pota, il conteggio mente e il bottone agisce su fantasmi.

**Il link della riga e la checkbox si contendono il clic.** Le righe degli elenchi
sono collegamenti alla scheda: la checkbox deve fermare la propagazione, e la riga
intera non deve diventare «seleziona» — chi vuole aprire la scheda deve poterlo fare
ancora.

**Venticinque `401` insieme.** Se il giro fosse parallelo e l'access token scadesse a
metà, venticinque richieste chiederebbero il rinnovo insieme: il singolo volo di
ADR-044 regge, ma in sequenza il caso non si presenta. Il test del giro va scritto
con una scadenza a metà, così se qualcuno lo rende parallelo il test lo dice.

**I contratti tentano.** «Tanto è la stessa tabella» — non lo è: due verbi, e uno
vuole un dialogo per riga. La decisione 5 è scritta perché la tentazione arriverà.

**Annullare una spesa già regolata cambia i saldi**, ed è giusto, ma va detto nella
conferma: chi annulla non pensa ai regolamenti fatti dopo.

**`R37` si infila.** «Visto che stiamo cancellando…» — no. È un'altra domanda, con
una migration e un ADR probabili, e questa sezione è il posto sbagliato per farla di
passaggio.

### Come si saprà che è finita

- si selezionano tre immobili, si preme *Archivia*, la conferma dice «3 immobili» e
  la conseguenza, e spariscono;
- si selezionano tre addebiti di cui uno già annullato: due si annullano, il terzo
  sparisce anch'esso, **niente in rosso**;
- si spegne l'API a metà del giro: le righe non processate restano selezionate con
  «rete non raggiungibile» accanto, quelle fatte sono fatte, si riprova con un clic;
- da ogni elenco che entra si agisce **anche su una riga sola**, con lo stesso verbo
  e la stessa conferma;
- una spesa co-living si annulla, e il saldo del gruppo cambia di conseguenza;
- nessun bottone del prodotto dice «Elimina»;
- `just ci` verde, e **il piano della sezione 8 scritto nello stesso commit
  dell'esito**.

### Che cosa **non** entra

- **i contratti e i pagamenti in blocco**, per le decisioni 5: due verbi con un
  dialogo per riga, e dieci giudizi con un clic;
- **«seleziona tutte le righe» oltre la pagina**, per la decisione 4;
- **l'annullamento di un'operazione** (undo): ogni verbo ha già la sua via di ritorno
  o non ce l'ha per scelta — un immobile archiviato non si riapre (è scritto nel
  dominio), un addebito annullato si riemette. Un «annulla l'ultima azione» generico
  sarebbe una seconda semantica sopra le prime;
- **`R37`**, la cancellazione delle persone, per la decisione 7;
- **una rotta in blocco**, per la decisione 2. Se un giorno un elenco avrà migliaia
  di righe da agire insieme, sarà una voce nuova con una misura dietro.

---

<a id="esito-sezione-7"></a>

## Esito — Sezione 7: cancellare, in generale

Eseguita il **12 settembre 2026**. Chiude `R15`.

**Una rotta, nessuna migration, nessun ADR, un evento di dominio.** I test Rust
passano da **459 a 461**, quelli del client da 107 a **124**. `just ci` verde, zero
`SALTATO`.

Il **nessun ADR** era una conclusione del piano e non una svista: l'unica decisione
di backend l'aveva già presa la migration `0028`, che concede `expenses.deleted_at`
*«per l'annullamento»* e lo scrive nel suo commento; il resto è client, e le sue
decisioni — il verbo dell'entità, N chiamate in sequenza, la selezione nella
primitiva — sono nel piano e restano lì.

### Che cosa è entrato

| Passo | Esito |
|---|---|
| `E1` | `useSelection`, `ThSelect`, `TdSelect`, `RowCheckbox`, `SelectionBar` in `@/components/ui/table`: la selezione è un `Set` di identificatori **derivato** per intersezione a ogni render, non potato da un effetto. Cinque test, la potatura verificata al contrario |
| `E2` | `@/api/batch.ts`: `runInSequence` e `classifyFailure`. Nove test, fra cui la scadenza del token a metà con il singolo volo **spento** — e la versione parallela dello stesso giro, eseguita nello stesso file, che fa cadere la catena. Sopra, `@/features/shared/bulk.tsx`: `useBulkAction`, `VerbButton`, `BulkBar` — il pezzo che i sette elenchi montano identico |
| `E3` | Immobili con l'azione per riga che mancava; stanze con l'archiviazione **spostata dalla modale alla riga**; allegati, che sono un `<ul>` e non una tabella |
| `E4` | Addebiti allineati alla primitiva — lo storno resta per riga e fuori dal blocco; bollette e segnalazioni con l'azione per riga che mancava |
| `E5` | `Expense::cancel`, `ExpenseRepository::cancel`, `CancelExpense`, `POST /expenses/{id}/cancel`, `ExpenseCancelled`, `Denial::ExpenseNotYours`, `created_by` su `ExpenseResponse`. Un test di contratto che prova che **i saldi scendono** anche dopo un regolamento confermato, più i tre rifiuti |
| `E6` | Le stringhe, [05](05-api.md), questo esito, e il [piano della sezione 8](11-piani-conclusi.md#piano-sezione-8) |

### Il piano aveva tre premesse sbagliate, e una era nella tabella delle verifiche

**«Archiviare un archiviato dà `409 property.archived`.»** Falso, ed era scritto
nella tabella *«verificato e non dedotto»*: era dedotto dal codice del dominio —
`Property::archive` rifiuta con `PropertyArchived` — senza guardare da dove arriva
l'aggregato. `find_by_id` legge con `deleted_at IS NULL`, quindi una seconda
archiviazione **non trova la riga** e risponde `404 property.not_found`. Vale per
tutta la famiglia del soft delete: immobili, stanze, documenti, e adesso le spese.
Il `409` esiste solo per chi cambia stato e resta visibile — addebiti, bollette,
segnalazioni.

La conseguenza sulla classificazione è che **il `404` sta con «era già così» e non
con «non fatta»**, ed è un'emendazione della decisione 3 e non un aggiramento: una
riga che era in tabella e adesso risponde `404` è una riga che la prossima
rilettura non mostrerà, perché elenco e dettaglio passano dalle stesse policy e
dallo stesso filtro. Un motivo scritto accanto a una riga che sta per sparire non
dice niente a nessuno — e con la potatura sarebbe sparito con lei nello stesso
istante, quindi le due classificazioni sarebbero state indistinguibili a schermo e
diverse solo nel conteggio del riepilogo. Il test di `classifyFailure` lo fissa con
la ragione accanto.

**«Chi può annullare una spesa: chi l'ha registrata, o lo staff — è la `USING` di
`expenses_update`.»** La `USING` è **più larga**: `deleted_at IS NULL AND org_id =
current_org_id() AND can_access_expense_group(group_id)`, cioè *qualunque membro
attivo del gruppo*. Verificato eseguendo, come il piano chiedeva. Il confine che il
piano voleva sta quindi nel caso d'uso — `Denial::ExpenseNotYours`, `403
expense.not_yours` — ed è un confine del contratto dell'API e non di sicurezza, la
stessa forma di `LeaveGroup` prima della `0029`. Si è aggiunto **chi ha anticipato**
a chi l'ha registrata: è il suo credito che la riga afferma, e chi l'ha registrata
potrebbe essere uscito dal gruppo. Non si è stretta la policy: sarebbe una
migration per un confine che il modulo tratta ovunque come applicativo.

**«Le tabelle passano da `@/components/ui/table`.»** Vero per sei elenchi su
sette: gli allegati sono un `<ul>`, e lo sono per una ragione scritta —
`AttachmentsCard` è un componente solo per sei owner, alcuni dentro una modale
larga la metà. La primitiva è quindi **la selezione**, e la colonna della tabella
è un involucro sopra `RowCheckbox` e `SelectAllCheckbox`; l'elenco degli allegati
monta gli stessi due pezzi in un `<li>`. Il piano diceva «nella primitiva della
tabella, una volta», e la sostanza regge: un solo insieme, una sola potatura, una
sola barra.

### Che cosa il piano non aveva previsto

**Due bottoni «Annulla» nella stessa modale, con effetti opposti.** `ConfirmAction`
chiudeva con `t.common.cancel` — *Annulla* — e il verbo di destra su addebiti,
bollette e segnalazioni era *Annulla*. Esisteva da prima, su ogni conferma di
annullamento del prodotto; montare il verbo in blocco l'ha reso impossibile da non
vedere. Il bottone che non fa niente dice ora **«Lascia com'è»**, ed è una stringa
sola in `common` per tutte le conferme.

**Il bottone «Annulla» della scheda di una bolletta compariva anche su `paid`**, e
rispondeva `409`: la condizione era «non `allocated` e non `void`», mentre
`BillStatus::is_editable` è `draft | pending_review | validated`. Allineato, e la
stessa regola decide quali righe si offrono alla selezione.

**Il nome accessibile incolla i nodi di testo senza spazio.** «Annulla» seguito da
uno `<span class="sr-only"> — Canone</span>` si legge *«Annulla— Canone»*, ed è la
forma che `RoomDialog` usa per «Modifica». Il bottone del verbo porta il nome della
riga in `aria-label`; la modale della stanza non è stata toccata, e resta un difetto
di lettura minore da sistemare quando la si riapre.

**Un immobile con un contratto in essere si archivia senza che nulla protesti.**
Trovato con il giro a mano di questa sezione, archiviando due immobili in blocco: uno
aveva un contratto attivo, ed è uscito dagli elenchi come l'altro, lasciando il
contratto a citare un immobile che le risposte chiamano «Immobile archiviato». Non è
di questa sezione — `ArchiveProperty` è della fetta 1 e non guarda i contratti — ed
è `R45` in [12](12-da-fare.md).

### Decisioni prese eseguendo

**«La pagina» sono le righe caricate.** Gli elenchi non hanno pagine: hanno «Carica
altri», e ciò che si vede è l'accumulo. Il conteggio dice *«N selezionate su M
caricate»*, e non «su questa pagina» come il piano scriveva: «caricate» è la parola
vera, e dice da sé perché non è «tutte».

**Una riga su cui il verbo non si applica ha la casella spenta, non assente**, negli
elenchi in cui la colonna c'è — addebiti pagati, bollette riaddebitate — perché le
righe restano allineate e una casella spenta dice «non questa» meglio di un vuoto.
Dove **nessuna** riga si può agire da chi guarda — un inquilino sulle segnalazioni in
lavorazione, un membro sulle spese altrui — la colonna non si monta affatto: venti
caselle spente non dicono niente.

**Chi ha segnalato annulla in blocco le proprie segnalazioni aperte**, con lo stesso
criterio della scheda: staff finché è in lavorazione, chi ha segnalato solo su
`open`. La funzione `cancellable` in `TicketsPage` **ripete** quella di
`TicketActions`, e deve restarle uguale.

**`created_by` esce su `ExpenseResponse`.** Senza, il client non può sapere chi ha
registrato una spesa e offrirebbe l'annullamento a chi riceverebbe `403`. È un
identificatore, come `reported_by` e `uploaded_by`, e non un dato personale.

**Una spesa su un conto chiuso non si annulla**: `open_group`, come ogni scrittura
del modulo, con `409 expense_group.archived`. Chi trova una spesa sbagliata su un
gruppo archiviato non ha una via, ed è coerente con il fatto che l'archiviazione non
si disfa. Il piano non lo diceva; è la scelta più stretta, e allargarla dopo costa
una riga.

**`ExpenseCancelled` porta chi aveva anticipato e chi ha annullato**, e niente
altro: è il credito che sparisce, e chi consuma deve sapere di chi era senza
rileggere una riga che non si legge più.

**L'avanzamento resta finché la rilettura non è finita**, non finché l'ultima
chiamata non risponde: il lavoro non è concluso finché la tabella non lo mostra, e
nel frattempo il bottone non c'è, così un secondo clic non parte su righe vecchie.

**Lo storno resta per riga e fuori dal blocco**, ed era implicito nel piano — il
verbo degli addebiti è *Annulla* — ma va detto: rinunciare a incassare è un giudizio
su una perdita, e dieci giudizi con un clic è la ragione per cui i pagamenti sono
fuori.

### Verificato al contrario, quattro volte

Dopo la 6 e la 5-quater — due test che asserivano il difetto — questa sezione non
consegna un test senza averlo visto fallire:

- `runInSequence` riscritta con `Promise.all`: **due** test cadono — il massimo di
  richieste in volo, e il token che scade a metà. Il terzo test del file esegue la
  versione parallela apposta, e prova che la catena cade;
- `useSelection` con `selected = chosen` invece dell'intersezione: il test della
  potatura cade, «2 selezionate» sopra una riga sola;
- `classifyFailure` con il `409` «era già così» letto come errore: il test di pagina
  degli addebiti cade, perché `beta` resta selezionata;
- `onSettled` chiamato per riga invece che alla fine: lo stesso test cade sul
  conteggio delle riletture, tre invece di una.

### Che cosa è stato provato a mano

Con l'API di questa sezione su una porta a parte e un account di prova creato
apposta, in locale: la selezione e l'archiviazione in blocco di due immobili dall'elenco,
con la conferma che dice «2 immobili» e la conseguenza; l'annullamento per riga di
una spesa co-living, con i saldi che scendono da 79,40 € a 49,40 € alla rilettura;
la selezione di tutti gli addebiti dallo scadenzario. **Non** provati a mano: la rete
che cade a metà del giro — il test di pagina lo copre con una `fetch` che solleva —
e i 375 px, che le tabelle già reggevano e a cui questa sezione aggiunge una colonna
stretta.

### Che cosa resta aperto

- **`R45`**, l'immobile con un contratto in essere che si archivia;
- **il nome accessibile di «Modifica» sulla stanza**, che legge *«Modifica— Camera A»*:
  la forma giusta è in `VerbButton`, e si applica quando si riapre quella modale;
- **`R37`**, la cancellazione delle persone, dove era: questa sezione ha toccato le
  righe di lavoro e nessun dato personale.

<a id="piano-sezione-8"></a>

## Piano — Sezione 8: il perimetro dei ruoli

Pianificata il **12 settembre 2026**, subito dopo l'esito della 7 e sotto la regola che
una sezione conclusa pianifica la successiva. Chiude `R9`. È la voce più grande della
lista e l'ultima fra quelle decise, e **il suo primo passo non è codice**: è un ADR.
Tocca chi può registrarsi e come, cioè il confine fra chi amministra e chi abita, e ha
una faccia di sicurezza; le sue domande hanno risposte che si escludono a vicenda, e
sceglierne una nel codice prima che sia registrata significherebbe scoprirla nel diff.

Come la 6 e la 7, nasce da una lettura del codice. A differenza loro, **questa sezione
non ha un collaudo che l'abbia trovata**: `R9` è nota dal primo giorno del client
([ADR-042](08-decisioni.md#adr-042) lo dice in chiaro) ed è stata tollerata perché il
prodotto è «per un proprietario che conosci». Smette di esserlo il giorno in cui la
registrazione è aperta a sconosciuti, e quel giorno è il deploy.

### Che cosa esiste davvero, verificato e non dedotto

Con la lezione della 7 applicata: per ogni riga è scritto **dove** è stato guardato.

| Premessa | Verifica |
|---|---|
| Chiunque si registri diventa owner di un'organizzazione propria | vero: `RegisterOwner` (`crates/application/src/use_cases/auth/register_owner.rs`) chiama `Organization::personal(...)` e `OrgMember::active(..., OrgRole::Owner, ...)` **sempre**, senza un ramo. È l'unico caso d'uso di registrazione, e apre `begin_privileged()` perché nessun contesto esiste ancora |
| Un invito conferisce solo `tenant` | vero, in due posti: `Invitation::issue` rifiuta `Owner` e `Manager` con una `Validation` (`crates/domain/src/identity/invitation.rs`), e la policy della `0018` ha `role = 'tenant'` nel ramo che abilita l'accettazione. Un invito che crei un proprietario **non esiste** |
| Gli inviti li emette lo staff dell'organizzazione | vero: `org_invitations_staff` è `FOR ALL USING (is_org_staff(org_id))`. Non c'è un livello sopra l'organizzazione che possa emetterne |
| Chi accetta un invito deve essere già registrato | vero: `AcceptInvitation` è `Authenticated`, non usa il pool privilegiato, e la `/invito` da fuori sessione (`AcceptInvitationSignedOutPage`) manda a `/registrazione` con `state.from`. Quindi **ogni inquilino è passato da `RegisterOwner`** e possiede un'organizzazione personale vuota |
| Dopo il login si atterra nell'organizzazione «giusta» | **falso per l'inquilino**: `pick_default_membership` (`login.rs`) ordina `Owner < Manager < Tenant` e prende la prima, quindi un inquilino appena registrato e invitato entra **come owner della propria organizzazione vuota**, dove il client gli mostra «Registra un immobile» a ragione. Il `localStorage` ricorda la scelta solo dal secondo accesso |
| Il client offre le azioni per ruolo, e non è un confine | vero e scritto: `useIsStaff()` in un posto solo, [ADR-042](08-decisioni.md#adr-042). Nella propria organizzazione il ramo dice `owner`, e la Row Level Security concorda |
| Esiste un ruolo di piattaforma | vero: `app.platform_role` è `('user','support','admin')`, `app.is_platform_admin()` legge un'impostazione di sessione che il contesto installa dai claim, e `RequestContext::is_staff` lo considera. **Nessuna rotta lo usa** oggi; l'unico modo di averlo è un `UPDATE` a mano (vedi la svista corretta nell'[esito della 6](11-piani-conclusi.md#esito-sezione-6): il valore è `'admin'`) |
| Ogni elenco esige `X-Org-Id` | vero: `ensure_member` risponde `403 org.not_selected` senza, e il client lo manda **sempre** (regola 24) tranne su tre rotte (`ApiRequestInit::withoutOrganization`). Un utente **senza nessuna organizzazione** oggi non è uno stato che il guscio sappia mostrare |
| I test costruiscono ogni inquilino con `register_and_login_with_email` e poi l'accettazione | vero (`crates/api/tests/common/mod.rs`, `coliving_tenant` e simili). **Sono decine**: cambiare la registrazione cambia l'harness prima che il prodotto |
| L'email di registrazione, se già presa, risponde `409 auth.email_taken` | vero: `RegisterOwner` lo controlla e il vincolo `users_email` lo conferma. È già un oracolo di esistenza degli indirizzi, e lo resterà |

### Le domande, con ciò che si sa su ciascuna

Sono le quattro di `R9`, più due che la verifica ha aggiunto. **L'ADR le chiude tutte
e sei**, o dice perché una resta aperta.

**1. Un utente nato inquilino ha un'organizzazione propria?** Oggi sì, ed è il
difetto: la possiede come owner e lì può registrare immobili. Tre risposte possibili:

- *nessuna organizzazione*: la membership arriva solo con l'invito. È la più onesta e la
  più cara: un utente senza organizzazione è uno stato che il guscio non conosce —
  ogni elenco risponderebbe `403 org.not_selected` — e va progettato come stato di
  prima classe, con una schermata che dica «aspetta un invito» invece di una
  navigazione vuota;
- *un'organizzazione personale, ma non come owner*: contraddice `org_members_insert`,
  il cui ramo di bootstrap ammette **solo** il primo membro come `owner`, e lascia
  un'organizzazione che non amministra nessuno;
- *un'organizzazione personale come owner, «spenta»*: una colonna in più e un ramo in
  ogni policy dello staff. È la risposta che sembra economica e non lo è.

**2. Come nasce un proprietario?** Oggi da `RegisterOwner`, aperto a chiunque. La via
che `R9` descrive — *si fa verificare fuori dal prodotto e riceve un invito per
registrarsi* — vuole **un invito di un genere che non esiste**: senza organizzazione
(la crea l'accettazione), emesso da qualcuno sopra le organizzazioni. Il posto naturale
è `platform_role = 'admin'`, che c'è e nessuno usa. Va deciso se sia una tabella nuova
(`app.owner_invitations`) o `org_invitations` con `org_id` annullabile — e la seconda
rompe `org_invitations_staff`, che è `FOR ALL` su `is_org_staff(org_id)`.

**3. Come si verifica un proprietario, e dove vive quello stato?** Fuori dal prodotto,
almeno all'inizio: un'email, una decisione di una persona, l'emissione del token. Dentro
il prodotto vive **solo il fatto che il token esiste**. Un flusso interno di
«richiesta di accesso come proprietario» è una seconda sezione, non questa.

**4. Che ne è degli utenti già registrati?** Sono tutti owner di un'organizzazione
personale, e sono di due specie: chi è davvero un proprietario, e chi si è registrato
perché invitato. Non c'è una colonna che li distingua; li distingue il **fatto** che la
loro organizzazione personale abbia o no un immobile. La risposta più stretta è non
toccarli: un'organizzazione personale vuota è innocua finché nessuno ci entra, e
nessuno può entrarci senza un invito dal suo owner.

**5. Con quale organizzazione si entra?** — trovata verificando. `pick_default_membership`
preferisce `Owner`, e con la risposta 1 = «nessuna» il problema sparisce da sé per gli
utenti nuovi; per quelli vecchi resta, e la scelta è fra correggere l'ordine (preferire
un'organizzazione **con immobili**?) e lasciare al `localStorage` il ricordo.

**6. Che cosa vede un invitato prima di accettare?** — trovata verificando. Con la
risposta 1 = «nessuna», la registrazione dell'inquilino **deve partire dall'invito** e
concludersi con la membership, in una transazione: registrarsi e poi accettare in due
passi lascerebbe fra i due un utente senza organizzazione, che è lo stato che la
risposta 1 esiste per non produrre.

### L'ADR è scritto, e il piano dei passi discende da lì

[ADR-053](08-decisioni.md#adr-053), **13 settembre 2026**: l'inquilino non ha
un'organizzazione propria e si registra dall'invito, dentro l'organizzazione di chi lo
ospita, in una transazione; il proprietario nasce da un invito di piattaforma, in una
tabella nuova `app.owner_invitations`, emesso da uno script in `ops/` e non da una rotta;
la verifica è fuori dal prodotto; **il database si svuota**, quindi nessuna migration di
dati; il login non cambia; `/invito` è l'ingresso per entrambi i generi, con l'email
vincolata a quella dell'invito e la stessa risposta per «token inesistente» e «token
usato o scaduto». Le sei domande della sezione precedente sono chiuse lì, con le
alternative scartate; qui sotto c'è solo ciò che discende.

**Che cosa il piano dà per verificato, oltre alla tabella sopra:**

| Premessa | Verifica |
|---|---|
| `register_and_login` è la porta di ogni test | 108 occorrenze in `crates/api/tests/http_contract.rs`, zero in `persistence.rs`. `register_and_login_with_email` è quella che l'inquilino usa per registrarsi **con l'email dell'invito** prima di accettare |
| La tabella dei privilegi di colonna è il controllo 12 di `ops/sql/checks/schema_invariants.sql`, e `org_invitations` c'è già con sei colonne | vero, righe 147–149 |
| I token opachi sono 256 bit in `base64url` senza padding, hash SHA-256 del testo | `crates/infrastructure/src/security/refresh.rs`, `random_token` e `hash_token` |
| `/registrazione` esiste due volte nel router: da fuori sessione è `RegisterPage`, da dentro rimanda a `returnTo` | `apps/web/src/app/App.tsx`, righe 138 e 217 |
| `password_reset_codes` è il precedente di una tabella senza `org_id` letta dal pool privilegiato, con RLS attiva e una policy «la riga è tua» solo per l'invariante 2 | migration `0037` |

### Come si lavora su questa sezione — un metodo nuovo, provato qui e **ritirato**

> **Ritirato il 16 settembre 2026, da `F8` in poi.** Il metodo degli agenti è stato
> provato su `F2`–`F7` e misurato; la misura sta nel diario più sotto e la ragione del
> ritiro in fondo a questa sottosezione. `F8` e le sezioni successive tornano alla forma
> delle sezioni 1–7: **una sola conversazione** pianifica, scrive il codice, verifica al
> contrario e scrive l'esito. Il testo che segue resta com'era, perché è la storia di
> che cosa si è provato e perché, e la si rilegge prima di riprovarci.

Deciso il **13 settembre 2026**, prima di cominciare. Fino alla 7 una sola
conversazione ha pianificato, scritto il codice, verificato e scritto l'esito. Da questa
sezione i tre ruoli si separano, e il modello che li esegue non è lo stesso:

| Ruolo | Chi | Che cosa |
|---|---|---|
| **Decidere e pianificare** | la conversazione principale, con il modello più capace | l'ADR, discusso con chi ha chiesto il lavoro; il piano dettagliato dei passi **dopo** l'ADR, con le premesse verificate contro il codice; le decisioni prese eseguendo |
| **Scrivere il codice** | un agente con un modello meno oneroso, **uno per passo** | il passo così com'è scritto, con il suo criterio di fine, i test previsti e le verifiche al contrario da fare e da riportare |
| **Verificare** | **un agente a sé con il modello più capace, a contesto pulito** — non la conversazione principale | ogni passo **prima** che cominci il successivo: rileggere il diff contro il piano e l'ADR, **rifare** le verifiche al contrario, `just ci`, e riferire alla conversazione principale ciò che non regge |
| **Chiudere** | la conversazione principale | la tabella dei commit, il push con il suo via libera, l'esito in [11](11-piani-conclusi.md) e il piano della sezione 9 |

**Perché separarli.** I passi *decisi e delimitati* — «monta la primitiva su questi tre
elenchi, con questo verbo e queste stringhe» — sono la parte più lunga di una sezione e
la meno bisognosa di giudizio; i piani di questo documento sono già scritti per chi non
era nella conversazione, e un agente è esattamente quel lettore. La parte che ha bisogno
di giudizio è breve e costa cara se sbagliata.

**Che cosa non si delega, e per quale ragione.** Nella 7 le tre premesse sbagliate del
piano sono venute fuori *scrivendo il codice*, e ognuna chiedeva di fermarsi e
riemendare il piano invece di aggirarlo. **Un agente che esegue un piano tende a farlo
funzionare, non a contestarlo**: quindi l'ADR, il dominio con le sue autorizzazioni, e
l'esito restano alla conversazione principale, e ogni passo delegato porta scritto
*«se una premessa del piano si rivela falsa, fermati e riferisci: non aggirarla»*.

**La verifica è una verifica, non una lettura.** La 6 e la 7 hanno mostrato che un test
verde può asserire il difetto. Chi verifica **rimette il difetto e guarda il test
cadere**, per ogni test consegnato dall'agente; leggere il diff non basta, e con un
autore meno forte basta ancora meno.

**Chi verifica non è chi ha pianificato.** Deciso il 13 settembre, dopo che le prime
tre revisioni (`F2`, `F3`, `F4`) le aveva fatte la conversazione principale: chi ha
scritto il piano e l'ADR è condizionato dal proprio contesto — legge il diff cercando
ciò che si aspettava di trovare, e ha già deciso che le premesse sono vere. Un agente
di revisione **a contesto pulito**, con il modello più capace, parte da `CLAUDE.md`,
dall'ADR, dal passo del piano, dal rapporto dell'autore e dal diff, e da niente altro:
non sa che cosa la conversazione ha discusso e non ha interesse a confermarlo. Riceve
gli stessi obblighi dell'autore — leggere prima di giudicare, rifare le verifiche al
contrario, `just ci` — e riferisce alla conversazione principale, che decide se
rimandare all'autore o proporre i commit. Il costo è un secondo contesto a freddo per
passo; è il prezzo di un giudizio che non sia il proprio.

**E la stessa regola vale per i documenti.** Questa sottosezione era stata scritta e
committata, ed è **sparita nel commit successivo**: riscrivendo i passi del piano si è
sostituito un intervallo di testo che la conteneva, e chi ha fatto la sostituzione non
se n'è accorto perché sapeva che cosa aveva scritto e non ha riletto. Ripristinata dal
commit che l'aveva introdotta. Un revisore a contesto pulito guarda anche il diff dei
documenti, e un intervallo cancellato lo vede.

**Il costo fisso di un agente è il contesto.** Parte a freddo e deve leggere
`CLAUDE.md`, i documenti pertinenti e il piano prima della prima riga: su un passo
piccolo quel costo supera il risparmio. Un agente **per passo** — `F2`, `F3`, … — e non
uno per la sezione: la revisione resta incrementale, e un errore non si porta dietro il
passo dopo.

**La conversazione principale non legge i diff.** Deciso il 15 settembre, dopo le
prime cinque revisioni. Il coordinatore è il contesto più caro del metodo — porta
tutta la sezione precedente, i documenti letti e ciò che ha già rivisto, e lo ripaga a
ogni turno — e su `F2`–`F4` ha letto i diff per intero trovando solo cosmetica: spesa
doppia per un giudizio peggiore di quello del revisore pulito. Da qui legge i **rapporti**
(dell'autore e del revisore), scrive i brief, decide, scrive i documenti. Il diff lo
legge chi non ha il piano in testa.

**Ogni turno ripaga l'intero contesto, e questo decide la forma dei brief.** Deciso il
**16 settembre 2026**, dopo aver misurato F6+F7 dalle trascrizioni: il consumo di un
agente non è ciò che legge o scrive, è *turni × contesto* — 640 turni su un contesto
da 600 k sono 235 M di token letti dalla cache, l'88 % della fase, e hanno esaurito il
limite delle 5 ore del piano. Quattro regole, da ora:

1. **Il brief porta estratti, non rimandi a documenti interi.** Il passo del piano, il
   paragrafo dell'ADR, le righe precise del codice: la lettura di partenza deve stare
   sotto ~40 k token. «Leggi `docs/05`» sono 2 053 righe che si ripagano a ogni turno;
2. **Un agente per passo, con un tetto**: se il contesto dell'autore supera ~200 k, il
   passo era due passi e si spezza al brief successivo. F6+F7 in un agente solo è
   costato il doppio di quel che valeva;
3. **Le correzioni e la seconda revisione vanno a un agente nuovo e stretto**, non alla
   ripresa di quello grasso: `SendMessage` conserva il contesto e lo fa ripagare a ogni
   turno — 47 M per 48 tool use. Un agente fresco con il verdetto, il rapporto delle
   correzioni e il diff dei soli file toccati parte da ~60 k. La ripresa resta giusta
   per un agente caduto a metà lavoro per un limite di utilizzo, dove il contesto è il
   lavoro;
4. **Risultati di tool corti**: `just ci 2>&1 | tail`, `sed -n a,b p` o `Read` con
   `offset/limit`, mai `cat` di un file intero. Il risultato di un tool entra nel
   contesto e non ne esce più.

Stima con queste regole: un passo come F7 costa ~250 turni × ~150 k ≈ 35–40 M invece
di 188 M.

**Il ritiro.** Deciso il **16 settembre 2026**, subito dopo la stima qui sopra, da chi
paga i token. Anche con le quattro regole un passo delegato costa decine di milioni di
token letti dalla cache — la fase `F6`+`F7` ne ha consumati **268 M** in tutto, contro i
~7 M della conversazione principale nello stesso arco — e il limite delle 5 ore del piano
è stato esaurito una volta. Il metodo aveva un pregio vero e misurato: il revisore a
contesto pulito ha trovato `R46`, `R47` e due test vacui che né l'autore né il
coordinatore avevano visto. Quel pregio si conserva senza agenti con la disciplina che
lo produceva: **ogni test si prova al contrario prima di consegnarlo**, rimettendo il
difetto e guardandolo cadere, e l'esito scrive quale assert è caduto; e chi ha chiesto il
lavoro rilegge il diff con lo stesso sospetto del revisore. Ciò che si perde è il giudizio
di un contesto che non aveva il piano in testa; è un costo, e lo si accetta.

*Trappola, per chi volesse riprovarci:* il consumo di un agente **non si vede dal suo
rapporto** — l'autore di `F6`+`F7` stimava 980 k token e ne aveva letti 235 M — e non
si vede dalla conversazione principale, che riceve solo il rapporto. Si vede solo nei
campi `usage` delle trascrizioni, sommando la cache letta turno per turno. Misurare
**prima** di decidere se delegare, non dopo il primo limite di utilizzo.

**Ciò che non cambia.** I commit si concordano come tabella e si fanno dalla
conversazione principale, dopo il rapporto del revisore; il push è una conferma a
parte; niente autoreferenza dell'assistente negli artefatti. **L'esito della 8 dice se il metodo ha retto**, con le
misure che ha: quanti passi delegati, quanti rifiutati alla verifica e perché, e se
qualche premessa falsa è passata inosservata all'agente.

### Decomposizione

L'ordine è quello del rischio: prima ciò che rompe la suite, poi ciò che la ripara,
poi il prodotto. Ogni passo è delegabile a un agente per passo, come scritto sopra, con
il suo criterio di fine; **`F1` è fatto e non si delega mai**.

##### `F1` — L'ADR · **fatto**

[ADR-053](08-decisioni.md#adr-053). Il resto di questa sezione lo cita e non lo ripete.

##### `F2` — Lo schema: `app.owner_invitations`

Una migration, `0040_owner_invitations.sql`, nella forma della `0037`: `id`, `email`
(`app.email_address`), `token_hash BYTEA` con indice unico, `expires_at` con il `CHECK`
sul `created_at`, `accepted_at`, `accepted_by` (FK a `users`, `ON DELETE SET NULL`, con il
suo indice per l'invariante 4), `revoked_at`, `created_at`. **Nessun `org_id`**: la
tabella è di piattaforma, e il controllo 1 delle invarianti va aggiornato perché la
riconosca come lo fa per `users` e `password_reset_codes`. **Nessun `updated_at`**, per
la ragione della `0037`. Un indice parziale unico su `email WHERE accepted_at IS NULL
AND revoked_at IS NULL`: un solo invito pendente per indirizzo, come `org_invitations_pending_uk`.
RLS attiva con una policy che nega tutto al ruolo applicativo (`USING (false)`): la
scrive il migratore dallo script, la legge il pool privilegiato; il controllo 2 vuole
una policy, e questa è la più stretta che soddisfi il controllo. `GRANT SELECT, UPDATE
(accepted_at, accepted_by)` al ruolo applicativo — è ciò che l'accettazione scrive — e
la coppia nel controllo 12. `docs/03` con la tabella e la ragione.

**Criterio di fine:** `./ops/ci/check-schema.sh` verde sul container effimero; un test
di persistenza che prova che `propflow_app` non può né inserire né leggere la tabella
fuori dal pool privilegiato, e che l'`UPDATE` su `email` o `token_hash` è `permission
denied`.

##### `F3` — Lo script che emette il token, e il runbook

`ops/dev/issue-owner-invitation.sh <email> [giorni]`: genera 32 byte casuali con
`openssl rand`, li codifica `base64url` senza padding **esattamente come
`random_token`** (il test di `F4` incolla un token prodotto dallo script, quindi le due
codifiche devono coincidere), calcola lo hash SHA-256 con `openssl dgst -binary`,
inserisce la riga con il ruolo del migratore (`psql` o il client dell'immagine, come gli
altri script di `ops/`) e stampa il token in chiaro una volta sola. Scadenza di default:
sette giorni, la stessa dell'invito all'inquilino. Il runbook in [07](07-operazioni.md):
quando si emette, come si consegna, come si revoca (`UPDATE … SET revoked_at`), come si
potano gli scaduti — tutto con il ruolo del migratore, come per `password_reset_codes`.

**Criterio di fine:** lo script gira contro il database di sviluppo, `bash -n` in CI
come per gli altri script, e la riga in `07` con il comando esatto.

##### `F4` — Il dominio e i casi d'uso, **con la porta vecchia ancora aperta**

Le rotte nuove si aggiungono; `POST /auth/register` resta com'è fino a `F6`, così la
suite non va mai in rosso e l'harness si può spostare un pezzo alla volta.

- `OwnerInvitation` nel dominio (`identity`), con `accept(now)` e le sue invarianti —
  pendente, non scaduta, non revocata — e senza `issue`: la emette lo script. Un
  `Invariant` con codice proprio, e la rotta che lo appiattisce su
  `401 auth.invalid_credentials` come per l'invito (decisione 6 dell'ADR);
- porta `OwnerInvitationRepository` con `find_pending_by_hash` e `mark_accepted`,
  implementazione su `PgUnitOfWork`, letta **dal pool privilegiato**;
- `AcceptOwnerInvitation`: il corpo di `RegisterOwner` di oggi più il token, stesso
  `begin_privileged`, la verifica del token per hash e per stato, l'email dell'invito al
  posto di quella del corpo. L'evento `UserRegistered` resta. Rotta:
  `POST /auth/accept-owner-invitation` (`token`, `full_name`, `password`,
  `organization_name?`), `201` con la sessione e il cookie come il login;
- `RegisterTenantFromInvitation`: `begin_privileged`, verifica del token in
  `org_invitations` per hash e per stato, `User::register` con l'email dell'invito, poi
  **le tre scritture di `AcceptInvitation` nello stesso ordine** — membership, parte
  contrattuale, invito marcato — e il suo evento. Non chiama `AcceptInvitation`: quello
  legge sotto `has_pending_invitation`, che qui non c'è; ne **condivide** le tre
  scritture attraverso una funzione del modulo, non una copia. Rotta:
  `POST /auth/register-from-invitation` (`token`, `full_name`, `password`), `201` con
  la sessione e il cookie;
- `AcceptInvitation` resta per chi ha già un account (ADR-051);
- rate limit come il login su entrambe; [05](05-api.md) e `openapi.rs`.

**Criterio di fine:** i test di contratto — un token da proprietario apre un account e
un'organizzazione; un token da inquilino apre un account **dentro** l'organizzazione di
chi ha invitato, con zero organizzazioni proprie e la parte collegata; token
inesistente, usato, scaduto e revocato rispondono **la stessa cosa**; un'email diversa
da quella dell'invito non è nemmeno un campo. La suite vecchia è ancora verde: la porta
vecchia è aperta.

##### `F5` — L'harness dei test passa dalle porte nuove

È la trappola più cara della sezione ed è il passo che la disinnesca, **prima** che la
porta vecchia si chiuda. Oggi ogni inquilino di test nasce con
`register_and_login_with_email` (owner della propria organizzazione) e poi accetta.
L'harness guadagna:

- `issue_owner_invitation(&app, email) -> token`, che scrive la riga **come farà lo
  script** — un `INSERT` con il ruolo del migratore attraverso la connessione di
  bootstrap che `common` già usa — così `register_and_login` diventa «emetti, accetta
  con il token, entra» e i **108** chiamanti non cambiano;
- `register_tenant_from_invitation(&app, token, name, password) -> (access_token, user_id)`,
  e `coliving_tenant` e simili passano da lì invece che da «registrati come owner, poi
  accetta».

Se la suite verde con l'harness nuovo richiede un comportamento del prodotto che l'ADR
non prevede, **ci si ferma**: è la premessa falsa da riferire, non da aggirare.

**Criterio di fine:** nessun test costruisce più un inquilino come owner; i test che
provavano proprio quel fatto — ad esempio quelli che contano le organizzazioni di un
inquilino — sono riscritti con l'aspettativa nuova, e **il vecchio valore compare nel
messaggio dell'assert** come cosa che non deve tornare. Suite verde.

##### `F6` — La porta vecchia si chiude

`POST /auth/register` **sparisce**: non «richiede un token», perché un corpo con un
campo in più su una rotta che si chiama ancora `register` insegnerebbe il contrario di
ciò che l'ADR decide. `RegisterOwner` viene rimosso; `openapi.rs`, [05](05-api.md) e il
test che elenca le rotte aperte senza sessione si aggiornano. Un test di contratto prova
che `POST /auth/register` risponde `404 endpoint.not_found`.

##### `F7` — Il client

- `/invito` chiede il codice; se non c'è una sessione, il passo dopo è il form di
  registrazione **con il codice già dentro** e senza il campo email — il nome
  dell'organizzazione compare solo se il server, alla verifica del codice, dice che è
  un invito da proprietario. Serve quindi una rotta di **sola lettura** del genere
  dell'invito? **No**: il form chiede nome e password e manda a
  `register-from-invitation`; se il token è da proprietario il server risponde
  `409 invitation.owner_token` e il client rimanda lo stesso corpo, più il nome
  dell'organizzazione, a `accept-owner-invitation`. Il codice esce **solo a chi
  possiede un token valido**, quindi non è un oracolo sui token emessi: chi indovina
  riceve `401` come per un token inesistente. L'ADR va emendato con questa riga se il
  passo la conferma;
- `/registrazione` da fuori sessione **smette di esistere come pagina aperta**: rimanda
  a `/invito` con una riga che dice che si entra su invito. Il link «Non hai un
  account? Registrati» della pagina di accesso porta lì;
- lo stato «nessuna organizzazione» nel guscio: quando `getOrganizations` risponde
  con zero elementi, il guscio mostra una schermata dedicata — chi si è, che l'accesso
  passa da un invito, l'uscita — e **nessuna navigazione**. Non è un errore e non è un
  vuoto di un elenco: è il terzo stato del guscio, accanto a «fuori sessione» e
  «dentro»;
- `AuthPage` perde il form di registrazione libera; i test che lo coprivano
  (`AuthPage.test.tsx`, i percorsi da tastiera) si spostano sul form nuovo.

**Criterio di fine:** tre test del client — la registrazione dall'invito che manda il
token e non un'email; il form che mostra il campo dell'organizzazione solo dopo il
`422`; il guscio con zero organizzazioni che mostra la schermata dedicata e **nessuna
richiesta con `X-Org-Id`** (è la proprietà del poscritto della 6, riapplicata).

##### `F8` — I documenti, l'esito, il piano della 9

[04](04-sicurezza.md) per i due flussi di registrazione e per il token di piattaforma;
[05](05-api.md) per le rotte; [07](07-operazioni.md) per il runbook; `R9` esce da
[12](12-da-fare.md); l'esito in [11](11-piani-conclusi.md) **con le misure del
metodo** — passi delegati, rifiutati alla verifica, premesse false passate inosservate;
e il piano della sezione 9 (`R8`, spiegare come si usa il sito) scritto subito dopo.

### Diario dell'esecuzione — le misure per l'esito

Si scrive a ogni passo, qui e non in un file di lavoro fuori dal repository: è ciò che
`F8` deve leggere per dire se il metodo ha retto, e una conversazione nuova non
avrebbe altro.

| Passo | Autore | Token · tool use · durata | Revisione | Esito |
|---|---|---|---|---|
| `F2` | Sonnet, 1 lancio | ~233 k · 105 · 15 min | conversazione principale | accettato; una correzione cosmetica (grant esplicito) |
| `F3`+`F4` | Sonnet, 1 lancio | ~502 k · 234 · 38 min | conversazione principale | accettato; una correzione minore (controllo numerico nello script) |
| `F5` | Sonnet, 1 lancio + 2 riprese per limiti di utilizzo | ~356 k · ~70 · ~10 min effettivi | **Opus a contesto pulito**: ~181 k · 66 · 8 min | accettato con 4 correzioni minori; **`R46`** (difetto di prodotto) e **`R47`** (errore sistemico nei docs) trovati dal revisore |
| `F6`+`F7` | Sonnet, 1 lancio + 1 ripresa per le correzioni (caduta per il limite delle 5 ore, ripresa il giorno dopo) | 1° giro: 556 turni · 326 · 35 min · **188 M** di cache letta; correzioni: 84 turni · 48 · 7 min · **47 M**; il test di `B1` rifatto da un agente fresco: 65 turni · 37 · 3 min · 4,4 M | 1ª: **Opus a contesto pulito**, 165 turni · 93 · 16 min · 27 M; 2ª: **Opus fresco a brief stretto** sulle sole correzioni, 81 turni · 38 · 15 min · 5,9 M (due giri) | rimandato alla 1ª revisione (2 bloccanti, 8 minori); accettato alla 2ª con 1 minore, chiuso e verificato al contrario |

**Che cosa gli autori hanno trovato di vero che il piano non sapeva:** `ALTER DEFAULT
PRIVILEGES` della `0015` concede a `propflow_app` ogni tabella nuova (il commento della
`0037` è falso per quel ruolo); `psql -c` non interpola `:'var'`, solo `-f`; `bash -n`
in CI non esiste; il login non ha rate limit; `TestEnv` di `persistence` non aveva
`TokenIssuer` né `RefreshTokenFactory`; `RegisterOwner` era usato 71 volte in
`persistence.rs` (il piano contava solo `register_and_login`). Nessuna aggirata: tutte
riferite.

**Decisioni prese dagli autori oltre il piano, accettate:** grant esplicito a
`propflow_worker` (è il pool privilegiato, e ha già `INSERT` ovunque dalla `0015`); le
rotte nuove in `auth.rs`; `email` nel corpo rifiutata con `422` come per `PATCH /me`;
`Denial::InvitationExpired/Revoked` riusati; fixture scaduti/revocati via `INSERT`
perché il worker non ha `UPDATE` su `revoked_at` — prova pratica del confine.

**Dove l'autore ha sbagliato a classificare:** in `F5` il token vecchio che continua a
dare accesso dopo l'uscita è stato letto come «assunzione del test» e coperto con un
login di comodo; era un difetto di prodotto (`R46`), e l'ha detto il revisore pulito
togliendo il login e guardando il `200` con il contratto nel corpo. È la ragione per cui
la seconda fase non è opzionale.

**Dove il coordinatore ha sbagliato:** ha creduto ai rapporti di `F2`–`F4` verificando
ciò che l'autore diceva di aver verificato; ha cancellato la sottosezione sul metodo
riscrivendo i passi (`d2944f8`) e non se n'è accorto rivedendo il proprio commit; ha
scritto nel brief di `F2` due premesse false (`password_reset_codes` documentata in
`03`, una «connessione di bootstrap» in `common`).

**Quel che il revisore pulito ha misurato e che nessun altro avrebbe visto:**
`coliving_tenant` rimesso sulla porta vecchia non è visto da **nessuno** degli undici
test chiamanti — il criterio «nessun inquilino nasce owner» è garantito dal `grep`, non
da un assert. Va saputo per `F6`.

**Limiti di utilizzo:** hanno colpito due volte l'autore di `F5` (limite di sessione,
poi settimanale). La ripresa con `SendMessage` allo stesso agente ha conservato il
contesto e il lavoro nell'albero; il costo è stato piccolo — **ma vedi sotto: era
piccolo perché il contesto era piccolo**.

**Che cosa l'autore ha trovato di vero che il piano non sapeva (F6+F7):** il «test che
elenca le rotte aperte senza sessione» non esiste; `AuthPage.test.tsx` non ha mai
coperto il form di registrazione (solo `LoginPage`), quindi non c'era niente da
spostare. Trovate dal coordinatore verificando prima del brief: `ops/dev/seed-walkthrough.py`
registrava tutti da `POST /auth/register` e il piano non lo nominava; il criterio di
fine di F7 diceva «dopo il `422`» dove il meccanismo è `409 invitation.owner_token`;
`malformed_email_is_rejected` non si «sposta»: sulle rotte nuove l'email non è un campo.

**Decisioni oltre il piano, accettate:** il form da invito vive in `features/invitations`
e non in `auth`, con `InvitationCodeError` estratta in `errors.tsx` per un ciclo di import;
le due chiamate di registrazione stanno in `SessionStore` (devono installare la sessione),
non in `ApiClient`; `clearOrganization()` nel contesto di sessione, senza il quale il guscio
avrebbe continuato a mandare l'`X-Org-Id` appena lasciato; il gate su `<Outlet/>` solo con
`organizationId === null`, così il caso comune non aspetta l'elenco; il seed prova il login
prima di emettere l'invito.

**Dove l'autore ha sbagliato a classificare (F7):** il test «nessuna richiesta con
`X-Org-Id`» partiva da una sessione già senza organizzazione, dove l'header non può
partire per costruzione: l'assert era vacuo, e `clearOrganization()` — la riga che
realizza la proprietà — non era vista da nessun test. L'ha detto il revisore togliendola
e guardando 127 verdi. Stessa forma del caso di F5: un test verde accanto al difetto.

**Dove il coordinatore ha sbagliato (F6+F7):** ha messo due passi in un agente (su
richiesta, ma il piano diceva «uno per passo») e il contesto è arrivato a 600 k; ha
chiesto letture intere (`docs/05`, cinque ADR, il poscritto) che sono costate 120 k di
contesto prima della prima modifica e si sono ripagate a ogni turno; ha ripreso l'agente
per le correzioni credendo che «tenere il contesto» fosse un risparmio — è stato il
contrario (§ sotto).

**Il costo vero, misurato dalle trascrizioni:** l'autore ha consumato l'88 % dei token
della fase (235 M di cache letta su 268 M totali). La causa è *turni × contesto*:
640 turni su un contesto cresciuto da 59 k a 600 k. La ripresa per le correzioni è
costata 47 M per 48 tool use, un quarto del primo giro. Il revisore pulito, con 165
turni su ~160 k medi, è costato 27 M. Il limite delle 5 ore del piano è stato toccato
una volta (autore, durante le correzioni).

**La misura è nei campi `usage` delle trascrizioni**, non nelle stime degli agenti: l'autore
di `F6`+`F7` stimava ~980 k token e 176 tool use, il sistema ne contava 510 k di contesto
finale e 287 tool use; le stime di `F2`–`F5` nella tabella sono di quella specie e vanno
lette come ordini di grandezza. Da qui in poi la colonna riporta i turni API e la cache
letta, che è ciò che il limite del piano conta.

### Trappole

**Scrivere il codice dell'ADR prima dell'ADR.** La tentazione è concreta: «basta un
flag su `RegisterOwner`». Ogni risposta alla domanda 1 cambia il guscio, i test e le
policy in modi diversi, e un flag messo prima è una scelta fatta senza dirlo.

**Un utente senza organizzazione fa `403` ovunque.** Non è un caso limite: con la
risposta 1 è **lo stato normale** di un invitato fra la registrazione e l'accettazione —
o di chi ha rifiutato l'invito. Il guscio deve saperlo mostrare, e `getOrganizations`
è già senza `X-Org-Id` per la ragione della 6.

**`pick_default_membership` preferisce `Owner`.** Chi tocca la registrazione senza
toccare il login lascia gli utenti vecchi ad atterrare in un'organizzazione vuota.

**`begin_privileged()` non ha Row Level Security.** La registrazione dall'invito
verifica un token *prima* di avere un contesto: la prova di possesso è dell'applicazione
(hash confrontato), come già in `AcceptInvitation`, ma là le policy facevano da seconda
guardia e qui non ci sono. Il token va cercato per hash e per **stato**, e l'email
dell'invito va confrontata con quella che si registra.

**L'harness dei test dà per scontato che ogni inquilino sia owner di qualcosa.**
`register_and_login_with_email` seguita dall'accettazione è in 108 test; l'harness
si sposta sulle porte nuove **mentre la vecchia è ancora aperta** (`F5` prima di
`F6`), con la suite verde a ogni passo — altrimenti si corregge il prodotto guardando
test che descrivono il mondo vecchio, o si chiude la porta con cento test rossi.

**`platform_role = 'admin'` si assegna solo a mano**, e il valore è `'admin'`, non
`'platform_admin'`. Un runbook in [07](07-operazioni.md) è il minimo; una rotta è un
altro perimetro di sicurezza da difendere, e va deciso nell'ADR se ne valga la pena.

**L'oracolo delle email esiste già** (`409 auth.email_taken`) e la registrazione
dall'invito ne aggiunge un secondo se risponde diversamente a «token valido, email
diversa» e a «token inesistente». Le due risposte devono essere **la stessa**.

### Come si saprà che è finita

- un indirizzo sconosciuto non può diventare proprietario di niente;
- un inquilino invitato si registra **senza** creare un'organizzazione, e al primo accesso
  è nell'organizzazione di chi lo ha invitato, come inquilino;
- un proprietario nuovo entra con un token emesso da un amministratore di piattaforma, e
  il runbook per emetterlo è scritto;
- gli utenti esistenti entrano come prima, e ciò che accade alle loro organizzazioni
  personali vuote è scritto nell'ADR;
- l'ADR esiste, è citato dalle policy e dai casi d'uso che ne discendono, e la suite è
  verde dopo aver cambiato l'harness;
- `just ci` verde, e **il piano della sezione 9 scritto nello stesso commit dell'esito**.

### Che cosa **non** entra

- **una richiesta di accesso come proprietario dentro il prodotto**: la verifica è fuori,
  e il prodotto registra solo il suo esito;
- **ruoli oltre i tre** (`owner`, `manager`, `tenant`): l'ADR può nominarli, la sezione
  non li costruisce;
- **`R37`**, la cancellazione delle persone, e **`R41`**, il rientro: sono domande sulla
  membership che questa sezione non cambia;
- **`R44`**, l'immobile che nasce in bozza: è dell'immobile, non del ruolo.

<a id="esito-sezione-8"></a>

## Esito — Sezione 8: il perimetro dei ruoli

Eseguita dal **13 al 16 settembre 2026**. Chiude `R9` e, per strada, `R47`.

**Un ADR, una migration, uno script, una rotta in meno e due in più, un terzo stato
del guscio.** I test Rust passano da **461 a 478**, quelli del client da 124 a **129**.
`just ci` verde, zero `SALTATO`. È anche **la prima sezione eseguita con i ruoli
separati — pianificazione, codice, revisione — e l'ultima**: il metodo è stato
misurato e ritirato, e la misura è la parte di questo esito che vale di più.

### Che cosa è entrato

- [ADR-053](08-decisioni.md#adr-053), scritto **prima** del codice: l'inquilino non ha
  un'organizzazione propria e si registra dall'invito, dentro l'organizzazione di chi lo
  ospita, in una transazione; il proprietario nasce da un invito di piattaforma; la
  verifica è fuori dal prodotto; il database si svuota; il login non cambia; `/invito` è
  l'ingresso per entrambi. Emendato il 16 settembre con la riga sul
  `409 invitation.owner_token`, che il passo `F7` ha confermato;
- la migration `0040_owner_invitations.sql`: una tabella di piattaforma senza `org_id`,
  RLS attiva con `USING (false)`, privilegi di colonna per l'accettazione, riconosciuta
  dai controlli 1, 2 e 12 delle invarianti;
- `ops/dev/issue-owner-invitation.sh` con il runbook in [07](07-operazioni.md#emettere-un-invito-da-proprietario):
  emissione, consegna, revoca, potatura, tutto con il ruolo del migratore;
- `OwnerInvitation` nel dominio, la porta e la sua implementazione sul pool
  privilegiato, `AcceptOwnerInvitation` e `RegisterTenantFromInvitation`, e le due rotte
  `POST /auth/accept-owner-invitation` e `POST /auth/register-from-invitation`;
  `AcceptInvitation` resta per chi ha già un account;
- **`POST /auth/register` non esiste più** e risponde `404 endpoint.not_found`;
  `RegisterOwner` è rimosso;
- l'harness dei test: `register_and_login` è diventato «emetti un invito da
  proprietario, accettalo, entra» senza cambiare i suoi 108 chiamanti, e nessun test
  costruisce più un inquilino come owner di qualcosa;
- il client: `/invito` da fuori sessione chiede il codice e poi mostra il form di
  registrazione con il codice già dentro e senza email; il nome dell'organizzazione
  compare solo dopo il `409`; `/registrazione` rimanda a `/invito`; e il **terzo stato
  del guscio** — nessuna organizzazione, nessuna navigazione, nessun `X-Org-Id` — con il
  suo ramo d'errore;
- `ops/dev/seed-walkthrough.py` passa dalle porte nuove: nessuno dei suoi personaggi
  nasce più da una rotta aperta.

### Come si saprà che è finita — riletta

| Criterio | Esito |
|---|---|
| un indirizzo sconosciuto non può diventare proprietario di niente | vero: senza un token in `owner_invitations` non c'è rotta che crei un'organizzazione |
| un inquilino invitato si registra senza creare un'organizzazione, e al primo accesso è in quella di chi lo ha invitato | vero, provato dal contratto: zero organizzazioni proprie, membership `tenant`, parte collegata, tutto in una transazione |
| un proprietario nuovo entra con un token emesso da un amministratore, e il runbook è scritto | vero |
| gli utenti esistenti entrano come prima, e le loro organizzazioni vuote sono nell'ADR | vero nella forma decisa: il database si svuota, nessuna migration di dati |
| l'ADR esiste ed è citato da policy e casi d'uso | vero |
| `just ci` verde e il piano della 9 nello stesso commit | vero |

### Il piano aveva premesse sbagliate, e il metodo le ha trovate tutte — ma non da solo

Otto, contro le tre della 7, e nessuna aggirata. Le ha trovate chi scriveva il codice:
`ALTER DEFAULT PRIVILEGES` della `0015` concede a `propflow_app` ogni tabella nuova, quindi
il commento della `0037` era falso per quel ruolo; `psql -c` non interpola `:'var'`;
`bash -n` in CI non esiste; il login non ha un rate limit da copiare; `TestEnv` di
`persistence` non aveva `TokenIssuer`; `RegisterOwner` era usato 71 volte in
`persistence.rs` e il piano contava solo `register_and_login`; il «test che elenca le
rotte aperte senza sessione» non esiste; `AuthPage.test.tsx` non ha mai coperto il form
di registrazione. E tre le ha trovate il coordinatore verificando prima di scrivere il
brief di `F6`: il seeder del collaudo sulla porta vecchia, che il piano non nominava; il
criterio di fine di `F7` che diceva `422` dove il meccanismo è `409`; e un test che non
si «sposta» perché sulle rotte nuove l'email non è un campo.

### Che cosa il revisore ha visto e nessun altro avrebbe visto

È la misura che decide se il metodo valeva, ed è netta. Il revisore a contesto pulito ha
trovato, in due passi:

- **`R46`**: chi è uscito da un'organizzazione continua a leggere e scrivere con il
  token vecchio per quindici minuti, e dopo ADR-053 è il caso normale di ogni inquilino.
  L'autore di `F5` l'aveva coperto con un login «di comodo» nel test, classificandolo
  come assunzione dell'harness; il revisore ha tolto il login e ha guardato il `200`;
- **`R47`**: ADR-013 citato per ADR-012 in dieci punti, ereditato copiando;
- un test di `F7` **vacuo**: «nessuna richiesta con `X-Org-Id`» partiva da una sessione
  già senza organizzazione, dove l'header non può partire per costruzione, e la riga
  che realizza la proprietà (`clearOrganization()`) non era vista da nessun test —
  tolta, 127 verdi. Stessa forma del caso di `F5`: un test verde accanto al difetto;
- il terzo stato del guscio senza ramo d'errore, contro la regola 23;
- `coliving_tenant` rimesso sulla porta vecchia non era visto da nessuno degli undici
  test chiamanti: il criterio «nessun inquilino nasce owner» era garantito dal `grep`,
  non da un assert.

Nessuna premessa falsa è passata inosservata a **entrambi**, per quanto si sa. Ma il
punto è un altro: **tre di questi cinque li ha trovati solo la revisione al contrario**,
rimettendo il difetto e guardando il test — non leggendo il diff. È la disciplina che
resta anche senza il revisore, ed è scritta come tale nel ritiro del metodo.

### Il metodo, misurato, e perché è stato ritirato

| Passo | Autore | Revisione | Esito alla revisione |
|---|---|---|---|
| `F2` | Sonnet | conversazione principale | accettato, 1 correzione cosmetica |
| `F3`+`F4` | Sonnet | conversazione principale | accettato, 1 correzione minore |
| `F5` | Sonnet (2 riprese per limiti) | Opus pulito | accettato con 4 minori; `R46`, `R47` |
| `F6`+`F7` | Sonnet (1 ripresa) | Opus pulito, poi Opus stretto | **rimandato** (2 bloccanti, 8 minori); accettato con 1 minore, chiuso |

Sei passi delegati in quattro lanci; uno rifiutato alla prima revisione; nessuno
rifiutato due volte. Sul **merito** il metodo ha retto: ogni passo consegnato era
eseguibile, le premesse false sono state riferite e non aggirate, e il revisore pulito
ha trovato ciò che il coordinatore non trovava. Sul **costo** no, e la misura è arrivata
tardi perché nessuno la stava guardando: il consumo di un agente **non si vede dal suo
rapporto** — l'autore di `F6`+`F7` stimava 980 k token e ne aveva letti 235 M — e non si
vede dalla conversazione principale, che riceve solo il rapporto. Sta nei campi `usage`
delle trascrizioni, e sommandoli:

| Contesto | Turni | Cache letta | Output |
|---|---|---|---|
| Autore Sonnet, `F6`+`F7` primo giro | 556 | 188 M | 190 k |
| Autore Sonnet, ripresa per le correzioni | 84 | 47 M | 26 k |
| Revisore Opus, prima passata | 165 | 27 M | 52 k |
| Revisore Opus stretto, due giri | 81 | 6 M | 20 k |
| Agente fresco per un test | 65 | 4 M | 16 k |
| Coordinatore | ~70 | ~7 M | ~90 k |

Il costo è **turni × contesto**: l'output di tutti gli agenti insieme è 394 k token, lo
0,14 % del totale; il resto è lo stesso contesto riletto a ogni turno, da 59 k a 600 k.
Tre cause, in ordine: letture obbligatorie intere (120 k prima della prima modifica,
ripagati 556 volte); due passi in un agente; la ripresa per le correzioni, che «tiene il
contesto» e lo fa pagare a ogni turno — 47 M per 48 tool use. Le quattro regole scritte
il 16 settembre (estratti, un agente per passo con un tetto, correzioni a un agente
fresco, output corti) hanno abbassato la revisione da 27 M a 6 M alla prima prova, su un perimetro più
stretto; **non è bastato**, e da `F8` una sola conversazione fa tutto, come nelle sezioni 1–7. La
ragione e la trappola stanno nel [piano](#piano-sezione-8), sotto «ritirato».

### Decisioni prese eseguendo

- **grant esplicito a `propflow_worker`** sulla tabella nuova, anche se `ALTER DEFAULT
  PRIVILEGES` lo dava già: scritto perché si legga;
- **`email` nel corpo delle rotte da invito rifiutata con `422`**, come per `PATCH /me`,
  e non ignorata: un campo ignorato in silenzio insegna che esiste;
- fixture scaduti e revocati scritti con `INSERT` e non con `UPDATE`, perché il ruolo
  privilegiato dei test non ha `UPDATE` su `revoked_at`: prova pratica del confine;
- il form da invito in `features/invitations`, non in `auth`: è l'altra metà del flusso
  che vive lì; `InvitationCodeError` estratta per un ciclo di import;
- le due chiamate di registrazione in `SessionStore` e non in `ApiClient`: installano
  una sessione, e uno store che non manda mai `X-Org-Id` è il posto giusto per una
  richiesta di chi non ha ancora un'organizzazione;
- `clearOrganization()` nel contesto di sessione: senza, il guscio continua a mandare
  l'`X-Org-Id` appena lasciato fino a un reload;
- il gate su `<Outlet/>` solo quando l'organizzazione è nulla: il caso comune non
  aspetta l'elenco;
- il seeder prova il login prima di emettere l'invito: rilevare «già seminato» sul `409`
  lasciava un invito pendente e faceva fallire il terzo giro sull'indice unico;
- `register_owner.rs` non rinominato: contiene ancora le scritture condivise.

### Verificato al contrario

Ogni test nuovo di `F6` e `F7` è stato rimesso sul difetto e guardato cadere, dal
revisore e non dall'autore: il `404` su `POST /auth/register` con l'assert su `405`
(`left: 404`); la password debole su entrambe le rotte togliendo `password_policy::validate`
(`left: 201, right: 422` due volte); l'email malformata rinominando il campo dell'errore;
il doppio invito da proprietario scambiando il codice del conflitto **nel punto giusto**
— il primo tentativo sul ramo `map_email_conflict` non cadeva, perché quel ramo non è sul
percorso: il `409` viene dal controllo esplicito `find_by_email`; il token e non l'email
nel corpo (`not to have property "email"`); il campo dell'organizzazione reso sempre
(`found <input`); `clearOrganization()` tolta (`Sara Neri` non compare); il ramo d'errore
tolto (`role="alert"` non compare); il `setQueryData` tolto (`expected 2 to be 1`); e la
sonda su `/properties` con l'`<Outlet/>` reso accanto alla schermata
(`expected true to be false`).

### Che cosa **non** è stato provato a mano

Niente di questa sezione. Il giro a mano di `F7` richiede il database di sviluppo
svuotato (decisione 4 dell'ADR: `just down-hard`, `just setup`) e non è stato fatto:
`/invito` da fuori sessione, il form a due passi con un token da proprietario vero
emesso dallo script, l'ingresso di un inquilino da un invito, lo stato senza
organizzazione dopo un'uscita. Sono coperti da test, e la 6 e la 7 hanno insegnato che
non basta. **È il primo passo della sezione 9**, prima di scrivere una riga.

### Che cosa resta aperto

- **`R46`**, deciso di non fare qui: il token vecchio dopo l'uscita. Due vie nella voce;
- **`R48`**, nuova: chi ha già un account e nessuna organizzazione non può diventare
  proprietario — l'ADR non copre l'accettazione di un invito da proprietario da un
  account esistente;
- **`R49`**, nuova: un secondo browser con in `localStorage` un'organizzazione lasciata
  altrove parte con un `X-Org-Id` morto — la finestra del poscritto della 6, riapplicata;
- **`R41`**, il rientro: questa sezione non lo tocca, come l'ADR dice.

<a id="poscritto-sezione-8"></a>

### Poscritto — il giro a mano della 8, fatto come `G0` della 9

**16 settembre 2026**, il giorno dopo l'esito, con il database svuotato (decisione 4
dell'ADR) e su porte a parte, perché `5432`, `8080` e `8081` erano di un altro progetto
sulla stessa macchina. Percorsi tutti e cinque i punti di `G0` più il seeder. Esito: un
difetto bloccante sul percorso principale della sezione, una stringa falsa, tre
scoperte da mettere in [12](12-da-fare.md), e tutto il resto come previsto.

**Il difetto che nessun test poteva vedere.** `/invito` da fuori sessione era una pagina
bianca, con `No QueryClient set` nella console: `AcceptInvitationSignedOutPage` chiamava
`useQueryClient()`, ma le rotte da fuori sessione stanno **fuori** da `QueryProvider`
(`App.tsx`: il provider avvolge solo il ramo autenticato). Il test della pagina la
avvolgeva in un provider suo, ed era verde; l'autore, due revisori e il coordinatore
l'hanno letto senza vederlo, perché nessuno ha confrontato l'albero del test con quello
di `App`. Il rimedio è togliere l'hook — l'invalidazione era comunque inutile: quando la
sessione nasce, `App` monta un `QueryClient` nuovo — e **rendere la pagina nel test come
la rende il prodotto**, senza provider. Al contrario: con `useQueryClient()` rimesso, due
test cadono con lo stesso errore della console. È la terza volta che un test verde sta
accanto al difetto in questa sezione, e la forma è sempre la stessa: l'harness dà al
codice qualcosa che il prodotto non gli dà.

**La stringa falsa.** Chi ripresenta da dentro sessione un invito già accettato leggeva
«sei già dentro»; dalla 6 si può uscire, e chi rilegge quel codice dopo l'uscita è
esattamente chi non è dentro. Ora dice che l'invito è speso e che la via è un invito
nuovo (`R41`).

**Che cosa ha retto, visto con gli occhi:** l'invito da proprietario dallo script; il form
senza email; il `409` che fa comparire il nome dell'organizzazione con nome e password
conservati; l'ingresso sul riepilogo vuoto come «Giro Immobili — Proprietario»; l'ingresso
dell'inquilina da un invito su `/addebiti`, con **una sola** organizzazione nel menu e
nessuna propria; l'uscita rifiutata con il contratto attivo (`409`, con il request id) e
riuscita dopo la disdetta; la schermata «Nessuna organizzazione» senza navigazione, con
dopo l'uscita **solo** `GET /organizations` e `GET /me` in rete; il ramo d'errore con
«Riprova» e «Esci» staccando `/organizations`, e la schermata dedicata dopo la riprova;
`/registrazione` come rimando; un token inesistente e uno già usato con lo **stesso**
messaggio, due `401` sulla stessa rotta; il seeder a buon fine sulle porte nuove e, alla
seconda esecuzione, il rifiuto con il messaggio previsto e zero inviti pendenti lasciati.

**Non verificabile con lo strumento, non un difetto:** l'Invio nel campo del codice. Il
browser pilotato manda eventi non fidati, che non fanno scattare l'invio implicito del
form; `form.requestSubmit()` avanza al passo due, quindi il form è corretto. Va provato
con una tastiera vera nel giro della 9.

**Tre scoperte per [12](12-da-fare.md):** la sonda `/ready` sta alla radice e il proxy
inoltra solo `/api/v1/*`, quindi il passo 4.1 del runbook del primo deploy non può
riuscire dal dominio pubblico (`R50`); un refresh fallito per rete al caricamento è
trattato come «nessuna sessione» e manda alla pagina di accesso senza dirlo (`R51`);
e la conferma, con gli occhi, di `R48`: dalla schermata senza organizzazione un token da
proprietario riceve «codice non valido».
<a id="piano-sezione-9"></a>

## Piano — Sezione 9: spiegare come si usa il sito

Pianificata il **16 settembre 2026**, subito dopo l'esito della 8 e sotto la regola che
una sezione conclusa pianifica la successiva. Chiude `R8`. È l'ultima delle sezioni
decise dopo il collaudo, ed era ultima **apposta**: scrivere spiegazioni su schermate
che cambiano significa riscriverle, e fino alla 8 cambiavano. Ora l'ingresso, il
guscio e i moduli hanno la forma che avranno al primo deploy.

**Si esegue in una sola conversazione**, come le sezioni 1–7: il metodo degli agenti è
stato [ritirato](#piano-sezione-8) con la 8. Resta la disciplina che
quel metodo aveva reso visibile: ogni test provato al contrario prima di consegnarlo,
con l'assert caduto scritto nell'esito.

### Che cosa esiste davvero, verificato e non dedotto

| Premessa | Verifica |
|---|---|
| Non esiste nessuna spiegazione del prodotto nel client | vero: `grep -n "help\|aiuto\|guida" apps/web/src/i18n/it.ts` non trova niente; la navigazione (`nav` in `it.ts`, righe 284–297) porta solo i nomi delle sezioni |
| Gli stati vuoti esistono e sono progettati | vero: `Empty` in `apps/web/src/components/state/index.tsx` (riga 56) accetta `title`, `hint`, `action`; è usato **22 volte** in `features/` |
| …ma quasi tutti sono un titolo e basta | **falso, ed era falso già il 16 settembre**: la riga diceva «5 usi su 22 con un `hint`, uno solo con un `action`», e contando davvero (17 settembre, rilettura prima di `G1`) i `hint` sono **13** — tre condizionali: la coda dei pagamenti solo per lo staff, spese e rimborsi solo se il gruppo non è archiviato — e gli `action` sono **4** (riepilogo, immobili, contratti, bollette; gli ultimi tre solo per lo staff), aggiunti fra il 4 e il 7 settembre e mai contati. **Senza `hint` restano 9**, e sono quasi tutti vuoti di *sotto-elenco* dentro una scheda (stanze e residenti di un immobile, movimenti di un addebito, storia di una segnalazione, allegati, membri e candidati di un gruppo di spesa) più due di elenco: **gli addebiti**, che è la prima schermata di ogni inquilino, e i pagamenti visti da chi abita. `G1` quindi non «riempie 22 vuoti»: ne riempie 9 e **rilegge** gli altri 13 contro il criterio delle tre cose. Due cose viste contando, che entrano in `G1`: `PaymentsPage` usa come vuoto dell'elenco `t.payment.empty`, «Nessun movimento su *questo addebito*», che è la stringa della scheda dell'addebito ed è falsa su `/pagamenti`; e `ChargesPage` ha **due** filtri (`insoluti` e `stato`) ma il titolo del vuoto guarda solo il primo, quindi con `?stato=` il vuoto dice «Nessun addebito» a chi ne ha |
| Esiste un precedente di pagina lunga nel client | vero: `features/legal/PrivacyPage.tsx`, con il testo in `it.ts` come `privacy.sections: [{title, body}]` (riga 159) — è la forma da riusare, non una nuova |
| Il primo accesso di un proprietario atterra sul riepilogo vuoto | vero: `DashboardPage.tsx:36` mostra `Empty` con `t.dashboard.empty` e `t.property.emptyHint` quando `properties_count === 0 && active_leases === 0` |
| Il primo accesso di un inquilino atterra sugli addebiti | vero: `TENANT_HOME = '/addebiti'` (`shell.tsx:104`), e un inquilino appena entrato con un contratto senza addebiti vede «Nessun addebito» — una schermata che non dice **perché è lì** né che cosa arriverà |
| Chi non ha un'organizzazione vede una schermata che spiega | vero dalla 8: `NoOrganizationScreen` dice che cosa si è e che si entra da un invito |
| Il giro a mano della 8 non è stato fatto | vero al momento del piano; **fatto** il 16 settembre come `G0`, [poscritto](#poscritto-sezione-8) |

### La decisione: dove sta la spiegazione

`R8` lascia aperto *dove*: primo accesso guidato, pagina d'aiuto, testi contestuali, o
una combinazione. Si decide qui, e non con un ADR — non è una decisione di modello né
di sicurezza — ma con le alternative scritte.

**Scelto: gli stati vuoti spiegano, e una pagina sola raccoglie.** Due livelli, nell'ordine:

1. **Ogni stato vuoto dice tre cose**: che cos'è questa sezione (una riga), che cosa
   la riempie (il gesto, con il bottone se chi guarda può farlo), e — per chi abita —
   chi la riempirà al posto suo. È il posto più economico perché è già progettato
   (regola 23), è dove la domanda «e adesso?» nasce, e non richiede navigazione. Il
   `hint` esiste, l'`action` esiste: si riempiono;
2. **una pagina `/guida`**, nella forma di `PrivacyPage`: un titolo per sezione della
   navigazione, tre-cinque righe ciascuna, **due versioni** — per chi amministra e per
   chi abita — scelte dal ruolo, non due pagine. Raggiungibile dal menu dell'account
   (dove stanno le cose che si fanno su di sé) e dalla schermata senza organizzazione.
   Il testo vive in `it.ts` come `guide.staff.sections` e `guide.tenant.sections`, con
   la stessa forma di `privacy.sections`.

*Alternativa scartata: il primo accesso guidato* (un tour a passi sopra l'interfaccia).
È la più cara — stato «già visto» da persistere per utente, un componente di overlay,
test per ogni passo — e la meno utile: il proprietario nuovo vede una schermata vuota,
e una schermata vuota che spiega sé stessa è già un primo accesso guidato senza
overlay. Si riapre se un giro a mano con una persona esterna dice che non basta.

*Alternativa scartata: i tooltip sui bottoni.* Spiegano un gesto, non una sezione, e
non si vedono dal telefono.

*Alternativa scartata: il manuale in `docs/`.* Chi usa il prodotto non apre il
repository; e la regola 18 dice che il testo visibile sta in `it.ts`, non in un
Markdown servito a parte.

**Ciò che la pagina non è:** non è un elenco di funzioni. Racconta un ciclo — si
registra un immobile, si fa un contratto, si invita chi abita, arrivano gli addebiti,
si registrano gli incassi — e per chi abita l'altro lato dello stesso ciclo. Il
glossario di [01](01-prodotto.md#glossario) è la fonte dei nomi.

### Decomposizione

##### `G0` — Il giro a mano della sezione 8 · **fatto** il 16 settembre 2026

Esito nel [poscritto della 8](#poscritto-sezione-8): un difetto
bloccante che nessun test vedeva (`/invito` da fuori sessione era una pagina bianca —
`useQueryClient()` fuori da `QueryProvider`, e il test la avvolgeva in un provider che il
prodotto non ha), una stringa falsa, `R50` e `R51` in [12](12-da-fare.md). Entrambi i
difetti corretti e provati al contrario prima di `G1`. L'Invio nel campo del codice non
è verificabile con il browser pilotato: **va provato con una tastiera vera** in `G1`.
Il testo del passo, com'era:

Con il database svuotato (`just down-hard`, `just setup`) e `ops/dev/seed-walkthrough.py`
per il mondo di sfondo:

1. `./ops/dev/issue-owner-invitation.sh` per un indirizzo nuovo; `/invito` da fuori
   sessione; il codice; il form senza email; il `409` che fa comparire il nome
   dell'organizzazione; l'ingresso sul riepilogo vuoto;
2. dallo stesso account, un immobile, un contratto, un invito; in un altro browser
   `/invito` con quel codice, il form, l'ingresso **da inquilino** sugli addebiti,
   nessuna organizzazione propria nel menu;
3. l'inquilino esce dall'organizzazione (`/account`); la schermata «nessuna
   organizzazione», nessuna navigazione, la riprova con `/organizations` staccando la
   rete, l'uscita;
4. `/registrazione` da fuori sessione: il rimando, non un form;
5. un token inesistente, e lo stesso token usato due volte: `401` identici.

Ciò che non torna entra in [12](12-da-fare.md) **prima** di cominciare `G1`: è la
regola che la 6 e la 7 hanno pagato.

##### `G1` — Gli stati vuoti spiegano

Per ognuno dei 22 usi di `Empty`: un `hint` che dica che cos'è la sezione e che cosa la
riempie, e l'`action` con il gesto dove chi guarda può farlo (`useIsStaff()`, ADR-042:
è una scelta di che cosa offrire). L'inquilino legge chi lo farà per lui («Il
proprietario registra qui le bollette; quando ne arriva una la vedi in questa lista»).
Le stringhe in `it.ts`, una per stato, accanto al titolo che già esiste. **Niente
componenti nuovi**: `Empty` ha già i tre slot.

Trappola: gli stati vuoti *filtrati* — «nessuna segnalazione aperta» con `showAll`
falso, «nessun addebito da saldare» con `outstanding` — non sono vuoti della sezione,
sono vuoti del filtro, e il testo deve dirlo («non c'è niente da fare qui: le chiuse
sono sotto *tutte*») invece di spiegare la sezione a chi la conosce già.

**Criterio di fine:** nessun `Empty` senza `hint`; un test per schermata nuova non
serve — sono stringhe — ma i test esistenti che cercano il titolo del vuoto restano
verdi, e uno prova che l'`action` **non** compare per chi non è staff.

##### `G2` — La pagina `/guida`

`features/guide/GuidePage.tsx` nella forma di `PrivacyPage`; `guide.staff` e
`guide.tenant` in `it.ts`, con `sections: [{title, body}]`; la versione scelta da
`useIsStaff()`; la rotta dentro il guscio; il collegamento dal menu dell'account
(`shell.tsx`, accanto a «Il mio account») e da `NoOrganizationScreen` (dove la versione
è quella di chi abita: chi non ha un'organizzazione aspetta un invito). Una sezione per
voce della navigazione, più «Come si entra» (l'invito, i due generi, l'email che è
quella dell'invito) e «Il tuo account» (password, uscita da un'organizzazione, privacy).

**Criterio di fine:** un test che la pagina mostra le sezioni dello staff a uno staff e
quelle dell'inquilino a un inquilino, provato al contrario scambiando il ruolo;
`typecheck` e `lint` verdi, e nessuna stringa nel JSX — non c'è un lint che lo
verifichi, lo verifica chi rilegge il diff, ed è una trappola elencata sotto.

##### `G3` — Il riepilogo vuoto è il primo accesso

Il `Empty` del riepilogo (`DashboardPage.tsx:36`) è il primo schermo di ogni
proprietario nuovo: diventa il posto dove il ciclo è raccontato in tre passi con i tre
bottoni — registra un immobile, poi un contratto, poi invita — e il collegamento alla
guida. Non un componente di overlay: è lo stesso `Empty`, con un `action` che è un
gruppo di tre invece di uno. Per l'inquilino l'equivalente è il vuoto degli addebiti,
che dice perché è lì.

**Criterio di fine:** il riepilogo **non ha un test** oggi (`features/dashboard/` contiene
solo `DashboardPage.tsx`): se ne scrive uno per il vuoto con i tre gesti, provato al
contrario con il ruolo scambiato — il gesto assente per chi non è staff.

##### `G4` — L'esito, `R8` fuori da [12](12-da-fare.md), la decisione sul deploy

L'esito in [11](11-piani-conclusi.md); `R8` esce; e con la 9 finisce la lista delle
sezioni decise dopo il collaudo. Il passo dopo non è una sezione: è la decisione se
fare il [primo deploy](07-operazioni.md#il-primo-deploy--runbook) con i mezzi che ci
sono, e quali fra `R46`, `R48`, `R49` e `R42` debbano precederlo. Si scrive qui, come
«prossimo passo suggerito», e non come piano.

### Trappole

**Scrivere la guida prima del giro a mano.** `G0` viene prima perché una spiegazione
scritta su una schermata che non funziona come si crede insegna la cosa sbagliata; la 8
non ha avuto un giro a mano, e la 6 e la 7 hanno mostrato che i test non bastano.

**Spiegare il prodotto invece della sezione.** Il `hint` di uno stato vuoto ha una riga,
due al massimo; se serve di più, il posto è la guida, e il vuoto rimanda lì.

**Due pagine invece di una con due versioni.** Sembra più pulito ed è il doppio dei
collegamenti, dei test e dei posti dove il testo diverge senza che nessuno se ne
accorga. Una pagina, il ruolo sceglie.

**Il testo nel JSX.** È la regola 18 e qui è dove si viola più facilmente, perché sono
paragrafi e non etichette. `privacy.sections` mostra che i paragrafi stanno in `it.ts`
come tutto il resto.

**La formattazione a mano dei nomi.** Le sezioni della guida si chiamano come le voci
della navigazione: `t.nav.*`, non una copia.

### Che cosa **non** entra

- **un primo accesso guidato con overlay**: scartato sopra, riaperto solo da un giro con
  una persona esterna;
- **la traduzione in inglese**: decisa e rimandata da ADR-039; la guida nasce in `it.ts`
  con le chiavi, e tradurre sarà aggiungere, non riscrivere;
- **`R46`, `R48`, `R49`**: sono del perimetro dei ruoli, e vanno decisi nella scelta sul
  deploy, non qui;
- **le email di notifica** (piano in [09 com'era](#prima-degli-utenti-veri), «Le notifiche»):
  spiegano un evento, non il sito.

<a id="esito-sezione-9"></a>

## Esito — Sezione 9: spiegare come si usa il sito

Eseguita il **17 settembre 2026**, in una conversazione sola e senza agenti, com'era
scritto. Chiude `R8`. I test Rust restano **478**, quelli del client passano da 129 a
**138**. `just ci` verde, zero `SALTATO`. È l'ultima delle sezioni decise dopo il
collaudo, e con lei finisce la lista: il passo dopo non è una sezione ma la decisione sul
[primo deploy](09-stato-e-roadmap.md#prossimo-passo-suggerito).

### Che cosa è entrato

- **ventidue stati vuoti che spiegano**, per ruolo: i nove che avevano solo un titolo
  hanno un `hint` — le stanze e chi abita di un immobile, i movimenti di un addebito, la
  storia di una segnalazione, gli allegati, i membri e i candidati di un gruppo di spesa,
  gli addebiti e i pagamenti — e i tredici che lo avevano già sono stati riletti contro il
  criterio delle tre cose: che cos'è la sezione, che cosa la riempie, chi la riempie per
  chi abita. Il gesto compare come `action` solo per chi può farlo (`useIsStaff()`,
  ADR-042) e solo negli elenchi di sezione: nelle schede, dove il bottone sta
  nell'intestazione a due righe di distanza, ripeterlo sarebbe rumore;
- **i vuoti filtrati dicono di esserlo.** Non solo gli addebiti e le segnalazioni, che il
  piano nominava: contando è uscito che anche bollette, contratti e immobili hanno filtri
  che il vuoto ignorava, e con `?stato=` dicevano «nessuna bolletta registrata» a chi ne
  ha. Ora la coppia `state.filteredTitle`/`state.filteredHint` vale per ogni elenco
  filtrabile, e il gesto sparisce insieme alla spiegazione: chi filtra conosce la sezione;
- **la pagina `/guida`**, `features/guide/GuidePage.tsx`, nella forma di `PrivacyPage`:
  una pagina, due versioni scelte da `useIsStaff()`, il testo in `it.ts` come
  `guide.staff.sections` e `guide.tenant.sections`. Racconta il ciclo — un immobile, un
  contratto, un invito, gli addebiti, gli incassi — e l'altro lato per chi abita. Le
  sezioni si chiamano con `nav.*` e non con una copia: `nav` è diventato una costante
  fuori dal catalogo, e un test lo verifica. Si apre dal menù dell'account («Come si
  usa», `nav.guide`) e dalla schermata senza organizzazione;
- **il riepilogo vuoto è il primo accesso**: tre gesti nell'ordine in cui si fanno —
  registra un immobile, redigi un contratto, invita chi abita — e il rimando alla guida.
  Lo stesso `Empty`, con un `action` che è un `<ol>` di tre. Per chi abita l'equivalente
  è il vuoto degli addebiti, che dice chi li emette e che cosa arriverà, con lo stesso
  rimando;
- **il primo test del riepilogo**, `features/dashboard/DashboardPage.test.tsx`;
- **due stringhe false trovate contando**: `/pagamenti` usava come vuoto dell'elenco
  «Nessun movimento su *questo addebito*», che è della scheda dell'addebito; e la
  scansione di una bolletta diceva a chi abita «la lettura parte quando ne carichi una»,
  cosa che lui non può fare.

### Il piano aveva una premessa sbagliata, ed era nella tabella delle verifiche

«5 usi su 22 con un `hint`, uno solo con un `action`» era falso il giorno in cui è stato
scritto: i `hint` erano **13** e gli `action` **4**, aggiunti fra il 4 e il 7 settembre e
mai contati. La riga si è emendata nella tabella prima di cominciare, com'è la regola. Non
ha cambiato una decisione — i due livelli, gli stati vuoti e la pagina, restano — ma ha
cambiato la natura di `G1`: da «riempire 22 vuoti» a «riempirne 9 e rileggere gli altri
13», ed è nella rilettura che sono uscite le due stringhe false e i tre elenchi filtrati
che il piano non nominava. È la stessa lezione della 7: *«verificato e non dedotto»* vale
solo se si è contato, e un numero nella tabella senza il comando che lo ha prodotto è
un'affermazione.

### Il piano aveva una seconda premessa sbagliata, e questa ha cambiato una riga di progetto

«La rotta dentro il guscio» per `/guida` non poteva funzionare con l'altro requisito
dello stesso passo, il collegamento dalla schermata senza organizzazione: `AppShell` rende
`NoOrganizationScreen` **al posto** dell'`Outlet` per ogni rotta quando le organizzazioni
sono zero, quindi una guida dentro il guscio da lì non si sarebbe mai aperta. La rotta sta
**fuori**, fra le rotte autenticate accanto al guscio, con il proprio confine di
`Suspense` e un rimando a `/` in fondo — che è del ruolo e porta ciascuno dove atterra. La
forma è esattamente quella di `PrivacyPage`, che il piano indicava: una pagina lunga
senza navigazione. L'alternativa — un'eccezione per `/guida` dentro il guscio — avrebbe
reso la barra laterale con zero organizzazioni, che è lo stato che il guscio
deliberatamente non ha.

### Decisioni prese eseguendo

- **Il vuoto di un filtro non offre il gesto.** «Registra una bolletta» sotto «Niente che
  corrisponda ai filtri» direbbe che la via è registrarne una, mentre la via è togliere il
  filtro. Il gesto resta nell'intestazione della pagina, che non cambia;
- **le segnalazioni partono filtrate**, con «Solo quelle aperte» spuntato, quindi il primo
  schermo di chi non ne ha mai aperta una è il vuoto del filtro e non quello della sezione.
  Si è seguita la trappola del piano — il testo dice dove stanno le altre — e il gesto
  «Segnala un guasto» resta come `action` per entrambi i ruoli, perché è anche il primo
  schermo; la sezione la spiega il sottotitolo, che c'è già;
- **`nav` fuori dal catalogo.** `it` è un oggetto letterale e non può riferirsi a sé:
  perché `guide.*.sections[].title` sia `nav.dashboard` e non `'Riepilogo'`, le voci
  stanno in una costante sopra, e `it.nav` la include. Il test «ogni sezione si chiama
  come una voce della navigazione» confronta i valori e non i riferimenti — una copia
  identica passa, una copia divergente no — ed è quello che serve: il difetto è la
  divergenza, non la copia;
- **«in alto a destra» non esisteva.** La prima stesura della guida diceva che il menù
  dell'account sta in alto a destra; sullo schermo largo sta in fondo alla barra laterale,
  in alto solo dal telefono. Visto con gli occhi nel giro, non dai test: una stringa che
  descrive l'interfaccia è verificabile solo guardandola;
- **l'invito lo si nomina come lo chiama la pagina di accesso**: «Registrati», che porta a
  `/invito`; dentro sessione la voce del menù è «Ho un codice d'invito». La guida usa le
  due parole nei due posti, e non un nome di comodo;
- **il riepilogo non ha un ramo sul ruolo**, e il criterio di fine del piano — «il gesto
  assente per chi non è staff» — lo presupponeva. È la **rotta** a essere dello staff
  (`App.tsx`: per chi abita `/` rimanda agli addebiti), e quel confine lo prova già il test
  del guscio. Il test nuovo prova ciò che il riepilogo mostra a chi ci arriva: i tre gesti
  nell'ordine, i tre indirizzi, il rimando. Il gesto assente per chi abita è provato sugli
  addebiti, che sono il suo primo schermo.

### Verificato al contrario

Ogni test nuovo è stato rimesso sul difetto e guardato cadere:

- il gesto offerto a tutti sugli addebiti (`isStaff &&` tolto dall'`action`): il test
  dell'inquilino cade con `expected document not to contain element, found <a`;
- il vuoto filtrato che spiega la sezione (ramo del filtro tolto dal `hint`):
  `Unable to find an element with the text: /i filtri sono attivi/i`;
- il ruolo scambiato nella guida (`isStaff ? t.guide.tenant : t.guide.staff`): tre test
  su quattro cadono, `Unable to find role="heading" and name "Immobili"` per lo staff e
  `… name "Spese"` per chi abita e per chi non ha ruolo;
- un titolo copiato che diverge (`title: 'Spese di casa'` al posto di `nav.expenses`):
  `expected [ 'Riepilogo', 'Immobili', …(11) ] to include 'Spese di casa'`;
- i passi del riepilogo in ordine sbagliato (secondo e terzo scambiati):
  `expected [ 'Registra un immobile', …(2) ] to deeply equal […]`; e il rimando alla
  guida tolto: `Unable to find an accessible element with the role "link" and name
  "Leggi come si usa"`.

Un'asserzione negativa ha avuto bisogno di un'attesa esplicita: il ruolo arriva con il
rinnovo, e il vuoto può rendere un istante **prima** — letto in quell'istante, «il gesto
non c'è per chi abita» passerebbe anche se il gesto fosse offerto a tutti. I test
aspettano il rinnovo prima di negare, e il test dello staff è stato il primo a cadere per
questa ragione, con la versione di chi abita sotto gli occhi.

### Che cosa è stato provato a mano

Su porte a parte — Postgres su `55432`, API su `8090`, client su `5174` — con un invito
da proprietario emesso dallo script: `/invito` da fuori sessione, il codice, il form
senza email, il `409` che fa comparire il nome dell'organizzazione, l'ingresso **sul
riepilogo vuoto con i tre gesti** e il rimando; la guida nella versione di chi
amministra, aperta dal rimando e dal menù dell'account, con «Come si usa» accanto a «Il
mio account»; i vuoti di addebiti, pagamenti, segnalazioni e bollette per lo staff; una
bolletta con `?stato=` che dice «Niente che corrisponda ai filtri». Poi, da un immobile,
un contratto e un invito costruiti dall'API, l'ingresso di un'inquilina **sugli addebiti
vuoti** che dicono chi li emette, la guida nella versione di chi abita, i vuoti di spese,
bollette e pagamenti, la scheda del contratto con gli addebiti e gli allegati vuoti, e il
rimando «Torna all'applicazione» che porta agli addebiti. Nessun errore in console oltre
ai `401` attesi. Non visto con gli occhi: il collegamento alla guida dalla schermata senza
organizzazione, perché nessun account di prova poteva uscire dalla propria — è un `Link`
identico a quelli sopra, e il test del guscio rende quella schermata.

**L'Invio nel campo del codice su `/invito` resta da provare con una tastiera vera**: il
browser pilotato manda eventi non fidati, e il poscritto della 8 lo aveva già detto.

### Che cosa resta aperto

- il riepilogo con un immobile e nessun contratto mostra i nove numeri a zero, e i «tre
  passi» non ci sono più: la condizione di vuoto è `properties_count === 0 &&
  active_leases === 0`. La guida e i vuoti di immobili e contratti coprono il passo
  successivo, ma il primo schermo dopo il primo gesto è meno parlante del primo schermo
  assoluto. Si riapre se un giro con una persona esterna lo trova;
- «Bozza» a chi abita (`R35`, dissolta in `R31`) non è stata toccata: la guida dice «è in
  bozza, e lo vedi lo stesso», che è la lettura «va bene così» resa esplicita invece che
  decisa. Una parola diversa è un'altra stringa, quando qualcuno la chiederà.

<a id="piano-sezione-10"></a>

## Piano — Sezione 10: tre chiusure prima del primo deploy

Pianificata il **17 settembre 2026**, subito dopo l'esito della 9 e sotto la regola che
una sezione conclusa pianifica la successiva. Chiude `R50`, `R51` e `R46`; **non** chiude
`R48`, `R49` e `R42`, e il perché di ogni scelta è nel [prossimo passo](09-stato-e-roadmap.md#prossimo-passo-suggerito),
che è dove la decisione sul deploy è scritta. Questa sezione è ciò che deve essere verde
**prima** di eseguire il [runbook](07-operazioni.md#il-primo-deploy--runbook); il runbook
stesso non è un passo di questa sezione, perché non è codice e non ha un test — ha un
criterio di fine, e il suo esito si scrive in [11](11-piani-conclusi.md) quando è fatto.

**Si esegue in una sola conversazione**, come la 9. Ogni test nuovo provato al contrario
prima di consegnarlo, con l'assert caduto scritto nell'esito.

### Che cosa esiste davvero, verificato e non dedotto

| Premessa | Verifica |
|---|---|
| Le sonde stanno alla radice del router, e ogni altra rotta porta il prefisso per intero | vero: `crates/api/src/routes/health.rs` monta `/health` e `/ready`; le altre rotte scrivono `"/api/v1/…"` per esteso (`routes/me.rs:31`), non c'è un `nest("/api/v1")` in `crates/api/src/lib.rs` — quindi aggiungere le due rotte con il prefisso è **due `.route()` in più nello stesso `router()`**, non uno spostamento |
| Il proxy inoltra solo `/api/v1/*` | vero: `ops/web/Caddyfile:18`, `handle /api/v1/*`; tutto il resto va al file server, che risponde `index.html` con `200` a qualunque percorso |
| Il runbook prova `…/api/v1/ready` | vero: [07](07-operazioni.md#il-primo-deploy--runbook), passo 4.1, riga 508 |
| Il ripristino della sessione tratta la rete caduta come «nessuna sessione» | vero: `apps/web/src/app/session.tsx:117`, `.catch(() => undefined)` sul `store.refresh()` dell'effetto di ripristino; `App.tsx:131` mostra `Loading` finché `restoring`, poi le rotte da fuori sessione se `session` è nullo |
| Il fallimento per rete è già distinguibile da un `401` | vero: `store.refresh()` in `apps/web/src/api/session.ts:369` lancia `offlineError()` **senza** cancellare la sessione, e solo su `!response.ok` fa `this.clear()`; `ApiError.isOffline` è in `api/problem.ts:67`, e il guscio lo usa già per il terzo stato (`shell.tsx:164`, `ErrorState` con `onRetry`) |
| Il ramo dell'inquilino delle due funzioni di accesso non guarda la membership | vero: `migrations/0014_rls.sql:53` (`can_access_lease`) e `:72` (`can_access_property`) passano da `lease_parties.user_id = app.current_user_id()` senza `is_org_member`; **nessuna migration successiva le ridefinisce** (`grep -c 'FUNCTION app.can_access_lease' migrations/*.sql` dà 1, in `0014`) — quindi la migration nuova le riscrive per intero con `CREATE OR REPLACE` |
| `is_org_member` esige una membership attiva | vero: `0014_rls.sql:28`, passa da `member_role`, che filtra `status = 'active'` e `deleted_at IS NULL` (`:17`); `LeaveOrganization` (sezione 6) tocca `org_members` e non `lease_parties` |
| Un test fa un login «di comodo» per non vedere il difetto | vero: `crates/api/tests/http_contract.rs:2719`, dentro `a_tenant_leaves_an_organization_unless_a_lease_is_in_force_and_can_come_back`, con il commento che lo dichiara; la voce `R46` dice di toglierlo quando si chiude |
| I test Rust sono 478, quelli del client 138 | vero, esito della 9 |

### Decomposizione

##### `D1` — `R50`: le sonde anche sotto `/api/v1`

In `routes/health.rs`, le stesse due funzioni montate anche su `/api/v1/health` e
`/api/v1/ready`; le rotte alla radice **restano**, perché l'`healthcheck` del container
parla all'API direttamente e non passa dal proxy. La riga della tabella delle sonde in
[07](07-operazioni.md) dice che le due forme sono equivalenti e perché esistono entrambe.

Alternativa scartata: un `handle /ready` nel Caddyfile. Sposta la conoscenza delle sonde
nel proxy, che oggi conosce solo il prefisso, e lascia `…/api/v1/ready` — la forma che il
runbook usa — a rispondere `404`.

**Criterio di fine:** un test in `crates/api/tests/` che `GET /api/v1/ready` risponde come
`GET /ready` (stesso stato, stesso corpo), provato al contrario togliendo la rotta nuova
(`404 endpoint.not_found`).

##### `D2` — `R51`: un `refresh` fallito per rete non è «nessuna sessione»

In `session.tsx`, l'effetto di ripristino distingue: `ApiError` con `isOffline` diventa
uno stato `restoreError` esposto dal contesto, tutto il resto resta «nessuna sessione»
com'è. In `App.tsx`, prima del ramo `!session`: se `restoreError`, un `ErrorState` a
schermo intero con `onRetry` che rilancia il ripristino — la stessa forma del terzo stato
del guscio. Niente «Esci»: senza rete un logout non arriva al server, e un bottone che
fallisce è peggio di uno che manca.

Trappola: il `finally` che mette `restoring` a `false` **deve restare** anche sul ramo
dell'errore, altrimenti la riprova non ha un caricamento da mostrare e lo schermo salta
dall'errore al contenuto. E la riprova rimette `restoring` a `true` prima di rilanciare.

**Criterio di fine:** un test che rende `App` (o `SessionProvider` con una sonda, se `App`
costa troppo) con la `fetch` di `/auth/refresh` che lancia `TypeError('Failed to fetch')`:
il form di accesso **non** compare, compare l'errore con «Riprova»; poi la rete torna, un
clic su «Riprova», e la sessione c'è. Al contrario, rimettendo `.catch(() => undefined)`:
il form di accesso compare.

##### `D3` — `R46`: la membership nel ramo dell'inquilino, al livello della RLS

Migration `0041_tenant_access_requires_membership.sql`: `CREATE OR REPLACE` di
`app.can_access_lease` e `app.can_access_property`, identiche a `0014` salvo
`AND app.is_org_member(l.org_id)` nel ramo `EXISTS` dell'inquilino (in
`can_access_property` l'organizzazione è `p.org_id`, e conviene usarla). Le funzioni sono
`SECURITY DEFINER` e `is_org_member` anche: il costo è una lettura in più di
`org_members` per riga, sull'indice che `member_role` usa già.

Alternativa scartata: rileggere la membership in `Authenticated` a ogni richiesta. Cambia
la forma del baratto di [ADR-012](08-decisioni.md#adr-012) — il JWT non revocabile — per
tutte le rotte, staff compreso, dove il [poscritto della 6](11-piani-conclusi.md#poscritto-sezione-6)
mostra che le policy passano già da `member_role` e restituiscono vuoto. Il difetto è
solo nel ramo dell'inquilino, e si chiude lì.

Le invarianti dello schema (`ops/sql/checks/schema_invariants.sql`) non guardano il corpo
delle funzioni: nessun controllo da aggiornare, ma `./ops/ci/check-schema.sh` va comunque
eseguito perché è la CI a dirlo.

**Criterio di fine:** il login «di comodo» esce dal test, e il test afferma con il **token
vecchio** che `GET /leases` non porta più il contratto disdetto e che aprire una
segnalazione nell'organizzazione lasciata è rifiutato — con quale codice lo dice
l'esecuzione (la policy di inserimento nega, e il bordo HTTP lo traduce come già traduce
un immobile non accessibile), e l'esito lo scrive. Al contrario, senza la migration: il
contratto torna nel corpo, che è la riga di `R46`.

##### `D4` — L'esito, le tre voci fuori da [12](12-da-fare.md), e il runbook

L'esito in [11](11-piani-conclusi.md); `R50`, `R51` e `R46` escono; la riga di `R42` in
[04](04-sicurezza.md), se la 6 l'ha lasciata, resta com'è perché `R42` non si tocca. Poi il
runbook, com'è scritto, e il suo esito come quello di una sezione: **le osservazioni si
scrivono mentre lo si percorre**, non alla fine — è la mancanza che la 5-bis dichiara di sé.

### Trappole

**Cercare un posto «sotto `/api/v1`» dove montarle.** Non esiste: l'autenticazione è un
estrattore per handler (`Authenticated`), non uno strato sul prefisso, e il prefisso è
scritto per esteso in ogni rotta. Le due rotte nuove stanno nello stesso `router()` di
`health.rs`, senza estrattore, come le due che ci sono; una sonda che chieda un token non è
una sonda.

**Un `ErrorState` senza `restoring`.** Vedi `D2`: lo stato di caricamento della riprova
è la parte che si dimentica, e senza quella la riprova sembra non fare niente.

**Toccare `member_role` invece delle due funzioni.** È usata da tutte le policy dello
staff, e un cambiamento lì cambia tutto; il difetto sta nel ramo che **non** la usa.

**Dichiarare il codice HTTP del rifiuto prima di vederlo.** Il bordo traduce un `EXISTS`
falso in modi diversi a seconda della rotta (`404` per una risorsa che «non è tua», `403`
per un'azione negata): il test afferma ciò che l'esecuzione mostra, e l'esito lo scrive.

### Che cosa **non** entra

- **`R48`, `R49`, `R42`**: lasciate con la ragione nel [prossimo passo](09-stato-e-roadmap.md#prossimo-passo-suggerito);
- **il deploy stesso**: non è codice e non ha test; si esegue dopo, dal runbook;
- **il «Ricordami»**: è `R42`, ADR prima del codice.

<a id="esito-sezione-10"></a>

## Esito — Sezione 10: tre chiusure prima del primo deploy

Eseguita il **18 settembre 2026**, in una conversazione sola e senza agenti, com'era
scritto. Chiude `R50`, `R51` e `R46`. I test Rust passano da 478 a **479**, quelli del
client da 138 a **141**. `just ci` verde, zero `SALTATO`. Con lei è verde ciò che doveva
precedere il [runbook](07-operazioni.md#il-primo-deploy--runbook): il passo dopo è il
merge della PR che porta le sezioni 1-10 in `dev`, e poi il deploy.

### Che cosa è entrato

- **`D1` — le sonde anche sotto `/api/v1`** (`R50`): due `.route()` in più nello stesso
  `router()` di `routes/health.rs`, stessi handler, nessun estrattore. Le rotte alla
  radice restano per l'`healthcheck` del container. La tabella delle sonde in
  [07](07-operazioni.md#sonde) e quella degli endpoint in [05](05-api.md) dicono che le
  due forme sono equivalenti e perché esistono entrambe. Un test,
  `the_probes_answer_the_same_at_the_root_and_under_the_prefix`, confronta stato e corpo
  per tutte e due le sonde, e afferma il `200` a parte — due `404` sarebbero «uguali»
  anche loro;
- **`D2` — un `refresh` fallito per rete al caricamento non è «nessuna sessione»**
  (`R51`): `SessionProvider` espone `restoreError` e `retryRestore`; `App` rende un
  `ErrorState` a schermo intero con «Riprova» **prima** del ramo `!session`, senza
  «Esci». La riprova rimette `restoring` a `true` prima di rilanciare, com'era la
  trappola del piano. **Il ramo copre anche i `5xx`**, e non era nel piano: vedi sotto.
  Tre stringhe nuove in `it.ts` — `http.502`, `http.503`, `http.504` — perché un corpo
  che non è JSON prende `http.<stato>` come codice e «non è stato possibile completare
  l'operazione» non direbbe che è passeggero. Tre test in `app/App.test.tsx`, che
  rendono `App` intera: la rete che lancia `TypeError`, il proxy che risponde `502`, e
  il `401` che resta «nessuna sessione»;
- **`D3` — la membership nel ramo dell'inquilino** (`R46`): migration
  `0041_tenant_access_requires_membership.sql`, `CREATE OR REPLACE` di
  `app.can_access_lease` e `app.can_access_property` identiche alla `0014` salvo
  `app.is_org_member(l.org_id)` e `app.is_org_member(p.org_id)` nel ramo dell'inquilino.
  `member_role` non è toccata. Il login «di comodo» è uscito dal test, che afferma con il
  **token vecchio**: `GET /leases` risponde `200` con un elenco che non porta più il
  contratto disdetto, e aprire una segnalazione nell'organizzazione lasciata risponde
  **`404 property.not_found`** — l'immobile non si legge, e il bordo lo traduce come ogni
  risorsa che «non è tua». La decisione e l'alternativa scartata sono
  [ADR-054](08-decisioni.md#adr-054); [03](03-modello-dati.md) e [04](04-sicurezza.md)
  descrivono lo stato nuovo.

### Il piano aveva una premessa incompleta, e il giro a mano l'ha trovata

«Il ripristino della sessione tratta la rete caduta come "nessuna sessione"» era vero,
ma `R51` era stata **vista** «con l'API spenta» nel giro della 8 — e con l'API spenta e
il proxy acceso `fetch` **non lancia**: risponde il proxy, `502 Bad Gateway`, `response.ok`
falso, `sendRefresh` fa `clear()` e lancia un `ApiError` con `status: 502`. Il ramo del
piano guardava solo `isOffline` (`status: 0`), cioè il telefono in galleria che non
raggiunge nemmeno Caddy, e lasciava il `502` a «nessuna sessione»: form di accesso, con
il cookie ancora buono — il sintomo esatto della voce. In produzione il `502` è il caso di
**ogni deploy**: mentre l'API si riavvia, chi ricarica finiva al login, e il login
avrebbe risposto `502` a sua volta.

Visto con gli occhi su porte a parte, prima del codice di D2 e dopo: con il ramo del
piano, il form; con l'emenda, l'errore. **Emendato, con la decisione presa in
conversazione e non da soli**: al ripristino `isOffline || status >= 500` diventa
`restoreError`; un `4xx` — il `401` in testa — resta «nessuna sessione». Il `TypeError`
resta coperto: era giusto, era solo la metà.

### Decisioni prese eseguendo

- **il lancio e la riprova sono due funzioni.** Il lint (`react/set-state-in-effect`)
  rifiuta un `setState` sincrono dentro l'effetto, e ha ragione: al montaggio `restoring`
  è già `true`. `launchRestore` non tocca lo stato e lo chiama l'effetto; `retryRestore`
  rimette davanti il caricamento e poi lancia. Un contatore su `useRef` fa ignorare a un
  tentativo superato — lo smontaggio dello `StrictMode`, una riprova mentre l'altra è in
  volo — la scrittura dello stato;
- **`ErrorState` com'è, senza una stringa dedicata al ripristino.** Il titolo e il
  messaggio sono quelli del terzo stato del guscio; la novità sono le tre stringhe dei
  gateway, che valgono per ogni `ErrorState` e non solo per questo;
- **il documento OpenAPI elenca solo le forme alla radice.** `utoipa::path` lega un
  percorso a un handler, e i due handler sono gli stessi; le due forme sono equivalenti
  per contratto scritto in [05](05-api.md), non per una voce doppia nella specifica;
- **`is_org_member(p.org_id)` in `can_access_property`**, non `l.org_id`: è
  l'organizzazione dell'immobile, che è quella che si sta guardando, ed è la stessa;
- **il rifiuto della segnalazione è `404`, non `403`**, e lo dice l'esecuzione: `OpenTicket`
  legge l'immobile prima di scrivere, la `SELECT` non lo trova, e il bordo risponde
  `property.not_found`. È coerente con il cambio di organizzazione, che non distingue
  «non esiste» da «non ne fai parte». Scritto in ADR-054 come conseguenza.

### Verificato al contrario

Ogni test nuovo è stato rimesso sul difetto e guardato cadere:

- le sonde con il prefisso tolte dal router: `assertion left == right failed:
  /api/v1/health non risponde come /health: {"code":"endpoint.not_found",…,"status":404}`;
- il ramo offline del ripristino tolto (l'errore torna «nessuna sessione»): `Unable to
  find role="alert"` — e il form di accesso sotto;
- la trappola del piano, `setRestoring(true)` tolto dalla riprova: `Unable to find
  role="status"` — dall'errore al contenuto senza un caricamento in mezzo. Il test tiene
  in volo il rinnovo della riprova finché non lo rilascia, ed è l'unico modo di guardare
  lo schermo *durante*; il rilascio aspetta che la `fetch` sia partita, perché fra il
  clic e la richiesta ci sono il lock fra le schede e il singolo volo;
- il `5xx` tolto dal ramo (`isOffline` da solo, com'era il piano): `Unable to find
  role="alert"` sul test del `502`;
- la migration `0041` tolta: `chi è uscito legge ancora il contratto disdetto con il
  token vecchio: {"items":[{…,"effective_status":"terminated",…}]}` — che è la riga di
  `R46`;
- la migration tolta **e** la prima asserzione silenziata, per vedere cadere anche la
  seconda: `assertion left == right failed: corpo: {…,"reference":"TK-000001",…,
  "status":"open",…}` — `201`, la segnalazione aperta nell'organizzazione lasciata.

### Che cosa è stato provato a mano

Su porte a parte — Postgres su `55432` con la `0041` applicata dal migratore e
`check-schema.sh` pulito, API su `8090`, client su `5174`. Le quattro sonde con `curl`,
tutte `200` con lo stesso corpo; `/api/v1/ready` attraverso il proxy del client, `200
ready`. Poi il client con l'API **spenta**: l'errore a schermo intero con «Riprova», il
messaggio del `502`, nessun form e nessun «Esci»; l'API riaccesa, un clic su «Riprova»,
`401` dal rinnovo e il form di accesso — non c'era un cookie. **Non visto con gli
occhi**: «Riprova» con un cookie buono, che porta dentro, perché il giro è stato fatto
senza entrare con un account; lo prova il test, che rende `App` intera. `R46` a mano
non è stato provato: nel mondo del seeder nessun inquilino può uscire, tutti hanno un
contratto in essere, e la prova è il test con le sue due asserzioni cadute.

**L'Invio nel campo del codice su `/invito` resta da provare con una tastiera vera**,
dal poscritto della 8.

### Che cosa resta aperto

- **il rinnovo dentro la sessione, su un `5xx`**: `sendRefresh` fa `clear()` su ogni
  `!response.ok`, quindi un `401` arrivato durante un riavvio dell'API, seguito da un
  rinnovo che risponde `502`, chiude la sessione in memoria — con il cookie ancora buono.
  È lo stesso difetto di `R51` sull'altro lato, e non era nel perimetro: `R52` in
  [12](12-da-fare.md);
- le stringhe `http.5xx` coprono un corpo che **non è JSON**: un `503` dell'API stessa
  (timeout della richiesta) porta un Problem Details con il suo codice, e passa da lì.


<a id="piano-sezione-11"></a>

## Piano — Sezione 11: il client dice il vero

Pianificata il **22 settembre 2026**, con la decisione di chiudere tutto il segnalato prima
del collaudo, ed **eseguita lo stesso giorno**. Il piano resta sotto com'è stato scritto;
ciò che l'esecuzione ha confermato, cambiato e trovato è in
[Esito](#esito-sezione-11). Chiude `R36`, `R26`, `R49`, `R52`, `R41`, `R27`: le sei voci di
[12](12-da-fare.md) che **non chiedono una decisione** — o la chiedono piccola e la si
prende qui — e che un collaudo vedrebbe al primo giro. Solo client e documentazione;
**nessuna migration, nessuna rotta nuova**. È la sezione che libera il campo alle cinque
che seguono, tutte con un ADR davanti.

**Si esegue in una sola conversazione**, senza agenti, come dalla 9. Ogni test nuovo
provato al contrario prima di consegnarlo, con l'asserzione caduta scritta nell'esito.

### Che cosa esiste davvero, verificato e non dedotto

| Premessa | Verifica |
|---|---|
| `ChargeStatusBadge` mostra la coppia «derivato (memorizzato)» ogni volta che i due stati differiscono | vero: `apps/web/src/features/shared/status.tsx:231`, `if (status === effectiveStatus)` una sola volta, poi il derivato con il memorizzato fra parentesi. Il memorizzato può essere solo `issued` o `partially_paid` quando differisce (`Charge::effective_status` diverge solo da `is_outstanding()`) |
| Il filtro degli immobili offre `archived`, che non può corrispondere a niente | vero: `PropertiesPage.tsx:40`, `STATUSES = ['draft', 'available', 'maintenance', 'archived']`, con il commento che lo dichiara come «il terzo di `R26` che questa sezione non chiude». Archiviare scrive `deleted_at` e `list`/`find_by_id` filtrano `deleted_at IS NULL` (`repositories/property.rs:59,76`); la conferma di archiviazione **promette** che sparisca dagli elenchi (`property.archiveExplain`) |
| Il filtro delle bollette offre `paid`, che nessuno scrive | vero: `BillsPage.tsx:42`, `STATUSES` include `'paid'`; `BillStatus::Paid` e `Disputed` in `crates/domain/src/billing/bill.rs:80-83` portano «mai scritto dall'applicazione». La sezione 4 ha dato alla bolletta `paid_at`, non lo stato |
| Il ricordo dell'organizzazione si applica **prima** che l'elenco risponda, e la sessione non sa da dove venga l'organizzazione attiva | vero: `session.tsx:202`, l'effetto chiama `store.selectOrganization(stored.active, …)` appena c'è `userId`; `Session` (`api/session.ts:14`) ha `organizationId` e `role`, nessuna origine. Il guscio aspetta l'elenco solo se `organizationId === null` (`shell.tsx:160`) e corregge il ricordo quando l'elenco arriva (`shell.tsx:474-501`) |
| `sendRefresh` cancella la sessione su **ogni** risposta non `ok` | vero: `api/session.ts:372`, `if (!response.ok) { this.clear(); throw … }`; il ramo del `catch` di rete (`:369`) invece non cancella. `ApiClient.request` (`api/client.ts:75`) fa `await this.session.refresh()` su un `401` di sessione e lascia propagare l'errore |
| Chi è uscito e riprova riceve `membership.already_left` senza la via | vero: `it.ts:1709`, «Eri già uscito da questa organizzazione.»; l'avviso **prima** dell'uscita (`it.ts:304`) dice già «Se qualcuno ti reinvita potrai rientrare» |
| `README.md` e `06` insegnano il primo avvio a mano, e `just setup` fa la stessa cosa | vero: `README.md:18-32` e `06-sviluppo.md:20-31` elencano chiavi, `docker compose`, i `cargo run`; `justfile:40`, `setup: keys env up db install`, e `api`/`worker`/`web` avviano. `06:264` già consiglia `just api` per un errore comune |
| I test sono 479 Rust e 141 del client | vero, esito della 10 e `just ci` su `dev` dopo il merge |

### Decomposizione

#### `S1` — `R36`: il badge di un addebito dice due fatti, non un fatto e un giudizio

In `status.tsx`, `ChargeStatusBadge` diventa asimmetrico: se il memorizzato è
`partially_paid` e il derivato differisce, **due badge** accanto — «Scaduto» e
«Parzialmente pagato» — perché sono due fatti veri insieme e nessuno dei due si
ricostruisce dall'altro; se il memorizzato è `issued`, **solo il derivato**, perché
«(Emesso)» accanto a «Scaduto» è esattamente `R30`. Niente parentesi: la parentesi era la
forma di «giudizio che il fatto smentisce», e qui non c'è un giudizio.

Alternativa scartata: un testo composto («Scaduto, parzialmente pagato»). È una stringa
nuova per ogni coppia, e la coppia possibile è una sola; due badge riusano ciò che c'è e
si leggono anche a colori spenti.

**Criterio di fine:** i test dei badge in `status.test.tsx` — quelli della 5-ter — con due
casi nuovi: `overdue`+`partially_paid` rende due badge e nessuna parentesi;
`overdue`+`issued` rende un badge solo. Al contrario, rimettendo la parentesi: il primo
cade su «expected not to contain "("», il secondo su un secondo elemento trovato.

#### `S2` — `R26`: un filtro offre solo ciò che può comparire

`archived` esce da `STATUSES` degli immobili, `paid` da quello delle bollette, e il
commento di `PropertiesPage` smette di dire che il terzo di `R26` è aperto. Il `<Select>`
degli immobili resta con tre valori — bozza, disponibile, in manutenzione — e la domanda
«gli affittati» resta al filtro dell'occupazione accanto, com'è già.

**Non** si aggiunge un «includi archiviati»: la conferma di archiviazione promette che
l'immobile *sparisce dagli elenchi*, e la traccia resta nelle righe che lo citano come
«Immobile archiviato». Rivederlo è una capacità nuova — un elenco degli archiviati con la
sua rotta, perché `list` filtra `deleted_at` — e non un filtro da sistemare; se un giorno
qualcuno la chiede, è una voce in `12`, non questa.

**Criterio di fine:** un test per elenco che le opzioni del filtro siano esattamente
quelle e nessun'altra — è un test sul catalogo delle opzioni, non sulla rete. Al contrario:
`archived` rimesso, l'assert `toEqual([...])` cade con l'elemento in più.

#### `S3` — `R49`: il ricordo non vale finché l'elenco non lo conferma

La sessione impara **da dove** viene l'organizzazione attiva. In `session.tsx` il
contesto espone `organizationRemembered: boolean`: vero da quando l'effetto applica il
ricordo del `localStorage` a quando l'elenco lo conferma o lo corregge; falso quando
l'organizzazione è quella del token o è stata scelta a mano (il selettore, l'accettazione
di un invito). È stato del **provider**, non dello store: lo store non sa che cosa sia il
`localStorage`, e non deve saperlo.

In `shell.tsx`, il gate su `<Outlet/>` si allarga: oggi aspetta l'elenco se
`organizationId === null`; ora aspetta anche se `organizationRemembered && organizations.isPending`.
È **la stessa attesa**, estesa al caso in cui l'organizzazione è ricordata — e la
correzione che il guscio fa già quando l'elenco arriva (`:474-501`) resta com'è, e mette
`organizationRemembered` a falso perché passa da `selectOrganization`/`clearOrganization`.

Trappola: l'effetto che applica il ricordo gira **prima** del render del guscio, quindi
la prima richiesta della schermata partirebbe con l'`X-Org-Id` ricordato **nello stesso
giro** in cui il gate non l'ha ancora visto; il gate deve leggere il flag dal contesto,
non da uno stato locale del guscio aggiornato in un effetto — altrimenti è un giro in
ritardo, che è tutto il difetto.

**Criterio di fine:** in `shell.test.tsx`, il token porta `A`, il `localStorage` ricorda
`B` (lasciata da un altro browser), l'elenco è tenuto in volo: **nessuna richiesta con
`x-org-id: B` parte**; poi l'elenco risponde senza `B`, e il guscio corregge su `A`. È la
stessa forma del test «l'elenco vero corregge il ricordo», con l'asserzione in più sulle
richieste **prima** dell'elenco. Al contrario, senza il gate allargato: `orgHeaders`
contiene `B`.

#### `S4` — `R52`: un `5xx` al rinnovo non è un rifiuto

In `api/session.ts`, `sendRefresh` fa `this.clear()` **solo su `401`**: è la risposta
«questa sessione non vale più», ed è l'unica. Ogni altro `!response.ok` — `502` dal proxy,
`503` per timeout, `500` — lancia `apiErrorFrom(response)` **senza** cancellare: la
sessione in memoria resta, il cookie anche, e la richiesta che aveva chiesto il rinnovo
fallisce con quell'errore. È la schermata a mostrare «Riprova» (i suoi tre stati lo fanno
già), non il guscio a uscire.

Simmetrico a ciò che la 10 ha fatto al ripristino: là il `5xx` è diventato uno stato
d'errore invece di «nessuna sessione»; qui diventa un errore della richiesta invece di un
logout. `ApiClient.request` non cambia: lascia già propagare l'errore del rinnovo.

Da decidere eseguendo, e da scrivere nell'esito: che cosa fare del `403` e del `429` al
rinnovo. La forma proposta è che **solo il `401`** cancelli — un `429` è «riprova fra
poco», e un `403` sul refresh non ha oggi nessuna via che lo produca.

**Criterio di fine:** in `session.test.ts`, accanto al singolo volo: un rinnovo che
risponde `502` lancia, la sessione **resta**, e un rinnovo successivo riesce senza un nuovo
login. Al contrario, con `clear()` su ogni `!ok`: `store.current` è `null` dopo il `502`.

#### `S5` — `R41`: chi è uscito e ci ripensa legge la via

Il minimo è una stringa: `membership.already_left` dice anche la via — «Eri già uscito da
questa organizzazione. Per rientrare serve un nuovo invito da chi la amministra.» — perché
è l'unico messaggio che chi ci ripensa incontra. Il secondo posto va deciso **guardando la
schermata** con lo stack in piedi, com'è scritto nella voce: la schermata «nessuna
organizzazione» offre già la guida e «Ho un codice d'invito», e la guida di chi abita ha
una sezione sull'uscita dove una frase sul rientro potrebbe stare. La decisione — quale
dei due, o entrambi — si prende nell'esecuzione e si scrive nell'esito.

**Criterio di fine:** il test della stringa non ha senso da solo; conta il test della
guida che esiste già (le sezioni si chiamano come la navigazione), che deve restare verde
se la frase entra là.

#### `S6` — `R27`: il primo avvio si insegna con un comando

`just setup` diventa **la via raccomandata** in `README.md` (in inglese) e in
[06](06-sviluppo.md): un comando per preparare — chiavi, `.env`, container, ruoli,
migration, `npm ci` — e `just api`, `just worker`, `just web` per avviare. La sequenza a
mano **resta**, in `06`, sotto un titolo che dice che cosa `just setup` fa, per chi non ha
`just` o vuole capire; e `CLAUDE.md` §4 non si tocca, perché è il riferimento e non un
tutorial. È la decisione che la voce lasciava aperta, e la ragione è quella della voce: la
pagina che un estraneo legge per prima dev'essere la più corta da sbagliare.

Trappola: `crates/api/tests/docs_config_reference.rs` deriva le impostazioni da
`AppConfig` e le cerca in `07`, non in `06` né nel README; ma un comando citato nel README
che non esiste nel `justfile` non lo trova nessun test. Verificare a mano che ogni ricetta
nominata esista (`just --list`).

**Criterio di fine:** non c'è un test; c'è un giro: da un clone pulito, `just setup` e i
tre avvii, seguendo il README e basta.

### Trappole

**Un badge in più che «semplifica» `S1`.** La coppia `overdue`+`partially_paid` è la
ragione per cui la 5-ter non ha accorpato `R36` a `R30`: chi vede due badge e ne toglie
uno per ordine reintroduce «Scaduto» che nasconde «parzialmente pagato». I due test di
`S1` esistono per questo.

**Sistemare `R49` nello store invece che nel provider.** Lo store è puro: sessione, token,
rinnovo. Il `localStorage` è del provider, e l'origine dell'organizzazione anche. Metterla
nello store porterebbe il browser dentro un oggetto che i test costruiscono senza browser.

**Il gate di `R49` letto in ritardo.** Vedi `S3`: il flag va dal contesto al gate nello
stesso render, non attraverso un effetto.

**Cancellare la sessione «per sicurezza» su un `5xx` (`S4`).** Sembra prudente e non lo
è: la sessione è nel cookie, non in memoria; cancellare la memoria non revoca niente e
manda al login chi ha un cookie buono. La prudenza vera è il `401`, che è l'unico
messaggio del server sul valore della sessione.

**Decidere la frase di `R41` senza guardare.** La voce lo dice e vale ancora: una
stringa che descrive l'interfaccia è verificabile solo guardandola (la 9 ne ha trovata
una falsa così).

### Che cosa **non** entra

- un elenco degli immobili archiviati: capacità nuova, vedi `S2`;
- il `403`/`429` al rinnovo oltre la forma proposta in `S4`: si decide eseguendo;
- `CLAUDE.md` §4: resta il riferimento dei comandi, non cambia con `S6`;
- qualunque cosa delle sezioni 12-16.


<a id="esito-sezione-11"></a>

## Esito — Sezione 11: il client dice il vero

Eseguita il **22 settembre 2026**, in una conversazione sola e senza agenti, com'era
scritto. Chiude `R36`, `R26`, `R49`, `R52`, `R41` e `R27`. I test Rust restano **479** —
nessuna riga di backend è cambiata — quelli del client passano da 141 a **147**, in 25
file. `just ci` verde, zero `SALTATO`. **Nessuna migration, nessuna rotta nuova**, nessun
ADR: le decisioni erano tutte piccole, e stanno qui.

### Che cosa è entrato

- **`S1` — il badge asimmetrico** (`R36`): `ChargeStatusBadge` rende il derivato da solo
  quando il memorizzato è `issued`, e **due badge** — «Scaduto» e «Parzialmente pagato» —
  quando è `partially_paid`. Niente parentesi. Il commento in `status.tsx` dice perché la
  coppia non si «semplifica», e i due test in `status.test.tsx` esistono per quello;
- **`S2` — i due filtri** (`R26`): `archived` fuori da `STATUSES` degli immobili, `paid`
  da quello delle bollette. Un test per pagina, `PropertiesPage.test.tsx` e
  `BillsPage.test.tsx`, che legge le `<option>` del `<Select>` e pretende con `toEqual`
  che siano **esattamente** quelle: cade per un'opzione in più come per una in meno.
  Nessun «includi archiviati», con la ragione del piano. **Il commento del verbo
  `ARCHIVE` era falso per la stessa causa**: diceva che un immobile archiviato «resta
  leggibile con la sua storia», e `find_by_id` filtra `deleted_at` come l'elenco — la
  scheda risponde `404`. Riscritto: la traccia resta nelle righe che lo citano, e non di
  più;
- **`S3` — il ricordo non vale finché l'elenco non lo conferma** (`R49`): `SessionProvider`
  espone `organizationRemembered`; il gate del guscio aspetta l'elenco anche quando
  `organizationRemembered && organizations.isPending`; l'effetto di riallineamento in
  `useOrganizations` **conferma passando da `selectOrganization`** quando l'organizzazione
  era ricordata, ed è ciò che fa cadere il flag. Un test in `shell.test.tsx`, con un probe
  `lazy` come le rotte vere — vedi sotto perché;
- **`S4` — `clear()` solo su `401`** (`R52`): in `sendRefresh` il `401` è l'unica risposta
  che chiude la sessione; ogni altro `!response.ok` lancia `apiErrorFrom(response)` con la
  sessione e il cookie intatti, e la richiesta chiamante fallisce con quell'errore. Un test
  in `session.test.ts`: un `502` al rinnovo lascia `store.current`, non chiama
  `onSessionEnded`, e il rinnovo dopo riesce senza login. **Il `403` e il `429`
  cancellano? No**, ed è la forma proposta dal piano, confermata: un `429` è «riprova fra
  poco», e un `403` sul rinnovo non ha oggi nessuna via che lo produca — nessuno dei due è
  un giudizio sul valore della sessione. Scritto nel commento del ramo. **Nessun
  documento descrittivo cambia**: né la 10 né questa hanno messo in un ADR il
  comportamento del rinnovo su `5xx`; l'esito è il registro;
- **`S5` — la via del rientro, in tre posti** (`R41`), decisi **guardando**:
  `membership.already_left` dice la via; la schermata «Nessuna organizzazione» ha una riga
  in più, `org.noneAfterLeaving`; la sezione «Il mio account» della guida di chi abita
  dice che uscire spende il codice. Perché tre e non uno, sotto;
- **`S6` — `just setup` è la via raccomandata** (`R27`): il README (in inglese) e il primo
  avvio di [06](06-sviluppo.md#primo-avvio) sono `just setup` più `just api`, `just
  worker`, `just web`; `just` entra fra i requisiti di 06. La sequenza a mano resta in 06
  sotto «Che cosa fa `just setup`», passo per passo con le ragioni di `--wait` e del
  one-shot fuori dall'attesa, per chi non ha `just` o vuole capire. `CLAUDE.md` §4 non
  cambia. **Il README aveva un errore in più della voce**: avviava `postgres minio
  minio-setup` senza `mailpit`, e l'API senza posta non parte — chi lo seguiva si fermava
  al primo `cargo run`. `just up` include `mailpit`, e il README ora non elenca più i
  container.

### Le premesse, riga per riga

Tutte vere, e due con un'aggravante che il piano non aveva visto: il commento di
`ARCHIVE` in `PropertiesPage.tsx` e il `mailpit` mancante nel README, entrambi chiusi
dentro i passi previsti. Il conteggio dei test Rust — 479 — è stato confermato da
`just ci` alla chiusura, non prima: contarli richiede il database.

### Il piano aveva una trappola sbagliata, e l'esecuzione l'ha corretta

**La trappola di `S3` diceva la cosa giusta per la ragione sbagliata.** «Il gate deve
leggere il flag dal contesto, nello stesso render, non da uno stato locale del guscio
aggiornato in un effetto» è vero; ma la prima forma provata — `organizationRemembered`
come `useState` del provider, scritto nello stesso effetto di `store.selectOrganization`
— era stata scartata per una ragione che si è rivelata falsa: che React rendesse in due
giri l'aggiornamento dello store (`useSyncExternalStore`, corsia sincrona) e il
`setState` (corsia di default). **Verificato con un test a parte, dentro e fuori `act`,
con React 19.2: li rende insieme.** La forma con `useState` non va per un'altra ragione,
che il lint dice da sé: `react/set-state-in-effect`, la stessa regola che la 10 aveva
incontrato con `restoring`. E un `useRef` letto nel render — la seconda forma provata —
cade sotto `react/refs`.

**La forma entrata**: `RememberedOrganization`, un valore con `subscribe`/`snapshot`/`set`
che il provider costruisce con `useMemo` e legge con `useSyncExternalStore`, come la
sessione stessa. L'effetto che applica il ricordo scrive **due sistemi esterni** nello
stesso giro — lo store e questo — e non tocca lo stato di React, che è precisamente ciò
che il suo commento prometteva già. È stato del provider, non dello store, come voleva il
piano: lo store non sa che cosa sia il `localStorage` e continua a non saperlo.

**Il test di `S3` è passato senza il gate, la prima volta.** Il probe della rotta figlia
— copiato da quello della 8 — chiamava `/properties` in un `useEffect` **sincrono**, che
è figlio del provider e quindi gira **prima** dell'effetto che applica il ricordo: partiva
con l'organizzazione del token, cioè con quella giusta, e descriveva un'applicazione che
non esiste. Le rotte vere sono `lazy` in `App`: la schermata arriva da un chunk, dopo, e
trova il ricordo già applicato. Il probe è diventato `lazy(() => Promise.resolve(…))`, e
il test aspetta **il tempo di un chunk** (`act` con un `setTimeout` di 100 ms) prima di
asserire — un'attesa a tempo, perché quando il gate tiene non c'è nessun evento da
aspettare. Con quel probe il test ha riprodotto il difetto — la seconda richiesta portava
l'organizzazione lasciata — e poi il gate l'ha chiuso.

**La conferma passa da `selectOrganization`, e il piano diceva che l'effetto restava
com'era.** Il piano descriveva il flag come vero «a quando l'elenco lo conferma o lo
corregge», e la correzione passava già da `selectOrganization`/`clearOrganization`; la
conferma — l'organizzazione ricordata **è** fra le proprie, con lo stesso ruolo — non
passava da niente, e il flag sarebbe restato vero per tutta la sessione. Innocuo per il
gate, che guarda anche `isPending`, ma falso di nome. L'effetto di riallineamento chiama
`selectOrganization(active.id, active.role)` anche quando `organizationRemembered` è
vero: riscrive lo stesso ricordo una volta, e il flag cade. Emendato nel commento.

### Decisioni prese eseguendo

- **`R41` in tre posti, non due.** Da ADR-053 alla schermata «Nessuna organizzazione» si
  arriva **solo uscendo** — chi entra da un invito è già dentro, chi diventa proprietario
  nasce con la sua — quindi chi la guarda è esattamente chi ci ripensa, e la schermata
  diceva «si entra da un invito» senza dire che il codice di allora è speso. La riga è
  condizionale, «se sei uscito», perché varrà anche per chi verrà tolto (`R39`). La guida
  è dove la regola si legge **prima** di uscire; `membership.already_left` è ciò che una
  scheda vecchia incontra. Nessuna stringa descrive l'interfaccia: nessuna delle tre può
  invecchiare come quella che la 9 aveva trovato;
- **la riga in più della guida non cambia i titoli**, quindi il test che allinea le
  sezioni alla navigazione resta verde senza toccarlo;
- **`S2` tocca solo i cataloghi**: `EDITABLE_STATUSES` del dialogo «Modifica» non cambia,
  perché là `archived` non c'era già.

### Verificato al contrario

Ogni test nuovo rimesso sul difetto e guardato cadere:

- `S1`, la parentesi rimessa: il primo test cade a `status.test.tsx:81`,
  `getByText('Parzialmente pagato')` — `Unable to find an element with the text` — perché
  la parentesi spezza il testo in tre nodi, `(`, il nome, `)`: **un assert prima** di
  quello previsto dal piano (`not.toContain('(')`). Il secondo cade a `:89`,
  `toHaveLength(1)` con tre `span`;
- `S2`, `archived` e `paid` rimessi: i due `toEqual` cadono con `+ "Archiviato"` e
  `+ "Pagata"` come elemento in più;
- `S3`, il gate stretto a `session.organizationId === null` com'era:
  `expected [ null, …(1) ] to not include '00000000-0000-4000-8000-0000000000ee'` —
  la seconda richiesta porta l'organizzazione lasciata altrove;
- `S4`, `clear()` su ogni `!ok` com'era: `expected null not to be null` a
  `session.test.ts:298`, `store.current` dopo il `502`.

### Che cosa è stato provato a mano

Su porte a parte — Postgres su `55432` (il database del giro della 10, quarantuno
migration), API su `8090`, client su `5174`. Per `S5` serviva un account **uscito** dalla
sua unica organizzazione: `carla.giro@example.it`, lasciata da un giro precedente con
stato `left` e password ignota — reimpostata con «Password dimenticata», leggendo il
codice da mailpit, cioè con il prodotto e non con una `UPDATE`. Il login l'ha fatto chi
possiede il progetto nel pannello del browser, non l'assistente. Visti con gli occhi: la
schermata «Nessuna organizzazione» prima e dopo la riga nuova, e la guida dal suo link.
Con `curl` e il token di Carla: un secondo `POST /organizations/{id}/leave` risponde
`409 membership.already_left`, che è la stringa cambiata.

Per `S6`, **da un clone pulito** nello scratchpad, seguendo il README e basta:
`just --list`, poi `just keys`, `just env`, `just db` e `just install` — il database
`propflow_s11` creato sul cluster del giro con le quarantuno migration applicate dal
migratore compilato nel clone, `npm ci` a buon fine. **`just up` non è stato eseguito
dal clone**: le porte di `docker-compose.yml` sono fisse, e un secondo progetto compose
avrebbe conteso `9000`, `1025` e `8025` ai container del giro. I tre avvii non sono stati
provati dal clone per la stessa ragione; lo sono stati dal repository, che è ciò che il
giro di `S5` ha usato.

**Non provato con gli occhi**: `S1`, `S2` e `S3`. Il database del giro non ha un addebito
scaduto e parzialmente pagato, e un secondo browser con un ricordo morto richiede due
profili; li provano i test, con le loro asserzioni cadute.

### Trovato eseguendo, e non chiuso

**`R53` — cambiare organizzazione dal selettore non svuota la cache delle query.**
Letto in `app/query.tsx` cercando dove il ricordo di `R49` incontrasse la cache: le chiavi
non portano l'organizzazione, e `QueryProvider` svuota solo al logout. Chi ha due
organizzazioni sceglie la seconda e la schermata resta sui dati della prima. In
[12](12-da-fare.md), con le due vie possibili.

### Che cosa resta aperto

- l'Invio nel campo del codice su `/invito` con una tastiera vera, dal poscritto della 8:
  ancora in sospeso, e resta nel passo 5 del cammino;
- `R53`, sopra: una voce nuova, non una mezza cosa lasciata a questa sezione.


<a id="piano-sezione-12"></a>

## Piano — Sezione 12: l'immobile, con un contratto in essere

Pianificata il **22 settembre 2026**, nello stesso giro dell'esito della
[11](#esito-sezione-11), com'è la regola, ed **eseguita lo stesso giorno**. Il piano
resta sotto com'è stato scritto — anche dove l'esecuzione l'ha smentito, che è la parte
più utile da rileggere; ciò che ha confermato, corretto e trovato è nell'
[esito](#esito-sezione-12). Chiude `R44`, `R45` e
`R29`: tre regole del service layer che le voci hanno già mezze decise — **in bozza non
si affitta, con un contratto in essere non si archivia né si torna in bozza, e il residuo
che chi abita legge non mente**. Sono la stessa domanda — *che cosa vieta, o che cosa
deve dire, un contratto in essere* — posta su tre righe, ed è la ragione per cui stanno
in una sezione sola. **Una migration** (la `0042`, per `R29`), **nessuna rotta nuova**,
un ADR prima del codice di `R29` e una nota a un ADR esistente per le altre due.

**Si esegue in una sola conversazione**, senza agenti. Ogni test nuovo provato al
contrario prima di consegnarlo, con l'asserzione caduta scritta nell'esito. I test con
il database solo con `./ops/ci/test-integration.sh`, che non ha un filtro: le prove al
contrario si combinano in un giro.

### Che cosa esiste davvero, verificato e non dedotto

| Premessa | Verifica |
|---|---|
| Un immobile nasce in `draft` e nessuna regola lo guarda | vero: `crates/domain/src/property/aggregate.rs:200`, `status: PropertyStatus::Draft` in `create`; `set_status` (`:291`) rifiuta solo `archived`; nessun caso d'uso in `crates/application/src/use_cases/{lease,property}/` legge lo stato dell'immobile per decidere |
| Il form di creazione non offre lo stato; il dialogo «Modifica» sì | vero: `CreatePropertyRequest` (`crates/api/src/routes/properties.rs:220`) non ha `status`, `UpdatePropertyRequest` (`:240`) sì; `NewPropertyPage.tsx` non nomina `status`, `PropertyDetailPage.tsx:280-297` ha il `<Select>` con `EDITABLE_STATUSES` |
| `ActivateLease` non legge l'immobile | vero: `crates/application/src/use_cases/lease/activate.rs:41-58` rilegge il contratto e le parti, poi `lease.activate(&parties, now)`; `property_id` compare solo nell'evento |
| `ArchiveProperty` non guarda i contratti | vero: `crates/application/src/use_cases/property/archive.rs:46-51`, `find_by_id`, `property.archive(now)`, `soft_delete` |
| «In essere» ha già una definizione sola, e un modo di chiederla per persona | vero: `app.lease_in_force(status, start_date, end_date)` dalla `0017`, usata da `LeaseRepository::list` con `LeaseQuery { property_id, in_force }` (`repositories/lease.rs:65-81`); `has_in_force_for_user` (`:224-231`) interroga `app.has_lease_in_force` per la sezione 6. **Non esiste** la forma per immobile |
| Una regola cross-aggregato si esprime come `Invariant` sollevato dal caso d'uso, e risponde `409` | vero: `LeaveOrganization` (`use_cases/organization/leave.rs:105-109`) solleva `Invariant::MembershipHasLeaseInForce`, `crates/domain/src/error.rs:456` gli dà il codice `membership.lease_in_force`, `crates/api/src/error.rs:161` traduce ogni `Invariant` in `409 invariant`. È la forma di [ADR-015](08-decisioni.md#adr-015): «almeno un owner» resta applicativo perché richiederebbe un trigger cross-row |
| Il saldo si somma sotto la RLS di chi legge, e i pagamenti non attribuiti sono invisibili a chi abita | vero: `app.v_charge_balances` è `security_invoker = true` (`0015:10`, `0024:71`), `payments_select` è staff **o** `payer_user_id` **o** `payee_user_id` (`0014:299-302`). Chi vede un addebito è una parte del suo contratto o staff (`charges_select`, `0014:291`, via `can_access_lease`) |
| Il commento che promette il contrario è in **due** posti | vero: `use_cases/payment/read.rs:33-35` («il totale incassato resta comunque visibile a tutti nel `paid_amount` del saldo») e `routes/payments.rs:161-163` su `RecordPaymentRequest.payer_user_id` |
| Esiste il precedente di una funzione `SECURITY DEFINER` che proietta poco | vero: `app.property_residents` (`0030:64`), `LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp`, con `REVOKE`/`GRANT` per ruolo (`0030:121-130`); l'invariante 6 di `ops/sql/checks/schema_invariants.sql:77-83` **fallisce** su un DEFINER senza `search_path` |
| `propertyJudgements` in `status.tsx` include `draft`, e un test lo difende | vero: `features/shared/status.tsx:70`, e `status.test.tsx:45` «una bozza che risulta affittata è comunque un'anomalia da vedere» |
| I test sono 479 Rust e 147 del client | vero, esito della 11 e `just ci` |

### Decisioni prese, con il perché

- **`R29`: il totale incassato si somma sotto `SECURITY DEFINER`, e il resto del saldo
  resta com'è.** Delle tre vie della voce, la prima — `payer_user_id` obbligatorio con un
  solo inquilino — non chiude il bonifico anonimo, che `submit.rs` dichiara voluto; la
  terza — non mostrare il totale a chi non vede tutte le righe — toglie a chi abita
  l'unico numero per cui apre la prima schermata, «quanto resta da pagare». La seconda lo
  chiude e costa una funzione: `app.charge_paid_amount(p_charge_id uuid) RETURNS
  app.money_amount`, `SECURITY DEFINER`, che somma i pagamenti nei **due stati che la
  vista già conta** (il predicato della `LATERAL` in `0024:86-95`, da riusare tale e
  quale) senza la RLS di chi chiama; `v_charge_balances` la usa al posto della `LATERAL`.
  **Che cosa rivela**: un aggregato — quanto è stato incassato su un addebito — a chi può
  vedere quell'addebito, cioè a una parte del contratto o allo staff; **non** chi ha
  pagato, né quando. È la stessa forma di [ADR-036](08-decisioni.md#adr-036), che proietta
  tre colonne dei residenti, e va scritta come **ADR-055** prima della migration, perché
  cambia una proprietà che [04](04-sicurezza.md) descrive: una vista contabile che
  legge oltre la policy;
- **`R44`: si rifiuta all'attivazione, non alla bozza del contratto.** Nel modello
  derivato «affittato» è `app.lease_in_force`, che comincia con l'attivazione: una bozza
  di contratto su una bozza di immobile sono due cose che si preparano insieme, e
  rifiutare `DraftLease` obbligherebbe a un ordine di compilazione. `ActivateLease`
  rilegge l'immobile e rifiuta con `Invariant::PropertyNotLettable` →
  `property.not_lettable`, `409`;
- **`R44`, la porta di servizio: non si torna in bozza con un contratto in essere.**
  `UpdateProperty` con `status: draft` su un immobile che ha un contratto in essere
  rifiuta con `Invariant::PropertyHasLeaseInForce` → `property.lease_in_force`. Senza,
  «Affittato (Bozza)» rientra dall'altra parte. `maintenance` resta ammesso: la 5-ter ha
  tenuto apposta «In manutenzione (Affittato)», che è un'anomalia vera da vedere;
- **`R45`: si rifiuta, non si disdice.** Archiviare è un soft delete; disdire un contratto
  è un atto con una data e delle conseguenze — gli addebiti che smettono di maturare, la
  persona che può uscire — e farlo *di nascosto* dentro un'archiviazione sarebbe
  esattamente il genere di effetto che nessuno cerca dove succede. Chi amministra
  disdice, poi archivia. Stesso `Invariant::PropertyHasLeaseInForce` di sopra: è la stessa
  regola, «un immobile con un contratto in essere non cambia natura»;
- **la domanda per immobile è un metodo della porta, non una query di elenco.**
  `LeaseRepository::has_in_force_for_property(property_id)` accanto a
  `has_in_force_for_user`, con `SELECT EXISTS (… WHERE property_id = $1 AND deleted_at IS
  NULL AND app.lease_in_force(status, start_date, end_date))`: **la stessa funzione**
  delle viste e dell'elenco, perché due definizioni di «in essere» farebbero rispondere
  l'archiviazione e la dashboard in modo diverso sullo stesso contratto. L'alternativa —
  `list(LeaseQuery { property_id, in_force: Some(true), limit: 1 })` — funziona e non
  dice che cosa chiede;
- **il form di creazione offre lo stato, con «Disponibile» proposto**, e il dominio non
  cambia default: `CreatePropertyRequest.status` opzionale (`draft`, `available`,
  `maintenance`; `archived` lo rifiuta già `set_status`), `CreatePropertyCommand.status`,
  e il caso d'uso lo applica dopo `Property::create` — leggendo `aggregate.rs:221`
  (`apply(PropertyChanges)`) si decide se passare da lì o da un parametro di `create`.
  `draft` resta scegliibile per chi sta compilando;
- **`propertyJudgements` non cambia, e nemmeno il suo test.** Se la regola regge,
  `rented`+`draft` non nasce più; ma le righe **già** in quello stato — il database del
  giro ne ha — restano, e il badge che le mostra come anomalia dice il vero. Nessun
  backfill nella migration: quale stato mettere a una bozza affittata è una decisione di
  chi amministra, che la prende dal dialogo «Modifica», dove `available` è ammesso.
  L'alternativa — togliere `draft` dai giudizi — farebbe sparire un'anomalia vera;
- **le regole vanno in [ADR-015](08-decisioni.md#adr-015) come nota**, non in un ADR
  nuovo: sono tre righe in più di «non nel database», con la stessa ragione di «almeno
  un owner».

### Decomposizione

#### `D0` — il commento falso, per primo

`use_cases/payment/read.rs:33-35` e `routes/payments.rs:161-163` smettono di promettere
che il totale «resta comunque visibile»: dicono che lo è **dalla `0042`**, per via della
funzione, e prima non lo era. È la cosa da correggere per prima perché è ciò che farebbe
concludere a chi passa di lì che il difetto non esiste — e va scritta anche se `D1`
dovesse slittare.

#### `D1` — `R29`: ADR-055, poi la `0042`

ADR-055 in [08](08-decisioni.md), con il perimetro di ciò che la funzione rivela e a chi.
Poi `migrations/0042_charge_paid_amount_reads_past_rls.sql`: la funzione, `REVOKE ALL ON
FUNCTION … FROM PUBLIC`, `GRANT EXECUTE` ai tre ruoli con lo stesso `DO $$ … EXECUTE
format(…)` della `0030`; `CREATE OR REPLACE VIEW app.v_charge_balances` con le **stesse
colonne nello stesso ordine** e `paid_amount` dalla funzione. [03](03-modello-dati.md) e
[04](04-sicurezza.md) descrivono lo stato nuovo.

**Criterio di fine:** un test di contratto HTTP in `crates/api/tests/http_contract.rs`,
dalla forma di `R29`: il proprietario registra `150.00` su un addebito da `420.00`
**senza** `payer_user_id`; l'inquilino legge lo stesso addebito e trova `paid_amount:
"150.00"`, `outstanding_amount: "270.00"`, e `status` che non contraddice più la riga.
Al contrario, con la `0042` tolta: `paid_amount: "0.00"` e `outstanding_amount:
"420.00"`, che è la voce.

#### `D2` — la domanda per immobile

`has_in_force_for_property` sulla porta (`crates/domain/src/ports/repositories.rs`, accanto
a `:431`), l'implementazione in `repositories/lease.rs`, e il finto repository dei test
applicativi che la conosce. Nessuna migration: `app.lease_in_force` c'è.

**Criterio di fine:** un test di persistenza in `crates/infrastructure/tests/`: un
contratto attivo con le date di oggi risponde vero; disdetto, o attivo con `end_date`
ieri, risponde falso — le date sono la metà della definizione, e un test che provasse
solo lo stato passerebbe anche con `status = 'active'` scritto a mano.

#### `D3` — `R44`: non si attiva su una bozza

`Invariant::PropertyNotLettable` con il codice `property.not_lettable` e il suo test
sul catalogo dei codici (`error::tests` in `crates/api/src/error.rs`). `ActivateLease`
rilegge l'immobile con `uow.properties().find_by_id(lease.property_id())` — se manca è
`not_found`, come per ogni riferimento — e rifiuta su `Draft` **prima** di
`lease.activate`. Nel client: la stringa in `errors` di `it.ts`, e la scheda del contratto
la mostra dove mostra già gli altri rifiuti dell'attivazione. [05](05-api.md) elenca il
codice sulla rotta.

**Criterio di fine:** un test HTTP: contratto in bozza con le parti giuste su un immobile
in bozza, `POST /leases/{id}/activate` → `409 property.not_lettable`; l'immobile messo
`available` con `PATCH`, la stessa chiamata → `200`. Al contrario, senza la lettura
dell'immobile: `200` al primo tentativo.

#### `D4` — `R44` e `R45`: con un contratto in essere non si cambia natura

`Invariant::PropertyHasLeaseInForce` → `property.lease_in_force`. `UpdateProperty`
rifiuta quando `changes.status == Some(Draft)` e `has_in_force_for_property` è vero — e
**solo** allora: un `PATCH` che non tocca lo stato, o lo mette in `maintenance`, non fa
la domanda. `ArchiveProperty` rifiuta quando è vero, prima di `property.archive(now)`.
Nel client: la stringa in `errors`; `property.archiveExplain` dice che con un contratto
in essere non si archivia e che prima si disdice, perché la conferma deve dirlo (era la
condizione della voce); l'archiviazione in blocco **non cambia**, il `409` è già un esito
per riga dalla sezione 7.

**Criterio di fine:** due test HTTP. `PATCH /properties/{id}` con `status: draft` su un
immobile con un contratto attivo → `409 property.lease_in_force`, e con `status:
maintenance` → `200`. `DELETE /properties/{id}` — che è l'archiviazione, `204` — sullo stesso → `409`; il
contratto disdetto, la stessa chiamata → `204` e l'immobile fuori dall'elenco. Al contrario, senza i
due rami: `200` e `204` al primo colpo — che è come è oggi.

#### `D5` — il form di creazione offre lo stato

`CreatePropertyRequest.status` e `CreatePropertyCommand.status`, il caso d'uso che lo
applica; `NewPropertyPage.tsx` con il `<Select>` — gli stessi tre valori di
`EDITABLE_STATUSES`, che diventa condivisa — e `available` preselezionato. Una stringa
di aiuto sotto il campo, in `it.ts`, che dice che «Bozza» è per chi sta ancora
compilando.

**Criterio di fine:** un test HTTP: `POST /properties` senza `status` risponde `draft`
(il default non cambia), con `status: available` risponde `available`, con `status:
archived` risponde `409 property.archive_requires_dedicated_operation`. Un test del
client sul catalogo delle opzioni, come quelli della 11.

#### `D6` — documentazione

La nota ad ADR-015; [03](03-modello-dati.md) sulla `0042` e sulla vista; [04](04-sicurezza.md)
sulla funzione e su ciò che rivela; [05](05-api.md) con i due codici nuovi sulle rotte che
li producono e `status` in `POST /properties`; le voci fuori da [12](12-da-fare.md).

### Trappole

**Cercare «affittato» nello stato memorizzato.** `rented` non è mai in database
([ADR-019](08-decisioni.md#adr-019)): `status = 'rented'` è sempre falso, e una regola
scritta così non rifiuterebbe mai niente e passerebbe ogni test che non attivi davvero un
contratto. La domanda è `app.lease_in_force`, e i test di `D2` esistono per questo.

**`CREATE OR REPLACE VIEW` cambia solo il corpo.** Le colonne devono restare le stesse,
nello stesso ordine e con lo stesso tipo, o PostgreSQL rifiuta; aggiungere o riordinare
significherebbe `DROP` e ricreare, e perdere i `GRANT` sulla vista — e `v_owner_dashboard`
legge da `v_charge_balances`.

**Un DEFINER senza `search_path` fissato non arriva alla CI.** L'invariante 6 dello
schema lo ferma prima; ma un DEFINER con `search_path` e senza `REVOKE … FROM PUBLIC`
passa, ed è eseguibile da chiunque abbia una connessione. La `0030` è il modello, con la
`DO $$` che concede ai soli tre ruoli.

**Rifiutare `DraftLease` invece di `ActivateLease`.** Sembra più prudente e obbliga a un
ordine: prima l'immobile disponibile, poi il contratto. La voce lo scarta, e il piano lo
conferma.

**Disdire dentro l'archiviazione.** Vedi la decisione: un effetto in un posto dove nessuno
lo cerca. Se un giorno servirà un «archivia e disdici», sarà un verbo con la sua
conferma.

**Leggere lo stato dell'immobile dopo `lease.activate`.** L'ordine conta poco per il
database, che è in transazione, e molto per il messaggio: l'invariante delle parti e delle
quote deve restare il primo rifiuto che chi attiva incontra solo se è quello vero;
leggere l'immobile prima evita di dire «le quote non coprono il canone» su un contratto
che comunque non si potrebbe attivare.

**Il finto repository dei test applicativi.** `has_in_force_for_property` va aggiunto
anche là, o il crate non compila; e il suo valore di default deve essere `false`, non
`true`, altrimenti i test esistenti di `ArchiveProperty` — che archiviano senza contratti
— cadono per una ragione che non è loro.

**Il backfill che sembra dovuto.** Una migration che portasse ad `available` le bozze
affittate deciderebbe al posto di chi amministra e nasconderebbe il badge che glielo dice.
Non si fa, e la ragione è scritta nelle decisioni.

### Che cosa **non** entra

- il subentro (`R28`): è la 13, e tocca l'invariante delle quote;
- togliere qualcuno (`R39`) e i contatti (`R43`): la 14, con i loro ADR;
- la cache che non si svuota cambiando organizzazione (`R53`): è la 15, decisa il 22
  settembre 2026 — vedi la [tabella](09-stato-e-roadmap.md#le-sezioni);
- un «archivia e disdici»: verbo nuovo, con la sua conferma, se qualcuno lo chiede;
- uno stato `unavailable` di stanza che blocchi l'attivazione: non è nelle voci, e la
  stanza ha già la sua regola d'occupazione (`app.room_is_occupied`);
- il backfill delle bozze affittate, con la ragione sopra.


<a id="esito-sezione-12"></a>

## Esito — Sezione 12: l'immobile, con un contratto in essere

Eseguita il **22 settembre 2026**, in una conversazione sola e senza agenti, com'era
scritto. Chiude `R44`, `R45` e `R29`. I test Rust passano da 479 a **486** — sei di
contratto HTTP e uno di persistenza — quelli del client da 147 a **150**, in 26 file.
`just ci` verde, zero `SALTATO`. **Una migration** (la `0042`), **nessuna rotta nuova**,
un ADR ([ADR-055](08-decisioni.md#adr-055)) e una nota ad
[ADR-015](08-decisioni.md#adr-015). Il giro a vista del giorno dopo ha aperto `R54`.

### Che cosa è entrato

- **`D0` — i due commenti falsi** (`R29`), per primi: `use_cases/payment/read.rs` e
  `routes/payments.rs` promettevano che «il totale incassato resta comunque visibile a
  tutti nel `paid_amount` del saldo», ed era falso. **Scritti come il vero di allora e
  non come il vero di dopo**, che è l'unico scostamento dalla lettera del piano e sta
  sotto;
- **`D1` — `app.charge_paid_amount`** (`R29`): funzione `SECURITY DEFINER` con
  `SET search_path = app, pg_temp`, `REVOKE ALL … FROM PUBLIC` e `GRANT EXECUTE` ai tre
  ruoli; `v_charge_balances` la chiama al posto della `LATERAL` che sommava in linea, con
  le stesse colonne nello stesso ordine. Il predicato sommato è quello di prima parola per
  parola — `succeeded` e `partially_refunded` — perché spostare la somma non è l'occasione
  per cambiare che cosa si somma. ADR-055 scritto **prima** della migration, com'era
  chiesto;
- **`D2` — `has_in_force_for_property`** sulla porta `LeaseRepository`, con
  `SELECT EXISTS (… AND app.lease_in_force(status, start_date, end_date))`: la stessa
  funzione delle viste e dell'elenco;
- **`D3` — in bozza non si affitta** (`R44`): `Invariant::PropertyNotLettable` →
  `property.not_lettable`, `409`. `ActivateLease` rilegge l'immobile **prima** delle parti;
- **`D4` — con un contratto in essere non si cambia natura** (`R44`, `R45`):
  `Invariant::PropertyHasLeaseInForce` → `property.lease_in_force`. `UpdateProperty`
  rifiuta `status: draft` e **solo** quello; `ArchiveProperty` rifiuta, e non disdice.
  `property.archiveExplain` lo dice nella conferma, che era la condizione della voce;
- **`D5` — lo stato alla creazione** (`R44`): `CreatePropertyRequest.status` e
  `CreatePropertyCommand.status`, opzionali; il caso d'uso li applica da
  `Property::apply`, quindi passano da `set_status` e `archived` è rifiutato come dal
  `PATCH`. Il form propone «Disponibile», `EDITABLE_STATUSES` diventa
  `EDITABLE_PROPERTY_STATUSES` in `features/shared/status.tsx`, condivisa fra creazione e
  dialogo «Modifica»;
- **`D6`** — [03](03-modello-dati.md), [04](04-sicurezza.md), [05](05-api.md), la nota ad
  ADR-015, le tre voci fuori da [12](12-da-fare.md).

### Le premesse, riga per riga, e le due sbagliate

Quindici su diciassette vere e verificate contro il codice prima di toccare un file. Le
due sbagliate erano **la stessa riga**, quella sul precedente `SECURITY DEFINER`:

1. la premessa diceva «con `REVOKE`/`GRANT` per ruolo (`0030:121-130`)». **La `0030` non
   ha nessun `REVOKE`**, e `grep -rn REVOKE migrations/` non trova un solo
   `REVOKE … ON FUNCTION … FROM PUBLIC` in tutto il repository: ci sono solo
   `REVOKE UPDATE`/`INSERT` su tabelle. Righe 121-130 sono il solo blocco `DO $$` dei
   `GRANT`;
2. la trappola diceva «la `DO $$` che concede ai soli **tre** ruoli». Ne concede a
   **due** — `propflow_app` e `propflow_worker`; il terzo, `propflow_migrator`, è l'owner
   della funzione e non ne ha bisogno.

**Il `REVOKE` si è fatto lo stesso**, ed è nuovo in questo repository: è precisamente la
cosa che la `0030` non fa e che una funzione che legge oltre la RLS dovrebbe fare,
perché l'invariante 6 dello schema ferma un `DEFINER` senza `search_path` ma non si
accorge di uno eseguibile da chiunque abbia una connessione. I `GRANT` elencano tutti e
tre i ruoli: quello al migrator è ridondante — è l'owner — ma dopo un
`REVOKE … FROM PUBLIC` scriverlo per esplicito è ciò che rende la migration leggibile
senza sapere chi possiede la funzione. Verificato sul database del giro:
`proacl` porta `propflow_migrator=X`, `propflow_app=X`, `propflow_worker=X` e **niente
`PUBLIC`**.

Scostamenti minori, senza conseguenze: il percorso è
`crates/infrastructure/src/db/repositories/lease.rs` e non `repositories/lease.rs`;
`has_in_force_for_user` comincia a `:218` e non `:224`. Il conteggio dei test Rust — 479 —
confermato da `just ci`, non prima.

### Una trappola del piano non si applicava, e una decisione è stata corretta

**«Il finto repository dei test applicativi» non esiste.** La trappola avvertiva di
aggiungere `has_in_force_for_property` anche là, con default `false`. Cercato: non c'è
alcun doppio in memoria di `LeaseRepository` — l'unico implementatore è quello sqlx, e i
sedici test applicativi sono unitari su `authorization`, `password_policy`, `error` e le
transizioni dei ticket. Niente da fare, e nessun rischio da evitare.

**`D0` è stato scritto al presente, non al futuro.** Il piano voleva che i due commenti
dicessero che il totale è visibile «**dalla `0042`**». Ma `D0` è per costruzione il passo
che **precede** la migration, e un commento che rimandi a una `0042` che non è ancora
nell'albero è falso a sua volta — cioè esattamente il difetto che `D0` corregge. I
commenti dicono quindi il vero di allora: il totale **non** gli è visibile, e perché.
`D1` ha riscritto le stesse righe quando il vero è cambiato. L'intento del piano — «va
scritta anche se `D1` dovesse slittare» — è rispettato alla lettera: se `D1` fosse
slittato, il commento sarebbe rimasto vero.

**E la seconda metà è stata quasi dimenticata.** Con la `0042` scritta e la suite verde,
i due commenti dicevano ancora che il totale non è visibile — di nuovo falsi, nella
direzione opposta. Sono stati corretti al controllo finale prima dei commit, e vale la
pena registrarlo: un commento in due tempi ha **due** occasioni di mentire, e la seconda
non la segnala nessun test.

### Il raggio d'azione che il piano non aveva contato

`ActivateLease` che rifiuta su una bozza **rompe ogni test che attivi un contratto su un
immobile creato dagli helper**, perché gli helper creavano in bozza: ventiquattro
attivazioni in `http_contract.rs` e una decina in `persistence.rs`. I due
`create_property` dei test creano ora un immobile **disponibile** — dopo `D5`, passando
`status` alla creazione, che è una richiesta sola — con il commento che dice perché, e il
default `draft` resta fissato da `full_property_lifecycle`, che crea senza passare
dall'helper.

Un test è cambiato di asserzione e non di senso:
`a_property_is_partially_rented_when_one_of_its_rooms_is` pretendeva
`status == "draft"` dopo l'attivazione, per dire che **lo stato dichiarato non cambia**.
Quella proprietà è intatta; è cambiato il valore di partenza, e ora pretende
`"available"`. Con la regola di `D3` quel test non poteva più esistere nella forma di
prima: descriveva un immobile in bozza e affittato, che è ciò che la sezione vieta.

**E il seeder.** `ops/dev/seed-walkthrough.py` metteva già `available` sulla prima casa
prima di attivare, ma non sulla seconda — quella del vicolo cieco della sezione 3, con
le quote 100/0, che esiste per far vedere un'attivazione rifiutata **per le quote**. Senza
il `PATCH`, da questa sezione in poi il rifiuto sarebbe stato `property.not_lettable`, e
il pezzo di collaudo avrebbe mostrato un'altra cosa. Aggiunto, con la ragione nel
commento.

### Decisioni prese eseguendo

- **La funzione si chiama una volta per riga, non tre.** Le tre colonne di
  `v_charge_balances` usano il pagato — `paid_amount`, `outstanding_amount`, `is_past_due`
  — e scriverci `app.charge_paid_amount(c.id)` tre volte sarebbe stato tre letture di
  `app.payments` dove ne basta una. La `LATERAL` resta, con la chiamata dentro al posto
  della somma: la forma della vista non cambia, e nemmeno il numero di letture;
- **`CreatePropertyRequest.status` è il tipo intero del filo**, `PropertyStatusDto`, e non
  i tre valori scegliibili. È lo stesso tipo del `PATCH`, quindi i due endpoint accettano
  e rifiutano le stesse cose per la stessa ragione — `set_status` — e quali offrirne resta
  una decisione del form;
- **`CreatePropertyInput.status` del client è anch'esso il tipo del filo**: `api/` descrive
  che cosa l'API accetta, e far dipendere il livello dei dati da una costante di
  `features/` avrebbe invertito il verso delle dipendenze;
- **`propertyJudgements` non cambia, e nemmeno il suo test**, come il piano prevedeva: le
  righe già in `rented`+`draft` restano — il database del giro ne ha tre — e il badge che
  le mostra come anomalia dice il vero.

### Verificato al contrario

Ogni test nuovo rimesso sul difetto e guardato cadere, in tre giri combinati della suite
con il database.

**Primo giro**, tre difetti insieme:

- **`D1`**, la vista che torna a sommare in linea sotto la RLS di chi legge:
  `an_unattributed_payment_still_counts_in_the_balance_the_tenant_reads` cade con
  `left: String("0.00") / right: "150.00"`, e il corpo stampato è **la voce parola per
  parola** — `"paid_amount":"0.00"`, `"outstanding_amount":"420.00"`,
  `"status":"partially_paid"`;
- **`D3`**, `ActivateLease` che non rilegge l'immobile:
  `a_draft_property_refuses_to_be_let_until_someone_declares_it_available` cade con
  `left: 200 / right: 409` sul contratto attivato su una bozza;
- **`D3`, l'ordine**: `a_draft_property_is_refused_before_the_parties_are_even_looked_at`
  cade con `left: String("lease.no_tenant_party") / right: "property.not_lettable"`.

**Secondo giro**, il solo `D2` — il primo si era fermato al binario rosso prima di
arrivare alla persistenza — con `app.lease_in_force(…)` sostituito da `status = 'active'`:
`a_property_has_a_lease_in_force_only_while_the_dates_say_so` cade a
`persistence.rs:663`, sull'assert «un contratto attivo scaduto ieri non è in essere, e uno
stato da solo direbbe il contrario». È la trappola del piano, riprodotta.

**Terzo giro**, i rami di `D4` e `D5`:

- `UpdateProperty` senza il ramo sulla bozza:
  `a_property_with_a_lease_in_force_cannot_go_back_to_draft_but_can_go_to_maintenance`
  cade con `left: 200 / right: 409`, e il corpo mostra `"status":"draft"` su un immobile
  affittato;
- `ArchiveProperty` senza il controllo:
  `a_property_with_a_lease_in_force_is_archived_only_after_the_lease_is_terminated` cade
  con `left: 204 / right: 409`;
- `CreateProperty` che sposta il default nel dominio (`command.status.or(Some(Available))`,
  cioè la via che il piano scarta):
  `a_property_can_be_born_available_and_stays_a_draft_when_nobody_says_otherwise` cade con
  `left: String("available") / right: "draft"`, e con lui `full_property_lifecycle`;
- **quarto giro, isolato**, perché l'assert su `archived` non arrivava a girare: `set_status`
  che smette di rifiutare `archived`. Lo stesso test cade con `left: 201 / right: 409`, e
  il corpo porta un immobile `"label":"Nato archiviato"`, `"status":"archived"`.

**I tre test del client**, con due difetti. Tolto il campo dal form, tutti e tre cadono
con `Unable to find a label with the text of: Stato dichiarato`; rimessi due difetti più
fini — `status: 'draft'` come proposta e la tendina che offre anche gli stati derivati —
cadono per il contenuto: `expected 'draft' to be 'available'` e
`expected [ 'Bozza', 'Disponibile', …(2) ] to deeply equal [ … ]` con `+ "Affittato"`.

### Che cosa è stato provato a mano

Su porte a parte — Postgres su `55432`, il database del giro della 10 e della 11, portato
dalla quarantunesima alla **quarantaduesima** migration dal migratore; API su `8090` con
`PROPFLOW__SERVER__BIND`; `.env` non toccato, un file d'ambiente a parte nello
scratchpad. Con `curl` e il token del proprietario del collaudo:

| Che cosa | Esito visto |
|---|---|
| `POST /properties` senza `status` | `draft` — il default non si è spostato |
| `POST /properties` con `available` | `available` |
| `POST /properties` con `archived` | `409 property.archive_requires_dedicated_operation` |
| bozza di contratto su immobile in bozza | **ammessa**, com'era deciso |
| `POST /leases/{id}/activate` su quella bozza | `409 property.not_lettable` |
| immobile messo `available`, stessa chiamata | `200`, contratto `active` |
| `PATCH` `status: draft` con quel contratto in essere | `409 property.lease_in_force` |
| `PATCH` `status: maintenance` sullo stesso | `200` — la domanda si fa solo sulla bozza |
| `DELETE /properties/{id}` sullo stesso | `409 property.lease_in_force` |
| contratto disdetto, stessa `DELETE` | `204` |

**E `R29` sul prodotto vero**, che è la prova per cui la sezione esiste. Registrato con
`POST /charges/{id}/payments` un incasso di `150,00` su un addebito da `450,00`
**senza `payer_user_id`**; l'inquilina Anna, con il proprio token, legge sulla stessa riga
`paid_amount 150.00`, `outstanding_amount 300.00`, `status partially_paid` — coerenti — e
`GET /charges/{id}/payments` le risponde **zero righe**. È il perimetro disegnato:
il totale sì, la riga no.

La funzione, letta dal catalogo del database: `prosecdef = t`,
`proconfig = search_path=app, pg_temp`, e `proacl` con i soli tre ruoli e **nessun
`PUBLIC`**.

### E poi con gli occhi, il giorno dopo

Il giro con `curl` non bastava: il client aveva solo i suoi test. Il 23 settembre, con il
login fatto da chi possiede il progetto — le password non le digita l'assistente — il
percorso è stato rifatto **dall'interfaccia**, su `http://localhost:5174`:

- il form di creazione ha la tendina «Stato dichiarato» con **esattamente** `Bozza`,
  `Disponibile`, `In manutenzione`, `available` preselezionato, e sotto la riga «Bozza è
  per chi sta ancora compilando: un immobile in bozza non si può affittare»;
- creato un immobile scegliendo «Bozza», la scheda lo mostra in bozza: il form onora la
  scelta;
- redatto un contratto su quell'immobile — ammesso, come deciso — e premuto «Attiva»
  **senza nessuna parte contrattuale**, la modale mostra «L'immobile è in bozza:
  dichiaralo disponibile prima di attivare il contratto.» È la decisione sull'ordine dei
  rifiuti vista nel prodotto: il contratto è sbagliato due volte e la risposta manda a
  sistemare quella giusta;
- la conferma dell'archiviazione porta la frase nuova: «Se ha un contratto in essere, va
  prima disdetto»;
- su un immobile con un contratto davvero in essere, la modale mostra «L'immobile ha un
  contratto in essere: disdicilo prima di cambiarne lo stato o archiviarlo.»

### Il giro a vista ha trovato una cosa, e per un'ora è sembrata un difetto della sezione

I primi due tentativi di archiviazione **sono passati**, su un immobile con un contratto
`active` che cominciava quel giorno. Con i test verdi e la stessa prova riuscita con
`curl` il giorno prima, sembrava un difetto dell'implementazione.

Non lo era, e la caccia vale la pena di raccontarla perché ogni passo escludeva una
spiegazione plausibile. Il binario in ascolto era di nove minuti più vecchio di quello su
disco: riavviato, stesso esito. La query eseguita a mano come `propflow_app`, con il
contesto RLS esatto dell'API, rispondeva **`true`** — quindi SQL e policy erano giusti. La
sonda decisiva è stata chiedere all'API la stessa domanda dall'elenco:
`GET /leases?property_id=…` restituiva il contratto `active`, e
`GET /leases?property_id=…&in_force=true` **zero**.

La causa: `app.lease_in_force` confronta con `CURRENT_DATE`, che dipende dal fuso **della
sessione**, e la connessione di `sqlx` nasce con `TimeZone = UTC`. Il giro era l'una di
notte del 23 a Roma, cioè le 23 del 22 in UTC: un contratto dal 23 non era ancora in
essere. Rifatta la prova con un contratto già cominciato in UTC, `PATCH status: draft` e
`DELETE` rispondono entrambi **`409 property.lease_in_force`**, e l'interfaccia mostra il
messaggio.

È diventata la voce **`R54`** in [12](12-da-fare.md): «oggi» del prodotto è il giorno UTC,
e per due ore ogni notte non è il giorno italiano. Non è un difetto introdotto qui — la
funzione è della `0017` e il fuso della connessione è di sempre — e i due livelli sono
**coerenti fra loro**, perché il dominio usa `Utc::now().date_naive()`. Tocca anche
`is_past_due` degli addebiti e `v_property_occupancy`.

**Non provato con gli occhi**: niente. Il giro a vista ha coperto tutto ciò che la
sezione cambia nell'interfaccia.

### Che cosa resta aperto

- l'Invio nel campo del codice su `/invito` con una tastiera vera, dal poscritto della 8:
  ancora in sospeso, e resta nel passo 5 del cammino;
- **`R54`**, sopra: una voce nuova, e l'unica delle nove aperte che non sta in una delle
  sezioni 13-16 — dove metterla è una decisione da prendere.


<a id="archivio-09-al-18-settembre-2026"></a>

## Archivio — «09 — Stato e roadmap» com'era prima della riscrittura del 18 settembre 2026

Il 18 settembre 2026 `09` è stato riscritto per intero: teneva tre piani uno sopra
l'altro — il prossimo passo corrente, la storia dei prossimi passi precedenti, e un
«Piano — Completare il prodotto» dell'8 settembre che chiedeva un collaudo fatto il 9 e
pianificava notifiche fatte il 10 — e la roadmap vera stava in `12`, che dichiara di non
essere un piano. Quello che segue è ciò che è stato tolto, **com'era**, perché altri
documenti lo citano e perché le sue lezioni restano vere anche dove la sua forma non lo
è più. Ogni blocco porta l'ancora con cui era citato.

<a id="storia-del-prossimo-passo"></a>

### La storia del «prossimo passo», sezione per sezione

Quanto segue è la storia del «prossimo passo» com'era prima, sezione per sezione, e resta
perché le sue lezioni sono citate da altri documenti.

**Com'era dopo la 9, prima della 10** — la decisione sul deploy e il criterio con cui
sono state scelte le tre voci:

**La [sezione 10](#piano-sezione-10): tre chiusure, poi il primo deploy.** Con la
[9](#esito-sezione-9), conclusa il 17 settembre 2026, finisce la lista
delle sezioni decise dopo il collaudo. Il passo dopo non è una sezione di prodotto: è la
decisione se fare il [primo deploy](07-operazioni.md#il-primo-deploy--runbook) con i
mezzi che ci sono, e che cosa debba precederlo. **La decisione è sì**, ed è presa qui con
le sue ragioni.

*Perché sì, adesso.* Dalla 5-quater il deploy era «rinviato per mancanza di mezzi e non di
priorità», e i mezzi — una VPS, un dominio, un relay SMTP con SPF, DKIM e DMARC — vanno
procurati da chi possiede il progetto, non costruiti: nessuna sezione li fa arrivare. Nel
frattempo ogni sezione ha reso meno della precedente, e le ultime tre sono state rifinitura
su schermate che nessun estraneo ha ancora visto. Il criterio di fine del runbook — *aprire
il prodotto dal telefono, in 4G* — è l'unica prova che il progetto non può darsi da solo,
ed è la prova che vale di più: la 6, la 7 e la 8 hanno mostrato che i test non bastano e che
un giro a mano rende in un'ora quanto una sezione, e un giro dal telefono di una persona
esterna è il giro che nessuna sezione sostituisce.

*Che cosa precede il deploy, e che cosa no.* Sei voci di [12](12-da-fare.md) erano
candidate; se ne prendono tre, per un criterio solo — **ciò che il primo deploy incontra da
sé, con un proprietario e i suoi inquilini, nel primo mese** — e le altre tre si lasciano
con la ragione scritta:

- **`R50` prima, senza discussione**: `/ready` sta alla radice del router e il proxy
  inoltra solo `/api/v1/*`, quindi il passo 4.1 del runbook — `curl …/api/v1/ready` — non
  può riuscire, ed è il criterio con cui il deploy si dichiara riuscito. Due righe in
  `routes/health.rs`;
- **`R51` prima**: un `refresh` fallito per rete al caricamento manda alla pagina di
  accesso senza dirlo. Sul telefono in 4G — che è il criterio di fine — è il caso normale,
  non quello raro, e la prima persona esterna lo incontrerebbe al primo tunnel. `ApiError.isOffline`
  esiste già; manca il ramo nel ripristino;
- **`R46` prima**: chi esce da un'organizzazione continua a leggerne i dati per i quindici
  minuti del suo access token, e dopo ADR-053 è il caso di **ogni** inquilino che esce. Nel
  primo mese uscire è raro — il contratto in vigore lo vieta — ma è un'esposizione di dati
  nota prima che dentro ci siano i dati di qualcuno, e la via scelta è una migration che
  chiude al livello della RLS: la cosa più economica da fare *prima* e la più cara da
  spiegare *dopo*;
- **`R48` dopo**: chi ha già un account e nessuna organizzazione non può diventare
  proprietario. Nel primo mese c'è un proprietario, che è chi possiede il progetto, e i
  suoi inquilini: il caso non si presenta. È una terza rotta e un ramo nel form, e va
  pianificato quando qualcuno lo chiede;
- **`R49` dopo**: il secondo browser con un'organizzazione lasciata altrove parte con un
  `X-Org-Id` morto, e l'elenco lo corregge da sé. È un lampo, non un vicolo cieco;
- **`R42` dopo**: la finestra scorrevole dei trenta giorni è la forma comune, è documentata,
  e il «Ricordami» è un ADR prima del codice — una decisione sulla sicurezza che non si
  prende di fretta per fare un deploy.

Il piano della 10 è [in 11](#piano-sezione-10), con le premesse verificate contro il
codice. Quando è verde, il runbook si esegue com'è scritto, e **l'esito del deploy si
scrive in [11](11-piani-conclusi.md)** come quello di una sezione: che cosa ha retto, che
cosa no, e il giro dal telefono con le sue osservazioni, riga per riga, mentre lo si fa.

*Che cosa la 9 lascia da portarsi dietro.* Due premesse della sua tabella erano sbagliate —
un conteggio mai fatto e una rotta che non poteva stare dove il piano la metteva — e
nessuna ha cambiato una decisione; una stringa che descriveva l'interfaccia («in alto a
destra») era falsa e solo il giro l'ha vista. **L'Invio nel campo del codice su `/invito`
resta da provare con una tastiera vera**, dal poscritto della 8.

`G0`, il giro a mano
della 8, è stato fatto e ha reso: un difetto bloccante sul percorso principale che 478 + 129
test verdi non vedevano, in un'ora. Il rendimento di un giro resta la misura più alta di
questo progetto — sei voci in mezza giornata contro le quattro di cinque sezioni di
lettura, e ora una pagina bianca dietro tre revisioni — e `mailpit` su `localhost:8025`
copre tutto tranne «arriva fuori dallo spam».

Il punto da percorrere per primo è il **rientro**, che è il cammino più lungo della
sezione 6: uscire da un'organizzazione, farsi reinvitare, accettare. Attraversa due
account e la posta, e i suoi test lo provano a pezzi.

**Quel giro è stato fatto, il 12 settembre, e nei primi dieci minuti ha trovato un
vicolo cieco che una suite da 459 test non vedeva**: chi usciva dall'organizzazione **attiva** restava con
il suo identificatore in `localStorage`, e l'unica richiesta che avrebbe potuto
correggerlo — l'elenco delle organizzazioni — lo dichiarava, quindi veniva rifiutata.
Corretto, con un test che riproduce lo stallo. Il resoconto è nel
[poscritto dell'esito](#poscritto-sezione-6), e la lezione da
portarsi dietro è più generale del difetto: **una richiesta che ripara uno stato non
deve dipendere da quello stato**. Il resto del giro — rientro compreso — non ha
trovato altro nella sezione, e due cose fuori: `R43` e `R44`.

**La sezione 7 è conclusa**, il 12 settembre, e il suo
[esito](#esito-sezione-7) è la seconda prova che pianificare
leggendo il codice regge — con un'avvertenza in più rispetto alla 6: **due premesse
scritte come «verificate» erano dedotte**, e l'esecuzione le ha smentite entrambe
(un secondo archivio risponde `404`, non `409`; la `USING` di `expenses_update` è più
larga di «chi l'ha registrata o lo staff»). Nessuna delle due ha cambiato una
decisione; entrambe hanno cambiato una riga di codice. La lezione è sul metodo:
*«verificato e non dedotto»* vale solo se si è guardato **da dove arriva** la riga, e
non solo che cosa il dominio fa quando ce l'ha.

**La sezione 8 è [conclusa](#esito-sezione-8)** il 16 settembre 2026, e la
**9 è [conclusa](#esito-sezione-9)** il 17: il suo primo passo è stato
il giro a mano della 8. La 8 è stata la prima il cui primo passo non era codice: un ADR. `R9` tocca chi può registrarsi e come, ha una
faccia di sicurezza, e le sue quattro domande hanno risposte che si escludono a vicenda —
scriverne una nel codice prima che sia registrata significherebbe scoprire nel diff che
si era scelto.

**Prima della 8, il giro a mano della 7**: due delle sette schermate sono state viste con
gli occhi e cinque no, e la rete che cade a metà del giro è coperta da un test e non da un
dito. È mezz'ora, non una sezione.

Il deploy era **rinviato in fondo**, per mancanza di mezzi e non di priorità: il
[runbook](07-operazioni.md#il-primo-deploy--runbook) è scritto, gli artefatti sono validi
per quello che si può validare senza una macchina, e servono una VPS, un dominio, un relay
SMTP con SPF, DKIM e DMARC e delle credenziali. **Dalla 9 è il prossimo passo**, con le
tre chiusure della [10](#piano-sezione-10) davanti, e il suo criterio di fine intatto:
*aprire il prodotto dal telefono, in 4G*.

**Che cosa cambia nel criterio d'ordine, e va detto.** Le sezioni 6-9 dovevano venire dopo
il mese con un proprietario vero, e *quale per prima lo avrebbe detto quel mese*. Quel mese
slitta, quindi l'ordine 7 → 8 → 9 resta ma è **scelto senza quella prova**: chi lo esegue
ha un ragionamento e non una misura.

La **sezione 6 è la prova che quel ragionamento può bastare**, e va detto con la stessa
precisione con cui il suo piano aveva dichiarato il contrario: pianificata leggendo il
codice e non usando il prodotto, si era premessa di essere *«un ragionamento, non una
misura»* — e tutte e cinque le trappole che aveva previsto si sono avverate eseguendo.
Quello che non aveva visto non stava nelle trappole ma altrove, ed è nel suo
[esito](#esito-sezione-6). Non rende inutile il giro a mano: rende
credibile pianificare a freddo **ciò che un test può vedere**.

**«Pronto» era per un proprietario che conosci; dalla 8 è per chi ha ricevuto un invito.**
`R9` è chiusa: la registrazione aperta non esiste più, e il perimetro è dato da chi può
emettere quale token — un inquilino da chi lo ospita, un proprietario da chi amministra
la piattaforma. Il deploy non è più tenuto fermo da questo; lo tengono fermo i mezzi.

**Il [collaudo in undici punti](#esito-sezione-5-bis), che è la
5-bis, è concluso** — percorso per intero il 9 e il 10 settembre 2026 da chi non aveva
scritto il codice — e con esso l'[esito della fetta
C](#esito-fetta-c) smette di essere aperto dopo due giorni e cinque
sezioni di attesa. **Nessuno degli undici punti si è fermato**: le sei voci uscite dal
giro sono difetti dentro percorsi che funzionano, il che dice che ciò che manca è
rifinitura e non fondamenta.

Quel giro ha anche fatto vedere per la prima volta **con gli occhi** le sezioni 3, 4 e 5,
che erano state verificate solo con i test. La 5-ter le ha ora rifinite, e resta vera la
cosa che l'esito della 5-bis segnala come propria mancanza: le undici righe sono state
riempite alla fine del giro invece che mentre lo si percorreva, e per quattro punti resta
l'affermazione che sono stati attraversati, senza l'osservazione. **Chi rifarà un
collaudo tenga il documento aperto accanto al browser.**

**La [sezione 6](#esito-sezione-6) è conclusa**: i propri dati si
guardano e si correggono, la password si cambia da dentro dando la vecchia — e chiude
tutte le sessioni, come il recupero, che è la decisione meno ovvia della fetta — e da
un'organizzazione si esce, con il divieto sul contratto in essere garantito da una policy
e non da un `if`. Chi esce **rientra** accettando un invito nuovo, che è il ramo che
[ADR-051](08-decisioni.md#adr-051) aveva previsto per iscritto lasciandolo non scritto.
[Piano](#piano-sezione-6) ·
[esito](#esito-sezione-6). **Nessun ADR**, ed era una conclusione del
piano e non una svista. Due cose da portarsi dietro: le policy permissive si combinano in
**OR**, quindi un divieto scritto solo nella policy nuova sarebbe valso per l'inquilino e
non per chi amministra; e un evento va scritto **prima** della scrittura che gli toglie il
permesso di esistere — `is_org_member` passa da `member_role`, che esige
`status = 'active'`.

**La [sezione 5-ter](#esito-sezione-5-ter) è conclusa**: un inquilino
che torna rientra dalla stessa porta, il badge dice il fatto e non più la coppia su ogni
riga, i collegamenti dicono il nome di dove portano, il riaddebito dice a chi e quanto
resta, e la data di un pagamento si mostra senza perdere l'ora vera.
[Piano](#piano-sezione-5-ter) ·
[esito](#esito-sezione-5-ter). Due cose da portarsi dietro: il piano
aveva una **premessa sbagliata** — l'intestatario non era in `LeaseResponse` — e si è
emendata la premessa invece di rinunciare alla decisione; e `Y3` è costato più del
previsto perché la lezione della sezione 5 — *i nomi su ogni risposta, transizioni
comprese* — vale identica su addebiti e bollette, e ha cambiato il tipo di ritorno di nove
casi d'uso di scrittura.

**La [sezione 5](#esito-sezione-5) è conclusa**: la visibilità di una
segnalazione è una scelta di chi la apre, il testo si corregge finché nessuno l'ha presa in
carico, le risposte dicono **chi** e non solo quando, su una segnalazione chiusa il thread
si legge e non si scrive, e riaprire pretende un motivo che finisce nella conversazione.
[Piano](#piano-sezione-5) ·
[esito](#esito-sezione-5). È la prima sezione che **revoca** una
decisione documentata — [ADR-048](08-decisioni.md#adr-048) emenda
[ADR-026](08-decisioni.md#adr-026) su due colonne — e il modo in cui lo fa, emendando
l'invariante di schema e il test dei privilegi nello stesso passo della migration, è la
parte da ricordare. Ha anche reso **portante** `users_select`: la `0030` aveva scritto che
la finestra per restringerla l'avrebbe chiusa il primo client che leggesse un nome da lì,
ed è questo.

**La [sezione 2](#esito-sezione-2) è conclusa quanto alle due
correzioni**: il punto 10 del collaudo adesso prova ciò che dichiara — vuole due schede
dello stesso account — e con lui il punto 4, che aveva lo stesso difetto e che nessuno
sapeva. Il messaggio della configurazione dice che `.env` lo carica `just`, in tutti e otto
i posti in cui quella frase viveva.

**La [sezione 3](#esito-sezione-3) è conclusa**: il vicolo cieco dei
contratti non esiste più. Una bozza si annulla, una quota sbagliata si corregge — anche a
contratto attivo, che è il caso dei contratti già rotti — una parte inserita per sbaglio si
toglie, e l'attivazione rifiuta una quota a zero ([ADR-046](08-decisioni.md#adr-046)).
Eseguirla ha chiuso anche un buco che nessuno aveva in elenco: si poteva aggiungere una
parte a un contratto **attivo**, e da lì le quote non sommavano più a 100 senza che niente
protestasse.

**La sezione 1 è conclusa** — sei ritocchi di sola interfaccia, nessuna decisione da
prendere: [piano](#piano-sezione-1) ·
[esito](#esito-sezione-1). Il backend non è stato toccato, e
`/accesso` è stata rimisurata: **−76,4 kB (−20,6 gz)** sulla prima schermata che vede un
utente nuovo. Ciò che quel lavoro ha trovato e che il piano non prevedeva — la
navigazione richiedeva **tre** modifiche e non due — sta nell'esito.

Il punto **10** — due finestre aperte insieme che non buttano fuori nessuno — è la prova
di `C2`, e prima di questa fetta era **certo** che fallisse. Il punto **2** e il punto
**4** provano `C4` e la revoca di `C1`, che prima non si potevano fare affatto.

Quello che il giro trova va scritto nell'esito **anche se non si corregge**. Una fetta che
chiude con «tutto bene» dopo un giro come quello è una fetta che il giro non l'ha fatto.

**Poi**, e non prima, la fase [Prima degli utenti veri](#prima-degli-utenti-veri): le
notifiche per prime. Aprire a chi non ha scritto il codice è ciò che cambia il criterio
d'ordine più sotto, ed è il collaudo a dire se si può.

**Le fette A, B, B-bis e C sono concluse quanto al codice.** Il giro del denaro dal lato del proprietario è
chiuso e l'insoluto scende; l'altra metà del prodotto esiste, e l'inquilino entra,
vede che cosa deve, **dichiara di aver pagato**, segnala un guasto e ne segue il
corso — e con la B-bis vede anche l'altra tasca, quella verso i coinquilini, che non
passa affatto dal proprietario. I resoconti sono in fondo all'archivio:
[fetta A](#piano-completare-il-prodotto)
· [esito](#esito-completare-il-prodotto-fetta-a) ·
[fetta B](#piano-inquilino-fetta-b)
· [esito](#esito-inquilino-fetta-b) ·
[fetta B-bis](#piano-spese-co-living-fetta-b-bis) ·
[fetta C](#piano-fetta-c)
· [esito](#esito-fetta-c), **chiuso dal collaudo il 10 settembre 2026**.

**Le notifiche vanno in fondo, e non sono più una fetta A-bis.** Erano i passi 9 e
10 del piano archiviato, e la scelta era «prima o dopo la B». È stata sciolta: *dopo*,
insieme a tutto il resto che serve solo a utenti veri. La ragione è nel criterio qui
sotto.

**Il co-living era provabile con dati veri solo dalla fetta B, ed è stato provato nella
B-bis**: due account che entrano da un browser e vedono cose diverse è la condizione per
accorgersi che un saldo è sbagliato, e con `curl` da entrambi i lati non lo sarebbe
stato. La verifica che ha chiuso la fetta — un coinquilino che esce senza saldare e
continua a comparire, **con il suo nome**, nei saldi che legge l'altro — non si sarebbe
potuta fare prima.

Le **due semantiche della dashboard** che erano rimaste aperte **sono state decise**:
`written_off_total` è il nono numero, ed è il residuo e non l'importo; il credito da
sovrapagamento resta fuori dal riepilogo e si evidenzia per addebito. Vedi
[ADR-040](08-decisioni.md#adr-040).

Il debito «nessun endpoint elenca le organizzazioni di un utente» — quello che la
sezione [Cosa un client incontrerà per prime](#cosa-un-client-incontrerà-per-prime)
dava come primo — **è chiuso**: vedi [ADR-041](08-decisioni.md#adr-041). Resta aperto
il suo gemello sulle **persone** di un'organizzazione, e resta aperto apposta: la
fetta B ha verificato che quel menù sarebbe **vuoto**, perché un'organizzazione ha
esattamente uno staff.

Resta la decisione che continua a bloccare più valore di ogni riga di codice, ed è
quella qui sotto: **il fornitore di OCR**.


<a id="raccolta-da-ordinare"></a>


<a id="criterio-d-ordine-com-era"></a>

### Il criterio d'ordine, finché a provare è chi costruisce

**Tutto ciò che serve soltanto a utenti veri va in fondo.** Vale finché il collaudo lo
fa chi sviluppa, ed è la ragione per cui le notifiche hanno smesso di essere una fetta
a sé e sono confluite in [Prima degli utenti veri](#prima-degli-utenti-veri).

Il criterio si applica a ciò che **compra soltanto** quel valore, non a ciò che è
scomodo: la fetta A è stata fatta per prima proprio perché il giro del denaro era
rotto per l'utente che già esiste, e quello non era rinviabile.

Alle notifiche il criterio si applica per intero: oggi il token d'invito **torna nella
risposta HTTP e il client lo mostra** con il suo bottone per copiarlo, presentato per
quello che è. Chi prova il prodotto da solo è il proprietario *e* l'inquilino, quindi
consegnarsi un codice da una scheda all'altra costa un `Ctrl+V`. Una dichiarazione di
pagamento in attesa non avvisa nessuno, ma dalla fetta A si trova in `/pagamenti`
senza sapere quale addebito guardare. In entrambi i casi il rimedio esiste e si vede.

**Verificato prima di decidere, perché è la cosa che renderebbe l'ordine impossibile:
la fetta B non dipende da `Notifier`.** Nessuno la inietta, nessuno la chiama —
l'unica riga che la nomina fuori dalla sua porta è la riesportazione in
`application::ports`. Il percorso dell'inquilino si prova quindi per intero senza che
un'email parta mai.

**Che cosa costa rinviarle, e va saputo**: fino ad allora l'onboarding **non è pronto
per utenti reali**, perché chiedere a un inquilino di farsi consegnare un codice a
mano non è un prodotto. Non è un debito nascosto — è questa riga.

Il criterio decade nel momento in cui qualcuno che non sia chi sviluppa apre il
prodotto. È lì che [Prima degli utenti veri](#prima-degli-utenti-veri) diventa la
prossima fetta, e non prima.

«Far sì che i coinquilini si vedano» è stato fatto: vedi
[l'esito](#esito-coinquilini-visibili-fetta-1). L'**OCR delle bollette** è stato fatto:
vedi [l'esito](#esito-ocr-bollette-fetta-1) — con l'avvertenza che quella fetta ha
comprato il **percorso** e non il valore, perché [ADR-037](08-decisioni.md#adr-037)
rimanda deliberatamente la scelta del fornitore.

**Il perimetro funzionale dell'MVP è chiuso.** Non resta alcun modulo che abbia lo
schema e non il codice. Ne segue un cambiamento nel modo di scegliere che cosa fare, e
vale la pena dirlo perché finora l'ordine se lo dettava lo schema: da qui in poi
**l'ordine non è più derivabile**. Le prossime decisioni sono di prodotto — se
l'onboarding debba funzionare senza che il proprietario consegni un codice a mano, se
l'accuratezza dell'OCR valga il costo di misurarla, dove il prodotto verrà esercito —
e non hanno una risposta che si possa leggere in `migrations/`.

La misura con cui la fetta A era stata pianificata, rifatta: delle **88 rotte
documentate** — 86 di prodotto più le due di salute — **29 non le chiama nessuno**, ed
erano 37 su 87. Le rimanenti sono quasi tutte in due blocchi, che sono i passi da B2 a
B6: **le segnalazioni** (13 rotte) e le **spese co-living con i regolamenti** (15).
Più l'archiviazione di un documento, che è il passo B8.

Il conteggio va letto con la sua avvertenza: «chiamata dal client» significa che una
funzione in `apps/web/src/api/endpoints.ts` la nomina, non che ogni suo parametro sia
raggiungibile dall'interfaccia.


<a id="cosa-un-client-incontrera-per-prime"></a>

### Cosa un client incontrerà per prime

Tre cose, tutte registrate fra i debiti: **non esiste un endpoint che elenchi le persone
di un'organizzazione**, `app.expense_group_members` non ha privilegi di colonna, e **un
job abbandonato resta `running` per sempre**. La prima si vede appena si costruisce
l'interfaccia di un gruppo di spesa; le altre due no, ed è la ragione per cui stanno
scritte.

**La prima si è già presentata due volte, e nessuna delle due chiedeva un endpoint.**
La fetta A l'ha incontrata su `payer_user_id` e l'ha risolta nel form, senza toccare la
colonna ([ADR-040](08-decisioni.md#adr-040)). La fetta B l'ha incontrata
sull'assegnazione di una segnalazione, e verificando ha scoperto che **quel menù
sarebbe vuoto**: `InviteTenant` scrive `role: OrgRole::Tenant` fisso e `OrgRole::Manager`
non lo scrive nessuno, quindi un'organizzazione ha esattamente uno staff. L'azione è
diventata «prendi in carico» ([ADR-042](08-decisioni.md#adr-042)).

Ne segue una cosa che vale la pena scrivere, perché è la lezione e non l'aneddoto: **il
debito è reale ma il suo rimedio non è quello che sembra**. Le prime due occorrenze
chiedevano un elenco di persone e nessuna delle due lo voleva davvero — una voleva un
form che non ammettesse un vuoto, l'altra voleva sapere che non c'è nessuno da
elencare. La terza occorrenza, la composizione di un gruppo di spesa in `B6`, ha già la
sua risposta e non è `GET /members`: è
`GET /properties/{id}/residents`, che esiste dalla migration `0030`. Chi partirà a
scrivere `GET /members` costruirà la cosa sbagliata prima di accorgersi che non serviva.


<a id="perche-la-dashboard-doveva-restare-ultima"></a>

### Perché la dashboard doveva restare ultima

**Scritta quando la dashboard non c'era, e tenuta perché la sua conclusione è
generale**: vale per la prossima cosa a buon mercato che sembrerà ovvia. Diceva perché
era la conclusione **sbagliata** che la fetta contabile invitava a trarre, e il piano
dei pagamenti l'aveva quasi presa.

Il seguito ha dato ragione a entrambe le parti dell'argomento: la dashboard è stata
fatta come **passo 1 del client**, cioè per l'eccezione che questa sezione ammetteva
(«vale se serve qualcosa da mostrare presto»), e le sue due semantiche aperte sono
state decise **una fetta dopo, con il client davanti** — che è precisamente quando
questa sezione diceva che si sarebbero potute decidere.

Fino a questa fetta due dei suoi numeri mentivano: `total_outstanding` contava come
credito anche gli addebiti annullati e stornati — corretto dalla `0024`,
[ADR-032](08-decisioni.md#adr-032) — e `paid_amount` era zero ovunque perché
`app.payments` era vuota. Entrambe le cause sono chiuse, quindi la dashboard **è
sbloccata**, e costa uno use case di lettura, un DTO e una rotta.

**Sbloccata non è prioritaria.** La finestra in cui cambiare il significato di quei
numeri è additivo la chiude il **primo client**, non l'esistenza dell'endpoint:
costruirla adesso non la consuma, non la restringe, e non compra tempo su niente.
Confondere «adesso si può» con «adesso conviene» è precisamente l'errore, ed è tanto più
facile quanto più il modulo è a buon mercato.

Contro l'anticiparla c'è invece una ragione di merito: **due delle sue semantiche non
sono decise**, ed è ciò che questo progetto tiene fuori per principio — la stessa
ragione per cui `bill_status.disputed` è rimasto fuori dalla fetta contabile. Aggiungere
un campo dopo è additivo; cambiare cosa significa un campo già pubblicato è una rottura.

Le due domande erano queste, **e hanno una risposta dalla fetta A**. Restano scritte
nella forma in cui erano aperte perché è quella che mostra che cosa il primo client
serviva a decidere:

- **`written_off` va riportato a parte come perdita?** *Sì*, ed è il nono numero
  `written_off_total`. Usciva dall'insoluto insieme a `void`, e i due erano distinti
  nello stato e indistinguibili in dashboard; la distinzione conta nei report, e la
  dashboard è il primo report. La risposta ha portato con sé una seconda decisione che
  la domanda non conteneva — **lo storno esteso a `partially_paid`** — senza la quale
  il numero avrebbe contato metà delle perdite;
- **il credito da sovrapagamento va mostrato?** *Sì, ma non qui.* `outstanding_amount`
  negativo esiste ed è escluso dal totale dal `WHERE outstanding_amount > 0` della
  vista. Non compensa gli altri insoluti — ed è giusto, un credito non è un insoluto
  negativo — e non sparisce più del tutto: si evidenzia **per addebito**, nell'elenco
  e nella scheda, con la parola accanto al numero.

Entrambe in [ADR-040](08-decisioni.md#adr-040). Ed erano additive quando sono state
prese, che è esattamente ciò che questa sezione sosteneva: la finestra l'ha chiusa il
primo client, non l'esistenza dell'endpoint.

L'unico argomento per anticiparla, e va riconosciuto perché è vero: è la prima
superficie di lettura che attraversa **tutti** i moduli insieme, quindi il primo posto
dove si vedrebbe un errore di integrazione fra le fette. È un argomento di
dimostrabilità, non di costruzione: vale se serve qualcosa da mostrare presto, e in quel
caso l'ordine si inverte deliberatamente e non per distrazione.

**La fonte di verità resta la vista.** `outstanding_amount` continua a venire da
`app.v_charge_balances` e non da `status`: lo stato memorizzato è una comodità
interrogabile — l'indice `charges_outstanding_ix` è parziale su quei tre valori — che il
modulo dei pagamenti tiene allineata come effetto secondario, non un secondo calcolo. Le
due definizioni non devono diventare due, ed è la ragione per cui
`PaymentRepository::sum_succeeded_for_charge` ripete `amount - refunded_amount` sugli
stessi due stati della vista invece di sommare gli importi e basta.

Manca ancora l'**invio delle notifiche**: la porta `Notifier` è dichiarata e senza
adattatore, quindi il token di invito torna nella risposta HTTP e lo consegna il
proprietario. Va implementata prima di considerare l'onboarding pronto per utenti
reali, ed è oggi l'ultima porta senza implementazione.

**Dalla fetta A ha un secondo consumatore in attesa, e la coda ne attenua il
sintomo**: una dichiarazione di pagamento in `pending` non avvisa nessuno, ma da
`/pagamenti` si trova senza sapere quale addebito guardare. Prima non si trovava
affatto. La notifica resta la cosa che toglie il «guardare», ed è in
[Prima degli utenti veri](#prima-degli-utenti-veri): finché a provare il prodotto è
chi lo costruisce, guardare la coda è esattamente ciò che si fa comunque.

---

<a id="piano-completare-il-prodotto-cosa-resta"></a>


<a id="piano-completare-il-prodotto-com-era"></a>

## Piano — Completare il prodotto: che cosa resta

Le fette A, B e B-bis sono concluse. I piani che le hanno prodotte sono archiviati per
intero in [11 — Piani conclusi](11-piani-conclusi.md), ognuno con il suo esito
([fetta A](#piano-completare-il-prodotto),
[fetta B](#piano-inquilino-fetta-b),
[fetta B-bis](#piano-spese-co-living-fetta-b-bis)), e **le loro
decisioni valgono ancora**: quel documento resta la fonte del *perché*, e questo del
*cosa*.

Le due liste si sovrappongono di proposito. L'archivio non si riscrive mai, quindi
contiene le fette **come erano state pensate** — comprese le notifiche nella posizione
che avevano allora, e i dodici passi della fetta B di cui se ne sono eseguiti sei;
questo file si riscrive a ogni fetta, e contiene ciò che resta con il dettaglio per
eseguirlo. Chi legge solo questo può eseguire; chi vuole sapere perché va di là.

**L'ordine qui è cambiato rispetto ai piani archiviati, ed è la sola differenza di
sostanza.** Là le notifiche erano una «fetta A-bis» da fare prima o dopo la B a
scelta, e la fetta C si chiamava così. Qui sono confluite in un'unica fase finale —
[Prima degli utenti veri](#prima-degli-utenti-veri) — per il
[criterio d'ordine](#criterio-d-ordine-com-era) più sopra.
La scomposizione dei due passi delle notifiche non è cambiata di una riga: è cambiato
soltanto **quando**.

**Le fette B e B-bis si sono fermate ai loro punti di taglio dichiarati**, `B5` e `B8`,
e non per stanchezza: erano scritti nei piani prima di cominciare. Ciò che resta — gli
ultimi due passi del client — riprende qui **con la numerazione originale**, perché è
quella con cui è stato pensato e perché l'archivio vi rimanda.

Restano quindi **due fette**, e della prima manca il collaudo:

| | Fetta | Stato |
|---|---|---|
| 1 | [C](#fetta-c) — `C1`–`C6`, i buchi che un collaudo completo incontrerebbe, `B9` e `B10` compresi | **codice eseguito, collaudo da percorrere** |
| 2 | [Prima degli utenti veri](#prima-degli-utenti-veri) — notifiche, poi il resto | dopo il collaudo, e non prima |

**Con B-bis l'MVP non ha più moduli con lo schema e senza interfaccia**, e con la C non ha
più operazioni dell'API irraggiungibili da una schermata — salvo `getPayment`, che resta
tale apposta. Quello che resta non è una capacità, ed è la prima volta da sei fette. La C
finisce con il collaudo completo, che è la condizione per aprire a chi non ha scritto il
codice, e **quel collaudo non è ancora stato fatto**.

### Fetta B-bis — le spese co-living · **conclusa**

Eseguita il **7 settembre 2026**, per intero e fino al suo punto di taglio: `B6`–`B8`.
Il piano e il suo esito sono in
[11 — Piani conclusi](#piano-spese-co-living-fetta-b-bis).

Il modulo che aveva lo schema e l'API e nessuna interfaccia adesso ce l'ha: gruppi con
la composizione, spese ripartite, saldi, e i rimborsi a due tempi con il ritiro. Il
backend non è stato toccato — **424 test Rust prima e dopo** — e la decisione che la
fetta esisteva per prendere è diventata [ADR-043](08-decisioni.md#adr-043): il nome di un
membro si scrive all'ingresso, perché dopo non esiste più da nessuna parte.

`B9` e `B10` non sono entrati, ed è la decisione 7 di quel piano applicata: sono `C3` e
`C5` della fetta qui sotto, che li ha assorbiti insieme alle altre cose minime.

<a id="fetta-c"></a>

### Fetta C — chiudere i buchi che un collaudo completo incontrerebbe · **eseguita, collaudo aperto**

Eseguita l'**8 settembre 2026**, per intero e fino al suo punto di taglio: `C1`–`C6`, con
`B9` e `B10` assorbiti. Il piano e il suo esito sono in
[11 — Piani conclusi](#piano-fetta-c).

Le cinque funzioni del livello dati che nessuna schermata chiamava adesso hanno la loro,
e la più importante chiudeva il giro del prodotto: **un invitato entra dall'interfaccia**,
senza `curl`. Il backend è stato toccato **una volta sola** — il messaggio della
configurazione di `C6` — e i 424 test Rust sono gli stessi prima e dopo; il client passa
da 61 a 69.

**La fetta non è chiusa**, e la parte che manca è quella che conta: il
[collaudo in undici punti](#esito-fetta-c) non è ancora stato
percorso. È il criterio della fetta, e finché non lo si è fatto «funziona» resta
un'affermazione su cui nessuno può essere smentito. Ciò che il giro trova va scritto
nell'esito **anche se non si corregge**.

Due decisioni sono diventate ADR: [044](08-decisioni.md#adr-044) sul rinnovo fra schede,
che si **serializza** invece di trasmettere la sessione, e
[045](08-decisioni.md#adr-045) sull'accettazione che sceglie l'organizzazione.
<a id="prima-degli-utenti-veri"></a>

<a id="prima-degli-utenti-veri"></a>

### Prima degli utenti veri

Tutto ciò che compra **soltanto** il valore che un utente vero richiede, e niente che
serva a chi costruisce. Per il criterio d'ordine più sopra questa fase viene ultima, e
**diventa la prossima fetta nel momento in cui qualcuno che non sia chi sviluppa apre
il prodotto** — non prima.

La fase ha due metà con statuti diversi, e confonderle costerebbe: la prima è
**pianificata**, la seconda è **nominata e non pianificata**.

#### Le notifiche — l'unica parte già pianificata

I passi **9 e 10** del piano archiviato, dichiarati come punto di taglio fin dalla
pianificazione: dopo A8 il proprietario ha un modulo contabile completo con ciò che
l'API già sa fare, e le notifiche sono un fronte diverso — tocca una **porta**, non
una schermata.

**Stanno qui e non prima della fetta B**, per il criterio d'ordine più sopra: comprano
soltanto ciò che serve a utenti veri. Oggi il token d'invito torna nella risposta HTTP
e il client lo mostra con il suo bottone per copiarlo, e chi prova il prodotto da solo
è il proprietario *e* l'inquilino. **Nessuna dipendenza tecnica in nessuna direzione**:
`Notifier` non è iniettata da nessuna parte, quindi la fetta B non la incontra mai.

Sono però **l'unica cosa di questa fase che è già pianificata**, ed è la ragione per
cui i due passi restano scritti per esteso qui sotto invece di essere solo nominati:
non dipendono da dove il prodotto verrà esercito, a differenza di tutto il resto.

1. **`Notifier`: la porta e l'adattatore.** La variante `PaymentDeclared` in
   `NotificationContent` — oggi ce n'è **una sola**, `TenantInvitation` —
   l'adattatore SMTP in `propflow-infrastructure`, il raccoglitore SMTP locale nel
   `docker-compose.yml`, la configurazione in `AppConfig` e nelle tabelle di
   [07 — Operazioni](07-operazioni.md).
2. **Gli invii.** Iniezione della porta in `InviteTenant` e in `DeclarePayment`:
   firma degli use case, `AppState`, `Dependencies`, `main.rs` e l'harness dei test.
   Invio **dopo il commit e best-effort**. Il client mostra «inviato» e tiene il
   codice sotto un dettaglio richiudibile.

**Le decisioni 5 e 8 del piano archiviato governano questi due passi e vanno lette
prima di toccarli.** In breve, e senza sostituire l'originale:

- **SMTP e non un fornitore con la sua API**, per la stessa ragione per cui
  `ObjectStorage` ha un adattatore solo che parla anche con MinIO: il fornitore
  diventa un **dato di configurazione** invece di un ramo del codice;
- **l'invito non può passare dall'outbox**, ed è la trappola più cara della fetta. Il
  token in chiaro esiste **solo** nella risposta HTTP — in database ne resta lo hash,
  e gli eventi di dominio non trasportano valori né credenziali — quindi un worker che
  leggesse `TenantInvited` non avrebbe niente da spedire. Metterci il token dentro
  significherebbe scrivere una credenziale in una tabella che esiste per essere
  pubblicata su un broker. L'invio è **dopo il commit, best-effort**: se la posta non
  risponde, l'invito resta valido, il token resta nella risposta, e il fallimento va
  nei log **senza il destinatario e senza il token**;
- **la dichiarazione di pagamento non ha quel vincolo** — non c'è una credenziale da
  recapitare — ma segue la stessa via, per non avere **due** meccanismi di invio
  appena nato il primo.

**Le altre due trappole di questi passi**, dal piano archiviato:

- **`Notifier` non è iniettata da nessuna parte.** `InviteTenant::new` prende tre
  argomenti e nessuno è un `Notifier`. Non è «scrivere un adattatore»;
- **l'adattatore SMTP è il posto più facile in cui far uscire dati personali.**
  `docs/04` vieta email, token e corpi nei log, e un client SMTP che registri la
  richiesta li registra tutti e tre insieme. La porta ha già un `Debug` redatto, e
  l'adattatore non deve annullarlo;
- **un raccoglitore SMTP locale non è un fornitore vero.** Accetta tutto: non verifica
  SPF, DKIM né i rimbalzi. Verifica che il messaggio parta e che cosa contiene, **non**
  che sarebbe recapitato. È lo stesso limite di MinIO rispetto a S3.

**Come si saprà che è finita:** un'email di invito che arriva davvero nel raccoglitore
locale, con il token nel corpo e in **nessuna** riga di log.

#### Il resto — nominato e non pianificato

Rate limiting per IP, verifica dell'email bloccante, manifest Kubernetes, backup e
restore, metriche, alerting, cron per le partizioni di `audit_log`.

Non è dimenticanza, ed è la ragione per cui ha un nome: dipendono da **dove** il
prodotto viene esercito, che non è deciso, e pianificarle adesso significherebbe
scrivere un piano su un'ipotesi. È anche la differenza con le notifiche qui sopra, che
non dipendono da quella scelta e sono infatti scritte per esteso.

**La verifica dell'email bloccante ha una conseguenza sull'ordine che vale la pena
dire**: renderla obbligatoria cambia il percorso di registrazione, cioè la prima cosa
che ogni collaudo attraversa. Farla mentre si prova da soli significherebbe pagarne il
costo a ogni giro senza comprarne il valore, ed è il caso più netto del criterio.


<a id="non-implementato-com-era"></a>

### «Non implementato» com'era, con la voce della dashboard tenuta per la sua lezione

## Non implementato

### Moduli MVP che hanno lo schema ma non il codice

In ordine di dipendenza suggerito. Ognuno segue il pattern già stabilito: dominio →
porta → use case → repository → endpoint → test.

1. **AI: triage delle segnalazioni** (`tickets.ai_job_id`, `ticket_messages.ai_job_id`)
   — l'**OCR delle bollette è fatto**, vedi [l'esito](#esito-ocr-bollette-fetta-1): la
   coda, il worker, il job e la revisione esistono, e `ocr_extracted`, `ocr_confidence`
   e `pending_review` sono scritti da una macchina.

   Quello che resta è il triage, e **non è una seconda chiamata allo stesso
   adattatore**: è decidere se una macchina possa scrivere categoria e priorità di una
   segnalazione, e con quale soglia di confidenza, che è una domanda di prodotto sua.
   La capacità `vision` e il task `ticket_triage` esistono nell'enum e nessuno li
   scrive.

   **Prima del triage viene però la scelta del fornitore**, che blocca anche l'OCR dal
   produrre valore: vedi [ADR-037](08-decisioni.md#adr-037) e la sezione
   [La decisione che ora blocca più valore di ogni riga di codice](09-stato-e-roadmap.md#la-decisione-che-ora-blocca-più-valore-di-ogni-riga-di-codice).
2. ~~**Dashboard**~~ — **fatta**, come passo 1 della fetta del client: `GET
   /api/v1/dashboard`, otto numeri letti da `v_owner_dashboard`, staff soltanto.
   Nessuna migration: la vista esiste dalla `0015`.

   **Le due semantiche che erano rimaste aperte sono state decise**, nella fetta A e
   con il client davanti, che era la condizione: `written_off_total` è il nono
   numero, ed è il **residuo** e non l'importo; il credito da sovrapagamento resta
   fuori dal riepilogo e si evidenzia per addebito. Vedi
   [ADR-040](08-decisioni.md#adr-040) e
   [l'esito](#esito-completare-il-prodotto-fetta-a).

   La voce si tiene perché la sua conclusione — «sbloccata non vuol dire
   prioritaria», e la finestra sulle semantiche la chiude il **primo client** e non
   l'esistenza dell'endpoint — resta vera e generale, e vale per la prossima cosa a
   buon mercato che sembrerà ovvia. La finestra si è chiusa nel modo previsto:
   costruendo il client, non scrivendo l'endpoint.

   L'eccezione che quella conclusione ammetteva — «vale se serve qualcosa da
   mostrare presto» — è esattamente il caso che si è verificato: la dashboard
   serviva al client, ed è stata fatta per quello e non prima.

   **L'argomento che l'ha portata avanti** è quello che era già scritto qui: è la
   prima superficie di lettura che attraversa immobili, contratti, segnalazioni,
   bollette, addebiti e pagamenti insieme, quindi il primo posto dove si vedrebbe un
   errore di integrazione fra le fette. Eseguendola non se n'è visto nessuno, il che
   è il migliore esito possibile e anche il meno raccontabile.

**Documenti** è implementato nella fetta 1: caricamento, elenco, download e
archiviazione su immobili, stanze, contratti, segnalazioni e — dal modulo contabile —
bollette e addebiti. Restano fuori, e non per dimenticanza, il versionamento (V2) e
l'OCR (punto 1); allargare gli owner ad avatar, spese e pagamenti è additivo e si fa
quando quei moduli esistono. Il resoconto è in fondo:
[Piano — Documenti, fetta 1](#piano-documenti-fetta-1).

**Pagamenti** è implementato nella fetta 1 e **completato dal lato del proprietario
nella fetta A**: incasso registrato dallo staff, dichiarazione dell'inquilino in
`pending`, conferma e rifiuto, dettaglio, elenco di un addebito e — dalla fetta A —
la **collezione dell'organizzazione** con i filtri `status` e `lease_id`, che è la
coda delle dichiarazioni. Idempotenza fornita dal client, e stato dell'addebito
ricalcolato dai pagamenti. Restano fuori, e non per dimenticanza, i rimborsi, i
provider veri con i loro webhook, i pagamenti di quote e regolamenti — che restano
fuori **per sempre**, non in attesa: vedi sotto — e il
pagamento della bolletta al fornitore, che non è modellato affatto. I resoconti sono
in fondo: [Piano — Pagamenti, fetta 1](#piano-pagamenti-fetta-1)
e [Piano — Completare il prodotto, fetta A](#piano-completare-il-prodotto).

**Spese co-living** è implementato nella fetta 1: gruppi con chi li crea già dentro,
composizione con uscita e rientro, spese ripartite `equal` e `weighted` con la somma
delle quote garantita dal database, saldi letti dalla vista, e regolamenti a due tempi
confermati da **chi riceve**. Ha richiesto **quattro migration** — la `0026` per la
policy che lascia iscriversi chi crea il gruppo, la `0027` per la vista che perdeva chi
usciva con un conto aperto, la `0028` per i privilegi di colonna che chiudono un buco
vero del trigger contabile, la `0029` per l'identità di una membership e per rendere
l'uscita un confine vero — e due ADR, [034](08-decisioni.md#adr-034) e
[035](08-decisioni.md#adr-035), il secondo con il corollario sulla direzione che la
`0029` ha aggiunto. Restano fuori, e non per dimenticanza, la
semplificazione dei debiti, i tre metodi di ripartizione oltre `equal` e `weighted`,
`is_settled`, il legame fra regolamento e pagamento, la ripartizione automatica di una
bolletta e gli allegati alle spese. Il resoconto è in fondo:
[Piano — Spese co-living, fetta 1](#piano-spese-co-living-fetta-1).

**E dalla fetta B-bis ha anche l'interfaccia**, senza che il backend sia stato toccato:
gruppi, saldi, spese con le loro quote, e i rimborsi a due tempi con il ritiro. La
decisione che è servita a costruirla è [ADR-043](08-decisioni.md#adr-043) — nessuna
risposta di quel modulo porta un nome, quindi il client lo scrive all'ingresso — e
l'esito è
[Piano — Le spese co-living, fetta B-bis](#piano-spese-co-living-fetta-b-bis).

**`payable_type` resta a un valore su tre, e non è un lavoro rinviato.**
`expense_share` e `settlement` non si scriveranno: un regolamento fra coinquilini non è
denaro che l'organizzazione incassa, e farne una riga in `app.payments` metterebbe nel
registro degli incassi del proprietario un movimento che non lo riguarda. Il piano dei
pagamenti prevedeva il contrario; la previsione era sbagliata, ed è registrata qui
perché è il genere di conclusione che si eredita senza riverificarla.

**Segnalazioni** è implementato nella fetta 1: apertura, elenco filtrato, dettaglio
con lo storico degli stati, thread con note interne, le sette transizioni del ciclo di
vita, costi e allegati. Restano fuori, e non per dimenticanza, il triage AI (punto 1),
la ricerca full-text (V2), gli SLA, la programmazione, gli allegati sui singoli
messaggi e le note di risoluzione. Il resoconto è in fondo:
[Piano — Segnalazioni, fetta 1](#piano-segnalazioni-fetta-1).


<a id="storia-dei-conteggi-dei-test"></a>

### La storia dei conteggi dei test, com'era

459 test: 214 unitari di dominio, 16 applicativi, 27 di infrastruttura, 11 unitari
del bordo HTTP, 82 di integrazione sulla persistenza, 3 sull'adattatore S3 contro
MinIO, 103 di contratto HTTP, 3 di guardia sulla documentazione della configurazione.

**I dieci della sezione 6 provano soprattutto che i confini sono nel database.** Il
più denso è quello dei privilegi: otto `UPDATE` scritte a mano come ruolo applicativo
su `app.users`, una per colonna negata, e quattro sulle concesse — perché è **l'elenco**
a essere il contratto, e `platform_role` da sola non basterebbe. Gli altri provano che
non si esce con un contratto in essere, che chi è uscito rientra accettando un invito
nuovo — e che **con il token di prima**, nei suoi quindici minuti, non legge più il
contratto disdetto né apre segnalazioni nell'organizzazione lasciata (`0041`,
[ADR-054](08-decisioni.md#adr-054)) — e che dopo un cambio password ciò che muore è il
**rinnovo** e non la lettura: l'access token è un JWT e vive i suoi quindici minuti.

I sette della sezione 5 provano soprattutto **confini**: la segnalazione condivisa che
arriva al coinquilino *con il thread e lo storico*, quella non condivisa che non gli
arriva affatto, il coinquilino che la vede e non la scrive, la correzione che si chiude ad
`assigned`, il thread chiuso che si legge e non si scrive, e il motivo della riapertura
che si va a rileggere nella conversazione.

**I sei della 5-quater provano la posta e il recupero**, e uno di essi ha trovato il
confine della decisione che doveva confermare: «revoca tutte le sessioni» revoca i
**refresh token**, non gli access token, che sono JWT e non sono revocabili. Il test
asseriva la cosa sbagliata, ed è finita in [ADR-052](08-decisioni.md#adr-052).

**I cinque della sezione 5-ter provano una cosa diversa: la seconda volta.** Due
percorrono la riproduzione di `R31` — un inquilino invitato due volte, che prima riceveva
`409` sul secondo invito — e uno di essi verifica che accettare come inquilino non
retroceda chi è già proprietario. Gli altri tre guardano i nomi su ogni risposta,
transizioni comprese, l'immobile archiviato che lascia il campo assente **senza far
sparire la riga**, e il residuo da riaddebitare con la regola che si perde: un addebito
annullato non conta.


104 test: 16 sulla sessione e sul client dell'API — fra cui il refresh a singolo
volo **e quello fra schede, entrambi riprodotti anche al contrario** — 11 sui
decimali e sulla formattazione, 5
sulla chiave di idempotenza, 4 sull'accettazione di un invito — l'header che **non**
si manda, l'organizzazione che si sceglie, e il messaggio che non parla di password —
10 sui percorsi da sola tastiera e sui form che non
perdono ciò che è stato digitato — fra cui il bottone che mostra la password, che è nel
giro di tabulazione e **non invia il form** — 3 su «Prendi in carico», che sparisce a chi
ce l'ha e resta su ciò che è di qualcun altro, 6 sulla dichiarazione di pagamento, 5 sul guscio —
la navigazione per ruolo, il selettore che mostra i nomi, e l'organizzazione
ricordata che l'elenco vero corregge — 13 sui contratti, che sono le due aritmetiche
replicate dal server: la quota che resta da assegnare e la sovrapposizione dei periodi come
la contano i vincoli di esclusione — e 13 sulle spese co-living: la risoluzione dei
nomi **compreso chi non è più residente**, la somma dei netti verificata sugli interi, e
la scheda del gruppo con un membro uscito che ha ancora un conto aperto. Gli ultimi
**sedici** sono della sezione 5-ter: 8 sui badge — e quelli che contano sono i tre
sull'**anomalia che resta doppia**, che è l'assert che qualcuno toglie semplificando — 2
sull'aritmetica della sottrazione, che è il posto in cui `parseFloat` rientrerebbe dalla
finestra, e 3 sulla data che si mostra e non si manda, il cui difetto è **invisibile a
schermo**: le due schermate sono identiche, e solo il corpo della richiesta le distingue.

La sezione 7 porta il conteggio a **124**, e i diciassette nuovi sono tutti
verificati **al contrario** prima di essere consegnati — è la regola che il poscritto
della 6 ha reso obbligatoria: 9 sul giro in sequenza, fra cui la scadenza del token a
metà con il singolo volo spento e **la stessa scena in parallelo che fa cadere la
catena**; 5 sulla selezione, con la potatura dopo una rilettura; 2 su una pagina vera,
con un `409` «era già così» che non è rosso e una rete caduta che resta selezionata.

I test sulla chiave di idempotenza sono stati **verificati contro il difetto**, e non
una volta sola: sostituendo `intent.current()` con un `randomUUID()` dentro
`mutationFn` falliscono, sia sulla registrazione di un incasso (fetta A) sia sulla
dichiarazione di pagamento (fetta B). Un test che non fallisce quando il codice è
sbagliato è un verde che non prova nulla.


<a id="ordine-di-esecuzione-com-era-in-12"></a>

### «L'ordine di esecuzione», com'era in 12 prima di passare in 09

## L'ordine di esecuzione

Deciso l'**8 settembre 2026**. Le voci si eseguono **per sezione, una per
conversazione**: è il vincolo che ha deciso i raggruppamenti, e non l'argomento.

Una sezione deve essere **autosufficiente** — chiudersi senza lasciare mezze cose a
quella dopo — perché chi la esegue apre una sessione nuova che non ha visto le
precedenti. Da qui due accorpamenti che per argomento non si farebbero:

- ~~**`R13`, `R17` e `R18` stanno insieme**~~ — **erano un nodo, e la sezione 3 l'ha
  sciolto.** Una quota sbagliata non si correggeva (`R17`), quindi il contratto non si
  attivava, quindi non si eliminava (`R13`), perché l'unica uscita sarebbe stata
  attivarlo. Prenderne una senza le altre avrebbe lasciato aperto il vicolo cieco, ed è la
  ragione per cui sono state fatte insieme;
- ~~**`R7`, `R21` e `R22` hanno la stessa radice**~~ — **la sezione 5 ha risolto la sua
  metà.** I DTO portavano identificatori e nessuna rotta risolveva le persone; `R21` e
  `R22` sono state fatte con le segnalazioni, dove si vedono, e la forma scelta —
  allargare la risposta con un modello di lettura, mai una rotta che risolva le persone —
  è [ADR-049](08-decisioni.md#adr-049). `R7` la eredita invece di riderivarla, ed è la
  ragione per cui la 5 andava **prima** della 6.

| | Sezione | Voci | Natura | Perché è lì |
|---|---|---|---|---|
| ~~**1**~~ | ~~Ritocchi visibili~~ — **conclusa** l'8 settembre 2026 | `R5` `R4` `R6` `R10` `R20` + il bottone del thread | solo client | Le sei voci sono uscite da questo file: [piano](#piano-sezione-1) · [esito](#esito-sezione-1). La riga resta perché l'ordine si legge per intero, e perché l'esito dice una cosa che serve alle sezioni dopo: la navigazione richiedeva **tre** modifiche e non due |
| ~~**2**~~ | ~~Le pendenze della fetta C~~ — **conclusa** il 9 settembre 2026, **senza il collaudo** | `R2` `R3` | doc + una stringa di backend | [Esito](#esito-sezione-2). `R2` ha corretto il punto 10 e, rileggendo gli altri dieci, ha trovato **il punto 4** con lo stesso difetto; `R3` era in **otto** posti e non in due. Il collaudo si è staccato ed è diventato la **5-bis** |
| ~~**3**~~ | ~~Contratti: correggere gli errori~~ — **conclusa** il 9 settembre 2026 | `R13` `R17` `R18` `R12` `R14` | backend | Era il nodo e il danno peggiore che esistesse: contratti attivi con le quote sbagliate e nessun modo di correggerli. [Piano](#piano-sezione-3) · [esito](#esito-sezione-3). Eseguirla ha chiuso **un buco che non era in elenco** — si poteva aggiungere una parte a un contratto attivo — e ne ha aperto uno nuovo: `R28`, il subentro |
| ~~**4**~~ | ~~Stati che nessuno aggiorna~~ — **conclusa** il 9 settembre 2026 | `R1` `R11` `R25` | migration + dominio + client | [Piano](#piano-sezione-4) · [esito](#esito-sezione-4). La risposta era già in [ADR-019](08-decisioni.md#adr-019): le conseguenze si derivano. Ha corretto **un numero falso che nessuno guardava** — `v_property_occupancy` diceva zero a tutti — e ha lasciato in un test la prova che un trigger sarebbe stato sbagliato |
| ~~**5**~~ | ~~Segnalazioni~~ — **conclusa** il 9 settembre 2026 | `R19` `R19-bis` `R21` `R22` `R23` `R24` | migration + dominio | Le sei voci sono uscite da questo file: [piano](#piano-sezione-5) · [esito](#esito-sezione-5). **Due** migration: la `0035` allarga chi *vede*, la `0036` chi *scrive* — e la seconda **revoca** [ADR-026](08-decisioni.md#adr-026) su due colonne, emendando nello stesso passo l'invariante di schema e il test che la difendevano. Ha reso **portante** `users_select`: è il primo client che legge un nome da `app.users` |
| ~~**5-bis**~~ | ~~Il collaudo in undici punti~~ — **concluso** il 10 settembre 2026, per intero | — | a mano, due account | [Esito](#esito-sezione-5-bis), e con esso l'[esito della fetta C](#esito-fetta-c) **smette di essere aperto** dopo due giorni e cinque sezioni. **Nessuno degli undici punti si è fermato**: le sei voci che ne sono uscite — `R30`–`R35` — sono difetti dentro percorsi che funzionano. Il rendimento che il rinvio aveva sospeso è misurato: mezza giornata di giro ha prodotto sei voci, cinque sezioni di lettura del codice ne avevano prodotte quattro. E una, `R31`, **nessun test poteva trovarla**: ogni test costruisce un inquilino nuovo, e quel difetto vive nella seconda volta che qualcosa succede alla stessa persona |
| ~~**5-ter**~~ | ~~Quello che il collaudo ha trovato~~ — **conclusa** il 10 settembre 2026 | `R30` `R31` `R32` `R33` `R34` | dominio + client | [Piano](#piano-sezione-5-ter) · [esito](#esito-sezione-5-ter). Un ADR e **nessuna migration**: il ramo che fa rientrare un inquilino passa dal primo ramo di `org_members_select`, che non chiede di essere già membri. Il piano aveva una premessa sbagliata — l'intestatario **non** era in `LeaseResponse` — e una decisione giusta: si è emendata la premessa con un terzo campo nella stessa query. Apre `R36`, il badge degli addebiti, che ha la stessa forma di `R30` e non lo stesso difetto |
| ~~**5-quater**~~ | ~~Prima degli utenti veri~~ — **conclusa** il 10 settembre 2026, **due sezioni su quattro eseguite** | — | posta + dominio + migration | [Piano](#piano-utenti-veri) · [esito](#esito-utenti-veri). `Z1` e `Z2` sono codice e girano; `Z3`, `Z4` e `Z5` hanno prodotto **artefatti e un runbook** e non sono stati eseguiti — servono una VPS, un dominio e delle credenziali. **Non è deployato**, e il criterio è il punto 4 del [runbook](07-operazioni.md#il-primo-deploy--runbook): aprire il prodotto dal telefono, in 4G. Apre `R37`, la cancellazione come capacità del prodotto |
| ~~**6**~~ | ~~Le persone~~ — **conclusa** l'11 settembre 2026 | `R7` `R16` `R38` | migration + backend + client | [Piano](#piano-sezione-6) · [esito](#esito-sezione-6). **Due migration e nessun ADR**, ed era una conclusione del piano: la revoca delle sessioni è stata resa uniforme ad [ADR-052](08-decisioni.md#adr-052) apposta per non doverne scrivere un secondo. È la prima sezione pianificata **leggendo il codice** in cui tutte e cinque le trappole previste si sono avverate; quello che il piano non aveva visto è altrove, e la voce più cara è che il divieto di uscire con un contratto in essere sarebbe valso **solo per chi non amministra** — le policy permissive si sommano in `OR`. Apre `R41` |
| ~~**7**~~ | ~~Cancellare, in generale~~ — **conclusa** il 12 settembre 2026 | `R15` | client trasversale, **più una rotta** | [Piano](#piano-sezione-7) · [esito](#esito-sezione-7). **Una rotta, nessuna migration, nessun ADR**: la decisione l'aveva presa la `0028`. Il piano aveva **tre premesse sbagliate**, e una stava nella tabella «verificato e non dedotto»: un secondo archivio risponde `404`, non `409`, perché la famiglia del soft delete non si rilegge — e il `404` è entrato fra gli «era già così». La `USING` di `expenses_update` è più larga di «chi l'ha registrata o lo staff», e il confine è nel caso d'uso. Quattro test verificati **al contrario** prima di consegnarli. Ha trovato `R45` usando il prodotto |
| ~~**8**~~ | ~~Il perimetro dei ruoli~~ — **conclusa** il 16 settembre 2026 | `R9` | ADR, poi migration + backend + client + `ops/` | [Piano](#piano-sezione-8) · [esito](#esito-sezione-8). [ADR-053](08-decisioni.md#adr-053) prima del codice: l'inquilino entra per invito senza un'organizzazione propria, il proprietario con un token di piattaforma emesso da un runbook, `POST /auth/register` non esiste più. **La prima sezione eseguita con agenti separati, e l'ultima**: l'esito riporta le misure e la ragione del ritiro |
| ~~**9**~~ | ~~Spiegare come si usa il sito~~ — **conclusa** il 17 settembre 2026 | `R8` | client | [Piano](#piano-sezione-9) · [esito](#esito-sezione-9). Era ultima **apposta**, e la scelta di *dove* — gli stati vuoti spiegano, una pagina sola raccoglie, il riepilogo vuoto è il primo accesso — è nel piano con le alternative scartate. Due premesse sbagliate, nessuna decisione cambiata |
| ~~**10**~~ | ~~Tre chiusure prima del primo deploy~~ — **conclusa** il 18 settembre 2026 | `R50` `R51` `R46` | due rotte di API, un ramo nel client, una migration | [Piano](#piano-sezione-10) · [esito](#esito-sezione-10). Ciò che il primo deploy incontra da sé nel primo mese. Una premessa incompleta, trovata dal giro a mano prima del codice: `R51` era un `502` del proxy, non un errore di rete, e il ramo copre entrambi. Ne è uscita `R52`, che non precede il deploy |
| **11** | **Il primo deploy** — [runbook](07-operazioni.md#il-primo-deploy--runbook) | — | a mano, su una macchina | **Deciso con la 9**: si fa con i mezzi che ci sono; la 10 è verde dal 18 settembre 2026, e prima viene il merge in `dev` della PR che porta le sezioni 1-10. Servono una VPS, un dominio, un relay SMTP con SPF, DKIM e DMARC e delle credenziali. Gli artefatti sono scritti e validati per quello che si può validare senza — `docker compose config`, `bash -n` — e non sono mai stati eseguiti. Il criterio di fine è il punto 4 del runbook: **aprire il prodotto dal telefono, in 4G**. Prima del deploy va provato il **ripristino**, una volta, mentre dentro non ci sono ancora i dati di nessuno. L'esito si scrive in [11](11-piani-conclusi.md) come quello di una sezione, **mentre** lo si percorre |

**Concluse la 1, la 2, la 3, la 4, la 5, il collaudo (5-bis), la 5-ter, la 5-quater, la
6, la 7, la 8, la 9 e la 10.** Si continua dall'**11**, il
[primo deploy](07-operazioni.md#il-primo-deploy--runbook), che non ha un piano qui perché
il runbook lo è: il deploy non è più rinviato in fondo — la decisione è presa con la 9 —
ma i mezzi restano da procurare, e la ragione del rinvio che c'era sta qui sotto com'era.

**Le tre sezioni dopo la 5 avevano un obiettivo dichiarato che le legava**: *un
proprietario vero, che non ha scritto il codice, usa il prodotto per un mese con i suoi
dati*. La 5-bis ha detto che il prodotto regge un giro; la 5-ter ha corretto ciò che il
giro ha trovato; la 5-quater ha aggiunto le quattro cose senza cui una persona esterna non
può cominciare — **due scritte, due preparate e non eseguite**.

**Il deploy si rinvia, e non per una scelta di priorità.** Servono una VPS, un dominio, un
relay SMTP con SPF, DKIM e DMARC e delle credenziali, e non ci sono. Gli artefatti e il
[runbook](07-operazioni.md#il-primo-deploy--runbook) sono scritti e restano lì finché
quelle quattro cose esistono; nel frattempo si continua a costruire.

**Ne segue che il criterio d'ordine cambia, e va detto.** La frase che stava qui —
*«quale delle sezioni 6-9 per prima lo dirà quel mese»* — poggiava su un'evidenza che
adesso non arriva. L'ordine 8 → 9 resta quello scritto sotto, ma è **scelto senza
quella prova**: chi lo esegue non ha una misura dietro, ha un ragionamento. Il rendimento
del collaudo — sei voci in mezza giornata contro le quattro di cinque sezioni di lettura —
dice che il modo più economico di riprendere quella prova, **senza aspettare la VPS**, è
un secondo giro in locale: `mailpit` mostra la posta su `localhost:8025`, e l'unica cosa
che resta fuori è «arriva fuori dallo spam».

**Che cosa «pronto» significa, e per chi.** Fino alla sezione 8 la lettura era *un
proprietario che conosci*, non *chiunque si registri*, perché chiunque si registrasse
diventava owner di un'organizzazione propria. Dalla 8 la registrazione aperta non esiste:
si entra per invito, e il perimetro è *chi ha ricevuto un token* — un inquilino da chi lo
ospita, un proprietario da chi amministra la piattaforma. Il deploy non è più tenuto
fermo da questo.

L'ordine non è inciso: se il collaudo o l'uso fanno emergere un danno più urgente, si
riordina. Quello che non si fa è **eseguire mezza sezione**, perché è ciò che lascia
alla conversazione successiva un contesto che non può ricostruire.


