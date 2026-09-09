# 12 — Da fare

Le cose da sistemare e da costruire, raccolte usando il prodotto e leggendo il codice.
**Nove voci**, con un identificatore stabile ciascuna.

Venti sono uscite: `R4`, `R5`, `R6`, `R10`, `R20` e il bottone del thread con la
[sezione 1](11-piani-conclusi.md#esito-sezione-1); `R2` e `R3` con la
[sezione 2](11-piani-conclusi.md#esito-sezione-2); `R12`, `R13`, `R14`, `R17` e `R18` con
la [sezione 3](11-piani-conclusi.md#esito-sezione-3); `R1`, `R11` e `R25` con la
[sezione 4](11-piani-conclusi.md#esito-sezione-4); `R19`, `R19-bis`, `R21`, `R22`, `R23` e
`R24` con la [sezione 5](11-piani-conclusi.md#esito-sezione-5). Quattro sono entrate —
`R26`, `R27`, `R28`, `R29` — e **tre delle quattro le ha trovate l'uso del prodotto**, non
la lettura del codice. `R29` è entrata **preparando** il collaudo, prima che cominciasse:
seminare dei dati veri è già usare il prodotto.

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
| ~~**1**~~ | ~~Ritocchi visibili~~ — **conclusa** l'8 settembre 2026 | `R5` `R4` `R6` `R10` `R20` + il bottone del thread | solo client | Le sei voci sono uscite da questo file: [piano](11-piani-conclusi.md#piano-sezione-1) · [esito](11-piani-conclusi.md#esito-sezione-1). La riga resta perché l'ordine si legge per intero, e perché l'esito dice una cosa che serve alle sezioni dopo: la navigazione richiedeva **tre** modifiche e non due |
| ~~**2**~~ | ~~Le pendenze della fetta C~~ — **conclusa** il 9 settembre 2026, **senza il collaudo** | `R2` `R3` | doc + una stringa di backend | [Esito](11-piani-conclusi.md#esito-sezione-2). `R2` ha corretto il punto 10 e, rileggendo gli altri dieci, ha trovato **il punto 4** con lo stesso difetto; `R3` era in **otto** posti e non in due. Il collaudo si è staccato ed è diventato la **5-bis** |
| ~~**3**~~ | ~~Contratti: correggere gli errori~~ — **conclusa** il 9 settembre 2026 | `R13` `R17` `R18` `R12` `R14` | backend | Era il nodo e il danno peggiore che esistesse: contratti attivi con le quote sbagliate e nessun modo di correggerli. [Piano](11-piani-conclusi.md#piano-sezione-3) · [esito](11-piani-conclusi.md#esito-sezione-3). Eseguirla ha chiuso **un buco che non era in elenco** — si poteva aggiungere una parte a un contratto attivo — e ne ha aperto uno nuovo: `R28`, il subentro |
| ~~**4**~~ | ~~Stati che nessuno aggiorna~~ — **conclusa** il 9 settembre 2026 | `R1` `R11` `R25` | migration + dominio + client | [Piano](11-piani-conclusi.md#piano-sezione-4) · [esito](11-piani-conclusi.md#esito-sezione-4). La risposta era già in [ADR-019](08-decisioni.md#adr-019): le conseguenze si derivano. Ha corretto **un numero falso che nessuno guardava** — `v_property_occupancy` diceva zero a tutti — e ha lasciato in un test la prova che un trigger sarebbe stato sbagliato |
| ~~**5**~~ | ~~Segnalazioni~~ — **conclusa** il 9 settembre 2026 | `R19` `R19-bis` `R21` `R22` `R23` `R24` | migration + dominio | Le sei voci sono uscite da questo file: [piano](11-piani-conclusi.md#piano-sezione-5) · [esito](11-piani-conclusi.md#esito-sezione-5). **Due** migration: la `0035` allarga chi *vede*, la `0036` chi *scrive* — e la seconda **revoca** [ADR-026](08-decisioni.md#adr-026) su due colonne, emendando nello stesso passo l'invariante di schema e il test che la difendevano. Ha reso **portante** `users_select`: è il primo client che legge un nome da `app.users` |
| **5-bis** | **Il collaudo in undici punti** — **è il prossimo** | — | a mano, due account | **Rinviato qui il 9 settembre 2026**, dopo averne percorsi due punti. Sei degli undici attraversano schermate che le sezioni 3, 4, 5, 8 e 9 riscrivono: camminarli prima vuol dire camminarli due volte. Dopo la 5 ne restano fermi nove su undici — l'1 e il 5 dipendono da `R9`, che è la sezione 8, e aspettare anche quella vorrebbe dire non provare niente per cinque sezioni. **Il piano è già scritto**: passi `T3`–`T7` dell'[esito della sezione 2](11-piani-conclusi.md#piano-sezione-2), con le sette trappole e la finta bolletta. Il costo del rinvio è che il giro **alimenta i piani** e non solo li verifica: `R26` è entrata così |
| **6** | Le persone | `R7` `R16` | backend | Due rotte nuove sugli utenti: il proprio account e l'uscita da un'organizzazione. La 5 ha già affrontato come si risolve un nome, ed è il modello da riusare: un modello di lettura che proietta `full_name` sotto `users_select`, non una rotta che risolva le persone ([ADR-049](08-decisioni.md#adr-049)) |
| **7** | Cancellare, in generale | `R15` | client trasversale | Dopo la 3, perché finché un contratto non si elimina «elimina» non è un'operazione sola. Ha una decisione sua: che cosa significa il verbo su entità che si archiviano, si annullano o non si toccano, e che cosa vuol dire un esito parziale su N chiamate |
| **8** | Il perimetro dei ruoli | `R9` | modello + ADR | Da sola e per ultima fra quelle decise: tocca registrazione, inviti, ruoli e probabilmente una migration, e ha una faccia di sicurezza. **ADR prima del codice** |
| **9** | Spiegare come si usa il sito | `R8` | da decidere | Ultima **apposta**: scrivere spiegazioni su un'interfaccia che si sta ancora correggendo significa riscriverle. Quando le schermate hanno smesso di cambiare si decide *dove* sta la spiegazione — stati vuoti, pagina d'aiuto, primo accesso — che è la domanda prima di *cosa dice* |

**Concluse la 1, la 2, la 3, la 4 e la 5: si continua dalla 5-bis, cioè dal collaudo.**

Non è in fondo alla fila per dimenticanza, e adesso è il suo turno: nove punti su undici
sono su codice fermo, e le sezioni 3, 4 e 5 sono state verificate **solo con i test**.

L'ordine non è inciso: se il collaudo o l'uso fanno emergere un danno più urgente, si
riordina. Quello che non si fa è **eseguire mezza sezione**, perché è ciò che lascia
alla conversazione successiva un contesto che non può ricostruire.

## L'elenco

### Dal collaudo e dalla lettura del codice

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|

### Interfaccia e prodotto

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R7 | aperta | **Un posto per vedere e modificare il proprio account** | Non esiste, e **non esiste nemmeno l'endpoint**: nessuna rotta legge o scrive l'utente corrente. Questo punto è **backend prima che client** — a differenza di tutta la fetta C, che non ha toccato l'API. Da decidere quali campi (nome, email, telefono, password) e che cosa comporta cambiare un'email, che è l'identificatore d'accesso. **La metà «come si risolve un nome» è già decisa** dalla sezione 5: un modello di lettura che proietta `full_name` sotto `users_select`, mai una rotta che risolva le persone ([ADR-049](08-decisioni.md#adr-049)) — e da quella sezione quella policy è portante, quindi il proprio account la deve rispettare invece di aggirarla |
| R8 | aperta | **Spiegare come si usa il sito** e che cosa fanno le sezioni | Aperto per intero: non è deciso *dove* stia — primo accesso guidato, pagina d'aiuto, testi contestuali sugli stati vuoti, o una combinazione. Nota: gli stati vuoti **esistono già** e sono progettati (`@/components/state`), e alcuni portano già il gesto che li riempie: è il posto più economico dove far entrare una spiegazione, e va valutato prima di costruire una sezione d'aiuto a sé |

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
| R29 | aperta | **A chi abita, un incasso non attribuito fa vedere un residuo più alto del vero** — e la riga si contraddice da sola | Trovato seminando i dati del collaudo, e **misurato**: il proprietario registra 150,00 su un addebito da 420,00 con `POST /charges/{id}/payments` senza `payer_user_id`, e sulla **stessa riga** l'inquilino legge `status: partially_paid` con `paid_amount: 0.00` e `outstanding_amount: 420.00`. Pagherebbe due volte. La causa è a due piani: `payments_select` mostra la riga a staff, `payer_user_id` e `payee_user_id`, quindi un incasso non attribuito è invisibile a chi lo ha fatto — e questo è **documentato e voluto** (`submit.rs`: «un bonifico da un conto che non si sa a chi ricondurre esiste»); ma `app.v_charge_balances` è `security_invoker = true`, quindi **somma i pagamenti sotto la Row Level Security di chi legge**, e il totale scende con essa. Il commento di `RecordPaymentRequest` promette il contrario — «il totale incassato gli resta comunque visibile in `paid_amount` sul saldo dell'addebito» — ed è **falso**: quella frase è la cosa da correggere per prima, perché è ciò che farebbe concludere a chi passa di lì che il difetto non esiste. Da decidere il rimedio, e sono tre strade diverse: rendere `payer_user_id` obbligatorio quando l'addebito ha un contratto con un solo inquilino registrato; oppure far sommare i pagamenti a una funzione `SECURITY DEFINER` che proietta il solo totale, come `app.property_residents` proietta tre colonne ([ADR-036](08-decisioni.md#adr-036)); oppure accettare che il totale sia parziale e **non mostrarlo affatto** a chi non vede tutte le righe, che è preferibile a mostrarne uno falso. La prima è la più piccola e non chiude il caso del bonifico anonimo; la seconda lo chiude e tocca una vista contabile; la terza è la sola che non richiede di scegliere adesso |

### Cancellare, in generale

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R15 | aperta | **Selezionare righe in tabella e cancellarle in blocco**, e poter cancellare anche una sola riga | La parte di interfaccia è ripetibile — le tabelle passano già da `@/components/ui/table` — ma **«elimina» non è un'operazione sola**, ed è la cosa da sapere prima di disegnare un bottone unico: un immobile e una stanza si **archiviano** (`DELETE`, cancellazione soft), un addebito e una bolletta si **annullano** (`void`, che è un fatto contabile e non una cancellazione), un contratto si **annulla** finché è una bozza, e da attivo si disdice soltanto (sezione 3). Un'azione di gruppo che chiami tutto «elimina» direbbe una cosa falsa su almeno due di questi. Va deciso anche che cosa fa una selezione multipla quando **una parte** fallisce: nessuna rotta accetta più identificatori, quindi sono N chiamate e un esito parziale è possibile |

### Uscire da un'organizzazione

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R16 | aperta | **Un inquilino deve poter uscire da un'organizzazione**, e l'operazione va negata se ha un contratto attivo | **Non esiste nessuna rotta**: su `/api/v1/organizations` c'è soltanto la `GET` che elenca le proprie. `app.org_members` ha però `status` e `deleted_at`, quindi l'uscita è già rappresentabile senza migration — è lo use case, la rotta e la regola a mancare. Il blocco richiesto è la parte interessante: «ha un contratto attivo» va verificato **e** garantito, e vale la lezione della `0029` sulle spese co-living, dove rendere l'uscita un confine vero ha richiesto una policy e non un `if`. Da decidere anche che cosa vede dopo: chi esce dall'unica organizzazione in cui abita resta con la propria, personale e vuota — cioè il caso di [ADR-045](08-decisioni.md#adr-045) al contrario |

### Dal collaudo della fetta C

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R26 | aperta | **Gli elenchi offrono filtri per stati che non possono comparire**, e sono tre elenchi con tre cause diverse | Trovato al punto 2 del collaudo archiviando un immobile e non ritrovandolo nemmeno filtrando per «Archiviato». **Immobili**: archiviare fa due cose nello stesso `UPDATE` (`soft_delete` in `crates/infrastructure/src/db/repositories/property.rs`) — `status = 'archived'` **e** `deleted_at` — ma `list` filtra `deleted_at IS NULL`, quindi la riga esce da ogni elenco, quel filtro compreso; e `find_by_id` ha lo stesso filtro, quindi nemmeno l'URL della scheda funziona più (`404`). Le altre quattro opzioni di quel `<Select>` sono vuote per la ragione di [R1](#lelenco). **Contratti**: l'opzione «Annullato» non può comparire perché `Lease::cancel()` **esiste nel dominio e non la chiama nessuno** — nessuno use case, nessuna rotta. **Chiuso dalla [sezione 3](11-piani-conclusi.md#esito-sezione-3)**, che ha esposto quella transizione: resta la faccia degli immobili e quella delle bollette. **Bollette**: «Pagata» e «Contestata» sono gli stati di [R25](#lelenco), che nessuno scrive. Le tre cause sono diverse ma la domanda è una sola, e conviene rispondere una volta: **un filtro deve offrire solo ciò che può comparire**, e dove lo stato è vero ma la riga è nascosta serve un modo di includerla. **Non toccare l'archiviazione**: che sia un soft delete è giusto |
| R27 | aperta | **`README.md` e [06](06-sviluppo.md) insegnano ancora il primo avvio a mano** | Trovato scrivendo `R3`. Le due pagine elencano `docker compose up`, `generate-jwt-keys.sh`, il `psql` dei ruoli e i tre `cargo run`, mentre `just setup` fa la stessa cosa in un comando e `just api`/`just worker`/`just web` avviano. Non è sbagliato — quei comandi funzionano, e `CLAUDE.md` §4 dice che la fonte della verità è lui — ma è **più lungo e più facile da sbagliare** proprio nella pagina che un estraneo legge per prima. Da decidere se il `justfile` diventi la via raccomandata anche lì, o se resti uno strumento di comodo per chi già conosce il progetto |

### Contratti: quel che la sezione 3 ha lasciato aperto

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R28 | aperta | **Il subentro: cambiare la composizione di un contratto attivo** | Nato dalla [sezione 3](11-piani-conclusi.md#esito-sezione-3), che ha chiuso la porta da cui si entrava per sbaglio: aggiungere una parte a un contratto attivo era ammesso e faceva smettere le quote di sommare a 100, quindi ora si rifiuta con `lease.parties_locked` ([ADR-046](08-decisioni.md#adr-046)). Il caso legittimo dietro resta: **un coinquilino che subentra a metà contratto**. Non è un'aggiunta — è un cambio di composizione che deve tenere la somma a 100, cioè **due righe che cambiano insieme**, e nessuna richiesta lo permette: `PATCH /lease-parties/{id}` ne tocca una sola, e in mezzo si passa da uno stato che l'attivazione rifiuterebbe. Da decidere la forma: una richiesta che porta **tutte** le parti insieme e le sostituisce, oppure una transizione dedicata («esce X, entra Y, con queste quote»). La seconda dice anche *quando*, che è ciò che serve agli addebiti. Legata a `left_at`, che il modello ha e nessuno scrive |

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
