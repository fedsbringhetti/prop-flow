# 04 — Sicurezza

## Difesa a strati

Nessun singolo controllo è l'unico. Le proprietà importanti sono imposte in più
punti, in modo che un errore in uno non diventi una fuga di dati.

| Proprietà | Imposta da |
|---|---|
| Isolamento fra organizzazioni | Policy RLS (autorità) + `authorization::ensure_member` (semantica del 403) |
| Soft delete anziché cancellazione | `DELETE` non concesso al ruolo applicativo + `deleted_at` nelle query |
| Nessun SQL fuori dal layer dati | `AppState` non espone il pool + controllo grep in CI |
| Il dominio non tocca l'infrastruttura | Confini fra crate + `check-domain-purity.sh` |
| Nessun segreto nei log | Redazione nell'audit + `Debug` redatto + `log_statement=none` in PostgreSQL |
| Traccia non falsificabile | Trigger `SECURITY DEFINER` + `audit_log` append-only + privilegi revocati |

## Autenticazione

### Access token

JWT firmato **EdDSA (Ed25519)**, non HMAC. Quando i moduli si separeranno in
servizi, ognuno potrà verificare i token con la sola chiave pubblica; con HS256
ogni verificatore sarebbe anche un potenziale emittente.

- Vita: **15 minuti** (`auth.access_token_ttl_seconds`). Breve perché **non è
  revocabile**.
- Claim: `sub`, `org`, `role`, `prole`, `iss`, `aud`, `exp`, `iat`, `jti`.
- `exp`, `iss`, `aud`, `sub` sono obbligatori in validazione: un token senza
  scadenza che passasse la verifica sarebbe eterno. Tolleranza di 5 secondi per lo
  sfasamento fra orologi.
- `role` serve a rispondere `403` senza interrogare il database. **Non è
  autorevole**: la RLS rilegge sempre la membership.

Le chiavi si passano come **percorsi di file**, non come contenuto: in Kubernetes
arrivano come Secret montati, e una chiave privata in una variabile d'ambiente è
leggibile da `/proc`, dai dump di crash e da `docker inspect`. Generazione:
`./ops/dev/generate-jwt-keys.sh`, che cerca un OpenSSL 3 perché l'`openssl` di
sistema su macOS è LibreSSL e non conosce Ed25519.

### Refresh token

**Opachi**, non JWT: devono poter essere revocati prima della scadenza, e un JWT
autoconsistente resta valido anche dopo un logout. La fonte di verità è la riga in
`app.user_sessions`, che si può revocare.

- 256 bit di entropia da `getrandom`, codificati base64url.
- In database va solo lo **hash SHA-256**. Un digest semplice basta: con 256 bit di
  entropia non esiste il dizionario che renderebbe utile un KDF lento. Ma una
  copia del dump non deve permettere di impersonare nessuno.
- Vita: **30 giorni**, e la finestra è **scorrevole**. I trenta giorni contano
  dall'**ultimo uso** e non dal login: la rotazione emette un token nuovo con
  `expires_at = now + 30 giorni`, quindi chi apre il prodotto anche una sola volta
  al mese resta dentro a tempo indeterminato. **Non esiste un tetto assoluto**, né
  un timeout di inattività più corto, né una scelta di chi entra: vale sempre e per
  tutti.

  Va scritto perché «vita: 30 giorni» si legge come un tetto, e non lo è. Le uniche
  cose che chiudono una sessione sono il logout, il cambio o il recupero della
  password, e il rilevamento del riuso. Che sia la forma giusta è una domanda
  aperta e ha una voce sua in [12](12-da-fare.md).
- **Rotazione a uso singolo**: ogni refresh consuma il token presentato ed emette
  uno nuovo, con una scadenza nuova.
