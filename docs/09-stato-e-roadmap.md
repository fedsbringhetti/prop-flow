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

Le 36 migration coprono l'intero perimetro funzionale dell'MVP, non solo la parte
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
| **I contratti si correggono**: annullamento di una bozza, correzione e rimozione di una parte — tre rotte e **nessuna migration**, perché `deleted_at`, `left_at` e `lease_parties_update` erano già nella `0014` | completo per la sezione 3 |
| **L'occupazione è derivata e non memorizzata**: `app.room_is_occupied` e `app.property_occupancy_status` (`0033`), con `v_property_occupancy` che smette di contare una colonna che nessuno scrive | completo per la sezione 4 |
| **Una bolletta si dichiara pagata** al fornitore, con la sua data (`0034`). Non è un pagamento: `app.payments` modella il denaro in entrata | completo per la sezione 4 |
| **Una segnalazione può essere di interesse comune** (`0035`): il quinto ramo sta in `app.can_access_ticket`, quindi con la riga arrivano il thread, lo storico e le fotografie. Non concede scrittura | completo per la sezione 5 |
| **Il testo di una segnalazione si corregge** finché nessuno l'ha presa in carico: `PATCH /tickets/{id}/report` e la migration `0036`, che **revoca** su due colonne la scelta della `0020` | completo per la sezione 5 |
| **Le risposte dicono chi**: nome di chi ha segnalato, di chi ha in carico e di chi scrive nel thread, letti da `app.users` sotto `users_select` | completo per la sezione 5 |
| **Un thread terminato si legge e non si scrive**, e riaprire pretende un motivo che finisce nella conversazione | completo per la sezione 5 |
| Immagine Docker, stack compose | completi |
| Pipeline CI | completa |

438 test: 206 unitari di dominio, 16 applicativi, 27 di infrastruttura, 11 unitari
del bordo HTTP, 81 di integrazione sulla persistenza, 3 sull'adattatore S3 contro
MinIO, 91 di contratto HTTP, 3 di guardia sulla documentazione della configurazione.

I sette nuovi sono della sezione 5, e provano soprattutto **confini**: la segnalazione
condivisa che arriva al coinquilino *con il thread e lo storico*, quella non condivisa che
non gli arriva affatto, il coinquilino che la vede e non la scrive, la correzione che si
chiude ad `assigned`, il thread chiuso che si legge e non si scrive, e il motivo della
riapertura che si va a rileggere nella conversazione.

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
| **La stanza si crea con tutti i suoi campi**, dallo stesso componente che la corregge: sette, non quattro contro cinque. `status` resta fuori finché non si decide se sia derivato (`R11`) | completo per la sezione 1 |
| **La barra laterale è alta quanto la finestra e resta ferma**, con il selettore dell'organizzazione ancorato in fondo e l'elenco che scorre a sé — da `lg` in su, che sotto è un menù a scomparsa | completo per la sezione 1 |
| **La password si può guardare**, e i due form di autenticazione non tirano più `zod`: −76,4 kB sulla prima schermata che vede un utente nuovo | completo per la sezione 1 |
| **Una bozza sbagliata si annulla e una parte si corregge**: quota, nome, telefono, ruolo, intestatario — l'email solo finché nessuno l'ha usata — e si toglie chi è stato inserito per sbaglio | completo per la sezione 3 |
| **Il form non produce più quote sbagliate**: propone il residuo, offre l'intestatario solo se libero, e l'invito dice quale quota riceverà chi accetta | completo per la sezione 3 |
| **L'elenco dei contratti dice quale casa e quale stanza**, e la creazione avverte se il periodo è già occupato | completo per la sezione 3 |
| **Stato dichiarato ed effettivo, mostrati insieme quando divergono**: una stanza fuori uso e insieme occupata è un'anomalia da vedere. Le tendine offrono solo ciò che si decide, e il filtro dell'occupazione sostituisce quello sullo stato derivato | completo per la sezione 4 |
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

