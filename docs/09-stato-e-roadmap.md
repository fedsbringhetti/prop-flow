# 09 — Stato e roadmap

Stato al **4 settembre 2026**.

## Come si usa questo documento

Qui c'è **lo stato** — che cosa esiste, che cosa manca, quali debiti sono stati
contratti apposta — e **il piano in corso**, con il dettaglio con cui è stato
pensato: la scomposizione in passi, le decisioni con il perché, le alternative
scartate e le trappole trovate. Un piano che resta nella conversazione in cui è nato
non esiste per la sessione successiva, che riparte da zero.

I piani **già eseguiti**, con i loro esiti, stanno in
[11 — Piani conclusi](11-piani-conclusi.md). La divisione è per **ciclo di vita** e
non per argomento: questo file si riscrive a ogni fetta, quello non si riscrive mai.
Una divisione fra «vecchio» e «nuovo» avrebbe spostato il confine una volta e poi
sarebbe marcita, perché il piano nuovo diventa storia appena eseguito.

**Un piano si sposta nell'archivio nello stesso commit in cui si scrive il suo
esito.** È l'unico momento in cui chi lavora ha già entrambi i file aperti, e una
regola di migrazione che chieda un passaggio a parte non viene rispettata.

Questo documento può essere lungo, ed è previsto che lo sia: accorciarlo per ordine
toglie esattamente ciò per cui esiste. Ciò che è rinviato va segnato come rinviato,
**con la ragione** — senza, si rilegge come lavoro pendente e qualcuno lo prende in
mano credendo che sia solo rimasto indietro.

Le regole stanno anche in `CLAUDE.md` §3.

## Implementato

### Layer dati — completo per l'MVP

Le 32 migration coprono l'intero perimetro funzionale dell'MVP, non solo la parte
già esposta dall'API: 24 tabelle più `audit_log` partizionata, 35 enum, 6 domini,
171 indici, 66 policy RLS, 50 trigger, 4 viste, 34 funzioni.

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
| Organizzazioni: `GET /organizations` con il **nome**, `ensure_authenticated`, risponde anche senza `X-Org-Id` | completo per la fetta B |
| CRUD immobili con PATCH parziale e archiviazione | completo |
| CRUD stanze annidate sull'immobile | completo |
| Contratti: bozza, parti, attivazione, disdetta, stati derivati | completo |
| Inviti: emissione, revoca, accettazione senza pool privilegiato | completo |
| Documenti: caricamento multipart, elenco, download pre-firmato, archiviazione | completo per la fetta 1 |
| Segnalazioni: apertura, elenco filtrato, dettaglio con storico, thread con note interne, sette transizioni, costi, allegati | completo per la fetta 1 |
| Bollette: registrazione manuale, correzione, validazione, annullamento, riaddebito, allegati | completo per la fetta 1 |
| Addebiti: emissione, elenco e dettaglio con il saldo dalla vista, annullamento, rinuncia all'incasso **anche da `partially_paid`**, allegati | completo per la fetta A |
| Pagamenti: incasso registrato dallo staff, dichiarazione dell'inquilino, conferma e rifiuto, dettaglio, elenco per addebito **e collezione dell'organizzazione** con i filtri `status` e `lease_id`, idempotenza fornita dal client, stato dell'addebito ricalcolato dai pagamenti | completo per la fetta A |
| Spese co-living: gruppi con il creatore già dentro, composizione con uscita e rientro, spese con ripartizione `equal` e `weighted`, saldi letti dalla vista, regolamenti a due tempi confermati da chi riceve | completo per la fetta 1 |
| Coinquilini visibili: elenco dei residenti di un immobile letto da una funzione che proietta, anagrafica ristretta al perimetro della casa | completo per la fetta 1 |
| OCR delle bollette: porta `OcrProvider` senza fornitore scelto, adattatore deterministico, coda `ai_jobs` con `SKIP LOCKED`, `pending_review` con le sue uscite, stato del job dall'API | completo per la fetta 1 |
| Storage oggetti: adattatore S3/MinIO, cifratura letta dal fornitore | completo |
| Migratore, dispatcher dell'outbox, lettura automatica delle bollette | completi |
| Dashboard: `GET /dashboard`, **nove** numeri letti da `v_owner_dashboard`, staff soltanto | completo |
| Sessione in cookie: refresh `HttpOnly` fuori dal corpo, `Secure` condizionale, test che tiene in piedi l'argomento CSRF | completo |
| Immagine Docker, stack compose | completi |
| Pipeline CI | completa |