- **Viaggia solo in un cookie** `HttpOnly; Secure; SameSite=Strict;
  Path=/api/v1/auth`, e non compare più nel corpo di alcuna risposta. Metterlo
  anche nel JSON del login vanificherebbe `HttpOnly`: un XSS lo leggerebbe da lì
  senza toccare `document.cookie`. Ne segue che **questa API non ha una difesa
  CSRF e non deve averne una** — l'argomento, e la condizione che lo tiene in
  piedi, sono in [ADR-038](08-decisioni.md#adr-038).

#### Rilevamento del riuso

Se arriva un token già ruotato, esistono due copie e non si sa quale parte sia
legittima: **cadono tutte le sessioni dell'utente**. Il caso è modellato nel
dominio come violazione di invariante (`RefreshSession::ensure_usable`), non come
semplice rifiuto, ed è coperto da un test che verifica che anche il successore
diventi inutilizzabile.

L'ordine di scrittura conta: il successore va inserito **prima** di aggiornare
`rotated_to`, che è una foreign key autoreferenziale. L'ordine inverso viola il
vincolo — è un bug che i test hanno trovato.

### Password

- **Argon2id**, profilo OWASP: 19 MiB di memoria, 3 iterazioni, parallelismo 1. La
  memoria è la difesa che conta contro il cracking su GPU, dove la potenza di
  calcolo è abbondante ma la RAM per thread no.
- L'hashing gira su `spawn_blocking`: decine di millisecondi di CPU sul reattore
  async bloccherebbero altre richieste. Per questo la porta `PasswordHasher` è
  `async`.
- Policy: minimo **12 caratteri**, massimo 512, nessun obbligo di classi miste. Le
  regole di composizione spingono verso password prevedibili tipo `Password1!`,
  mentre la lunghezza è la leva che conta. Il massimo non è estetico: Argon2 lavora
  sull'intero input, e una password da un megabyte è un vettore di denial of
  service.
- `PasswordHash` ha un `Debug` che stampa `<redatto>`: la strada più comune perché
  un hash finisca in un log è un `dbg!` dimenticato.

### Anti-enumerazione degli account

Tre accorgimenti, tutti verificati da test:

1. Credenziali errate e account inesistente producono **la stessa risposta**:
   `401` con lo stesso corpo.
2. Quando l'utente non esiste si spende comunque il tempo di una verifica Argon2 su
   un hash di riferimento generato all'avvio: senza questo, la latenza
   distinguerebbe gli account esistenti a parità di risposta.
3. Un'email malformata al login dà `401`, non `422`: dire "email non valida" a chi
   prova indirizzi a caso è informazione gratuita.

Analogamente, `404` e non `403` quando una risorsa non è visibile: un `403`
confermerebbe che esiste.

Il codice d'errore introdotto da [ADR-022](08-decisioni.md#adr-022) non riapre
nessuna di queste porte: i due `401` del punto 1 portano anche lo **stesso `code`**
(`auth.invalid_credentials`), ed è verificato da un test. L'unica distinzione
concessa è fra access token scaduto (`auth.token_expired`) e non valido
(`auth.token_invalid`), che non è un oracolo — per ottenere il primo bisogna
presentare un token che abbiamo firmato noi, quindi si dice solo a chi già lo sapeva.
Il **refresh** invece resta indistinto: lì separare "scaduto" da "riusato" direbbe a
chi ha rubato un token se è ancora vivo.

### Il recupero della password non riapre nessuna di queste porte

Le stesse tre proprietà, applicate a una rotta nuova. La risposta è `202` **sempre**,
senza corpo: un indirizzo noto e uno sconosciuto ricevono la stessa cosa. Il **tempo**
è lo stesso, perché per un indirizzo senza account si calcola comunque lo hash di un
valore fittizio — è il punto 2 qui sopra, spostato dalla verifica Argon2 alla
generazione del codice. E il limite agli invii rifiuta **in silenzio**: dire «troppo
presto» confermerebbe che l'account esiste.

Ne segue una cosa che va saputa: **un fallimento dell'invio non può essere riportato a
chi chiede**. Si tenta solo per un indirizzo che ha un account, quindi qualunque
differenza — un codice, un messaggio, una durata — riaprirebbe l'oracolo. Un relay di
posta caduto è perciò **muto**, e a renderlo visibile è il controllo di prontezza, che
interroga SMTP insieme al database.

Il codice vale trenta minuti e una volta sola; sconosciuto, scaduto e già usato danno lo
stesso `401`. In database c'è lo **hash**, come per il token d'invito e per il refresh:
il valore in chiaro esiste solo nell'email.

**Confermare revoca tutte le sessioni**, e la frase va letta con precisione: revoca i
**refresh token**. L'access token è un JWT firmato e non è revocabile
([ADR-013](08-decisioni.md#adr-013)), quindi chi ne ha uno in mano resta dentro fino
alla scadenza — quindici minuti. La catena si chiude, e da lì in poi non si rinnova più.

### Cambiare la password da dentro chiude tutto, come il recupero

`POST /api/v1/me/password`, e **pretende la password corrente** anche se chi chiama è
già autenticato. Non è cerimonia: il contesto è diverso da quello del recupero. Là la
prova d'identità è il controllo della casella di posta; qui è **una sessione**, e una
sessione può essere una scheda lasciata aperta. Senza la vecchia, chiunque si sieda
davanti a un browser non bloccato si prende l'account per sempre.

**Revoca tutte le sessioni, compresa quella da cui si è chiesto**, esattamente come il
recupero — e l'uniformità è la decisione, presa contro l'alternativa che sembra più
gentile. *«Cambiare la password chiude tutto»* è **una frase sola**, e va scritta in
quattro posti: la schermata del recupero, quella del proprio account,
[05](05-api.md#il-proprio-account) e questo documento. Due regole diverse per due
percorsi che una persona non distingue divergono alla prima modifica. Vale identica la
precisazione qui sopra: si revocano i **refresh token**, e chi ha un access token in
mano resta dentro fino alla scadenza — ciò che muore all'istante è il **rinnovo**.

**La password corrente errata dà `401` e non incrementa il contatore dei tentativi**, al
contrario del login. Il blocco dopo dieci tentativi difende chi *non* è autenticato; qui
chi prova ha già una sessione, e incrementarlo darebbe a chiunque rubi una scheda il
modo di bloccare fuori il legittimo proprietario. Per la stessa ragione il cambio da
dentro **non sblocca** un account bloccato, mentre il recupero sì: le due prove
d'identità non sono la stessa, e il pool applicativo non ha comunque il privilegio di
scrivere `locked_until` (migration `0038`).

### Blocco dopo tentativi falliti

Dieci tentativi consecutivi bloccano l'account per **15 minuti**. Temporaneo e non
permanente: un attacco a dizionario non deve tradursi in un blocco definitivo, che
sarebbe un denial of service verso l'utente legittimo. Il contatore viene
persistito anche quando la richiesta fallisce, altrimenti non conterebbe nulla.

## Autorizzazione

### Ruoli di organizzazione

| Ruolo | Staff | Poteri |
|---|---|---|
| `owner` | sì | tutto nell'organizzazione, incluso amministrarla |
| `manager` | sì | operazioni sul patrimonio, contratti, contabilità |
| `tenant` | no | i propri contratti, addebiti, segnalazioni, spese condivise |

`is_staff` include anche l'amministratore di piattaforma
(`users.platform_role = 'admin'`).

### Cambio di organizzazione

Header `X-Org-Id`. Se coincide con l'organizzazione del token, il ruolo nel token
vale. Se è diversa, quel ruolo non si applica e la membership viene **riletta dal
database**: la policy su `org_members` mostra a ciascuno le proprie appartenenze
(`user_id = app.current_user_id()`), quindi basta un contesto con il solo utente.

Non si distingue "organizzazione inesistente" da "non ne fai parte": la differenza
rivelerebbe l'esistenza di organizzazioni altrui.

### Il pool privilegiato

`begin_privileged()` usa un ruolo con `BYPASSRLS`. Ammesso **solo** dove un
contesto di sicurezza non può esistere:

- registrazione — utente, organizzazione e membership non esistono ancora;
- login — la policy su `users` mostra una riga solo a chi è già identificato;
- rotazione del refresh token — il chiamante presenta un token, non un'identità;
- **recupero della password** — chi l'ha dimenticata non è autenticato per
  definizione, quindi non c'è un `current_user_id` da installare e la Row Level
  Security non ha nulla su cui appoggiarsi ([ADR-052](08-decisioni.md#adr-052));
- **accettazione di un invito da proprietario** e **registrazione di un
  inquilino da invito** — le due rotte di [ADR-053](08-decisioni.md#adr-053):
  come la registrazione, creano utente (e per la prima, organizzazione) quando
  nessuno dei due esiste ancora. La prova di possesso del token è tutta
  dell'applicazione — hash e stato, con la stessa forma del recupero della
  password — perché qui non c'è nessuna Row Level Security a fare da seconda
  guardia, a differenza dell'accettazione di un invito da inquilino qui sotto.

Ogni altro uso è un errore di progetto — e il **cambio password da autenticati** è il
caso che mostra dove passa il confine: somiglia al recupero, condivide con esso la
validazione e la revoca delle sessioni, e gira sul pool **applicativo**, perché chi lo
chiede una sessione ce l'ha. Il logout, per contrasto, passa dal
contesto normale: la policy su `user_sessions` impedisce di revocare la sessione di
qualcun altro anche presentandone l'hash corretto.

Lo stesso vale per l'**accettazione di un invito**, che sarebbe stata il candidato
naturale a un quarto caso: scrive in un'organizzazione di cui chi accetta non è
ancora membro, ma un contesto di sicurezza *esiste* — è autenticato — e manca solo la
membership. La risposta è stata estendere la Row Level Security
([ADR-020](08-decisioni.md#adr-020)), e un test di integrazione esegue quel flusso
con una fabbrica il cui `begin_privileged()` solleva un panic.

### Inviti

Il token di invito è opaco, 256 bit, con lo **hash** SHA-256 in database: stesso
trattamento del refresh token, e per lo stesso motivo — una copia del dump non deve
permettere di entrare in un'organizzazione. Il valore in chiaro esiste solo nella
risposta che lo consegna, e la colonna `token_hash` è fra quelle **redatte**
nell'audit log.

Due controlli indipendenti proteggono l'accettazione: l'applicazione confronta lo
hash del token, le policy verificano che esista un invito pendente per l'email
dell'utente corrente. Un invito può conferire solo il ruolo `tenant`, e la condizione
è ripetuta nella policy: un token spedito per email non deve poter diventare un ruolo
di staff nemmeno con una query scritta a mano.

Un token sconosciuto e un token intestato a un'altra email danno lo stesso `401` con
lo stesso corpo — `code` compreso — per la ragione di
[ADR-017](08-decisioni.md#adr-017): distinguerli sarebbe un oracolo per capire quali
inviti esistono. Lo stesso vale per un invito scaduto o revocato: il dominio sa
distinguerli e i codici esistono, ma il caso d'uso li ripiega tutti su
`auth.invalid_credentials` prima della risposta ([ADR-022](08-decisioni.md#adr-022)). La scadenza è di **sette
giorni**: abbastanza perché un'email non urgente venga letta, poco perché una casella
compromessa mesi dopo valga ancora come credenziale.

### Inviti da proprietario

Stessa forma dell'invito da inquilino — token opaco a 256 bit, hash SHA-256 in
database, sette giorni di validità di default — con due differenze che discendono
da [ADR-053](08-decisioni.md#adr-053).

**Nessuna prova aggiuntiva oltre il token.** L'invito da inquilino ha due controlli
indipendenti: l'hash del token e la policy che verifica un invito pendente per
l'email di chi è già autenticato. Qui non c'è un'email da confrontare con quella di
un utente esistente — l'utente non esiste ancora — quindi il token è l'unica prova
che esiste, ed è anche per questo che lo emette uno script con il ruolo del
migratore e non una rotta: verificare che il possessore del token sia davvero un
proprietario è un giudizio umano che sta fuori dal prodotto.

**Quattro esiti, una sola risposta.** Scaduto, revocato e **già accettato** danno lo
stesso `401 auth.invalid_credentials` di un token inesistente — a differenza
dell'invito da inquilino, dove "già accettato" resta un `409` distinto perché chi lo
presenta è già autenticato e merita di saperlo. Qui non lo è: distinguere "già
accettato" da "mai esistito" sarebbe un oracolo su quali token sono stati emessi,
esattamente come per un token mai emesso.

**Il token non sblocca da solo il ruolo di piattaforma.** `platform_role = 'admin'`
resta assegnato a mano — ADR-053 non lo tocca — e l'invito da proprietario dà
soltanto ciò che `RegisterOwner` dà oggi a chiunque: un'organizzazione personale con
sé stesso come `owner`. Le due cose restano indipendenti finché non nasce una rotta
che usi `is_platform_admin()`.

## Dati personali

### Il perimetro dell'anagrafica è la casa, non l'organizzazione

Chi può leggere la riga `app.users` di un'altra persona — nome, **email**, **telefono**
— è una garanzia, e dalla migration `0030` è più stretta di prima. `users_select`
ammette cinque casi:

| Ramo | Chi vede chi |
|---|---|
| `id = app.current_user_id()` | sé stessi |
| `app.shares_home_with(id)` | chi ha una parte contrattuale su un contratto dello stesso immobile |
| `app.is_staff_of_user(id)` | lo staff vede le persone delle proprie organizzazioni |
| `app.is_my_staff(id)` | e l'inquilino vede lo staff della propria |
| `app.is_platform_admin()` | l'amministratore di piattaforma |

**Prima era `app.shares_org_with`, cioè l'organizzazione intera.** Un inquilino leggeva
la riga di ogni altra persona dell'organizzazione, inquilini di immobili diversi
compresi — persone con cui non ha alcun rapporto. Nessun endpoint lo esponeva, quindi
non trapelava nulla: era un **pavimento** mancante, non una fuga in corso.

**Restringere non ha fatto fallire alcun test**, e va letto per quello che è: prova che
la capacità per cui `shares_org_with` esisteva non era mai stata costruita — `app.users`
compare in tre query, tutte nei flussi di autenticazione, tutte sul chiamante stesso.
Non prova che restringere sia sicuro. La verifica è una matrice di visibilità su uno
scenario a due immobili, ed è un test di integrazione. Ragioni in
[ADR-036](08-decisioni.md#adr-036).

**Quella policy è ora portante, e non lo era.** La `0030` aveva scritto che la finestra
per restringere l'avrebbe chiusa il primo client che leggesse un nome da lì: è la sezione
5 delle segnalazioni, che risolve i nomi di chi ha segnalato, di chi ha in carico e di chi
scrive nel thread leggendo `app.users.full_name` **sotto queste cinque condizioni**
([ADR-049](08-decisioni.md#adr-049)). Da qui in avanti restringere `users_select` non è
più gratuito, e i due rami che quel modulo poggia sono `shares_home_with` — il coinquilino
legge il nome dell'altro — e `is_my_staff` — l'inquilino legge il nome di chi ha in carico
la sua segnalazione.

La proiezione è di **una colonna sola**, `full_name`, e non dell'aggregato `User`: email e
telefono restano dove sono. È la stessa forma di `app.property_residents` — proiettare
invece di rendere leggibile — e la ragione per cui un nome che non torna non è un errore,
ma un'etichetta neutra a schermo.

I due rami sullo staff non sono decorazione e **non sono l'uno l'inverso dell'altro**:
senza il terzo il proprietario perderebbe di vista i propri inquilini, senza il quarto
un inquilino non potrebbe mai sapere il nome del proprio proprietario.

### L'elenco dei residenti proietta, e per questo non porta l'email

`GET /properties/{id}/residents` restituisce `user_id`, `full_name` e `room_name` —
quanto basta ad aggiungere qualcuno a un gruppo di spesa e a riconoscerlo — e **non
l'email**.

Non è prudenza a costo: la risposta viene da `app.property_residents`, una funzione
`SECURITY DEFINER` che proietta tre colonne senza rendere leggibile la riga da cui
vengono. Con la via alternativa — allargare `lease_parties_select` e `leases_select` —
la riga sarebbe uscita **intera**, e con essa `leases.rent_amount`: il canone dei
coinquilini. Una policy è per riga e non per colonna.

- **Cifratura applicativa** per i campi sensibili: `fiscal_code_encrypted`,
  `mfa_secret_encrypted`. Il database vede solo il testo cifrato e un riferimento
  logico alla chiave (`encryption_key_ref`, es. `kms:v3`), mai il materiale
  crittografico. Il provider di cifratura è dietro interfaccia.
- `fiscal_code_fingerprint` è un HMAC-SHA256 che permette lookup e deduplica senza
  decifrare.
- `documents.is_encrypted` e `encryption_key_ref`: i file sono cifrati a riposo
  nello storage oggetti.
- `retention_until` sui documenti abilita un purge job. Non esiste ancora un job.
- Soft delete: la cancellazione di un utente va progettata insieme agli obblighi di
  conservazione fiscale, che superano il diritto alla cancellazione per i dati
  contrattuali.

## Logging

Regola: **niente dati personali nei log, nelle tracce, negli eventi.**

- I layer applicativi non registrano email, token o corpi di richiesta.
- `PasswordHash` e `TokenHash` hanno `Debug` redatto. Farlo anche per i tipi nuovi
  che contengono segreti.
- L'audit redige le colonne sensibili prima di scriverle.
- PostgreSQL gira con `log_statement=none`: i parametri delle query contengono dati
  personali.
- Gli errori interni restituiscono al client solo il `request_id`; il dettaglio
  tecnico resta nei log correlato da quello. Un messaggio di errore del database in
  una risposta HTTP è ricognizione gratuita.
- Gli eventi di dominio trasportano identificatori e nomi di campo, non valori.
- **L'adattatore SMTP è il posto più facile in cui far uscire dati personali**, perché
  una notifica *deve* trasportarli: senza l'indirizzo non si recapita niente. Un client
  che registrasse la richiesta scriverebbe destinatario, oggetto e corpo insieme. Non
  si registra nulla dell'invio riuscito, e del fallito solo la **classe** dell'errore.
  `Notification` e `NotificationContent` hanno `Debug` redatto, e l'adattatore non lo
  annulla. Il fallimento di un invito registra l'identificatore dell'invito, mai il
  destinatario né il token; quello di un codice di recupero non registra nemmeno
  l'identificatore, perché lì l'unica cosa da cui si risalirebbe è l'indirizzo.

## Difese al bordo HTTP

| Difesa | Come |
|---|---|
| Limite sul corpo | `RequestBodyLimitLayer`, 1 MiB per default |
| Timeout | 30 secondi, risposta `503` |
| `statement_timeout` | 10 secondi per connessione, impostato con `set_config` |
| Request id | Generato se assente, propagato in risposta |
| `WWW-Authenticate` | Presente sui `401` |
| Errori uniformi | RFC 9457 anche per il `404` di rotta sconosciuta |

## Contenitore

Utente non privilegiato `uid 10001`, filesystem in sola lettura,
`no-new-privileges`. Un processo che non ha bisogno di root non deve averlo.

## Ciò che non c'è ancora

Elenco onesto, con i dettagli in [09 — Stato e roadmap](09-stato-e-roadmap.md).

- **Rate limiting** su login e registrazione. Il lockout per utente c'è, un limite
  per IP no: oggi è demandato all'ingress.
- **Verifica dell'email** bloccante: il campo `email_verified_at` esiste, il flusso
  no.
- **MFA**: la colonna esiste, la logica no.
- **CORS** non configurato (il layer è disponibile, non è montato). **Non è un
  debito da chiudere ma una scelta**: client e API stanno sulla stessa origine
  dietro un reverse proxy, quindi non c'è nessuna richiesta cross-origin da
  autorizzare, e non montarlo è metà dell'argomento per cui non serve una difesa
  CSRF ([ADR-038](08-decisioni.md#adr-038)). Resta aperto per il giorno in cui
  esistesse un client di terze parti, e quel giorno sarà una decisione sua.
- **Gate sulle licenze** delle dipendenze: la CI controlla vulnerabilità, crate
  ritirati e provenienza, non le licenze.
- **Rotazione delle chiavi** di firma: non c'è un `kid` nei token, quindi ruotare
  la chiave invalida i token in circolazione.
