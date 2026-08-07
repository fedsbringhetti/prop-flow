# 10 — Rilasci

## Versionamento

**Semantic Versioning**, ramo **0.x**. Prima di 1.0 le rotture di compatibilità
alzano il *minor* e le correzioni il *patch*: è la convenzione che permette di
cambiare idea mentre il prodotto prende forma senza mentire ai consumatori.

Si passa a **1.0.0** quando l'API HTTP è considerata stabile verso client esterni,
non quando l'MVP è completo. Sono due traguardi diversi: si può avere un MVP intero
con un contratto REST ancora in movimento.

La versione dell'**API HTTP** è indipendente e vive nel percorso (`/api/v1`). Un
rilascio `0.4.0 → 0.5.0` non cambia il contratto REST; è `v1 → v2` a cambiarlo, e
quello accade molto più di rado.

### Una sola versione per il workspace

`[workspace.package] version` nella radice; i sei crate dichiarano
`version.workspace = true`. I crate non hanno vita propria e non vengono pubblicati
su crates.io, quindi versionarli separatamente creerebbe sei numeri da tenere
allineati per nulla. **Un rilascio tocca una riga.**

La versione è osservabile dove serve:

| Dove | Come |
|---|---|
| Documento OpenAPI | `env!("CARGO_PKG_VERSION")` in `crates/api/src/openapi.rs` |
| Log di avvio | campo `version` nella riga "API in ascolto" |
| Immagine container | etichette OCI `org.opencontainers.image.version` e `.revision` |

Nessuna di queste è una stringa scritta a mano: una versione copiata diverge dal
binario al primo rilascio.

## Changelog

`CHANGELOG.md` è **generato** da [git-cliff](https://git-cliff.org/) a partire dai
Conventional Commits, con la configurazione in `cliff.toml`. I messaggi sono già
nella forma giusta dal primo commit, quindi il changelog è quasi gratuito — e un
changelog mantenuto a mano diverge dalla storia al primo rilascio in cui qualcuno ha
fretta.

I commit `chore` e `ci` non compaiono: chi legge un changelog vuole sapere cosa è
cambiato per lui, non che abbiamo cambiato una GitHub Action. I merge commit e i
commit non convenzionali vengono scartati.

Rigenerarlo:

```bash
docker run --rm -v "$PWD:/app" -w /app orhunp/git-cliff:latest --tag vX.Y.Z --output CHANGELOG.md
```

## Flusso di rilascio

```
1. branch  release/vX.Y.Z  da dev
2. bump della versione in Cargo.toml + CHANGELOG.md rigenerato
3. PR verso main  →  la CI gira
4. merge con merge commit (mai squash)
5. tag  vX.Y.Z  su main  →  il workflow Release pubblica
6. back-merge di main dentro dev
```

Il **passo 6 non è facoltativo**: senza, `main` contiene un commit di merge che `dev`
non ha, i due rami divergono e il rilascio successivo porta con sé un diff
inspiegabile.

### Cosa fa il workflow al tag

`.github/workflows/release.yml`, su tag `v*.*.*`:

1. **`guard`** — confronta il tag con la versione nel manifest e si ferma se non
   concordano. Senza, si può taggare `v0.2.0` mentre il workspace dichiara `0.1.0`, e
   l'immagine pubblicata riporterebbe nelle etichette e nell'OpenAPI una versione
   diversa da quella del tag che l'ha prodotta.
2. **`verify`** — richiama `ci.yml` con `workflow_call`. La verifica di un rilascio è
   *la stessa* di una pull request, non una copia che col tempo diverge.
3. **`publish`** — costruisce e pubblica su GHCR.
4. **`release`** — estrae la sezione di changelog della versione e crea la GitHub
   Release.

### Tag dell'immagine

`ghcr.io/fedsbringhetti/prop-flow`, con:

| Tag | Esempio | Quando |
|---|---|---|
| `X.Y.Z` | `0.1.0` | sempre |
| `X.Y` | `0.1` | sempre, segue l'ultima patch |
| `sha-<short>` | `sha-a1b2c3d` | sempre, riferimento immutabile |
| `latest` | | solo versioni stabili, mai una pre-release |

Una pre-release si riconosce dal trattino nel tag (`v0.2.0-rc.1`) e non diventa
`latest`: chi scarica per difetto non deve ricevere una candidata.

## Hotfix

Branch da **`main`**, non da `dev`: `dev` può contenere lavoro non rilasciabile, e un
hotfix deve portare in produzione solo la correzione.

```
1. branch  hotfix/descrizione  da main
2. correzione + bump del patch + CHANGELOG
3. PR verso main  →  merge  →  tag vX.Y.Z+1
4. back-merge di main dentro dev
```

## Rollback

Si ridistribuisce il tag precedente: le immagini restano su GHCR e non vengono
sovrascritte.

**Lo schema è un'altra cosa.** Le migration sono append-only e non hanno un `down`:
tornare a un'immagine precedente **non** annulla le migration già applicate. Una
versione che introduce una migration incompatibile all'indietro non è ridistribuibile
semplicemente: serve una migration nuova che compensi.

Ne segue una regola pratica: **una migration dovrebbe essere compatibile con la
versione precedente del codice**, così un rollback dell'immagine funziona senza
toccare il database. Aggiungere una colonna nullable è compatibile; rinominarne una
no — si fa in due rilasci, aggiungendo prima e togliendo poi.

## Protezione dei branch

Da configurare su GitHub, non nel repository. Su `main` e `dev`: pull request
obbligatoria, check **`CI`** richiesto, force-push vietato, cancellazione vietata.

`CI` è il job di riepilogo, quindi aggiungere un job alla pipeline non richiede di
riconfigurare GitHub.

## Cosa il rilascio non è

Produce un artefatto versionato e pubblicato. **Non è un deploy.** Mancano ancora i
manifest Kubernetes, il backup e il restore del database, il rate limiting per IP, le
metriche, l'alerting e il cron per le partizioni di `audit_log`. Vedi
[09 — Stato e roadmap](09-stato-e-roadmap.md).
