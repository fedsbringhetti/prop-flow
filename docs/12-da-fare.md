# 12 — Da fare

Le cose da sistemare e da costruire, raccolte usando il prodotto e leggendo il codice.
**Dieci voci aperte**, una delle quali rinviata con la sua ragione (`R40`), con un identificatore stabile ciascuna — `R35` è entrata e si è dissolta nello stesso giorno, dentro `R31`.

Trentadue sono uscite: `R4`, `R5`, `R6`, `R10`, `R20` e il bottone del thread con la
[sezione 1](11-piani-conclusi.md#esito-sezione-1); `R2` e `R3` con la
[sezione 2](11-piani-conclusi.md#esito-sezione-2); `R12`, `R13`, `R14`, `R17` e `R18` con
la [sezione 3](11-piani-conclusi.md#esito-sezione-3); `R1`, `R11` e `R25` con la
[sezione 4](11-piani-conclusi.md#esito-sezione-4); `R19`, `R19-bis`, `R21`, `R22`, `R23` e
`R24` con la [sezione 5](11-piani-conclusi.md#esito-sezione-5); `R30`, `R31`, `R32`, `R33`
e `R34` con la [sezione 5-ter](11-piani-conclusi.md#esito-sezione-5-ter); `R7`, `R16` e
`R38` con la [sezione 6](11-piani-conclusi.md#esito-sezione-6); `R15` con la
[sezione 7](11-piani-conclusi.md#esito-sezione-7); `R9` e `R47` con la
[sezione 8](11-piani-conclusi.md#esito-sezione-8); `R8` con la
[sezione 9](11-piani-conclusi.md#esito-sezione-9); `R50`, `R51` e `R46` con la
[sezione 10](11-piani-conclusi.md#esito-sezione-10); `R36`, `R26`, `R49`, `R52`, `R41` e
`R27` con la [sezione 11](11-piani-conclusi.md#esito-sezione-11); `R44`, `R45` e `R29`
con la [sezione 12](11-piani-conclusi.md#esito-sezione-12). Sono entrate
`R26`, `R27`, `R28` e `R29`, poi `R30`–`R35` dal collaudo, e infine `R36` **eseguendo** la
5-ter, `R37` **rivedendo** il piano della 5-quater contro il codice e `R38` **verificandone
le premesse** prima di pianificare la sezione 6, con `R39` e `R40` uscite dalla stessa
pianificazione come ciò che quella sezione decide di **non** fare — e `R41` **eseguendo** la
6, come conseguenza di ciò che quella sezione ha costruito, `R42` **controllando la
sessione** dopo averla conclusa, `R43` e `R44` **usando** il prodotto — nel giro a
mano che l'esito della 6 chiedeva — `R45` **usando** il prodotto nel giro a mano della
7, archiviando in blocco — e `R46` e `R47` dal **revisore a contesto pulito** del passo
`F5` della 8, che è il primo passo rivisto da qualcuno che non aveva scritto il piano,
e `R48` e `R49` dal revisore dei passi `F6`+`F7`, che sono anche gli ultimi rivisti così:
il metodo è stato [ritirato](11-piani-conclusi.md#piano-sezione-8)
per il costo, e ciò che vedeva lo si cerca da sé; `R50` e `R51` dal **giro a mano della
8**, fatto come primo passo della 9 ([poscritto](11-piani-conclusi.md#poscritto-sezione-8));
`R52` **eseguendo** la 10 e `R53` **eseguendo** la 11, entrambe viste di fianco a ciò che
si stava sistemando; `R54` dal **giro a mano della 12**, che è tornato a essere il modo in
cui si trovano le cose — l'ha scoperta l'ora in cui il giro è stato fatto; `R59` **pianificando** lo storage
senza MinIO, leggendo gli artefatti del deploy.

**Quasi tutte le ha trovate l'uso del prodotto**, non la lettura del codice, e le ultime
due sono l'eccezione che vale la pena distinguere: non vengono da un giro né da una
lettura a freddo, ma dal *lavorare su una cosa e vederne un'altra di fianco*. `R29` è
entrata **preparando** il collaudo, prima che cominciasse: seminare dei dati veri è già
usare il prodotto. Le sei da `R30` a
`R35` sono uscite dalla prima mezza giornata di giro vero, fatto da chi non aveva scritto
il codice — che è il rendimento che il collaudo aveva promesso e che cinque sezioni di
rinvio avevano tenuto fermo.

## Perché un file a sé

I documenti di questo progetto si dividono per **ciclo di vita**, non per argomento, e
questo ne ha uno terzo che né `09` né `11` hanno:

- [09 — Stato e roadmap](09-stato-e-roadmap.md) **si riscrive a ogni fetta**: contiene
  lo stato e il piano in corso;
- [11 — Piani conclusi](11-piani-conclusi.md) **non si riscrive mai**: è l'archivio;
- questo file **cresce e si svuota di continuo**: una voce entra quando la si incontra
  ed esce quando è fatta.

Tenerlo dentro `09` lo faceva cancellare a ogni riscrittura del piano, che è
precisamente ciò che una raccolta non deve subire.

## Che cosa questo file **non** è

**Non è un piano**, e la distinzione è operativa e non formale. Le voci qui sono
*riconosciute*, non *istruite*: hanno una nota tecnica verificata, non una
decomposizione in passi. Un piano — con la scomposizione, le decisioni, le alternative
scartate e le trappole — si scrive in `09` quando si sceglie di eseguire una sezione, e
poi si archivia in `11` con il suo esito. Vedi `CLAUDE.md` §3.

Trattare questo elenco come lavoro istruito è l'errore che rende inutile la distinzione:
diverse voci qui sotto **cambiano di dimensione appena le si guarda da vicino**, e tre
si sono già rivelate non difetti ma comportamenti voluti.

## Come si usa

**Gli identificatori sono stabili e non si riusano.** `R7` è `R7` per sempre: si cita
nei messaggi di commit, nei corpi delle PR e da una conversazione all'altra. Una voce
conclusa **si toglie da qui** e resta nella storia di git e nell'esito della fetta che
l'ha chiusa; il suo numero non viene riassegnato a nient'altro.

**Lo stato dice a che punto è**, e sono tre:

| Stato | Vuol dire |
|---|---|
| *aperta* | riconosciuta, e le domande che porta non hanno ancora una risposta |
| **decisa** | la forma è stata scelta; restano da scrivere il piano e il codice |
| *fatta* | esce da questo file |

**Il lavoro procede per sezioni, una per conversazione.** Chi apre una sessione nuova
legge: questo file per sapere che cosa c'è, `09` per il piano della sezione in corso, e
l'esito in `11` della sezione appena chiusa per sapere che cosa è cambiato sotto i
piedi. È la ragione per cui gli esiti si scrivono anche quando vanno bene.

<a id="lordine-di-esecuzione"></a>

## L'ordine di esecuzione sta in [09](09-stato-e-roadmap.md#le-sezioni)

Le voci si eseguono **per sezione, una per conversazione**, e la tabella delle sezioni —
quali voci, in che ordine, con quale esito — è la roadmap: dal 18 settembre 2026 sta in
[09 — Il cammino](09-stato-e-roadmap.md#il-cammino), che è il documento che si riscrive a
ogni fetta. Qui restano le voci, con il loro stato. Com'era, con le ragioni dell'ordine, è
[archiviato](11-piani-conclusi.md#ordine-di-esecuzione-com-era-in-12).

## L'elenco

### Dal collaudo e dalla lettura del codice

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R59 | aperta | **In produzione il download di un documento non raggiunge il browser** | Trovata **pianificando lo storage senza MinIO** ([ADR-057](08-decisioni.md#adr-057)), leggendo gli artefatti del deploy, che non sono mai stati eseguiti: **dedotta, non osservata** — ma riproducibile in locale, vedi sotto. `DownloadDocument` restituisce un URL **prefirmato** sull'endpoint dello storage (`use_cases/document/download.rs`), e il client ci manda il browser. Con l'API lanciata sull'host (`cargo run`) l'endpoint è `http://localhost:9000`, che il browser raggiunge, ed è la ragione per cui non si è mai visto; nel compose di produzione è `http://storage:8333` — prima `http://minio:9000` — cioè un nome che esiste solo **dentro la rete del compose**, e il Caddyfile (`ops/deploy/Caddyfile`) inoltra soltanto `/api/v1/*` e i file statici. Il primo documento scaricato dal telefono, al punto 5 del cammino, fallirebbe. **E si osserva oggi, in locale, senza aspettare il deploy**: con `docker compose up` — lo stack dietro il proxy su `:8081`, che `CLAUDE.md` §4 chiama «la stessa forma che avrà in produzione» — l'API ha già `PROPFLOW__STORAGE__ENDPOINT: http://storage:8333` (`docker-compose.yml:132`) e anche `ops/web/Caddyfile` instrada solo `/api/v1/*`. Riprodurla costa dieci minuti ed è la cosa più economica da fare **prima** di decidere fra le due vie. **Due vie, e una decisione.** Esporre lo storage su un'origine pubblica — un sottodominio, perché SigV4 firma l'host e un prefisso di percorso aggiunto dal proxy invaliderebbe la firma — con un secondo endpoint usato **solo** per firmare; oppure far passare il download dall'API, che rilegge l'oggetto e lo serve, togliendo l'URL prefirmato e con lui il «permesso trasferibile» che `presign_ttl_seconds` limita. La prima tiene i byte fuori dall'API, la seconda tiene un'origine sola, che è metà dell'argomento di [ADR-038](08-decisioni.md#adr-038). **Va decisa prima del deploy** |

### Interfaccia e prodotto

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R54 | aperta | **«Oggi» del prodotto è il giorno UTC, e per due ore ogni notte non è il giorno italiano** | Trovata **eseguendo la sezione 12**, facendo il giro a mano all'una di notte: un contratto attivo dal 23 settembre, creato il 23 settembre alle 01:00, **non risultava in essere** — e l'immobile si archiviava senza il rifiuto appena costruito. Non è un difetto della sezione: è `app.lease_in_force` (`0017`) che confronta con `CURRENT_DATE`, valutato nel fuso **della sessione**, e la connessione di `sqlx` nasce con `TimeZone = UTC`. Alle 01:00 di Roma in UTC è ancora il giorno prima. Verificato: nella stessa transazione, `SET TimeZone='UTC'` fa rispondere `false` a `app.lease_in_force('active','2026-09-23','2027-09-22')` e `Europe/Rome` `true`. **I due livelli sono coerenti fra loro**, ed è la cosa buona: il dominio usa `clock.now().date_naive()`, cioè `Utc::now()`, quindi `Lease::is_in_force` e la funzione SQL dicono la stessa cosa. Il prodotto ha «oggi» in UTC, e basta. **Non tocca solo i contratti**: `is_past_due` degli addebiti (`0024`) confronta `due_date < CURRENT_DATE`, e `v_property_occupancy` passa da `lease_in_force`; per due ore ogni notte (una d'inverno) un addebito scaduto oggi non risulta scaduto e un contratto che comincia oggi non è in essere. **Due vie, e nessuna è ovvia.** Fissare il fuso della sessione applicativa a `Europe/Rome` allinea il prodotto a chi lo usa, ma lo lega a un fuso solo — e la V3 multi-tenant non lo è; oppure si dichiara che le date sono UTC e lo si scrive dove si vede, che è coerente ma sposta il problema su chi legge. **Da decidere anche se valga la pena**: la finestra è di due ore di notte, e il danno osservato è che un'archiviazione passa quando non dovrebbe |

### Stati che nessuno aggiorna

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|

**R11 e [R1](#lelenco) sono la stessa famiglia** — un campo di stato che l'API accetta e che nessuno aggiorna — ma il rimedio non è lo stesso, e vale la pena non trattarli insieme senza deciderlo.

Su un immobile lo stato è un giudizio di chi amministra, quindi un campo nel form è la risposta giusta. **Su una stanza no**: che sia occupata è un *fatto*, e discende dai contratti. Metterci una tendina significherebbe chiedere a una persona di ricordarsi di aggiornare a mano qualcosa che il database sa già — e `app.property_residents` della migration `0030` lo calcola di fatto, unendo `lease_parties` a `leases` e a `rooms` per contratti attivi.

Le domande da sciogliere prima di scrivere codice, e sono la ragione per cui questa voce non è un rimedio di due righe:

- **derivato o scritto?** Una colonna aggiornata da un trigger sull'attivazione e sulla disdetta di un contratto, oppure uno stato calcolato al momento della lettura, che non può divergere ma non si può filtrare con un indice;
- **che cosa fa la disdetta**, e che cosa fa un contratto futuro: una stanza con una locazione che comincia fra un mese oggi è `available`, e domani?
- **`unavailable` resta manuale**, qualunque cosa si scelga: quello è un giudizio — una stanza in ristrutturazione — e non discende da nessun contratto. Un rimedio che derivi tutto lo cancellerebbe.

### Contratti: creare, disfare, riconoscere

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|


**Il vicolo cieco di `R13`, `R17` e `R18` è stato sciolto dalla
[sezione 3](11-piani-conclusi.md#esito-sezione-3).** Come ci si arrivava — il form che
proponeva 100 alla prima parte, l'invito successivo a zero, l'attivazione che verificava
la somma e non il senso, e nessuna correzione possibile — è scritto nel piano, e la regola
che adesso lo impedisce è [ADR-046](08-decisioni.md#adr-046).

### Segnalazioni

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|

**Le sei voci sono uscite con la [sezione 5](11-piani-conclusi.md#esito-sezione-5).** La
visibilità è una scelta di chi apre (`R19`, migration `0035`), il testo si corregge finché
nessuno ci ha messo mano (`R19-bis`, migration `0036` e `PATCH /tickets/{id}/report`), le
risposte dicono **chi** e non solo quando (`R21`, `R22`), su una segnalazione chiusa il
thread si legge e non si scrive (`R23`), e riaprire pretende un motivo che finisce nella
conversazione (`R24`).

Due cose che quella sezione lascia dette, e che serviranno a chi tocca il modulo:
`ticket_status_history.reason` **resta non scritta**, con la ragione in
[ADR-050](08-decisioni.md#adr-050); e `users_select` è ora **portante**, perché quella
sezione è il primo client che legge un nome da `app.users`
([ADR-049](08-decisioni.md#adr-049)).

### Bollette

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|

### Addebiti e pagamenti

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|

### Dal collaudo, traccia A e B — la prima mezza giornata di giro vero

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R35 | ~~aperta~~ **si dissolve in `R31`** | ~~Che cosa vede l'inquilino di un contratto non ancora attivo~~ | **Riprodotto il 9 settembre 2026, e la premessa era sbagliata.** Un inquilino invitato su un contratto **in bozza** accetta (`200`) e **lo vede**, marcato `draft`: non esiste nessuna regola che nasconda le bozze a chi è collegato. Il contratto che non compariva non compariva perché la persona **non era collegata** — `lease_parties.user_id` lo scrive solo l'accettazione, e per chi è già membro quella fallisce con `409 membership.already_exists`. È `R31` visto dall'altro lato, ed è la sua conferma dal vivo. Resta una sola domanda, piccola e di presentazione: se la parola «Bozza» — che è lo stato interno di chi amministra — sia quella giusta da mostrare a chi abiterà lì, o se serva «In attesa di attivazione». Andava con `R8`, chiusa con la [sezione 9](11-piani-conclusi.md#esito-sezione-9), che ha lasciato la parola com'è — la guida dice a chi abita «è in bozza, e lo vedi lo stesso» — e lo scrive nel suo esito come cosa aperta: è una stringa, quando qualcuno la chiederà. Comportamento di oggi, per il resto verificato: `InviteTenant` **non guarda lo stato** del contratto, quindi si invita anche su una bozza; `AcceptInvitation` collega la parte; `app.can_access_lease` non filtra lo stato. L'inquilino quindi **vede** il contratto, marcato «Bozza». Le letture possibili sono tre e portano a lavori opposti: che non debba vederlo finché non è attivo; che debba vederlo ma non con la parola «Bozza», che è lo stato interno di chi amministra e a lui non dice niente; oppure che vada bene così. Da chiarire prima di pianificare, ed è la ragione per cui questa voce non entra nella sezione con le altre cinque |

### Cancellare, in generale

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R37 | aperta | **Cancellare i dati di una persona su sua richiesta non è un'operazione del prodotto** | Riconosciuta **rivedendo il piano della 5-quater** contro il codice, non usando. `Z5` produce la pagina della privacy e un **runbook** eseguito con il ruolo del migratore: la capacità vera resta fuori, e la ragione è nello schema. Il ruolo applicativo non ha il privilegio `DELETE` ([ADR-003](08-decisioni.md#adr-003), regola 7), tutto passa da `deleted_at`, e `audit_log` e gli eventi di dominio continuano a citare le righe — quindi una cancellazione vera non si soddisfa con un soft delete. È la stessa domanda di `R15` — chiusa dalla [sezione 7](11-piani-conclusi.md#esito-sezione-7), che ha toccato le righe di lavoro e nessun dato personale — posta su un altro perimetro: che cosa significhi il verbo su righe che altre tabelle nominano, e che cosa resti quando la persona non c'è più. Probabile una migration e un ADR; **non** è un dettaglio di una pagina. Si pianifica quando qualcuno la chiede davvero, e fino ad allora la pagina dichiara un tempo di risposta invece di promettere un bottone |

### Uscire da un'organizzazione

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R39 | aperta | **Un proprietario non può togliere nessuno dalla propria organizzazione** | Il gemello di `R16`, tenuto **fuori dalla sezione 6** perché è una decisione a sé e non un secondo verso della stessa. Metà del lavoro esiste già: `org_members_update` è `is_org_staff(org_id)` su USING e WITH CHECK, quindi la policy lo ammette e manca la rotta — e il divieto «non con un contratto in essere» userebbe `app.has_lease_in_force`, che la sezione 6 costruisce. Quello che va deciso è il resto: che ne è dei suoi contratti, dei suoi addebiti e dei messaggi che ha scritto; se sia un'espulsione o una chiusura concordata; e il fatto che qui il divieto va garantito su una persona che **non è chi chiama**, cioè il caso che la `0029` ha già trovato difficile sulle spese co-living. Oggi un proprietario che ha invitato la persona sbagliata non ha nessun modo di disfarlo se non revocare l'invito prima che venga accettato |
| R40 | aperta — **rinviata** il 22 settembre 2026 a dopo il deploy, sola fra le sedici: porta con sé la verifica dell'email, che cambia il percorso di registrazione e si prova solo con un relay vero | **La propria email non si cambia**, e non si verifica | Tenuta fuori dalla sezione 6 con la ragione scritta nel suo piano: `users.email` non è un dato anagrafico, è **la credenziale d'accesso**, ed è la chiave con cui tre cose si ritrovano — `app.has_pending_invitation` fa `JOIN app.users u ON u.email = i.email`, `app.current_user_email()` regge quattro policy della `0018`, e `lease_parties.email` è come l'accettazione trova la propria riga. Cambiarla senza un giro di verifica sul **nuovo** indirizzo produce un account che non riceve più niente o un invito che nessuno può più accettare, in entrambi i casi con un sintomo muto. La precondizione è quindi la **verifica dell'email**, che è deliberatamente fuori perimetro perché cambia il percorso di registrazione che ogni collaudo attraversa: le due voci si fanno insieme o non si fanno. Da decidere anche che cosa succede agli inviti pendenti per l'indirizzo vecchio |

### I contatti fra chi affitta e chi abita

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R43 | aperta | **Un inquilino non ha nessun modo di sapere come contattare chi gli affitta casa** | Chiesta **usando il prodotto**. È già vera per **metà**: il proprietario vede email e telefono dell'inquilino nella tabella delle parti sulla scheda del contratto — sono quelli che ha digitato lui per l'invito — e anche due coinquilini sullo **stesso** contratto si vedono fra loro, perché `lease_parties_select` è per contratto. A mancare è la direzione opposta, e i coinquilini con contratti **separati**. **Il perimetro è deciso: solo il rapporto contrattuale**, cioè l'inquilino vede i contatti di chi amministra la propria organizzazione; i coinquilini con contratti separati restano come sono. **Email e telefono**, non la sola email: per contattare chi ti affitta casa il numero è spesso più utile, è sulla stessa riga e non costa lavoro in più. La parte che sorprende è che **non serve nessuna policy e nessuna migration**: `users_select` ammette già `app.is_my_staff(id)` — la `0030` quel ramo lo ha aggiunto apposta *«per tenere in piedi il rapporto proprietario ↔ inquilino»* — quindi il confine è già dove serve, e a mancare è soltanto la **proiezione**. **Non emenda [ADR-036](08-decisioni.md#adr-036)**, e va detto perché sembrerebbe di sì: quella decisione governa il perimetro della **casa**, e sceglie di proiettare da `app.property_residents` il solo nome; questa voce sta sul ramo che la stessa migration ha tenuto aperto, e lo lascia intatto. La forma da riusare è [ADR-049](08-decisioni.md#adr-049): un modello di lettura che **proietta** le colonne scelte sotto `users_select`, mai una rotta che risolva le persone per identificatore — in pratica un `contacts(ids)` accanto a `UserRepository::names(ids)`, e **non** `select_user!`, che legge l'aggregato intero, hash della password compreso. **La trappola è che `is_my_staff` risponde per tutti**: restituisce vero per ogni `owner` e `manager` attivo di un'organizzazione a cui il chiamante appartiene, non per «il proprietario del **mio** contratto». Con un'organizzazione che ha più gestori, un elenco che li mostri tutti non dice chi contattare — e il legame con il contratto non passa da lì. Da decidere: se mostrare il solo `owner`, tutti, o risalire alla persona dal contratto. Da decidere anche **dove**: l'inquilino ha un contratto solo, quindi «chi amministra» ha una risposta sola, e la scheda del contratto è il candidato naturale. **Due cose vanno insieme al codice e si dimenticano**: l'[informativa sulla privacy](11-piani-conclusi.md#esito-utenti-veri) dichiara con chi si condividono i dati, e questo è una comunicazione di dato personale a un destinatario nuovo; e il perimetro scelto diventa **il pavimento** — allargarlo dopo è additivo, stringerlo è una rottura per chi ci contava. **ADR prima del codice** |

### La sessione e l'organizzazione attiva

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R53 | aperta | **Cambiare organizzazione dal selettore non svuota la cache delle query**: la schermata resta sui dati di quella lasciata | Vista **eseguendo la sezione 11**, leggendo `app/query.tsx` per `R49`. Le chiavi in `keys` non portano l'organizzazione attiva — `['dashboard']`, `['charges', filtri]` — e `QueryProvider` svuota la cache solo quando la sessione cade (`store.current === null`), non quando `selectOrganization` cambia `X-Org-Id`. Chi ha due organizzazioni — un inquilino con due case, chi è proprietario di una e inquilino in un'altra — sceglie la seconda dal menù e la schermata aperta **resta su quella di prima**: nessuna rilettura parte finché la stessa query resta montata, perché `staleTime` conta solo a un nuovo montaggio, e navigando altrove le chiavi già in cache rispondono con i dati vecchi per trenta secondi. È un `200` con i dati sbagliati, la forma peggiore di sbagliare, e la stessa che la regola 24 esiste per impedire. Due vie, da decidere: **le chiavi portano l'organizzazione** (`keys.charges(org, filtri)`), che è la forma giusta di React Query ma tocca ogni `useQuery` e ogni invalidazione; oppure **`queryClient.clear()` al cambio**, come al logout, che è una riga in `QueryProvider` e costa una rilettura di tutto — accettabile, perché cambiare organizzazione è un evento raro e cambia davvero tutto. Nel primo mese c'è un proprietario con un'organizzazione: non si presenta, ma è dentro il collaudo di [13](13-collaudo.md) appena una persona ha due ruoli |
| R42 | aperta | **La sessione non scade mai davvero, e non lo si è mai scelto.** Si vuole un «Ricordami per 30 giorni» | Trovata **controllando la sessione** dopo la sezione 6, non usando il prodotto: una scheda lasciata aperta un giorno con l'API spenta è rientrata da sé, ed è corretto — a tenerla dentro era il cookie del refresh. Controllando *perché*, è uscito che la finestra dei 30 giorni è **scorrevole**: `RefreshSession::issue` in `crates/application/src/use_cases/auth/refresh.rs` costruisce il successore con `expires_at = now + lifetime`, quindi i trenta giorni contano dall'**ultimo uso** e non dal login. Chi apre il prodotto una volta al mese resta dentro per sempre. Non è un difetto — è la forma comune — ma **non risulta scelto**, e [04](04-sicurezza.md) diceva «vita: 30 giorni», che si legge come un tetto: la descrizione è stata corretta, la decisione resta da prendere. **La decisione di prodotto c'è ed è questa: un «Ricordami per 30 giorni» al login, esplicito, e senza la spunta una sessione che finisce con il browser.** Quel che si sa già del lavoro: la durata è **già per riga** — `app.user_sessions.expires_at` — quindi **nessuna migration**; a essere globale è `RefreshTokenFactory::lifetime()`, che il caso d'uso del login dovrà ricevere come parametro invece di leggere dalla configurazione. Il cookie oggi porta **sempre** `Max-Age`, calcolato dalla vita residua del token (`session_reply` in `crates/api/src/routes/auth.rs`): senza la spunta va **omesso**, che è ciò che lo rende un cookie di sessione e lo fa morire con il browser. Servono un campo nel corpo del login, una casella nel form e una stringa nel catalogo. **La trappola è la rotazione**: se la finestra continua a scorrere, il bottone dice «30 giorni» e significa «per sempre» — cioè l'etichetta mente. Scegliere quella spunta implica quindi che il successore **non** superi la scadenza di chi lo precede, ed è la riga che si dimentica, perché oggi il codice fa il contrario e sembra giusto. Da decidere solo il resto: se il rinnovo silenzioso del `refresh` debba comportarsi allo stesso modo per entrambi i casi, e se serva anche un tetto assoluto per chi la spunta ce l'ha. **ADR prima del codice**: è una decisione sulla sicurezza, e cambia una proprietà che [04](04-sicurezza.md) descrive |

### Contratti: quel che la sezione 3 ha lasciato aperto

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R28 | aperta | **Il subentro: cambiare la composizione di un contratto attivo** | Nato dalla [sezione 3](11-piani-conclusi.md#esito-sezione-3), che ha chiuso la porta da cui si entrava per sbaglio: aggiungere una parte a un contratto attivo era ammesso e faceva smettere le quote di sommare a 100, quindi ora si rifiuta con `lease.parties_locked` ([ADR-046](08-decisioni.md#adr-046)). Il caso legittimo dietro resta: **un coinquilino che subentra a metà contratto**. Non è un'aggiunta — è un cambio di composizione che deve tenere la somma a 100, cioè **due righe che cambiano insieme**, e nessuna richiesta lo permette: `PATCH /lease-parties/{id}` ne tocca una sola, e in mezzo si passa da uno stato che l'attivazione rifiuterebbe. Da decidere la forma: una richiesta che porta **tutte** le parti insieme e le sostituisce, oppure una transizione dedicata («esce X, entra Y, con queste quote»). La seconda dice anche *quando*, che è ciò che serve agli addebiti. Legata a `left_at`, che il modello ha e nessuno scrive |

### Il perimetro dei ruoli

`R9` è uscita con la [sezione 8](11-piani-conclusi.md#esito-sezione-8) e
[ADR-053](08-decisioni.md#adr-053). Quel che resta è ciò che l'ADR ha lasciato
deliberatamente fuori e che l'esecuzione ha reso visibile.

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R48 | aperta | **Chi ha già un account e nessuna organizzazione non può diventare proprietario** | Trovata dal revisore dei passi `F6`+`F7`. `accept-owner-invitation` crea un utente nuovo e risponde `409 auth.email_taken` se l'email esiste; non c'è un'accettazione di un invito da proprietario *da un account esistente*. Nella schermata «nessuna organizzazione» il codice incollato va solo a `accept-invitation` (inquilino): un token da proprietario lì riceve `401` «codice non valido». Il caso è reale — un inquilino che compra casa, o un proprietario che è uscito dalla propria organizzazione — e ADR-053 non lo copre. La via naturale è una terza rotta, `Authenticated`, che accetti un invito da proprietario e crei l'organizzazione per l'utente corrente, con l'email dell'invito vincolata a quella dell'account; e il form di `/invito` da dentro sessione che, sul `401`, riprovi lì. Da decidere se prima del deploy |
