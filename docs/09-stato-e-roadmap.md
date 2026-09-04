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

**La fetta A del [piano per completare il prodotto](#piano-completare-il-prodotto)**:
i pagamenti dal lato del proprietario.

È la scelta contro l'istinto, e la ragione sta nella decisione 1 di quel piano: il
percorso dell'inquilino è la metà che manca, ma **il giro del denaro è rotto adesso**,
per l'utente che già esiste. Dall'interfaccia un addebito si emette e poi si può solo
annullare o stornare — l'insoluto sale e non scende mai, e l'unico modo di farlo
scendere è rinunciare ai soldi.

Il percorso dell'inquilino è la fetta B, ed è già scomposta: adesso — per la prima
volta — è provabile con dati veri, perché un inquilino non esiste finché un
proprietario non lo invita.

Le **due semantiche della dashboard** che erano rimaste aperte sono state decise, e
sono nel piano: `written_off` diventa un nono numero (decisione 2, insieme allo storno
parziale, perché altrimenti conterebbe metà delle perdite) e il credito da
sovrapagamento resta visibile per addebito e non in dashboard (decisione 4).

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

---

<a id="piano-completare-il-prodotto"></a>

## Piano — Completare il prodotto

Pianificato il **4 settembre 2026**. Il piano resta qui come è stato scritto finché
non è concluso; a esito scritto si sposta in
[11 — Piani conclusi](11-piani-conclusi.md), nello stesso commit.

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
- **la ricerca full-text**, che è V2 e che va alimentata dall'outbox.

### Che cosa è stato deciso prima di pianificare

Quattro scelte, prese esplicitamente. Il resto del piano le dà per assodate.

| | Scelta | Alternativa scartata |
|---|---|---|
| Ordine | pagamenti, poi inquilino | l'inquilino per intero subito, pagamenti compresi |
| Perdite | nono numero in dashboard, **dentro la fetta A** | lasciare `written_off` indistinguibile da `void` |
| Sovrapagamenti | visibili per addebito, **non** in dashboard | un decimo riquadro, oppure compensare l'insoluto |
| Porte senza adattatore | `Notifier` dentro, OCR fuori | entrambe, o nessuna |

### Decisioni prese, con il perché

#### 1. I pagamenti prima dell'inquilino, ed è una scelta contro l'istinto

L'istinto dice l'inquilino: è metà del prodotto, è la parte che manca, ed è appena
diventata provabile con dati veri. È un buon argomento e non è quello decisivo.

Il decisivo è che **il giro del denaro è rotto adesso, per l'utente che già esiste**.
Dall'interfaccia un proprietario emette un addebito e poi può solo annullarlo o
stornarlo: **non può registrare un incasso**. Ne segue che l'insoluto in dashboard
sale e non scende mai, e che l'unico modo di farlo scendere è rinunciare ai soldi.
Il numero più in vista del prodotto è oggi un numero che mente per omissione.

Costa poco: le sei rotte esistono, l'idempotenza è già imposta dal bordo HTTP, i
saldi si leggono da `v_charge_balances` e sono veri dalla fetta dei pagamenti. È una
fetta corta che chiude un buco vero, e ne segue anche che la fetta B parte con la
contabilità già completa invece di doverla attraversare a metà.

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

#### 4. Il credito da sovrapagamento resta per addebito

Un `outstanding_amount` negativo è un credito verso l'inquilino, e oggi non compare
da nessuna parte perché la vista somma con `WHERE outstanding_amount > 0`.

Resta così in dashboard, e diventa **visibile e filtrabile nell'elenco degli
addebiti**, dove già si vede in verde. La dashboard è un cruscotto di ciò che è
dovuto, e un credito non è un insoluto negativo: metterceli insieme chiederebbe a chi
guarda di sottrarre due cose che non si sottraggono.

**Alternativa scartata: compensare `total_outstanding`.** Cambierebbe il significato
di un numero che un client legge già, quindi sarebbe una rottura, e nasconderebbe
esattamente lo scarto che la contabilità esiste per scoprire.

#### 5. `Notifier` dentro, e l'adattatore è SMTP

È l'ultima porta dichiarata e senza implementazione, e ha **due** consumatori in
attesa: l'invito, che oggi passa da «copia questo codice e mandalo tu», e — dalla
fetta A — la dichiarazione di pagamento in `pending`, che non avvisa nessuno e che lo
staff scopre solo guardando.

**SMTP e non un fornitore con la sua API**, e la ragione è la stessa per cui
`ObjectStorage` ha un adattatore solo che parla anche con MinIO: con SMTP il
fornitore diventa un **dato di configurazione** invece di un ramo del codice, e
cambiarlo è una variabile d'ambiente. Un adattatore per l'API di un fornitore
specifico si scrive quando quel fornitore offre qualcosa che SMTP non ha — tracciamento
delle aperture, template remoti — che è roba di V2.

**Il token dell'invito continua a tornare nella risposta HTTP.** Toglierlo sarebbe una
rottura, e soprattutto lascerebbe senza rimedio il caso in cui l'email non arriva. Il
client mostra «invito inviato a …» e tiene il codice sotto un dettaglio richiudibile,
che è la verità: l'email è partita, e se non arriva c'è ancora una via.

**Resta da confermare prima di eseguire:** quale server SMTP, e con quale indirizzo
mittente. È una scelta ordinaria, ma è una scelta.

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

### Decomposizione

#### Fetta A — il giro del denaro si chiude

Nove passi. Il **punto di taglio è dopo A7**: con l'incasso, la conferma e i due
numeri onesti il proprietario ha un modulo contabile coerente, e le notifiche sono una
fetta A-bis.

1. **Dominio: lo storno da `partially_paid`.** `Charge::write_off` ammette anche quello
   stato; `void` no, e il commento che lo spiega resta. Test unitari sulle due
   transizioni e sul rifiuto di `void`. Nessuna migration: la `0023` concede già
   l'`UPDATE` sulla sola `status`.
2. **Migration: il nono numero.** `CREATE OR REPLACE VIEW app.v_owner_dashboard` in una
   migration nuova — lo schema è append-only, quindi la `0015` non si tocca — con
   `written_off_total` calcolato come `SUM(amount - paid_amount)` sugli addebiti
   `written_off`. Invarianti dello schema, e il controllo che la vista resti
   `security_invoker`.
3. **Backend: il campo esce.** `OwnerDashboard`, il repository, il DTO, il test di
   contratto che verifica il numero su un addebito **parzialmente pagato** e stornato.
4. **Client: il livello dati dei pagamenti.** Le sei rotte, con la chiave di
   idempotenza generata **una volta per intento** e riusata a ogni ritentativo.
5. **Client: registrare un incasso.** Dal dettaglio di un addebito. `payer_user_id` è
   una scelta esplicita fra le parti del contratto, non un campo che si dimentica.
6. **Client: la coda delle dichiarazioni.** Elenco dei pagamenti `pending`, conferma e
   rifiuto, con la spiegazione di che cosa muove il saldo e che cosa no.
7. **Client: i due numeri.** Il nono riquadro in dashboard e il credito evidenziato e
   filtrabile nell'elenco addebiti.
8. **`Notifier`: l'adattatore SMTP.** Implementazione in `infrastructure`,
   configurazione in `AppConfig` e nelle tabelle di
   [07 — Operazioni](07-operazioni.md), iniezione in `main.rs`, invio sull'invito e
   sulla dichiarazione. Il client mostra «inviato» e tiene il codice sotto un
   dettaglio.
9. **Documentazione ed esito.** ADR per l'adattatore SMTP e per il nono numero;
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

Tutte lette nel codice e nei documenti prima di scriverle qui.

**La chiave di idempotenza non si rigenera al ritentativo.** La genera il client, ed è
deliberato: se la generasse il server ogni ritentativo produrrebbe una chiave nuova,
quindi un secondo pagamento, quindi un saldo che scende **due volte**, e `audit_log`
mostrerebbe due inserimenti perfettamente legittimi. Ne segue una condizione precisa
lato client: la chiave si genera **all'intento** — l'invio del form — e si riusa per
ogni tentativo successivo. Generarla dentro `mutationFn` la rigenererebbe a ogni
ritentativo, che è esattamente il difetto che l'header esiste per impedire.

**La perdita è il residuo e non l'importo.** Vedi la decisione 3. Sommare `amount`
gonfia le perdite di tutto ciò che è stato incassato.

**`outstanding_amount` non serve a calcolare le perdite**, perché la `0024` lo azzera
proprio su `written_off`. Va ricalcolato come `amount - paid_amount` dentro la vista.

**Una chiave ripetuta risponde `200`, non `201` e non `409`.** Il client deve trattare
i due stati come lo stesso esito riuscito. Il `409` esiste per due casi che la
rilettura non assorbe — la stessa chiave verso un **altro** addebito, e una chiave
usata da qualcuno di cui il chiamante non vede i pagamenti — e nessuno dei due si
risolve riprovando.

**Chi dichiara non può ritirare, e il client non deve offrirglielo.**
`payments_update` è staff soltanto, quindi un bottone «ritira la dichiarazione»
otterrebbe zero righe aggiornate, cioè `auth.denied_by_policy` — il codice riservato
ai nostri difetti. L'uscita da `pending` è il rifiuto, ed è dello staff. È un debito
già registrato e va rispettato, non aggirato.

**`payer_user_id` omesso rende l'incasso invisibile a chi ha pagato.** È lecito — un
bonifico da un conto che non si sa a chi ricondurre esiste — ma è anche il
comportamento di chi semplicemente dimentica il campo, che è il caso più facile. Il
debito dice che con il primo client si vede se debba diventare obbligatorio: **questa
è quella occasione**, e la risposta va scritta nell'esito.

**Il rifiuto azzera `paid_at` e non muove il saldo**, perché un pagamento `pending`
non lo aveva mosso. Un client che mostrasse «saldo aggiornato» dopo un rifiuto direbbe
il falso.

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

**L'adattatore SMTP è il posto più facile in cui far uscire dati personali.**
`docs/04` vieta email, token e corpi nei log, e un client SMTP che registri la
richiesta li registra tutti e tre insieme. Il token dell'invito in particolare è una
**credenziale**: va nel corpo del messaggio e in nessun log.

**Il bundle diviso per rotta senza `Suspense` progettati produce una pagina bianca**
dove prima c'era uno scheletro. I tre stati esistono già come componenti: i confini
vanno messi dove quelli sono, non dove capita.

### Come si saprà che è finita

`./ops/ci/test-integration.sh` verde e il job del client verde sono il minimo, non il
criterio.

**Fetta A, le tre verifiche che contano:**

1. **L'insoluto scende.** Un giro a mano dal browser: addebito da 800 €, incasso di
   300 € registrato, la dashboard passa da 800 a 500 e lo stato dell'addebito diventa
   `partially_paid`. È il giro che oggi non si può fare.
2. **La stessa chiave inviata due volte lascia il saldo dov'è.** Non è la suite: è il
   doppio clic su un bottone, fatto a mano, con il saldo guardato prima e dopo.
3. **Il nono numero dice il residuo.** Sull'addebito del punto 1, stornato dopo
   l'incasso parziale: `written_off_total` deve valere **500** e non 800.

Più, se A8 è dentro: **un'email di invito che arriva davvero**, con il token nel corpo
e in nessuna riga di log.

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
