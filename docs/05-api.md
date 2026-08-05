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
| Importi | stringhe decimali (`"62.50"`), non float: un binario a virgola mobile non rappresenta esattamente 0.10 |
| Date | ISO 8601 UTC |
| Creazione | `201` con la risorsa nel corpo |
| Eliminazione | `204` senza corpo (è un'archiviazione) |

## Modello di errore

RFC 9457, uno per tutta l'API: il client ha un contratto da gestire invece di
indovinare la forma della risposta endpoint per endpoint.

```json
{
  "type": "https://propflow.dev/problems/validation",
  "title": "Dati non validi",
  "status": 422,
  "detail": "…",
  "request_id": "9c9a0514-6dd6-4f40-ba71-74ab825e8a9d",
  "errors": [{ "field": "password", "message": "deve contenere almeno 12 caratteri" }]
}
```

`detail` ed `errors` sono presenti solo quando informano. `errors` compare sulle
validazioni.

| Stato | `type` | Quando |
|---|---|---|
| 400 | `bad-request` | header malformato |
| 401 | `unauthorized` | token assente, non valido o scaduto; credenziali errate |
| 403 | `forbidden` | autenticato ma senza il ruolo necessario |
| 404 | `not-found` | risorsa inesistente **o non visibile**, e rotta sconosciuta |
| 409 | `conflict` | vincolo di unicità violato |
| 409 | `invariant` | operazione non ammessa nello stato attuale (es. immobile archiviato) |
| 422 | `validation` | corpo sintatticamente valido, semanticamente no |
| 500 | `internal` | errore non previsto: solo `request_id`, nessun dettaglio |
| 503 | — | timeout della richiesta, o `/ready` con database non raggiungibile |

Due scelte deliberate:

- **404 e non 403** quando una risorsa non è visibile: un `403` confermerebbe che
  esiste. Sotto Row Level Security "non esiste" e "non è tua" sono indistinguibili,
  ed è corretto che lo restino anche in risposta.
- **401 identico** per credenziali errate e account inesistente.

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
