# 05 — API

Base URL: `/api/v1`. Documento OpenAPI generato dal codice: `/openapi.json`,
Swagger UI su `/docs`.

La specifica è **generata dai derive `utoipa` sugli handler**, non scritta a mano:
una specifica mantenuta a parte divergerebbe dal codice alla prima modifica, e una
specifica sbagliata è peggio di nessuna specifica.

## Convenzioni

| Aspetto | Scelta |
|---|---|
| Autenticazione | `Authorization: Bearer <access_token>` |
| Organizzazione attiva | quella nel token; `X-Org-Id: <uuid>` per cambiarla |
| Correlazione | `X-Request-Id` in ingresso, generato se assente, sempre restituito |
| Formato | JSON. Errori in `application/problem+json` |
| Nomi | `snake_case` nei corpi, `kebab-case` nei percorsi |
| Enum sul filo | `snake_case` (`apartment`, `partially_rented`) tranne le classi energetiche, maiuscole (`A4`, `G`) |
| Importi | stringhe decimali (`"62.50"`), non float: un binario a virgola mobile non rappresenta esattamente 0.10. Vale per ogni valore decimale, comprese percentuali e metrature |
| Date | ISO 8601 UTC |
| Creazione | `201` con la risorsa nel corpo |
| Eliminazione | `204` senza corpo (è un'archiviazione) |

## Modello di errore

RFC 9457, uno per tutta l'API: il client ha un contratto da gestire invece di
indovinare la forma della risposta endpoint per endpoint.

```json
{
  "type": "https://propflow.dev/problems/validation",
  "title": "Validation failed",
  "status": 422,
  "code": "validation.failed",
  "request_id": "9c9a0514-6dd6-4f40-ba71-74ab825e8a9d",
  "errors": [{ "field": "password", "message": "must be at least 12 characters long" }]
}
```

`detail` ed `errors` sono presenti solo quando informano; `errors` compare sulle
validazioni. `code` c'è sempre.

I messaggi (`title`, `detail`, `errors[].message`) sono **in inglese**: sono
superficie pubblica, e la documentazione interna resta italiana.

### `type`, `code` e chi usa quale

`type` è la **classe** del problema, `code` è la **condizione**. Servono a due cose
diverse: `type` a gestire "tutti i conflitti" senza enumerarli, `code` a decidere
cosa mostrare o cosa riprovare.

**`code` è il contratto.** `title` e `detail` sono per un umano che legge i log e
possono essere riscritti quando si trova una formulazione migliore; `code` no.
Un client che confronti i messaggi si romperà, ed è il motivo per cui `code` esiste
(ADR-022). La localizzazione è del client: il server non sa in quale lingua né in
quale registro va mostrato l'errore.

Aggiungere un codice è additivo, rinominarne uno è una rottura.

| Stato | `type` | Quando |
|---|---|---|
| 400 | `bad-request` | header malformato |
| 401 | `unauthorized` | token assente, non valido o scaduto; credenziali errate |
| 403 | `forbidden` | autenticato ma senza il ruolo necessario |
| 404 | `not-found` | risorsa inesistente **o non visibile**, e rotta sconosciuta |
| 409 | `conflict` | vincolo di unicità violato, o periodo occupato |
| 409 | `invariant` | operazione non ammessa nello stato attuale (es. immobile archiviato) |
| 413 | `payload-too-large` | corpo oltre il limite della rotta |
| 422 | `validation` | corpo sintatticamente valido, semanticamente no |
| 500 | `internal` | errore non previsto: solo `request_id`, nessun dettaglio |
| 503 | — | timeout della richiesta, o `/ready` con database non raggiungibile |

### Codici

**Richiesta e autenticazione**

| `code` | Stato | Condizione |
|---|---|---|
| `request.invalid_header` | 400 | `X-Org-Id` o `Idempotency-Key` non sono testo valido, o `X-Org-Id` non è un UUID |
| `request.missing_header` | 400 | manca un header obbligatorio per la rotta: oggi solo `Idempotency-Key` sulle due rotte che creano un pagamento |
| `request.payload_too_large` | 413 | corpo oltre il limite della rotta |
| `auth.token_missing` | 401 | nessun header `Authorization: Bearer` |
| `auth.token_invalid` | 401 | firma o forma del token rifiutate |
| `auth.token_expired` | 401 | token nostro, `exp` superato — **rinnovare**, non rifare il login |
| `auth.invalid_credentials` | 401 | login fallito, refresh non valido, token d'invito rifiutato |
| `auth.required` | 403 | l'operazione richiede un utente autenticato |
| `auth.denied_by_policy` | 403 | rifiutato da una policy di riga; se compare è un difetto nostro |
| `org.not_selected` | 403 | manca l'organizzazione attiva |
| `org.not_a_member` | 403 | non appartieni a questa organizzazione |
| `org.staff_role_required` | 403 | serve il ruolo `owner` o `manager` |
| `ticket.not_yours` | 403 | riaprire o annullare una segnalazione spetta a chi l'ha aperta, e nessun ruolo lo sostituisce |
| `settlement.not_addressed_to_you` | 403 | confermare o rifiutare un regolamento spetta a **chi riceve**: è l'unico che sa se il bonifico è arrivato |
| `settlement.not_declared_by_you` | 403 | ritirare un regolamento spetta a **chi lo ha dichiarato**. A chi riceve tocca il rifiuto, che è un fatto diverso |
| `expense_group.membership_not_yours` | 403 | si esce da un gruppo di spesa per sé stessi; togliere qualcun altro richiede lo staff. Dalla `0029` lo dice anche la RLS: il codice qui è la versione parlante di un rifiuto che la policy darebbe comunque |
| `expense.not_yours` | 403 | una spesa la annulla chi l'ha **registrata**, chi l'ha **anticipata**, o lo staff. Qui la RLS è più larga — `expenses_update` ammette qualunque membro attivo del gruppo — quindi è un confine del contratto dell'API e non di sicurezza |

**Risorse non trovate**

| `code` | Stato |
|---|---|
| `property.not_found` · `room.not_found` · `lease.not_found` · `lease.party_not_found` · `invitation.not_found` · `membership.not_found` · `document.not_found` · `ticket.not_found` · `utility_bill.not_found` · `charge.not_found` · `payment.not_found` · `expense_group.not_found` · `expense.not_found` · `settlement.not_found` | 404 |
| `endpoint.not_found` | 404 (rotta sconosciuta) |

**Conflitti**

| `code` | Stato | Condizione |
|---|---|---|
| `auth.email_taken` | 409 | esiste già un account con questa email |
| `room.name_taken` | 409 | nome di stanza già usato nell'immobile |
| `invitation.already_pending` | 409 | esiste già un invito pendente per questa email |
| `lease.primary_party_exists` | 409 | il contratto ha già un intestatario principale |
| `membership.already_exists` | 409 | l'utente appartiene già all'organizzazione |
| `lease.period_overlap` | 409 | il periodo si sovrappone a una locazione attiva |
| `utility_bill.duplicate` | 409 | fattura già registrata per questo fornitore: `utility_bills_dedup_uk` è unico su `(org_id, supplier_vat, invoice_number)` e scatta solo quando **entrambi** sono valorizzati |
| `payment.idempotency_key_reused` | 409 | la chiave di idempotenza è già stata usata per un **altro** pagamento. Non è il doppio invio, che risponde `200`: è la stessa chiave verso un addebito diverso, oppure una chiave già usata da qualcuno di cui il chiamante non vede i pagamenti |
| `expense_group.membership_exists` | 409 | la persona è già membro attivo del gruppo. `expense_group_members_group_user_uk` non è parziale: il rientro è un aggiornamento, non un secondo inserimento |
| `invitation.owner_token` | 409 | `register-from-invitation` ha ricevuto un token da **proprietario**, pendente e valido: il client deve rimandare lo stesso corpo, più il nome dell'organizzazione, a `accept-owner-invitation` |
| `conflict.constraint_violated` | 409 | vincolo violato che l'API non sa ancora nominare |

**Stato non ammesso**

| `code` | Stato | Condizione |
|---|---|---|
| `property.archived` | 409 | immobile archiviato, non modificabile |
| `property.archive_requires_dedicated_operation` | 409 | archiviare non si fa con un `PATCH` di stato |
| `room.archived` | 409 | stanza archiviata, non modificabile |
| `lease.invalid_transition` | 409 | attivazione, disdetta o annullamento non ammessi nello stato attuale |
| `lease.party_already_linked` | 409 | la parte è già collegata a un altro utente |
| `lease.no_tenant_party` | 409 | il contratto non ha inquilini fra le parti |
| `lease.shares_not_hundred` | 409 | le quote degli inquilini non sommano a 100 |
| `invitation.already_accepted` | 409 | l'invito è già stato accettato |
| `document.archived` | 409 | il documento è già archiviato |
| `ticket.invalid_transition` | 409 | classificazione, assegnazione, avvio, sospensione, chiusura, riapertura o annullamento non ammessi nello stato attuale |
| `ticket.cancelled` | 409 | la segnalazione è annullata e non si modifica più |
| `utility_bill.invalid_transition` | 409 | correzione, validazione, riaddebito o annullamento non ammessi nello stato attuale |
| `charge.invalid_transition` | 409 | annullamento o rinuncia all'incasso non ammessi nello stato attuale, **oppure** un pagamento verso un addebito `void` o `written_off` |
| `payment.invalid_transition` | 409 | conferma o rifiuto non ammessi nello stato attuale: `pending` è l'unico da cui si esce |
| `expense_group.archived` | 409 | il conto è chiuso: non accetta più spese né regolamenti. I saldi restano leggibili |
| `expense_group.membership_unchanged` | 409 | è già uscito, oppure c'è già: l'operazione non avrebbe niente da fare |
| `expense.payer_not_in_group` | 409 | chi ha anticipato non è un membro attivo. Lo schema non lo impone, e senza il controllo quel credito non comparirebbe in nessun saldo |
| `settlement.invalid_transition` | 409 | conferma, rifiuto o ritiro non ammessi nello stato attuale: `pending` è l'unico da cui si esce, e ne ha **tre** uscite |
| `membership.lease_in_force` | 409 | non si esce da un'organizzazione in cui si è parte di un contratto in essere, garanti compresi. Dalla `0039` lo dice anche la RLS: il codice qui è la versione parlante di un rifiuto che la policy darebbe comunque |
| `membership.last_owner` | 409 | l'ultimo proprietario non esce: l'organizzazione resterebbe senza nessuno che la amministri, e non c'è nessuna operazione del prodotto per rientrarci. È l'unico dei tre che **nessuna policy garantisce** |
| `membership.already_left` | 409 | si era già usciti. La riga resta — uscire è uno stato — quindi il caso è osservabile e merita un messaggio invece di un secondo `200` muto |
| `session.refresh_token_reused` | 409 | refresh token già ruotato |

**Validazione e interni**

| `code` | Stato | Condizione |
|---|---|---|
| `validation.failed` | 422 | uno o più campi non validi; il dettaglio è in `errors[]` |
| `lease.derived_status_filter` | 422 | filtro su uno stato derivato: usare `in_force` |
| `charge.derived_status_filter` | 422 | filtro su `status=overdue`: è derivato dalla scadenza, usare `outstanding=true` |
| `internal.error` | 500 | errore non previsto |

Tre scelte deliberate:

- **404 e non 403** quando una risorsa non è visibile: un `403` confermerebbe che
  esiste. Sotto Row Level Security "non esiste" e "non è tua" sono indistinguibili,
  ed è corretto che lo restino anche in risposta.
- **401 identico** per credenziali errate e account inesistente — stesso stato,
  stesso `title`, stesso `code`.
- **`auth.token_expired` distinto** da `auth.token_invalid`, perché il rimedio è
  diverso e la distinzione non rivela nulla: per ottenerlo bisogna già presentare un
  token firmato da noi. Il *refresh* invece resta indistinto.

## Paginazione

Keyset, non OFFSET: con OFFSET il database scarta N righe a ogni pagina, e le righe
inserite nel frattempo fanno saltare o ripetere elementi.

```
GET /api/v1/properties?limit=25&after=<ultimo-id-della-pagina-precedente>
```

```json
{ "items": [ … ], "next_cursor": "019fce39-1001-7a30-ab7b-9ac641487245" }
```

`next_cursor` assente significa fine dei risultati. Gli id sono UUIDv7, quindi
ordinarli equivale a ordinarli per data di creazione. `limit` va da 1 a 100, default
25, e viene **troncato** invece di rifiutato: un client che chiede 10 000 righe
ottiene la pagina massima, non un errore da gestire. Il tetto resta perché un solo
GET non deve poter esaurire memoria e connessione.

## Semantica del PATCH

Un aggiornamento parziale distingue tre casi:

| Corpo | Effetto |
|---|---|
| campo assente | invariato |
| `"campo": null` | **azzerato** |
| `"campo": valore` | impostato |

Senza questa distinzione un PATCH non potrebbe cancellare un valore, e "non
inviato" finirebbe per significare "metti a null". Nel dominio è
`Patch<T> = Option<Option<T>>`; al bordo un deserializzatore dedicato.

Un PATCH che non cambia nulla restituisce `200` con la risorsa, senza produrre
scrittura, evento o riga di audit.

## Endpoint

### Sistema

| Metodo | Percorso | Auth | Note |
|---|---|---|---|
| GET | `/health` | no | Liveness. **Non tocca il database**: se lo facesse, un'indisponibilità di PostgreSQL farebbe riavviare in loop tutti i pod, che è il contrario di quel che serve mentre il database torna su |
| GET | `/ready` | no | Readiness: verifica il database. `503` se non raggiungibile |
| GET | `/docs` | no | Swagger UI |
| GET | `/openapi.json` | no | Specifica |

### Autenticazione

| Metodo | Percorso | Auth | Esito |
|---|---|---|---|
| POST | `/api/v1/auth/register` | no | `201` con `user_id` e `organization_id`. Crea utente e organizzazione personale in una transazione |
| POST | `/api/v1/auth/login` | no | `200` con la sessione, più il **cookie** del refresh token |
| POST | `/api/v1/auth/refresh` | cookie | `200`, ruota il refresh token e riposa il cookie. **Nessun corpo** |
| POST | `/api/v1/auth/logout` | sì, più il cookie | `204`, idempotente. Cancella il cookie |
| POST | `/api/v1/auth/password-reset` | no | `202` **sempre**, senza corpo. Se quell'indirizzo ha un account, riceve un codice |
| POST | `/api/v1/auth/password-reset/confirm` | no | `204`. **Revoca tutte le sessioni.** `401` se il codice non vale, `422` se la password è debole |
| POST | `/api/v1/auth/accept-owner-invitation` | no | `201` con la sessione e il cookie, come il login. Crea proprietario e organizzazione personale dietro un token di piattaforma (ADR-053) |
| POST | `/api/v1/auth/register-from-invitation` | no | `201` con la sessione e il cookie, come il login. Registra un inquilino **dentro l'organizzazione di chi lo ha invitato**, con zero organizzazioni proprie (ADR-053) |

Il **cambio password da autenticati** non è qui: è
[`/api/v1/me/password`](#il-proprio-account), sotto, perché chi lo chiede **è** già
dentro. Le due rotte qui sopra servono a chi **non può entrare**.

#### Il recupero della password

Quattro proprietà, e tre si dimenticano. Vedi [ADR-052](08-decisioni.md#adr-052).

**La risposta è la stessa per un indirizzo noto e per uno sconosciuto**, e non ha un
corpo: `202` a chiunque. Distinguerli sarebbe un oracolo per sapere chi ha un account
([ADR-017](08-decisioni.md#adr-017)). **Anche il tempo è lo stesso**: per un indirizzo
senza account si calcola comunque lo hash di un valore fittizio, altrimenti la latenza
direbbe ciò che la risposta tace. Il client scrive «se quell'indirizzo ha un account,
riceverà un codice», che è la verità e non una scusa.

**Il codice si incolla, non si clicca.** L'email porta un codice e non un link: un token
in un URL finisce nella cronologia, nel `Referer` e nei log di ogni proxy davanti al
client. È la stessa decisione presa per l'invito ([ADR-045](08-decisioni.md#adr-045)).
Vale **trenta minuti** e una volta sola; sconosciuto, scaduto e già usato danno lo stesso
`401`.

**Confermare revoca tutte le sessioni** — ma sono i **refresh token**, non gli access
token, che sono JWT firmati e non revocabili ([ADR-013](08-decisioni.md#adr-013)): chi ne
ha uno in mano resta dentro fino alla scadenza, quindici minuti. La catena si chiude
comunque, e da lì in poi non si rinnova più.

**Il limite agli invii è per email**: un secondo codice chiesto entro due minuti viene
rifiutato **in silenzio**, con lo stesso `202`. Dirlo sarebbe di nuovo l'oracolo — «troppo
presto» conferma che l'account esiste.

**Il cambio password da autenticati è [`/api/v1/me/password`](#il-proprio-account)**,
non qui: questa coppia di rotte serve a chi **non può entrare**, non a chi è già dentro.
Le due condividono la revoca di tutte le sessioni e la validazione della password.

Risposta di sessione, nella forma attesa da un client OAuth2 **meno il refresh
token**:

```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJFZERTQSJ9…",
  "token_type": "Bearer",
  "expires_in": 899,
  "organization_id": "019fce39-0e43-7011-84bf-1d03f6764daf",
  "role": "owner",
  "user_id": "019fce39-0e43-7011-84bf-1cf9fb30d27b"
}
```

#### Il refresh token è un cookie, e solo un cookie

`SessionResponse` **non porta più `refresh_token`**, e le richieste di refresh e di
logout **non lo accettano più nel corpo**. Esce e rientra soltanto qui:

```
Set-Cookie: propflow_refresh=…; Max-Age=2591999; Path=/api/v1/auth; HttpOnly; Secure; SameSite=Strict
```

Metterlo *anche* nel JSON sarebbe metà misura a costo pieno: `HttpOnly` protegge
dal fatto che uno script lo legga da `document.cookie`, e non servirebbe a nulla se
lo stesso valore comparisse nella risposta del login, dove un XSS lo prenderebbe
senza sforzo. Trenta giorni di sessione sono precisamente ciò che non deve essere
leggibile da uno script.

Ne segue la divisione che un client deve rispettare: **l'access token vive in
memoria** e lo manda il client come `Authorization: Bearer`; **il refresh token
vive nel cookie** e lo manda il browser da sé, solo verso `/api/v1/auth`. L'access
token muore al ricaricamento della pagina, e non è un problema: si riottiene dal
cookie.

`Path=/api/v1/auth` non è cosmetico: una richiesta a `/api/v1/properties` non si
porta dietro il cookie nemmeno per sbaglio. `SameSite=Strict` significa che il
browser non lo manda su una richiesta che parte da un altro sito, ed è metà
dell'argomento per cui **questa API non ha né vuole una difesa CSRF** — l'altra
metà è che ogni rotta di business è autenticata dall'header `Authorization`, che
un sito attaccante non può impostare cross-origin senza il permesso di CORS, che
non concediamo perché CORS non è montato affatto. Vedi
[ADR-038](08-decisioni.md#adr-038), che dice anche che cosa farebbe cadere
l'argomento.

`Secure` è **condizionale**: `PROPFLOW__SERVER__SECURE_COOKIES=false` lo toglie per
lo sviluppo su `http://localhost`, dove un browser non manderebbe il cookie e la
sessione durerebbe i quindici minuti dell'access token. Il default è `true`.

**È una rottura del contratto**, non un'aggiunta, e sulla linea 0.x è ammessa. Chi
usa `curl` passa a un cookie jar:

```bash
# Il login scrive il cookie nel jar; il corpo non porta più il refresh token.
curl -sX POST http://localhost:8080/api/v1/auth/login \
  -c cookies.txt -H 'content-type: application/json' \
  -d '{ "email": "mario.rossi@example.it", "password": "cavallo batteria graffetta" }'

# Il refresh non ha corpo: il token lo manda il jar, e ne riceve uno nuovo.
curl -sX POST http://localhost:8080/api/v1/auth/refresh -b cookies.txt -c cookies.txt

# L'uscita revoca la sessione e cancella il cookie.
curl -X POST http://localhost:8080/api/v1/auth/logout \
  -b cookies.txt -c cookies.txt -H "authorization: Bearer $TOKEN"
```

**Cookie assente e cookie rifiutato danno la stessa risposta**, `401` con lo stesso
`code`: distinguerli direbbe a chi ha rubato un token se è ancora vivo, ed è la
ragione per cui il refresh non separa già oggi «scaduto» da «riusato».

Il **logout senza cookie resta `204`**. Non c'è nulla da revocare, il cookie si
cancella comunque, e costringere un client che l'ha già perso a distinguere due
esiti che per lui sono lo stesso non lo aiuterebbe.

#### Registrarsi è entrare su invito, non aprire una porta

[ADR-053](08-decisioni.md#adr-053): `POST /auth/register` **resta com'è per ora**
— sparisce in una fetta successiva — ma le due vie sotto sono quelle su cui il
prodotto atterra. Entrambe rispondono `201` con la stessa `SessionResponse` e lo
stesso cookie del login: chi le chiama è dentro, non deve rifare un secondo
`POST /auth/login`.

**`POST /auth/accept-owner-invitation`** (`token`, `full_name`, `password`,
`organization_name?`): crea un proprietario e la sua organizzazione personale,
esattamente come `register` fa oggi, ma dietro un token emesso da uno script in
`ops/` e non su una rotta aperta a chiunque. Il token è quello che
[07 — Operazioni](07-operazioni.md#emettere-un-invito-da-proprietario) descrive
come emetterlo e consegnarlo.

**`POST /auth/register-from-invitation`** (`token`, `full_name`, `password`):
registra un inquilino **a partire dall'invito** che oggi vive in
`/auth/register` + `/auth/accept-invitation` in due passi — e li fa insieme, in
una transazione: non esiste un istante in cui l'inquilino è registrato e senza
organizzazione. Chi si registra così entra **nell'organizzazione di chi lo ha
invitato**, con zero organizzazioni proprie: non passa da `RegisterOwner`, e
quindi non erediterebbe mai un'organizzazione personale vuota. **Nessun campo
`email`**: è quella dell'invito, e si riconosce e si rifiuta con `422` se il
corpo la porta, con la stessa forma di `email` in `PATCH /me` — un campo
sconosciuto si ignora ovunque nell'API, ma questo non lo è: è quello che
chiunque proverà a mandare per primo.

**Le quattro cause di rifiuto rispondono tutte `401 auth.invalid_credentials`**,
identico in ogni caso, su entrambe le rotte: token inesistente, scaduto, usato
o revocato. Distinguerle sarebbe un oracolo su quali token sono stati emessi —
la stessa ragione di [ADR-017](08-decisioni.md#adr-017) per l'invito
all'inquilino e per il codice di recupero. **Un'eccezione sola**, e solo su
`register-from-invitation`: un token che corrisponde a un invito da
**proprietario** pendente e valido dà `409 invitation.owner_token` invece di
`401`, perché il client deve chiedere il nome dell'organizzazione e rimandare
lo stesso corpo a `accept-owner-invitation`. Resta vero che un token che non
esiste, in **nessuna** delle due tabelle, o che non è più valido in
**entrambe**, dà sempre `401`: il `409` esce solo a chi possiede davvero un
token da proprietario, quindi non è un secondo oracolo.

Rate limit: **nessuno**, oggi, né su queste due rotte né su `login` — è una
premessa del piano che l'esecuzione ha trovato imprecisa, non un debito
introdotto qui.

### Organizzazioni

| Metodo | Percorso | Ruolo minimo | Esito |
|---|---|---|---|
| GET | `/api/v1/organizations` | **autenticato** | `200` con le organizzazioni di cui si è membro attivo, ognuna con `id`, `name`, `slug` e il proprio `role` |
| POST | `/api/v1/organizations/{id}/leave` | **autenticato** | `200` con `organization_id` e `role`. Esce dall'organizzazione: `status` passa a `left`, **la riga resta**. `404 membership.not_found`, `409 membership.lease_in_force` / `membership.last_owner` / `membership.already_left` |

```json
{
  "items": [
    { "id": "019fce39-0e43-7011-84bf-1d03f6764daf", "name": "Immobili di Mario Rossi",
      "slug": "immobili-di-mario-rossi-4f21ab", "role": "owner" },
    { "id": "019fce41-8a02-7c19-9d33-2b7cc9107ef1", "name": "Case Bianchi",
      "slug": "case-bianchi-91cd02", "role": "tenant" }
  ]
}
```

**È l'unica rotta che risponde senza un'organizzazione attiva**, e non è una
deroga: «quali organizzazioni sono mie» non appartiene a nessuna di esse, e
pretendere un `X-Org-Id` per poterlo domandare sarebbe circolare — romperebbe
proprio il caso che l'endpoint serve, cioè chi non ne ha una selezionata. Il
service layer usa quindi `ensure_authenticated` e non `ensure_member`.

Funziona perché la Row Level Security lo permette: `app.member_role(p_org_id)`
filtra su `app.current_user_id()` e sull'organizzazione **passata come argomento**,
e `app.current_org_id()` non vi compare; `organizations_select` è
`app.is_org_member(id)`. Mandare comunque `X-Org-Id` non cambia la risposta.

**Il `name` è il punto della rotta.** Nessun altro endpoint restituisce il nome di
un'organizzazione a chi non ne è staff, quindi senza questo un client può mostrare
solo l'identificatore: per un proprietario, che ne ha una, è indistinguibile da un
elenco vero; per un inquilino sono due UUID fra cui indovinare.

**Nessun cursore.** Le organizzazioni di una persona sono poche per costruzione — la
propria, più quelle di chi le affitta casa — e una paginazione su un elenco che sta
in una riga è cerimonia. È la stessa scelta di
[`/properties/{id}/residents`](#chi-abita-la-casa).

**Senza prefisso `me`**: la Row Level Security restringe già alle proprie, e il
percorso dice il vero senza aggiungere una parola.

Compaiono le sole appartenenze **attive**. Non è prudenza: `app.member_role` esige
`status = 'active'`, quindi un'appartenenza sospesa o lasciata comparirebbe nel
menù e poi non concederebbe una sola riga — una schermata vuota senza errore, che è
il modo peggiore di sbagliare.

#### Uscire da un'organizzazione

`POST /api/v1/organizations/{id}/leave`. Non prende corpo, e **prende un
identificatore** al contrario di `/api/v1/me`: non si agisce su di sé, si agisce su
un **rapporto**, e quale sia va detto. Non dev'essere quella attiva — chi ne ha due
può uscire da quella che non sta guardando — e per questo la rotta non pretende
`X-Org-Id`.

```json
{ "organization_id": "019fce41-8a02-7c19-9d33-2b7cc9107ef1", "role": "tenant" }
```

**Uscire è uno stato, non una cancellazione.** `app.org_members.status` passa a
`left` e la riga **resta**. Con `deleted_at` sarebbe sparita da
`OrgMemberRepository::find`, e `org_members_org_user_uk` è unico e **non parziale**:
chi fosse uscito così non potrebbe più rientrare né con un `INSERT`, che sbatte sul
vincolo, né con un `UPDATE` su una riga che non si trova. È il difetto che la
[`0029`](03-modello-dati.md) ha già pagato sulle spese co-living.

**Chi è uscito rientra dalla stessa porta**: accettando un invito nuovo.
`AcceptInvitation` riattiva una membership non attiva invece di inserirne una — è il
ramo che [ADR-051](08-decisioni.md#adr-051) aveva previsto per iscritto lasciandolo
non scritto, *«perché finché non si esce non serve a nessuno»*. Il **ruolo non si
tocca**: chi era `owner` ed è uscito rientra `owner`, e il privilegio di colonna
della `0039` lo garantisce comunque, perché `role` non è scrivibile dal pool
applicativo.

I tre rifiuti, e **chi li garantisce davvero**:

| Codice | Stato | Garantito da |
|---|---|---|
| `membership.not_found` | `404` | la Row Level Security: la riga non torna |
| `membership.lease_in_force` | `409` | la `WITH CHECK` di `org_members_update_self`, che interroga `app.has_lease_in_force` |
| `membership.last_owner` | `409` | il **service layer**, e niente altro |
| `membership.already_left` | `409` | il dominio: `OrgMember::leave` su una riga già `left` |

I primi due sono la versione **parlante** di un rifiuto che il database darebbe
comunque: senza la lettura anticipata, il secondo risponderebbe
`auth.denied_by_policy`, che nella tassonomia di [ADR-022](08-decisioni.md#adr-022)
significa «difetto nostro» e manderebbe a cercare un guasto invece di una disdetta.
Il caso d'uso e la policy interrogano **la stessa funzione**, perché due definizioni
di «in essere» divergerebbero e il sintomo sarebbe una schermata che dice di sì e
un'`UPDATE` che dice di no.

Il terzo no, ed è l'unico che vive per intero nel service layer: lo schema dichiara
«almeno un owner per org» come invariante applicativa, perché un trigger cross-row
sarebbe workflow nel database. Senza, uscire dalla propria organizzazione personale
la lascerebbe **senza nessuno che la amministri** — e `organizations_select` è
`is_org_member(id)`, quindi immobili, contratti e addebiti resterebbero in una riga
che non è più visibile a nessuno, senza nessuna operazione del prodotto per
rientrarci.

**Il divieto vale anche per lo staff**, ed è una riga in più nella migration: le
policy permissive si combinano in **OR**, quindi la `WITH CHECK` di
`org_members_update` — `is_org_staff(org_id)` — avrebbe lasciato passare l'uscita di
un proprietario con un contratto in essere, cioè lo stesso caso dalla porta accanto.
La `0039` la emenda con lo stesso ramo.

**Chi esce dall'unica organizzazione in cui abita resta con la propria**, personale
e vuota: è il caso di [ADR-045](08-decisioni.md#adr-045) al contrario. Il client lo
dice **prima**, non lo lascia scoprire dopo con un riepilogo vuoto e nessuna
spiegazione.

**Togliere qualcun altro dalla propria organizzazione non esiste**, ed è il gemello
di questa rotta e non questa rotta: vedi [12](12-da-fare.md).

<a id="il-proprio-account"></a>

### Il proprio account

| Metodo | Percorso | Ruolo minimo | Esito |
|---|---|---|---|
| GET | `/api/v1/me` | **autenticato** | `200` con identificatore, email, nome, telefono, lingua e la data di verifica dell'email |
| PATCH | `/api/v1/me` | **autenticato** | `200` con i dati aggiornati. Corpo vuoto: `200` invariato, nessuna scrittura |
| POST | `/api/v1/me/password` | **autenticato** | `204`. Pretende la password corrente e **revoca tutte le sessioni** |

```json
{
  "id": "019fce2b-77aa-7a1e-9d7c-2c0a5e1b44c1",
  "email": "mario.rossi@example.it",
  "full_name": "Mario Rossi",
  "phone": "+39 333 1234567",
  "locale": "it-IT",
  "created_at": "2026-01-14T09:12:00Z"
}
```

**Il percorso non prende mai un identificatore di utente**, ed è la forma e non una
comodità: è ciò che tiene chiusa la domanda di
[ADR-036](08-decisioni.md#adr-036) — nessuna rotta proietta un utente per
identificatore. Un `PATCH /users/{id}` che accettasse **solo** il proprio sarebbe la
stessa cosa con una guardia applicativa davanti, cioè la porta aperta e un cartello:
la prima persona che rimuovesse quella guardia «uniformando» le rotte non
troverebbe niente che glielo impedisca. Chi si legge viene dal token.

**Risponde senza `X-Org-Id`**, come `/organizations`, e per la stessa ragione: i
propri dati non appartengono a nessuna organizzazione. Pretenderne una attiva
romperebbe proprio il caso in cui la rotta serve di più — chi è appena uscito
dall'unica in cui abitava. Regge perché `users_select` ammette
`id = app.current_user_id()` come primo ramo, e quel ramo non guarda
`app.current_org_id()`.

**Non escono `platform_role`, `failed_login_count` e `locked_until`.** Il ruolo di
piattaforma è una proprietà del sistema e non dell'account, e i due contatori sono
una difesa: mostrarli direbbe a chi ha rubato una scheda quanto gli manca per
bloccare fuori il proprietario.

#### Che cosa si corregge, e che cosa no

`full_name`, `phone`, `locale`. Semantica del `PATCH` di sempre: campo omesso
significa invariato, `null` esplicito cancella — e `phone` è l'unico dei tre che si
può azzerare.

**L'email si legge e non si modifica**, e questa è la decisione che sembra una
mancanza e non lo è. `users.email` non è un dato anagrafico: è **la credenziale
d'accesso**, ed è la chiave con cui tre cose si ritrovano.

- `app.has_pending_invitation` fa `JOIN app.users u ON u.email = i.email`: cambiare
  l'email **rompe in silenzio** ogni invito pendente per quell'indirizzo;
- `app.current_user_email()` è ciò su cui poggiano quattro policy della `0018`;
- `lease_parties.email` è come l'accettazione trova la propria riga.

Cambiarla bene richiede un giro di verifica sul nuovo indirizzo — cioè la verifica
dell'email, che è deliberatamente fuori perimetro — e una decisione su che cosa
succede agli inviti pendenti. Farla male significa un account che non riceve più
niente e un invito che non si può più accettare, con un sintomo muto in entrambi i
casi.

**`email` nel corpo si riconosce e si rifiuta con `422`**, invece di essere ignorata
come ogni altro campo sconosciuto. È l'eccezione alla tolleranza dell'API, e la
ragione è che non è un campo sconosciuto: è quello che chiunque proverà a mandare
per primo. Ignorarla darebbe `200` con l'indirizzo di prima — cioè esattamente il
sintomo muto che questa scelta esiste per non produrre.

Il telefono e la lingua sono **validati nel dominio** e non solo nello schema.
`app.users.phone` ha un `CHECK` con una forma precisa, e senza la stessa regola nel
dominio un numero malformato morirebbe sul vincolo, cioè con un `409
conflict.constraint_violated` dove il chiamante merita un `422` che nomina il campo.
`locale` dev'essere un'etichetta BCP 47 plausibile per una ragione concreta e non
formale: il client formatta date e importi con `Intl` passando quel valore, e
`Intl.DateTimeFormat` **solleva** su un'etichetta malformata — un `locale` di
spazzatura accettato qui diventa una schermata che non si apre più.

#### Cambiare la password da dentro

`POST /api/v1/me/password`, con `current_password` e `new_password`.

**La vecchia serve**, anche se chi chiama è già autenticato, perché il contesto è
diverso da quello del recupero: là la prova d'identità è il controllo della casella
di posta, qui è **una sessione** — e una sessione può essere una scheda lasciata
aperta. Senza la vecchia, chiunque si sieda davanti a un browser non bloccato si
prende l'account per sempre.

**Revoca tutte le sessioni, compresa la propria**, esattamente come
[ADR-052](08-decisioni.md#adr-052) fa per il recupero. La scelta è l'**uniformità**,
ed è stata presa contro l'alternativa che sembra più gentile — tenere viva la
sessione da cui si sta cambiando, riconoscendola dal cookie di refresh che la
richiesta porta già. Il motivo: *«cambiare la password chiude tutto»* è **una frase
sola**, e va scritta in quattro posti — la schermata del recupero, quella del
proprio account, [04](04-sicurezza.md) e questo documento. Due regole diverse per
due percorsi che una persona non distingue sono due frasi da tenere allineate, e la
seconda diverge alla prima modifica. L'attrito che si paga è piccolo e visibile:
chi cambia rientra subito con la password che ha appena scelto.

**E «tutte le sessioni» sono i refresh token, non gli access token.** L'access token
è un JWT firmato e non è revocabile ([ADR-013](08-decisioni.md#adr-013)): chi ne ha
uno in mano resta dentro fino alla scadenza, quindici minuti. Ciò che muore
all'istante è il **rinnovo**. È la nota di ADR-052 che si dimentica di continuo, ed
è la stessa su cui un test della 5-quater aveva asserito la cosa sbagliata.

**La password corrente errata dà `401` e non scrive niente — nemmeno il contatore
dei tentativi.** Il blocco dopo dieci tentativi difende il *login*, dove chi prova
non è autenticato; qui chi prova ha già una sessione, e incrementarlo darebbe a
chiunque rubi una scheda il modo di bloccare fuori il legittimo proprietario. La
`0038` non concede comunque quella colonna al pool applicativo.

**Il `204` non ripulisce il cookie di refresh**, ed è una scelta: il cookie è
limitato a `Path=/api/v1/auth`, quindi questa rotta non lo raggiunge nemmeno; e il
refresh che porta è ormai revocato, quindi il prossimo rinnovo fallisce e il client
torna all'accesso da sé.

#### Che cosa scrive, e con quale privilegio

Gira sul pool **applicativo** — chi cambia la propria password è autenticato, quindi
la regola 27 non ammette il privilegiato — e questo è il motivo per cui la
[`0038`](03-modello-dati.md) esiste. Quella migration revoca al ruolo applicativo
l'`UPDATE` sull'intera `app.users` e lo riconcede su quattro colonne sole:
`full_name`, `phone`, `locale`, `password_hash`. Tutto il resto — `platform_role`,
`status`, `email`, i due contatori del blocco — lo scrive il pool privilegiato, dove
vivono login e recupero password.

**`/api/v1/me` è la prima rotta che scrive `app.users` dal pool applicativo**, ed è
la ragione per cui quella migration entra in questa fetta e non in una sua: finché
non esisteva, la porta era chiusa da un privilegio che nessuno usava.

### Dashboard

| Metodo | Percorso | Ruolo minimo | Esito |
|---|---|---|---|
| GET | `/api/v1/dashboard` | **staff** | `200` con i nove numeri dell'organizzazione attiva |

Non prende parametri: l'organizzazione è quella attiva — quella del token, o quella
dichiarata con `X-Org-Id`. Metterla nel percorso inviterebbe a chiederne una di cui
non si è membri, e la risposta a quella domanda sarebbe comunque `403`.

```json
{
  "organization_id": "019fce39-0e43-7011-84bf-1d03f6764daf",
  "properties_count": 3,
  "active_leases": 2,
  "monthly_rent_expected": "1450.00",
  "total_outstanding": "184.20",
  "overdue_charges": 1,
  "open_tickets": 2,
  "leases_expiring_90d": 1,
  "bills_pending_review": 4,
  "written_off_total": "500.00"
}
```

I tre importi sono **stringhe decimali** come ovunque; i sei conteggi sono interi,
perché contare non ha decimali.

**È dello staff e non del membro.** La Row Level Security da sola non basterebbe:
`app.v_owner_dashboard` è `security_invoker` e passa da `organizations_select`, che
è `is_org_member` e non `is_org_staff` — un inquilino otterrebbe la riga con dentro
i numeri dell'intera organizzazione. È uno dei pochi casi in cui il controllo
applicativo non duplica la policy ma la **stringe**.

#### Che cosa questi numeri contano davvero

Sono quelli della vista, non un calcolo dell'API, e alcuni dicono meno di quanto il
nome prometta:

- **`monthly_rent_expected` è un'attesa dichiarata, non un incasso previsto.** Somma
  `rent_amount` dei contratti con `status = 'active'`, e lo stato memorizzato resta
  `active` anche dopo la scadenza perché nessuno scheduler lo riscrive
  ([ADR-019](08-decisioni.md#adr-019)). `leases_expiring_90d` sta accanto per
  questo: è quanto di quell'attesa sta per non esserci più;
- **`total_outstanding` esclude `void` e `written_off`** e somma i soli residui
  **positivi** (migration `0024`, [ADR-032](08-decisioni.md#adr-032));
- **`written_off_total` è il residuo degli addebiti stornati, non il loro importo.**
  Un addebito da 800 con 300 incassati e poi stornato vale **500**: gli altri
  trecento sono entrati e non si sono persi. Dalla migration `0032`,
  [ADR-040](08-decisioni.md#adr-040).

  Sta accanto a `total_outstanding` e **non lo compensa**: quello dice quanto è
  ancora dovuto, questo quanto non lo sarà mai. Un addebito **annullato** non compare
  qui — `void` dice «non doveva essere chiesto», quindi non è una perdita — ed è la
  distinzione per cui il numero esiste.

  Non si calcola da `outstanding_amount`: la `0024` lo azzera proprio su
  `written_off`, quindi è la stessa colonna che rende il numero necessario e che non
  può produrlo;
- **`bills_pending_review` conta `draft` *e* `pending_review`**, che sono due
  popolazioni diverse: una bolletta registrata e senza scansione, e una che la
  macchina ha letto e che aspetta una persona. Le conta insieme dalla `0015`;
- **`open_tickets`** sono i sei stati non terminali, gli stessi dell'indice parziale
  `tickets_open_ix` e dello stesso filtro `?open=true`.

#### Due cose che non dice, e non per dimenticanza

- **`written_off` non è distinto da `void`.** Entrambi escono dall'insoluto, ma solo
  il primo è una perdita, e la distinzione conta in un report. Aggiungere un nono
  numero è additivo; cambiare che cosa significa `total_outstanding` non lo è;
- **il credito da sovrapagamento sparisce.** Un `outstanding_amount` negativo non
  compensa gli altri insoluti — probabilmente giusto — e non compare da nessuna
  parte, che è il numero che qualcuno cercherà.

Nessuna migration: la vista esiste dalla `0015`.

### Immobili

| Metodo | Percorso | Ruolo minimo | Esito |
|---|---|---|---|
| POST | `/api/v1/properties` | staff | `201`, stato iniziale `draft` |
| GET | `/api/v1/properties` | membro | `200`, pagina keyset. Filtri `status`, `city` |
| GET | `/api/v1/properties/{id}` | membro | `200` o `404` |
| PATCH | `/api/v1/properties/{id}` | staff | `200`. `409` se archiviato |
| DELETE | `/api/v1/properties/{id}` | staff | `204`. Archiviazione, non cancellazione |
| GET | `/api/v1/properties/{id}/residents` | membro | `200` con `items`, `404` se l'immobile non è visibile |

L'inquilino vede gli immobili in cui abita, non gli altri della stessa
organizzazione. Non può crearli né modificarli: `403`.

Lo stato non si porta ad `archived` con un PATCH — l'archiviazione ha effetti propri
(soft delete, evento dedicato) e ha il suo verbo. Il tentativo dà `409`.

#### Chi abita la casa

`GET /properties/{id}/residents` risponde `{ "items": [ … ] }`, senza paginazione: i
residenti di un immobile sono pochi per costruzione, come le stanze.

```json
{ "items": [
  { "user_id": "0192...", "full_name": "Giulia Neri",  "room_name": "Camera doppia" },
  {                       "full_name": "Marco Bianchi", "room_name": "Camera singola" }
] }
```

**È l'elenco da cui un client prende gli `user_id`** del modulo delle spese co-living,
ed è la ragione per cui esiste: vedi [`user_id`: come un client li trova](#user_id-come-un-client-li-trova).

Chi ci **abita**, non chi ha firmato:

- il **proprietario non compare**. Vede l'elenco perché è staff dell'organizzazione, e
  non ne fa parte: chi si aspetta di trovarlo sta cercando un elenco diverso;
- il **garante non compare**: non abita lì;
- chi aveva un contratto **scaduto** non compare. Il filtro è `app.lease_in_force`, la
  stessa definizione di "in essere" del resto del progetto ([ADR-019](08-decisioni.md#adr-019)):
  lo stato memorizzato resta `active` anche dopo la scadenza, perché nessuno scheduler
  lo aggiorna.

**`user_id` è assente per chi è stato invitato e non ha ancora accettato.** La persona
ha un nome e nessun identificatore, quindi non è ancora aggiungibile a un gruppo di
spesa. Compare comunque, ed è deliberato: ometterla lascerebbe chi guarda senza una
spiegazione per un coinquilino che sa di avere in casa e non trova nell'elenco.

`room_name` è assente sui contratti sull'immobile intero, che non hanno una stanza. Chi
affitta due stanze dello stesso immobile compare **due volte**, con la stanza a
distinguerle.

**Non c'è l'email, e non è un compromesso.** La risposta viene da una funzione che
proietta tre colonne, non da una riga resa leggibile: non c'è nulla da cui ometterla.
Con una policy allargata la riga sarebbe uscita intera — e da `app.leases` sarebbe
uscito il **canone dei coinquilini**. Perché la differenza conta:
[ADR-036](08-decisioni.md#adr-036).

Un `id` non visibile dà `404` e non un elenco vuoto: un elenco vuoto è uno stato
legittimo (un immobile sfitto), e restituirlo confermerebbe che l'immobile esiste.

### Stanze

| Metodo | Percorso | Ruolo minimo | Esito |
|---|---|---|---|
| POST | `/api/v1/properties/{property_id}/rooms` | staff | `201`, stato iniziale `available` |
| GET | `/api/v1/properties/{property_id}/rooms` | membro | `200` con `items`, `404` se l'immobile non è visibile |
| PATCH | `/api/v1/rooms/{id}` | staff | `200`. `409` se archiviata o se il nome è già usato |
| DELETE | `/api/v1/rooms/{id}` | staff | `204`. Archiviazione |

Creazione ed elenco sono annidati sotto l'immobile perché una stanza non ha vita
propria; modifica e archiviazione vanno per id, perché il percorso dell'immobile non
aggiungerebbe nulla e obbligherebbe il client a portarselo dietro.

Un `property_id` non visibile dà `404` e non `403`, anche in scrittura: senza il
controllo la richiesta arriverebbe al trigger di coerenza o alle policy di riga, e il
chiamante vedrebbe un `409` o un `403` dove la risposta corretta è "non esiste".

`monthly_rent` è una stringa decimale (`"450.00"`) come tutti gli importi. Il nome è
unico dentro l'immobile fra le stanze non archiviate: archiviarne una libera il nome.

L'elenco delle stanze non è paginato — sono poche per costruzione — e la risposta ha
la forma `{ "items": [ … ] }` senza `next_cursor`.

### Contratti

| Metodo | Percorso | Ruolo minimo | Esito |
|---|---|---|---|
| POST | `/api/v1/leases` | staff | `201`, stato iniziale `draft` |
| GET | `/api/v1/leases` | membro | `200`, pagina keyset. Filtri `property_id`, `status`, `in_force` |
| GET | `/api/v1/leases/{id}` | membro | `200` con le parti contrattuali, o `404` |
| POST | `/api/v1/leases/{id}/parties` | staff | `201`. `409` se esiste già un intestatario principale |
| POST | `/api/v1/leases/{id}/activate` | staff | `200`. `409` su periodo occupato o parti incomplete |
| POST | `/api/v1/leases/{id}/terminate` | staff | `200`. `409` se il contratto non è attivo |

L'inquilino vede il **proprio** contratto con `GET`, non gli altri della stessa
organizzazione: la distinzione la fa `app.can_access_lease` attraverso
`lease_parties`, non un ramo applicativo sul ruolo.

Attivazione e disdetta sono sotto-risorse e non un `PATCH` sullo stato: hanno
effetti propri — firma, occupazione del periodo, data di fine rapporto — come già
l'archiviazione degli immobili.

Il tipo di contratto **non si dichiara**: si deduce dalla presenza di `room_id`.
Dichiararlo a parte sarebbe soltanto un modo di contraddirlo, e il `CHECK`
`leases_target_ck` lo rifiuterebbe.

#### `property_label`, `room_name`, `primary_tenant`: come si chiama un contratto

I tre campi che rendono un contratto riconoscibile in un elenco. Un contratto non ha un
nome proprio in database, e senza di loro l'elenco mostra tipo e date: venticinque righe
identiche.

| Campo | Assente quando |
|---|---|
| `property_label` | l'immobile è **archiviato** — la riga non si legge più, il contratto resta |
| `room_name` | il contratto è sull'**immobile intero**, che è il caso normale |
| `primary_tenant` | non c'è ancora una parte marcata `is_primary` — una bozza vuota |

Vengono da un **modello di lettura** e non dall'aggregato: il nome di un immobile
appartiene all'immobile, e un contratto che se lo portasse dietro avrebbe due fonti di
verità che divergono alla prima correzione. Una query per pagina, con `LEFT JOIN` e
`deleted_at IS NULL` sulle righe collegate.

`primary_tenant` completa gli altri due dove non bastano: due contratti sulla stessa
stanza in due anni diversi hanno lo stesso immobile e la stessa stanza, e a distinguerli
è chi ci abita. È il `full_name` denormalizzato su `lease_parties`, che sopravvive a chi
se ne va ([ADR-043](08-decisioni.md#adr-043)).

**Assenti sulle risposte a una singola operazione** — attivazione, disdetta,
annullamento — dove chi chiama ha già il contratto sotto gli occhi.

#### `status` ed `effective_status`

La risposta porta due stati:

- `status` è quello **memorizzato**, e cambia solo su azioni esplicite;
- `effective_status` è quello **alla data odierna**, e può valere `expiring` o
  `expired` mentre `status` resta `active`.

Il client non ricalcola le date, e nessuno scheduler riscrive le righe: vedi
[ADR-019](08-decisioni.md#adr-019). Per la stessa ragione `?status=expiring` e
`?status=expired` danno `422` con l'indicazione di usare `?in_force=false`: quei
valori non esistono in database, e rispondere con una pagina vuota sarebbe il modo
peggiore di dirlo.

`end_date` è **esclusiva**: un contratto valido fino al 31 dicembre 2026 si registra
con `end_date` al 1° gennaio 2027.

#### Periodo occupato

Il `409` sulla sovrapposizione arriva dal vincolo di esclusione del database — una
verifica applicativa sarebbe soggetta a race condition — ma il nome del vincolo
resta nei log:

```json
{
  "type": "https://propflow.dev/problems/conflict",
  "title": "Periodo non disponibile",
  "status": 409,
  "detail": "esiste già una locazione attiva per questa stanza nel periodo indicato"
}
```

Immobile e stanza sono distinti nel messaggio: in un co-living "l'immobile è
occupato" sarebbe falso, perché le altre stanze restano libere.

### Documenti

| Metodo | Percorso | Ruolo minimo | Esito |
|---|---|---|---|
| POST | `/api/v1/properties/{property_id}/documents` | membro | `201` con i metadati. `multipart/form-data` |
| POST | `/api/v1/rooms/{room_id}/documents` | membro | come sopra |
| POST | `/api/v1/leases/{lease_id}/documents` | membro | come sopra |
| POST | `/api/v1/tickets/{ticket_id}/documents` | membro | come sopra |
| POST | `/api/v1/utility-bills/{utility_bill_id}/documents` | membro | come sopra. La scansione della bolletta |
| POST | `/api/v1/charges/{charge_id}/documents` | membro | come sopra |
| GET | `/api/v1/properties/{property_id}/documents` | membro | `200`, pagina keyset |
| GET | `/api/v1/rooms/{room_id}/documents` | membro | come sopra |
| GET | `/api/v1/leases/{lease_id}/documents` | membro | come sopra |
| GET | `/api/v1/tickets/{ticket_id}/documents` | membro | come sopra |
| GET | `/api/v1/utility-bills/{utility_bill_id}/documents` | membro | come sopra |
| GET | `/api/v1/charges/{charge_id}/documents` | membro | come sopra |
| GET | `/api/v1/documents/{id}/content` | membro | `307` verso l'URL pre-firmato, in `Location` |
| DELETE | `/api/v1/documents/{id}` | membro, e staff **o** chi lo ha caricato | `204`. Archiviazione |

Elenco e caricamento sono annidati sotto l'oggetto a cui il documento è allegato,
perché un documento non ha vita propria; download e archiviazione vanno per id,
perché il percorso dell'owner non aggiungerebbe nulla. È la stessa scelta delle
stanze.

**Ruolo minimo `membro` e non `staff`.** L'inquilino allega al **proprio** contratto
— la carta d'identità, una ricevuta — e vede i documenti del proprio immobile. Quali
oggetti siano suoi lo decide `app.can_access_attachment` attraverso le policy, non un
ramo applicativo sul ruolo. Un owner non visibile dà `404`, non `403`: sotto Row Level
Security «non è tuo» e «non esiste» sono indistinguibili.

**Archiviare** richiede il ruolo di staff **oppure** essere chi ha caricato il
documento — la stessa condizione della policy `documents_update`. A chi non è né l'uno
né l'altro risponde `403 org.staff_role_required`.

#### Il caricamento passa dall'API, il download no

L'asimmetria è deliberata. In caricamento i byte devono passare da noi almeno una
volta, perché `checksum_sha256` è `NOT NULL` e un checksum dichiarato dal client
certifica solo che il client sa calcolare uno SHA-256. In download non c'è nulla da
verificare, e un URL pre-firmato toglie dall'API il traffico più pesante.

Il corpo del caricamento è `multipart/form-data` con due campi:

| Campo | Obbligatorio | Note |
|---|---|---|
| `file` | sì | Nome e tipo MIME si leggono dalle **intestazioni della parte**, non da campi separati: sono ciò che il browser manda da sé, e chiederli due volte vorrebbe dire poterli ricevere in disaccordo. Senza `Content-Type` sulla parte vale `application/octet-stream` |
| `kind` | no | Classificazione. Assente vale `other`; un valore sconosciuto dà `422` invece di ripiegare in silenzio |

Il limite del corpo su queste rotte è `PROPFLOW__SERVER__UPLOAD_BODY_LIMIT_BYTES`
(25 MiB), separato dal limite generale di 1 MiB che vale su tutto il resto. Oltre il
limite la risposta è `413` con `request.payload_too_large`. Vedi
[ADR-025](08-decisioni.md#adr-025) per il motivo per cui i limiti sono due.

`GET /api/v1/documents/{id}/content` risponde `307` e non `302`: preserva il metodo e
dichiara la redirezione temporanea. La risposta porta `Cache-Control: no-store,
private`, perché l'URL firmato è un **permesso trasferibile** e a tempo — chi lo
intercetta lo usa fino alla scadenza senza presentare un token. Il TTL è
`PROPFLOW__STORAGE__PRESIGN_TTL_SECONDS`, 5 minuti di default.

```bash
curl -X POST http://localhost:8080/api/v1/leases/$LEASE/documents \
  -H "authorization: Bearer $TOKEN" \
  -F "kind=lease_contract" \
  -F "file=@contratto.pdf;type=application/pdf"

# Il download segue la redirezione fino allo storage.
curl -L http://localhost:8080/api/v1/documents/$DOC/content \
  -H "authorization: Bearer $TOKEN" -o contratto.pdf
```

#### Cosa la risposta non dice, e perché

`checksum_sha256` è calcolato dall'API sui byte ricevuti. `is_encrypted` riporta la
cifratura a riposo **confermata dal fornitore di storage**, non richiesta: su un
bucket senza cifratura predefinita vale `false`, ed è la verità
([ADR-024](08-decisioni.md#adr-024)).

Non compaiono `status`, `version` né `extracted_data`: il versionamento è fuori
perimetro, e l'OCR **non scrive sul documento** — la proposta di una macchina finisce
sulla bolletta (`ocr_extracted`) e la traccia di chi l'ha prodotta in `app.ai_jobs`.
Le colonne esistono nello schema e restano ai loro default: vedere una colonna non è
una ragione per riempirla.

**Un caricamento su una bolletta in bozza fa però una cosa in più**, ed è invisibile
nella risposta: accoda la lettura automatica. Vedi
[Lettura automatica](#lettura-automatica-delle-bollette).

**L'archiviazione è soft, e l'oggetto nel bucket sopravvive alla riga.** Il ruolo
applicativo non ha il privilegio `DELETE`, e rimuovere i byte è un lavoro separato, da
pensare insieme al purge di `retention_until`. È registrato fra i debiti noti in
[09 — Stato e roadmap](09-stato-e-roadmap.md#debiti-tecnici-noti). Non esiste un
ripristino: la policy `documents_update` ha `deleted_at IS NULL` nella `USING`, quindi
un update su una riga archiviata aggiorna zero righe senza dare errore, e ripristinare
richiederebbe una migration sulla policy.

Gli owner esposti sono **sei** su undici: immobile, stanza, contratto, segnalazione,
bolletta e addebito. Ogni modulo ne ha portati i propri, e il costo è stato ogni volta
una variante d'enum e due rotte, senza toccare gli use case dei documenti — è il
dividendo di aver fatto i documenti per primi. Allargare agli owner rimasti — avatar,
spese, pagamenti — è altrettanto additivo, e si fa quando quei moduli esistono.

**Con una sola eccezione, che vale la pena ricordare.** Il ramo `utility_bill` di
`app.can_access_attachment` esisteva dal primo giorno ed era **più largo** della policy
della bolletta: risolveva su `can_access_property`, vera per *ogni* inquilino
dell'immobile, mentre `utility_bills_select` chiede di essere parte del contratto
indicato. In un co-living con contratti per stanza, il coinquilino avrebbe scaricato la
scansione della bolletta imputata all'altro senza poterne vedere la riga. La migration
`0022` riallinea la funzione alla policy, ed è arrivata **prima** delle rotte che
aprivano quella porta ([ADR-031](08-decisioni.md#adr-031)).

`users.avatar_document_id`, `leases.contract_document_id`, `utility_bills.document_id`
e `charges.document_id` restano **non popolati dall'API**: il documento si allega con
il riferimento polimorfico, e popolare anche la colonna darebbe due risposte alla
domanda «qual è il documento di questa riga».

### Segnalazioni

| Metodo | Percorso | Ruolo minimo | Esito |
|---|---|---|---|
| POST | `/api/v1/properties/{property_id}/tickets` | membro | `201`, stato iniziale `open`. `422` se stanza o contratto sono di un altro immobile |
| GET | `/api/v1/tickets` | membro | `200`, pagina keyset. Filtri `property_id`, `status`, `open` |
| GET | `/api/v1/tickets/{id}` | membro | `200` con lo storico degli stati, o `404` |
| PATCH | `/api/v1/tickets/{id}` | staff | `200`. Solo i costi. `409` se annullata |
| PATCH | `/api/v1/tickets/{id}/report` | staff, o chi ha segnalato finché è `open` | `200`. Titolo, descrizione, `visible_to_residents`. `409` se annullata |
| POST | `/api/v1/tickets/{id}/messages` | membro | `201`. `403` a un inquilino che chiede `is_internal`. **`409 ticket.thread_closed` se la segnalazione non è più in lavorazione** |
| GET | `/api/v1/tickets/{id}/messages` | membro | `200`, pagina keyset. Le note interne non compaiono all'inquilino |
| POST | `/api/v1/tickets/{id}/triage` | staff | `200`. Imposta categoria e priorità |
| POST | `/api/v1/tickets/{id}/assign` | staff | `200`. Corpo: `assignee_id` |
| POST | `/api/v1/tickets/{id}/start` | staff | `200`. Avvia la lavorazione, o la riprende |
| POST | `/api/v1/tickets/{id}/wait` | staff | `200`. Corpo: `waiting_on` (`tenant` o `owner`) |
| POST | `/api/v1/tickets/{id}/close` | staff | `200`. Valorizza `closed_at` e, se inviato, `resolution_notes` |
| POST | `/api/v1/tickets/{id}/reopen` | **chi ha segnalato**, o staff | `200`. Corpo: `reason`, **obbligatorio**. Riporta a `open`, azzera `closed_at` e `resolution_notes`, e scrive il motivo nel thread |
| POST | `/api/v1/tickets/{id}/cancel` | staff, o chi ha segnalato finché è `open` | `200` |

L'apertura è annidata sotto l'immobile perché una segnalazione nasce **sempre** su un
immobile; tutto il resto va per id, perché il percorso dell'immobile non aggiungerebbe
nulla e obbligherebbe il client a portarselo dietro. È la stessa scelta delle stanze e
dei documenti.

**Ruolo minimo `membro` in apertura, elenco, dettaglio e thread.** È il modulo in cui
l'inquilino prende la parola: fino a qui poteva solo leggere. Quali segnalazioni siano
sue lo decide `app.can_access_ticket` — chi ha aperto, chi è assegnato, chi è parte
del contratto indicato, e chi **abita l'immobile** se la segnalazione è dichiarata di
interesse comune — non un ramo applicativo sul ruolo.

#### La visibilità è una scelta di chi apre

`visible_to_residents` nasce `false`, che è il comportamento di sempre: una segnalazione
la vedono lo staff, chi l'ha aperta, chi ce l'ha in carico e chi è parte del contratto
indicato. Con `true` la leggono anche i **residenti dell'immobile** — la caldaia
condominiale interessa a tutti — e con essa il **thread**, lo **storico** e le
**fotografie**, perché il ramo sta dentro `app.can_access_ticket`, che quattro policy
consultano. Il predicato dei residenti è `app.property_residents`, la funzione che la
`0030` ha già introdotto, e non una terza definizione di «chi abita qui».

**Non concede nessuna scrittura**: `tickets_update` ha un predicato in linea — staff,
`reported_by`, `assigned_to` — e non passa da quella funzione. Un residente vede e non
riapre, non annulla, non corregge. Le **note interne** restano invisibili: le filtra un
`AND` a parte di `ticket_messages_select`.

Migration `0035`, [ADR-047](08-decisioni.md#adr-047).

#### Chi è chi: i nomi viaggiano nella risposta

`TicketResponse` porta `reported_by_name` e `assigned_to_name`; `TicketMessageResponse`
porta `author_name`. Sono **facoltativi**, e l'assenza non è un errore: la policy
`users_select` non concede a chiunque la riga di chiunque, e chi legge deve mettere
un'etichetta neutra — mai l'identificatore ([ADR-043](08-decisioni.md#adr-043)).

I nomi viaggiano **dentro** la risposta e non da una rotta che risolva le persone, che
[ADR-036](08-decisioni.md#adr-036) e ADR-043 hanno già scartato due volte. Il ruolo resta
accanto al nome e non al suo posto: `author_kind` dice da che parte del rapporto sta chi
scrive, il nome dice chi è — e in un co-living due coinquilini sono entrambi `tenant`.
Vedi [ADR-049](08-decisioni.md#adr-049).

#### E dove: `property_label` e `room_name`

`TicketResponse` porta anche i nomi **del posto**: `property_label` e `room_name`. Stessa
forma e stessa regola dell'assenza — campo mancante, etichetta neutra, mai
l'identificatore — con una differenza fra i due:

| Campo | Assente quando |
|---|---|
| `property_label` | l'immobile è **archiviato**. La segnalazione resta, il nome no |
| `room_name` | la segnalazione riguarda l'**immobile intero**, che è il caso normale |

La query è un `LEFT JOIN` con `deleted_at IS NULL` sulle righe collegate, **una per
pagina**. Con un `INNER JOIN` sparirebbero dall'elenco tutte le segnalazioni di ogni
immobile archiviato, che è il modo di far scomparire righe che esistono per andare a
prendere un nome — la trappola che la sezione 3 aveva già pagato sui contratti.

#### Il ciclo di vita

```
open ──► triaged ──► assigned ──► in_progress ──► closed
                          ▲            │  ▲          │
                          │            ▼  │          ▼
                          └──── waiting_tenant     open
                                waiting_owner   (riapertura)

cancelled ◄── da qualunque stato di lavorazione
```

`waiting_tenant` e `waiting_owner` non sono una fase a sé ma una **sospensione** della
lavorazione: si entra con `/wait` e si esce con `/start`. Servono a dire perché una
segnalazione è ferma — «aspetto che tu sia in casa» è lo stato più comune di una
manutenzione reale, e senza un nome resta una segnalazione che sembra abbandonata.

**Su una segnalazione che non è più in lavorazione il thread si legge e non si
scrive.** `POST /messages` risponde `409 ticket.thread_closed` su `closed`, `cancelled`
e `resolved`; `GET /messages` continua a rispondere `200` con il thread intero. È una
regola di **dominio** — il predicato è `TicketStatus::is_working`, lo stesso di `close` e
`cancel` — e non un controllo di interfaccia. Chi vuole dire «non è risolto» **riapre**.

**La riapertura è dell'inquilino**, ed è ciò che rende la segnalazione un rapporto
invece di una casella postale: senza, un guasto ricorrente diventa una segnalazione
nuova ogni volta e lo storico si frantuma proprio dove servirebbe intero. Solo **chi
ha segnalato** può riaprire, e non è una scelta di prodotto: la `USING` di
`tickets_update` ammette staff, chi ha segnalato e chi è assegnato, e non passa dal
contratto. Un coinquilino *vede* la segnalazione ma non può aggiornarla; allargare
richiederebbe una migration sulla policy.

A lui l'API risponde `403 ticket.not_yours` e **non** `org.staff_role_required`: non
gli manca un ruolo — nessun ruolo gli darebbe questa operazione — gli manca l'essere
stato lui a segnalare, e i due rimedi sono diversi. `org.staff_role_required` resta
invece corretto in un caso: chi ha segnalato prova ad **annullare** una segnalazione
già in lavorazione, dove il ruolo di staff serve davvero.

**Riaprire pretende un `reason`**, e l'asimmetria con `resolution_notes` — che è
facoltativo — è deliberata: la chiusura lascia comunque `closed_at` e una riga di
storico, mentre da quando il thread chiuso non riceve messaggi la riapertura senza motivo
non lascerebbe niente che la spieghi. Il motivo **diventa il primo messaggio del thread**,
pubblico e di chi ha riaperto: il thread non si riscrive mai, quindi sopravvive alla
chiusura successiva, e una segnalazione riaperta tre volte ha tre motivi, che una colonna
non terrebbe. `ticket_status_history.reason` resta **non scritta**, e la ragione è in
[ADR-050](08-decisioni.md#adr-050).

Chiudere accetta un corpo con `resolution_notes`, facoltativo: è la risposta a «com'è
andata a finire», che altrimenti andrebbe dedotta rileggendo il thread. La vede
chiunque veda la segnalazione, l'inquilino compreso. **Una riapertura la cancella**,
insieme a `closed_at`: riaprire disfa la chiusura, e la nota fa parte della chiusura —
una segnalazione `open` che dichiara come è stata risolta direbbe il falso sul proprio
stato. Se quel testo deve sopravvivere alla riapertura, il posto è il thread, che non
si riscrive mai.

`resolved` esiste nell'enum e **l'API non lo produce mai**. Il flusso `resolved` con
conferma dell'inquilino è stato scartato: introdurrebbe un'attesa che senza scheduler
non si chiude mai da sé, e un ticket risolto e mai confermato resterebbe appeso per
sempre. La riapertura ottiene lo stesso risultato senza richiedere un orologio.

#### Perché le transizioni sono sotto-risorse e il `PATCH` è solo per i costi

Ognuna ha effetti propri — `closed_at`, una riga di storico, un evento — e un `PATCH`
che li producesse di soppiatto renderebbe invisibile che siano avvenuti. È la stessa
scelta di attivazione e disdetta dei contratti.

Il `PATCH` di `/tickets/{id}` resta per ciò che è davvero un attributo:
`estimated_cost` e `actual_cost`, con la semantica generale del PATCH (assente =
invariato, `null` = azzerato). Il resto:

- **titolo, descrizione e visibilità** si correggono da `PATCH /tickets/{id}/report`, che
  è una **rotta a sé** e non un allargamento di questa. La ragione è
  l'autorizzazione: qui è staff, là è «staff sempre, chi ha segnalato finché è `open`», e
  una rotta sola avrebbe due autorizzazioni **secondo i campi presenti nel corpo** — che è
  il genere di cosa che si sbaglia in silenzio, perché con la semantica del `PATCH` i
  campi sono assenti e un campo aggiunto un anno dopo sfuggirebbe al ramo.
  [ADR-048](08-decisioni.md#adr-048) spiega perché la scelta di prima — «il testo non si
  modifica, il thread esiste per correggere il tiro» — è stata revocata, e che cosa la
  revoca costa;
- **categoria e priorità** si impostano con il triage, perché sono ciò che `triaged`
  *significa*: lasciarle modificabili altrove renderebbe lo stato una dichiarazione
  non verificabile;
- **immobile, stanza e contratto** non sono nemmeno più scrivibili dal ruolo
  applicativo, per privilegio di colonna ([ADR-026](08-decisioni.md#adr-026)). Una
  segnalazione aperta sull'immobile sbagliato si annulla e si riapre.

I costi sono nella valuta della segnalazione, che nasce `EUR` e non è modificabile: la
colonna `currency` è una sola per entrambi, e due importi in valute diverse non
sarebbero rappresentabili. Renderla dichiarabile è rinviato al modulo delle bollette e
degli addebiti, dove il problema si pone davvero.

#### Chi vede quale costo

`actual_cost` lo vede **chiunque veda la segnalazione**, l'inquilino compreso: chi ha
subito il guasto sa quanto è costato ripararlo.

`estimated_cost` è `null` per chi non è staff, **anche quando un valore c'è**. È la
cifra di una trattativa con un fornitore e non riguarda chi abita la casa. La
distinzione non è un confine di sicurezza — la Row Level Security lascia comunque
leggere la riga a chi ha segnalato — ma una regola del contratto dell'API, decisa nel
service layer (`authorization::discloses_estimated_cost`) e applicata alla proiezione.
La direzione è deliberata: aggiungere un campo a una risposta è additivo, toglierlo è
una rottura.

#### Il dettaglio, lo storico e il thread

`GET /api/v1/tickets/{id}` restituisce la segnalazione e lo **storico degli stati**,
che è corto e limitato per costruzione — una riga per transizione. I **messaggi**
stanno su `GET /api/v1/tickets/{id}/messages` con il proprio cursore: una
conversazione non ha un tetto, e incorporarla nel dettaglio significherebbe che il
giorno in cui qualcuno scrive duecento messaggi il dettaglio smette di rispondere.

Lo storico lo scrive un **trigger** del database, non l'API: la prima riga è sempre
`from_status: null → open` e la produce l'inserimento. Per questo `changed_by` può
essere `null` — è `app.current_user_id()`, che fuori da un contesto di richiesta non
c'è.

Un messaggio **non si modifica e non si cancella**, nemmeno in modo soft:
`app.ticket_messages` ha due sole policy, `_select` e `_insert`. Non è un limite da
aggirare, è una proprietà del thread; l'API non espone nulla che lo prometta.

Le **note interne** (`is_internal: true`) le può scrivere solo lo staff, e non
compaiono affatto nell'elenco dell'inquilino: le filtra la policy di SELECT, non un
campo della risposta. A un inquilino che le chieda l'API risponde `403
org.staff_role_required` invece di lasciar produrre `auth.denied_by_policy` alla RLS,
che significa «difetto nostro».

#### Il filtro della coda

`?open=true` restituisce le segnalazioni ancora in lavorazione — i sei stati non
terminali — e `?open=false` quelle chiuse, annullate o risolte. Non è zucchero
sintattico per `status <> 'closed'`: l'indice parziale `tickets_open_ix` è definito su
esattamente quei sei valori, e una disuguaglianza non lo userebbe. È l'elenco che la
dashboard del proprietario legge.

#### `reference`

Ogni segnalazione porta un riferimento leggibile, `TK-000042`, **assegnato dal
database**: è l'unico identificatore del progetto che non nasce nell'applicazione.
La numerazione è **per organizzazione**: la prima segnalazione di ognuna è
`TK-000001`, e due organizzazioni diverse hanno legittimamente lo stesso riferimento.
Il numero dice quante segnalazioni ha aperto quel proprietario, non quante ne esistono
sulla piattaforma ([ADR-027](08-decisioni.md#adr-027)).

Ne segue che **il riferimento non è un identificatore globale**: per raggiungere una
segnalazione serve il suo `id`, e un client che indicizzasse per `reference` senza
l'organizzazione fonderebbe righe diverse. Non ha buchi: una richiesta che fallisce non
consuma un numero, perché il contatore è una riga e non una sequenza.

#### Cosa la risposta non dice, e perché

Non compaiono `ai_suggested_category`, `ai_suggested_priority`, `ai_confidence`,
`ai_analysis`, `sla_due_at` né `scheduled_at`: triage AI, SLA e programmazione sono
fuori dal perimetro di questa fetta. Le colonne esistono nello schema e restano ai loro
default — vedere una colonna non è una ragione per riempirla, e i suggerimenti del
modello sono tenuti separati dai valori decisi da una persona proprio perché si possa
misurare l'accuratezza dopo.

Non c'è una `?q=`: `search_vector` esiste con il suo indice GIN, ma la ricerca è
Meilisearch in V2 ed è pensata per essere alimentata dall'outbox. Esporre ora una
ricerca su `tsvector` significherebbe costruire un contratto da rompere.

```bash
# L'inquilino apre una segnalazione sul proprio immobile e vi allega una foto.
curl -X POST http://localhost:8080/api/v1/properties/$PROPERTY/tickets \
  -H "authorization: Bearer $TENANT_TOKEN" -H "x-org-id: $OWNER_ORG" \
  -H 'content-type: application/json' \
  -d '{ "title": "Il boiler perde", "description": "Perde da ieri sera", "lease_id": "'$LEASE'" }'

curl -X POST http://localhost:8080/api/v1/tickets/$TICKET/documents \
  -H "authorization: Bearer $TENANT_TOKEN" -H "x-org-id: $OWNER_ORG" \
  -F "kind=photo" -F "file=@boiler.jpg;type=image/jpeg"

# Il proprietario classifica, assegna e chiude.
curl -X POST http://localhost:8080/api/v1/tickets/$TICKET/triage \
  -H "authorization: Bearer $OWNER_TOKEN" -H 'content-type: application/json' \
  -d '{ "category": "plumbing", "priority": "high" }'

curl -X POST http://localhost:8080/api/v1/tickets/$TICKET/close \
  -H "authorization: Bearer $OWNER_TOKEN" -H 'content-type: application/json' \
  -d '{ "resolution_notes": "sostituita la valvola di sicurezza" }'
```

### Bollette

| Metodo | Percorso | Ruolo minimo | Esito |
|---|---|---|---|
| POST | `/api/v1/properties/{property_id}/utility-bills` | staff | `201`, stato iniziale `draft`. `409` se la stessa fattura è già registrata; `422` se stanza o contratto sono di un altro immobile |
| GET | `/api/v1/utility-bills` | membro | `200`, pagina keyset. Filtri `property_id`, `lease_id`, `status`, `utility_type` |
| GET | `/api/v1/utility-bills/{id}` | membro | `200`, o `404` |
| PATCH | `/api/v1/utility-bills/{id}` | staff | `200`. **Su una bolletta validata riporta lo stato a `draft`.** `409` da `allocated` in poi |
| POST | `/api/v1/utility-bills/{id}/validate` | staff | `200`. Valorizza `validated_at` e `validated_by` |
| POST | `/api/v1/utility-bills/{id}/void` | staff | `200`. La bolletta **sparisce** dagli elenchi. `409` se già riaddebitata |
| POST | `/api/v1/utility-bills/{id}/recharge` | staff | `201` con **addebito e bolletta**. Ripetibile: la stessa bolletta si riaddebita a più contratti |
| POST | `/api/v1/utility-bills/{id}/documents` | membro | `201`. La scansione, e su una **bozza** accoda la lettura automatica |
| GET | `/api/v1/utility-bills/{id}/documents` | membro | `200`, pagina keyset |
| GET | `/api/v1/utility-bills/{id}/ocr` | **staff** | `200` con lo stato del job e la proposta. `job` è `null` finché nessuno ha caricato una scansione |

**Ruolo minimo `membro` sulle letture, `staff` su tutto ciò che scrive.** Registrare
una bolletta è contabilità; *quali* bollette un inquilino veda lo decide la policy
`utility_bills_select` — staff **oppure** parte del contratto indicato — e non un ramo
applicativo sul ruolo. Ne segue che **una bolletta senza `lease_id` non la vede nessun
inquilino**, nemmeno quello che consuma l'utenza: è il campo da valorizzare perché la
bolletta sia visibile a chi la paga.

**La correzione disfa la validazione**, e non è un effetto collaterale. Validare è
l'affermazione «questi numeri sono quelli», e cambiare i numeri la smentisce: chi la
valida di nuovo deve rifarlo apposta. È la stessa regola per cui riaprire una
segnalazione cancella `closed_at` e la nota di risoluzione. **Da `pending_review` la
bolletta invece resta dov'è**: lì non c'è nessuna affermazione da smentire, c'è una
proposta di una macchina, e correggerla *è* la revisione. Vedi
[Lettura automatica](#lettura-automatica-delle-bollette).

#### `recharged_amount`: quanto è già stato riaddebitato

`UtilityBillResponse` porta la somma delle quote già riaddebitate. Il **residuo** è
`total_amount` meno quel valore, e serve a una cosa sola: riaddebitare una bolletta
d'immobile a tre contratti sono tre sottrazioni a mente, e a mente si sbagliano.

**Non è un vincolo, ed è deliberato.** Nessuno verifica che le quote sommino al totale:
lo schema non ha il vincolo e un controllo applicativo cadrebbe fra due riaddebiti
concorrenti — il criterio di [ADR-015](08-decisioni.md#adr-015), per cui un'invariante
del genere sta nel database o non sta da nessuna parte. **Riaddebitare più del totale
resta possibile**, e il residuo va sotto zero.

Gli addebiti **annullati non contano**: `void` dice «non doveva essere chiesto», quindi
quella quota torna disponibile. Gli **stornati sì**: erano dovuti, e rinunciare a
incassarli non disfa il riaddebito.

**`null` per chi non è staff**, come `estimated_cost` sulle segnalazioni ma con una
ragione più stringente. La somma si legge sotto Row Level Security e `charges_select`
mostra all'inquilino le sole quote sue: il numero che riceverebbe sarebbe **parziale e
indistinguibile da un totale**. Un numero sbagliato è peggio di un numero assente.

#### `property_label` e `room_name`: il **contratto imputato**, non l'immobile della bolletta

`UtilityBillResponse` porta i due nomi, e nominano il **contratto** a cui la bolletta è
imputata: è ciò a cui il collegamento porta, e il nome di un contratto è la casa più la
stanza. L'immobile *della bolletta* resta `property_id` e non ha un nome nella risposta.

L'assenza ha quindi **due cause diverse**, entrambe legittime: la bolletta non è imputata
a nessun contratto — il caso della bolletta d'immobile intero, che non la vede nessun
inquilino — oppure l'immobile di quel contratto è archiviato. In entrambi i casi il campo
manca e chi legge mette un'etichetta neutra, mai l'identificatore
([ADR-043](08-decisioni.md#adr-043), [ADR-049](08-decisioni.md#adr-049)).

I due campi ci sono su **ogni** risposta che porti una bolletta, transizioni comprese, e
non sulle sole letture: una risposta che li portasse a volte sì e a volte no non
permetterebbe di distinguere «archiviato» da «non risolto», e un client mostrerebbe
l'etichetta neutra subito dopo un'azione riuscita.

#### Lettura automatica delle bollette

Allegare una scansione a una bolletta **in bozza** accoda un job di OCR. Il
caricamento risponde `201` come sempre: **l'esito di un caricamento non dipende dalla
disponibilità del fornitore**, e la bolletta resta in bozza con il suo allegato se
l'OCR non è disponibile.

Da lì il percorso è questo, e nessun passo lo compie una richiesta HTTP:

```text
POST .../documents   →  job `queued`, bolletta `draft`
        (worker)     →  job `running`
        (worker)     →  job `succeeded`, bolletta `pending_review`  con la proposta
                     →  job `failed`,    bolletta `draft`           con `error_code`
```

**La macchina non scrive i campi della bolletta.** Scrive `ocr_extracted`,
`ocr_confidence` e `ai_job_id`, e porta lo stato a `pending_review`;
`supplier_name`, `total_amount`, `invoice_number` e gli altri restano quelli che ha
digitato una persona finché una persona non li cambia. La separazione non è prudenza:
è ciò che permetterà di **misurare** quanto l'OCR è accurato, confrontando quello che
ha proposto con quello che una persona ha confermato. Un client che scrivesse la
proposta sopra i campi della bolletta senza chiedere distruggerebbe quel dato prima
di raccoglierlo.

`GET /utility-bills/{id}/ocr` è **dello staff** e non del membro, ed è la stessa
condizione della policy `ai_jobs_select`. Un inquilino che vede la bolletta imputata
al proprio contratto riceve `403 org.staff_role_required` sul job: «non ti è
permesso» e «non c'è» sono due risposte diverse, e un `200` con tutto a `null`
direbbe la seconda al posto della prima.

I valori proposti sono **stringhe**, con un `type` che dice come leggerle
(`text`, `date`, `amount`, `quantity`), per la stessa ragione per cui gli importi lo
sono ovunque in questa API: un decimale che passa da un numero JSON passa da un float
binario, e `123.4567` che torna `123.45670000000001` è un valore che una persona
dovrebbe correggere a mano proprio nel campo che la macchina aveva letto giusto.

`confidence` è la **peggiore** fra quelle dei campi, non la media: dieci campi letti a
`0.99` non devono nascondere il totale letto a `0.40`, che è l'unica cifra che
qualcuno pagherà. **Una confidenza bassa non è un errore** — il job resta `succeeded`
— ed è il campo da guardare per primo. Non esiste una soglia di rifiuto automatico:
sceglierla richiede di aver visto le confidenze di un fornitore vero su bollette vere.

`error_code` (`unsupported_document`, `provider_unavailable`, `document_archived`,
`document_unavailable`, `subject_not_processable`) **non è un codice di errore
dell'API**: un job fallito è l'esito riuscito della richiesta che lo ha accodato, e
non compare mai nel `code` di una risposta di errore.

**Fuori perimetro, e non per dimenticanza:** il fornitore vero
([ADR-037](08-decisioni.md#adr-037)), il ritentativo automatico di un job fallito, il
tetto di spesa per organizzazione, e una rotta che *lanci* l'OCR a comando — che
legherebbe una richiesta HTTP ai secondi di un fornitore esterno. Un job abbandonato
resta `running` per sempre: non c'è scadenza, e il sintomo è muto — una bolletta che
non arriva mai a `pending_review` senza che nulla sia in errore.

**Annullare una bolletta la fa sparire, annullare un addebito no.** L'asimmetria è
deliberata e ha due ragioni che si sommano. Una bolletta inserita male è un documento
nostro e non c'è nessuno a cui dirlo; e `utility_bills_dedup_uk` è parziale su
`deleted_at IS NULL`, quindi senza il soft delete la stessa fattura non sarebbe
**reinseribile mai più**. La traccia resta in `audit_log`, che è append-only.

**Il riaddebito è una sotto-risorsa della bolletta**, e `POST /leases/{id}/charges`
**non accetta `utility_bill_id`**: c'è una via sola perché una bolletta e un addebito
si leghino, ed è quella che tiene onesto lo stato della bolletta. Il tipo dell'addebito
lo decide l'endpoint (`utility_recharge`) e il periodo è quello della bolletta: sono i
due campi che, lasciati al chiamante, permetterebbero di emettere un canone che cita
una bolletta o di dichiarare un periodo diverso da quello fatturato.

**Nessuno verifica che le quote riaddebitate sommino al totale.** Lo schema non ha un
vincolo, e un controllo applicativo sarebbe soggetto a race condition fra due
riaddebiti concorrenti — il criterio di [ADR-015](08-decisioni.md#adr-015) per cui quel
genere di invariante sta nel database o non sta da nessuna parte. La ripartizione con
garanzia di somma è il modulo delle spese co-living, che ha il
`CONSTRAINT TRIGGER … DEFERRABLE` di `expense_shares`. **Riaddebitare più del totale è
oggi possibile.**

**Il consumo ammette quattro decimali**, gli importi due: la colonna è
`NUMERIC(14,4)`. Valore e unità viaggiano insieme in un oggetto `consumption`, perché
`123.4567` senza unità non dice niente.

**Fuori dall'API:** gli stati `paid` — il pagamento *al fornitore* non è modellato
affatto, `app.payable_type` non comprende `utility_bill` — e `disputed`, che non ha una
via di ritorno disegnata. Le colonne esistono e restano ai loro default.
`utility_bills.document_id` **non è popolata**: la scansione si allega con il
riferimento polimorfico dei documenti, come per ogni altro aggregato.

### Addebiti

| Metodo | Percorso | Ruolo minimo | Esito |
|---|---|---|---|
| POST | `/api/v1/leases/{lease_id}/charges` | staff | `201`, stato iniziale **`issued`**. `404` se il contratto non è visibile |
| GET | `/api/v1/charges` | membro | `200`, pagina keyset con il saldo. Filtri `lease_id`, `charge_type`, `status`, `outstanding` |
| GET | `/api/v1/charges/{id}` | membro | `200` con il saldo, o `404` |
| POST | `/api/v1/charges/{id}/void` | staff | `200` dal solo `issued`. **Resta visibile** a chi doveva pagarlo. `409` se è già entrato del denaro |
| POST | `/api/v1/charges/{id}/write-off` | staff | `200` da `issued` **e da `partially_paid`**. Rinuncia all'incasso: diverso dall'annullamento |
| POST | `/api/v1/charges/{id}/documents` | membro | `201` |
| GET | `/api/v1/charges/{id}/documents` | membro | `200`, pagina keyset |
| POST | `/api/v1/charges/{charge_id}/payments` | staff | `201`. Vedi [Pagamenti](#pagamenti) |
| POST | `/api/v1/charges/{charge_id}/payment-declarations` | membro | `201`. Vedi [Pagamenti](#pagamenti) |
| GET | `/api/v1/charges/{charge_id}/payments` | membro | `200`, pagina keyset |

#### `property_label` e `room_name`

`ChargeResponse` porta i nomi del contratto a cui l'addebito è emesso: **la casa e la
stanza**, che è come `R14` aveva già risolto lo stesso difetto nell'elenco dei contratti.
Un collegamento etichettato «Contratti» non dice a chi il denaro è chiesto, e in un
co-living con una riga per stanza non distingue niente.

`room_name` è assente sui contratti sull'immobile intero, che è il caso normale;
`property_label` è assente quando l'immobile è archiviato. Vale anche qui la regola
dell'etichetta neutra e la presenza su **ogni** risposta, transizioni comprese: vedi
[le bollette](#property_label-e-room_name-il-contratto-imputato-non-limmobile-della-bolletta).

#### Le due uscite non sono simmetriche

**`void` è ammesso dal solo `issued`; `write-off` anche da `partially_paid`.**
Non è una svista da appianare:

- **annullare** significa «la richiesta non doveva essere fatta». Su un addebito su
  cui è entrato del denaro sarebbe falso — qualcosa è stato ricevuto contro una
  pretesa che allora non è mai esistita — e a quella domanda questo modello non
  risponde. Risposta: `409 charge.invalid_transition`;
- **stornare** significa «era dovuto e ci rinunciamo». L'incassato **resta
  incassato** e si rinuncia al residuo, quindi la domanda non si pone. È anzi il
  caso in cui una perdita si materializza davvero.

Da `paid`, `void` e `written_off` sono rifiutati entrambi: non c'è un residuo a cui
rinunciare, e gli ultimi due sono terminali.

La perdita che ne risulta è `written_off_total` in dashboard, ed è il **residuo**:
vedi [ADR-040](08-decisioni.md#adr-040).

**Non esiste un `PATCH`**, e non è una dimenticanza: un addebito emesso è una
**pretesa verso una persona**, e riscriverne l'importo dopo averla comunicata non è una
correzione. La migration `0023` toglie al ruolo applicativo il privilegio di scrivere
qualunque colonna che non sia `status`, quindi un `PATCH` non avrebbe nulla da
scrivere. Prorogare una scadenza si fa **annullando e riemettendo**
([ADR-029](08-decisioni.md#adr-029)).

**Non esiste nemmeno uno stato di bozza.** `draft` è nell'enum del database e nessuno
lo scrive: la policy `charges_select` non guarda lo stato, quindi una bozza comparirebbe
nell'elenco di chi deve pagarla esattamente come un addebito emesso; ed emettere non
manda niente a nessuno, perché `NotificationContent` non ha una variante per gli
addebiti — la posta recapita inviti e codici di recupero, non solleciti. Fra i due stati
non resterebbe alcuna differenza osservabile.

**`void` e `write_off` non sono sinonimi.** `void` dice «non doveva essere chiesto»,
`write_off` dice «era dovuto e ci rinunciamo». Entrambi escono dall'insoluto, ma solo il
secondo è una perdita, e la distinzione conta nei report.

#### `status` ed `effective_status`

La risposta porta **due** stati, come i contratti:

| Campo | Significato |
|---|---|
| `status` | quello **memorizzato**, che cambia solo su azioni esplicite |
| `effective_status` | vale `overdue` quando la scadenza è passata e il residuo è positivo |

`overdue` **non è mai memorizzato**: è derivato, per la stessa ragione di `expiring`
sui contratti ([ADR-019](08-decisioni.md#adr-019), esteso agli addebiti da
[ADR-028](08-decisioni.md#adr-028)). Ne segue che **`?status=overdue` risponde `422`**
con il codice `charge.derived_status_filter`, indicando `?outstanding=true`: quel
filtro darebbe sempre zero righe, e una pagina vuota sarebbe il modo peggiore di dirlo.

`?outstanding=true` è lo **scadenzario**, ed enumera i tre stati dell'insoluto
(`issued`, `partially_paid`, `overdue`) invece di scrivere una disuguaglianza: l'indice
`charges_outstanding_ix` è parziale su quell'elenco esatto.

#### Il saldo

`paid_amount` e `outstanding_amount` si leggono da `app.v_charge_balances`, che è
`security_invoker` e applica quindi la RLS del chiamante. Sono **veri dal modulo dei
pagamenti**: fino a quello `paid_amount` valeva sempre `"0.00"` non per un difetto del
calcolo ma per l'assenza di righe in `app.payments`.

`partially_paid` e `paid` sono di conseguenza stati che l'API **produce**, e non li
dichiara nessuno: si ricalcolano dai pagamenti che contano nel saldo, sulle tre soglie
`paid <= 0` → `issued`, `0 < paid < amount` → `partially_paid`, `paid >= amount` →
`paid`. La scrittura passa dalla `GRANT UPDATE (status)` della migration `0023`, che la
fetta contabile aveva posato apposta.

**`outstanding_amount` vale `"0.00"` su un addebito `void` o `written_off`**, e
`effective_status` non diventa mai `overdue` per quei due: la pretesa è caduta, e non è
dovuto niente a prescindere dai pagamenti. La riga **resta comunque leggibile** — chi
si era visto chiedere quella somma deve poter vedere che la richiesta non c'è più — e
`amount` continua a dire quanto era stato chiesto. È la migration `0024` e
[ADR-032](08-decisioni.md#adr-032).

`paid` **non** è nell'elenco che azzera, ed è la parte che conta: dove l'aritmetica è
già significativa, azzerare per stato farebbe scavalcare i pagamenti dallo stato
memorizzato, nascondendo esattamente lo scarto che la contabilità esiste per scoprire.

**Il sovrapagamento porta `outstanding_amount` negativo**, che è un credito verso
l'inquilino. Non è impedito: lo schema non ha il vincolo, e un controllo applicativo
sarebbe soggetto a race condition fra due pagamenti concorrenti
([ADR-015](08-decisioni.md#adr-015)). La dashboard filtra `outstanding_amount > 0`,
quindi il credito **non compensa** gli altri insoluti.

#### La valuta

`currency` è sempre `"EUR"` e **non è dichiarabile** — non compare in nessun corpo di
richiesta. La ragione non è la colonna, che c'è già con il suo default: sono le due
viste dei saldi, che sommano importi **senza confrontare la valuta**. Due valute
renderebbero `outstanding_amount` e `total_outstanding` silenziosamente falsi. Vedi
[ADR-030](08-decisioni.md#adr-030).

```bash
# Il mese del proprietario: registra, valida, riaddebita.
BILL=$(curl -sX POST http://localhost:8080/api/v1/properties/$PROPERTY/utility-bills   -H "authorization: Bearer $OWNER_TOKEN" -H 'content-type: application/json'   -d '{
        "utility_type": "gas",
        "lease_id": "'$LEASE'",
        "supplier_vat": "IT12345678901",
        "invoice_number": "2026/A/00417",
        "period_start": "2026-06-01", "period_end": "2026-07-31",
        "total_amount": "184.20",
        "consumption": { "value": "123.4567", "unit": "Smc" }
      }' | jq -r .id)

curl -X POST http://localhost:8080/api/v1/utility-bills/$BILL/validate   -H "authorization: Bearer $OWNER_TOKEN"

curl -X POST http://localhost:8080/api/v1/utility-bills/$BILL/recharge   -H "authorization: Bearer $OWNER_TOKEN" -H 'content-type: application/json'   -d '{ "lease_id": "'$LEASE'", "amount": "120.00", "due_date": "2026-09-10" }'

# Lo scadenzario del proprietario.
curl -s 'http://localhost:8080/api/v1/charges?outstanding=true'   -H "authorization: Bearer $OWNER_TOKEN"
```

### Pagamenti

Il denaro che entra. Due vie di ingresso, e una sola porta da cui il saldo si muove.

| Metodo | Percorso | Ruolo minimo | Esito |
|---|---|---|---|
| POST | `/api/v1/charges/{charge_id}/payments` | staff | `201`, stato iniziale **`succeeded`**: l'incasso visto sull'estratto conto. `200` se la chiave di idempotenza era già stata usata |
| POST | `/api/v1/charges/{charge_id}/payment-declarations` | membro | `201`, stato iniziale **`pending`**: l'inquilino dichiara di aver pagato. **Non muove il saldo.** `200` sulla chiave ripetuta |
| GET | `/api/v1/charges/{charge_id}/payments` | membro | `200`, pagina keyset. `404` se l'addebito non è visibile |
| GET | `/api/v1/payments` | membro | `200`, pagina keyset. Filtri `status`, `lease_id`. Con `?status=pending` è la **coda delle dichiarazioni** |
| GET | `/api/v1/payments/{id}` | membro | `200` o `404` |
| POST | `/api/v1/payments/{id}/confirm` | staff | `200`. Porta a `succeeded`: da qui il saldo si muove e lo stato dell'addebito si ricalcola |
| POST | `/api/v1/payments/{id}/reject` | staff | `200`. Porta a `cancelled` e **azzera `paid_at`**. Il saldo resta dov'era |

#### Due elenchi, e la ragione non è la simmetria REST

`/charges/{id}/payments` risponde «quali pagamenti ha ricevuto **questo** addebito».
È una sotto-risorsa: l'addebito si rilegge prima, quindi uno che non si vede dà
`404` e non una pagina vuota — che sarebbe il modo peggiore di dire «non è tuo».

`/payments` risponde a una domanda che quella rotta non può porre, perché per porla
bisognerebbe già sapere quale addebito guardare: **quali dichiarazioni aspettano una
conferma, in tutta l'organizzazione**. Senza, una dichiarazione si scoprirebbe
aprendo gli addebiti a uno a uno.

Qui invece **un elenco vuoto è una risposta e non un `404`**: non c'è una risorsa
padre di cui verificare la visibilità, e «non ci sono pagamenti» e «non ce ne sono di
tuoi» danno la stessa pagina — distinguerle richiederebbe di leggere righe che al
chiamante non spettano.

**Non è staff soltanto.** A un inquilino risponde `200` con i **propri** pagamenti: a
scegliere le righe è `payments_select`, non un ramo sul ruolo. È la differenza con
`/dashboard`, dove il controllo applicativo *stringe* la policy invece di ripeterla.
Ne segue che un incasso registrato senza `payer_user_id` non compare a chi l'ha
pagato — vedi [`payer_user_id`, che non è anagrafica](#payer_user_id-che-non-è-anagrafica).

**Nessun filtro sullo stato è derivato**, a differenza degli addebiti e dei
contratti: lo stato di un pagamento è sempre quello memorizzato, quindi nessun valore
risponde `422`. Quattro dei sette descrivono però situazioni che questa API non
scrive mai — `authorized`, `failed`, `refunded`, `partially_refunded` — e filtrarli
restituisce legittimamente zero righe.

`lease_id` **non è una colonna di `app.payments`**: il legame passa dall'addebito
pagato, e il filtro è quindi un `EXISTS` sul contratto di quell'addebito.

L'organizzazione non è un parametro, qui come su ogni altra collezione: a
restringerla è la policy.

#### `Idempotency-Key`: obbligatoria, e la fornisce il client

Le due rotte che creano un pagamento richiedono l'header **`Idempotency-Key`**. Assente
dà `400 request.missing_header`; malformato — spazi, byte di controllo, caratteri non
ASCII stampabili — dà `422` con `errors[].field = "idempotency_key"`.

**Una chiave ripetuta risponde `200` con il pagamento già registrato**, non `201` e non
`409`. È la convenzione dell'idempotenza, e un `409` costringerebbe il client a
distinguere «l'ho già mandato io» da «qualcun altro ha usato la mia chiave» —
distinzione che non può fare.

La chiave la genera **il client**, e non è una comodità: se la generasse il server ogni
ritentativo produrrebbe una chiave nuova, quindi un secondo pagamento, quindi un saldo
che scende **due volte**, e `audit_log` mostrerebbe due inserimenti perfettamente
legittimi. Vedi [ADR-033](08-decisioni.md#adr-033). Un UUID o un ULID per richiesta va
bene; la cassa **non** viene normalizzata, quindi `A1` e `a1` sono due chiavi.

Il `409 payment.idempotency_key_reused` resta per i due casi che la rilettura non
assorbe: la stessa chiave verso un **altro** addebito — dove un `200` con un pagamento
diverso da quello chiesto sarebbe una bugia — e una chiave già usata da qualcuno di cui
il chiamante non vede i pagamenti, che la RLS gli nasconde.

#### Perché due rotte di creazione e non una

Registrare un incasso e dichiarare di aver pagato producono **righe diverse**:
`succeeded` la prima, `pending` la seconda. Una rotta sola con un ramo sul ruolo di chi
chiama metterebbe una decisione di autorizzazione dentro un handler, che deserializza e
serializza e nient'altro. Due rotte rendono anche la specifica OpenAPI vera: le due
operazioni hanno esiti diversi, e un client sa quale sta chiamando.

Non è una scelta che si sta forzando sullo schema: `payments_insert` ammette
`is_org_staff(org_id) OR payer_user_id = app.current_user_id()`, e quelle due
condizioni sono le due rotte.

#### Chi dichiara non può ritirare

**Non esiste una rotta per annullare la propria dichiarazione**, ed è controintuitivo
abbastanza da doverlo dire: si può creare qualcosa che poi non si può disfare. Non è
una scelta di prodotto rivedibile nel codice — `payments_update` è **staff soltanto**,
quindi un ritiro otterrebbe zero righe aggiornate, cioè `auth.denied_by_policy`, il
codice riservato ai nostri difetti. L'uscita da `pending` esiste ed è il rifiuto, che è
dello staff.

Il rifiuto porta a `cancelled` e **non** a `failed`: `failed` significa che il pagamento
non è andato a buon fine **presso il provider**, e con `provider = 'manual'` un provider
non c'è.

#### `payer_user_id`, che non è anagrafica

`payments_select` mostra la riga a chi ha pagato
(`payer_user_id = app.current_user_id()`), oltre che allo staff. Ne segue che
**valorizzarlo è ciò che rende il pagamento visibile all'inquilino**:

- sulla **dichiarazione** è sempre chi chiama, e il corpo non lo accetta: la policy
  esige `payer_user_id = current_user_id()`, e lasciarlo scegliere produrrebbe zero
  righe inserite;
- sulla **registrazione** è un campo opzionale del corpo. Omesso, l'incasso resta
  visibile allo staff soltanto — lecito, un bonifico da un conto che non si sa a chi
  ricondurre esiste, ma va scelto e non subìto.

**Resta opzionale, ed è stato deciso e non rinviato.** Il debito chiedeva se con il
primo client dovesse diventare obbligatorio, o se convenisse dedurlo dalle parti del
contratto: **nessuna delle due**. Obbligarlo toglierebbe all'API il modo di dire
«non attribuito», e dedurlo non si può perché un contratto ha spesso più inquilini
con le loro quote. Il difetto vero era che ometterlo fosse il *default*, e il rimedio
sta nel form: nessun vuoto inviabile, e una voce «non attribuito» che porta scritta
la sua conseguenza. Vedi [ADR-040](08-decisioni.md#adr-040).

Il totale incassato resta comunque visibile a chiunque veda l'addebito, nel
`paid_amount` del saldo, che viene dalla vista e non dall'elenco dei pagamenti.

#### `paid_at`, e il fuso di chi guarda

`paid_at` è l'istante in cui il denaro si è mosso, dichiarato da chi paga. Assente
significa «adesso». La conferma **non lo riscrive**: è il fatto dichiarato, non il
momento in cui qualcuno l'ha guardato. Il rifiuto invece lo **azzera**, perché un
pagamento che non è avvenuto non ha una data in cui è avvenuto.

È un `TIMESTAMPTZ`, e va interpretato: «ieri» per chi dichiara e il `CURRENT_DATE` **del
database** che decide `is_past_due` non sono necessariamente lo stesso giorno. È lo
stesso scarto già registrato per la scadenza degli addebiti in
[ADR-028](08-decisioni.md#adr-028), e qui si vede una seconda volta.

`reconciled_at` è valorizzato quando una persona riconosce il movimento: alla
registrazione diretta e alla conferma. Resta nullo su una dichiarazione pendente e su
una rifiutata.

#### Fuori dall'API in questa fetta

- **i rimborsi**: `refunded_amount` è sempre `"0.00"`, e gli stati `refunded` e
  `partially_refunded` non si scrivono. `partially_refunded` è comunque **contato** dal
  saldo, il che va saputo;
- **i provider veri** (`stripe`, `gocardless`): `provider` resta `manual`,
  `provider_payment_id` e `provider_payload` restano ai default. Gli stati `authorized`
  e `failed` descrivono ciò che accade presso un provider, e non si scrivono;
- **i pagamenti di quote e regolamenti**: `payable_type` li contempla, ma
  `expense_share` e `settlement` appartengono al modulo delle spese co-living. La
  risposta porta `charge_id` e non una coppia polimorfica;
- **il pagamento della bolletta al fornitore**: non è modellato affatto,
  `app.payable_type` non comprende `utility_bill`.

**Un pagamento non si cancella, nemmeno soft.** `app.payments` non ha `deleted_at` e il
ruolo applicativo non ha `DELETE`: un pagamento sbagliato si corregge muovendone lo
stato.

```bash
# L'inquilino dichiara. Il saldo non si muove.
KEY=$(uuidgen)
PAYMENT=$(curl -sX POST http://localhost:8080/api/v1/charges/$CHARGE/payment-declarations   -H "authorization: Bearer $TENANT_TOKEN" -H "x-org-id: $ORG"   -H "idempotency-key: $KEY" -H 'content-type: application/json'   -d '{ "amount": "800.00", "method": "bank_transfer", "reference": "Bonifico del 3" }' | jq -r .id)

# Lo staff conferma. Ora sì.
curl -X POST http://localhost:8080/api/v1/payments/$PAYMENT/confirm   -H "authorization: Bearer $OWNER_TOKEN"

# Lo stesso invio ripetuto: 200, e il saldo scende una volta sola.
curl -sX POST http://localhost:8080/api/v1/charges/$CHARGE/payment-declarations   -H "authorization: Bearer $TENANT_TOKEN" -H "x-org-id: $ORG"   -H "idempotency-key: $KEY" -H 'content-type: application/json'   -d '{ "amount": "800.00", "method": "bank_transfer" }' -o /dev/null -w '%{http_code}\n'
```

### Spese co-living

Il denaro che i coinquilini si devono **fra loro**. È una cosa diversa dagli addebiti:
non è una pretesa del proprietario verso l'inquilino, è un conto fra pari. Il
proprietario può guardarlo — è staff dell'organizzazione — ma non ne è parte, e nessuna
riga di questo modulo entra nella sua contabilità.

| Metodo | Percorso | Ruolo minimo | Esito |
|---|---|---|---|
| POST | `/api/v1/expense-groups` | membro | `201`, **con chi lo crea già dentro** |
| GET | `/api/v1/expense-groups` | membro | `200`, pagina keyset |
| GET | `/api/v1/expense-groups/{id}` | membro | `200` con la composizione, chi è uscito compreso |
| POST | `/api/v1/expense-groups/{id}/archive` | membro | `200`. Chiude il conto; **non** cancella niente |
| POST | `/api/v1/expense-groups/{id}/members` | membro | `201`. Iscrive **o fa rientrare** |
| DELETE | `/api/v1/expense-groups/{id}/members/{user_id}` | membro (per sé), staff (per altri) | `200`. Valorizza `left_at`, **la riga resta** |
| GET | `/api/v1/expense-groups/{id}/balances` | membro | `200`. Chi è in credito e chi in debito |
| POST | `/api/v1/expense-groups/{id}/expenses` | membro | `201` con le **quote** |
| GET | `/api/v1/expense-groups/{id}/expenses` | membro | `200`, pagina keyset |
| GET | `/api/v1/expenses/{id}` | membro | `200` con le quote |
| POST | `/api/v1/expenses/{id}/cancel` | **chi l'ha registrata o anticipata**, o staff | `200`. Soft delete: **esce dai saldi**, anche dopo un regolamento |
| POST | `/api/v1/expense-groups/{id}/settlements` | membro | `201`, stato **`pending`**: non muove i saldi |
| GET | `/api/v1/expense-groups/{id}/settlements` | membro | `200`. `?status=pending` è la coda di chi deve confermare |
| POST | `/api/v1/settlements/{id}/confirm` | **chi riceve**, o staff | `200`. Da qui i saldi si muovono |
| POST | `/api/v1/settlements/{id}/reject` | **chi riceve**, o staff | `200`. Il denaro non è arrivato |
| POST | `/api/v1/settlements/{id}/cancel` | **chi ha dichiarato**, o staff | `200`. Ritira la propria dichiarazione |

**Quasi tutto è `membro` e non `staff`**, che è l'opposto della contabilità. Non è una
concessione: le policy della migration `0014` erano già scritte così —
`expense_groups_insert` chiede `is_org_member`, tutto il resto chiede
`can_access_expense_group`. Un co-living che non può organizzarsi da sé non è il
prodotto descritto in [01 — Prodotto](01-prodotto.md).

#### Una spesa sbagliata si annulla, e i saldi cambiano

`POST /expenses/{id}/cancel` scrive `deleted_at` — la migration `0028` concede la
colonna «per l'annullamento», e `app.expenses` non ha uno stato: il soft delete **è**
l'annullamento. `app.v_expense_group_balances` filtra `deleted_at IS NULL` dalla
`0027`, quindi la spesa esce dai saldi da sola, senza nessuna vista da toccare.

**Annullare una spesa già regolata cambia i saldi**, ed è voluto: un regolamento salda
un *saldo*, non una spesa. Se Marco ha rimborsato 33,33 € a Giulia per una spesa che
poi si rivela sbagliata, annullarla lascia il regolamento e toglie la spesa — e Giulia
si ritrova in debito di 33,33 € verso Marco, che è esattamente lo stato vero. Il
client lo dice nella conferma; l'API non chiede nulla di più.

Dopo, la spesa **non si legge più**: `GET /expenses/{id}` risponde `404`, l'elenco non
la contiene, e un secondo annullamento risponde `404` e non `409` — è la stessa forma
di immobili, stanze e documenti, e un client che agisce su più righe la tratta come
«era già così». Non esiste un ripristino: una spesa annullata per sbaglio si riscrive.

Su un gruppo **archiviato** non si annulla (`409 expense_group.archived`): disfare una
spesa è una scrittura sul conto, e un conto chiuso non ne accetta.

#### Chi crea un gruppo ne è già membro

`POST /expense-groups` restituisce il gruppo **con `members` di un elemento**: chi ha
chiamato. Le due scritture sono nella stessa transazione, e se la seconda fallisce cade
anche la prima.

Non è una comodità dell'API, è una correzione: lo schema, letto come usciva dalla
`0014`, produceva un gruppo **inutilizzabile da chi lo aveva creato**. Un inquilino
soddisfa `expense_groups_insert` ma non `expense_group_members_insert`, perché appena
creato il gruppo non ne è membro e quindi non può aggiungere nessuno — nemmeno sé
stesso. La migration `0026` estende la policy con un ramo stretto: **il creatore può
iscrivere sé stesso, e nessun altro**.

#### `user_id`: come un client li trova

I corpi di questo modulo nominano le persone — chi ha anticipato, a chi si è pagato — e
la via principale è **l'elenco dei residenti dell'immobile**:

- `GET /api/v1/properties/{id}/residents` restituisce chi abita la casa, con il loro
  `user_id`. È il **perimetro della casa**, quindi un coinquilino ci trova gli altri
  coinquilini anche quando ognuno ha il proprio contratto. Vedi
  [Chi abita la casa](#chi-abita-la-casa);
- `GET /api/v1/leases/{id}` restituisce le parti contrattuali con il loro `user_id`.
  **Solo di quel contratto**: `can_access_lease` ammette lo staff, oppure chi è parte di
  quel contratto — non dell'immobile;
- `GET /api/v1/expense-groups/{id}` restituisce la composizione, quindi dal secondo
  membro in poi il gruppo si legge da sé.

**Il giro completo, senza passare dal proprietario:** un coinquilino elenca i residenti
di casa propria, ci trova gli altri con il loro identificatore, crea il gruppo e ce li
aggiunge. Fino alla migration `0030` questo giro **non si poteva fare** in un co-living
**per stanza** (`lease_type = 'room'`): ognuno ha il proprio contratto, quindi non è
parte di quello degli altri, e l'`user_id` dei coinquilini non compariva da nessuna
parte. Il modulo era usabile solo con un contratto unico condiviso.

**Non esiste — e resta deliberatamente fuori — un endpoint che elenchi le persone di
un'organizzazione.** Il perimetro è la casa, ed è precisamente la decisione presa:
[ADR-036](08-decisioni.md#adr-036).

**Aggiungere a un gruppo qualcuno che non abita lì è possibile, ed è voluto.**
`POST /expense-groups/{id}/members` **non verifica** che la persona sia residente
dell'immobile del gruppo, e la ragione non è la fretta: `property_id` sul gruppo è
**opzionale**, quindi una verifica del genere avrebbe un'uscita per costruzione — un
gruppo creato senza immobile non avrebbe nulla contro cui verificare, e una regola che
si aggira omettendo un campo facoltativo si legge come una garanzia senza esserlo. Il
controllo che conta è su **chi aggiunge**, non su chi è aggiunto:
`can_access_expense_group` ammette solo un membro attivo del gruppo o lo staff. E i casi
legittimi esistono — chi ha lasciato la casa il mese scorso e sta ancora saldando il suo
conto è precisamente la persona che la migration `0027` esiste per non perdere di vista.

#### La ripartizione: `equal` e `weighted`, e le altre tre che danno `422`

`split_method` accetta **`equal`** e **`weighted`**. Gli altri tre valori dell'enum
compaiono nella specifica e l'API **li rifiuta con `422`** invece di approssimare:
`percentage` ed `exact` chiedono un input per membro che questa fetta non ha, `by_usage`
chiede letture di contatori che non esistono. Una spesa ripartita con un metodo diverso
da quello che dichiara sarebbe una riga che mente.

Il metodo del gruppo (`default_split_method`) è la **consuetudine**, non una regola:
ogni spesa può dichiararne una diversa — la bombola del gas di chi ha la stufa esiste.

**Il peso è una stringa decimale con tre decimali**, non due: `weight` sta in
`NUMERIC(6,3)` mentre gli importi stanno in `NUMERIC(14,2)`, e la doppia si esprime con
`"2"`. Un quarto decimale dà `422` con `errors[].field = "weight"`.

**La somma delle quote è sempre esattamente l'importo.** Il centesimo di resto va ai
primi partecipanti in ordine di `user_id` — vedi [ADR-034](08-decisioni.md#adr-034) — e
ne segue che una quota può valere `"0.00"`: un centesimo diviso fra tre dà
`0.01 / 0.00 / 0.00`, e lo schema lo prevede (`CHECK (share_amount >= 0)`).

`share_weight` è **nullo** sulle ripartizioni `equal`, dove il peso non ha partecipato
al calcolo.

#### Chi ha anticipato deve essere del gruppo

`paid_by_user_id` è opzionale e assente significa «io». Deve però essere un **membro
attivo**, e un estraneo dà `409 expense.payer_not_in_group`.

Lo schema **non** lo impone — su quella colonna non c'è alcun vincolo verso la
membership — e il danno che il controllo evita è silenzioso: la vista dei saldi parte
dai membri, quindi un credito intestato a un estraneo non comparirebbe a nessuno e la
somma dei saldi del gruppo smetterebbe di fare zero.

#### I saldi, e la somma che fa zero

`GET /expense-groups/{id}/balances` restituisce per ogni persona quanto ha anticipato,
quanto le è stato imputato, quanto ha versato e ricevuto in regolamenti **confermati**,
e il netto:

```
net_balance = total_paid − total_owed + total_settled_out − total_settled_in
```

Positivo = **in credito**, negativo = in debito. `net_balance` **può essere negativo**,
ed è metà dell'informazione per cui il campo esiste.

**La somma dei `net_balance` di un gruppo fa zero**, ed è l'invariante che rende questi
numeri leggibili. Fino alla migration `0027` non la faceva: la vista partiva dai membri
con `left_at IS NULL`, quindi chi lasciava il gruppo **spariva portandosi via il
debito** — il coinquilino che deve 200 euro esce, e quei 200 euro non risultano più a
nessuno, mentre il credito di chi aveva anticipato resta lì senza contropartita. Dalla
`0027` chi è uscito resta finché il suo netto è diverso da zero; chi esce in pari
sparisce, che è il comportamento voluto.

**Una conseguenza da sapere**: il debitore uscito compare nei saldi che leggono gli
**altri** e non nei propri. La vista è `security_invoker` e passa da
`expense_group_members_select`, che chiede una membership **attiva**: per chi è uscito
il gruppo è invisibile. È corretto rispetto all'isolamento, ed è deliberatamente fuori
perimetro — farglielo vedere è un allargamento della visibilità, cioè una decisione di
prodotto con la sua policy.

I saldi **si leggono e non si ricalcolano**, per la ragione di
[ADR-032](08-decisioni.md#adr-032): due definizioni dello stesso numero si contraddicono
al primo caso limite.

#### Uscire, rientrare, archiviare

`DELETE .../members/{user_id}` **non cancella la riga**: valorizza `left_at` e risponde
`200` con la membership aggiornata. Deve restare, perché le quote intestate a quella
persona restano e perché i saldi partono da lì.

**Si esce per sé stessi.** Togliere qualcun altro richiede il ruolo di staff, e dà
altrimenti `403 expense_group.membership_not_yours`. **È un confine vero, non solo una
regola dell'API**: dalla migration `0029` la `WITH CHECK` di
`expense_group_members_update` ammette la riga di un altro solo se il risultato lo
lascia dentro il gruppo, quindi anche una query scritta a mano è rifiutata. La ragione è
che un coinquilino non deve poter togliere unilateralmente un altro da un conto
condiviso, tanto meno uno con un saldo aperto.

**Reintegrare invece resta di chiunque nel gruppo**, ed è la distinzione che la policy
esprime: si può riportare dentro, non si può mandare fuori.

**Rientrare è la stessa rotta dell'iscrizione**: `expense_group_members_group_user_uk` è
unico su `(group_id, user_id)` e **non è parziale**, quindi una seconda riga non può
esistere e il rientro è un aggiornamento. Chi rientra **ridichiara il proprio peso**: la
stanza può essere cambiata, e conservare quello vecchio farebbe pagare la doppia a chi è
tornato nella singola.

**Archiviare chiude il conto e non cancella niente**: spese e saldi restano leggibili, e
un gruppo archiviato rifiuta spese e regolamenti con `409 expense_group.archived`. Si
chiude **anche con dei conti aperti**, ed è deliberato: chi se n'è andato senza pagare
non deve poter tenere il gruppo in ostaggio.

#### Il regolamento è a due tempi, e conferma chi riceve

Chi paga dichiara — la riga nasce `pending` e **non muove i saldi** — e chi riceve
conferma. È la forma dei pagamenti con la differenza che conta: là confermava lo
**staff**, qui conferma il **creditore**, perché è l'unico che sa se il bonifico è
arrivato. Confermare o rifiutare senza esserne il destinatario dà
`403 settlement.not_addressed_to_you`.

**Chi dichiara può ritirare**, ed è l'opposto dei pagamenti. Non è una scelta di
prodotto rivedibile nel codice: la `USING` di `settlements_update` nomina anche
`from_user_id`, mentre `payments_update` è staff soltanto. **Fra pari il ritiro è
lecito, verso il proprietario no.** Ritirare senza aver dichiarato dà
`403 settlement.not_declared_by_you`: il ritiro è di chi ha mandato, il rifiuto di chi
doveva ricevere, e i due fatti sono diversi — «non l'ho mandato» contro «non è
arrivato».

`settled_at` è la data in cui il denaro si è mosso. Assente significa «adesso», la
conferma **non la riscrive** — è il fatto dichiarato, non il momento in cui qualcuno
l'ha guardato — e le due uscite negative la **azzerano**, perché un trasferimento che
non è avvenuto non ha una data in cui è avvenuto.

**Solo `confirmed` muove i saldi.** Lo decide la vista, non l'applicazione:
`app.v_expense_group_balances` somma `WHERE st.status = 'confirmed'`.

#### Fuori dall'API in questa fetta

- **la semplificazione dei debiti**, cioè il «chi paga chi» a numero minimo di bonifici.
  I saldi netti dicono già chi è in credito e chi in debito, e l'algoritmo che minimizza
  i trasferimenti è una comodità di presentazione, non un fatto contabile;
- **`percentage`, `exact` e `by_usage`**, che danno `422`;
- **`expense_shares.is_settled`**, che resta `false` e che la vista **non guarda**:
  sarebbe una seconda definizione di «saldato» che nessuno riconcilia;
- **il legame fra regolamento e pagamento**: `settlements.payment_id` resta nullo e
  `payments.payable_type = 'settlement'` non si scrive. Sono due modi diversi di legare
  la stessa coppia di righe, e un regolamento fra coinquilini non è denaro che
  l'organizzazione incassa — farne una riga in `app.payments` metterebbe nel registro
  degli incassi del proprietario un movimento che non lo riguarda;
- **la ripartizione automatica di una bolletta** (`expenses.utility_bill_id`): il legame
  esiste a schema ed è additivo, ma richiede di decidere *quale* bolletta di un immobile
  riguarda *quale* gruppo — domanda a cui oggi non c'è modo di rispondere, perché
  `property_id` sul gruppo è opzionale;
- **gli allegati alle spese**: `attachment_owner_type` comprende `expense`, il dominio
  no. È la stessa migration additiva già fatta per bollette e addebiti;
- **la correzione di una spesa**: una spesa sbagliata si annulla e si riscrive, come un
  addebito. La migration `0028` concede l'`UPDATE` sulle sole colonne che una correzione
  legittima tocca, e nega `amount` — vedi [ADR-035](08-decisioni.md#adr-035).

```bash
# Un inquilino crea il gruppo, e si ritrova dentro.
GROUP=$(curl -sX POST http://localhost:8080/api/v1/expense-groups \
  -H "authorization: Bearer $TENANT_TOKEN" -H "x-org-id: $ORG" \
  -H 'content-type: application/json' \
  -d '{ "name": "Casa di via Roma", "property_id": "'$PROPERTY'" }' | jq -r .id)

# Aggiunge il coinquilino della doppia.
curl -X POST http://localhost:8080/api/v1/expense-groups/$GROUP/members \
  -H "authorization: Bearer $TENANT_TOKEN" -H "x-org-id: $ORG" \
  -H 'content-type: application/json' \
  -d '{ "user_id": "'$OTHER'", "weight": "2" }'

# Cento euro di spesa: le quote sommano a 100.00 esatti.
curl -sX POST http://localhost:8080/api/v1/expense-groups/$GROUP/expenses \
  -H "authorization: Bearer $TENANT_TOKEN" -H "x-org-id: $ORG" \
  -H 'content-type: application/json' \
  -d '{ "description": "Spesa della settimana", "amount": "100.00", "category": "groceries" }' \
  | jq '.shares'

# Chi deve dichiara, e il saldo non si muove finché il creditore non conferma.
SETTLEMENT=$(curl -sX POST http://localhost:8080/api/v1/expense-groups/$GROUP/settlements \
  -H "authorization: Bearer $OTHER_TOKEN" -H "x-org-id: $ORG" \
  -H 'content-type: application/json' \
  -d '{ "to_user_id": "'$TENANT'", "amount": "66.67" }' | jq -r .id)

curl -X POST http://localhost:8080/api/v1/settlements/$SETTLEMENT/confirm \
  -H "authorization: Bearer $TENANT_TOKEN" -H "x-org-id: $ORG"

# La somma dei saldi fa zero.
curl -s http://localhost:8080/api/v1/expense-groups/$GROUP/balances \
  -H "authorization: Bearer $TENANT_TOKEN" -H "x-org-id: $ORG" \
  | jq '[.items[].net_balance | tonumber] | add'
```

### Inviti

| Metodo | Percorso | Ruolo minimo | Esito |
|---|---|---|---|
| POST | `/api/v1/leases/{id}/invitations` | staff | `201` con il token da consegnare. `409` se c'è già un invito pendente per quell'email |
| DELETE | `/api/v1/invitations/{id}` | staff | `204`. `409` se già accettato |
| POST | `/api/v1/auth/accept-invitation` | autenticato | `200`. `401` se il token è sconosciuto, scaduto o revocato; `409` se già accettato |

Emettere l'invito crea anche la **parte contrattuale** con `user_id` nullo, se non
esiste già una riga con quella email: è ciò che l'invitato collegherà a sé, e senza
di essa accettare non lo renderebbe parte di nulla.

Il token in chiaro esiste **solo** nella risposta a `POST .../invitations` e
nell'email, come il refresh token: in database ne resta lo hash.

**L'email parte dopo il commit e best-effort**, e il token resta nella risposta anche
per questo: se il relay non risponde, l'invito è valido lo stesso e il proprietario
consegna il codice come faceva prima: legare la validità di un invito alla
disponibilità di un servizio esterno sarebbe il baratto sbagliato. Il fallimento va nei
log **senza il destinatario e senza il token**.

**L'invito non passa dall'outbox**, e non è un dettaglio implementativo: gli eventi di
dominio trasportano identificatori e nomi di campo, mai valori, quindi un worker che
leggesse `TenantInvited` non avrebbe niente da spedire. Metterci il token dentro
significherebbe scrivere una credenziale in una tabella fatta per essere pubblicata su
un broker.

`accept-invitation` richiede l'autenticazione ma **non** l'appartenenza: è
l'operazione che la crea. Chi accetta non deve passare `X-Org-Id`, perché di quella
organizzazione non è ancora membro.

**Dopo l'accettazione `X-Org-Id` serve, e non basta rifare il login.** La
registrazione crea sempre un'organizzazione personale di cui l'utente è owner, e
l'organizzazione predefinita all'accesso è quella con il ruolo più alto — owner,
poi manager, poi tenant (`pick_default_membership` in `application::use_cases::auth`).
Un inquilino che si è registrato da sé ha quindi sempre la propria organizzazione in
testa, e senza `X-Org-Id` un `GET /api/v1/leases` gli risponde `200` con zero
elementi: nessun errore, nessun indizio. È registrato fra i debiti noti in
[09 — Stato e roadmap](09-stato-e-roadmap.md#debiti-tecnici-noti).

Un token sconosciuto e un token valido intestato a un'altra email danno lo stesso
`401` con lo stesso corpo: distinguerli sarebbe un oracolo per capire quali inviti
esistono. Un invito **già accettato** dà invece `409`, perché chi presenta quel token
è la persona giusta e merita di sapere che l'operazione è già avvenuta.

Un invito può conferire soltanto il ruolo `tenant`. Il vincolo è nel dominio e
ripetuto nella policy di riga: nominare uno staff passa da chi amministra
l'organizzazione, non da un token spedito per email.

Esempio di creazione:

```bash
curl -X POST http://localhost:8080/api/v1/properties \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{
    "label": "Bilocale centro",
    "property_type": "apartment",
    "address": { "street": "Via Roma 1", "city": "Milano", "province": "MI", "postal_code": "20121" }
  }'
```

## DTO separati dagli aggregati

I tipi sul filo sono definiti nel crate `api` e convertiti verso il dominio, non
sono gli aggregati riesportati. La rappresentazione HTTP è un contratto verso i
client: legarla alla forma interna significa non poter più cambiare l'una senza
rompere l'altra.

## Aggiungere un endpoint

1. DTO di richiesta e risposta in `crates/api/src/routes/<modulo>.rs`, con
   `#[derive(utoipa::ToSchema)]` e le conversioni verso i tipi di dominio.
2. Handler: deserializza, chiama lo use case, serializza. Nient'altro.
3. Annotazione `#[utoipa::path(...)]` con tutti gli stati che l'endpoint può
   restituire.
4. Registrazione in `crates/api/src/openapi.rs` (`paths` e `components`) e nel
   `Router` del modulo.
5. Test di contratto in `crates/api/tests/http_contract.rs`, includendo i casi di
   errore: sono quelli che regrediscono.