424 test: 206 unitari di dominio, 16 applicativi, 27 di infrastruttura, 11 unitari
del bordo HTTP, 76 di integrazione sulla persistenza, 3 sull'adattatore S3 contro
MinIO, 82 di contratto HTTP, 3 di guardia sulla documentazione della configurazione.

### Client web — le due metà

`apps/web/`, React e TypeScript in `strict`, servito dalla **stessa origine**
dell'API dietro un reverse proxy.

| Componente | Stato |
|---|---|
| Catalogo delle stringhe, formattazione con `Intl` e la lingua come parametro | completo |
| Aritmetica decimale su interi: `parseFloat` non compare | completo |
| Client dell'API con il **refresh a singolo volo** e la traduzione di `ProblemDetails` | completo |
| Guscio: registrazione, accesso, uscita, organizzazione attiva, navigazione | completo |
| **Il selettore delle organizzazioni letto dall'API**, con i nomi al posto degli UUID; il `localStorage` ridotto a ricordare *quale* era attiva | completo per la fetta B |
| **La navigazione e la rotta iniziale sono funzioni del ruolo**: `useIsStaff()` in un posto solo, e le azioni di scrittura non si offrono a chi riceverebbe `403` | completo per la fetta B |
| Segnalazioni: apertura sull'immobile, elenco con `?open=true`, dettaglio con lo storico, thread paginato a parte, le **sette** transizioni e il `PATCH` dei soli costi | completo per la fetta B |
| **Dichiarazione di pagamento** dell'inquilino, con la chiave di idempotenza della fetta A riusata e non riscritta | completo per la fetta B |
| I tre stati — caricamento, vuoto, errore — come componenti di prima classe | completo |
| Dashboard: **nove** numeri, ognuno collegato al posto dove si agisce | completo |
| Immobili: elenco a cursore, dettaglio, creazione, `PATCH` parziale, archiviazione | completo per la fetta 1 |
| Stanze e residenti | completo per la fetta 1 |
| Contratti: elenco, bozza, parti, attivazione, disdetta, stati derivati mostrati e non filtrati | completo per la fetta 1 |
| Inviti: emissione e consegna del token, presentato per quello che è | completo per la fetta 1 |
| Bollette: registrazione, scansione, **revisione dell'OCR campo per campo**, validazione, riaddebito | completo per la fetta 1 |
| Addebiti: scadenzario, filtro sullo stato memorizzato, **scheda del singolo addebito**, annullamento, rinuncia all'incasso, credito da sovrapagamento evidenziato | completo per la fetta A |
| Pagamenti: registrazione di un incasso con l'attribuzione come scelta esplicita, **coda delle dichiarazioni** con conferma e rifiuto, chiave di idempotenza generata una volta per intento | completo per la fetta A |
| Reverse proxy in sviluppo e in `docker-compose.yml`, immagine del client | completi |

48 test: 12 sulla sessione e sul client dell'API — fra cui il refresh a singolo
volo **riprodotto anche al contrario** — 11 sui decimali e sulla formattazione, 5
sulla chiave di idempotenza, 9 sui percorsi da sola tastiera e sui form che non
perdono ciò che è stato digitato, 6 sulla dichiarazione di pagamento e 5 sul guscio —
la navigazione per ruolo, il selettore che mostra i nomi, e l'organizzazione
ricordata che l'elenco vero corregge.

I test sulla chiave di idempotenza sono stati **verificati contro il difetto**, e non
una volta sola: sostituendo `intent.current()` con un `randomUUID()` dentro
`mutationFn` falliscono, sia sulla registrazione di un incasso (fetta A) sia sulla
dichiarazione di pagamento (fetta B). Un test che non fallisce quando il codice è
sbagliato è un verde che non prova nulla.

## Non implementato

### Moduli MVP che hanno lo schema ma non il codice

