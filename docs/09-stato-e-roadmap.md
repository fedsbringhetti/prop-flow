# 09 — Stato e roadmap

Stato al **22 settembre 2026**. Riscritto per intero in questa data: la forma precedente,
cresciuta per stratificazione dal 3 al 17 settembre fino a tenere tre piani uno sopra
l'altro, è archiviata com'era in
[11 — Piani conclusi](11-piani-conclusi.md#archivio-09-al-18-settembre-2026).

## Come si usa questo documento

Qui ci sono **tre cose e in quest'ordine**: dove siamo, il cammino — che cosa viene
adesso, che cosa dopo, che cosa è fuori e perché — e lo stato di ciò che esiste. È il
documento che si legge per rispondere a «cosa faccio adesso».

I piani **già eseguiti**, con i loro esiti, stanno in
[11 — Piani conclusi](11-piani-conclusi.md); le cose da sistemare, una voce alla volta,
in [12 — Da fare](12-da-fare.md). I tre file si dividono per **ciclo di vita**: `09` si
riscrive a ogni fetta, `11` non si riscrive mai, `12` cresce e si svuota di continuo.

Le regole, che stanno anche in `CLAUDE.md` §3:

- **un piano si scrive qui prima di implementarlo**, con la scomposizione in passi, le
  decisioni con il perché, le alternative scartate e le trappole: un piano che resta nella
  conversazione in cui è nato non esiste per la sessione successiva;
- **a esito scritto il piano si sposta in `11` nello stesso commit**, e nello stesso giro
  si pianifica la sezione successiva, perché chi ha appena chiuso ha in testa le trappole
  che la sessione dopo non avrà;
- **ciò che è rinviato si segna come rinviato, con la ragione**, altrimenti si rilegge come
  lavoro pendente;
- **ciò che descrive com'era e non com'è si sposta in `11`** nel commit che lo rende
  storia. È la regola aggiunta il 18 settembre 2026, dopo che questo file aveva accumulato
  duecentocinquanta righe di «prossimi passi» precedenti e un piano dell'8 settembre che
  chiedeva un collaudo fatto il 9. Questo documento può essere lungo; non può essere
  stratificato.

## Dove siamo

**Il prodotto esiste, e nessuno che non l'abbia scritto l'ha ancora usato.** PropFlow è
una piattaforma per chi affitta e per chi abita: backend Rust (Axum, PostgreSQL con Row
Level Security, porte e adattatori), client React in italiano servito dalla stessa
origine. Tutti i moduli dell'MVP hanno schema, API e interfaccia: immobili e stanze,
contratti con le parti e gli inviti, documenti, segnalazioni con il thread, bollette con la
lettura automatica, addebiti e pagamenti, spese co-living, il riepilogo del proprietario,
il proprio account, la posta in uscita e il recupero della password. **Si entra solo per
invito** ([ADR-053](08-decisioni.md#adr-053)): un inquilino dal contratto di chi lo
ospita, un proprietario da un token di piattaforma emesso da uno script.

I numeri, contati sullo schema migrato e non dedotti: **42 migration**, 27 tabelle più le
tre partizioni di `audit_log`, 35 enum, 6 domini, 163 indici, 69 policy RLS, 50 trigger, 4
viste, 38 funzioni. **486 test Rust** — 221 di dominio, 16 applicativi, 27 di
infrastruttura, 11 unitari del bordo HTTP, 86 di integrazione sulla persistenza, 3
sull'adattatore S3, 119 di contratto HTTP, 3 di guardia sulla documentazione della
configurazione — e **150 del client** in 26 file. `just ci` è il comando che li esegue
tutti, con lo schema verificato dalle sue invarianti.

**Com'è arrivato qui.** Dal 3 all'8 settembre 2026 le fette — le «fetta 1» dei moduli, poi
A, B, B-bis e C — hanno costruito le capacità; il 9 e 10 settembre il
[collaudo in undici punti](11-piani-conclusi.md#esito-sezione-5-bis) è stato la prima
volta che il prodotto veniva usato per intero, e ha prodotto la maggior parte di
[12](12-da-fare.md); dall'8 al 18 settembre le **dieci sezioni** hanno chiuso ciò che il
collaudo ha trovato, tolto la registrazione aperta, spiegato il prodotto da dentro e
sistemato le tre cose che il primo deploy avrebbe incontrato da sé; il 22 settembre la
[11](11-piani-conclusi.md#esito-sezione-11) ha aperto la serie che chiude tutto il
segnalato prima del collaudo, e la
[12](11-piani-conclusi.md#esito-sezione-12) ha scritto che cosa vieta un contratto in
essere. Ogni sezione ha il suo piano e il suo esito in
[11](11-piani-conclusi.md), e la [tabella](#le-sezioni) qui sotto le elenca.

**Che cosa blocca.** Dal 10 settembre il deploy aspetta **mezzi e non codice**: una VPS, un
dominio, un relay SMTP con SPF, DKIM e DMARC, delle credenziali. Gli artefatti — il
Caddyfile di produzione, il compose, la copia e il ripristino — sono scritti, validati per
quanto si può senza una macchina, e **mai eseguiti**. Il
[runbook](07-operazioni.md#il-primo-deploy--runbook) è pronto.

**Che cosa le ultime sezioni hanno insegnato**, e va portato avanti: i test non
sostituiscono un giro a mano — la 8 ha trovato una pagina bianca dietro 478 + 129 test
verdi, la 10 una premessa sbagliata del suo piano prima di scrivere una riga — e un giro
rende in un'ora quanto una sezione. Il giro che nessuna sezione sostituisce è quello di
una persona esterna, dal telefono.

## Il cammino

<a id="prossimo-passo-suggerito"></a>

### Adesso: chiudere tutto il segnalato, poi il deploy e il collaudo

**Deciso il 22 settembre 2026, da chi possiede il progetto**, e cambia l'ordine che
valeva dal 18: **le voci aperte di [12](12-da-fare.md) si chiudono tutte prima del
collaudo**, salvo una. Il criterio precedente — *niente prima del mese con un proprietario
vero* — poggiava su un'idea giusta, che l'ordine fra le voci lo decida una misura e non un
ragionamento; la ragione per metterlo da parte è altrettanto semplice: il collaudo di
[13](13-collaudo.md) si percorre **una volta**, sulla macchina vera, e percorrerlo con
sedici difetti noti dentro misurerebbe i difetti e non il prodotto. Le voci si chiudono
per **sezioni**, una per conversazione, com'è la regola, e sono la tabella
[qui sotto](#le-sezioni): dalla 11 alla 16.

**L'unica che resta aperta è `R40`**, cambiare la propria email: porta con sé la verifica
dell'email, che cambia il percorso di registrazione che ogni collaudo attraversa e ha
bisogno del relay vero per essere provata — cioè del deploy. Si fa dopo, con la ragione
scritta nella sua riga.

Poi, nell'ordine:

| | Passo | Chi | Stato |
|---|---|---|---|
| 1 | **Le sezioni 11-16**, una per conversazione, ognuna con il piano in questo file prima e l'esito in [11](11-piani-conclusi.md) dopo | chi sviluppa | la 11 è [conclusa](11-piani-conclusi.md#esito-sezione-11), la 12 è [pianificata](#piano-in-corso) |
| 2 | I mezzi: VPS, dominio, relay SMTP con SPF, DKIM e DMARC, credenziali | chi possiede il progetto | **il blocco reale**, dal 10 settembre; si procurano in parallelo alle sezioni |
| 3 | **Il ripristino provato una volta**, mentre dentro non ci sono i dati di nessuno | runbook | mai eseguito |
| 4 | Il primo deploy dal runbook, com'è scritto | runbook | criterio di fine: *aprire il prodotto dal telefono, in 4G* |
| 5 | **Il collaudo di [13](13-collaudo.md)**, per intero, dal telefono — compreso l'Invio su `/invito` con una tastiera vera (B2), in sospeso dal poscritto della 8 | a mano, meglio se da chi non ha scritto il codice | l'esito in [11](11-piani-conclusi.md), **mentre lo si percorre** |
| 6 | **Il mese con un proprietario vero** | esterno | la misura che decide tutto ciò che viene dopo |

*Fatto, e non si ripete:* la CI verde e il merge della PR #57 in `dev` (`fafc2bd`, merge
commit, non squash) il 18 settembre; il collaudo scritto e mergiato (#58, `822d4f9`) il 22.

*Perché il deploy adesso, con i mezzi che ci sono.* Deciso con l'esito della 9. I mezzi
vanno procurati, non costruiti: nessuna sezione li fa arrivare. Nel frattempo ogni sezione
ha reso meno della precedente, e le ultime tre sono state rifinitura su schermate che
nessun estraneo ha visto. Il criterio di fine del runbook è l'unica prova che il progetto
non può darsi da solo.

*Che cosa la sezione 10 aveva lasciato fuori, e perché.* Sei voci erano candidate a
precedere il deploy; la [10](11-piani-conclusi.md#esito-sezione-10) ne ha prese tre — `R50`,
`R51`, `R46` — per un criterio solo, *ciò che il primo deploy incontra da sé nel primo
mese*, e ha lasciato le altre con queste ragioni, che restano perché `11` e `12` le citano
e perché dicono che cosa si è scelto di **non** fare di fretta. **Dal 22 settembre le
quattro entrano nelle sezioni 11 e 15**, con il tempo che chiedevano:

- **`R48` dopo**: chi ha già un account e nessuna organizzazione non può diventare
  proprietario. Nel primo mese c'è un proprietario, che è chi possiede il progetto, e i
  suoi inquilini: il caso non si presenta. È una terza rotta e un ramo nel form, e va
  pianificato quando qualcuno lo chiede;
- **`R49` dopo**: il secondo browser con un'organizzazione lasciata altrove parte con un
  `X-Org-Id` morto, e l'elenco lo corregge da sé. È un lampo, non un vicolo cieco;
- **`R42` dopo**: la finestra scorrevole dei trenta giorni è la forma comune, è documentata,
  e il «Ricordami» è un ADR prima del codice — una decisione sulla sicurezza che non si
  prende di fretta per fare un deploy;
- **`R52` dopo**, nata eseguendo la 10: il rinnovo *dentro* la sessione su un `5xx` chiude
  la sessione con il cookie ancora buono. Si manifesta solo se un `401` cade esattamente
  durante un riavvio dell'API, e la via d'uscita è un ricaricamento, che dalla 10 non
  manda più al login.

<a id="le-sezioni"></a>

### Le sezioni: com'erano decise e come sono andate

Decise l'**8 settembre 2026**, dopo il collaudo, e spostate qui da `12` il 18: sono la
roadmap, e `12` è l'elenco delle voci, non un piano. Si eseguono **per sezione, una per
conversazione**, ed è il vincolo che ha deciso i raggruppamenti, non l'argomento: una
sezione deve chiudersi senza lasciare mezze cose a quella dopo, perché chi la esegue apre
una sessione nuova che non ha visto le precedenti. Quello che non si fa è **eseguire mezza
sezione**.

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
| ~~**7**~~ | ~~Cancellare, in generale~~ — **conclusa** il 12 settembre 2026 | `R15` | client trasversale, **più una rotta** | [Piano](11-piani-conclusi.md#piano-sezione-7) · [esito](11-piani-conclusi.md#esito-sezione-7). **Una rotta, nessuna migration, nessun ADR**: la decisione l'aveva presa la `0028`. Il piano aveva **tre premesse sbagliate**, e una stava nella tabella «verificato e non dedotto»: un secondo archivio risponde `404`, non `409`, perché la famiglia del soft delete non si rilegge — e il `404` è entrato fra gli «era già così». La `USING` di `expenses_update` è più larga di «chi l'ha registrata o lo staff», e il confine è nel caso d'uso. Quattro test verificati **al contrario** prima di consegnarli. Ha trovato `R45` usando il prodotto |
| ~~**8**~~ | ~~Il perimetro dei ruoli~~ — **conclusa** il 16 settembre 2026 | `R9` | ADR, poi migration + backend + client + `ops/` | [Piano](11-piani-conclusi.md#piano-sezione-8) · [esito](11-piani-conclusi.md#esito-sezione-8). [ADR-053](08-decisioni.md#adr-053) prima del codice: l'inquilino entra per invito senza un'organizzazione propria, il proprietario con un token di piattaforma emesso da un runbook, `POST /auth/register` non esiste più. **La prima sezione eseguita con agenti separati, e l'ultima**: l'esito riporta le misure e la ragione del ritiro |
| ~~**9**~~ | ~~Spiegare come si usa il sito~~ — **conclusa** il 17 settembre 2026 | `R8` | client | [Piano](11-piani-conclusi.md#piano-sezione-9) · [esito](11-piani-conclusi.md#esito-sezione-9). Era ultima **apposta**, e la scelta di *dove* — gli stati vuoti spiegano, una pagina sola raccoglie, il riepilogo vuoto è il primo accesso — è nel piano con le alternative scartate. Due premesse sbagliate, nessuna decisione cambiata |
| ~~**10**~~ | ~~Tre chiusure prima del primo deploy~~ — **conclusa** il 18 settembre 2026 | `R50` `R51` `R46` | due rotte di API, un ramo nel client, una migration | [Piano](11-piani-conclusi.md#piano-sezione-10) · [esito](11-piani-conclusi.md#esito-sezione-10). Ciò che il primo deploy incontra da sé nel primo mese. Una premessa incompleta, trovata dal giro a mano prima del codice: `R51` era un `502` del proxy, non un errore di rete, e il ramo copre entrambi. Ne è uscita `R52`, che non precede il deploy |
| ~~**11**~~ | ~~Il client dice il vero~~ — **conclusa** il 22 settembre 2026 | `R36` `R26` `R49` `R52` `R41` `R27` | client + docs | [Piano](11-piani-conclusi.md#piano-sezione-11) · [esito](11-piani-conclusi.md#esito-sezione-11). Nessuna migration, nessuna rotta, nessun ADR, com'era scritto. Il piano aveva **una trappola giusta per la ragione sbagliata** — il flag di `R49` come `useState` è stato scartato per un giro doppio di React che, verificato, non esiste; lo ferma il lint — e un test che passava **senza** il rimedio, perché il probe non era `lazy` come le rotte vere. `R41` è entrata in tre posti guardando la schermata. Ha trovato `R53`: cambiare organizzazione non svuota la cache |
| ~~**12**~~ | ~~L'immobile, con un contratto in essere~~ — **conclusa** il 22 settembre 2026 | `R44` `R45` `R29` | dominio + client, una migration, ADR-055 | [Piano](11-piani-conclusi.md#piano-sezione-12) · [esito](11-piani-conclusi.md#esito-sezione-12). Tre regole del service layer, una domanda sola: *che cosa vieta, o che cosa deve dire, un contratto in essere*. La `0042` fa sommare il totale incassato a una funzione `SECURITY DEFINER`, e il residuo che chi abita legge ha smesso di mentire. Due premesse del piano erano sbagliate, ed erano la stessa riga: nel repository non esisteva **nessun** `REVOKE … FROM PUBLIC` su una funzione, e ora ce n'è uno |
| **13** | **Il subentro** — [piano](#piano-in-corso) | `R28` | dominio + rotta + client | La transizione «esce X, entra Y, con queste quote», che scrive `left_at`. Da sola: è il pezzo di dominio più grande, e tocca l'invariante delle quote che [ADR-046](08-decisioni.md#adr-046) ha appena chiuso |
| **14** | **Le persone, l'altra direzione** | `R43` `R39` | ADR ×2, poi backend + client | Chi abita vede chi contattare; chi amministra può togliere qualcuno. Entrambe usano `has_lease_in_force`, che esiste; entrambe sono un ADR prima del codice perché una comunica un dato personale a un destinatario nuovo e l'altra decide che ne è dei contratti di chi viene tolto |
| **15** | **La sessione, e l'organizzazione attiva** | `R42` `R48` `R53` | ADR, poi backend + client | «Ricordami» con la rotazione che non scorre, l'invito da proprietario da un account che esiste già, e la cache che non si svuota cambiando organizzazione. Le prime due sono ciò che resta dell'autenticazione, e `R42` cambia una proprietà che [04](04-sicurezza.md) descrive; `R53` è qui dal 22 settembre 2026, deciso con l'esito della 11 che l'ha trovata — si tocca lo stesso guscio, e una delle sue due vie è una riga in `QueryProvider`. Nella 12 sarebbe stata un corpo estraneo: quella sezione risponde a *che cosa vieta un contratto in essere* |
| **16** | **Cancellare una persona** | `R37` | ADR, migration, runbook | La cancellazione vera contro la regola 7 — `DELETE` vietato — e contro `audit_log`, che nomina le righe: che cosa significhi il verbo e che cosa resti quando la persona non c'è più. Ultima perché tocca tutto ciò che le altre costruiscono |
| **17** | **Il primo deploy e il collaudo** — [runbook](07-operazioni.md#il-primo-deploy--runbook), [collaudo](13-collaudo.md) | — | a mano, su una macchina | Dopo la 16. Servono i mezzi. Il criterio di fine è il punto 4 del runbook, e il collaudo di 13 si percorre per intero dal telefono, con l'esito in [11](11-piani-conclusi.md) mentre lo si fa |

**Le sezioni 6-9 aspettavano una prova che non è arrivata.** Avevano un obiettivo
dichiarato — *un proprietario vero, che non ha scritto il codice, usa il prodotto per un
mese con i suoi dati* — e l'ordine fra loro doveva deciderlo quel mese. Il mese slitta con
il deploy, quindi l'ordine 6 → 7 → 8 → 9 → 10 è stato **scelto senza quella prova**: un
ragionamento, non una misura. È la ragione per cui il passo 8 del cammino, il mese, viene
prima di qualunque altra sezione.

### Dopo il deploy: che cosa quel mese decide

**Tutto ciò che serve soltanto a utenti veri viene dopo che ci sono.** È il criterio
d'ordine che vale dal primo piano, e ha già cambiato forma due volte: le notifiche sono
state rinviate finché a provare era chi costruisce, poi fatte nella 5-quater quando è
diventato chiaro che senza posta una persona esterna non può nemmeno cominciare. Il
criterio si applica a ciò che **compra soltanto** quel valore, non a ciò che è scomodo.

Dal 22 settembre il criterio ha un'eccezione dichiarata: le voci aperte in
[12](12-da-fare.md) si chiudono **prima** del collaudo, nelle sezioni 11-16, per la ragione
scritta [sopra](#prossimo-passo-suggerito). Erano sedici quando la decisione è stata presa;
con la [11](11-piani-conclusi.md#esito-sezione-11) e la
[12](11-piani-conclusi.md#esito-sezione-12) concluse restano **nove**, di cui **sette
assegnate** alle sezioni 13-16. Vale anche per ciò che entra **dopo** la
decisione e che una sezione incontra da sé: `R53`, nata eseguendo la 11, è andata nella
15 invece di aspettare. **`R54` no, ed è l'eccezione da guardare**: nata dal giro a mano
della 12 — «oggi» del prodotto è il giorno UTC — non sta in nessuna delle quattro
sezioni rimaste, perché non è la domanda di nessuna di esse. Resta **da ordinare**, e la
decisione è se meriti una sezione sua prima del collaudo o se la sua finestra di due ore
notturne la mandi dopo. Finché non è presa, quella voce è l'unica cosa segnalata che il
collaudo troverebbe ancora aperta. Dopo il collaudo e il mese resta **`R40`** — l'email come
credenziale, con la verifica che le va insieme — più ciò che il collaudo e il mese
troveranno, che è la parte per cui il criterio esiste: quelle voci nuove si ordinano con
la misura, non con un ragionamento.

### Ciò che dipende da dove si esercisce

Rate limiting per IP, verifica dell'email bloccante, MFA, `kid` nei token per la rotazione
delle chiavi, metriche, alerting, il cron per le partizioni di `audit_log`, il gate sulle
licenze, i test di carico. Sono i [trasversali mancanti](#trasversali-mancanti) qui sotto,
**nominati e non pianificati** apposta: dipendono dalla macchina su cui il prodotto gira,
che prima del passo 6 non esiste. Pianificarli adesso significherebbe scrivere un piano su
un'ipotesi. La verifica dell'email bloccante ha in più una conseguenza sull'ordine: cambia
il percorso di registrazione, cioè la prima cosa che ogni collaudo attraversa.

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
  conversazione sulle notifiche push. `Notifier` esiste dalla 5-quater; quella
  conversazione viene dopo il deploy e il mese con un proprietario vero;
- **la ricerca full-text**, che è V2 e va alimentata dall'outbox;
- **il filtro sui crediti da sovrapagamento.** Rinviato con la sua ragione in
  [ADR-040](08-decisioni.md#adr-040): richiederebbe un parametro nuovo e un indice
  parziale che lo regga, e il caso d'uso «mostrami tutti i crediti» non è ancora nato.

<a id="piano-in-corso"></a>

## Piano in corso

<a id="piano-sezione-13"></a>

### Piano — Sezione 13: il subentro

Pianificata il **22 settembre 2026**, nello stesso giro dell'esito della
[12](11-piani-conclusi.md#esito-sezione-12), com'è la regola. Chiude `R28`: **cambiare la
composizione di un contratto attivo**, cioè «esce X, entra Y, con queste quote».

È da sola perché è il pezzo di dominio più grande rimasto e tocca l'invariante delle quote
che [ADR-046](08-decisioni.md#adr-046) ha appena chiuso: è la **sola via legittima** per
aggiungere una parte a un contratto attivo, e deve esserlo senza riaprire la porta che
quell'ADR ha sbarrato. **Una rotta nuova** (è il primo caso da parecchie sezioni in cui ne
serve davvero una), un ADR prima del codice, **nessuna migration** — e il perché della
migration mancante è una delle decisioni, non una dimenticanza.

**Si esegue in una sola conversazione**, senza agenti. Ogni test nuovo provato al
contrario prima di consegnarlo, con l'asserzione caduta scritta nell'esito. I test con il
database solo con `./ops/ci/test-integration.sh`, che non ha un filtro: le prove al
contrario si combinano in un giro.

#### Che cosa esiste davvero, verificato contro il codice il 22 settembre 2026

| Premessa | Verifica |
|---|---|
| A un contratto attivo **non** si aggiungono parti | vero: `AddLeaseParty` chiama `lease.ensure_accepts_parties()` (`use_cases/lease/add_party.rs:61`), che ammette solo `Draft` e `PendingSignature` e altrimenti solleva `Invariant::LeasePartiesLocked` (`domain/src/lease/aggregate.rs:229-237`) |
| `PATCH /lease-parties/{id}` tocca **una** riga e non ricontrolla la somma | vero, e deliberato: `LeaseParty::correct` (`domain/src/lease/party.rs:164-215`) non guarda le altre parti; il suo commento lo dice e ADR-046 lo motiva — per portare `100 / 0` a `50 / 50` si passa da uno stato intermedio |
| `DELETE /lease-parties/{id}` non serve a far uscire nessuno | vero: `LeaseParty::remove` (`party.rs:225-236`) rifiuta con `Invariant::LeasePartyHasUser` se la parte ha un utente collegato, e il suo commento nomina già `left_at` come «la cosa che non è questa» |
| `left_at` c'è dappertutto e **nessuno lo scrive** | vero, in cinque punti: colonna `app.lease_parties.left_at` (`0005:94`), nel `GRANT UPDATE` per colonna a `propflow_app` (`0019:67-69`), letto e riscritto dal repository (`db/repositories/lease_party.rs:87` e `:119`), esposto in lettura dall'API (`routes/leases.rs:467`, `:483`) e nel tipo del client (`api/types.ts:296`). **Nessun caso d'uso lo valorizza**, e `CorrectLeasePartyRequest` lo esclude per esplicito (`routes/leases.rs:265-266`) |
| L'invariante delle quote **sa già** di chi è uscito | vero: `ensure_tenancy_complete` filtra `party.left_at.is_none()` (`party.rs:247`), pretende almeno un inquilino e rifiuta una quota a zero |
| E lo sanno anche le due aritmetiche della quota residua | vero e **già allineate**: `remaining_share` del server (`use_cases/invitation/invite_tenant.rs:205-212`) e `assignedShare` del client (`features/leases/shares.ts:22-27`) filtrano entrambe `left_at` |
| `share_percent` **non ripartisce nessun addebito** | vero, ed è la premessa che cambia la dimensione della sezione: `grep -rn share_percent crates/application/src/use_cases/` dà due soli punti, `add_party.rs` e `invite_tenant.rs`. Gli addebiti stanno sul **contratto** (`IssueCharge` prende un `lease_id`), non sulla parte: la quota è dichiarativa e serve solo all'attivazione |
| Chi esce sparisce da «in essere» e dai residenti | vero: `app.has_lease_in_force` filtra `lp.left_at IS NULL` (`0039:79`) e `app.property_residents` pure (`0030`). Scrivere `left_at` fa quindi due cose da sé: la persona può uscire dall'organizzazione (`R16`, sezione 6) e sparisce dall'elenco dei residenti |
| **`app.can_access_lease` invece `left_at` non lo guarda** | vero, ed è la scoperta che decide la sezione: il ramo dell'inquilino chiede `lp.user_id = app.current_user_id() AND lp.deleted_at IS NULL` più `app.is_org_member` (`0041:54-76`). Chi esce da un contratto **continua a leggerlo** finché resta membro dell'organizzazione |
| `lease_parties_primary_uk` non esclude chi è uscito | vero: `UNIQUE (lease_id) WHERE is_primary AND deleted_at IS NULL` (`0005:103-104`). Se esce l'intestatario principale ed entra il suo, per un istante sono due |
| `lease_parties_lease_user_uk` nemmeno | vero: `UNIQUE (lease_id, user_id) WHERE user_id IS NOT NULL AND deleted_at IS NULL` (`0005:100-101`). La stessa persona non può avere due righe sullo stesso contratto, nemmeno se la prima è chiusa |
| C'è un `CHECK` sulle date, e oggi è quasi sempre vuoto | vero: `lease_parties_dates_ck CHECK (left_at IS NULL OR joined_at IS NULL OR left_at >= joined_at)` (`0005:98`); `joined_at` lo scrive **solo** `link_user` (`party.rs:135`), cioè l'accettazione dell'invito. Una parte mai accettata ha `joined_at` nullo e il `CHECK` non dice niente |
| Gli eventi delle parti esistono, e sono eventi **del contratto** | vero: `LeasePartyAdded`, `LeasePartyCorrected`, `LeasePartyRemoved` (`domain/src/events/mod.rs:125-127`), con `aggregate_type` del contratto (`:224-231`) |
| Il client mostra tutte le parti e **non dice chi è uscito** | vero: `LeaseDetailPage.tsx:188-210`, la tabella stampa nome, ruolo, email e quota per ogni riga di `data.parties`, senza guardare `left_at` |
| I verbi del contratto sono `POST /leases/{id}/<verbo>` | vero: `activate`, `terminate`, `cancel` (`routes/leases.rs:35-44`), la forma di [ADR-042](08-decisioni.md#adr-042) |
| I test sono 486 Rust e 150 del client | vero, esito della 12 e `just ci` |

#### Decisioni prese, con il perché

- **Una transizione dedicata, non una richiesta che sostituisce tutte le parti.** La voce
  registrava le due vie e questo piano sceglie la seconda, per tre ragioni. *Primo*: dice
  **quando**, e la data è l'unica cosa che `left_at` chiede e che una sostituzione
  massiva non porta. *Secondo*: una `PUT /leases/{id}/parties` che accetti un elenco
  intero riaprirebbe in silenzio ciò che [ADR-046](08-decisioni.md#adr-046) ha chiuso —
  aggiungere una parte a un contratto attivo — perché il suo corpo *è* un'aggiunta
  travestita da sostituzione, e non ci sarebbe più un punto in cui dire di no. *Terzo*: la
  transizione ha un nome, e un nome si può rifiutare, tracciare e raccontare nella
  conferma; una `PUT` è muta;
- **Una rotta sola: `POST /api/v1/leases/{id}/handover`.** Lo stesso verbo di `activate` e
  `terminate`, in inglese come tutte. Il corpo porta **tutto ciò che deve cambiare
  insieme**: chi esce, chi entra, da quando, e le quote di tutti dopo. Una bozza:

  ```json
  {
    "effective_date": "2026-11-01",
    "leaving_party_id": "…",
    "joining": { "full_name": "…", "email": "…", "role": "cotenant", "share_percent": "50.00" },
    "shares": [ { "party_id": "…", "share_percent": "50.00" } ]
  }
  ```

  `shares` **non è opzionale quando serve**: se dopo l'uscita le quote non sommano a 100 e
  nessuno le riassegna, il contratto resta attivo con un'invariante violata, che è
  esattamente il danno di ADR-046. Chi entra può anche mancare — «esce X e basta» è un
  caso vero, con le quote ridistribuite fra chi resta;
- **L'invariante si riverifica alla fine della transizione, e solo lì.** `ensure_tenancy_complete`
  sullo stato finale, dentro la stessa unità di lavoro. È la differenza con il `PATCH`:
  là lo stato intermedio è legittimo e nessuno può controllare, qui lo stato intermedio
  **non esiste** — una richiesta sola, una transazione sola. Il rifiuto è
  `Invariant::LeaseSharesNotHundred` o `LeaseWithoutTenantParty`, che esistono già e hanno
  già i loro codici;
- **Il caso d'uso costruisce la parte nuova da sé e non passa da `AddLeaseParty`.** Quello
  chiama `ensure_accepts_parties`, che rifiuta su un contratto attivo — ed è giusto che
  continui a rifiutare. La regola di ADR-046 non si allenta: resta vero che *aggiungere*
  una parte a un contratto attivo è vietato, e il subentro non è un'aggiunta;
- **`can_access_lease` non si tocca, e quindi non c'è nessuna migration.** È la decisione
  che costa di più e va detta per esteso. La tentazione è aggiungere `AND lp.left_at IS
  NULL` al ramo dell'inquilino, come [ADR-054](08-decisioni.md#adr-054) ha aggiunto
  `is_org_member`. **Non si fa**: chi esce da un contratto a metà **ha ancora degli
  addebiti da pagare**, e togliergli la lettura del contratto gli toglierebbe la pagina da
  cui li vede. Uscire da un contratto non è uscire dall'organizzazione. La risposta, in
  una riga: **si smette di essere residente, non si smette di essere debitore** — e le due
  cose sono già distinte nello schema, perché `has_lease_in_force` e `property_residents`
  filtrano `left_at` e `can_access_lease` no. Va scritta in un ADR perché *sembra* una
  dimenticanza e non lo è;
- **Se esce l'intestatario principale, il suo posto si assegna nella stessa richiesta.**
  Il corpo porta `is_primary` dentro `shares` o su `joining`; se chi esce era `is_primary`
  e nessuno lo diventa, si rifiuta. Un contratto attivo senza intestatario non è uno stato
  che qualcuno sappia nominare, ed è quello in cui la sezione 3 aveva trovato i contratti
  rotti;
- **Il subentro non muove un euro degli addebiti già emessi**, e il piano lo dice invece di
  lasciarlo scoprire: le quote sono dichiarative, gli addebiti stanno sul contratto. Chi si
  aspetta che l'inquilino nuovo cominci a essere addebitato automaticamente si aspetta una
  capacità che non esiste — la ripartizione per quota è una voce che **non c'è**, e se
  servirà nascerà da un giro a mano, non da qui;
- **Un evento nuovo, `LeaseHandedOver`**, accanto ai tre che ci sono: porta
  `lease_id`, `org_id`, la data, e gli **identificatori** delle parti che escono ed
  entrano — mai nomi né email (regola 25). Non si riusa `LeasePartyAdded` +
  `LeasePartyRemoved`: sono due fatti dove ce n'è uno, e chi li consuma non avrebbe modo di
  sapere che erano la stessa transizione.

#### Decomposizione

##### `E0` — l'ADR, prima del codice

ADR-056 in [08](08-decisioni.md): la forma della transizione, il perché non è una
sostituzione, e soprattutto il **perimetro di chi è uscito** — che cosa continua a vedere e
perché `can_access_lease` resta com'è. È l'unica parte che non si deduce dal codice e
quella che, senza un ADR, la sezione dopo rideriverebbe al contrario.

##### `E1` — il dominio

`LeaseParty::leave(effective_date, now)`: valorizza `left_at`, rifiuta se è già uscita e
rifiuta se `joined_at` è valorizzato ed `effective_date` gli è precedente — **con un
messaggio**, perché altrimenti lo rifiuta `lease_parties_dates_ck` e chi legge riceve un
errore di vincolo. Un metodo del contratto che, date le parti e la transizione, dica se
lo stato finale regge.

**Criterio di fine:** test unitari di dominio, senza database: chi esce non conta più nelle
quote; una data precedente a `joined_at` è rifiutata con il suo codice; uscire due volte è
rifiutato.

##### `E2` — il caso d'uso

`HandoverLease` in `use_cases/lease/handover.rs`: `ensure_staff`, rilegge contratto e
parti, applica l'uscita, costruisce la parte nuova, riassegna le quote e `is_primary`,
riverifica `ensure_tenancy_complete`, scrive, emette `LeaseHandedOver`, `commit`. Una
transazione sola.

**Criterio di fine:** test di persistenza in `crates/infrastructure/tests/`: dopo il
subentro le quote sommano a 100 sulle parti non uscite, `left_at` è la data richiesta, e
`app.has_lease_in_force` risponde **falso** per chi è uscito e **vero** per chi è entrato
— che è la prova che il subentro ha fatto le due cose che contano.

##### `E3` — il bordo HTTP

`POST /api/v1/leases/{id}/handover`, il DTO, i codici in [05](05-api.md). Nessuna rotta
nuova oltre questa.

**Criterio di fine:** test di contratto HTTP: il subentro su un contratto attivo risponde
`200` con le parti aggiornate; senza `shares` che riportino a 100 risponde `409
lease.shares_not_hundred`; con `effective_date` prima di `joined_at` risponde il codice di
`E1`; su un contratto in bozza risponde `409` — in bozza si aggiunge una parte, non si
subentra. Al contrario, senza la riverifica finale: `200` con le quote che sommano a 50.

##### `E4` — il client

La tabella delle parti dice **chi è uscito** e da quando — oggi non lo dice, ed è la riga
che rende visibile ciò che la transizione scrive. Un dialogo «Subentro» sulla scheda del
contratto, offerto **solo** quando il contratto è attivo, con le quote proposte come le
propone `AddPartyDialog` (da `@/lib/decimal`, mai `parseFloat`). Le stringhe in
`i18n/it.ts`.

**Criterio di fine:** un test del client che il dialogo non compaia su un contratto in
bozza, e uno sulle quote proposte.

##### `E5` — documentazione

ADR-056 già scritto in `E0`; [05](05-api.md) con la rotta e i codici;
[03](03-modello-dati.md) su `left_at`, che da qui in poi qualcuno scrive;
[04](04-sicurezza.md) sul perimetro di chi è uscito; `R28` fuori da [12](12-da-fare.md).

#### Trappole

**Scrivere `left_at` e credere che chiuda la lettura.** Non la chiude:
`app.can_access_lease` guarda `deleted_at` e la membership, non `left_at` (`0041:54-76`).
Un test che verificasse «chi è uscito non vede più il contratto» **fallirebbe**, e la
reazione istintiva — aggiungere il filtro nella funzione — è la decisione sbagliata presa
per la ragione sbagliata. Chi esce continua a vedere, ed è voluto: ha degli addebiti.

**L'ordine delle due scritture su `is_primary`.** `lease_parties_primary_uk` è un indice
unico parziale su `(lease_id) WHERE is_primary AND deleted_at IS NULL`, e non esclude chi è
uscito. Nella stessa transazione, l'`UPDATE` che mette `is_primary` alla parte nuova
**prima** di toglierlo a quella che esce viola il vincolo. Togliere prima, mettere dopo.

**La stessa persona non rientra sullo stesso contratto.**
`lease_parties_lease_user_uk` è su `(lease_id, user_id) WHERE user_id IS NOT NULL AND
deleted_at IS NULL` e non guarda `left_at`: chi è uscito da un contratto e vuole
rientrarci non può avere una seconda riga. Non è nel perimetro della sezione — ma è il
genere di cosa che si scopre da un errore di vincolo in produzione, e va saputa prima.

**Riusare `AddLeaseParty` perché «tanto fa la stessa cosa».** Chiama
`ensure_accepts_parties` e rifiuta su un contratto attivo. Toglierne la chiamata per far
funzionare il subentro riaprirebbe la porta di ADR-046 per tutti.

**Aspettarsi che gli addebiti si spostino.** Non si spostano, e non c'è niente da
spostare: la quota non ripartisce nessun addebito (verificato, due soli usi di
`share_percent` in tutto il livello applicativo). Un piano che desse per scontato il
contrario si troverebbe a scrivere una capacità intera non pianificata.

**`joined_at` nullo.** Una parte mai accettata non ha `joined_at`, quindi il `CHECK` sulle
date non dice niente e una `effective_date` qualunque passa. Il controllo in `E1` deve
gestire il nullo senza rifiutare: rifiutare un subentro perché l'uscente non aveva mai
accettato l'invito sarebbe il caso più comune dei contratti veri.

**Un solo evento e non due.** `LeasePartyAdded` + `LeasePartyRemoved` descriverebbero due
fatti indipendenti, e chi li consuma non saprebbe che erano una transizione sola. Per lo
stesso motivo l'evento porta identificatori e date, mai nomi o email.

#### Che cosa **non** entra

- **la ripartizione degli addebiti per quota**: non è una voce, non esiste oggi, e nascerà
  da un giro a mano se servirà;
- **il filtro `left_at` in `can_access_lease`**: deciso di non farlo, con la ragione
  sopra — non è un rinvio;
- **rientrare sullo stesso contratto** dopo esserne usciti: il vincolo unico lo impedisce,
  e la via è un contratto nuovo;
- togliere qualcuno dall'organizzazione (`R39`) e i contatti (`R43`): la 14, con i loro
  ADR;
- una migration di bonifica dei contratti con le quote già rotte: la stessa ragione di
  ADR-046, quale sia la quota giusta lo sa solo chi amministra.

## Implementato

### Schema — completo per l'MVP

Le 42 migration coprono l'intero perimetro funzionale dell'MVP: utenti e organizzazioni,
immobili e stanze, contratti e parti contrattuali, inviti dei due generi, documenti con
allegato polimorfico, segnalazioni con thread e storico, bollette con esito OCR, addebiti,
pagamenti polimorfici, spese condivise con quote e regolamenti, registro dei job AI,
outbox degli eventi, audit con redazione, codici di recupero, uscita da un'organizzazione.
I numeri sono in [Dove siamo](#dove-siamo).

### Backend

| Componente | Stato |
|---|---|
| Workspace a 6 crate, vincoli verificati in CI | completo |
| Dominio: value object, identità, immobili, stanze, contratti, inviti, documenti, eventi, porte | completo |
| Unit of Work con contesto RLS, due pool | completo |
| Repository: utenti, organizzazioni, membership, sessioni, immobili, stanze, contratti, inviti, documenti | completi |
| Outbox | completo |
| Argon2id, EdDSA, refresh opachi con rotazione | completo |
| Telemetria: log strutturati, OTLP | completo |
| Bordo HTTP: Problem Details, estrattori, OpenAPI | completo |
| Registrazione, login, refresh, logout | completo |
| Organizzazioni: `GET /organizations` con il **nome**, `ensure_authenticated`, risponde anche senza `X-Org-Id` | completo |
| CRUD immobili con PATCH parziale e archiviazione | completo |
| CRUD stanze annidate sull'immobile | completo |
| Contratti: bozza, parti, attivazione, disdetta, stati derivati | completo |
| Inviti: emissione, revoca, accettazione senza pool privilegiato | completo |
| Documenti: caricamento multipart, elenco, download pre-firmato, archiviazione | completo |
| Segnalazioni: apertura, elenco filtrato, dettaglio con storico, thread con note interne, sette transizioni, costi, allegati | completo |
| Bollette: registrazione manuale, correzione, validazione, annullamento, riaddebito, allegati | completo |
| Addebiti: emissione, elenco e dettaglio con il saldo dalla vista, annullamento, rinuncia all'incasso **anche da `partially_paid`**, allegati | completo |
| Pagamenti: incasso registrato dallo staff, dichiarazione dell'inquilino, conferma e rifiuto, dettaglio, elenco per addebito **e collezione dell'organizzazione** con i filtri `status` e `lease_id`, idempotenza fornita dal client, stato dell'addebito ricalcolato dai pagamenti | completo |
| Spese co-living: gruppi con il creatore già dentro, composizione con uscita e rientro, spese con ripartizione `equal` e `weighted`, saldi letti dalla vista, regolamenti a due tempi confermati da chi riceve | completo |
| **Una spesa si annulla** (`POST /expenses/{id}/cancel`): è il soft delete che la `0028` concedeva e nessuno faceva, ed esce dai saldi da sola — **anche dopo un regolamento**, che salda un saldo e non una spesa. Chi l'ha registrata, chi l'ha anticipata, o lo staff; la `USING` è più larga, e il confine è nel caso d'uso | completo |
| Coinquilini visibili: elenco dei residenti di un immobile letto da una funzione che proietta, anagrafica ristretta al perimetro della casa | completo |
| OCR delle bollette: porta `OcrProvider` senza fornitore scelto, adattatore deterministico, coda `ai_jobs` con `SKIP LOCKED`, `pending_review` con le sue uscite, stato del job dall'API | completo |
| Storage oggetti: adattatore S3/MinIO, cifratura letta dal fornitore | completo |
| Migratore, dispatcher dell'outbox, lettura automatica delle bollette | completi |
| Dashboard: `GET /dashboard`, **nove** numeri letti da `v_owner_dashboard`, staff soltanto | completo |
| Sessione in cookie: refresh `HttpOnly` fuori dal corpo, `Secure` condizionale, test che tiene in piedi l'argomento CSRF | completo |
| **I contratti si correggono**: annullamento di una bozza, correzione e rimozione di una parte — tre rotte e **nessuna migration**, perché `deleted_at`, `left_at` e `lease_parties_update` erano già nella `0014` | completo |
| **L'occupazione è derivata e non memorizzata**: `app.room_is_occupied` e `app.property_occupancy_status` (`0033`), con `v_property_occupancy` che smette di contare una colonna che nessuno scrive | completo |
| **Una bolletta si dichiara pagata** al fornitore, con la sua data (`0034`). Non è un pagamento: `app.payments` modella il denaro in entrata | completo |
| **Una segnalazione può essere di interesse comune** (`0035`): il quinto ramo sta in `app.can_access_ticket`, quindi con la riga arrivano il thread, lo storico e le fotografie. Non concede scrittura | completo |
| **Il testo di una segnalazione si corregge** finché nessuno l'ha presa in carico: `PATCH /tickets/{id}/report` e la migration `0036`, che **revoca** su due colonne la scelta della `0020` | completo |
| **Le risposte dicono chi**: nome di chi ha segnalato, di chi ha in carico e di chi scrive nel thread, letti da `app.users` sotto `users_select` | completo |
| **Un thread terminato si legge e non si scrive**, e riaprire pretende un motivo che finisce nella conversazione | completo |
| Immagine Docker, stack compose | completi |
| **Posta in uscita**: adattatore SMTP, configurazione obbligatoria all'avvio, raccoglitore locale nel compose, e il relay dentro `/ready` | completo |
| **Recupero della password**: un codice da incollare, a uso singolo, che revoca i refresh token ([ADR-052](08-decisioni.md#adr-052)) | completo |
| **`app.users` si difende con i privilegi di colonna** (`0038`): una richiesta autenticata scrive `full_name`, `phone`, `locale` e `password_hash`, e nient'altro. `platform_role`, `status` e i due contatori del blocco passano dal pool privilegiato | completo |
| **Il proprio account**: `GET` e `PATCH /api/v1/me`, e `POST /api/v1/me/password` che pretende la vecchia e revoca tutte le sessioni, uniforme ad [ADR-052](08-decisioni.md#adr-052). **Mai un identificatore nel percorso** ([ADR-036](08-decisioni.md#adr-036)) | completo |
| **Si esce da un'organizzazione** (`0039`): `status = 'left'` e non un soft delete, negato con un contratto in essere da una policy e non da un `if`, e **chi esce rientra** accettando un invito nuovo — il ramo che [ADR-051](08-decisioni.md#adr-051) aveva previsto senza scriverlo | completo |
| Artefatti di deploy e di backup — Caddyfile di produzione, compose, copia e ripristino | **scritti e mai eseguiti**: vedi il [runbook](07-operazioni.md#il-primo-deploy--runbook) |
| Pipeline CI | completa |

### Client web

`apps/web/`, React e TypeScript in `strict`, servito dalla **stessa origine** dell'API
dietro un reverse proxy.

| Componente | Stato |
|---|---|
| Catalogo delle stringhe, formattazione con `Intl` e la lingua come parametro | completo |
| Aritmetica decimale su interi: `parseFloat` non compare | completo |
| Client dell'API con il **refresh a singolo volo** e la traduzione di `ProblemDetails` | completo |
| Guscio: ingresso da invito (i due generi, il codice prima del form), accesso, uscita, organizzazione attiva, **stato senza organizzazione**, navigazione | completo |
| **Il ripristino della sessione distingue la rete dal rifiuto**: un `refresh` che non arriva al server, o che riceve un `5xx` dal proxy, è uno stato d'errore con «Riprova» e non «nessuna sessione» — chi ricarica in galleria o durante un deploy non finisce al login con il cookie ancora buono. **Lo stesso vale per il rinnovo dentro la sessione** dalla 11: solo il `401` chiude la sessione, un `5xx` fa fallire la richiesta che aveva chiesto il rinnovo e la schermata mostra «Riprova» | completo |
| **Il selettore delle organizzazioni letto dall'API**, con i nomi al posto degli UUID; il `localStorage` ridotto a ricordare *quale* era attiva — e dalla 11 **il ricordo non vale finché l'elenco non lo conferma**: il guscio aspetta l'elenco anche quando l'organizzazione è ricordata, così un secondo browser con in memoria un'organizzazione lasciata altrove non parte con quell'`X-Org-Id` | completo |
| **La navigazione e la rotta iniziale sono funzioni del ruolo**: `useIsStaff()` in un posto solo, e le azioni di scrittura non si offrono a chi riceverebbe `403` | completo |
| Segnalazioni: apertura sull'immobile, elenco con `?open=true`, dettaglio con lo storico, thread paginato a parte, le **sette** transizioni e il `PATCH` dei soli costi | completo |
| **Dichiarazione di pagamento** dell'inquilino, con la chiave di idempotenza della fetta A riusata e non riscritta | completo |
| **Spese co-living**: elenco dei gruppi con il **proprio netto complessivo** in cima, scheda del gruppo con saldi, spese paginate, quote lette e non ricalcolate, composizione con uscita e rientro, archiviazione | completo |
| **Rimborsi a due tempi**: dichiarazione, conferma e rifiuto di chi riceve, **ritiro di chi ha dichiarato** — che qui esiste e fra i pagamenti no | completo |
| **Il nome di un membro scritto all'ingresso**, perché nessuna risposta di quel modulo ne porta uno e chi lascia la casa non è più fra i residenti ([ADR-043](08-decisioni.md#adr-043)) | completo |
| **L'invitato entra dall'interfaccia**: `/invito`, un campo in cui incollare il codice, e l'organizzazione appena accettata diventa quella attiva ([ADR-045](08-decisioni.md#adr-045)) | completo |
| **La revoca di un invito**, nella modale che l'ha creato e da nessun'altra parte: nessuna rotta li elenca | completo |
| **Il rinnovo è serializzato fra le schede**: lock per origine sopra il singolo volo, e il token non lascia la memoria ([ADR-044](08-decisioni.md#adr-044)) | completo |
| **Allegati su sei owner** da un componente solo: immobile, stanza, contratto, segnalazione, bolletta, addebito — con il limite dei 25 MiB controllato prima di inviare e l'archiviazione offerta a chi la policy ammette | completo |
| **Le stanze si correggono e si archiviano**, da una modale che è la loro unica superficie: `GET /rooms/{id}` non esiste | completo |
| **La stanza si crea con tutti i suoi campi**, dallo stesso componente che la corregge: sette, non quattro contro cinque. `status` resta fuori finché non si decide se sia derivato (`R11`) | completo |
| **La barra laterale è alta quanto la finestra e resta ferma**, con il selettore dell'organizzazione ancorato in fondo e l'elenco che scorre a sé — da `lg` in su, che sotto è un menù a scomparsa | completo |
| **La password si può guardare**, e i due form di autenticazione non tirano più `zod`: −76,4 kB sulla prima schermata che vede un utente nuovo | completo |
| **Una bozza sbagliata si annulla e una parte si corregge**: quota, nome, telefono, ruolo, intestatario — l'email solo finché nessuno l'ha usata — e si toglie chi è stato inserito per sbaglio | completo |
| **Il form non produce più quote sbagliate**: propone il residuo, offre l'intestatario solo se libero, e l'invito dice quale quota riceverà chi accetta | completo |
| **L'elenco dei contratti dice quale casa e quale stanza**, e la creazione avverte se il periodo è già occupato | completo |
| **Stato dichiarato ed effettivo, mostrati insieme quando il dichiarato è un giudizio che il fatto smentisce**: una stanza fuori uso e insieme occupata è l'anomalia da vedere; una stanza disponibile e occupata dice solo «Occupata». Le tendine offrono solo ciò che si decide, e il filtro dell'occupazione sostituisce quello sullo stato derivato | completo |
| **Lo stato dichiarato vive nella scheda e accanto al filtro**, non fra parentesi su ogni riga: è dove nasce la domanda «ho chiesto gli attivi e me ne torna uno scaduto» | completo |
| **I collegamenti dicono il nome di dove portano**: segnalazione, addebito e bolletta nominano immobile, stanza e contratto invece di «Immobili» e «Contratti»; un immobile archiviato lo dice | completo |
| **Il riaddebito dice a chi e quanto resta**: la tendina elenca *immobile · stanza · intestatario* e i soli contratti in corso, e il dialogo propone il **residuo** invece del totale | completo |
| **La data di un pagamento si mostra e non si manda**: il campo parte da oggi, la richiesta tace finché non la si cambia, e l'ora vera non si perde | completo |
| **Il bundle è diviso per rotta**, con i confini di `Suspense` dentro il guscio e con il loro stato | completo |
| **Il recupero della password**, in una schermata e due tempi, con la conseguenza della revoca detta prima di premere | completo |
| **L'informativa sulla privacy**, raggiungibile **senza sessione**: la si legge prima di consegnare i propri dati | completo |
| **Il proprio account** (`/account`): nome, telefono e lingua si correggono; l'**email si mostra e non si modifica, con una riga che dice perché**, perché un campo disabilitato senza spiegazione si legge come un difetto; la password si cambia con l'avviso della revoca **prima** del bottone; l'uscita da un'organizzazione dice che cosa resta dopo, e non si offre per la propria | completo |
| **La selezione in tabella e il verbo in blocco**, una volta nella primitiva: un `Set` di identificatori potato per intersezione a ogni render, «seleziona tutto» che è *le righe caricate*, e un giro di N chiamate **in sequenza** con tre esiti per riga — fatta, era già così (i `409` di transizione **e i `404`**), non fatta con il motivo accanto e la riga ancora selezionata. Sette elenchi, un verbo ciascuno, **nessun «Elimina»**; la riga singola è un giro di lunghezza uno con la stessa conferma | completo |
| I tre stati — caricamento, vuoto, errore — come componenti di prima classe; **ogni vuoto spiega**: che cos'è la sezione, che cosa la riempie, chi la riempie per chi abita, con il gesto solo per chi può farlo; i vuoti **filtrati** dicono di esserlo | completo |
| **La guida** (`/guida`): una pagina, due versioni scelte dal ruolo, le sezioni con i nomi della navigazione; dal menù dell'account e dalla schermata senza organizzazione. **Il riepilogo vuoto è il primo accesso**: tre gesti nell'ordine e il rimando alla guida | completo |
| Dashboard: **nove** numeri, ognuno collegato al posto dove si agisce | completo |
| Immobili: elenco a cursore, dettaglio, creazione, `PATCH` parziale, archiviazione | completo |
| Stanze e residenti | completo |
| Contratti: elenco, bozza, parti, attivazione, disdetta, stati derivati mostrati e non filtrati | completo |
| Inviti: emissione e consegna del token, presentato per quello che è | completo |
| Bollette: registrazione, scansione, **revisione dell'OCR campo per campo**, validazione, riaddebito | completo |
| Addebiti: scadenzario, filtro sullo stato memorizzato, **scheda del singolo addebito**, annullamento, rinuncia all'incasso, credito da sovrapagamento evidenziato | completo |
| Pagamenti: registrazione di un incasso con l'attribuzione come scelta esplicita, **coda delle dichiarazioni** con conferma e rifiuto, chiave di idempotenza generata una volta per intento | completo |
| Reverse proxy in sviluppo e in `docker-compose.yml`, immagine del client | completi |

### Come i test sono contati, e che cosa provano

I conteggi stanno in [Dove siamo](#dove-siamo) e si aggiornano a ogni esito. Che cosa
ciascun gruppo prova è scritto nell'esito della fetta o della sezione che l'ha aggiunto,
in [11](11-piani-conclusi.md); la storia dei conteggi com'era in questo file è
[archiviata](11-piani-conclusi.md#storia-dei-conteggi-dei-test). Tre cose valgono per
tutti: ogni test nuovo è **provato al contrario** prima di essere consegnato, con
l'asserzione caduta scritta nell'esito — la regola è del poscritto della 6; i test con il
database girano solo contro un PostgreSQL vero, in un container effimero, perché la Row
Level Security è comportamento del database; e un test che si dichiara `SALTATO` fa
fallire la CI, perché un verde a vuoto non prova nulla.

## Non implementato

### Moduli MVP che hanno lo schema ma non il codice

Uno solo: **il triage AI delle segnalazioni** (`tickets.ai_job_id`,
`ticket_messages.ai_job_id`). L'OCR delle bollette è fatto — coda, worker, job e
revisione — e il triage **non è una seconda chiamata allo stesso adattatore**: è decidere
se una macchina possa scrivere categoria e priorità di una segnalazione, e con quale
soglia di confidenza, che è una domanda di prodotto sua. La capacità `vision` e il task
`ticket_triage` esistono nell'enum e nessuno li scrive. **Prima del triage viene la scelta
del fornitore**, che blocca anche l'OCR dal produrre valore: vedi
[la decisione che blocca valore](#la-decisione-che-ora-blocca-più-valore-di-ogni-riga-di-codice).

### Che cosa ogni modulo lascia fuori, e non per dimenticanza

| Modulo | Fuori | Perché |
|---|---|---|
| Documenti | versionamento; l'OCR come capacità generale | V2; il fornitore |
| Pagamenti | rimborsi; provider veri con i webhook; il pagamento della bolletta al fornitore | non modellati. **`payable_type` resta a un valore su tre per sempre**: `expense_share` e `settlement` non si scriveranno, perché un regolamento fra coinquilini non è denaro che l'organizzazione incassa e farne una riga in `app.payments` metterebbe nel registro del proprietario un movimento che non lo riguarda. Il piano dei pagamenti prevedeva il contrario; la previsione era sbagliata, e va ereditata come tale |
| Spese co-living | semplificazione dei debiti; i tre metodi di ripartizione oltre `equal` e `weighted`; `is_settled`; il legame regolamento-pagamento; la ripartizione automatica di una bolletta; allegati alle spese | V2, con [ADR-034](08-decisioni.md#adr-034) e [035](08-decisioni.md#adr-035) alle spalle |
| Segnalazioni | triage AI; ricerca full-text; SLA; programmazione; allegati sui singoli messaggi; note di risoluzione | il fornitore; V2 |
| Contratti | il subentro (`R28`) | in [12](12-da-fare.md) |

<a id="trasversali-mancanti"></a>

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
| Il pacchetto su GHCR non è raggiungibile | `docker pull ghcr.io/fedsbringhetti/prop-flow:0.2.0` risponde `403` senza `read:packages`, e le note di rilascio invitano a eseguirlo. Coerente finché il repository è privato — il pacchetto ne eredita la visibilità — e rotto per tutti il giorno in cui non lo sarà. Si cambia nelle impostazioni del pacchetto, non nel workflow | Prima di rendere pubblico il repository, o al primo utente esterno |
| L'oggetto nel bucket sopravvive alla riga archiviata | La cancellazione è soft e il ruolo applicativo non ha `DELETE`; `ObjectStorage` non espone una rimozione, di proposito. Un documento archiviato lascia i suoi byte nello storage a tempo indeterminato | Insieme al purge di `retention_until`: sono lo stesso lavoro — un job applicativo che cancella righe e oggetti, che lo schema prevede e che nessuno ha ancora scritto |
| `retention_until` non viene applicato da nessuno | La colonna esiste e nessuna riga la valorizza; anche valorizzata, nessun job la legge. Lo schema la prevede come job applicativo e non come trigger, deliberatamente | Prima del primo dato reale soggetto a un obbligo di conservazione |
| Il caricamento tiene il file intero in memoria | `UploadDocument` riceve `Vec<u8>` e lo passa allo storage. Con il limite di 25 MiB e poche richieste concorrenti è sostenibile; non lo è con caricamenti grandi e concorrenza alta | Quando i caricamenti superano le decine di MiB, o quando il profilo di memoria dell'API lo mostra. Lo streaming verso lo storage richiede di cambiare la firma della porta |
| Il caricamento apre due transazioni | La prima verifica che l'owner sia visibile prima di scrivere l'oggetto, la seconda registra la riga: l'oggetto va scritto fuori transazione, e senza la verifica preliminare chiunque potrebbe riempire il bucket puntando a identificatori a caso | Se il costo si vede sul pool di connessioni. La verifica non si toglie: è ciò che impedisce di scrivere byte per una richiesta che finirà `404` |
| Nessuna via per ritirare una dichiarazione | `payments_update` è staff soltanto, quindi chi dichiara di aver pagato può creare qualcosa che poi non può disfare. L'uscita esiste ed è il rifiuto dello staff, ma l'inquilino che si accorge subito di aver sbagliato importo non ha rimedio proprio. Darglielo non è una rotta in più: richiede una policy che ammetta l'`UPDATE` del pagante sulla **propria** riga in `pending`, cioè una migration e un secondo `WITH CHECK` da scrivere con attenzione | Se il caso si presenta davvero. Documentato nell'API perché è controintuitivo |
| `refunded_amount` è sottratto e non è mai valorizzato | Il repository somma `amount - refunded_amount` come fa la vista, e la colonna vale sempre zero. È deliberato — è ciò che rende la formula *la stessa* il giorno in cui non varrà più zero — e va saputo, perché letto oggi sembra codice per un caso che non esiste | Con i rimborsi, che sono anche il momento in cui la `GRANT` della `0025` va allargata |
| Un job dell'OCR abbandonato resta `running` per sempre | Non c'è scadenza e il ritentativo è fuori perimetro: `attempts` sale alla presa in carico ma nessuno riporta a `queued` un job che il processo ha lasciato a metà. **Il sintomo è muto** — una bolletta che non arriva mai a `pending_review` senza che nulla sia in errore — e finché il ritentativo non esiste il rimedio è guardare `app.ai_jobs` per gli `running` vecchi | Insieme al ritentativo, che è la stessa conversazione del primo fornitore vero: quali errori produce, quali si riprovano, e cosa significa un terminale senza `finished_at` — che è anche la domanda che tiene `cancelled` fuori dall'enum scritto |
| Il worker legge la configurazione dello storage con `std::env::var` | Sei variabili senza una struct che le valida, a differenza dell'API che passa da `AppConfig`. È la stessa scelta del migratore e la stessa ragione — `AppConfig` gli chiederebbe un bind e delle chiavi di firma che non usa — ma vuol dire che il test di guardia `docs_config_reference.rs` **non copre le impostazioni del worker**: la tabella in `docs/07-operazioni.md` può restare indietro senza che nulla fallisca | Se il worker acquisisce altra configurazione. Con sei variabili il rischio è basso; con dodici la guardia va estesa, ed è un parser in più nello stesso test |
| Gli allegati si scaricano da un link senza `Authorization` | `GET /documents/{id}/content` risponde `307` verso un URL pre-firmato, e il client ci arriva con un `<a href>` — che è ciò che permette al browser di scaricare invece di tenere i byte in memoria. Il link però non porta il token, quindi funziona solo finché la rotta accetta la richiesta dal contesto della scheda. Oggi funziona; il giorno in cui l'API pretendesse l'header su quella rotta, il download smetterebbe di funzionare **senza che nulla in questo repository cambi** | Se il download smette di funzionare, o quando si decida come firmare i link lato client. L'alternativa è un `fetch` con il token seguito da un `blob:` URL, che tiene il file in memoria — accettabile per un PDF, non per una scansione da 25 MB |
| `actions/checkout@v4` gira su Node 20 | I runner lo forzano già su Node 24 con un avviso di deprecazione. Non rompe oggi, romperà quando GitHub toglierà il ponte | Al prossimo intervento sui workflow, o quando l'avviso diventa errore |
| La coda delle dichiarazioni non si filtra per immobile | `GET /api/v1/payments` ha `status` e `lease_id`, e non `property_id`: `app.payments` non ha quella colonna e il legame passerebbe da due salti (`payment → charge → lease → property`). Con pochi immobili la coda sta in una pagina e il filtro non compra niente; con decine diventa lunga | Quando gli immobili per organizzazione passano la decina, o se qualcuno chiede «i pagamenti di questa casa» |
| Un filtro parametrico su un enum non usa un indice parziale | Misurato, non dedotto: `($1::text IS NULL OR status = $1::app.payment_status)` **non** tocca `payments_pending_ix`, perché il cast `text → enum` passa da `enum_in`, che PostgreSQL dichiara `STABLE` e non `IMMUTABLE`, e non viene quindi ridotto a una costante. Il rimedio è nel repository dei pagamenti — una condizione ridondante che nomina il letterale — ed è **locale a quella query**. Ogni altro elenco del progetto usa lo stesso pattern parametrico su uno stato, e nessuno di quelli ha oggi un indice parziale da mancare | Quando un altro elenco filtrato per stato acquisisce un indice parziale. La misura e il rimedio stanno in [ADR-040](08-decisioni.md#adr-040) |

### Chiusi

Restano qui una riga ciascuno, perché altri documenti li citano; il dettaglio è
nell'esito che li ha chiusi.

- ~~L'organizzazione dell'invito non diventa mai predefinita~~ — **chiuso** da [ADR-053](08-decisioni.md#adr-053), decisione 5: un inquilino non ha più una membership `Owner` da preferire, e il caso sparisce invece di essere corretto
- ~~L'incasso non attribuito è invisibile a chi ha pagato~~ — **chiuso** dalla fetta A, e non toccando la colonna
- ~~Un coinquilino non può scoprire l'`user_id` di un altro~~ — **chiuso** dalla migration `0030`, vedi [l'esito](11-piani-conclusi.md#esito-coinquilini-visibili-fetta-1)
- ~~L'anagrafica è visibile per **organizzazione**, non per immobile~~ — **chiuso** dalla migration `0030`
- ~~Il refresh non è coordinato **fra schede**~~ — **chiuso** dalla fetta C, `C2`
- ~~Non esiste un endpoint che elenchi le **organizzazioni** di un utente~~ — **chiuso** dalla fetta B
- ~~Il bundle del client è un pezzo solo~~ — **chiuso** dalla fetta C, `C5`, e ne lascia uno nuovo
- ~~**Il login è la rotta iniziale più pesante**~~ — **chiuso** dalla sezione 1, `S2`
- ~~Un `.env` più vecchio dello storage impedisce l'avvio dell'API~~ — **chiuso** dalla fetta C, `C6`
