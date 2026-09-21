# 13 — Il collaudo completo

Le procedure da percorrere **a mano**, quando c'è una macchina e il prodotto ci gira
sopra. Non è una suite automatica e non la sostituisce: `just ci` verde è il **minimo**,
non il criterio. Il criterio è che una persona attraversi il prodotto per intero da un
browser, con più account, senza mai aprire `curl`.

Questo documento nasce il 22 settembre 2026, dopo il merge delle sezioni 1-10, e
generalizza il [collaudo in undici punti](11-piani-conclusi.md#il-collaudo-completo-che-chiude-la-fetta)
della fetta C — percorso il 9 e 10 settembre, con il suo
[esito](11-piani-conclusi.md#esito-sezione-5-bis) — a tutto ciò che da allora è stato
costruito: l'ingresso per invito, il proprio account, l'uscita da un'organizzazione, la
cancellazione trasversale, gli stati vuoti, la guida, le sonde, il ripristino della
sessione e la membership nella Row Level Security.

---

## Come si usa

**Si scrive la riga mentre si percorre il punto, non alla fine del giro.** È la lezione
più cara dell'esito della 5-bis, e l'unica che quel giro ha dichiarato di aver mancato: il
resoconto è arrivato consolidato — *tutto andava tranne quello che ho scritto* — e per
quattro punti su undici resta l'affermazione che sono stati percorsi, senza l'osservazione.
Fra sei mesi nessuno saprà se i saldi tornavano a zero **da entrambe le parti** o solo da
una. Il rimedio non è ricordarsi meglio: è tenere questo documento aperto accanto al
browser.

**Che cosa si scrive**, per ogni punto: che cosa si è visto, non un segno di spunta. Un
punto che è andato bene merita una riga come uno che è andato male — un esito che elenca
solo i difetti fa credere che il resto non sia stato guardato.

**Dove si scrive:** l'esito del giro va in [11 — Piani conclusi](11-piani-conclusi.md),
come quello di una sezione. **Ciò che il giro trova si scrive anche se non si corregge**, e
diventa una voce in [12 — Da fare](12-da-fare.md) con il suo identificatore.

**Il giro non si interrompe per correggere.** Si annota e si prosegue: fermarsi a sistemare
al terzo punto significa non arrivare all'undicesimo, ed è l'undicesimo che nessuno ha mai
guardato.

**L'ordine conta.** Ogni parte prepara la successiva: senza il proprietario della parte B
non ci sono immobili nella C, senza i contratti della C non c'è nessuno da invitare nella
D. Le parti I, J e K si possono percorrere in qualunque momento dopo la D.

**Due colonne, non una.** Alcuni punti chiedono **due account insieme** — il proprietario
in una finestra, chi abita in un'altra — e sono segnati con «due finestre». Usare due
profili del browser, non due schede dello stesso, salvo dove è scritto il contrario
(la parte I prova proprio le due schede).

---

## Prima di cominciare

**Su che cosa si percorre.** Il collaudo che conta è quello **sulla macchina vera**, dopo
il deploy, dal telefono e in 4G — è il criterio di fine del
[runbook](07-operazioni.md#il-primo-deploy--runbook). Lo stesso documento si può percorrere
in locale come prova generale, e allora vale `docker compose up` dietro il reverse proxy su
`http://localhost:8081`, che è la stessa forma che avrà in produzione. **Non** il percorso
nativo con due porte: quello non ha un proxy davanti, e tre punti di questo documento (A1,
I2, I3) esistono proprio perché il proxy c'è.

**Con quali dati.** Un database **appena migrato**, senza il seeder: il collaudo prova
anche che il primo ingresso funzioni, e un mondo già popolato lo salta. Il seeder
(`ops/dev/seed-walkthrough.py`) serve a un giro veloce di rifinitura, non a questo.

**Quanti account servono:** un proprietario e **due** persone che abitano, perché i
coinquilini e le spese condivise hanno senso solo in due. Le email devono essere vere se si
prova il recupero della password sulla macchina vera (punto B6).

**Quanto dura.** Mezza giornata per il giro intero la prima volta. Le parti B, C, D, E sono
il percorso principale e non si saltano; le altre si possono distribuire su più sedute
purché il database non venga ricreato in mezzo.

---

## Parte A — Il deploy

Questi punti **non si ripetono qui**: sono il [runbook](07-operazioni.md#il-primo-deploy--runbook),
che è l'unico posto dove vivono. Duplicarli è il modo in cui nascono due runbook che
divergono. Si percorre il runbook, e in questo documento si annota soltanto l'esito.

| | Punto | Dove | Che cosa deve succedere |
|---|---|---|---|
| A1 | Le sonde dal dominio | runbook 4.1 | `https://<dominio>/api/v1/ready` risponde `200 ready`. Se dice `posta non raggiungibile` il relay è sbagliato, e va sistemato **adesso**: dopo il sintomo diventa muto. **`/ready` senza prefisso non prova niente** — non arriva all'API, risponde il file server con `index.html` e `200` |
| A2 | Swagger non è esposto | runbook 4.4 | `https://<dominio>/docs` risponde `404` |
| A3 | La privacy senza sessione | runbook 6 | `/privacy` si legge da fuori, e mostra un indirizzo e non il nome della variabile |
| A4 | Il backup | runbook 5 | `backup.sh` produce database **e** bucket, e la copia esce dalla macchina |
| A5 | **Il ripristino provato** | runbook 5 | Su un database vuoto: si entra con un account che esisteva **prima** del dump, si apre un contratto e si **scarica un documento**. Finché non è successo non c'è un backup, c'è un file |

---

## Parte B — Il primo ingresso

È il percorso che un database appena migrato impone: non c'è nessun account, e non esiste
una registrazione aperta ([ADR-053](08-decisioni.md#adr-053)).

| | Procedura | Che cosa deve succedere | Perché è qui |
|---|---|---|---|
| B1 | Emetti il primo invito da proprietario: `./ops/dev/issue-owner-invitation.sh <email>` | Stampa il token **una volta sola** sull'ultima riga. Non si rilegge: in database c'è solo lo hash | È l'unico modo di avere un account. Se questo passo non riesce, il prodotto è inaccessibile |
| B2 | Apri `/invito` da fuori sessione, incolla il token, **premi Invio senza toccare il bottone** | Compare il form di registrazione con il codice già dentro | **Il punto che nessun test può dare**: il browser pilotato manda eventi non fidati, quindi l'Invio è provato solo da un dito vero. In sospeso dal poscritto della sezione 8 |
| B3 | Completa: nome, password, nome dell'organizzazione | Si entra **già in sessione**, senza un secondo giro di login | `accept-owner-invitation` installa la sessione come il login |
| B4 | Guarda dove si atterra | Il **riepilogo vuoto**: tre gesti nell'ordine — registra un immobile, redigi un contratto, invita chi abita — e il rimando alla guida | È il primo accesso, ed è l'unica schermata che una persona nuova vede prima di qualunque dato |
| B5 | Apri la guida dal rimando e dal menù dell'account («Come si usa») | La versione di chi amministra: il ciclo — immobile, contratto, invito, addebiti, incassi. **Le sezioni si chiamano come le voci della navigazione** | La guida è l'unico posto che spiega il prodotto a chi non l'ha costruito |
| B6 | Esci, poi «Password dimenticata?»: chiedi il codice, incollalo, cambia la password | Il codice arriva **davvero** all'indirizzo, vale una volta sola, e dopo il cambio le sessioni aperte non si rinnovano più | Sulla macchina vera è anche la prova che la posta esce e **non finisce nello spam** — la sola cosa che `mailpit` in locale non può dire |
| B7 | Prova a riusare lo stesso codice di recupero | `401`, con lo stesso messaggio di un codice sconosciuto | Nessun oracolo sugli account ([ADR-017](08-decisioni.md#adr-017)) |
| B8 | Apri `/registrazione` | Non è un form: è una pagina che dice perché e dove andare | Chi arriva da un segnalibro vecchio non deve trovare un `404` muto |

---

## Parte C — Il ciclo di chi amministra

| | Procedura | Che cosa deve succedere | Perché è qui |
|---|---|---|---|
| C1 | Registra un immobile | Compare nell'elenco; l'elenco vuoto di prima diceva che cos'è la sezione e chi la riempie | — |
| C2 | Aggiungi **due stanze**, poi **correggine una** e archiviane una terza creata apposta | La correzione si fa dalla modale, che è la loro unica superficie; l'archiviata sparisce dall'elenco e non dalla storia | Le stanze non hanno una pagina propria: se la modale non basta, non c'è un'altra via |
| C3 | Redigi **due contratti per stanza** sullo stesso immobile, con le parti | La creazione avverte se il periodo è già occupato; il form propone il **residuo** della quota e offre l'intestatario solo se libero | Le due aritmetiche replicate dal server — quota e sovrapposizione — sono dove un form sbagliato produce dati sbagliati |
| C4 | Sbaglia apposta: metti una quota che non somma, poi correggila; togli una parte inserita per errore; annulla una bozza | Tutte e tre le vie esistono dall'interfaccia | Era il nodo della sezione 3: un contratto che non si attiva perché una quota è sbagliata, e non si elimina perché l'unica uscita sarebbe attivarlo |
| C5 | Attiva i due contratti | Lo stato passa e l'occupazione della stanza lo segue **senza che nessuno la scriva** | L'occupazione è derivata dalla `0033`: se diverge, la vista è tornata a contare una colonna |
| C6 | Invita le due persone che abitano, **revoca un invito** e rifallo | Il codice revocato, presentato davvero, viene **rifiutato** | La revoca vive nella modale che ha creato l'invito e in nessun altro posto: nessuna rotta li elenca |
| C7 | Carica un documento su immobile, contratto e stanza; archiviane uno | Sei owner dallo stesso componente, limite di 25 MiB controllato **prima** di inviare | — |

---

## Parte D — Chi abita entra

Due finestre da qui in avanti.

| | Procedura | Che cosa deve succedere | Perché è qui |
|---|---|---|---|
| D1 | La prima persona apre `/invito`, incolla il codice, si registra | Entra **nell'organizzazione di chi la ospita**, non in una propria | Dopo ADR-053 una persona che abita non ha un'organizzazione propria: se ne vede una, la regressione è grave |
| D2 | Guarda dove atterra | Sugli **addebiti**, non sul riepilogo: la rotta iniziale è del ruolo. Il vuoto dice **chi li emette** e che cosa arriverà | Il riepilogo le risponderebbe `403`: una voce che porta a un errore è peggio di una voce che manca |
| D3 | Conta le voci della navigazione | Cinque: addebiti, spese, segnalazioni, contratto, bollette. **Niente riepilogo, niente immobili** | — |
| D4 | Apri la guida dal menù dell'account | La versione di chi abita, con le sue sezioni | Due versioni, una pagina sola |
| D5 | Gira i vuoti: spese, bollette, pagamenti, allegati del contratto | Ognuno dice **che cos'è la sezione, che cosa la riempie e chi**, e **non** offre gesti che riceverebbero `403` | Ventidue stati vuoti riletti nella sezione 9: il caso mancante si vede solo usandoli |
| D6 | La seconda persona accetta il suo invito | Stesso esito, e le due si vedono come **coinquiline** nell'immobile | L'anagrafica è ristretta al perimetro della casa, non dell'organizzazione |

---

## Parte E — Il denaro

| | Procedura | Che cosa deve succedere | Perché è qui |
|---|---|---|---|
| E1 | Chi amministra registra una bolletta e ne carica la **scansione** | Se un fornitore OCR è configurato, la lettura compare campo per campo e si **rivede**; senza, la registrazione manuale resta la via | Il fornitore non è scelto ([ADR-037](08-decisioni.md#adr-037)): senza, questo punto prova la via manuale, e va scritto quale delle due si è percorsa |
| E2 | Valida la bolletta e **riaddebitala** a un contratto | La tendina elenca *immobile · stanza · intestatario* e i **soli contratti in corso**; il dialogo propone il **residuo**, non il totale | Un addebito annullato non conta nel residuo: è la regola che si perde |
| E3 | Dichiara la bolletta **pagata al fornitore** | È una data sulla bolletta, **non** un pagamento: non compare fra gli incassi | `app.payments` modella il denaro in entrata, e confondere le due cose sporca il registro |
| E4 | Emetti un addebito su un contratto; guardalo nello scadenzario e nella sua scheda | Stato, scadenza e residuo coerenti fra elenco e scheda | — |
| E5 | **Chi abita** dichiara di aver pagato | Lo vede fra i propri addebiti; la dichiarazione entra in `pending` | — |
| E6 | Chi amministra la trova nella **coda delle dichiarazioni** e la conferma | Il residuo scende; lo stato dell'addebito è **ricalcolato dai pagamenti** e non scritto a mano | — |
| E7 | Ripeti E5 e **rifiuta** la dichiarazione | Chi ha dichiarato lo vede; non resta un limbo | — |
| E8 | Registra un incasso **non attribuito**, poi uno in eccesso | L'attribuzione è una scelta esplicita, non un default; il credito da sovrapagamento è evidenziato sull'addebito e **resta fuori** dal riepilogo | Deciso con [ADR-040](08-decisioni.md#adr-040), e il riepilogo è la superficie dove un numero sbagliato si propaga |
| E9 | Rinuncia all'incasso su un addebito **parzialmente pagato** | La via esiste anche da lì, e il riepilogo mostra il **residuo** e non l'importo | La `0024` e [ADR-032](08-decisioni.md#adr-032) hanno corretto due numeri che mentivano |
| E10 | Guarda il riepilogo | **Nove** numeri, ognuno collegato al posto dove si agisce | È la prima superficie che attraversa tutti i moduli insieme: un errore di integrazione si vede qui |

---

## Parte F — Le segnalazioni

| | Procedura | Che cosa deve succedere | Perché è qui |
|---|---|---|---|
| F1 | Chi abita apre una segnalazione **con una foto** | L'allegato entra; la segnalazione compare a chi amministra | — |
| F2 | Chi ha aperto **corregge il testo** prima che qualcuno la prenda in carico | Si può; dopo `assigned` non più | La `0036` revoca su due colonne ciò che la `0020` concedeva |
| F3 | Chi amministra la porta per il ciclo: triage, presa in carico, sospensione, ripresa, chiusura | Sette transizioni; lo storico le mostra tutte con **chi** e quando | — |
| F4 | Dichiara una segnalazione **di interesse comune** | L'altra persona che abita la vede **con il thread, lo storico e le foto** — e non la scrive | Il quinto ramo sta in `app.can_access_ticket`: allargare la sola riga darebbe una segnalazione che si apre e non si legge |
| F5 | Scrivi nel thread da entrambe le parti; poi chiudi e prova a scrivere | Il thread chiuso si legge e non si scrive; **riaprire pretende un motivo**, che finisce nella conversazione | — |
| F6 | Parti dall'elenco senza averne mai aperta una | Il primo schermo è il **vuoto del filtro** («solo quelle aperte» è spuntato), che dice dove stanno le altre, e il gesto «Segnala un guasto» resta | Il vuoto di un filtro non offre il gesto della sezione: la via è togliere il filtro |

---

## Parte G — Le spese fra coinquilini

| | Procedura | Che cosa deve succedere | Perché è qui |
|---|---|---|---|
| G1 | Le due persone creano un gruppo di spesa | Chi lo crea è **già dentro** | La `0026` esiste per questo |
| G2 | Una spesa a testa, una `equal` e una `weighted` | Le quote **sommano esattamente**: le calcola il database, il client non le ricalcola | `parseFloat` rientrerebbe dalla finestra proprio qui |
| G3 | Una rimborsa l'altra; **chi riceve** conferma | I saldi tornano a zero **da entrambe le parti** — guardali da tutt'e due gli account | È il punto che il giro della 5-bis ha dichiarato di non aver verificato da entrambi i lati |
| G4 | Dichiara un rimborso e poi **ritiralo** | Il ritiro esiste qui e fra i pagamenti no | Asimmetria voluta: va vista, non dedotta |
| G5 | **Annulla una spesa già regolata** | Esce dai saldi da sola: un regolamento salda un saldo, non una spesa | La `0028` lo concedeva e nessuno lo faceva |
| G6 | Fai uscire una persona dal gruppo mentre ha **un conto aperto** | Resta nei saldi e il suo nome resta leggibile | Il nome si scrive all'ingresso ([ADR-043](08-decisioni.md#adr-043)): dopo non esiste più da nessuna parte |

---

## Parte H — Le persone, e la seconda volta

**È la parte che le suite non attraversano**: ogni test di integrazione costruisce una
persona nuova, e i difetti peggiori vivono nella *seconda* volta che qualcosa succede alla
stessa persona. È così che è stata trovata `R31`.

| | Procedura | Che cosa deve succedere | Perché è qui |
|---|---|---|---|
| H1 | Apri `/account`: correggi nome, telefono, lingua | Si correggono. L'**email si mostra e non si modifica**, con una riga che dice perché | Un campo disabilitato senza spiegazione si legge come un difetto |
| H2 | Cambia la password da dentro (pretende quella vecchia) | Tutte le **sessioni** si chiudono: l'altra finestra, al primo rinnovo, è fuori | Revoca i *refresh token*: un access token già emesso vive i suoi quindici minuti, e non è un difetto |
| H3 | Disdici il contratto di una delle due persone, poi falla **uscire** dall'organizzazione | Con il contratto in essere l'uscita è **rifiutata dal database**, non da un `if`; dopo la disdetta riesce | La `0039` e la sua policy |
| H4 | **Senza rifare il login**, con la stessa finestra, guarda i contratti e prova ad aprire una segnalazione | L'elenco **non porta più** il contratto disdetto; la segnalazione è rifiutata | È `R46`, chiuso dalla `0041` ([ADR-054](08-decisioni.md#adr-054)): prima quei quindici minuti erano dati leggibili |
| H5 | La stessa persona viene **reinvitata** su un contratto nuovo e accetta | Rientra, compare **una volta sola** fra le organizzazioni, e vede il contratto nuovo | La porta di servizio di [ADR-051](08-decisioni.md#adr-051): senza il ramo di riattivazione morirebbe con `409` |
| H6 | Fai uscire una persona dall'organizzazione **attiva** e ricarica | Non resta in un vicolo cieco: la schermata «nessuna organizzazione» spiega e offre la guida | È il difetto trovato nel giro della sezione 6: una richiesta che ripara uno stato non deve dipendere da quello stato |
| H7 | Prova a far uscire l'**ultimo proprietario** | Rifiutato | Un'organizzazione senza nessuno che la amministri non la vedrebbe più nessuno |

---

## Parte I — Quando la rete e il server non collaborano

| | Procedura | Che cosa deve succedere | Perché è qui |
|---|---|---|---|
| I1 | Due schede dello stesso account, lascia **scadere** l'access token, ricaricale **insieme** | Nessuna delle due viene buttata fuori | Il rinnovo è serializzato fra le schede ([ADR-044](08-decisioni.md#adr-044)); tre richieste scadute insieme farebbero revocare la catena |
| I2 | **Spegni l'API** lasciando su il proxy, poi ricarica una pagina interna | Uno **stato d'errore con «Riprova»**, non il form di accesso, e nessun «Esci» | È `R51`: il `502` del proxy e la rete caduta sono lo stesso sintomo per chi guarda |
| I3 | Riaccendi l'API e premi «Riprova» | Prima il **caricamento**, poi si è dentro dove si stava andando | Senza il caricamento, la riprova sembra non fare niente |
| I4 | Metti il telefono in modalità aereo per qualche secondo dentro una schermata, poi torna | Il messaggio parla di connessione, e ciò che era stato digitato non si perde | Sulla macchina vera, in 4G, è il caso normale |
| I5 | Apri il prodotto in un **secondo browser** dopo aver lasciato un'organizzazione nel primo | Al massimo un lampo: l'elenco corregge l'organizzazione ricordata | È `R49`, lasciata aperta con questa ragione: va confermato che sia un lampo e non un vicolo cieco |

---

## Parte J — Dal telefono

**È il criterio di fine del primo deploy**, non un punto in coda: *aprire il prodotto dal
telefono, in 4G* — non dalla stessa rete della macchina.

| | Procedura | Che cosa deve succedere |
|---|---|---|
| J1 | Il percorso di chi abita per intero, dal telefono: ingresso da invito, addebiti, dichiarazione di pagamento, una segnalazione con foto scattata sul momento | Nessuna schermata che chieda di girare il telefono, nessun campo irraggiungibile sopra la tastiera |
| J2 | La navigazione a scomparsa | Si apre, si chiude, e il salto al contenuto funziona |
| J3 | Il percorso di chi amministra, almeno fino a emettere un invito | Le tabelle larghe scorrono invece di tagliare |
| J4 | A 375 px sul desktop, gli stessi due percorsi | Uguale: è la larghezza su cui il client è stato progettato |

---

## Parte K — Cancellare e archiviare, di traverso

| | Procedura | Che cosa deve succedere | Perché è qui |
|---|---|---|---|
| K1 | Da ogni elenco che lo offre, archivia una riga: immobile, stanza, documento, gruppo di spesa | Il verbo c'è **dall'elenco**, non solo dalla scheda, e la riga sparisce senza sparire dalla storia | Era la sezione 7: la cancellazione è trasversale o non c'è |
| K2 | Seleziona **più righe** e archiviale in blocco | «Seleziona tutto» è *le righe caricate*; le chiamate vanno **in sequenza**; un `409` «era già così» non è rosso | Un giro in parallelo su N righe è il modo di far cadere la catena delle sessioni |
| K3 | Archivia un immobile che ha **un contratto in essere** | Oggi non protesta nulla — è `R45`, aperta | Va confermato che sia ancora così e non peggio |
| K4 | Guarda un collegamento a qualcosa di archiviato | Dice che è archiviato invece di far sparire la riga | — |

---

## Parte L — Alla fine

1. **Rileggi le righe che hai scritto** e separa: che cosa ha retto, che cosa no, che cosa
   non hai potuto provare e perché. Le tre categorie servono tutte.
2. **Scrivi l'esito in [11](11-piani-conclusi.md)**, con la data e con chi l'ha percorso.
   Se chi ha percorso il giro non ha scritto il codice, **dillo**: è la condizione che rende
   il giro diverso da una suite.
3. **Apri le voci in [12](12-da-fare.md)** per ciò che hai trovato, una per difetto, con
   l'identificatore nuovo e ciò che si sa già. Anche per ciò che non si correggerà adesso.
4. **Aggiorna [09](09-stato-e-roadmap.md)** se il giro cambia l'ordine di ciò che viene
   dopo. Dopo il primo mese con una persona vera, è probabile che lo cambi.

**Un giro che finisce con «tutto bene» è un giro che non è stato fatto.** Il precedente, su
un prodotto già passato per tre revisioni e 478 test verdi, ha trovato sei difetti in mezza
giornata, e uno era un buco nel ciclo di vita normale.