86 test: 16 sulla sessione e sul client dell'API — fra cui il refresh a singolo
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
| ~~**Il login è la rotta iniziale più pesante**~~ — **chiuso** dalla sezione 1, `S2` | `zod` è uscito da `AuthPage` e da lì soltanto: le regole di quei due form sono tre — un'email, dodici caratteri, dei campi obbligatori — e `react-hook-form` le esprime da sé. Rimisurato prima e dopo sullo stesso albero: **−76,4 kB (−20,6 gz)**, da 556,5 kB (176,6) a **480,1 (156,0)**. Restano 30 kB sopra il riepilogo, e non sono `zod`: sono `react-hook-form`, che quei form usano davvero. **Le altre tre schermate tengono `zod`**, e non è un'incoerenza da sanare — sono rotte pigre, e «uniformare» rimetterebbe il chunk sulla prima schermata | Chiuso. I numeri e il metodo sono nell'[esito](11-piani-conclusi.md#esito-sezione-1) |
| Gli allegati si scaricano da un link senza `Authorization` | `GET /documents/{id}/content` risponde `307` verso un URL pre-firmato, e il client ci arriva con un `<a href>` — che è ciò che permette al browser di scaricare invece di tenere i byte in memoria. Il link però non porta il token, quindi funziona solo finché la rotta accetta la richiesta dal contesto della scheda. Oggi funziona; il giorno in cui l'API pretendesse l'header su quella rotta, il download smetterebbe di funzionare **senza che nulla in questo repository cambi** | Se il download smette di funzionare, o quando si decida come firmare i link lato client. L'alternativa è un `fetch` con il token seguito da un `blob:` URL, che tiene il file in memoria — accettabile per un PDF, non per una scansione da 25 MB |
| `actions/checkout@v4` gira su Node 20 | I runner lo forzano già su Node 24 con un avviso di deprecazione. Non rompe oggi, romperà quando GitHub toglierà il ponte | Al prossimo intervento sui workflow, o quando l'avviso diventa errore |
| La coda delle dichiarazioni non si filtra per immobile | `GET /api/v1/payments` ha `status` e `lease_id`, e non `property_id`: `app.payments` non ha quella colonna e il legame passerebbe da due salti (`payment → charge → lease → property`). Con pochi immobili la coda sta in una pagina e il filtro non compra niente; con decine diventa lunga | Quando gli immobili per organizzazione passano la decina, o se qualcuno chiede «i pagamenti di questa casa» |
| Un filtro parametrico su un enum non usa un indice parziale | Misurato, non dedotto: `($1::text IS NULL OR status = $1::app.payment_status)` **non** tocca `payments_pending_ix`, perché il cast `text → enum` passa da `enum_in`, che PostgreSQL dichiara `STABLE` e non `IMMUTABLE`, e non viene quindi ridotto a una costante. Il rimedio è nel repository dei pagamenti — una condizione ridondante che nomina il letterale — ed è **locale a quella query**. Ogni altro elenco del progetto usa lo stesso pattern parametrico su uno stato, e nessuno di quelli ha oggi un indice parziale da mancare | Quando un altro elenco filtrato per stato acquisisce un indice parziale. La misura e il rimedio stanno in [ADR-040](08-decisioni.md#adr-040) |
| ~~Un `.env` più vecchio dello storage impedisce l'avvio dell'API~~ — **chiuso** dalla fetta C, `C6` | L'avvio fallisce ancora, ed è giusto che fallisca: **il rimedio non era un default**, perché uno storage configurato a caso sposta il fallimento al primo caricamento e lo fa somigliare a un problema di rete. È cambiato ciò che si legge: `AppConfig::load` distingue un'impostazione **mancante** da una **sbagliata**, e sulla prima dice che le variabili hanno prefisso `PROPFLOW__`, che l'elenco completo è in `.env.example`, e che nessuno lo carica da solo. Il messaggio di figment — quale campo, sotto quale chiave — resta nella catena delle cause | Chiuso, dopo tre occorrenze in tre fette |

## Prossimo passo suggerito

**La [5-ter](#piano-sezione-5-ter), poi la [5-quater](#piano-utenti-veri).** Il
[collaudo](11-piani-conclusi.md#esito-sezione-5-bis) è **concluso** — percorso per intero
il 9 e il 10 settembre 2026, da chi non aveva scritto il codice — e con esso l'[esito
della fetta C](11-piani-conclusi.md#esito-fetta-c) smette di essere aperto. **Nessuno
degli undici punti si è fermato**: le sei voci uscite dal giro sono difetti dentro
percorsi che funzionano, il che dice che ciò che manca è rifinitura e non fondamenta.

Le due sezioni che restano hanno un obiettivo dichiarato che le lega: *un proprietario vero, che non ha scritto il codice, usa
il prodotto per un mese con i suoi dati*. Il giro dice se il prodotto regge; la 5-ter
corregge ciò che il giro ha trovato; la 5-quater aggiunge le quattro cose senza cui una
persona esterna non può cominciare — la posta, il **recupero della password**, il deploy,
il backup.

La 5-ter è quella che il collaudo ha prodotto: mezza giornata di
giro vero, fatto da chi non aveva scritto il codice, ha dato **sei voci** — `R30`–`R35` —
e una di esse, `R31`, è un buco nel **ciclo di vita normale**. Un inquilino il cui
contratto scade e che ne firma un altro **non può rientrare**: l'invito è l'unica via che
scrive `lease_parties.user_id`, e rifiuta chi è già membro. Cinque sezioni di lettura del
codice non l'avevano visto, e nessun test poteva vederlo — ogni test costruisce un
inquilino nuovo.

**È il rendimento che il rinvio del collaudo aveva sospeso**, e che l'[esito della sezione
2](11-piani-conclusi.md#esito-sezione-2) aveva scritto come costo: *il giro alimenta i
piani, non solo li verifica*.

**Il [collaudo in undici punti](12-da-fare.md#lordine-di-esecuzione), che è la 5-bis.**
Il piano è già scritto — passi `T3`–`T7` dell'[esito della sezione
2](11-piani-conclusi.md#piano-sezione-2), con le sette trappole e la finta bolletta — e da
adesso **nove punti su undici sono su codice fermo**: l'1 e il 5 dipendono da `R9`, che è
la sezione 8, e aspettare anche quella vorrebbe dire non provare niente per altre tre
sezioni.

**È anche il momento in cui le sezioni 3, 4 e 5 si vedono per la prima volta con gli
occhi.** Tutte e tre sono state verificate **solo con i test**, e la 5 cambia la scheda
della segnalazione in quattro punti — la casella all'apertura, i nomi, il bottone
«Correggi», la conversazione che si chiude — che il punto **7** attraversa per intero.

**Chi riprende deve riavviare l'API e ricreare il database**: il binario in esecuzione è
precedente alle rotte delle sezioni 3, 4 e 5, e le migration `0033`–`0036` vanno applicate.
Il costo del rinvio è scritto nell'[esito della sezione
2](11-piani-conclusi.md#esito-sezione-2) e resta vero: il giro **alimenta i piani** e non
solo li verifica — `R26` è entrata così.

**La [sezione 5](11-piani-conclusi.md#esito-sezione-5) è conclusa**: la visibilità di una
segnalazione è una scelta di chi la apre, il testo si corregge finché nessuno l'ha presa in
carico, le risposte dicono **chi** e non solo quando, su una segnalazione chiusa il thread
si legge e non si scrive, e riaprire pretende un motivo che finisce nella conversazione.
[Piano](11-piani-conclusi.md#piano-sezione-5) ·
[esito](11-piani-conclusi.md#esito-sezione-5). È la prima sezione che **revoca** una
decisione documentata — [ADR-048](08-decisioni.md#adr-048) emenda
[ADR-026](08-decisioni.md#adr-026) su due colonne — e il modo in cui lo fa, emendando
l'invariante di schema e il test dei privilegi nello stesso passo della migration, è la
parte da ricordare. Ha anche reso **portante** `users_select`: la `0030` aveva scritto che
la finestra per restringerla l'avrebbe chiusa il primo client che leggesse un nome da lì,
ed è questo.

**La [sezione 2](11-piani-conclusi.md#esito-sezione-2) è conclusa quanto alle due
correzioni**: il punto 10 del collaudo adesso prova ciò che dichiara — vuole due schede
dello stesso account — e con lui il punto 4, che aveva lo stesso difetto e che nessuno
sapeva. Il messaggio della configurazione dice che `.env` lo carica `just`, in tutti e otto
i posti in cui quella frase viveva.

**La [sezione 3](11-piani-conclusi.md#esito-sezione-3) è conclusa**: il vicolo cieco dei
contratti non esiste più. Una bozza si annulla, una quota sbagliata si corregge — anche a
contratto attivo, che è il caso dei contratti già rotti — una parte inserita per sbaglio si
toglie, e l'attivazione rifiuta una quota a zero ([ADR-046](08-decisioni.md#adr-046)).
Eseguirla ha chiuso anche un buco che nessuno aveva in elenco: si poteva aggiungere una
parte a un contratto **attivo**, e da lì le quote non sommavano più a 100 senza che niente
protestasse.

**La sezione 1 è conclusa** — sei ritocchi di sola interfaccia, nessuna decisione da
prendere: [piano](11-piani-conclusi.md#piano-sezione-1) ·
[esito](11-piani-conclusi.md#esito-sezione-1). Il backend non è stato toccato, e
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
[fetta A](11-piani-conclusi.md#piano-completare-il-prodotto)
· [esito](11-piani-conclusi.md#esito-completare-il-prodotto-fetta-a) ·
[fetta B](11-piani-conclusi.md#piano-inquilino-fetta-b)
· [esito](11-piani-conclusi.md#esito-inquilino-fetta-b) ·
[fetta B-bis](11-piani-conclusi.md#piano-spese-co-living-fetta-b-bis) ·
[fetta C](11-piani-conclusi.md#piano-fetta-c)
· [esito](11-piani-conclusi.md#esito-fetta-c), **chiuso dal collaudo il 10 settembre 2026**.

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

#### 2. Il codice di recupero si incolla, non si clicca — ADR-051

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

### Decomposizione

##### `Z1` — La posta

I passi 9 e 10 di [Prima degli utenti veri](#prima-degli-utenti-veri), da rileggere per
intero: la porta esiste, l'adattatore no, e `Notifier` **non è iniettata da nessuna
parte**.

Qui si aggiungono le due cose che quella prosa non aveva:

- **le impostazioni sono obbligatorie** in `AppConfig`, con il messaggio di avvio che dice
  che cosa manca;
- **un raccoglitore locale nel `docker-compose.yml`** per lo sviluppo, e la configurazione
  di un mittente vero per la produzione, che è cosa diversa — vedi le trappole.

##### `Z2` — Il recupero della password — **ADR-051 prima**

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

Tre test, e sono i tre che contano: la conferma **revoca le sessioni**; un indirizzo
sconosciuto riceve la stessa risposta di uno noto; un codice già usato non vale due volte.

##### `Z3` — Il deploy

VPS, dominio, il `Caddyfile` che punta al dominio, i segreti fuori dal repository, il
migratore prima dell'API, e la politica di riavvio.

Il criterio non è «i container sono su»: è **aprire il prodotto dal telefono, in 4G, e
fare l'accesso**. Finché non è successo, non è deployato.

##### `Z4` — Il backup, e il ripristino provato

`pg_dump` a orario e copia del bucket, cifrati, **fuori dalla macchina**, con una
ritenzione dichiarata. Poi il ripristino, una volta, su un database vuoto.

##### `Z5` — La privacy

Una pagina raggiungibile dal prodotto: che cosa si raccoglie, perché, dove sta, per quanto
e come si chiede la cancellazione. E il giro corrispondente: quando il tuo proprietario
chiederà di cancellare tutto, deve esistere una risposta che non sia una `DELETE` a mano.

### Trappole

**La posta da una VPS nuova finisce nello spam, ed è la trappola pratica più cara di
questa fase.** Un IP appena assegnato non ha reputazione, e senza SPF, DKIM e DMARC sul
dominio l'email di recupero non arriva — e il sintomo è **muto**: nessun errore, il codice
non compare. Non è un difetto del codice ed è il motivo per cui l'adattatore parla SMTP e
non con l'API di qualcuno: il mittente vero è un dato di configurazione. Va provato **con
un indirizzo vero**, non solo con il raccoglitore locale.

**Un raccoglitore locale non è un fornitore.** Accetta tutto: non verifica SPF, DKIM né i
rimbalzi. Prova che il messaggio parte e che cosa contiene, non che sarebbe recapitato.

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
- una password si recupera da capo a fondo, e le sessioni aperte prima **non funzionano
  più**;
- chiedere il recupero per un indirizzo che non esiste dà la stessa risposta di uno che
  esiste, e il client lo dice invece di lasciarlo intuire;
- il prodotto risponde su un dominio in HTTPS, aperto **dal telefono in 4G**;
- un dump viene ripristinato in un database vuoto e il prodotto ci parte sopra;
- esiste una pagina che dice che cosa si raccoglie e come si chiede la cancellazione;
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
- **il modello di ricavo.** Questa fase esiste per poter fare la domanda, non per
  rispondere.


<a id="raccolta-da-ordinare"></a>

## Da fare — l'elenco sta in [12](12-da-fare.md)

Le cose da sistemare e da costruire — **otto voci**, raccolte usando il prodotto e
leggendo il codice — hanno un file loro: [12 — Da fare](12-da-fare.md). Erano ventuno: le
sezioni 1–5 ne hanno chiuse venti, e tre sono entrate nel frattempo.

**Stanno lì e non qui per una ragione di ciclo di vita**, che è il criterio con cui
questi documenti si dividono: `09` si riscrive a ogni fetta, e una raccolta che vive
dentro il piano in corso viene cancellata insieme a esso. `12` invece cresce e si svuota
di continuo, una voce alla volta.

Quel file **non è un piano** e lo dichiara: le voci sono riconosciute e non istruite. Il
piano di una sezione si scrive **qui**, quando si sceglie di eseguirla, e poi si archivia
in [11](11-piani-conclusi.md) con il suo esito.

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
