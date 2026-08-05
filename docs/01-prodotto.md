# 01 — Prodotto

## Il problema

Fra proprietario e inquilino c'è attrito, e non è (solo) contabile. Un guasto va
segnalato, capito, preventivato, programmato; una bolletta va letta, ripartita,
riaddebitata; un contratto va rinnovato o disdetto entro scadenze che nessuno
ricorda. Gli strumenti esistenti coprono la contabilità e lasciano fuori la
comunicazione operativa, che è dove si consuma il tempo.

## L'obiettivo

Una piattaforma **bidirezionale**: proprietario e inquilino usano lo stesso
sistema con viste e permessi diversi, non due prodotti che si scambiano PDF. Con
due leve che gli strumenti tradizionali non hanno:

- **automazione intelligente** — OCR delle bollette, triage dei guasti da
  fotografia, così l'inquilino descrive il problema mostrandolo;
- **co-living** — ripartizione delle spese fra coinquilini integrata nella
  gestione dell'immobile, non delegata a un'app separata.

## Utenti

| Utente | Ruolo nel sistema | Cosa fa |
|---|---|---|
| Piccolo proprietario privato | `owner` | Gestisce immobili, contratti, addebiti, guasti. È l'amministratore della propria organizzazione. |
| Gestore delegato | `manager` | Stessi poteri operativi dell'owner, senza il controllo dell'organizzazione. Predisposto, non ancora esposto dall'API. |
| Inquilino | `tenant` | Vede il proprio contratto e i propri addebiti, apre segnalazioni, divide le spese con i coinquilini. |

Un utente può appartenere a più organizzazioni con ruoli diversi: chi affitta la
propria casa e ne abita un'altra è `owner` in una e `tenant` in un'altra.

## Perimetro dell'MVP (V1)

Requisiti funzionali previsti:

gestione utenti · immobili · stanze · contratti · documenti · segnalazioni · OCR
bollette · triage AI da immagine · divisione spese co-living · dashboard ·
pagamenti di base · API REST documentata.

Requisiti non funzionali: Rust/Axum/PostgreSQL/SQLx · Docker e Kubernetes ·
storage oggetti S3-compatibile · JWT e RBAC · Row Level Security · audit log ·
logging privo di dati sensibili · cifratura dei dati personali · osservabilità con
OpenTelemetry.

Lo **stato di avanzamento** rispetto a questo perimetro è in
[09 — Stato e roadmap](09-stato-e-roadmap.md). Lo schema del database copre
l'intero MVP; il backend copre autenticazione e immobili.

## Roadmap

Non va implementata ora, ma l'architettura e lo schema devono poterla accogliere
senza rifacimenti strutturali. Dove esiste già un punto di aggancio è indicato.

### V2

| Funzionalità | Aggancio già presente |
|---|---|
| Assistente AI (RAG, tool calling) | `ai_jobs` provider-agnostico, `documents.extracted_data` |
| Marketplace tecnici | `tickets.assigned_to`, `tickets.metadata` |
| Pagamenti Stripe/SEPA | `payments.provider`, `provider_payment_id`, `provider_payload`, `idempotency_key` |
| Notifiche push | outbox degli eventi di dominio |
| Calendario avanzato | `tickets.scheduled_at`, `leases` con scadenze |
| Analytics (ROI, cash flow) | `properties.purchase_price` e `current_value`, viste `v_*` |
| Motore di ricerca (Meilisearch) | colonne `search_vector` con indice GIN, alimentabili dall'outbox |
| OCR avanzato (catasto, APE) | `properties.cadastral_data`, `document_kind` dedicati |
| Stima costi AI, manutenzione predittiva | `tickets.estimated_cost`, storico in `ticket_status_history` |
| Versionamento documentale | `documents.version`, `supersedes_document_id`, `is_current` |

### V3

Condomini e multi-proprietario (`organizations.kind = 'condominium'`) · smart home
e IoT · digital twin · firma digitale · integrazione Agenzia delle Entrate,
catasto, banche (`leases.registration_number`) · contabilità avanzata (F24) ·
white label · API pubbliche e webhook · SaaS multi-tenant (l'`org_id` con RLS su
ogni tabella è già il confine di isolamento).

## Vincoli tassativi

Valgono per ogni scelta tecnica e sono la ragione di molte decisioni in
[08](08-decisioni.md):

1. Nessun vendor lock-in.
2. Tutti i provider esterni (AI, storage, pagamenti) astratti dietro interfacce.
3. Ogni modulo deve poter essere estratto come microservizio: disaccoppiamento
   reale, non solo cartelle separate.
4. Il dominio non dipende dall'infrastruttura.
5. Nessuna logica di business nei controller HTTP.
6. Le operazioni critiche producono un evento di dominio e una riga di audit.

## Glossario

Il termine "tenant" è ambiguo — in PropTech è l'inquilino, in un SaaS è il
confine di isolamento. Qui vale sempre la prima accezione.

| Termine | Significato |
|---|---|
| **Organization** | Confine di isolamento dei dati. Ogni entità di business le appartiene. Nell'MVP ogni proprietario ne ha una personale. |
| **Tenant / inquilino** | Persona che abita l'immobile. Ruolo `tenant` in un'organizzazione, parte di un contratto. |
| **Lease** | Contratto di locazione. Il nome inglese evita di sovraccaricare "contratto", che in Rust è anche un'interfaccia. |
| **Lease party** | Parte contrattuale: intestatario, co-intestatario o garante. |
| **Charge** | Partita a debito dell'inquilino: canone, deposito, riaddebito di utenze. Non è una fattura fiscale. |
| **Expense / expense share** | Spesa condivisa fra coinquilini e la quota di ciascuno. |
| **Settlement** | Regolamento di un saldo fra due coinquilini. |
| **Outbox** | Tabella `domain_events`: gli eventi di dominio scritti nella stessa transazione del comando che li ha prodotti. |
