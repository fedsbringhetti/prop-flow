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

423 test: 206 unitari di dominio, 16 applicativi, 27 di infrastruttura, 11 unitari
del bordo HTTP, 76 di integrazione sulla persistenza, 3 sull'adattatore S3 contro
MinIO, 81 di contratto HTTP, 3 di guardia sulla documentazione della configurazione.

### Client web — il percorso del proprietario

`apps/web/`, React e TypeScript in `strict`, servito dalla **stessa origine**
dell'API dietro un reverse proxy.

| Componente | Stato |
|---|---|
| Catalogo delle stringhe, formattazione con `Intl` e la lingua come parametro | completo |
| Aritmetica decimale su interi: `parseFloat` non compare | completo |
| Client dell'API con il **refresh a singolo volo** e la traduzione di `ProblemDetails` | completo |
| Guscio: registrazione, accesso, uscita, organizzazione attiva, navigazione | completo |
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

37 test: 12 sulla sessione e sul client dell'API — fra cui il refresh a singolo
volo **riprodotto anche al contrario** — 11 sui decimali e sulla formattazione, 5
sulla chiave di idempotenza, 9 sui percorsi da sola tastiera e sui form che non
perdono ciò che è stato digitato.

Il test sulla chiave di idempotenza è stato **verificato contro il difetto**:
sostituendo `intent.current()` con un `randomUUID()` dentro `mutationFn` fallisce. Un
test che non fallisce quando il codice è sbagliato è un verde che non prova nulla.

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
| Prodotto | il percorso dell'**inquilino** nel client, che è la fetta 2. Il client esiste e copre il proprietario: [ADR-039](08-decisioni.md#adr-039) rivede l'ipotesi «mobile nativo» che questa riga conteneva |

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
| Il refresh non è coordinato **fra schede** | Il singolo volo è una promessa in memoria, e due schede sono due contesti JavaScript: se l'access token di entrambe scade nello stesso istante, la seconda presenta un refresh token già ruotato e l'API revoca la catena. Serve `BroadcastChannel` o un lock su `localStorage`. Il sintomo è raro e il rimedio è ricaricare | Se qualcuno lo incontra davvero. È la sola parte della decisione 5 del piano del client che resta aperta, ed è deliberatamente accettata |
| Non esiste un endpoint che elenchi le **organizzazioni** di un utente | Il selettore dell'organizzazione mostra quelle che *questo browser* ha conosciuto — dal login e dall'accettazione di un invito — e le tiene in `localStorage`. Per il proprietario, che ne ha una, è indistinguibile da un elenco vero; per l'inquilino della fetta 2, che ne ha almeno due, non lo sarà: entrando su un browser nuovo non vedrebbe l'organizzazione del proprietario finché non riaccetta un invito. È il gemello del debito «non esiste un endpoint che elenchi le **persone** di un'organizzazione», ed è emerso allo stesso modo — costruendo il client | Con la fetta 2. È un `GET /api/v1/memberships` di poche righe, e la domanda vera è quali colonne esporre: il nome dell'organizzazione è un dato che oggi nessun endpoint restituisce a chi non ne è staff |
| Il bundle del client è un pezzo solo, 597 kB (181 kB compressi) | Nessuna divisione per rotta: chi apre il riepilogo scarica anche le schermate delle bollette. Su desktop non si nota, e il proprietario è su desktop; l'inquilino della fetta 2 è su rete mobile e lo noterà. `React.lazy` per rotta è la via, e costa dei confini di `Suspense` che vanno progettati insieme agli stati di caricamento che già esistono | Con la fetta 2, insieme al percorso dell'inquilino |
| Gli allegati si scaricano da un link senza `Authorization` | `GET /documents/{id}/content` risponde `307` verso un URL pre-firmato, e il client ci arriva con un `<a href>` — che è ciò che permette al browser di scaricare invece di tenere i byte in memoria. Il link però non porta il token, quindi funziona solo finché la rotta accetta la richiesta dal contesto della scheda. Oggi funziona; il giorno in cui l'API pretendesse l'header su quella rotta, il download smetterebbe di funzionare **senza che nulla in questo repository cambi** | Se il download smette di funzionare, o quando si decida come firmare i link lato client. L'alternativa è un `fetch` con il token seguito da un `blob:` URL, che tiene il file in memoria — accettabile per un PDF, non per una scansione da 25 MB |
| `actions/checkout@v4` gira su Node 20 | I runner lo forzano già su Node 24 con un avviso di deprecazione. Non rompe oggi, romperà quando GitHub toglierà il ponte | Al prossimo intervento sui workflow, o quando l'avviso diventa errore |
| La coda delle dichiarazioni non si filtra per immobile | `GET /api/v1/payments` ha `status` e `lease_id`, e non `property_id`: `app.payments` non ha quella colonna e il legame passerebbe da due salti (`payment → charge → lease → property`). Con pochi immobili la coda sta in una pagina e il filtro non compra niente; con decine diventa lunga | Quando gli immobili per organizzazione passano la decina, o se qualcuno chiede «i pagamenti di questa casa» |
| Un filtro parametrico su un enum non usa un indice parziale | Misurato, non dedotto: `($1::text IS NULL OR status = $1::app.payment_status)` **non** tocca `payments_pending_ix`, perché il cast `text → enum` passa da `enum_in`, che PostgreSQL dichiara `STABLE` e non `IMMUTABLE`, e non viene quindi ridotto a una costante. Il rimedio è nel repository dei pagamenti — una condizione ridondante che nomina il letterale — ed è **locale a quella query**. Ogni altro elenco del progetto usa lo stesso pattern parametrico su uno stato, e nessuno di quelli ha oggi un indice parziale da mancare | Quando un altro elenco filtrato per stato acquisisce un indice parziale. La misura e il rimedio stanno in [ADR-040](08-decisioni.md#adr-040) |
| Un `.env` più vecchio dello storage impedisce l'avvio dell'API | `.env.example` ha quattro variabili `PROPFLOW__STORAGE__*` che un file copiato prima della fetta dei documenti non ha, e l'API si ferma all'avvio con `missing field access_key_id for key "default.storage"`. È la validazione della configurazione che fa il suo mestiere, ma il messaggio non dice **da dove** debba arrivare la variabile né che `.env.example` la porti | Quando qualcuno lo incontra. Il rimedio non è un default — uno storage configurato a caso è peggio di un avvio fallito — ma un messaggio che nomini `.env.example` |

## Prossimo passo suggerito

**La fetta B: l'inquilino** — il piano è più sotto, e il passo che lo apre è `B0`,
l'endpoint delle organizzazioni.

**La fetta A è conclusa**: il giro del denaro dal lato del proprietario è chiuso, e
l'insoluto adesso scende. Il resoconto è in fondo all'archivio:
[Piano — Completare il prodotto, fetta A](11-piani-conclusi.md#piano-completare-il-prodotto)
· [esito](11-piani-conclusi.md#esito-completare-il-prodotto-fetta-a).

**Le notifiche vanno in fondo, e non sono più una fetta A-bis.** Erano i passi 9 e
10 del piano archiviato, e la scelta era «prima o dopo la B». È stata sciolta: *dopo*,
insieme a tutto il resto che serve solo a utenti veri. La ragione è nel criterio qui
sotto.

Il percorso dell'inquilino è **provabile con dati veri per la prima volta**, perché un
inquilino non esiste finché un proprietario non lo invita — e ora c'è anche un
proprietario che sa incassare.

Le **due semantiche della dashboard** che erano rimaste aperte **sono state decise**:
`written_off_total` è il nono numero, ed è il residuo e non l'importo; il credito da
sovrapagamento resta fuori dal riepilogo e si evidenzia per addebito. Vedi
[ADR-040](08-decisioni.md#adr-040).

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

La fetta A è conclusa. Il piano che l'aveva prodotta è archiviato per intero in
[11 — Piani conclusi](11-piani-conclusi.md#piano-completare-il-prodotto), con il suo
esito, e **le sue otto decisioni valgono ancora**: quel documento resta la fonte del
*perché*, e questo del *cosa*.

Le due liste si sovrappongono di proposito. L'archivio non si riscrive mai, quindi
contiene anche le fette B e C **come erano state pensate**, comprese le notifiche
nella posizione che avevano allora; questo file si riscrive a ogni fetta, e contiene
ciò che resta con il dettaglio per eseguirlo. Chi legge solo questo può eseguire; chi
vuole sapere perché va di là.

**L'ordine qui è cambiato rispetto al piano archiviato, ed è la sola differenza di
sostanza.** Là le notifiche erano una «fetta A-bis» da fare prima o dopo la B a
scelta, e la fetta C si chiamava così. Qui sono confluite in un'unica fase finale —
[Prima degli utenti veri](#prima-degli-utenti-veri) — per il
[criterio d'ordine](#il-criterio-dordine-finché-a-provare-è-chi-costruisce) più sopra.
La scomposizione dei due passi delle notifiche non è cambiata di una riga: è cambiato
soltanto **quando**.

Restano quindi **due fette**:

| | Fetta | Stato |
|---|---|---|
| 1 | [L'inquilino](#fetta-b--linquilino) — dodici passi, punto di taglio dopo `B5` | la prossima |
| 2 | [Prima degli utenti veri](#prima-degli-utenti-veri) — notifiche, poi il resto | quando il prodotto lo apre qualcun altro |

### Fetta B — l'inquilino

Pianificata il **4 settembre 2026**, dopo la fetta A e leggendo il codice che tocca.
**Dodici passi**, `B0`–`B11`. Il **punto di taglio è dopo `B5`**: lì l'inquilino ha un
prodotto che sta in piedi — entra, vede che cosa deve, **dichiara di aver pagato**,
segnala un guasto e ne segue il corso — e le spese co-living sono una fetta B-bis.

#### Obiettivo

L'altra metà del prodotto. Oggi il client copre il proprietario e basta: un inquilino
che entrasse **atterrerebbe su un errore**, perché la pagina iniziale è la dashboard e
la dashboard è staff soltanto.

Il perimetro è la differenza fra le due metà, non il modulo: le segnalazioni sono
dell'inquilino *e* del proprietario, e vanno costruite per entrambi in una volta —
sono la stessa schermata vista da due ruoli, e farne due sarebbe farne due da tenere
allineate.

#### Che cosa è stato deciso prima di pianificare

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

#### Decisioni prese, con il perché

##### 1. `B0` è `ensure_authenticated`, e non `ensure_member`

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

##### 2. La risposta porta il **nome**, ed è il punto dell'endpoint

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

##### 3. L'assegnazione è **solo a sé stessi**, e non è una semplificazione

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

##### 4. L'inquilino atterra sullo scadenzario, e non su una dashboard sua

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

##### 5. Il guscio si adatta al ruolo, ed è un passo suo

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

##### 6. La composizione dei gruppi passa dai **residenti dell'immobile**

`AddGroupMemberCommand` vuole un `UserId`, e questa volta l'elenco esiste:
`GET /api/v1/properties/{id}/residents` restituisce `user_id` e `full_name` per la
**casa**, ed è esattamente ciò per cui la fetta «i coinquilini si vedono» e la
migration `0030` esistono. È il ritorno di un investimento fatto due fette fa.

Ne segue la sua trappola, ed è la stessa forma di `payer_user_id` nella fetta A: un
residente **senza `user_id` non ha ancora accettato l'invito**, e non è aggiungibile a
un gruppo — la policy confronterebbe un identificatore che non esiste. Va mostrato
come non selezionabile con la ragione, non nascosto: nasconderlo farebbe cercare una
persona che sullo schermo c'è.

#### Decomposizione

##### `B0` — Backend: `GET /api/v1/organizations`

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

##### `B1` — Client: il selettore letto dall'API

`getOrganizations` nel livello dati, la query nel guscio, i nomi al posto degli UUID.

Il `localStorage` **resta**, ma per una cosa sola: ricordare **quale** era attiva. È
una preferenza di quel browser, non un elenco — e la distinzione è ciò che il commento
in `session.tsx` già dichiara e che questo passo rende vera.

I tre stati vanno progettati anche qui: un selettore che sta caricando non deve
mostrare un elenco vuoto, perché un elenco vuoto è indistinguibile da «non appartieni
a niente».

##### `B2` — Client: il guscio si adatta al ruolo

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

##### `B3` — Client: segnalazioni, la lettura

Apertura su un immobile (`POST /properties/{id}/tickets`, `ensure_member` — è **il**
punto in cui l'inquilino prende la parola), elenco con `?open=true`, dettaglio con lo
storico degli stati, thread dei messaggi **paginato a parte** dal dettaglio.

##### `B4` — Client: segnalazioni, le transizioni

Le sette sotto-risorse, con i percorsi esatti perché uno di essi non si chiama come il
suo use case:

`/triage` · `/assign` · `/start` · **`/wait`** · `/close` · `/reopen` · `/cancel`

Più il `PATCH /tickets/{id}`, che è **solo i costi** ed è staff.

L'instradamento è **sui codici e non sul ruolo** — vedi le trappole.

##### `B5` — Client: la dichiarazione di pagamento ← *punto di taglio*

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

##### `B6` — Client: spese co-living, i gruppi

Creazione, composizione dai residenti (decisione 6), ingresso e uscita,
archiviazione. Da qui compare la voce *Spese* nella navigazione.

##### `B7` — Client: spese e saldi

Ripartizione `equal` e `weighted`, quote mostrate, saldi con il netto.

##### `B8` — Client: i regolamenti a due tempi

Dichiarazione, conferma di **chi riceve**, rifiuto, e il **ritiro di chi ha
dichiarato** — che fra i pagamenti non esiste. Vedi le trappole: le due cose si
somigliano e non sono la stessa.

##### `B9` — Client: gli allegati sugli owner rimasti

Immobile, stanza, contratto, segnalazione, addebito — e l'archiviazione di un
documento, che è l'unica rotta dei documenti che nessuno chiama.

##### `B10` — Client: i due debiti che l'inquilino incontra

Divisione del bundle per rotta con i `Suspense` progettati, e il coordinamento del
refresh fra schede. Entrambi registrati fra i debiti noti con il loro perché.

##### `B11` — Documentazione ed esito

ADR per ciò che sopravvive alla fetta, aggiornamento di [05 — API](05-api.md), esito,
e **il piano si sposta in [11 — Piani conclusi](11-piani-conclusi.md) nello stesso
commit**.

#### Trappole

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

#### Come si saprà che è finita

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
