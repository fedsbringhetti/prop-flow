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

**Risorse non trovate**

| `code` | Stato |
|---|---|
| `property.not_found` · `room.not_found` · `lease.not_found` · `invitation.not_found` · `document.not_found` · `ticket.not_found` · `utility_bill.not_found` · `charge.not_found` · `payment.not_found` · `expense_group.not_found` · `expense.not_found` · `settlement.not_found` | 404 |
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
| POST | `/api/v1/tickets/{id}/messages` | membro | `201`. `403` a un inquilino che chiede `is_internal` |
| GET | `/api/v1/tickets/{id}/messages` | membro | `200`, pagina keyset. Le note interne non compaiono all'inquilino |
| POST | `/api/v1/tickets/{id}/triage` | staff | `200`. Imposta categoria e priorità |
| POST | `/api/v1/tickets/{id}/assign` | staff | `200`. Corpo: `assignee_id` |
| POST | `/api/v1/tickets/{id}/start` | staff | `200`. Avvia la lavorazione, o la riprende |
| POST | `/api/v1/tickets/{id}/wait` | staff | `200`. Corpo: `waiting_on` (`tenant` o `owner`) |
| POST | `/api/v1/tickets/{id}/close` | staff | `200`. Valorizza `closed_at` e, se inviato, `resolution_notes` |
| POST | `/api/v1/tickets/{id}/reopen` | **chi ha segnalato**, o staff | `200`. Riporta a `open` e azzera `closed_at` e `resolution_notes` |
| POST | `/api/v1/tickets/{id}/cancel` | staff, o chi ha segnalato finché è `open` | `200` |

L'apertura è annidata sotto l'immobile perché una segnalazione nasce **sempre** su un
immobile; tutto il resto va per id, perché il percorso dell'immobile non aggiungerebbe
nulla e obbligherebbe il client a portarselo dietro. È la stessa scelta delle stanze e
dei documenti.

**Ruolo minimo `membro` in apertura, elenco, dettaglio e thread.** È il modulo in cui
l'inquilino prende la parola: fino a qui poteva solo leggere. Quali segnalazioni siano
sue lo decide `app.can_access_ticket` — chi ha aperto, chi è assegnato, e chi è parte
del contratto indicato — non un ramo applicativo sul ruolo.

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

Il `PATCH` resta per ciò che è davvero un attributo: `estimated_cost` e `actual_cost`,
con la semantica generale del PATCH (assente = invariato, `null` = azzerato). Il resto
non è modificabile, e non per dimenticanza:

- **titolo e descrizione** sono il testo con cui una persona ha descritto un guasto in
  un momento preciso, e il thread esiste apposta per correggere il tiro. Un titolo
  riscritto dopo tre messaggi rende incomprensibili i tre messaggi;
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
| POST | `/api/v1/utility-bills/{id}/documents` | membro | `201`. La scansione |
| GET | `/api/v1/utility-bills/{id}/documents` | membro | `200`, pagina keyset |

**Ruolo minimo `membro` sulle letture, `staff` su tutto ciò che scrive.** Registrare
una bolletta è contabilità; *quali* bollette un inquilino veda lo decide la policy
`utility_bills_select` — staff **oppure** parte del contratto indicato — e non un ramo
applicativo sul ruolo. Ne segue che **una bolletta senza `lease_id` non la vede nessun
inquilino**, nemmeno quello che consuma l'utenza: è il campo da valorizzare perché la
bolletta sia visibile a chi la paga.

**La correzione disfa la validazione**, e non è un effetto collaterale. Validare è
l'affermazione «questi numeri sono quelli», e cambiare i numeri la smentisce: chi la
valida di nuovo deve rifarlo apposta. È la stessa regola per cui riaprire una
segnalazione cancella `closed_at` e la nota di risoluzione.

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

**Fuori dall'API in questa fetta:** l'OCR (`ocr_extracted`, `ocr_confidence`,
`ai_job_id`) e gli stati `pending_review`, `paid` — il pagamento *al fornitore* non è
modellato affatto, `app.payable_type` non comprende `utility_bill` — e `disputed`, che
non ha una via di ritorno disegnata. Le colonne esistono e restano ai loro default.
`utility_bills.document_id` **non è popolata**: la scansione si allega con il
riferimento polimorfico dei documenti, come per ogni altro aggregato.

### Addebiti

| Metodo | Percorso | Ruolo minimo | Esito |
|---|---|---|---|
| POST | `/api/v1/leases/{lease_id}/charges` | staff | `201`, stato iniziale **`issued`**. `404` se il contratto non è visibile |
| GET | `/api/v1/charges` | membro | `200`, pagina keyset con il saldo. Filtri `lease_id`, `charge_type`, `status`, `outstanding` |
| GET | `/api/v1/charges/{id}` | membro | `200` con il saldo, o `404` |
| POST | `/api/v1/charges/{id}/void` | staff | `200`. **Resta visibile** a chi doveva pagarlo |
| POST | `/api/v1/charges/{id}/write-off` | staff | `200`. Rinuncia all'incasso: diverso dall'annullamento |
| POST | `/api/v1/charges/{id}/documents` | membro | `201` |
| GET | `/api/v1/charges/{id}/documents` | membro | `200`, pagina keyset |
| POST | `/api/v1/charges/{charge_id}/payments` | staff | `201`. Vedi [Pagamenti](#pagamenti) |
| POST | `/api/v1/charges/{charge_id}/payment-declarations` | membro | `201`. Vedi [Pagamenti](#pagamenti) |
| GET | `/api/v1/charges/{charge_id}/payments` | membro | `200`, pagina keyset |

**Non esiste un `PATCH`**, e non è una dimenticanza: un addebito emesso è una
**pretesa verso una persona**, e riscriverne l'importo dopo averla comunicata non è una
correzione. La migration `0023` toglie al ruolo applicativo il privilegio di scrivere
qualunque colonna che non sia `status`, quindi un `PATCH` non avrebbe nulla da
scrivere. Prorogare una scadenza si fa **annullando e riemettendo**
([ADR-029](08-decisioni.md#adr-029)).

**Non esiste nemmeno uno stato di bozza.** `draft` è nell'enum del database e nessuno
lo scrive: la policy `charges_select` non guarda lo stato, quindi una bozza comparirebbe
nell'elenco di chi deve pagarla esattamente come un addebito emesso; ed emettere non
manda niente a nessuno, perché la porta `Notifier` è dichiarata e senza adattatore. Fra
i due stati non resterebbe alcuna differenza osservabile.

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
| GET | `/api/v1/payments/{id}` | membro | `200` o `404` |
| POST | `/api/v1/payments/{id}/confirm` | staff | `200`. Porta a `succeeded`: da qui il saldo si muove e lo stato dell'addebito si ricalcola |
| POST | `/api/v1/payments/{id}/reject` | staff | `200`. Porta a `cancelled` e **azzera `paid_at`**. Il saldo resta dov'era |

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
