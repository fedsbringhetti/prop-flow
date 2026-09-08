# 12 — Da fare

Le cose da sistemare e da costruire, raccolte usando il prodotto e leggendo il codice.
**Ventisei voci**, con un identificatore stabile ciascuna.

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

## L'ordine di esecuzione

Deciso l'**8 settembre 2026**. Le voci si eseguono **per sezione, una per
conversazione**: è il vincolo che ha deciso i raggruppamenti, e non l'argomento.

Una sezione deve essere **autosufficiente** — chiudersi senza lasciare mezze cose a
quella dopo — perché chi la esegue apre una sessione nuova che non ha visto le
precedenti. Da qui due accorpamenti che per argomento non si farebbero:

- **`R13`, `R17` e `R18` stanno insieme** perché sono un nodo, non tre voci: una quota
  sbagliata non si corregge (`R17`), quindi il contratto non si attiva, quindi non si
  elimina (`R13`), perché l'unica uscita sarebbe attivarlo. Prenderne una senza le
  altre lascia il vicolo cieco aperto;
- **`R7`, `R21` e `R22` hanno la stessa radice** — i DTO portano identificatori e
  nessuna rotta risolve le persone — ma `R21` e `R22` restano con le segnalazioni,
  dove si vedono. La sezione 5 va quindi eseguita **prima** della 6, o la 6 rifarà
  quel lavoro.

| | Sezione | Voci | Natura | Perché è lì |
|---|---|---|---|---|
| **1** | Ritocchi visibili | `R5` `R4` `R6` `R10` `R20` + il bottone del thread | solo client | Nessuna decisione aperta e nessuna dipendenza: è la sola sezione che si può eseguire senza discutere niente. Restituisce un prodotto più usabile per continuare a provarlo, che è il modo in cui queste voci stanno uscendo. `R4` è qui e non altrove perché tocca `AuthPage.tsx`, lo **stesso file** di `R5`: aprire quei due form una volta invece di due |
| **2** | Le pendenze della fetta C | `R2` `R3` + il collaudo in undici punti | doc + due righe di backend | Chiude la fetta precedente invece di lasciarla a metà. `R2` è una correzione al testo del collaudo che va fatta **prima** di percorrerlo, o il punto 10 non prova ciò che dice |
| **3** | Contratti: correggere gli errori | `R13` `R17` `R18` `R12` `R14` | backend | **Il nodo**, e il danno peggiore che esiste adesso: in database ci sono già contratti attivi con le quote sbagliate e nessun modo di correggerli. `R12` e `R14` entrano perché si toccano gli stessi handler |
| **4** | Stati che nessuno aggiorna | `R1` `R11` `R25` | da decidere | Tre volte la stessa domanda — **derivato o scritto a mano?** — e conviene rispondere una volta sola. `R11` propende per derivato, `R1` per scritto, `R25` è una capacità che manca del tutto |
| **5** | Segnalazioni | `R19` `R19-bis` `R21` `R22` `R23` `R24` | migration + dominio | Tre voci su sei sono già **decise**. Porta una migration (`R19`), due regole di dominio che mancano (`R23`, `R24`) e il primo pezzo del problema dei nomi (`R21`, `R22`) |
| **6** | Le persone | `R7` `R16` | backend | Due rotte nuove sugli utenti: il proprio account e l'uscita da un'organizzazione. Dopo la 5, che avrà già affrontato come si risolve un nome |
| **7** | Cancellare, in generale | `R15` | client trasversale | Dopo la 3, perché finché un contratto non si elimina «elimina» non è un'operazione sola. Ha una decisione sua: che cosa significa il verbo su entità che si archiviano, si annullano o non si toccano, e che cosa vuol dire un esito parziale su N chiamate |
| **8** | Il perimetro dei ruoli | `R9` | modello + ADR | Da sola e per ultima fra quelle decise: tocca registrazione, inviti, ruoli e probabilmente una migration, e ha una faccia di sicurezza. **ADR prima del codice** |
| **9** | Spiegare come si usa il sito | `R8` | da decidere | Ultima **apposta**: scrivere spiegazioni su un'interfaccia che si sta ancora correggendo significa riscriverle. Quando le schermate hanno smesso di cambiare si decide *dove* sta la spiegazione — stati vuoti, pagina d'aiuto, primo accesso — che è la domanda prima di *cosa dice* |

