# 12 — Da fare

Le cose da sistemare e da costruire, raccolte usando il prodotto e leggendo il codice.
**Quattordici voci aperte**, con un identificatore stabile ciascuna — `R35` è entrata e si è dissolta nello stesso giorno, dentro `R31`.

Venti sono uscite: `R4`, `R5`, `R6`, `R10`, `R20` e il bottone del thread con la
[sezione 1](11-piani-conclusi.md#esito-sezione-1); `R2` e `R3` con la
[sezione 2](11-piani-conclusi.md#esito-sezione-2); `R12`, `R13`, `R14`, `R17` e `R18` con
la [sezione 3](11-piani-conclusi.md#esito-sezione-3); `R1`, `R11` e `R25` con la
[sezione 4](11-piani-conclusi.md#esito-sezione-4); `R19`, `R19-bis`, `R21`, `R22`, `R23` e
`R24` con la [sezione 5](11-piani-conclusi.md#esito-sezione-5); `R30`, `R31`, `R32`, `R33`
e `R34` con la [sezione 5-ter](11-piani-conclusi.md#esito-sezione-5-ter); `R7`, `R16` e
`R38` con la [sezione 6](11-piani-conclusi.md#esito-sezione-6). Sono entrate
`R26`, `R27`, `R28` e `R29`, poi `R30`–`R35` dal collaudo, e infine `R36` **eseguendo** la
5-ter, `R37` **rivedendo** il piano della 5-quater contro il codice e `R38` **verificandone
le premesse** prima di pianificare la sezione 6, con `R39` e `R40` uscite dalla stessa
pianificazione come ciò che quella sezione decide di **non** fare — e `R41` **eseguendo** la
6, come conseguenza di ciò che quella sezione ha costruito, `R42` **controllando la
sessione** dopo averla conclusa, e `R43` **usando** il prodotto.

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
| ~~**5-bis**~~ | ~~Il collaudo in undici punti~~ — **concluso** il 10 settembre 2026, per intero | — | a mano, due account | [Esito](11-piani-conclusi.md#esito-sezione-5-bis), e con esso l'[esito della fetta C](11-piani-conclusi.md#esito-fetta-c) **smette di essere aperto** dopo due giorni e cinque sezioni. **Nessuno degli undici punti si è fermato**: le sei voci che ne sono uscite — `R30`–`R35` — sono difetti dentro percorsi che funzionano. Il rendimento che il rinvio aveva sospeso è misurato: mezza giornata di giro ha prodotto sei voci, cinque sezioni di lettura del codice ne avevano prodotte quattro. E una, `R31`, **nessun test poteva trovarla**: ogni test costruisce un inquilino nuovo, e quel difetto vive nella seconda volta che qualcosa succede alla stessa persona |
| ~~**5-ter**~~ | ~~Quello che il collaudo ha trovato~~ — **conclusa** il 10 settembre 2026 | `R30` `R31` `R32` `R33` `R34` | dominio + client | [Piano](11-piani-conclusi.md#piano-sezione-5-ter) · [esito](11-piani-conclusi.md#esito-sezione-5-ter). Un ADR e **nessuna migration**: il ramo che fa rientrare un inquilino passa dal primo ramo di `org_members_select`, che non chiede di essere già membri. Il piano aveva una premessa sbagliata — l'intestatario **non** era in `LeaseResponse` — e una decisione giusta: si è emendata la premessa con un terzo campo nella stessa query. Apre `R36`, il badge degli addebiti, che ha la stessa forma di `R30` e non lo stesso difetto |
| ~~**5-quater**~~ | ~~Prima degli utenti veri~~ — **conclusa** il 10 settembre 2026, **due sezioni su quattro eseguite** | — | posta + dominio + migration | [Piano](11-piani-conclusi.md#piano-utenti-veri) · [esito](11-piani-conclusi.md#esito-utenti-veri). `Z1` e `Z2` sono codice e girano; `Z3`, `Z4` e `Z5` hanno prodotto **artefatti e un runbook** e non sono stati eseguiti — servono una VPS, un dominio e delle credenziali. **Non è deployato**, e il criterio è il punto 4 del [runbook](07-operazioni.md#il-primo-deploy--runbook): aprire il prodotto dal telefono, in 4G. Apre `R37`, la cancellazione come capacità del prodotto |
| ~~**6**~~ | ~~Le persone~~ — **conclusa** l'11 settembre 2026 | `R7` `R16` `R38` | migration + backend + client | [Piano](11-piani-conclusi.md#piano-sezione-6) · [esito](11-piani-conclusi.md#esito-sezione-6). **Due migration e nessun ADR**, ed era una conclusione del piano: la revoca delle sessioni è stata resa uniforme ad [ADR-052](08-decisioni.md#adr-052) apposta per non doverne scrivere un secondo. È la prima sezione pianificata **leggendo il codice** in cui tutte e cinque le trappole previste si sono avverate; quello che il piano non aveva visto è altrove, e la voce più cara è che il divieto di uscire con un contratto in essere sarebbe valso **solo per chi non amministra** — le policy permissive si sommano in `OR`. Apre `R41` |
| **7** | Cancellare, in generale | `R15` | client trasversale | Dopo la 3, perché finché un contratto non si elimina «elimina» non è un'operazione sola. Ha una decisione sua: che cosa significa il verbo su entità che si archiviano, si annullano o non si toccano, e che cosa vuol dire un esito parziale su N chiamate |
| **8** | Il perimetro dei ruoli | `R9` | modello + ADR | Da sola e per ultima fra quelle decise: tocca registrazione, inviti, ruoli e probabilmente una migration, e ha una faccia di sicurezza. **ADR prima del codice** |
| **9** | Spiegare come si usa il sito | `R8` | da decidere | Ultima **apposta**: scrivere spiegazioni su un'interfaccia che si sta ancora correggendo significa riscriverle. Quando le schermate hanno smesso di cambiare si decide *dove* sta la spiegazione — stati vuoti, pagina d'aiuto, primo accesso — che è la domanda prima di *cosa dice* |
| **10** | **Il primo deploy** — [runbook](07-operazioni.md#il-primo-deploy--runbook) | — | a mano, su una macchina | **Rinviato per mancanza di mezzi e non di priorità**: servono una VPS, un dominio, un relay SMTP con SPF, DKIM e DMARC e delle credenziali. Gli artefatti sono scritti e validati per quello che si può validare senza — `docker compose config`, `bash -n` — e non sono mai stati eseguiti. Il criterio di fine è il punto 4 del runbook: **aprire il prodotto dal telefono, in 4G**. Prima del deploy va provato il **ripristino**, una volta, mentre dentro non ci sono ancora i dati di nessuno |

**Concluse la 1, la 2, la 3, la 4, la 5, il collaudo (5-bis), la 5-ter, la 5-quater e la
6.** Si continua dalla **7**: il [runbook del primo deploy](07-operazioni.md#il-primo-deploy--runbook)
è **rinviato in fondo**, e la ragione sta qui sotto.

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
adesso non arriva. L'ordine 6 → 7 → 8 → 9 resta quello scritto sotto, ma è **scelto senza
quella prova**: chi lo esegue non ha una misura dietro, ha un ragionamento. Il rendimento
del collaudo — sei voci in mezza giornata contro le quattro di cinque sezioni di lettura —
dice che il modo più economico di riprendere quella prova, **senza aspettare la VPS**, è
un secondo giro in locale: `mailpit` mostra la posta su `localhost:8025`, e l'unica cosa
che resta fuori è «arriva fuori dallo spam».

**Che cosa «pronto» significa, e per chi.** La lettura scelta è *un proprietario che
conosci*, non *chiunque si registri*: `R9` resta quindi in fondo, e con esso il fatto che
oggi chiunque si registri è owner di una propria organizzazione. È tollerabile perché
scritto, e smette di esserlo il giorno in cui la registrazione è aperta a degli
sconosciuti.

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

### Dal collaudo, traccia A e B — la prima mezza giornata di giro vero

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R36 | aperta | **Il doppio badge resta su un addebito scaduto**: «Scaduto (Emesso)» | Trovata **eseguendo la sezione 5-ter**, che ha corretto la stessa regola su contratti, stanze e immobili e ha lasciato fuori gli addebiti. `ChargeStatusBadge` in `apps/web/src/features/shared/status.tsx` mostra ancora la coppia ogni volta che differisce. La coppia **non è del tutto rumore**, ed è la ragione per cui la sezione non l'ha accorpata al volo: `Charge::effective_status` diverge solo da uno stato `is_outstanding()`, cioè da `issued` **o** da `partially_paid`, e «parzialmente pagato» non si ricostruisce da «scaduto» — mentre «(Emesso)» accanto a «Scaduto» è esattamente `R30`. Il rimedio è quindi asimmetrico: `partially_paid` resta accanto al derivato, `issued` sparisce. Serve decidere anche *come* dirlo, perché «Scaduto» e «parzialmente pagato» sono due fatti veri insieme e la parentesi non è più la forma giusta |
| R35 | ~~aperta~~ **si dissolve in `R31`** | ~~Che cosa vede l'inquilino di un contratto non ancora attivo~~ | **Riprodotto il 9 settembre 2026, e la premessa era sbagliata.** Un inquilino invitato su un contratto **in bozza** accetta (`200`) e **lo vede**, marcato `draft`: non esiste nessuna regola che nasconda le bozze a chi è collegato. Il contratto che non compariva non compariva perché la persona **non era collegata** — `lease_parties.user_id` lo scrive solo l'accettazione, e per chi è già membro quella fallisce con `409 membership.already_exists`. È `R31` visto dall'altro lato, ed è la sua conferma dal vivo. Resta una sola domanda, piccola e di presentazione: se la parola «Bozza» — che è lo stato interno di chi amministra — sia quella giusta da mostrare a chi abiterà lì, o se serva «In attesa di attivazione». Va con [R8](#interfaccia-e-prodotto), non con la sezione 5-ter. Comportamento di oggi, per il resto verificato: `InviteTenant` **non guarda lo stato** del contratto, quindi si invita anche su una bozza; `AcceptInvitation` collega la parte; `app.can_access_lease` non filtra lo stato. L'inquilino quindi **vede** il contratto, marcato «Bozza». Le letture possibili sono tre e portano a lavori opposti: che non debba vederlo finché non è attivo; che debba vederlo ma non con la parola «Bozza», che è lo stato interno di chi amministra e a lui non dice niente; oppure che vada bene così. Da chiarire prima di pianificare, ed è la ragione per cui questa voce non entra nella sezione con le altre cinque |

### Cancellare, in generale

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R15 | aperta | **Selezionare righe in tabella e cancellarle in blocco**, e poter cancellare anche una sola riga | La parte di interfaccia è ripetibile — le tabelle passano già da `@/components/ui/table` — ma **«elimina» non è un'operazione sola**, ed è la cosa da sapere prima di disegnare un bottone unico: un immobile e una stanza si **archiviano** (`DELETE`, cancellazione soft), un addebito e una bolletta si **annullano** (`void`, che è un fatto contabile e non una cancellazione), un contratto si **annulla** finché è una bozza, e da attivo si disdice soltanto (sezione 3). Un'azione di gruppo che chiami tutto «elimina» direbbe una cosa falsa su almeno due di questi. Va deciso anche che cosa fa una selezione multipla quando **una parte** fallisce: nessuna rotta accetta più identificatori, quindi sono N chiamate e un esito parziale è possibile |
| R37 | aperta | **Cancellare i dati di una persona su sua richiesta non è un'operazione del prodotto** | Riconosciuta **rivedendo il piano della 5-quater** contro il codice, non usando. `Z5` produce la pagina della privacy e un **runbook** eseguito con il ruolo del migratore: la capacità vera resta fuori, e la ragione è nello schema. Il ruolo applicativo non ha il privilegio `DELETE` ([ADR-003](08-decisioni.md#adr-003), regola 7), tutto passa da `deleted_at`, e `audit_log` e gli eventi di dominio continuano a citare le righe — quindi una cancellazione vera non si soddisfa con un soft delete. È la stessa domanda di `R15`, qui sopra, posta su un altro perimetro: che cosa significhi il verbo su righe che altre tabelle nominano, e che cosa resti quando la persona non c'è più. Probabile una migration e un ADR; **non** è un dettaglio di una pagina. Si pianifica quando qualcuno la chiede davvero, e fino ad allora la pagina dichiara un tempo di risposta invece di promettere un bottone |

### Uscire da un'organizzazione

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R39 | aperta | **Un proprietario non può togliere nessuno dalla propria organizzazione** | Il gemello di `R16`, tenuto **fuori dalla sezione 6** perché è una decisione a sé e non un secondo verso della stessa. Metà del lavoro esiste già: `org_members_update` è `is_org_staff(org_id)` su USING e WITH CHECK, quindi la policy lo ammette e manca la rotta — e il divieto «non con un contratto in essere» userebbe `app.has_lease_in_force`, che la sezione 6 costruisce. Quello che va deciso è il resto: che ne è dei suoi contratti, dei suoi addebiti e dei messaggi che ha scritto; se sia un'espulsione o una chiusura concordata; e il fatto che qui il divieto va garantito su una persona che **non è chi chiama**, cioè il caso che la `0029` ha già trovato difficile sulle spese co-living. Oggi un proprietario che ha invitato la persona sbagliata non ha nessun modo di disfarlo se non revocare l'invito prima che venga accettato |
| R41 | aperta | **Chi è uscito non può chiedere di rientrare** | Nata **eseguendo la sezione 6**, come conseguenza diretta di ciò che quella sezione ha costruito. Chi esce rientra accettando un invito nuovo, e l'invito lo emette chi amministra: non c'è nessuna via per cui una persona torni di propria iniziativa, né per cui segnali di volerlo. È voluto ed è la stessa «via sola» di [ADR-051](08-decisioni.md#adr-051) — due meccanismi per lo stesso fatto obbligherebbero il proprietario a ricordarsi se quella persona sia già stata sua inquilina — quindi **la voce non è «serve una rotta»**. L'avviso prima dell'uscita lo dice già (*«Se qualcuno ti reinvita potrai rientrare»*), e ciò che resta scoperto è **dopo**: chi è uscito e ci ripensa non ha niente da guardare, e `membership.already_left` su un secondo tentativo non spiega la via. Probabilmente una riga di testo, e va decisa guardando la schermata — non si pianifica prima di un giro a mano |
| R40 | aperta | **La propria email non si cambia**, e non si verifica | Tenuta fuori dalla sezione 6 con la ragione scritta nel suo piano: `users.email` non è un dato anagrafico, è **la credenziale d'accesso**, ed è la chiave con cui tre cose si ritrovano — `app.has_pending_invitation` fa `JOIN app.users u ON u.email = i.email`, `app.current_user_email()` regge quattro policy della `0018`, e `lease_parties.email` è come l'accettazione trova la propria riga. Cambiarla senza un giro di verifica sul **nuovo** indirizzo produce un account che non riceve più niente o un invito che nessuno può più accettare, in entrambi i casi con un sintomo muto. La precondizione è quindi la **verifica dell'email**, che è deliberatamente fuori perimetro perché cambia il percorso di registrazione che ogni collaudo attraversa: le due voci si fanno insieme o non si fanno. Da decidere anche che cosa succede agli inviti pendenti per l'indirizzo vecchio |

### Dal collaudo della fetta C

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R26 | aperta | **Gli elenchi offrono filtri per stati che non possono comparire**, e sono tre elenchi con tre cause diverse | Trovato al punto 2 del collaudo archiviando un immobile e non ritrovandolo nemmeno filtrando per «Archiviato». **Immobili**: archiviare fa due cose nello stesso `UPDATE` (`soft_delete` in `crates/infrastructure/src/db/repositories/property.rs`) — `status = 'archived'` **e** `deleted_at` — ma `list` filtra `deleted_at IS NULL`, quindi la riga esce da ogni elenco, quel filtro compreso; e `find_by_id` ha lo stesso filtro, quindi nemmeno l'URL della scheda funziona più (`404`). Le altre quattro opzioni di quel `<Select>` sono vuote per la ragione di [R1](#lelenco). **Contratti**: l'opzione «Annullato» non può comparire perché `Lease::cancel()` **esiste nel dominio e non la chiama nessuno** — nessuno use case, nessuna rotta. **Chiuso dalla [sezione 3](11-piani-conclusi.md#esito-sezione-3)**, che ha esposto quella transizione: resta la faccia degli immobili e quella delle bollette. **Bollette**: «Pagata» e «Contestata» sono gli stati di [R25](#lelenco), che nessuno scrive. Le tre cause sono diverse ma la domanda è una sola, e conviene rispondere una volta: **un filtro deve offrire solo ciò che può comparire**, e dove lo stato è vero ma la riga è nascosta serve un modo di includerla. **Non toccare l'archiviazione**: che sia un soft delete è giusto |
| R27 | aperta | **`README.md` e [06](06-sviluppo.md) insegnano ancora il primo avvio a mano** | Trovato scrivendo `R3`. Le due pagine elencano `docker compose up`, `generate-jwt-keys.sh`, il `psql` dei ruoli e i tre `cargo run`, mentre `just setup` fa la stessa cosa in un comando e `just api`/`just worker`/`just web` avviano. Non è sbagliato — quei comandi funzionano, e `CLAUDE.md` §4 dice che la fonte della verità è lui — ma è **più lungo e più facile da sbagliare** proprio nella pagina che un estraneo legge per prima. Da decidere se il `justfile` diventi la via raccomandata anche lì, o se resti uno strumento di comodo per chi già conosce il progetto |

### I contatti fra chi affitta e chi abita

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R43 | aperta | **Un inquilino non ha nessun modo di sapere come contattare chi gli affitta casa** | Chiesta **usando il prodotto**. È già vera per **metà**: il proprietario vede email e telefono dell'inquilino nella tabella delle parti sulla scheda del contratto — sono quelli che ha digitato lui per l'invito — e anche due coinquilini sullo **stesso** contratto si vedono fra loro, perché `lease_parties_select` è per contratto. A mancare è la direzione opposta, e i coinquilini con contratti **separati**. **Il perimetro è deciso: solo il rapporto contrattuale**, cioè l'inquilino vede i contatti di chi amministra la propria organizzazione; i coinquilini con contratti separati restano come sono. **Email e telefono**, non la sola email: per contattare chi ti affitta casa il numero è spesso più utile, è sulla stessa riga e non costa lavoro in più. La parte che sorprende è che **non serve nessuna policy e nessuna migration**: `users_select` ammette già `app.is_my_staff(id)` — la `0030` quel ramo lo ha aggiunto apposta *«per tenere in piedi il rapporto proprietario ↔ inquilino»* — quindi il confine è già dove serve, e a mancare è soltanto la **proiezione**. **Non emenda [ADR-036](08-decisioni.md#adr-036)**, e va detto perché sembrerebbe di sì: quella decisione governa il perimetro della **casa**, e sceglie di proiettare da `app.property_residents` il solo nome; questa voce sta sul ramo che la stessa migration ha tenuto aperto, e lo lascia intatto. La forma da riusare è [ADR-049](08-decisioni.md#adr-049): un modello di lettura che **proietta** le colonne scelte sotto `users_select`, mai una rotta che risolva le persone per identificatore — in pratica un `contacts(ids)` accanto a `UserRepository::names(ids)`, e **non** `select_user!`, che legge l'aggregato intero, hash della password compreso. **La trappola è che `is_my_staff` risponde per tutti**: restituisce vero per ogni `owner` e `manager` attivo di un'organizzazione a cui il chiamante appartiene, non per «il proprietario del **mio** contratto». Con un'organizzazione che ha più gestori, un elenco che li mostri tutti non dice chi contattare — e il legame con il contratto non passa da lì. Da decidere: se mostrare il solo `owner`, tutti, o risalire alla persona dal contratto. Da decidere anche **dove**: l'inquilino ha un contratto solo, quindi «chi amministra» ha una risposta sola, e la scheda del contratto è il candidato naturale. **Due cose vanno insieme al codice e si dimenticano**: l'[informativa sulla privacy](11-piani-conclusi.md#esito-utenti-veri) dichiara con chi si condividono i dati, e questo è una comunicazione di dato personale a un destinatario nuovo; e il perimetro scelto diventa **il pavimento** — allargarlo dopo è additivo, stringerlo è una rottura per chi ci contava. **ADR prima del codice** |

### La durata della sessione

| # | Stato | Cosa | Quel che si sa già |
|---|---|---|---|
| R42 | aperta | **La sessione non scade mai davvero, e non lo si è mai scelto.** Si vuole un «Ricordami per 30 giorni» | Trovata **controllando la sessione** dopo la sezione 6, non usando il prodotto: una scheda lasciata aperta un giorno con l'API spenta è rientrata da sé, ed è corretto — a tenerla dentro era il cookie del refresh. Controllando *perché*, è uscito che la finestra dei 30 giorni è **scorrevole**: `RefreshSession::issue` in `crates/application/src/use_cases/auth/refresh.rs` costruisce il successore con `expires_at = now + lifetime`, quindi i trenta giorni contano dall'**ultimo uso** e non dal login. Chi apre il prodotto una volta al mese resta dentro per sempre. Non è un difetto — è la forma comune — ma **non risulta scelto**, e [04](04-sicurezza.md) diceva «vita: 30 giorni», che si legge come un tetto: la descrizione è stata corretta, la decisione resta da prendere. **La decisione di prodotto c'è ed è questa: un «Ricordami per 30 giorni» al login, esplicito, e senza la spunta una sessione che finisce con il browser.** Quel che si sa già del lavoro: la durata è **già per riga** — `app.user_sessions.expires_at` — quindi **nessuna migration**; a essere globale è `RefreshTokenFactory::lifetime()`, che il caso d'uso del login dovrà ricevere come parametro invece di leggere dalla configurazione. Il cookie oggi porta **sempre** `Max-Age`, calcolato dalla vita residua del token (`session_reply` in `crates/api/src/routes/auth.rs`): senza la spunta va **omesso**, che è ciò che lo rende un cookie di sessione e lo fa morire con il browser. Servono un campo nel corpo del login, una casella nel form e una stringa nel catalogo. **La trappola è la rotazione**: se la finestra continua a scorrere, il bottone dice «30 giorni» e significa «per sempre» — cioè l'etichetta mente. Scegliere quella spunta implica quindi che il successore **non** superi la scadenza di chi lo precede, ed è la riga che si dimentica, perché oggi il codice fa il contrario e sembra giusto. Da decidere solo il resto: se il rinnovo silenzioso del `refresh` debba comportarsi allo stesso modo per entrambi i casi, e se serva anche un tetto assoluto per chi la spunta ce l'ha. **ADR prima del codice**: è una decisione sulla sicurezza, e cambia una proprietà che [04](04-sicurezza.md) descrive |

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
