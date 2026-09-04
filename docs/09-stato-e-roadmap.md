# 09 — Stato e roadmap

Stato al **1 settembre 2026**.

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

Le 31 migration coprono l'intero perimetro funzionale dell'MVP, non solo la parte
già esposta dall'API: 24 tabelle più `audit_log` partizionata, 35 enum, 6 domini,
170 indici, 66 policy RLS, 50 trigger, 4 viste, 34 funzioni.

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
| Pagamenti: incasso registrato dallo staff, dichiarazione dell'inquilino, conferma e rifiuto, elenco e dettaglio, idempotenza fornita dal client, stato dell'addebito ricalcolato dai pagamenti | completo per la fetta 1 |
| Spese co-living: gruppi con il creatore già dentro, composizione con uscita e rientro, spese con ripartizione `equal` e `weighted`, saldi letti dalla vista, regolamenti a due tempi confermati da chi riceve | completo per la fetta 1 |
| Coinquilini visibili: elenco dei residenti di un immobile letto da una funzione che proietta, anagrafica ristretta al perimetro della casa | completo per la fetta 1 |
| OCR delle bollette: porta `OcrProvider` senza fornitore scelto, adattatore deterministico, coda `ai_jobs` con `SKIP LOCKED`, `pending_review` con le sue uscite, stato del job dall'API | completo per la fetta 1 |
| Storage oggetti: adattatore S3/MinIO, cifratura letta dal fornitore | completo |
| Migratore, dispatcher dell'outbox, lettura automatica delle bollette | completi |
| Dashboard: `GET /dashboard`, otto numeri letti da `v_owner_dashboard`, staff soltanto | completo |
| Sessione in cookie: refresh `HttpOnly` fuori dal corpo, `Secure` condizionale, test che tiene in piedi l'argomento CSRF | completo |
| Immagine Docker, stack compose | completi |
| Pipeline CI | completa |

413 test: 204 unitari di dominio, 16 applicativi, 26 di infrastruttura, 11 unitari
del bordo HTTP, 75 di integrazione sulla persistenza, 3 sull'adattatore S3 contro
MinIO, 75 di contratto HTTP, 3 di guardia sulla documentazione della configurazione.

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
| Dashboard: otto numeri, ognuno collegato al posto dove si agisce | completo |
| Immobili: elenco a cursore, dettaglio, creazione, `PATCH` parziale, archiviazione | completo per la fetta 1 |
| Stanze e residenti | completo per la fetta 1 |
| Contratti: elenco, bozza, parti, attivazione, disdetta, stati derivati mostrati e non filtrati | completo per la fetta 1 |
| Inviti: emissione e consegna del token, presentato per quello che è | completo per la fetta 1 |
| Bollette: registrazione, scansione, **revisione dell'OCR campo per campo**, validazione, riaddebito | completo per la fetta 1 |
| Addebiti: scadenzario, annullamento, rinuncia all'incasso | completo per la fetta 1 |
| Reverse proxy in sviluppo e in `docker-compose.yml`, immagine del client | completi |

27 test: 12 sulla sessione e sul client dell'API — fra cui il refresh a singolo
volo **riprodotto anche al contrario** — 11 sui decimali e sulla formattazione, 4
sul percorso da sola tastiera.

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

   La voce si tiene perché la sua conclusione — «sbloccata non vuol dire
   prioritaria», e la finestra sulle semantiche la chiude il **primo client** e non
   l'esistenza dell'endpoint — resta vera e generale, e perché quella finestra
   adesso **si è chiusa**: le due semantiche non decise (se `written_off` sia una
   perdita a sé, se il credito da sovrapagamento vada mostrato) sono ancora aperte
   e cambiarle ora è una rottura. Sono scritte nel tipo di dominio `OwnerDashboard`
   e nel DTO, così chi le deciderà le trova dove guarderà.

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

