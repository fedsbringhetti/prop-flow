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
| `request.invalid_header` | 400 | `X-Org-Id` non è testo valido o non è un UUID |
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

**Risorse non trovate**

| `code` | Stato |
|---|---|
| `property.not_found` · `room.not_found` · `lease.not_found` · `invitation.not_found` · `document.not_found` | 404 |
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
| `session.refresh_token_reused` | 409 | refresh token già ruotato |

**Validazione e interni**

| `code` | Stato | Condizione |
|---|---|---|
| `validation.failed` | 422 | uno o più campi non validi; il dettaglio è in `errors[]` |
| `lease.derived_status_filter` | 422 | filtro su uno stato derivato: usare `in_force` |
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
| POST | `/api/v1/auth/login` | no | `200` con la sessione |
| POST | `/api/v1/auth/refresh` | no | `200`, ruota il refresh token |
| POST | `/api/v1/auth/logout` | sì | `204`, idempotente |

Risposta di sessione, nella forma attesa da un client OAuth2:

```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJFZERTQSJ9…",
  "token_type": "Bearer",
  "expires_in": 899,
  "refresh_token": "15WVHF3OgkEM0SA6M2LbrA6l…",
  "organization_id": "019fce39-0e43-7011-84bf-1d03f6764daf",
  "role": "owner",
  "user_id": "019fce39-0e43-7011-84bf-1cf9fb30d27b"
}
```

Il refresh token in chiaro esiste **solo** in questa risposta: in database ne resta
lo hash.

### Immobili

| Metodo | Percorso | Ruolo minimo | Esito |
|---|---|---|---|
| POST | `/api/v1/properties` | staff | `201`, stato iniziale `draft` |
| GET | `/api/v1/properties` | membro | `200`, pagina keyset. Filtri `status`, `city` |
| GET | `/api/v1/properties/{id}` | membro | `200` o `404` |
| PATCH | `/api/v1/properties/{id}` | staff | `200`. `409` se archiviato |
| DELETE | `/api/v1/properties/{id}` | staff | `204`. Archiviazione, non cancellazione |

L'inquilino vede gli immobili in cui abita, non gli altri della stessa
organizzazione. Non può crearli né modificarli: `403`.

Lo stato non si porta ad `archived` con un PATCH — l'archiviazione ha effetti propri
(soft delete, evento dedicato) e ha il suo verbo. Il tentativo dà `409`.

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
| GET | `/api/v1/properties/{property_id}/documents` | membro | `200`, pagina keyset |
| GET | `/api/v1/rooms/{room_id}/documents` | membro | come sopra |
| GET | `/api/v1/leases/{lease_id}/documents` | membro | come sopra |
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

Non compaiono `status` né `version`: OCR e versionamento sono fuori dal perimetro di
questa fetta. Le colonne esistono nello schema e restano ai loro default — vedere una
colonna non è una ragione per riempirla.

**L'archiviazione è soft, e l'oggetto nel bucket sopravvive alla riga.** Il ruolo
applicativo non ha il privilegio `DELETE`, e rimuovere i byte è un lavoro separato, da
pensare insieme al purge di `retention_until`. È registrato fra i debiti noti in
[09 — Stato e roadmap](09-stato-e-roadmap.md#debiti-tecnici-noti). Non esiste un
ripristino: la policy `documents_update` ha `deleted_at IS NULL` nella `USING`, quindi
un update su una riga archiviata aggiorna zero righe senza dare errore, e ripristinare
richiederebbe una migration sulla policy.

`users.avatar_document_id` e `leases.contract_document_id` restano **non popolati
dall'API**: gli allegati ad avatar, ticket, bollette e spese sono fuori perimetro,
anche se `can_access_attachment` li copre già a livello di policy.

### Inviti

| Metodo | Percorso | Ruolo minimo | Esito |
|---|---|---|---|
| POST | `/api/v1/leases/{id}/invitations` | staff | `201` con il token da consegnare. `409` se c'è già un invito pendente per quell'email |
| DELETE | `/api/v1/invitations/{id}` | staff | `204`. `409` se già accettato |
| POST | `/api/v1/auth/accept-invitation` | autenticato | `200`. `401` se il token è sconosciuto, scaduto o revocato; `409` se già accettato |

Emettere l'invito crea anche la **parte contrattuale** con `user_id` nullo, se non
esiste già una riga con quella email: è ciò che l'invitato collegherà a sé, e senza
di essa accettare non lo renderebbe parte di nulla.

Il token in chiaro esiste **solo** nella risposta a `POST .../invitations`, come il
refresh token: in database ne resta lo hash. L'invio dell'email non è implementato —
la porta `Notifier` è dichiarata e senza adattatore — quindi è il proprietario a
consegnare il token.

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