**Si comincia dalla sezione 1.**

L'ordine non è inciso: se il collaudo o l'uso fanno emergere un danno più urgente, si
riordina. Quello che non si fa è **eseguire mezza sezione**, perché è ciò che lascia
alla conversazione successiva un contesto che non può ricostruire.

## L'elenco

### Dal collaudo e dalla lettura del codice

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R1 | aperta | **Un immobile non esce mai da `draft`** | `PATCH /properties/{id}` accetta `status` (`UpdatePropertyRequest`), e **nessuna schermata lo manda**: né la creazione né `EditPropertyDialog`. Niente lo deriva, quindi l'etichetta resta sbagliata per sempre. **Non blocca nulla** — nessun use case guarda lo stato dell'immobile. Da chiudere è un `<Select>` nel dialogo. Nota di metodo: l'audit della fetta C non poteva trovarlo, perché confronta quali *funzioni* nessuno chiama e `updateProperty` **è** chiamata, solo mai con quel campo |
| R2 | aperta | **Il punto 10 del collaudo non prova `C2` come è scritto** | Dice «il proprietario in una finestra, un inquilino nell'altra», e due account richiedono due contenitori di cookie: senza cookie condiviso non c'è contesa, e il punto passerebbe anche con il lock rimosso. La versione che lo esercita è **due schede dello stesso account nello stesso profilo, ricaricate insieme**. Correzione al testo in `11-piani-conclusi.md` |
| R3 | aperta | **Il messaggio di `C6` è invecchiato di un giorno** | Dice «nessuno lo carica da solo: lo fa la shell», e il `justfile` ora lo carica con `set dotenv-load`. Due righe in `crates/api/src/config.rs` |
| R4 | aperta | **`zod` rende `/accesso` la rotta iniziale più pesante** | 543,4 kB (172,4 gz) contro i 438,7 (141,6) del riepilogo: un chunk da 107,9 kB che nessun'altra rotta iniziale tira, sulla prima schermata che vede un utente nuovo. Già in [Debiti tecnici noti](09-stato-e-roadmap.md#debiti-tecnici-noti). Il rimedio è non usarlo **su quei due form**, dove le regole sono un'email, dodici caratteri e un campo obbligatorio |

### Interfaccia e prodotto

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R5 | aperta | **Mostrare la password**, in accesso e registrazione | `AuthPage.tsx`, i due `Input type="password"`. Il bottone va dentro il campo e deve restare raggiungibile da tastiera; l'etichetta cambia con lo stato e va nel catalogo, non nel JSX |
| R10 | aperta | **La creazione di una stanza offre meno campi della modifica** | `NewRoomDialog` chiede nome, canone, superficie e bagno privato; `RoomDialog` — la modale di `C4` — aggiunge *arredata*, e il `POST` ne accetterebbe altri ancora. Chi crea una stanza non ha modo di sapere che il resto esiste, e nemmeno che si può correggere dopo: il nome è unico per immobile, quindi «tanto lo sistemo dopo» è proprio la cosa da dire, e non è detta. Da allineare i due form e a mettere la frase nel primo — la stringa non esiste ancora nel catalogo |
| R6 | aperta | **La navigazione non si vede tutta**: su alcune schermate compare solo la metà superiore. Separatore e sezione dell'organizzazione vanno **in basso** | `#navigazione-principale` in `shell.tsx`. Oggi è un `<nav>` con la lista e, sotto, il selettore in un blocco `lg:block`: manca un'altezza piena con il selettore ancorato in fondo (`flex` in colonna, `min-h`, e la lista che scorre). Va guardato **anche a schermo stretto**, dove il menù è a scomparsa e il selettore sta già nella barra in alto |
| R7 | aperta | **Un posto per vedere e modificare il proprio account** | Non esiste, e **non esiste nemmeno l'endpoint**: nessuna rotta legge o scrive l'utente corrente. Questo punto è **backend prima che client** — a differenza di tutta la fetta C, che non ha toccato l'API. Da decidere quali campi (nome, email, telefono, password) e che cosa comporta cambiare un'email, che è l'identificatore d'accesso |
| R8 | aperta | **Spiegare come si usa il sito** e che cosa fanno le sezioni | Aperto per intero: non è deciso *dove* stia — primo accesso guidato, pagina d'aiuto, testi contestuali sugli stati vuoti, o una combinazione. Nota: gli stati vuoti **esistono già** e sono progettati (`@/components/state`), e alcuni portano già il gesto che li riempie: è il posto più economico dove far entrare una spiegazione, e va valutato prima di costruire una sezione d'aiuto a sé |