**Pagamenti** è implementato nella fetta 1: incasso registrato dallo staff,
dichiarazione dell'inquilino in `pending`, conferma e rifiuto, elenco e dettaglio dei
pagamenti di un addebito, idempotenza fornita dal client, e stato dell'addebito
ricalcolato dai pagamenti. Restano fuori, e non per dimenticanza, i rimborsi, i
provider veri con i loro webhook, i pagamenti di quote e regolamenti — che restano
fuori **per sempre**, non in attesa: vedi sotto — e il
pagamento della bolletta al fornitore, che non è modellato affatto. Il resoconto è in
fondo: [Piano — Pagamenti, fetta 1](11-piani-conclusi.md#piano-pagamenti-fetta-1).

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
| L'incasso non attribuito è invisibile a chi ha pagato | `payments_select` mostra la riga a staff, pagante e incassante. Uno staff che registri un incasso senza valorizzare `payer_user_id` produce un pagamento che l'inquilino **non vede nel proprio elenco**, pur vedendone l'effetto in `paid_amount` sul saldo. È lecito — un bonifico da un conto che non si sa a chi ricondurre esiste — ma oggi è anche il comportamento di chi semplicemente omette il campo, cioè il caso più facile | Con il primo client: è lì che si vede se l'attribuzione debba diventare obbligatoria, o se convenga dedurla dalle parti del contratto |
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

## Prossimo passo suggerito

Il **percorso dell'inquilino nel client**, che è la fetta 2 e che adesso — per la
prima volta — è provabile con dati veri: un inquilino non esiste finché un
proprietario non lo invita, e il percorso del proprietario esiste da questa fetta.
Comprende segnalazioni, spese co-living e pagamenti dichiarati, che nell'API ci sono
già. Vedi [l'esito del client](11-piani-conclusi.md#esito-client-fetta-1).

Prima di cominciarla vale la pena decidere le **due semantiche della dashboard**
lasciate aperte — se `written_off` sia una perdita a sé, e se il credito da
sovrapagamento vada mostrato — perché la finestra in cui erano additive si è chiusa
con il primo client, che adesso c'è.

Resta anche la decisione che continua a bloccare più valore di ogni riga di codice,
ed è quella qui sotto: **il fornitore di OCR**.

La sezione che segue sulla dashboard è stata scritta quando la dashboard non c'era.
Si tiene perché la conclusione a cui arriva — «sbloccata non è prioritaria» — è
generale e vale per la prossima cosa a buon mercato che sembrerà ovvia; e perché la
sua eccezione, «vale se serve qualcosa da mostrare presto», è precisamente il caso
che si è verificato: la dashboard è stata fatta come **passo 1 del client**, cioè per
la ragione che quella sezione ammetteva.

«Far sì che i coinquilini si vedano» è stato fatto: vedi
[l'esito](11-piani-conclusi.md#esito-coinquilini-visibili-fetta-1). L'**OCR delle bollette** è stato fatto:
vedi [l'esito](11-piani-conclusi.md#esito-ocr-bollette-fetta-1) — con l'avvertenza che quella fetta ha
comprato il **percorso** e non il valore, perché [ADR-037](08-decisioni.md#adr-037)
rimanda deliberatamente la scelta del fornitore.

**Il perimetro funzionale dell'MVP è chiuso.** Non resta alcun modulo che abbia lo
schema e non il codice. Ne segue un cambiamento nel modo di scegliere che cosa fare, e
vale la pena dirlo perché finora l'ordine se lo dettava lo schema: da qui in poi
**l'ordine non è più derivabile**. Le prossime decisioni sono di prodotto — quali sono i
due numeri della dashboard la cui semantica non è decisa, se serva un client mostrabile
presto, se l'accuratezza dell'OCR valga il costo di misurarla — e non hanno una risposta
che si possa leggere in `migrations/`.

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

### Perché la dashboard resta ultima

Va detto qui perché è la conclusione **sbagliata** che questa fetta invitava a trarre, e
il piano dei pagamenti l'aveva quasi presa.

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

Le due domande, da decidere **prima** di scrivere l'endpoint e non dopo:

- **`written_off` va riportato a parte come perdita?** Oggi esce dall'insoluto insieme
  a `void`, e i due sono distinti nello stato e indistinguibili in dashboard. La
  distinzione conta nei report, e la dashboard è il primo report;
- **il credito da sovrapagamento va mostrato?** `outstanding_amount` negativo esiste ed
  è escluso dal totale dal `WHERE outstanding_amount > 0` della vista. Non compensa gli
  altri insoluti — probabilmente giusto — ma sparisce del tutto, e un credito verso
  l'inquilino che non compare da nessuna parte è un numero che qualcuno cercherà.

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
reali, ed è oggi l'ultima porta senza implementazione — e da questa fetta ha un secondo
consumatore in attesa: una dichiarazione di pagamento in `pending` non avvisa nessuno,
quindi lo staff la scopre solo guardando.
