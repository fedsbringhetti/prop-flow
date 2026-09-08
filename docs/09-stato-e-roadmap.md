# 09 — Stato e roadmap

Stato al **8 settembre 2026**.

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
| **Spese co-living**: elenco dei gruppi con il **proprio netto complessivo** in cima, scheda del gruppo con saldi, spese paginate, quote lette e non ricalcolate, composizione con uscita e rientro, archiviazione | completo per la fetta B-bis |
| **Rimborsi a due tempi**: dichiarazione, conferma e rifiuto di chi riceve, **ritiro di chi ha dichiarato** — che qui esiste e fra i pagamenti no | completo per la fetta B-bis |
| **Il nome di un membro scritto all'ingresso**, perché nessuna risposta di quel modulo ne porta uno e chi lascia la casa non è più fra i residenti ([ADR-043](08-decisioni.md#adr-043)) | completo per la fetta B-bis |
| **L'invitato entra dall'interfaccia**: `/invito`, un campo in cui incollare il codice, e l'organizzazione appena accettata diventa quella attiva ([ADR-045](08-decisioni.md#adr-045)) | completo per la fetta C |
| **La revoca di un invito**, nella modale che l'ha creato e da nessun'altra parte: nessuna rotta li elenca | completo per la fetta C |
| **Il rinnovo è serializzato fra le schede**: lock per origine sopra il singolo volo, e il token non lascia la memoria ([ADR-044](08-decisioni.md#adr-044)) | completo per la fetta C |
| **Allegati su sei owner** da un componente solo: immobile, stanza, contratto, segnalazione, bolletta, addebito — con il limite dei 25 MiB controllato prima di inviare e l'archiviazione offerta a chi la policy ammette | completo per la fetta C |
| **Le stanze si correggono e si archiviano**, da una modale che è la loro unica superficie: `GET /rooms/{id}` non esiste | completo per la fetta C |
| **Il bundle è diviso per rotta**, con i confini di `Suspense` dentro il guscio e con il loro stato | completo per la fetta C |
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

69 test: 16 sulla sessione e sul client dell'API — fra cui il refresh a singolo
volo **e quello fra schede, entrambi riprodotti anche al contrario** — 11 sui
decimali e sulla formattazione, 5
sulla chiave di idempotenza, 4 sull'accettazione di un invito — l'header che **non**
si manda, l'organizzazione che si sceglie, e il messaggio che non parla di password —
9 sui percorsi da sola tastiera e sui form che non
perdono ciò che è stato digitato, 6 sulla dichiarazione di pagamento, 5 sul guscio —
la navigazione per ruolo, il selettore che mostra i nomi, e l'organizzazione
ricordata che l'elenco vero corregge — e 13 sulle spese co-living: la risoluzione dei
nomi **compreso chi non è più residente**, la somma dei netti verificata sugli interi, e
la scheda del gruppo con un membro uscito che ha ancora un conto aperto.

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

**E dalla fetta B-bis ha anche l'interfaccia**, senza che il backend sia stato toccato:
gruppi, saldi, spese con le loro quote, e i rimborsi a due tempi con il ritiro. La
decisione che è servita a costruirla è [ADR-043](08-decisioni.md#adr-043) — nessuna
risposta di quel modulo porta un nome, quindi il client lo scrive all'ingresso — e
l'esito è
[Piano — Le spese co-living, fetta B-bis](11-piani-conclusi.md#piano-spese-co-living-fetta-b-bis).

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
| Prodotto | **niente**, per la prima volta. Le spese co-living erano l'ultima voce di questa riga e sono entrate con la fetta B-bis: l'inquilino entra, vede che cosa deve al proprietario e che cosa deve ai coinquilini, dichiara di aver pagato gli uni e gli altri, e segnala un guasto. Ciò che resta è manutenzione (`B9`, `B10`) e ciò che serve a utenti veri. [ADR-039](08-decisioni.md#adr-039) rivede l'ipotesi «mobile nativo» che questa riga conteneva |

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
| L'organizzazione dell'invito non diventa mai predefinita — **mezzo chiuso** dalla fetta C, `C1` | `pick_default_membership` preferisce il ruolo più alto e la registrazione crea sempre un'organizzazione personale di cui si è owner, quindi un inquilino registrato da sé atterrava **sempre** nella propria organizzazione vuota. Ora **accettare un invito sceglie l'organizzazione appena accettata**, che è l'unico momento in cui il client sa quale si vuole. La metà che resta è deliberata: chi entra da un browser nuovo, mesi dopo, riparte dal default di sempre, perché il ricordo vive in `localStorage`. Cambiare `pick_default_membership` deciderebbe **per tutti** quale appartenenza vinca, e il caso legittimo esiste — chi affitta una casa e ne abita un'altra. Vedi [ADR-045](08-decisioni.md#adr-045) | La metà che conta è chiusa: è la **prima volta** a far dire «non funziona» |
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
| ~~Il refresh non è coordinato **fra schede**~~ — **chiuso** dalla fetta C, `C2` | Era il difetto più costoso della lista, ed era **osservato** e non ipotetico: due schede aperte, l'access token scaduto insieme, la seconda che presenta un refresh già ruotato, e l'API che revoca la catena. La previsione era «serve `BroadcastChannel` o un lock su `localStorage`», e la seconda metà è quella giusta — ma la parte che contava non era il meccanismo: era **che cosa** le schede si scambiano. Trasmettere l'access token avrebbe funzionato e avrebbe rotto la proprietà per cui quel token vive in memoria. Si serializza invece: `navigator.locks` con ripiego su un marcatore che scade, e ognuna rinnova per sé presentando il cookie ruotato. Vedi [ADR-044](08-decisioni.md#adr-044) | Chiuso. Il punto 10 del collaudo è la sua prova, e prima della fetta era **certo** che fallisse |
| ~~Non esiste un endpoint che elenchi le **organizzazioni** di un utente~~ — **chiuso** dalla fetta B | `GET /api/v1/organizations` restituisce `id`, `name`, `slug` e `role`, ed è **l'unica rotta che risponde senza un'organizzazione attiva**: chiederne una per sapere quali si hanno sarebbe circolare. La previsione era «un `GET /memberships` di poche righe, e la domanda vera è quali colonne esporre»: il percorso è `/organizations` senza prefisso `me` — la Row Level Security restringe già alle proprie — e le colonne sono quattro. Il `localStorage` si è ridotto a ricordare *quale* era attiva. Vedi [ADR-041](08-decisioni.md#adr-041) | Chiuso |
| ~~Il bundle del client è un pezzo solo~~ — **chiuso** dalla fetta C, `C5`, e ne lascia uno nuovo | Diviso per rotta con `React.lazy`, e i confini di `Suspense` stanno **dentro** il guscio: fuori, ogni cambio di rotta avrebbe smontato navigazione e selettore dell'organizzazione. La rotta iniziale di chi abita passa da 690,6 kB (201,7 gz) a **455,7 (148,3)**, quella di chi amministra a **438,7 (141,6)**. Il totale su tutte le rotte **cresce** — 706,8 kB, 226,2 gz — ed è il baratto giusto, ma va saputo prima che qualcuno lo misuri senza dire di quale rotta | Chiuso. I numeri per rotta sono nell'[esito](11-piani-conclusi.md#esito-fetta-c) |
| **Il login è la rotta iniziale più pesante**, ed è la prima che si vede | Emerso misurando `C5`, e prima era invisibile perché ogni rotta pesava uguale. `AuthPage` valida i form con `zod`, che è un chunk da 107,9 kB (32,5 gz) tirato dentro da nessun'altra rotta iniziale: `/accesso` sta a **543,4 kB (172,4 gz)** contro i 438,7 (141,6) del riepilogo. È il posto peggiore in cui avere il picco — un utente nuovo non ha nulla in cache — e il rimedio non è togliere `zod` dal progetto: è non usarlo **su quei due form**, dove le regole sono un'email, dodici caratteri e un campo obbligatorio. Fuori dalla fetta C perché il perimetro era chiuso, non perché sia difficile | Quando qualcuno di esterno userà il login su rete mobile, o insieme al prossimo intervento sui form di autenticazione |
| Gli allegati si scaricano da un link senza `Authorization` | `GET /documents/{id}/content` risponde `307` verso un URL pre-firmato, e il client ci arriva con un `<a href>` — che è ciò che permette al browser di scaricare invece di tenere i byte in memoria. Il link però non porta il token, quindi funziona solo finché la rotta accetta la richiesta dal contesto della scheda. Oggi funziona; il giorno in cui l'API pretendesse l'header su quella rotta, il download smetterebbe di funzionare **senza che nulla in questo repository cambi** | Se il download smette di funzionare, o quando si decida come firmare i link lato client. L'alternativa è un `fetch` con il token seguito da un `blob:` URL, che tiene il file in memoria — accettabile per un PDF, non per una scansione da 25 MB |
| `actions/checkout@v4` gira su Node 20 | I runner lo forzano già su Node 24 con un avviso di deprecazione. Non rompe oggi, romperà quando GitHub toglierà il ponte | Al prossimo intervento sui workflow, o quando l'avviso diventa errore |
| La coda delle dichiarazioni non si filtra per immobile | `GET /api/v1/payments` ha `status` e `lease_id`, e non `property_id`: `app.payments` non ha quella colonna e il legame passerebbe da due salti (`payment → charge → lease → property`). Con pochi immobili la coda sta in una pagina e il filtro non compra niente; con decine diventa lunga | Quando gli immobili per organizzazione passano la decina, o se qualcuno chiede «i pagamenti di questa casa» |
| Un filtro parametrico su un enum non usa un indice parziale | Misurato, non dedotto: `($1::text IS NULL OR status = $1::app.payment_status)` **non** tocca `payments_pending_ix`, perché il cast `text → enum` passa da `enum_in`, che PostgreSQL dichiara `STABLE` e non `IMMUTABLE`, e non viene quindi ridotto a una costante. Il rimedio è nel repository dei pagamenti — una condizione ridondante che nomina il letterale — ed è **locale a quella query**. Ogni altro elenco del progetto usa lo stesso pattern parametrico su uno stato, e nessuno di quelli ha oggi un indice parziale da mancare | Quando un altro elenco filtrato per stato acquisisce un indice parziale. La misura e il rimedio stanno in [ADR-040](08-decisioni.md#adr-040) |
| ~~Un `.env` più vecchio dello storage impedisce l'avvio dell'API~~ — **chiuso** dalla fetta C, `C6` | L'avvio fallisce ancora, ed è giusto che fallisca: **il rimedio non era un default**, perché uno storage configurato a caso sposta il fallimento al primo caricamento e lo fa somigliare a un problema di rete. È cambiato ciò che si legge: `AppConfig::load` distingue un'impostazione **mancante** da una **sbagliata**, e sulla prima dice che le variabili hanno prefisso `PROPFLOW__`, che l'elenco completo è in `.env.example`, e che nessuno lo carica da solo. Il messaggio di figment — quale campo, sotto quale chiave — resta nella catena delle cause | Chiuso, dopo tre occorrenze in tre fette |

<a id="raccolta-da-ordinare"></a>

## Raccolta da ordinare — **non è un piano**

Cose emerse usando il prodotto e leggendo il codice, messe qui perché non si
perdano. **Non sono decomposte, non sono ordinate e non sono decise**: diventano un
piano quando si sceglie di farne una fetta, con il dettaglio che `CLAUDE.md` §3
richiede. Tenerle qui in forma grezza è meglio che tenerle in una conversazione, ed
è peggio che pianificarle: la differenza è dichiarata perché nessuno le prenda per
lavoro istruito.

Le note tecniche accanto a ciascuna sono **verificate**, non supposte: servono a chi
pianificherà a non ripartire da zero.

### Dal collaudo e dalla lettura del codice

| # | Cosa | Quel che si sa già |
|---|---|---|
| R1 | **Un immobile non esce mai da `draft`** | `PATCH /properties/{id}` accetta `status` (`UpdatePropertyRequest`), e **nessuna schermata lo manda**: né la creazione né `EditPropertyDialog`. Niente lo deriva, quindi l'etichetta resta sbagliata per sempre. **Non blocca nulla** — nessun use case guarda lo stato dell'immobile. Da chiudere è un `<Select>` nel dialogo. Nota di metodo: l'audit della fetta C non poteva trovarlo, perché confronta quali *funzioni* nessuno chiama e `updateProperty` **è** chiamata, solo mai con quel campo |
| R2 | **Il punto 10 del collaudo non prova `C2` come è scritto** | Dice «il proprietario in una finestra, un inquilino nell'altra», e due account richiedono due contenitori di cookie: senza cookie condiviso non c'è contesa, e il punto passerebbe anche con il lock rimosso. La versione che lo esercita è **due schede dello stesso account nello stesso profilo, ricaricate insieme**. Correzione al testo in `11-piani-conclusi.md` |
| R3 | **Il messaggio di `C6` è invecchiato di un giorno** | Dice «nessuno lo carica da solo: lo fa la shell», e il `justfile` ora lo carica con `set dotenv-load`. Due righe in `crates/api/src/config.rs` |
| R4 | **`zod` rende `/accesso` la rotta iniziale più pesante** | 543,4 kB (172,4 gz) contro i 438,7 (141,6) del riepilogo: un chunk da 107,9 kB che nessun'altra rotta iniziale tira, sulla prima schermata che vede un utente nuovo. Già in [Debiti tecnici noti](#debiti-tecnici-noti). Il rimedio è non usarlo **su quei due form**, dove le regole sono un'email, dodici caratteri e un campo obbligatorio |

### Interfaccia e prodotto

| # | Cosa | Quel che si sa già |
|---|---|---|
| R5 | **Mostrare la password**, in accesso e registrazione | `AuthPage.tsx`, i due `Input type="password"`. Il bottone va dentro il campo e deve restare raggiungibile da tastiera; l'etichetta cambia con lo stato e va nel catalogo, non nel JSX |
| R10 | **La creazione di una stanza offre meno campi della modifica** | `NewRoomDialog` chiede nome, canone, superficie e bagno privato; `RoomDialog` — la modale di `C4` — aggiunge *arredata*, e il `POST` ne accetterebbe altri ancora. Chi crea una stanza non ha modo di sapere che il resto esiste, e nemmeno che si può correggere dopo: il nome è unico per immobile, quindi «tanto lo sistemo dopo» è proprio la cosa da dire, e non è detta. Da allineare i due form e a mettere la frase nel primo — la stringa non esiste ancora nel catalogo |
| R6 | **La navigazione non si vede tutta**: su alcune schermate compare solo la metà superiore. Separatore e sezione dell'organizzazione vanno **in basso** | `#navigazione-principale` in `shell.tsx`. Oggi è un `<nav>` con la lista e, sotto, il selettore in un blocco `lg:block`: manca un'altezza piena con il selettore ancorato in fondo (`flex` in colonna, `min-h`, e la lista che scorre). Va guardato **anche a schermo stretto**, dove il menù è a scomparsa e il selettore sta già nella barra in alto |
| R7 | **Un posto per vedere e modificare il proprio account** | Non esiste, e **non esiste nemmeno l'endpoint**: nessuna rotta legge o scrive l'utente corrente. Questo punto è **backend prima che client** — a differenza di tutta la fetta C, che non ha toccato l'API. Da decidere quali campi (nome, email, telefono, password) e che cosa comporta cambiare un'email, che è l'identificatore d'accesso |
| R8 | **Spiegare come si usa il sito** e che cosa fanno le sezioni | Aperto per intero: non è deciso *dove* stia — primo accesso guidato, pagina d'aiuto, testi contestuali sugli stati vuoti, o una combinazione. Nota: gli stati vuoti **esistono già** e sono progettati (`@/components/state`), e alcuni portano già il gesto che li riempie: è il posto più economico dove far entrare una spiegazione, e va valutato prima di costruire una sezione d'aiuto a sé |

### Stati che nessuno aggiorna

| # | Cosa | Quel che si sa già |
|---|---|---|
| R11 | **Una stanza occupata resta «Disponibile»** | Osservato dalla scheda di un immobile: *Chi ci abita* mostra la persona **con la sua stanza**, e la tabella delle stanze accanto dice `available`. La causa non è di presentazione: `RoomStatus::Occupied` esiste nell'enum del dominio e in `app.room_status`, e **nessuno lo scrive mai** — nessun use case, nessun trigger. Il commento di `Room::create` in `crates/domain/src/property/room.rs` dichiara «lo stato passa a `Occupied` quando un contratto la impegna», e quella transizione **non è implementata**: è un invariante scritto e non mantenuto, che è peggio di uno assente perché chi legge il dominio lo dà per vero |

**R11 e [R1](#raccolta-da-ordinare) sono la stessa famiglia** — un campo di stato che l'API accetta e che nessuno aggiorna — ma il rimedio non è lo stesso, e vale la pena non trattarli insieme senza deciderlo.

Su un immobile lo stato è un giudizio di chi amministra, quindi un campo nel form è la risposta giusta. **Su una stanza no**: che sia occupata è un *fatto*, e discende dai contratti. Metterci una tendina significherebbe chiedere a una persona di ricordarsi di aggiornare a mano qualcosa che il database sa già — e `app.property_residents` della migration `0030` lo calcola di fatto, unendo `lease_parties` a `leases` e a `rooms` per contratti attivi.

Le domande da sciogliere prima di scrivere codice, e sono la ragione per cui questa voce non è un rimedio di due righe:

- **derivato o scritto?** Una colonna aggiornata da un trigger sull'attivazione e sulla disdetta di un contratto, oppure uno stato calcolato al momento della lettura, che non può divergere ma non si può filtrare con un indice;
- **che cosa fa la disdetta**, e che cosa fa un contratto futuro: una stanza con una locazione che comincia fra un mese oggi è `available`, e domani?
- **`unavailable` resta manuale**, qualunque cosa si scelga: quello è un giudizio — una stanza in ristrutturazione — e non discende da nessun contratto. Un rimedio che derivi tutto lo cancellerebbe.

### Contratti: creare, disfare, riconoscere

| # | Cosa | Quel che si sa già |
|---|---|---|
| R12 | **Avvertire della sovrapposizione alla creazione**, non solo all'attivazione: se in quel periodo c'è già un contratto, «Crea» chiede una conferma in più | **È una modifica di solo client**, ed è la scoperta che rende la voce piccola. I due vincoli `leases_no_overlap_property` e `leases_no_overlap_room` della `0005` hanno una `WHERE` che copre `pending_signature`, `active` ed `expiring` — **e non `draft`**: le bozze sono escluse *apposta*, perché una bozza è lavoro in corso e due bozze che si sovrappongono sono legittime. Il `409` all'attivazione è quindi corretto e resta; quello che manca è dirlo prima, e il client può saperlo con `listLeases` filtrato sull'immobile confrontando i periodi. **Non toccare i vincoli**: allargarli a `draft` vieterebbe di abbozzare due ipotesi alternative sullo stesso periodo |
| R13 | **Un contratto creato per sbaglio non si elimina**: l'unica via è attivarlo e disdirlo | Le rotte dei contratti sono cinque — `POST /leases`, `GET /leases`, `GET /leases/{id}`, `POST .../parties`, `POST .../activate`, `POST .../terminate` — e **non c'è una `DELETE`**. La colonna `deleted_at` c'è, quindi il livello dati regge già la cancellazione soft: manca lo use case e la rotta. Da decidere **quali stati** siano archiviabili: una bozza certamente; un contratto attivo no, perché per quello la via è la disdetta, che è un fatto diverso e lascia traccia |
| R14 | **Dare un nome ai contratti**, perché in tabella sono tutti uguali | `app.leases` **non ha una colonna nome**: ci sono `notes` e `registration_number`, e l'elenco mostra tipo più data, da cui le righe identiche. Ci sono due vie e vanno pesate insieme, non scelte per prima: (a) una colonna `label`, cioè migration, dominio, DTO e form; (b) comporre l'etichetta da ciò che il contratto già è — immobile e stanza — che **non è gratis**, perché il DTO `Lease` porta `property_id` e `room_id` e **nessun nome**, quindi vorrebbe dire allargare la risposta o fare una richiesta per riga. La (b) risolve «sono tutti uguali»; solo la (a) permette di chiamarlo *«Rossi, secondo piano»* |

| R17 | **Le parti contrattuali non si modificano dopo**, nemmeno da chi amministra | C'è **solo** `POST /leases/{id}/parties`: nessuna `PATCH`, nessuna `DELETE`. Una parte si aggiunge e resta com'è — nome, email, telefono, ruolo, quota, intestatario principale. `app.lease_parties` ha però sia `left_at` sia `deleted_at`, quindi correggere e togliere sono **già rappresentabili** senza migration: mancano gli use case e le rotte |

**R17 e R13 insieme fanno un vicolo cieco, e va visto prima di stimarli separatamente.** L'attivazione pretende che le quote degli inquilini sommino a 100 (`lease.shares_not_hundred`). Se una parte viene inserita con la quota sbagliata: non la si può correggere (R17), quindi il contratto non si attiva; e non lo si può eliminare (R13), perché la sola uscita sarebbe attivarlo e disdirlo — che è precisamente ciò che non riesce. **Il contratto resta lì per sempre**, non attivabile e non rimovibile, e l'unico rimedio è rifarlo daccapo lasciando in elenco quello rotto — che con R14 non ancora fatto è pure indistinguibile dal buono.

È la ragione per cui queste due, se si sceglie di farle, conviene farle nella stessa fetta.

| R18 | **Una quota può finire a zero in silenzio**, e il contratto si attiva lo stesso | Vedi sotto: **osservato in database**, non dedotto |

#### R18 — come si arriva a un contratto attivo con i dati sbagliati

Trovato durante il collaudo e verificato sulle righe vere. Su una stanza, un contratto **attivo** con due parti:

| parte | quota | intestatario | ha accettato |
|---|---|---|---|
| `inq2` | **100,00** | sì | **no** |
| `inq3` | **0,00** | no | sì |

Cioè: **l'inquilino che è entrato davvero non deve niente, e il 100% è di qualcuno che non ha mai accettato.** Il contratto è `active`, quindi ha superato ogni controllo.

**Il backend non è ingenuo, ed è importante saperlo prima di "aggiustarlo" nel posto sbagliato.** `InviteTenant` non mette una quota fissa: chiama `remaining_share(&parties)`, che è `100 − assegnato` con il minimo a zero (`crates/application/src/use_cases/invitation/invite_tenant.rs`). È la scelta giusta — si invita il secondo inquilino e prende ciò che resta.

Il difetto nasce dal **concorso di quattro cose**, e nessuna delle quattro è sbagliata da sola:

1. `AddPartyDialog` propone `share_percent: '100'` e `is_primary: true`. Chi aggiunge la **prima** parte si prende quindi tutto, e nel co-living la prima parte non è l'unica;
2. ogni invito successivo riceve `remaining_share`, che a quel punto è **zero**, e nessuno lo dice: né il form dell'invito, né la modale che consegna il codice, né l'elenco delle parti, che mostra `0,00 %` come un dato qualunque;
3. **l'attivazione verifica la somma, non il senso**: `100 + 0 = 100` passa, e la parte che regge quel 100 può essere una che non ha mai accettato l'invito. `lease.shares_not_hundred` non ha nulla da eccepire;
4. **niente di tutto questo si corregge dopo** ([R17](#raccolta-da-ordinare)): non si cambia una quota, non si toglie una parte.

**Le conseguenze non restano nel contratto.** `app.property_residents` proietta le parti dei contratti attivi, quindi la parte fantasma compare fra **chi abita l'immobile** — con il suo nome, accanto all'inquilino vero. Ed è anche una parte che, se un giorno accettasse, si troverebbe intestataria del 100%.

**Da decidere, e sono domande distinte:** se il form debba proporre 100 solo quando la parte è davvero l'unica; se una quota a zero vada segnalata al momento in cui si invita, invece che scoperta dopo; se l'attivazione debba pretendere qualcosa di più della somma — per esempio che nessun inquilino sia a zero, o che l'intestatario principale abbia accettato. **La correzione a posteriori (R17) serve comunque**, perché i contratti già rotti esistono già.

### Cancellare, in generale

| # | Cosa | Quel che si sa già |
|---|---|---|
| R15 | **Selezionare righe in tabella e cancellarle in blocco**, e poter cancellare anche una sola riga | La parte di interfaccia è ripetibile — le tabelle passano già da `@/components/ui/table` — ma **«elimina» non è un'operazione sola**, ed è la cosa da sapere prima di disegnare un bottone unico: un immobile e una stanza si **archiviano** (`DELETE`, cancellazione soft), un addebito e una bolletta si **annullano** (`void`, che è un fatto contabile e non una cancellazione), un contratto oggi **non si tocca affatto** (R13). Un'azione di gruppo che chiami tutto «elimina» direbbe una cosa falsa su almeno due di questi. Va deciso anche che cosa fa una selezione multipla quando **una parte** fallisce: nessuna rotta accetta più identificatori, quindi sono N chiamate e un esito parziale è possibile |

### Uscire da un'organizzazione

| # | Cosa | Quel che si sa già |
|---|---|---|
| R16 | **Un inquilino deve poter uscire da un'organizzazione**, e l'operazione va negata se ha un contratto attivo | **Non esiste nessuna rotta**: su `/api/v1/organizations` c'è soltanto la `GET` che elenca le proprie. `app.org_members` ha però `status` e `deleted_at`, quindi l'uscita è già rappresentabile senza migration — è lo use case, la rotta e la regola a mancare. Il blocco richiesto è la parte interessante: «ha un contratto attivo» va verificato **e** garantito, e vale la lezione della `0029` sulle spese co-living, dove rendere l'uscita un confine vero ha richiesto una policy e non un `if`. Da decidere anche che cosa vede dopo: chi esce dall'unica organizzazione in cui abita resta con la propria, personale e vuota — cioè il caso di [ADR-045](08-decisioni.md#adr-045) al contrario |

### Il perimetro dei ruoli — **la voce più grande della lista**

| # | Cosa | Quel che si sa già |
|---|---|---|
| R9 | **Separare inquilini e proprietari**: accesso unico, **registrazione libera solo per gli inquilini**; un proprietario si fa verificare fuori dal prodotto e riceve un invito per registrarsi. Un inquilino non deve poter creare immobili | Vedi sotto: non è una fetta di client |

**R9 non si chiude nascondendo un bottone**, e va detto prima che qualcuno ci provi.

L'unico caso d'uso di registrazione che esiste si chiama `RegisterOwner`
(`crates/application/src/use_cases/auth/register_owner.rs`) e crea **sempre**
un'organizzazione personale con `OrgRole::Owner`: `Organization::personal(...)` e
`OrgMember::active(..., OrgRole::Owner, ...)`. Ne segue che **ogni utente registrato
è owner di una propria organizzazione**, quindi un inquilino che si iscrive da sé, in
casa propria, è staff — e lì può creare immobili legittimamente, perché la Row Level
Security e `ensure_staff` glielo concedono.

Il client oggi fa la cosa giusta e non basta: `useIsStaff()` toglie *Immobili* dalla
navigazione e nasconde «Registra un immobile» ([ADR-042](08-decisioni.md#adr-042)),
ma è una scelta di **cosa offrire** e non un confine — ed è scritto proprio così
nell'ADR. Nella propria organizzazione quel ramo dice `owner`, e il bottone ricompare
a ragione.

Quindi R9 tocca il **modello**, non la presentazione, e le domande da sciogliere sono
almeno queste:

- un utente che nasce inquilino **ha** un'organizzazione propria? Se no, che cosa
  vede prima di accettare un invito, dato che oggi ogni elenco ha bisogno di
  `X-Org-Id`? Se sì, in che ruolo?
- l'invito che oggi esiste (`org_invitations`) porta l'appartenenza a
  un'organizzazione **esistente** e conferisce solo `tenant` — vincolo nel dominio e
  ripetuto nella policy. Un invito che *crea* un proprietario con la sua
  organizzazione è un'altra cosa e non esiste;
- come si verifica un proprietario, e dove vive quello stato: fuori dal prodotto —
  una email, una decisione manuale — oppure dentro, con una colonna e una
  transizione;
- che cosa succede agli utenti già registrati, che sono tutti owner della propria
  organizzazione.

Tocca registrazione, inviti, ruoli e probabilmente una migration, e ha una faccia di
sicurezza: è la voce che merita un ADR **prima** del codice, non dopo.


## Prossimo passo suggerito

**Il [collaudo della fetta C](11-piani-conclusi.md#esito-fetta-c)**, in undici punti, a
mano, con due account e senza mai aprire `curl`. Non è una formalità dopo il lavoro: **è
il criterio della fetta**, e il codice di `C1`–`C6` è scritto, verde e non ancora
attraversato da nessuno.

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
[fetta A](11-piani-conclusi.md#piano-completare-il-prodotto)
· [esito](11-piani-conclusi.md#esito-completare-il-prodotto-fetta-a) ·
[fetta B](11-piani-conclusi.md#piano-inquilino-fetta-b)
· [esito](11-piani-conclusi.md#esito-inquilino-fetta-b) ·
[fetta B-bis](11-piani-conclusi.md#piano-spese-co-living-fetta-b-bis) ·
[fetta C](11-piani-conclusi.md#piano-fetta-c)
· [esito](11-piani-conclusi.md#esito-fetta-c), **ancora aperto sul collaudo**.

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

Le fette A, B e B-bis sono concluse. I piani che le hanno prodotte sono archiviati per
intero in [11 — Piani conclusi](11-piani-conclusi.md), ognuno con il suo esito
([fetta A](11-piani-conclusi.md#piano-completare-il-prodotto),
[fetta B](11-piani-conclusi.md#piano-inquilino-fetta-b),
[fetta B-bis](11-piani-conclusi.md#piano-spese-co-living-fetta-b-bis)), e **le loro
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
[criterio d'ordine](#il-criterio-dordine-finché-a-provare-è-chi-costruisce) più sopra.
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
[11 — Piani conclusi](11-piani-conclusi.md#piano-spese-co-living-fetta-b-bis).

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
[11 — Piani conclusi](11-piani-conclusi.md#piano-fetta-c).

Le cinque funzioni del livello dati che nessuna schermata chiamava adesso hanno la loro,
e la più importante chiudeva il giro del prodotto: **un invitato entra dall'interfaccia**,
senza `curl`. Il backend è stato toccato **una volta sola** — il messaggio della
configurazione di `C6` — e i 424 test Rust sono gli stessi prima e dopo; il client passa
da 61 a 69.

**La fetta non è chiusa**, e la parte che manca è quella che conta: il
[collaudo in undici punti](11-piani-conclusi.md#esito-fetta-c) non è ancora stato
percorso. È il criterio della fetta, e finché non lo si è fatto «funziona» resta
un'affermazione su cui nessuno può essere smentito. Ciò che il giro trova va scritto
nell'esito **anche se non si corregge**.

Due decisioni sono diventate ADR: [044](08-decisioni.md#adr-044) sul rinnovo fra schede,
che si **serializza** invece di trasmettere la sessione, e
[045](08-decisioni.md#adr-045) sull'accettazione che sceglie l'organizzazione.
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