### Stati che nessuno aggiorna

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R11 | aperta | **Una stanza occupata resta «Disponibile»** | Osservato dalla scheda di un immobile: *Chi ci abita* mostra la persona **con la sua stanza**, e la tabella delle stanze accanto dice `available`. La causa non è di presentazione: `RoomStatus::Occupied` esiste nell'enum del dominio e in `app.room_status`, e **nessuno lo scrive mai** — nessun use case, nessun trigger. Il commento di `Room::create` in `crates/domain/src/property/room.rs` dichiara «lo stato passa a `Occupied` quando un contratto la impegna», e quella transizione **non è implementata**: è un invariante scritto e non mantenuto, che è peggio di uno assente perché chi legge il dominio lo dà per vero |

**R11 e [R1](#lelenco) sono la stessa famiglia** — un campo di stato che l'API accetta e che nessuno aggiorna — ma il rimedio non è lo stesso, e vale la pena non trattarli insieme senza deciderlo.

Su un immobile lo stato è un giudizio di chi amministra, quindi un campo nel form è la risposta giusta. **Su una stanza no**: che sia occupata è un *fatto*, e discende dai contratti. Metterci una tendina significherebbe chiedere a una persona di ricordarsi di aggiornare a mano qualcosa che il database sa già — e `app.property_residents` della migration `0030` lo calcola di fatto, unendo `lease_parties` a `leases` e a `rooms` per contratti attivi.

Le domande da sciogliere prima di scrivere codice, e sono la ragione per cui questa voce non è un rimedio di due righe:

- **derivato o scritto?** Una colonna aggiornata da un trigger sull'attivazione e sulla disdetta di un contratto, oppure uno stato calcolato al momento della lettura, che non può divergere ma non si può filtrare con un indice;
- **che cosa fa la disdetta**, e che cosa fa un contratto futuro: una stanza con una locazione che comincia fra un mese oggi è `available`, e domani?
- **`unavailable` resta manuale**, qualunque cosa si scelga: quello è un giudizio — una stanza in ristrutturazione — e non discende da nessun contratto. Un rimedio che derivi tutto lo cancellerebbe.

### Contratti: creare, disfare, riconoscere

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R12 | aperta | **Avvertire della sovrapposizione alla creazione**, non solo all'attivazione: se in quel periodo c'è già un contratto, «Crea» chiede una conferma in più | **È una modifica di solo client**, ed è la scoperta che rende la voce piccola. I due vincoli `leases_no_overlap_property` e `leases_no_overlap_room` della `0005` hanno una `WHERE` che copre `pending_signature`, `active` ed `expiring` — **e non `draft`**: le bozze sono escluse *apposta*, perché una bozza è lavoro in corso e due bozze che si sovrappongono sono legittime. Il `409` all'attivazione è quindi corretto e resta; quello che manca è dirlo prima, e il client può saperlo con `listLeases` filtrato sull'immobile confrontando i periodi. **Non toccare i vincoli**: allargarli a `draft` vieterebbe di abbozzare due ipotesi alternative sullo stesso periodo |
| R13 | aperta | **Un contratto creato per sbaglio non si elimina**: l'unica via è attivarlo e disdirlo | Le rotte dei contratti sono cinque — `POST /leases`, `GET /leases`, `GET /leases/{id}`, `POST .../parties`, `POST .../activate`, `POST .../terminate` — e **non c'è una `DELETE`**. La colonna `deleted_at` c'è, quindi il livello dati regge già la cancellazione soft: manca lo use case e la rotta. Da decidere **quali stati** siano archiviabili: una bozza certamente; un contratto attivo no, perché per quello la via è la disdetta, che è un fatto diverso e lascia traccia |
| R14 | aperta | **Dare un nome ai contratti**, perché in tabella sono tutti uguali | `app.leases` **non ha una colonna nome**: ci sono `notes` e `registration_number`, e l'elenco mostra tipo più data, da cui le righe identiche. Ci sono due vie e vanno pesate insieme, non scelte per prima: (a) una colonna `label`, cioè migration, dominio, DTO e form; (b) comporre l'etichetta da ciò che il contratto già è — immobile e stanza — che **non è gratis**, perché il DTO `Lease` porta `property_id` e `room_id` e **nessun nome**, quindi vorrebbe dire allargare la risposta o fare una richiesta per riga. La (b) risolve «sono tutti uguali»; solo la (a) permette di chiamarlo *«Rossi, secondo piano»* |

| R17 | aperta | **Le parti contrattuali non si modificano dopo**, nemmeno da chi amministra | C'è **solo** `POST /leases/{id}/parties`: nessuna `PATCH`, nessuna `DELETE`. Una parte si aggiunge e resta com'è — nome, email, telefono, ruolo, quota, intestatario principale. `app.lease_parties` ha però sia `left_at` sia `deleted_at`, quindi correggere e togliere sono **già rappresentabili** senza migration: mancano gli use case e le rotte |

**R17 e R13 insieme fanno un vicolo cieco, e va visto prima di stimarli separatamente.** L'attivazione pretende che le quote degli inquilini sommino a 100 (`lease.shares_not_hundred`). Se una parte viene inserita con la quota sbagliata: non la si può correggere (R17), quindi il contratto non si attiva; e non lo si può eliminare (R13), perché la sola uscita sarebbe attivarlo e disdirlo — che è precisamente ciò che non riesce. **Il contratto resta lì per sempre**, non attivabile e non rimovibile, e l'unico rimedio è rifarlo daccapo lasciando in elenco quello rotto — che con R14 non ancora fatto è pure indistinguibile dal buono.

È la ragione per cui queste due, se si sceglie di farle, conviene farle nella stessa fetta.

| R18 | aperta | **Una quota può finire a zero in silenzio**, e il contratto si attiva lo stesso | Vedi sotto: **osservato in database**, non dedotto |

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
4. **niente di tutto questo si corregge dopo** ([R17](#lelenco)): non si cambia una quota, non si toglie una parte.

**Le conseguenze non restano nel contratto.** `app.property_residents` proietta le parti dei contratti attivi, quindi la parte fantasma compare fra **chi abita l'immobile** — con il suo nome, accanto all'inquilino vero. Ed è anche una parte che, se un giorno accettasse, si troverebbe intestataria del 100%.

**Da decidere, e sono domande distinte:** se il form debba proporre 100 solo quando la parte è davvero l'unica; se una quota a zero vada segnalata al momento in cui si invita, invece che scoperta dopo; se l'attivazione debba pretendere qualcosa di più della somma — per esempio che nessun inquilino sia a zero, o che l'intestatario principale abbia accettato. **La correzione a posteriori (R17) serve comunque**, perché i contratti già rotti esistono già.

### Segnalazioni

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R19 | **decisa** | **La visibilità di una segnalazione diventa una scelta di chi la apre** — casella *«visibile a tutti gli inquilini»* — e il comportamento di oggi resta il default | **Decisa.** Oggi non è un difetto ma la policy: `app.can_access_ticket` (`0014`) concede a staff, a chi l'ha aperta, a chi l'ha in carico, e a `t.lease_id IS NOT NULL AND can_access_lease(...)` — quindi una segnalazione sull'immobile intero, che non ha contratto, resta privata. **Quel default resta**, e si aggiunge una scelta esplicita per il caso vero: la caldaia condominiale interessa a tutti. Serve una colonna su `app.tickets`, un `ALTER POLICY` con un ramo in più, il campo nel DTO e la casella nel form — quindi **una migration**, e non un filtro applicativo: la visibilità è un confine del database. Il predicato «chi guarda abita in questo immobile» va scelto fra ciò che la `0030` ha già introdotto (`app.shares_home_with`, `app.property_residents`) invece di scriverne un terzo. Da decidere se la casella si possa cambiare dopo l'apertura, il che la lega a R19-bis |
| R19-bis | **decisa** | **Chi ha aperto una segnalazione può modificarla finché nessuno l'ha presa in carico** | **Decisa.** La forma della regola **esiste già** ed è quella dell'annullamento: lo staff sempre, chi ha segnalato **solo finché è `open`** — «finché nessuno ci ha messo mano». Qui vale lo stesso confine, cioè fino ad `assigned`. La trappola sta nella rotta: **`PATCH /tickets/{id}` esiste già** ma è `update_costs`, ed è **solo staff** e sui soli campi di costo. Allargarla significherebbe una rotta con due autorizzazioni diverse a seconda dei campi presenti — costi allo staff, testo a chi ha segnalato — che è il genere di cosa che si sbaglia in silenzio. Da decidere: allargare con attenzione o una rotta a sé, e **quali campi** siano modificabili (titolo e descrizione sì; categoria e priorità sono il lavoro del triage, che è dello staff) |
| R20 | **decisa** | **«Prendi in carico» si nasconde a chi l'ha già presa in carico** | **Decisa, ed è solo client.** Oggi `assignable = open \|\| triaged \|\| assigned` in `TicketActions.tsx`: `assigned` è nell'elenco perché il bottone serve anche a **riassegnare**, e per questo non si toglie del tutto — sparisce quando `ticket.assigned_to` è chi guarda. Chi la vede assegnata a un altro continua a poterla prendere |
| R21 | aperta | **«In carico a» mostra la data e non chi** | `TicketResponse` porta `assigned_to` come **UUID** e `assigned_at`, e nessun nome. È lo stesso ceppo di [R14](#lelenco): i DTO trasportano identificatori, e i nomi delle persone non hanno una rotta — il debito «nessun endpoint elenca le persone di un'organizzazione» è aperto **apposta**, perché la fetta B aveva verificato che quel menù sarebbe vuoto avendo un'organizzazione esattamente uno staff. Qui però il nome serve davvero, e le vie sono le solite due: allargare la risposta della segnalazione, oppure una rotta che risolva le persone |
| R22 | aperta | **Nel thread non si capisce chi scrive**: si vorrebbe ruolo e nome — *Proprietario · Mario Rossi* | Stesso ceppo di R21: i messaggi portano l'autore come identificatore. Il **ruolo** però è più economico del nome — si deduce da chi è staff — e da solo risolve gran parte della domanda («me lo dice il proprietario o un coinquilino?»). Vale la pena separare le due metà: il ruolo si può fare senza toccare l'API, il nome no |
| R23 | aperta | **Su una segnalazione chiusa si possono ancora mandare messaggi** | Verificato: `PostTicketMessage` (`crates/application/src/use_cases/ticket/thread.rs`) fa `ensure_member` e **non guarda lo stato**. Nessun invariante lo vieta, quindi il thread di una segnalazione chiusa o annullata accetta scritture. È una **regola di dominio che manca**, non un controllo di interfaccia: nasconderlo nel client lascerebbe la rotta aperta. Da decidere se `closed` ammetta ancora una replica — chi ha segnalato potrebbe voler dire «non è risolto» — o se quella sia proprio la via di riapertura |
| R24 | aperta | **Riaprire dovrebbe chiedere il motivo** | `POST /tickets/{id}/reopen` **non accetta nulla**: né il client né l'handler hanno un corpo. La chiusura invece ha già `resolution_notes`, quindi il precedente esiste e la simmetria è naturale. Se R23 si chiude vietando i messaggi dopo la chiusura, questa diventa **l'unica via** per dire «non è risolto», e allora il motivo non è un di più: è l'unica cosa che spiega perché la segnalazione è tornata viva |

### Bollette

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R25 | aperta | **Una bolletta pagata non risulta pagata** | `app.bill_status` contiene già `'paid'` e `'disputed'`, e **nessuno li scrive**: le transizioni esposte sono validazione, riaddebito e annullamento. È la stessa famiglia di [R11](#lelenco) — uno stato che l'enum prevede e che niente aggiorna — ma qui manca una **capacità**, non un aggiornamento: pagare la bolletta **al fornitore** non è modellato affatto, ed è registrato da tre fette fra ciò che resta fuori dal modulo dei pagamenti. Da decidere se sia una transizione a mano di chi amministra («l'ho pagata»), o un pagamento vero con la sua riga: `app.payments` è polimorfico ma oggi punta agli addebiti, cioè al denaro che entra, mentre questo è denaro che esce |

#### Il thread non diventa live: un bottone «Aggiorna» — **deciso**

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
e conviene prenderla dopo [R9](#lelenco), che ridisegna comunque chi vede
che cosa.

### Cancellare, in generale

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R15 | aperta | **Selezionare righe in tabella e cancellarle in blocco**, e poter cancellare anche una sola riga | La parte di interfaccia è ripetibile — le tabelle passano già da `@/components/ui/table` — ma **«elimina» non è un'operazione sola**, ed è la cosa da sapere prima di disegnare un bottone unico: un immobile e una stanza si **archiviano** (`DELETE`, cancellazione soft), un addebito e una bolletta si **annullano** (`void`, che è un fatto contabile e non una cancellazione), un contratto oggi **non si tocca affatto** (R13). Un'azione di gruppo che chiami tutto «elimina» direbbe una cosa falsa su almeno due di questi. Va deciso anche che cosa fa una selezione multipla quando **una parte** fallisce: nessuna rotta accetta più identificatori, quindi sono N chiamate e un esito parziale è possibile |

### Uscire da un'organizzazione

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R16 | aperta | **Un inquilino deve poter uscire da un'organizzazione**, e l'operazione va negata se ha un contratto attivo | **Non esiste nessuna rotta**: su `/api/v1/organizations` c'è soltanto la `GET` che elenca le proprie. `app.org_members` ha però `status` e `deleted_at`, quindi l'uscita è già rappresentabile senza migration — è lo use case, la rotta e la regola a mancare. Il blocco richiesto è la parte interessante: «ha un contratto attivo» va verificato **e** garantito, e vale la lezione della `0029` sulle spese co-living, dove rendere l'uscita un confine vero ha richiesto una policy e non un `if`. Da decidere anche che cosa vede dopo: chi esce dall'unica organizzazione in cui abita resta con la propria, personale e vuota — cioè il caso di [ADR-045](08-decisioni.md#adr-045) al contrario |

### Il perimetro dei ruoli — **la voce più grande della lista**

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R9 | aperta | **Separare inquilini e proprietari**: accesso unico, **registrazione libera solo per gli inquilini**; un proprietario si fa verificare fuori dal prodotto e riceve un invito per registrarsi. Un inquilino non deve poter creare immobili | Vedi sotto: non è una fetta di client |

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