In ordine di dipendenza suggerito. Ognuno segue il pattern già stabilito: dominio →
porta → use case → repository → endpoint → test.

1. **AI: triage delle segnalazioni** (`tickets.ai_job_id`, `ticket_messages.ai_job_id`)
   — l'**OCR delle bollette è fatto**, vedi [l'esito](11-piani-conclusi.md#esito-ocr-bollette-fetta-1): la
   coda, il worker, il job e la revisione esistono, e `ocr_extracted`, `ocr_confidence`
   e `pending_review` sono scritti da una macchina.

   Quello che resta è il triage, e **non è una seconda chiamata allo stesso
   adattatore**: è decidere se una macchina possa scrivere categoria e priorità di una
   segnalazione, e con quale soglia di confidenza, che è una domanda di prodotto sua.
   La capacità `vision` e il task `ticket_triage` esistono nell'enum e nessuno li
   scrive.

   **Prima del triage viene però la scelta del fornitore**, che blocca anche l'OCR dal
   produrre valore: vedi [ADR-037](08-decisioni.md#adr-037) e la sezione
   [La decisione che ora blocca più valore di ogni riga di codice](#la-decisione-che-ora-blocca-più-valore-di-ogni-riga-di-codice).
2. ~~**Dashboard**~~ — **fatta**, come passo 1 della fetta del client: `GET
   /api/v1/dashboard`, otto numeri letti da `v_owner_dashboard`, staff soltanto.
   Nessuna migration: la vista esiste dalla `0015`.

   **Le due semantiche che erano rimaste aperte sono state decise**, nella fetta A e
   con il client davanti, che era la condizione: `written_off_total` è il nono
   numero, ed è il **residuo** e non l'importo; il credito da sovrapagamento resta
   fuori dal riepilogo e si evidenzia per addebito. Vedi
   [ADR-040](08-decisioni.md#adr-040) e
   [l'esito](11-piani-conclusi.md#esito-completare-il-prodotto-fetta-a).

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
[Piano — Documenti, fetta 1](11-piani-conclusi.md#piano-documenti-fetta-1).

**Pagamenti** è implementato nella fetta 1 e **completato dal lato del proprietario
nella fetta A**: incasso registrato dallo staff, dichiarazione dell'inquilino in
`pending`, conferma e rifiuto, dettaglio, elenco di un addebito e — dalla fetta A —
la **collezione dell'organizzazione** con i filtri `status` e `lease_id`, che è la
coda delle dichiarazioni. Idempotenza fornita dal client, e stato dell'addebito
ricalcolato dai pagamenti. Restano fuori, e non per dimenticanza, i rimborsi, i
provider veri con i loro webhook, i pagamenti di quote e regolamenti — che restano
fuori **per sempre**, non in attesa: vedi sotto — e il
pagamento della bolletta al fornitore, che non è modellato affatto. I resoconti sono
in fondo: [Piano — Pagamenti, fetta 1](11-piani-conclusi.md#piano-pagamenti-fetta-1)
e [Piano — Completare il prodotto, fetta A](11-piani-conclusi.md#piano-completare-il-prodotto).

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
[Piano — Spese co-living, fetta 1](11-piani-conclusi.md#piano-spese-co-living-fetta-1).

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
[Piano — Segnalazioni, fetta 1](11-piani-conclusi.md#piano-segnalazioni-fetta-1).

### Trasversali mancanti

| Ambito | Cosa manca |
|---|---|
| Sicurezza | rate limiting per IP; verifica email bloccante; MFA (colonna presente); `kid` nei token per la rotazione delle chiavi. **CORS non è più in questo elenco**: non montarlo è una scelta, non un debito — vedi [ADR-038](08-decisioni.md#adr-038) |
| Operazioni | manifest Kubernetes; backup e restore; metriche; alerting; cron per le partizioni di audit |
| Qualità | gate sulle licenze delle dipendenze; test di carico; benchmark |
| Prodotto | le **spese co-living** nel client, che sono la fetta B-bis. Il percorso dell'inquilino non è più in questa riga: entra, vede che cosa deve, dichiara di aver pagato e segnala un guasto. [ADR-039](08-decisioni.md#adr-039) rivede l'ipotesi «mobile nativo» che questa riga conteneva |

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
| ~~L'incasso non attribuito è invisibile a chi ha pagato~~ — **chiuso** dalla fetta A, e non toccando la colonna | La domanda era «obbligatorio, o dedotto dalle parti?»: **nessuno dei due**. Obbligarlo toglierebbe all'API il modo di dire «non attribuito», che è un caso vero; dedurlo non si può, perché un contratto ha spesso più inquilini con le loro quote, e una parte che non ha ancora accettato l'invito non ha un `user_id` a cui attribuire nulla. Il difetto era che ometterlo fosse il **default**, e il rimedio sta nel form: nessun vuoto inviabile, e una voce «non attribuito» che porta scritta la sua conseguenza. Vedi [ADR-040](08-decisioni.md#adr-040) | Chiuso |
| Nessuna via per ritirare una dichiarazione | `payments_update` è staff soltanto, quindi chi dichiara di aver pagato può creare qualcosa che poi non può disfare. L'uscita esiste ed è il rifiuto dello staff, ma l'inquilino che si accorge subito di aver sbagliato importo non ha rimedio proprio. Darglielo non è una rotta in più: richiede una policy che ammetta l'`UPDATE` del pagante sulla **propria** riga in `pending`, cioè una migration e un secondo `WITH CHECK` da scrivere con attenzione | Se il caso si presenta davvero. Documentato nell'API perché è controintuitivo |
| `refunded_amount` è sottratto e non è mai valorizzato | Il repository somma `amount - refunded_amount` come fa la vista, e la colonna vale sempre zero. È deliberato — è ciò che rende la formula *la stessa* il giorno in cui non varrà più zero — e va saputo, perché letto oggi sembra codice per un caso che non esiste | Con i rimborsi, che sono anche il momento in cui la `GRANT` della `0025` va allargata |
| ~~Un coinquilino non può scoprire l'`user_id` di un altro~~ — **chiuso** dalla migration `0030`, vedi [l'esito](11-piani-conclusi.md#esito-coinquilini-visibili-fetta-1) | Era bloccante nel co-living per stanza: con `lease_type = 'room'` ognuno ha il proprio contratto, `can_access_lease` è per contratto, e l'`user_id` dei coinquilini non compariva da nessuna parte. `GET /properties/{id}/residents` lo restituisce, e il perimetro è la **casa** | Chiuso |
| ~~L'anagrafica è visibile per **organizzazione**, non per immobile~~ — **chiuso** dalla migration `0030` | `users_select` passava da `app.shares_org_with`, quindi un inquilino leggeva nome, email e telefono di ogni persona dell'organizzazione. Ora passa da `app.shares_home_with`, più i due rami sullo staff. Il timore registrato qui — che restringerlo fosse un filtro applicativo e non un confine del database — non si è avverato: è un `ALTER POLICY`, quindi il confine è dove doveva stare. Vedi [ADR-036](08-decisioni.md#adr-036) | Chiuso |
| Un job dell'OCR abbandonato resta `running` per sempre | Non c'è scadenza e il ritentativo è fuori perimetro: `attempts` sale alla presa in carico ma nessuno riporta a `queued` un job che il processo ha lasciato a metà. **Il sintomo è muto** — una bolletta che non arriva mai a `pending_review` senza che nulla sia in errore — e finché il ritentativo non esiste il rimedio è guardare `app.ai_jobs` per gli `running` vecchi | Insieme al ritentativo, che è la stessa conversazione del primo fornitore vero: quali errori produce, quali si riprovano, e cosa significa un terminale senza `finished_at` — che è anche la domanda che tiene `cancelled` fuori dall'enum scritto |
| Il worker legge la configurazione dello storage con `std::env::var` | Sei variabili senza una struct che le valida, a differenza dell'API che passa da `AppConfig`. È la stessa scelta del migratore e la stessa ragione — `AppConfig` gli chiederebbe un bind e delle chiavi di firma che non usa — ma vuol dire che il test di guardia `docs_config_reference.rs` **non copre le impostazioni del worker**: la tabella in `docs/07-operazioni.md` può restare indietro senza che nulla fallisca | Se il worker acquisisce altra configurazione. Con sei variabili il rischio è basso; con dodici la guardia va estesa, ed è un parser in più nello stesso test |
| Il refresh non è coordinato **fra schede** | Il singolo volo è una promessa in memoria, e due schede sono due contesti JavaScript: se l'access token di entrambe scade nello stesso istante, la seconda presenta un refresh token già ruotato e l'API revoca la catena. Serve `BroadcastChannel` o un lock su `localStorage`. Il sintomo è raro e il rimedio è ricaricare | `B10` della [fetta di manutenzione](#manutenzione-del-client-b9-e-b10), insieme alla divisione del bundle. È la sola parte della decisione 5 del piano del client che resta aperta, ed è deliberatamente accettata |
| ~~Non esiste un endpoint che elenchi le **organizzazioni** di un utente~~ — **chiuso** dalla fetta B | `GET /api/v1/organizations` restituisce `id`, `name`, `slug` e `role`, ed è **l'unica rotta che risponde senza un'organizzazione attiva**: chiederne una per sapere quali si hanno sarebbe circolare. La previsione era «un `GET /memberships` di poche righe, e la domanda vera è quali colonne esporre»: il percorso è `/organizations` senza prefisso `me` — la Row Level Security restringe già alle proprie — e le colonne sono quattro. Il `localStorage` si è ridotto a ricordare *quale* era attiva. Vedi [ADR-041](08-decisioni.md#adr-041) | Chiuso |
| Il bundle del client è un pezzo solo, **648 kB (193 kB compressi)** | Nessuna divisione per rotta: chi apre il riepilogo scarica anche le schermate delle bollette e delle segnalazioni. Era 597 kB alla fetta 1, 614 alla A, 648 ora. Su desktop non si nota, e il proprietario è su desktop; **l'inquilino è su rete mobile**, e ora esiste davvero — la fetta B lo ha portato dentro senza affrontare questo. `React.lazy` per rotta è la via, e costa dei confini di `Suspense` che vanno progettati insieme agli stati di caricamento che già esistono | `B10` della [fetta di manutenzione](#manutenzione-del-client-b9-e-b10), che è uscita dalle spese co-living apposta: il suo criterio di completamento non ha niente in comune con «i saldi sommano a zero», e una fetta con due criteri scollegati si taglia a metà mentre la si esegue |
| Gli allegati si scaricano da un link senza `Authorization` | `GET /documents/{id}/content` risponde `307` verso un URL pre-firmato, e il client ci arriva con un `<a href>` — che è ciò che permette al browser di scaricare invece di tenere i byte in memoria. Il link però non porta il token, quindi funziona solo finché la rotta accetta la richiesta dal contesto della scheda. Oggi funziona; il giorno in cui l'API pretendesse l'header su quella rotta, il download smetterebbe di funzionare **senza che nulla in questo repository cambi** | Se il download smette di funzionare, o quando si decida come firmare i link lato client. L'alternativa è un `fetch` con il token seguito da un `blob:` URL, che tiene il file in memoria — accettabile per un PDF, non per una scansione da 25 MB |
| `actions/checkout@v4` gira su Node 20 | I runner lo forzano già su Node 24 con un avviso di deprecazione. Non rompe oggi, romperà quando GitHub toglierà il ponte | Al prossimo intervento sui workflow, o quando l'avviso diventa errore |
| La coda delle dichiarazioni non si filtra per immobile | `GET /api/v1/payments` ha `status` e `lease_id`, e non `property_id`: `app.payments` non ha quella colonna e il legame passerebbe da due salti (`payment → charge → lease → property`). Con pochi immobili la coda sta in una pagina e il filtro non compra niente; con decine diventa lunga | Quando gli immobili per organizzazione passano la decina, o se qualcuno chiede «i pagamenti di questa casa» |
| Un filtro parametrico su un enum non usa un indice parziale | Misurato, non dedotto: `($1::text IS NULL OR status = $1::app.payment_status)` **non** tocca `payments_pending_ix`, perché il cast `text → enum` passa da `enum_in`, che PostgreSQL dichiara `STABLE` e non `IMMUTABLE`, e non viene quindi ridotto a una costante. Il rimedio è nel repository dei pagamenti — una condizione ridondante che nomina il letterale — ed è **locale a quella query**. Ogni altro elenco del progetto usa lo stesso pattern parametrico su uno stato, e nessuno di quelli ha oggi un indice parziale da mancare | Quando un altro elenco filtrato per stato acquisisce un indice parziale. La misura e il rimedio stanno in [ADR-040](08-decisioni.md#adr-040) |
| Un `.env` più vecchio dello storage impedisce l'avvio dell'API | `.env.example` ha quattro variabili `PROPFLOW__STORAGE__*` che un file copiato prima della fetta dei documenti non ha, e l'API si ferma all'avvio con `missing field access_key_id for key "default.storage"`. È la validazione della configurazione che fa il suo mestiere, ma il messaggio non dice **da dove** debba arrivare la variabile né che `.env.example` la porti | **Incontrato una seconda volta** nella fetta B, avviando l'API per il giro a mano: si aggira passando le quattro variabili sulla riga di comando. Due occorrenze sono abbastanza per dire che il messaggio va corretto. Il rimedio non è un default — uno storage configurato a caso è peggio di un avvio fallito — ma un messaggio che nomini `.env.example` |

## Prossimo passo suggerito

**La fetta B-bis: le spese co-living** — il piano è più sotto, pianificato il 7
settembre 2026 **leggendo il codice che tocca**, e il passo che lo apre è `B6`, i
gruppi. È l'ultimo modulo dell'MVP che ha lo schema e l'API e non ha un'interfaccia, ed
è il primo in cui il denaro non passa dal proprietario.

**Le fette A e B sono concluse.** Il giro del denaro dal lato del proprietario è
chiuso e l'insoluto scende; l'altra metà del prodotto esiste, e l'inquilino entra,
vede che cosa deve, **dichiara di aver pagato**, segnala un guasto e ne segue il
corso. I resoconti sono in fondo all'archivio:
[fetta A](11-piani-conclusi.md#piano-completare-il-prodotto)
· [esito](11-piani-conclusi.md#esito-completare-il-prodotto-fetta-a) ·
[fetta B](11-piani-conclusi.md#piano-inquilino-fetta-b)
· [esito](11-piani-conclusi.md#esito-inquilino-fetta-b).

**Le notifiche vanno in fondo, e non sono più una fetta A-bis.** Erano i passi 9 e
10 del piano archiviato, e la scelta era «prima o dopo la B». È stata sciolta: *dopo*,
insieme a tutto il resto che serve solo a utenti veri. La ragione è nel criterio qui
sotto.

**Il co-living è ora provabile con dati veri**, e non lo era prima: un gruppo di spesa
ha senso fra coinquilini, e finché l'inquilino non aveva un'interfaccia si potevano
creare gruppi con `curl` e guardarli con `curl`. Ora ci sono due account che entrano
da un browser e vedono cose diverse, che è la condizione per accorgersi che un saldo è
sbagliato.

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
[l'esito](11-piani-conclusi.md#esito-coinquilini-visibili-fetta-1). L'**OCR delle bollette** è stato fatto:
vedi [l'esito](11-piani-conclusi.md#esito-ocr-bollette-fetta-1) — con l'avvertenza che quella fetta ha
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

### La decisione che ora blocca più valore di ogni riga di codice

**Scegliere il fornitore di OCR.** Non è un compito di sviluppo e non va messo in coda
con quelli: la porta c'è, l'adattatore deterministico percorre tutto il tragitto, il
worker gira. Quello che manca è una decisione sul **trattamento dei dati personali** —
l'immagine di una bolletta porta fuori dal perimetro fornitore, indirizzo, importi e
spesso il codice cliente — e spetta a chi risponde del trattamento.

Quando sarà presa, il lavoro tecnico è un adattatore, ed è il momento in cui il piano
chiede di fare altre due cose che oggi sarebbero premature:

1. **rileggere la porta `OcrProvider`** con un fornitore vero davanti. È scritto nel
   codice della porta perché chi lo scriverà lo trovi;
2. **rifarsi la domanda sui due binari.** Oggi i due cicli del worker stanno in un
   processo solo perché il carico dell'OCR non esiste; con un fornitore vero un job
   lungo può affamare il dispatcher dell'outbox. Il ciclo sta dietro `ProcessAiJob`, che
   dipende solo da porte, quindi separarlo è un refactor e non un ridisegno.

Sono la stessa conversazione, e vale la pena affrontarle insieme.

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

## Piano — Completare il prodotto: che cosa resta

Le fette A e B sono concluse. I piani che le hanno prodotte sono archiviati per intero
in [11 — Piani conclusi](11-piani-conclusi.md), ognuno con il suo esito
([fetta A](11-piani-conclusi.md#piano-completare-il-prodotto),
[fetta B](11-piani-conclusi.md#piano-inquilino-fetta-b)), e **le loro decisioni valgono
ancora**: quel documento resta la fonte del *perché*, e questo del *cosa*.

Le due liste si sovrappongono di proposito. L'archivio non si riscrive mai, quindi
contiene le fette **come erano state pensate** — comprese le notifiche nella posizione
che avevano allora, e i dodici passi della fetta B di cui se ne sono eseguiti sei;
questo file si riscrive a ogni fetta, e contiene ciò che resta con il dettaglio per
eseguirlo. Chi legge solo questo può eseguire; chi vuole sapere perché va di là.

**L'ordine qui è cambiato rispetto ai piani archiviati, ed è la sola differenza di
sostanza.** Là le notifiche erano una «fetta A-bis» da fare prima o dopo la B a
scelta, e la fetta C si chiamava così. Qui sono confluite in un'unica fase finale —
[Prima degli utenti veri](#prima-degli-utenti-veri) — per il
[criterio d'ordine](#il-criterio-dordine-finché-a-provare-è-chi-costruisce) più sopra.
La scomposizione dei due passi delle notifiche non è cambiata di una riga: è cambiato
soltanto **quando**.

**La fetta B si è fermata al suo punto di taglio dichiarato**, `B5`, e non per
stanchezza: il taglio era scritto nel piano prima di cominciare, e lì l'inquilino ha
un prodotto che sta in piedi. Ciò che restava — le spese co-living e gli ultimi due
passi del client — riprende qui **con la numerazione originale**, perché è quella con
cui è stato pensato e perché l'archivio vi rimanda.

Restano quindi **tre fette**:

| | Fetta | Stato |
|---|---|---|
| 1 | [B-bis](#fetta-b-bis--le-spese-co-living) — `B6`–`B8`, le spese co-living: l'ultimo modulo dell'MVP con lo schema e senza interfaccia | la prossima |
| 2 | [Manutenzione del client](#manutenzione-del-client-b9-e-b10) — `B9` e `B10`, allegati e i due debiti | quando pesano, o prima degli utenti veri |
| 3 | [Prima degli utenti veri](#prima-degli-utenti-veri) — notifiche, poi il resto | quando il prodotto lo apre qualcun altro |

### Fetta B-bis — le spese co-living

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
[Manutenzione del client](#manutenzione-del-client-b9-e-b10). Non è un rinvio per
stanchezza: sono lavoro di manutenzione e non una capacità, il loro criterio di
completamento non ha niente a che vedere con quello delle spese, e tenerli insieme
rifarebbe l'errore che la fetta B ha dovuto correggere a metà dichiarando un taglio.

#### Obiettivo

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

#### Che cosa è stato deciso prima di pianificare

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

#### Decisioni prese, con il perché

##### 1. I nomi si scrivono all'ingresso, perché dopo non si recuperano

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

##### 2. Uscire da un gruppo lo fa sparire, e va detto **prima**

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

##### 3. La voce *Spese* è solo dell'inquilino

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

##### 4. Il netto in cima a *Spese*, e non un contatore in navigazione

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

##### 5. Si mostrano i netti, e non si calcola chi paga chi

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

##### 6. La ripartizione si sceglie per spesa

`default_split_method` sul gruppo è **una consuetudine, non una regola**: il DTO lo dice,
e ogni spesa può dichiararne una diversa. Il form della spesa propone quindi il default
del gruppo e lo lascia cambiare, invece di fissarlo alla creazione.

`equal` e `weighted` sono le due ammesse. **`percentage`, `exact` e `by_usage` compaiono
nell'enum e danno `422`**: il menù si scrive a mano con due voci, come i filtri di stato
delle segnalazioni si scrivono a mano per non offrire `resolved`.

##### 7. Il punto di taglio è dopo `B8`

Le spese sono una capacità e finiscono quando i saldi tornano a zero. `B9` e `B10` sono
manutenzione: allegati su owner che già esistono, e due debiti registrati da tre fette.
Il loro criterio di completamento — «un allegato si carica anche da qui», «il bundle è
diviso» — non ha niente in comune con «i saldi sommano a zero», e una fetta con due
criteri scollegati è una fetta che si taglia a metà mentre la si esegue. È successo
nella fetta B ed è stato gestito dichiarando il taglio prima; qui il taglio è dichiarato
non facendo entrare le due cose.

#### Decomposizione

##### `B6` — Client: i gruppi, e i nomi che ci entrano dentro

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

##### `B7` — Client: le spese e i saldi

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

##### `B8` — Client: i regolamenti a due tempi ← *punto di taglio*

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

#### Trappole

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

#### Come si saprà che è finita

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

<a id="manutenzione-del-client-b9-e-b10"></a>

### Manutenzione del client — `B9` e `B10`

Usciti dalla fetta B-bis con la decisione 7: sono manutenzione e non una capacità, e il
loro criterio non ha niente in comune con quello delle spese. Restano numerati come nel
piano della fetta B perché è così che l'archivio vi rimanda.

#### `B9` — Gli allegati sugli owner rimasti

Immobile, stanza, contratto, segnalazione, addebito — e l'archiviazione di un documento,
che è l'unica rotta dei documenti che nessuno chiama. Il caricamento sulle bollette
esiste dalla fetta 1 e ne è il modello, `ScansCard` compreso: il controllo della
dimensione **prima** di inviare, perché oltre il limite la risposta è un `413` che non
dice quale limite né su quale file.

Vale la trappola già registrata fra i debiti: **il link di download non porta
l'`Authorization`**, e funziona perché il browser lo segue nel contesto della scheda.

#### `B10` — I due debiti che l'inquilino incontra

**La divisione del bundle per rotta**, con i `Suspense` progettati: i tre stati esistono
già come componenti, e i confini vanno messi dove quelli sono, non dove capita — un
confine senza il suo stato produce una pagina bianca dove prima c'era uno scheletro. Il
bundle era 597 kB alla fetta 1, 614 alla A, 648 alla B, e crescerà ancora con le spese.

**Il coordinamento del refresh fra schede.** Il singolo volo è una promessa in memoria e
due schede sono due contesti JavaScript: se l'access token di entrambe scade nello stesso
istante, la seconda presenta un refresh token già ruotato e l'API revoca la catena. Serve
`BroadcastChannel` o un lock su `localStorage`. È rimandato tre volte, ogni volta con la
sua ragione, e il sintomo resta raro.

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

### Fuori, e non per dimenticanza

- **il fornitore di OCR.** Resta la decisione che blocca più valore di ogni riga di
  codice, ed è una decisione sul trattamento dei dati personali: non è un compito di
  sviluppo e non va messa in coda con quelli. Vedi
  [ADR-037](08-decisioni.md#adr-037) e la sezione più sopra;
- **il triage AI delle segnalazioni**, che dipende dalla stessa scelta e in più ha una
  domanda di prodotto sua — con quale soglia di confidenza una macchina possa scrivere
  la categoria di una segnalazione;
- **la traduzione in inglese.** Il catalogo esiste dal primo giorno, quindi aggiungere
  una lingua è un file in più e non una riscrittura: è una decisione di mercato, non
  di completezza, e non blocca nulla;
- **il mobile nativo**, che [ADR-039](08-decisioni.md#adr-039) rimanda alla
  conversazione sulle notifiche push — cioè a **dopo che `Notifier` esiste**, e quindi
  a dopo [Prima degli utenti veri](#prima-degli-utenti-veri);
- **la ricerca full-text**, che è V2 e va alimentata dall'outbox;
- **il filtro sui crediti da sovrapagamento.** Rinviato con la sua ragione in
  [ADR-040](08-decisioni.md#adr-040): richiederebbe un parametro nuovo e un indice
  parziale che lo regga, e il caso d'uso «mostrami tutti i crediti» non è ancora nato.
