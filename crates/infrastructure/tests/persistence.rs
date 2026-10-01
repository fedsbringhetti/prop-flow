// Crate di test: qui `expect` è lo strumento corretto. Un panic segnala una
// precondizione di test non soddisfatta e fa fallire il test, che è il
// comportamento desiderato; gestire l'errore lo renderebbe verde a vuoto.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

//! Test di integrazione sul layer di persistenza.
//!
//! Verificano ciò che i test unitari non possono: che le query siano corrette,
//! che il contesto RLS venga davvero installato, che eventi e audit finiscano
//! nella stessa transazione del comando.

mod common;

use std::sync::Arc;

use propflow_application::ports::ObjectStorage as _;
use propflow_application::ports::OpaqueTokenFactory;
use propflow_application::use_cases::ai::{ProcessAiJob, Processed};
use propflow_application::use_cases::auth::{AcceptOwnerInvitation, AcceptOwnerInvitationCommand};
use propflow_application::use_cases::billing::{
    GetCharge, IssueCharge, IssueChargeCommand, ListCharges, ListChargesQuery, ListUtilityBills,
    ListUtilityBillsQuery, RechargeUtilityBill, RechargeUtilityBillCommand, RegisterUtilityBill,
    RegisterUtilityBillCommand, UpdateUtilityBill, ValidateUtilityBill, VoidCharge,
    VoidUtilityBill,
};
use propflow_application::use_cases::document::{
    ArchiveDocument, ArchiveDocumentCommand, ListDocuments, ListDocumentsCommand, UploadDocument,
    UploadDocumentCommand,
};
use propflow_application::use_cases::expense::{
    AddGroupMember, AddGroupMemberCommand, CancelSettlement, ConfirmSettlement, CreateExpenseGroup,
    CreateExpenseGroupCommand, DeclareSettlement, DeclareSettlementCommand, GetExpense,
    GetExpenseGroup, GetGroupBalances, LeaveGroup, RecordExpense, RecordExpenseCommand,
    RejectSettlement,
};
use propflow_application::use_cases::invitation::{
    AcceptInvitation, AcceptInvitationCommand, InviteTenant, InviteTenantCommand,
};
use propflow_application::use_cases::lease::{
    ActivateLease, ActivateLeaseCommand, AddLeaseParty, AddLeasePartyCommand, CorrectLeaseParty,
    CorrectLeasePartyCommand, DraftLease, DraftLeaseCommand, GetLease, ListLeases, ListLeasesQuery,
    TerminateLease, TerminateLeaseCommand,
};
use propflow_application::use_cases::payment::{
    ConfirmPayment, DeclarePayment, DeclarePaymentCommand, ListPayments, ListPaymentsQuery,
    RecordPayment, RecordPaymentCommand, RejectPayment,
};
use propflow_application::use_cases::property::{
    ArchiveProperty, ArchivePropertyCommand, CreateProperty, CreatePropertyCommand, ListProperties,
    ListPropertiesQuery, ListPropertyResidents,
};
use propflow_application::use_cases::room::{CreateRoom, CreateRoomCommand, ListRooms};
use propflow_application::use_cases::ticket::{
    AssignTicket, AssignTicketCommand, CancelTicket, CancelTicketCommand, CloseTicket,
    CloseTicketCommand, CorrectTicketReport, CorrectTicketReportCommand, GetTicket,
    ListTicketMessages, ListTicketMessagesCommand, ListTickets, ListTicketsQuery, OpenTicket,
    OpenTicketCommand, PostTicketMessage, PostTicketMessageCommand, ReopenTicket,
    ReopenTicketCommand, StartTicketWork, StartTicketWorkCommand, SuspendTicket,
    SuspendTicketCommand, TriageTicket, TriageTicketCommand,
};
use propflow_domain::RequestContext;
use propflow_domain::ai::{AiJobErrorCode, AiJobStatus, AiTask};
use propflow_domain::billing::{
    BillStatus, ChargeStatus, ChargeType, InvoiceNumber, SupplierVat, UtilityBillChanges,
    UtilityType,
};
use propflow_domain::document::{AttachmentOwner, DocumentKind};
use propflow_domain::expense::{
    ExpenseCategory, ExpenseGroupKind, SettlementStatus, SplitMethod, Weight,
};
use propflow_domain::identity::OrgRole;
use propflow_domain::lease::{LeasePartyChanges, LeasePartyRole, LeaseStatus, RentFrequency};
use propflow_domain::payment::{IdempotencyKey, PaymentMethod, PaymentStatus};
use propflow_domain::ports::WorkerUnitOfWorkFactory as _;
use propflow_domain::ports::{LeaseOverlap, RepositoryError};
use propflow_domain::property::{PropertyStatus, PropertyType, Room, RoomChanges};
use propflow_domain::shared::{Address, CountryCode, Currency, Money, PropertyId, RoomId};
use propflow_domain::ticket::{
    MessageBody, TicketCategory, TicketCorrection, TicketDescription, TicketPriority,
    TicketReference, TicketStatus, TicketStatusChange, TicketTitle, WaitingOn,
};
use propflow_infrastructure::db::PgWorkerUnitOfWorkFactory;
use propflow_infrastructure::security::OpaqueTokenGenerator;

fn address(city: &str) -> Address {
    Address::new(
        "Via Test 1",
        None,
        city,
        Some("MI".into()),
        Some("20100".into()),
        CountryCode::IT,
    )
    .expect("indirizzo valido")
}

/// Registra un proprietario e restituisce il contesto della sua organizzazione.
///
/// Passa da `AcceptOwnerInvitation` (ADR-053) e non più dal vecchio caso d'uso
/// di registrazione libera: un invito da proprietario nella tabella
/// `app.owner_invitations`, inserito con il ruolo privilegiato da
/// `common::issue_owner_invitation` **come farà lo script**, e poi
/// l'accettazione — lo stesso caso d'uso e lo stesso pool privilegiato che
/// userà la rotta vera. Il vecchio caso d'uso sparisce in `F6`: i test di
/// persistenza non devono dipendere da qualcosa che sta per sparire.
async fn register_owner(env: &common::TestEnv, label: &str) -> RequestContext {
    let email = common::unique_email(label);
    let token = common::issue_owner_invitation(&env.pools, &email).await;

    let use_case = AcceptOwnerInvitation::new(
        Arc::clone(&env.factory),
        Arc::clone(&env.hasher),
        Arc::clone(&env.tokens),
        Arc::clone(&env.refresh_tokens),
        Arc::clone(&env.invitation_tokens),
        Arc::clone(&env.clock) as Arc<dyn propflow_domain::ports::Clock>,
    );
    let session = use_case
        .execute(
            &RequestContext::anonymous(),
            AcceptOwnerInvitationCommand {
                token,
                full_name: format!("Proprietario {label}"),
                password: "cavallo batteria graffetta".to_owned(),
                organization_name: Some(format!("Org {label}")),
            },
        )
        .await
        .expect("accettazione dell'invito da proprietario riuscita");

    RequestContext::authenticated(session.user_id).with_org(
        session
            .org_id
            .expect("un proprietario ha sempre un'organizzazione"),
        session
            .org_role
            .expect("un proprietario ha sempre un ruolo"),
    )
}

#[tokio::test]
async fn registration_creates_user_organization_and_membership_atomically() {
    let env = require_db!();
    let context = register_owner(&env, "atomica").await;

    let mut uow = env
        .factory
        .begin(&context)
        .await
        .expect("transazione aperta");
    let org = uow
        .organizations()
        .find_by_id(context.org_id().expect("org presente"))
        .await
        .expect("query riuscita")
        .expect("organizzazione visibile al proprio owner");
    assert!(
        org.slug().as_str().starts_with("org-"),
        "slug: {}",
        org.slug()
    );

    let membership = uow
        .members()
        .find(org.id(), context.user_id().expect("utente"))
        .await
        .expect("query riuscita")
        .expect("membership creata");
    assert_eq!(membership.role(), OrgRole::Owner);
    assert!(membership.is_active());
}

#[tokio::test]
async fn row_level_security_isolates_organizations() {
    let env = require_db!();
    let alfa = register_owner(&env, "alfa").await;
    let beta = register_owner(&env, "beta").await;

    let create = CreateProperty::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _);
    let property = create
        .execute(
            &alfa,
            CreatePropertyCommand {
                label: "Immobile di Alfa".to_owned(),
                property_type: PropertyType::Apartment,
                address: address("Milano"),
                status: None,
            },
        )
        .await
        .expect("creazione riuscita");

    let list = ListProperties::new(Arc::clone(&env.factory));

    // Il proprietario vede il proprio immobile.
    let own = list
        .execute(&alfa, ListPropertiesQuery::default())
        .await
        .expect("elenco riuscito");
    assert!(
        own.items
            .iter()
            .any(|item| item.property.id() == property.id())
    );

    // L'organizzazione Beta non vede nulla di Alfa.
    let other = list
        .execute(&beta, ListPropertiesQuery::default())
        .await
        .expect("elenco riuscito");
    assert!(
        !other
            .items
            .iter()
            .any(|item| item.property.id() == property.id()),
        "l'organizzazione Beta ha visto un immobile di Alfa"
    );

    // Contesto falsificato: l'utente di Alfa dichiara l'organizzazione di Beta.
    // La policy rilegge la membership dal database, quindi non ottiene righe.
    let spoofed = RequestContext::authenticated(alfa.user_id().expect("utente"))
        .with_org(beta.org_id().expect("org"), OrgRole::Owner);
    let spoofed_page = list
        .execute(&spoofed, ListPropertiesQuery::default())
        .await
        .expect("elenco riuscito");
    assert!(
        spoofed_page.items.is_empty(),
        "una GUC falsificata ha concesso visibilità"
    );
}

#[tokio::test]
async fn creating_a_property_writes_one_event_and_one_audit_row() {
    let env = require_db!();
    let context = register_owner(&env, "outbox").await;

    let create = CreateProperty::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _);
    let property = create
        .execute(
            &context,
            CreatePropertyCommand {
                label: "Immobile tracciato".to_owned(),
                property_type: PropertyType::House,
                address: address("Torino"),
                status: None,
            },
        )
        .await
        .expect("creazione riuscita");

    // L'outbox e l'audit si leggono col ruolo privilegiato: l'audit_log è
    // visibile solo agli owner e l'outbox solo agli amministratori di piattaforma.
    let events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM app.domain_events
          WHERE aggregate_id = $1 AND event_type = 'property.created'",
    )
    .bind(property.id().as_uuid())
    .fetch_one(env.pools.privileged())
    .await
    .expect("conteggio eventi");
    assert_eq!(events, 1, "atteso esattamente un evento di dominio");

    let audit: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM app.audit_log
          WHERE table_name = 'properties' AND record_id = $1 AND action = 'insert'",
    )
    .bind(property.id().as_uuid())
    .fetch_one(env.pools.privileged())
    .await
    .expect("conteggio audit");
    assert_eq!(audit, 1, "atteso esattamente una riga di audit");
}

#[tokio::test]
async fn rollback_removes_both_the_row_and_its_event() {
    let env = require_db!();
    let context = register_owner(&env, "rollback").await;

    let property_id = {
        let mut uow = env
            .factory
            .begin(&context)
            .await
            .expect("transazione aperta");
        let property = propflow_domain::property::Property::create(
            PropertyId::generate(),
            context.org_id().expect("org"),
            context.user_id().expect("utente"),
            "Immobile mai salvato",
            PropertyType::Studio,
            address("Genova"),
            chrono::Utc::now(),
        )
        .expect("aggregato valido");

        uow.properties()
            .insert(&property)
            .await
            .expect("insert riuscita");
        uow.events()
            .append(&propflow_domain::events::DomainEvent::new(
                propflow_domain::events::DomainEventPayload::PropertyCreated(
                    propflow_domain::events::PropertyCreated {
                        property_id: property.id(),
                        org_id: context.org_id().expect("org"),
                        owner_id: context.user_id().expect("utente"),
                        city: "Genova".to_owned(),
                    },
                ),
                chrono::Utc::now(),
                propflow_domain::events::EventMetadata::from_context(&context),
            ))
            .await
            .expect("evento accodato");
        property.id()
        // Nessun commit: il Drop della transazione fa rollback.
    };

    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM app.properties WHERE id = $1")
        .bind(property_id.as_uuid())
        .fetch_one(env.pools.privileged())
        .await
        .expect("conteggio immobili");
    let events: i64 =
        sqlx::query_scalar("SELECT count(*) FROM app.domain_events WHERE aggregate_id = $1")
            .bind(property_id.as_uuid())
            .fetch_one(env.pools.privileged())
            .await
            .expect("conteggio eventi");

    assert_eq!(rows, 0, "l'immobile è stato scritto malgrado il rollback");
    assert_eq!(events, 0, "l'evento è sopravvissuto al rollback");
}

#[tokio::test]
async fn archiving_is_a_soft_delete_and_the_app_role_cannot_hard_delete() {
    let env = require_db!();
    let context = register_owner(&env, "archivio").await;

    let create = CreateProperty::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _);
    let property = create
        .execute(
            &context,
            CreatePropertyCommand {
                label: "Da archiviare".to_owned(),
                property_type: PropertyType::Garage,
                address: address("Bologna"),
                status: None,
            },
        )
        .await
        .expect("creazione riuscita");

    ArchiveProperty::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&context, ArchivePropertyCommand { id: property.id() })
        .await
        .expect("archiviazione riuscita");

    // La riga esiste ancora, con deleted_at valorizzato.
    let deleted_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT deleted_at FROM app.properties WHERE id = $1")
            .bind(property.id().as_uuid())
            .fetch_one(env.pools.privileged())
            .await
            .expect("lettura riuscita");
    assert!(
        deleted_at.is_some(),
        "l'archiviazione non ha valorizzato deleted_at"
    );

    // Il ruolo applicativo non ha il privilegio DELETE: il soft delete non è una
    // convenzione ma un vincolo imposto dai permessi.
    let hard_delete = sqlx::query("DELETE FROM app.properties WHERE id = $1")
        .bind(property.id().as_uuid())
        .execute(env.pools.application())
        .await;
    let error = hard_delete.expect_err("DELETE doveva essere negato");
    assert!(
        error.to_string().contains("permission denied")
            || error.to_string().contains("insufficient"),
        "errore inatteso: {error}"
    );
}

/// Crea un immobile e restituisce il suo identificatore.
/// Crea un immobile **disponibile** e restituisce il suo id.
///
/// Disponibile e non in bozza perché la gran parte di chi lo chiama ci attiva poi
/// un contratto, e dalla sezione 12 un immobile in bozza lo rifiuta
/// (`Invariant::PropertyNotLettable`, ADR-015). Che il default sia `draft` resta
/// fissato dai test del dominio e dal bordo HTTP.
async fn create_property(
    env: &common::TestEnv,
    context: &RequestContext,
    label: &str,
    city: &str,
) -> PropertyId {
    CreateProperty::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            context,
            CreatePropertyCommand {
                label: label.to_owned(),
                property_type: PropertyType::Apartment,
                address: address(city),
                status: Some(PropertyStatus::Available),
            },
        )
        .await
        .expect("creazione riuscita")
        .id()
}

#[tokio::test]
async fn rooms_are_written_and_read_back_through_their_property() {
    let env = require_db!();
    let context = register_owner(&env, "stanze").await;
    let property_id = create_property(&env, &context, "Casa in condivisione", "Padova").await;

    let create = CreateRoom::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _);
    let room = create
        .execute(
            &context,
            CreateRoomCommand {
                property_id,
                name: "Camera A".to_owned(),
                attributes: RoomChanges {
                    monthly_rent: Some(Some("450.00".parse().expect("decimale valido"))),
                    utility_share_percent: Some(Some("33.33".parse().expect("decimale valido"))),
                    has_private_bathroom: Some(true),
                    ..Default::default()
                },
            },
        )
        .await
        .expect("creazione della stanza riuscita");

    // La rilettura passa dal database: verifica insieme i cast su
    // `app.money_amount`, `app.percentage` e l'enum `room_status`.
    let rooms = ListRooms::new(Arc::clone(&env.factory))
        .execute(&context, property_id)
        .await
        .expect("elenco riuscito");
    let stored = rooms
        .iter()
        .find(|item| item.room.id() == room.id())
        .expect("stanza presente nell'elenco");
    assert_eq!(stored.room.name(), "Camera A");
    assert_eq!(
        stored
            .room
            .monthly_rent()
            .map(|money| money.amount().to_string()),
        Some("450.00".to_owned())
    );
    assert_eq!(
        stored
            .room
            .utility_share_percent()
            .map(|value| value.to_string()),
        Some("33.33".to_owned())
    );
    assert!(stored.room.has_private_bathroom());
    // Nessun contratto: la derivazione dice «libera», e lo stato memorizzato
    // resta quello che è.
    assert!(!stored.occupied);
}

#[tokio::test]
async fn a_room_cannot_be_attached_to_a_property_of_another_organization() {
    let env = require_db!();
    let alfa = register_owner(&env, "stitching-alfa").await;
    let beta = register_owner(&env, "stitching-beta").await;
    let alfa_property = create_property(&env, &alfa, "Immobile di Alfa", "Verona").await;

    // Stanza dichiarata nell'organizzazione Beta ma agganciata a un immobile di
    // Alfa: sarebbe una riga invisibile alla RLS di entrambe, e quindi non più
    // correggibile dall'interfaccia. Il trigger `assert_org_matches_property` la
    // rifiuta prima che esista.
    let room = Room::create(
        RoomId::generate(),
        beta.org_id().expect("org"),
        alfa_property,
        "Camera rubata",
        chrono::Utc::now(),
    )
    .expect("aggregato valido: il dominio non conosce le organizzazioni altrui");

    let mut uow = env.factory.begin(&beta).await.expect("transazione aperta");
    let error = uow
        .rooms()
        .insert(&room)
        .await
        .expect_err("il cross-org stitching doveva essere rifiutato");
    assert!(
        matches!(error, RepositoryError::Integrity { .. }),
        "atteso Integrity, ottenuto {error:?}"
    );
}

fn money(raw: &str) -> Money {
    Money::new(raw.parse().expect("decimale valido"), Currency::EUR).expect("importo valido")
}

fn days_from_today(days: i64) -> chrono::NaiveDate {
    chrono::Utc::now().date_naive() + chrono::Duration::days(days)
}

/// Redige un contratto con un inquilino al 100% e lo restituisce.
async fn draft_lease(
    env: &common::TestEnv,
    context: &RequestContext,
    property_id: PropertyId,
    room_id: Option<propflow_domain::shared::RoomId>,
    start: chrono::NaiveDate,
    end: Option<chrono::NaiveDate>,
) -> propflow_domain::lease::Lease {
    let view = DraftLease::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            context,
            DraftLeaseCommand {
                property_id,
                room_id,
                start_date: start,
                end_date: end,
                rent: money("800.00"),
                deposit: None,
                rent_frequency: RentFrequency::Monthly,
                rent_due_day: 1,
                notice_months: None,
                auto_renew: false,
                renewal_months: None,
                utilities_included: false,
                istat_indexation: false,
                notes: None,
            },
        )
        .await
        .expect("bozza redatta");

    AddLeaseParty::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            context,
            AddLeasePartyCommand {
                lease_id: view.lease.id(),
                role: LeasePartyRole::Tenant,
                full_name: "Inquilino di prova".to_owned(),
                email: None,
                share_percent: rust_decimal::Decimal::from(100),
                is_primary: true,
            },
        )
        .await
        .expect("parte aggiunta");

    view.lease
}

async fn activate(
    env: &common::TestEnv,
    context: &RequestContext,
    lease_id: propflow_domain::shared::LeaseId,
) -> Result<(), propflow_application::AppError> {
    ActivateLease::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(context, ActivateLeaseCommand { id: lease_id })
        .await
        .map(|_| ())
}

/// `has_in_force_for_property` chiede **le date oltre allo stato**.
///
/// Le tre regole della sezione 12 — non si affitta una bozza, un immobile
/// affittato non torna in bozza né si archivia — si reggono su questa domanda, e
/// una condizione scritta come `status = 'active'` le farebbe rifiutare su un
/// contratto scaduto o disdetto. Il test percorre i tre casi sullo stesso
/// immobile e su uno vicino, perché è l'unico modo di vedere la differenza fra lo
/// stato memorizzato e `app.lease_in_force`.
#[tokio::test]
async fn a_property_has_a_lease_in_force_only_while_the_dates_say_so() {
    let env = require_db!();
    let owner = register_owner(&env, "in-essere-immobile").await;

    let property_id = create_property(&env, &owner, "Immobile in essere", "Trento").await;
    let vacant_id = create_property(&env, &owner, "Immobile sfitto", "Trento").await;

    // Un immobile senza contratti: falso, ed è il caso che l'archiviazione incontra
    // sempre.
    assert!(
        !has_lease_in_force(&env, &owner, vacant_id).await,
        "un immobile senza contratti non ha nulla in essere"
    );

    // Attivo, con oggi dentro il periodo: vero.
    // `draft_lease` mette già un inquilino al 100%: è quanto basta ad attivare, e
    // un invito in più non aggiungerebbe niente alla domanda che si sta provando.
    let current = draft_lease(
        &env,
        &owner,
        property_id,
        None,
        days_from_today(-30),
        Some(days_from_today(300)),
    )
    .await;
    activate(&env, &owner, current.id())
        .await
        .expect("attivato");

    assert!(
        has_lease_in_force(&env, &owner, property_id).await,
        "un contratto attivo con oggi dentro il periodo è in essere"
    );
    assert!(
        !has_lease_in_force(&env, &owner, vacant_id).await,
        "la domanda è per immobile: il contratto dell'uno non risponde per l'altro"
    );

    // Disdetto: falso. Lo stato memorizzato è cambiato.
    TerminateLease::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            TerminateLeaseCommand {
                id: current.id(),
                reason: Some("Disdetta anticipata".to_owned()),
            },
        )
        .await
        .expect("disdetto");

    assert!(
        !has_lease_in_force(&env, &owner, property_id).await,
        "un contratto disdetto non è più in essere"
    );

    // **Attivo e scaduto ieri: falso**, ed è la metà della definizione che uno
    // stato da solo non vede. Nessuno scrive `expired` (ADR-019): la riga resta
    // `active` per sempre, e `status = 'active'` risponderebbe vero.
    let expired = draft_lease(
        &env,
        &owner,
        property_id,
        None,
        days_from_today(-400),
        Some(days_from_today(-1)),
    )
    .await;
    activate(&env, &owner, expired.id())
        .await
        .expect("attivato: l'attivazione non guarda le date");

    let mut uow = env.factory.begin(&owner).await.expect("transazione aperta");
    let stored = uow
        .leases()
        .find_by_id(expired.id())
        .await
        .expect("query riuscita")
        .expect("contratto visibile");
    assert_eq!(
        stored.status(),
        LeaseStatus::Active,
        "lo stato memorizzato resta `active`: è la premessa del caso"
    );
    drop(uow);

    assert!(
        !has_lease_in_force(&env, &owner, property_id).await,
        "un contratto attivo scaduto ieri non è in essere, e uno stato da solo \
         direbbe il contrario"
    );

    // E quello che comincia domani nemmeno: l'altro estremo dell'intervallo.
    let future = draft_lease(
        &env,
        &owner,
        property_id,
        None,
        days_from_today(1),
        Some(days_from_today(400)),
    )
    .await;
    activate(&env, &owner, future.id()).await.expect("attivato");

    assert!(
        !has_lease_in_force(&env, &owner, property_id).await,
        "un contratto che comincia domani non è ancora in essere"
    );
}

async fn has_lease_in_force(
    env: &common::TestEnv,
    context: &RequestContext,
    property_id: PropertyId,
) -> bool {
    let mut uow = env
        .factory
        .begin(context)
        .await
        .expect("transazione aperta");
    uow.leases()
        .has_in_force_for_property(property_id)
        .await
        .expect("query riuscita")
}

#[tokio::test]
async fn two_overlapping_leases_on_the_same_property_cannot_both_be_active() {
    let env = require_db!();
    let context = register_owner(&env, "sovrapposizione").await;
    let property_id = create_property(&env, &context, "Immobile conteso", "Firenze").await;

    let first = draft_lease(
        &env,
        &context,
        property_id,
        None,
        days_from_today(0),
        Some(days_from_today(365)),
    )
    .await;
    let second = draft_lease(
        &env,
        &context,
        property_id,
        None,
        days_from_today(180),
        Some(days_from_today(540)),
    )
    .await;

    // Due bozze sullo stesso periodo convivono: il vincolo di esclusione riguarda
    // solo gli stati che impegnano l'immobile, quindi si possono preparare più
    // ipotesi e attivarne una.
    activate(&env, &context, first.id())
        .await
        .expect("la prima attivazione riesce");

    let error = activate(&env, &context, second.id())
        .await
        .expect_err("la seconda deve essere rifiutata");
    assert!(
        matches!(
            error,
            propflow_application::AppError::OverlappingLease(LeaseOverlap::Property)
        ),
        "atteso un conflitto parlante sull'immobile, ottenuto {error:?}"
    );
}

#[tokio::test]
async fn rooms_are_let_independently_but_a_room_cannot_be_let_twice() {
    let env = require_db!();
    let context = register_owner(&env, "per-stanza").await;
    let property_id = create_property(&env, &context, "Casa a stanze", "Bologna").await;

    let create_room = CreateRoom::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _);
    let mut rooms = Vec::new();
    for name in ["Camera A", "Camera B"] {
        let room = create_room
            .execute(
                &context,
                CreateRoomCommand {
                    property_id,
                    name: name.to_owned(),
                    attributes: RoomChanges::default(),
                },
            )
            .await
            .expect("stanza creata");
        rooms.push(room.id());
    }

    let start = days_from_today(0);
    let end = Some(days_from_today(365));

    // Due stanze diverse dello stesso immobile si affittano nello stesso periodo:
    // il vincolo è su `room_id`, non sull'immobile.
    for room_id in &rooms {
        let lease = draft_lease(&env, &context, property_id, Some(*room_id), start, end).await;
        activate(&env, &context, lease.id())
            .await
            .expect("ogni stanza si affitta per conto proprio");
    }

    // La stessa stanza no.
    let duplicate = draft_lease(
        &env,
        &context,
        property_id,
        Some(rooms[0]),
        days_from_today(30),
        Some(days_from_today(400)),
    )
    .await;
    let error = activate(&env, &context, duplicate.id())
        .await
        .expect_err("la stessa stanza non si affitta due volte");
    assert!(
        matches!(
            error,
            propflow_application::AppError::OverlappingLease(LeaseOverlap::Room)
        ),
        "atteso un conflitto parlante sulla stanza, ottenuto {error:?}"
    );
}

#[tokio::test]
async fn an_expired_lease_keeps_its_stored_state_but_leaves_the_dashboard() {
    let env = require_db!();
    let context = register_owner(&env, "scadenza").await;
    let property_id = create_property(&env, &context, "Immobile scaduto", "Napoli").await;
    let org_id = context.org_id().expect("org").as_uuid();

    // Contratto già finito: iniziato due anni fa, terminato un mese fa.
    let expired = draft_lease(
        &env,
        &context,
        property_id,
        None,
        days_from_today(-730),
        Some(days_from_today(-30)),
    )
    .await;
    activate(&env, &context, expired.id())
        .await
        .expect("attivazione riuscita");

    let detail = GetLease::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&context, expired.id())
        .await
        .expect("lettura riuscita");

    // Nessuno scrive 'expired': lo stato memorizzato resta quello deciso da chi
    // amministra, e la scadenza si deriva dalle date.
    assert_eq!(detail.lease.status(), LeaseStatus::Active);
    assert_eq!(detail.effective_status, LeaseStatus::Expired);

    // Ed è la stessa risposta che dà la dashboard, che è il punto della migration
    // 0017: prima contava `status = 'active'` e teneva dentro i contratti scaduti.
    let active_leases: i64 =
        sqlx::query_scalar("SELECT active_leases FROM app.v_owner_dashboard WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(env.pools.privileged())
            .await
            .expect("lettura della dashboard");
    assert_eq!(
        active_leases, 0,
        "un contratto scaduto non è più fra quelli in essere"
    );

    let occupancy: i64 = sqlx::query_scalar(
        "SELECT active_leases FROM app.v_property_occupancy WHERE property_id = $1",
    )
    .bind(property_id.as_uuid())
    .fetch_one(env.pools.privileged())
    .await
    .expect("lettura dell'occupazione");
    assert_eq!(occupancy, 0);

    // **E le stanze occupate non sono più sempre zero.**
    //
    // La vista contava `r.status = 'occupied'`, che è la colonna che nessuno
    // scrive: `rooms_occupied` e `occupancy_percent` erano zero per ogni immobile
    // del sistema, sempre, e nessuno se n'era accorto perché quella vista la legge
    // solo questo test — che leggeva l'unica colonna giusta. Qui si guarda un
    // immobile con una stanza davvero occupata, e la vista lo dice.
    let abitato = create_property(&env, &context, "Immobile abitato", "Torino").await;
    let stanza = CreateRoom::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &context,
            CreateRoomCommand {
                property_id: abitato,
                name: "Camera occupata".to_owned(),
                attributes: RoomChanges::default(),
            },
        )
        .await
        .expect("stanza creata");
    let in_essere = draft_lease(
        &env,
        &context,
        abitato,
        Some(stanza.id()),
        days_from_today(-10),
        Some(days_from_today(355)),
    )
    .await;
    // `draft_lease` aggiunge già l'inquilino al 100%: senza, l'attivazione
    // rifiuterebbe.
    activate(&env, &context, in_essere.id())
        .await
        .expect("attivazione riuscita");

    // **Il contesto va installato a mano**, e il motivo è la forma della funzione:
    // `app.room_is_occupied` è `SECURITY DEFINER` e porta il confine nel corpo —
    // `can_access_property` — come `app.property_residents` della `0030`. Sul pool
    // privilegiato, che non ha contesto, quel confine risponde «no» e la vista
    // conta zero. Non è un difetto della vista: è la ragione per cui una funzione
    // che legge senza policy deve controllare da sé chi la chiama.
    let mut conn = env
        .pools
        .privileged()
        .acquire()
        .await
        .expect("connessione privilegiata");
    // Servono **tutti e due**: `can_access_property` passa da `is_org_staff`, che
    // chiede il ruolo di *questa* persona in *quella* organizzazione.
    sqlx::query("SELECT set_config('app.current_org_id', $1, false)")
        .bind(org_id.to_string())
        .execute(&mut *conn)
        .await
        .expect("organizzazione installata");
    sqlx::query("SELECT set_config('app.current_user_id', $1, false)")
        .bind(context.user_id().expect("utente").as_uuid().to_string())
        .execute(&mut *conn)
        .await
        .expect("utente installato");

    let (occupate, percentuale): (i64, Option<rust_decimal::Decimal>) = sqlx::query_as(
        "SELECT rooms_occupied, occupancy_percent
           FROM app.v_property_occupancy WHERE property_id = $1",
    )
    .bind(abitato.as_uuid())
    .fetch_one(&mut *conn)
    .await
    .expect("lettura dell'occupazione");
    assert_eq!(occupate, 1, "la vista conta le stanze davvero occupate");
    assert_eq!(
        percentuale.map(|value| value.to_string()),
        Some("100.00".to_owned())
    );

    // Il filtro `in_force` dell'elenco usa la stessa funzione delle viste.
    let list = ListLeases::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _);
    let in_force = list
        .execute(
            &context,
            ListLeasesQuery {
                in_force: Some(true),
                ..Default::default()
            },
        )
        .await
        .expect("elenco riuscito");
    assert!(
        !in_force
            .items
            .iter()
            .any(|item| item.view.lease.id() == expired.id()),
        "un contratto scaduto non deve comparire fra quelli in essere"
    );
}

/// Fabbrica che rifiuta il pool privilegiato.
///
/// La regola 20 limita `begin_privileged()` a registrazione, login e rotazione del
/// refresh token, e ADR-020 dice che l'accettazione di un invito non è fra questi.
/// Un commento non lo dimostrerebbe: questa fabbrica fa fallire il test se il
/// percorso privilegiato viene usato, anche per distrazione, anche fra sei mesi.
struct ApplicationPoolOnly(Arc<dyn propflow_domain::ports::UnitOfWorkFactory>);

#[async_trait::async_trait]
impl propflow_domain::ports::UnitOfWorkFactory for ApplicationPoolOnly {
    async fn begin(
        &self,
        context: &RequestContext,
    ) -> propflow_domain::ports::RepositoryResult<Box<dyn propflow_domain::ports::UnitOfWork>> {
        self.0.begin(context).await
    }

    async fn begin_privileged(
        &self,
    ) -> propflow_domain::ports::RepositoryResult<Box<dyn propflow_domain::ports::UnitOfWork>> {
        panic!("l'accettazione di un invito non deve usare il pool privilegiato");
    }
}

#[tokio::test]
async fn accepting_an_invitation_never_touches_the_privileged_pool() {
    let env = require_db!();
    let owner = register_owner(&env, "invito-rls").await;
    let property_id = create_property(&env, &owner, "Immobile con inquilino", "Trieste").await;
    let lease = draft_lease(
        &env,
        &owner,
        property_id,
        None,
        days_from_today(0),
        Some(days_from_today(365)),
    )
    .await;

    // L'inquilino esiste già come utente, con la sua organizzazione personale: è
    // esattamente la situazione in cui un contesto di sicurezza *esiste* e manca
    // solo la membership.
    let tenant = register_owner(&env, "invito-inquilino").await;
    let tenant_email = {
        let mut uow = env
            .factory
            .begin(&tenant)
            .await
            .expect("transazione aperta");
        uow.users()
            .find_by_id(tenant.user_id().expect("utente"))
            .await
            .expect("query riuscita")
            .expect("utente visibile a sé stesso")
            .email()
            .clone()
    };

    let issued = InviteTenant::new(
        Arc::clone(&env.factory),
        Arc::new(OpaqueTokenGenerator),
        Arc::clone(&env.clock) as _,
        Arc::new(SilentNotifier),
    )
    .execute(
        &owner,
        InviteTenantCommand {
            lease_id: lease.id(),
            email: tenant_email,
            full_name: "Inquilino Invitato".to_owned(),
        },
    )
    .await
    .expect("invito emesso");

    let restricted: Arc<dyn propflow_domain::ports::UnitOfWorkFactory> =
        Arc::new(ApplicationPoolOnly(Arc::clone(&env.factory)));
    let accepted = AcceptInvitation::new(
        restricted,
        Arc::new(OpaqueTokenGenerator),
        Arc::clone(&env.clock) as _,
    )
    .execute(
        &tenant,
        AcceptInvitationCommand {
            token: issued.token,
        },
    )
    .await
    .expect("accettazione riuscita con il solo pool applicativo");

    assert_eq!(accepted.org_id, owner.org_id().expect("org"));
    assert_eq!(accepted.role, OrgRole::Tenant);
    assert_eq!(accepted.lease_id, Some(lease.id()));

    // Le tre scritture sono avvenute, e nella stessa transazione: membership,
    // collegamento della parte contrattuale, invito marcato.
    let tenant_context = RequestContext::authenticated(tenant.user_id().expect("utente"))
        .with_org(accepted.org_id, OrgRole::Tenant);
    let mut uow = env
        .factory
        .begin(&tenant_context)
        .await
        .expect("transazione aperta");

    let membership = uow
        .members()
        .find(accepted.org_id, tenant.user_id().expect("utente"))
        .await
        .expect("query riuscita")
        .expect("membership creata");
    assert_eq!(membership.role(), OrgRole::Tenant);
    assert!(membership.is_active());

    let parties = uow
        .lease_parties()
        .list_for_lease(lease.id())
        .await
        .expect("query riuscita");
    assert!(
        parties
            .iter()
            .any(|party| party.user_id() == tenant.user_id()),
        "la parte contrattuale non è stata collegata all'utente"
    );

    // E ora l'inquilino vede il contratto: la visibilità passa da `lease_parties`.
    let visible = uow
        .leases()
        .find_by_id(lease.id())
        .await
        .expect("query riuscita");
    assert!(
        visible.is_some(),
        "l'inquilino non vede il contratto di cui è parte"
    );
}

/// Le colonne che definiscono il confine di isolamento non devono essere
/// aggiornabili dal ruolo applicativo.
///
/// Prima della migration 0019 non lo erano: il `WITH CHECK` della policy
/// `org_invitations_invitee_accept` vincolava solo l'email, quindi un utente con un
/// invito pendente poteva ripuntarlo su un'altra organizzazione, farsi risultare
/// invitato là e iscriversi come inquilino. Vedi ADR-021.
///
/// Il privilegio viene verificato prima delle policy e prima che le righe vengano
/// esaminate, quindi il tentativo fallisce anche su un identificatore inesistente:
/// il test non ha bisogno di ricostruire lo scenario dell'attacco per dimostrare che
/// la strada è chiusa.
#[tokio::test]
async fn the_application_role_cannot_repoint_an_invitation_to_another_organization() {
    let env = require_db!();

    let attempt = sqlx::query("UPDATE app.org_invitations SET org_id = $1 WHERE id = $2")
        .bind(uuid::Uuid::now_v7())
        .bind(uuid::Uuid::now_v7())
        .execute(env.pools.application())
        .await;

    let error = attempt.expect_err("l'aggiornamento di org_id doveva essere negato");
    assert!(
        error.to_string().contains("permission denied"),
        "atteso un rifiuto per privilegi, ottenuto: {error}"
    );
}

/// Stessa forma sulla parte contrattuale: `lease_id` decide *quale* contratto la riga
/// collega, e l'invitato non deve poterlo cambiare per agganciarsi a un contratto
/// diverso da quello per cui è stato invitato.
#[tokio::test]
async fn the_application_role_cannot_repoint_a_lease_party_to_another_lease() {
    let env = require_db!();

    let attempt = sqlx::query("UPDATE app.lease_parties SET lease_id = $1 WHERE id = $2")
        .bind(uuid::Uuid::now_v7())
        .bind(uuid::Uuid::now_v7())
        .execute(env.pools.application())
        .await;

    let error = attempt.expect_err("l'aggiornamento di lease_id doveva essere negato");
    assert!(
        error.to_string().contains("permission denied"),
        "atteso un rifiuto per privilegi, ottenuto: {error}"
    );

    // Il collegamento legittimo — `user_id` — resta invece concesso: la restrizione
    // riguarda il confine, non l'operazione.
    let allowed = sqlx::query("UPDATE app.lease_parties SET user_id = $1 WHERE id = $2")
        .bind(uuid::Uuid::now_v7())
        .bind(uuid::Uuid::now_v7())
        .execute(env.pools.application())
        .await;
    assert!(
        allowed.is_ok(),
        "l'aggiornamento di user_id non doveva essere negato: {:?}",
        allowed.err()
    );
}

/// I privilegi di colonna della `0038`, **una colonna per volta**.
///
/// `platform_role` da sola non basterebbe, ed è la ragione per cui il test le
/// prova tutte: **è l'elenco a essere il contratto**. La `WITH CHECK` di
/// `users_update_self` nomina la sola identità della riga, quindi ciò che il
/// privilegio non nega resta scrivibile da chiunque abbia le credenziali del ruolo
/// applicativo.
///
/// La più cara è `platform_role`, e la catena è quella scritta in testa alla
/// migration: `Login` la mette nei claim, il contesto installa
/// `app.platform_admin = 'on'`, e da lì `is_org_member` e `is_org_staff` sono vere
/// su **ogni** organizzazione. Seguono `status`, con cui ci si toglie una
/// sospensione, e `failed_login_count` con `locked_until`, che sono la difesa dal
/// blocco dopo dieci tentativi.
///
/// Il privilegio si verifica **prima** delle policy e prima che le righe vengano
/// esaminate: il tentativo fallisce anche su un identificatore inesistente, quindi
/// non serve ricostruire lo scenario per dimostrare che la strada è chiusa.
#[tokio::test]
async fn the_application_role_cannot_rewrite_what_an_account_is() {
    let env = require_db!();
    let id = uuid::Uuid::now_v7();

    for statement in [
        "UPDATE app.users SET platform_role = 'admin' WHERE id = $1",
        "UPDATE app.users SET status = 'active' WHERE id = $1",
        "UPDATE app.users SET email = 'altro@example.it' WHERE id = $1",
        "UPDATE app.users SET email_verified_at = now() WHERE id = $1",
        "UPDATE app.users SET last_login_at = now() WHERE id = $1",
        "UPDATE app.users SET failed_login_count = 0 WHERE id = $1",
        "UPDATE app.users SET locked_until = NULL WHERE id = $1",
        "UPDATE app.users SET deleted_at = NULL WHERE id = $1",
    ] {
        let error = sqlx::query(statement)
            .bind(id)
            .execute(env.pools.application())
            .await
            .expect_err("l'aggiornamento doveva essere negato");
        assert!(
            error.to_string().contains("permission denied"),
            "atteso un rifiuto per privilegi su `{statement}`, ottenuto: {error}"
        );
    }

    // Le quattro concesse restano concesse, ed è la metà che rende il test un
    // contratto invece di un divieto: `update_profile` nomina esattamente queste.
    for statement in [
        "UPDATE app.users SET full_name = 'Nome Nuovo' WHERE id = $1",
        "UPDATE app.users SET phone = '+39 333 1234567' WHERE id = $1",
        "UPDATE app.users SET locale = 'it-IT' WHERE id = $1",
        "UPDATE app.users SET password_hash = '$argon2id$nuovo' WHERE id = $1",
    ] {
        let allowed = sqlx::query(statement)
            .bind(id)
            .execute(env.pools.application())
            .await;
        assert!(
            allowed.is_ok(),
            "`{statement}` non doveva essere negata: {:?}",
            allowed.err()
        );
    }
}

// ---------------------------------------------------------------------------
// Documenti
// ---------------------------------------------------------------------------

/// Doppio in memoria di `ObjectStorage` per i test di persistenza.
///
/// Qui si verifica il **repository**, non S3: legare questi test a un MinIO
/// acceso significherebbe non poterli eseguire quando lo storage manca, e la
/// query non ha nulla da guadagnarne.
/// Doppio di `Notifier` che non fa niente.
///
/// Questi test verificano la **persistenza** — che cosa entra in database e che
/// cosa la Row Level Security concede — e l'invio è best-effort dopo il commit:
/// legarli a un canale di posta significherebbe far fallire un test dello schema
/// perché un doppio ha detto di no.
struct SilentNotifier;

#[async_trait::async_trait]
impl propflow_application::ports::Notifier for SilentNotifier {
    async fn send(
        &self,
        _notification: &propflow_application::ports::Notification,
    ) -> Result<(), propflow_application::ports::NotifyError> {
        Ok(())
    }

    async fn probe(&self) -> Result<(), propflow_application::ports::NotifyError> {
        Ok(())
    }
}

#[derive(Default)]
struct MemoryStorage {
    objects: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
}

#[async_trait::async_trait]
impl propflow_application::ports::ObjectStorage for MemoryStorage {
    async fn put(
        &self,
        key: &str,
        _content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<propflow_application::ports::StoredObject, propflow_application::ports::StorageError>
    {
        use sha2::{Digest as _, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let checksum: [u8; 32] = hasher.finalize().into();
        let size_bytes = bytes.len() as u64;
        self.objects
            .lock()
            .expect("lock")
            .insert(key.to_owned(), bytes);
        Ok(propflow_application::ports::StoredObject {
            provider: "memory".to_owned(),
            bucket: "test".to_owned(),
            key: key.to_owned(),
            size_bytes,
            checksum_sha256: checksum,
            encryption: Some("AES256".to_owned()),
        })
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, propflow_application::ports::StorageError> {
        self.objects
            .lock()
            .expect("lock")
            .get(key)
            .cloned()
            .ok_or(propflow_application::ports::StorageError::NotFound)
    }

    async fn presigned_get_url(
        &self,
        key: &str,
        ttl_seconds: u32,
    ) -> Result<String, propflow_application::ports::StorageError> {
        Ok(format!("https://storage.test/{key}?ttl={ttl_seconds}"))
    }
}

/// Crea immobile e contratto, e restituisce l'owner del documento.
async fn lease_owner(
    env: &common::TestEnv,
    context: &RequestContext,
    city: &str,
) -> AttachmentOwner {
    let property = CreateProperty::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            context,
            CreatePropertyCommand {
                label: format!("Immobile {city}"),
                property_type: PropertyType::Apartment,
                address: address(city),
                status: None,
            },
        )
        .await
        .expect("immobile creato");

    let lease = draft_lease(
        env,
        context,
        property.id(),
        None,
        days_from_today(0),
        Some(days_from_today(365)),
    )
    .await;
    AttachmentOwner::Lease(lease.id())
}

async fn upload(
    env: &common::TestEnv,
    storage: &Arc<MemoryStorage>,
    context: &RequestContext,
    owner: AttachmentOwner,
    filename: &str,
) -> propflow_domain::document::Document {
    UploadDocument::new(
        Arc::clone(&env.factory),
        Arc::clone(storage) as Arc<dyn propflow_application::ports::ObjectStorage>,
        Arc::clone(&env.clock) as _,
        propflow_infrastructure::DeterministicOcrProvider::NAME,
    )
    .execute(
        context,
        UploadDocumentCommand {
            owner,
            kind: DocumentKind::LeaseContract,
            original_filename: filename.to_owned(),
            mime_type: "application/pdf".to_owned(),
            content: b"%PDF-1.7 contenuto".to_vec(),
        },
    )
    .await
    .expect("caricamento riuscito")
}

/// Il giro completo sul repository: la riga si rilegge identica, e i campi che
/// arrivano dallo storage — dimensione, digest, modalità di cifratura — non sono
/// dichiarati dal chiamante.
#[tokio::test]
async fn a_document_survives_a_round_trip_through_the_repository() {
    let env = require_db!();
    let context = register_owner(&env, "doc-round-trip").await;
    let owner = lease_owner(&env, &context, "Bologna").await;
    let storage = Arc::new(MemoryStorage::default());

    let saved = upload(&env, &storage, &context, owner, "contratto-mario-rossi.pdf").await;

    let mut uow = env.factory.begin(&context).await.expect("transazione");
    let read = uow
        .documents()
        .find_by_id(saved.id())
        .await
        .expect("query riuscita")
        .expect("documento visibile a chi lo ha caricato");

    assert_eq!(read.id(), saved.id());
    assert_eq!(read.org_id(), saved.org_id());
    assert_eq!(read.owner(), owner);
    assert_eq!(read.kind(), DocumentKind::LeaseContract);
    assert_eq!(read.mime_type().as_str(), "application/pdf");
    assert_eq!(read.size_bytes(), saved.size_bytes());
    assert_eq!(read.checksum(), saved.checksum());
    assert_eq!(
        read.original_filename().as_str(),
        "contratto-mario-rossi.pdf"
    );
    assert_eq!(read.uploaded_by(), context.user_id());
    // Il doppio dichiara `AES256`: la colonna registra la modalità, non una chiave.
    assert!(read.is_encrypted());
    assert_eq!(read.encryption_key_ref(), Some("AES256"));

    // La chiave dell'oggetto si deriva dagli identificatori: è la sola garanzia
    // che il nome del file non finisca nei log del fornitore di storage, e che
    // due omonimi non si scontrino sull'indice unico `documents_object_uk`.
    assert_eq!(
        read.storage().object_key(),
        format!("{}/{}", read.org_id(), read.id())
    );
}

/// Un documento di un'altra organizzazione non esiste, e il caso d'uso lo dice
/// con un `404` sull'owner invece di lasciarlo arrivare alle policy di riga.
#[tokio::test]
async fn documents_are_isolated_between_organizations() {
    let env = require_db!();
    let alfa = register_owner(&env, "doc-alfa").await;
    let beta = register_owner(&env, "doc-beta").await;
    let storage = Arc::new(MemoryStorage::default());

    let owner = lease_owner(&env, &alfa, "Torino").await;
    let document = upload(&env, &storage, &alfa, owner, "riservato.pdf").await;

    let mut uow = env.factory.begin(&beta).await.expect("transazione");
    let visible = uow
        .documents()
        .find_by_id(document.id())
        .await
        .expect("query riuscita");
    assert!(
        visible.is_none(),
        "il documento di un'altra organizzazione non deve essere visibile"
    );
    drop(uow);

    // Il caso d'uso si ferma prima, sull'owner: `lease.not_found` e non
    // `auth.denied_by_policy`, che segnalerebbe un difetto nostro.
    let error = UploadDocument::new(
        Arc::clone(&env.factory),
        Arc::clone(&storage) as Arc<dyn propflow_application::ports::ObjectStorage>,
        Arc::clone(&env.clock) as _,
        propflow_infrastructure::DeterministicOcrProvider::NAME,
    )
    .execute(
        &beta,
        UploadDocumentCommand {
            owner,
            kind: DocumentKind::Other,
            original_filename: "intruso.pdf".to_owned(),
            mime_type: "application/pdf".to_owned(),
            content: b"x".to_vec(),
        },
    )
    .await
    .expect_err("deve rifiutare");
    assert_eq!(error.code(), "lease.not_found");
}

/// L'elenco è paginato con cursore keyset e filtra `deleted_at IS NULL` e
/// `is_current`, che è ciò che rende utilizzabile l'indice parziale
/// `documents_owner_ix`.
#[tokio::test]
async fn listing_by_owner_paginates_and_hides_archived_documents() {
    let env = require_db!();
    let context = register_owner(&env, "doc-elenco").await;
    let owner = lease_owner(&env, &context, "Genova").await;
    let storage = Arc::new(MemoryStorage::default());

    let mut ids = Vec::new();
    for index in 0..3 {
        ids.push(
            upload(
                &env,
                &storage,
                &context,
                owner,
                &format!("allegato-{index}.pdf"),
            )
            .await,
        );
    }

    let list = ListDocuments::new(Arc::clone(&env.factory));

    // Prima pagina da due: il cursore c'è perché ne resta una.
    let page = list
        .execute(
            &context,
            ListDocumentsCommand {
                owner,
                after: None,
                limit: Some(2),
            },
        )
        .await
        .expect("elenco riuscito");
    assert_eq!(page.items.len(), 2);
    assert_eq!(page.next_cursor, Some(page.items[1].id()));

    // Seconda pagina: l'ultima, senza cursore. Gli id sono UUIDv7, quindi
    // l'ordine per id è quello di creazione.
    let page = list
        .execute(
            &context,
            ListDocumentsCommand {
                owner,
                after: page.next_cursor,
                limit: Some(2),
            },
        )
        .await
        .expect("elenco riuscito");
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id(), ids[2].id());
    assert_eq!(page.next_cursor, None);

    // Archiviato sparisce dall'elenco. **L'oggetto resta nel bucket**: nessuno lo
    // rimuove, ed è un debito dichiarato, non una dimenticanza.
    ArchiveDocument::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&context, ArchiveDocumentCommand { id: ids[0].id() })
        .await
        .expect("archiviazione riuscita");

    let page = list
        .execute(
            &context,
            ListDocumentsCommand {
                owner,
                after: None,
                limit: None,
            },
        )
        .await
        .expect("elenco riuscito");
    let remaining: Vec<_> = page
        .items
        .iter()
        .map(propflow_domain::document::Document::id)
        .collect();
    assert_eq!(remaining, vec![ids[1].id(), ids[2].id()]);
    assert_eq!(
        storage.objects.lock().expect("lock").len(),
        3,
        "l'archiviazione non deve toccare lo storage"
    );
}

/// Non esiste un ripristino, e riarchiviare non è un no-op silenzioso.
///
/// La policy `documents_update` ha `deleted_at IS NULL` nella `USING`: un update
/// su una riga già archiviata non dà errore dal database — aggiorna **zero
/// righe**. L'invariante di dominio ferma la richiesta prima, così il chiamante
/// riceve un `409` invece di un successo che non ha cambiato nulla.
#[tokio::test]
async fn an_archived_document_cannot_be_archived_twice() {
    let env = require_db!();
    let context = register_owner(&env, "doc-riarchivio").await;
    let owner = lease_owner(&env, &context, "Verona").await;
    let storage = Arc::new(MemoryStorage::default());
    let document = upload(&env, &storage, &context, owner, "unico.pdf").await;

    let archive = ArchiveDocument::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _);
    archive
        .execute(&context, ArchiveDocumentCommand { id: document.id() })
        .await
        .expect("prima archiviazione riuscita");

    // La seconda non trova più la riga: archiviata è invisibile alla policy di
    // SELECT, e per il chiamante "archiviato" e "non esiste" coincidono.
    let error = archive
        .execute(&context, ArchiveDocumentCommand { id: document.id() })
        .await
        .expect_err("deve rifiutare");
    assert_eq!(error.code(), "document.not_found");
}

/// Comando ed evento sono atomici, come per ogni altro aggregato: l'outbox riceve
/// `document.uploaded` nella stessa transazione dell'inserimento.
#[tokio::test]
async fn uploading_a_document_writes_the_event_in_the_same_transaction() {
    let env = require_db!();
    let context = register_owner(&env, "doc-evento").await;
    let owner = lease_owner(&env, &context, "Padova").await;
    let storage = Arc::new(MemoryStorage::default());
    let document = upload(&env, &storage, &context, owner, "con-evento.pdf").await;

    let row: (String, String, serde_json::Value) = sqlx::query_as(
        "SELECT event_type, aggregate_type, payload
           FROM app.domain_events
          WHERE aggregate_id = $1",
    )
    .bind(document.id().as_uuid())
    .fetch_one(env.pools.privileged())
    .await
    .expect("evento presente");

    assert_eq!(row.0, "document.uploaded");
    assert_eq!(row.1, "Document");
    // Regola 18: nessun dato personale nell'evento. Il nome del file è il dato
    // personale di questo aggregato, e non deve comparire nel payload.
    let payload = row.2.to_string();
    assert!(
        !payload.contains("con-evento"),
        "il nome del file è finito nell'evento: {payload}"
    );
    assert!(payload.contains("lease"), "payload: {payload}");
}

// ---------------------------------------------------------------------------
// Segnalazioni
// ---------------------------------------------------------------------------

/// Apre una segnalazione sull'immobile indicato e la restituisce.
async fn open_ticket(
    env: &common::TestEnv,
    context: &RequestContext,
    property_id: PropertyId,
    lease_id: Option<propflow_domain::shared::LeaseId>,
    title: &str,
) -> propflow_domain::ticket::Ticket {
    OpenTicket::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            context,
            OpenTicketCommand {
                property_id,
                room_id: None,
                lease_id,
                title: TicketTitle::parse(title).expect("titolo valido"),
                description: TicketDescription::parse("Descrizione di prova")
                    .expect("descrizione valida"),
                category: None,
                priority: None,
                visible_to_residents: None,
            },
        )
        .await
        .expect("segnalazione aperta")
        .ticket
}

/// Crea un immobile e restituisce il suo identificatore.
async fn a_property(env: &common::TestEnv, context: &RequestContext, city: &str) -> PropertyId {
    CreateProperty::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            context,
            CreatePropertyCommand {
                label: format!("Immobile {city}"),
                property_type: PropertyType::Apartment,
                address: address(city),
                status: None,
            },
        )
        .await
        .expect("immobile creato")
        .id()
}

/// Il riferimento lo assegna il **database**, non il dominio.
///
/// È l'unico identificatore del progetto che non nasce da `generate()`: la
/// `INSERT` non nomina la colonna, il `DEFAULT` attinge a
/// `app.ticket_reference_seq` e la `RETURNING` lo riporta indietro. Se qualcuno
/// tornasse a generarlo nell'applicazione, la sequenza avanzerebbe comunque e le
/// collisioni su `tickets_reference_uk` arriverebbero mesi dopo.
#[tokio::test]
async fn the_reference_comes_back_from_the_insert() {
    let env = require_db!();
    let context = register_owner(&env, "ticket-reference").await;
    let property = a_property(&env, &context, "Trento").await;

    let ticket = open_ticket(&env, &context, property, None, "Il boiler perde").await;

    let reference = ticket.reference().expect("assegnato dalla INSERT");
    assert!(
        reference.as_str().starts_with("TK-"),
        "riferimento inatteso: {reference}"
    );

    // E la riga riletta porta lo stesso riferimento: non è un valore inventato
    // dal caso d'uso e mai scritto.
    let mut uow = env.factory.begin(&context).await.expect("transazione");
    let read = uow
        .tickets()
        .find_by_id(ticket.id())
        .await
        .expect("query riuscita")
        .expect("visibile a chi l'ha aperta");
    assert_eq!(
        read.reference().map(TicketReference::as_str),
        Some(reference.as_str())
    );
}

/// Lo storico lo scrive un trigger, e lo scrive **già all'inserimento**.
///
/// `tg_ticket_status_history` scatta `AFTER INSERT OR UPDATE OF status`: la riga
/// `NULL → open` esiste senza che il caso d'uso ne aggiunga una. Se qualcuno la
/// scrivesse anche dall'applicazione, qui se ne vedrebbero due — e comunque
/// `ticket_status_history` non ha una policy di `INSERT`, quindi la scrittura
/// applicativa verrebbe rifiutata.
#[tokio::test]
async fn the_history_writes_itself_from_the_first_row() {
    let env = require_db!();
    let context = register_owner(&env, "ticket-storico").await;
    let property = a_property(&env, &context, "Ancona").await;
    let ticket = open_ticket(&env, &context, property, None, "Storico").await;

    let mut uow = env.factory.begin(&context).await.expect("transazione");
    let history = uow
        .tickets()
        .list_status_history(ticket.id())
        .await
        .expect("query riuscita");

    assert_eq!(history.len(), 1, "storico: {history:?}");
    assert_eq!(history[0].from_status(), None);
    assert_eq!(history[0].to_status(), TicketStatus::Open);
    assert_eq!(
        history[0].changed_by(),
        context.user_id(),
        "`changed_by` lo riempie app.current_user_id() dal contesto della richiesta"
    );
}

/// Il ciclo intero, con la riapertura, lascia nello storico la catena completa.
#[tokio::test]
async fn the_whole_cycle_leaves_its_trace_in_the_history() {
    let env = require_db!();
    let context = register_owner(&env, "ticket-ciclo").await;
    let property = a_property(&env, &context, "Perugia").await;
    let ticket = open_ticket(&env, &context, property, None, "Ciclo completo").await;
    let assignee = context.user_id().expect("utente");

    let factory = || Arc::clone(&env.factory);
    let clock = || Arc::clone(&env.clock) as Arc<dyn propflow_domain::ports::Clock>;

    TriageTicket::new(factory(), clock())
        .execute(
            &context,
            TriageTicketCommand {
                id: ticket.id(),
                category: TicketCategory::Plumbing,
                priority: TicketPriority::High,
            },
        )
        .await
        .expect("classificata");
    AssignTicket::new(factory(), clock())
        .execute(
            &context,
            AssignTicketCommand {
                id: ticket.id(),
                assignee,
            },
        )
        .await
        .expect("assegnata");
    StartTicketWork::new(factory(), clock())
        .execute(&context, StartTicketWorkCommand { id: ticket.id() })
        .await
        .expect("avviata");
    SuspendTicket::new(factory(), clock())
        .execute(
            &context,
            SuspendTicketCommand {
                id: ticket.id(),
                waiting_on: WaitingOn::Tenant,
            },
        )
        .await
        .expect("sospesa");
    StartTicketWork::new(factory(), clock())
        .execute(&context, StartTicketWorkCommand { id: ticket.id() })
        .await
        .expect("ripresa");
    let closed = CloseTicket::new(factory(), clock())
        .execute(
            &context,
            CloseTicketCommand {
                id: ticket.id(),
                resolution_notes: Some("sostituita la valvola".to_owned()),
            },
        )
        .await
        .expect("chiusa");
    assert!(closed.ticket.closed_at().is_some());
    assert_eq!(
        closed.ticket.resolution_notes(),
        Some("sostituita la valvola")
    );

    let reopened = ReopenTicket::new(factory(), clock())
        .execute(
            &context,
            ReopenTicketCommand {
                id: ticket.id(),
                reason: MessageBody::parse("Ha ricominciato a perdere").expect("motivo"),
            },
        )
        .await
        .expect("riaperta");
    assert_eq!(reopened.ticket.status(), TicketStatus::Open);
    // La trappola: `tickets_closed_ck` vincola una sola direzione, quindi il
    // database accetterebbe senza dire niente una segnalazione `open` con una data
    // di chiusura nel passato. Deve azzerarla il dominio, e la riga riletta lo
    // conferma — non solo l'aggregato in memoria.
    assert_eq!(reopened.ticket.closed_at(), None);

    let mut uow = env.factory.begin(&context).await.expect("transazione");
    let read = uow
        .tickets()
        .find_by_id(ticket.id())
        .await
        .expect("query")
        .expect("visibile");
    assert_eq!(read.closed_at(), None, "closed_at è rimasto in database");
    // Riaprire disfa la chiusura, e la nota di risoluzione fa parte della
    // chiusura: una segnalazione `open` che dichiara come è stata risolta
    // direbbe il falso, e la risposta la vede anche chi ha segnalato.
    assert_eq!(
        read.resolution_notes(),
        None,
        "la nota di risoluzione è sopravvissuta alla riapertura"
    );

    let history = uow
        .tickets()
        .list_status_history(ticket.id())
        .await
        .expect("query");
    let chain: Vec<TicketStatus> = history.iter().map(TicketStatusChange::to_status).collect();
    assert_eq!(
        chain,
        vec![
            TicketStatus::Open,
            TicketStatus::Triaged,
            TicketStatus::Assigned,
            TicketStatus::InProgress,
            TicketStatus::WaitingTenant,
            TicketStatus::InProgress,
            TicketStatus::Closed,
            TicketStatus::Open,
        ],
        "la catena degli stati non è quella percorsa"
    );
}

/// Le colonne strutturali non sono più scrivibili dal ruolo applicativo.
///
/// È la migration `0020`. Il privilegio è verificato **prima** delle policy,
/// quindi il rifiuto arriva come `permission denied` anche su una riga che non
/// esiste: non serve costruire il caso reale per provare che la porta è chiusa.
/// Il precedente è
/// `the_application_role_cannot_repoint_a_lease_party_to_another_lease`.
///
/// **`title` e `description` sono passate dai rifiuti alle concessioni con la
/// migration `0036`**, ed è la revoca di ADR-026 su quelle due sole colonne
/// (ADR-048). Il test le tiene qui e non le toglie: che siano scrivibili è
/// un'affermazione tanto quanto che non lo fossero, e chi togliesse la `GRANT`
/// per prudenza vedrebbe fallire questo test invece di scoprirlo dal client.
#[tokio::test]
async fn the_application_role_cannot_move_a_ticket_to_another_property() {
    let env = require_db!();

    for column in ["property_id", "room_id", "lease_id"] {
        let statement = match column {
            "property_id" => "UPDATE app.tickets SET property_id = $1 WHERE id = $2",
            "room_id" => "UPDATE app.tickets SET room_id = $1 WHERE id = $2",
            _ => "UPDATE app.tickets SET lease_id = $1 WHERE id = $2",
        };
        let error = sqlx::query(statement)
            .bind(uuid::Uuid::now_v7())
            .bind(uuid::Uuid::now_v7())
            .execute(env.pools.application())
            .await
            .expect_err("l'aggiornamento doveva essere negato");
        assert!(
            error.to_string().contains("permission denied"),
            "su `{column}` atteso un rifiuto per privilegi, ottenuto: {error}"
        );
    }

    // Nemmeno il riferimento, che lo assegna il trigger della `0021`, né chi ha
    // segnalato.
    for statement in [
        "UPDATE app.tickets SET reference = 'TK-999999' WHERE id = $1",
        "UPDATE app.tickets SET reported_by = NULL WHERE id = $1",
        "UPDATE app.tickets SET reported_by_kind = 'owner' WHERE id = $1",
        "UPDATE app.tickets SET currency = 'USD' WHERE id = $1",
    ] {
        let error = sqlx::query(statement)
            .bind(uuid::Uuid::now_v7())
            .execute(env.pools.application())
            .await
            .expect_err("l'aggiornamento doveva essere negato");
        assert!(
            error.to_string().contains("permission denied"),
            "`{statement}` doveva essere negato, ottenuto: {error}"
        );
    }

    // Ciò che è il lavoro dell'applicazione resta invece concesso: la restrizione
    // riguarda l'incoerenza strutturale, non l'operazione.
    for statement in [
        "UPDATE app.tickets SET status = 'closed', closed_at = now() WHERE id = $1",
        "UPDATE app.tickets SET actual_cost = 10.00 WHERE id = $1",
        "UPDATE app.tickets SET assigned_to = NULL WHERE id = $1",
        // Concessa dalla 0021: la chiusura registra come è stata risolta.
        "UPDATE app.tickets SET resolution_notes = 'fatto' WHERE id = $1",
        // Concessa dalla 0035: la visibilità è una scelta di chi apre, e si può
        // cambiare idea dopo.
        "UPDATE app.tickets SET visible_to_residents = true WHERE id = $1",
        // **Concesse dalla 0036, ed erano negate fino a quella migration.** È la
        // revoca di ADR-026 su queste due colonne sole, documentata in ADR-048:
        // un refuso nel titolo è un motivo legittimo di cambiare, e annullare e
        // riaprire costerebbe il riferimento già comunicato. Il limite «finché è
        // `open`» non è qui e non può esserci — una policy non confronta la riga
        // nuova con quella vecchia — ed è applicativo, come per `status`.
        "UPDATE app.tickets SET title = 'riscritto' WHERE id = $1",
        "UPDATE app.tickets SET description = 'riscritta' WHERE id = $1",
    ] {
        let allowed = sqlx::query(statement)
            .bind(uuid::Uuid::now_v7())
            .execute(env.pools.application())
            .await;
        assert!(
            allowed.is_ok(),
            "`{statement}` non doveva essere negato: {:?}",
            allowed.err()
        );
    }
}

/// Il contatore dei riferimenti non è manipolabile dal ruolo applicativo.
///
/// Lo scrive il trigger `tg_ticket_reference`, che è `SECURITY DEFINER`. Al ruolo
/// applicativo i privilegi predefiniti della 0015 lasciano `INSERT` e `UPDATE` a
/// livello di tabella, ma l'unica policy è di lettura: la Row Level Security
/// rifiuta l'inserimento e l'aggiornamento non trova righe. Senza questa
/// asimmetria un membro qualsiasi potrebbe riportare il contatore indietro e far
/// collidere il prossimo riferimento con uno già emesso.
#[tokio::test]
async fn the_reference_counter_is_written_by_the_trigger_and_by_nobody_else() {
    let env = require_db!();

    let insert = sqlx::query(
        "INSERT INTO app.ticket_reference_counters (org_id, last_number) VALUES ($1, 999)",
    )
    .bind(uuid::Uuid::now_v7())
    .execute(env.pools.application())
    .await;
    let error = insert.expect_err("l'inserimento doveva essere rifiutato");
    assert!(
        error.to_string().contains("row-level security"),
        "atteso un rifiuto della RLS, ottenuto: {error}"
    );

    // L'aggiornamento non fallisce: semplicemente non vede alcuna riga, perché
    // l'unica policy è di SELECT.
    let updated = sqlx::query("UPDATE app.ticket_reference_counters SET last_number = 0")
        .execute(env.pools.application())
        .await
        .expect("l'aggiornamento non deve dare errore");
    assert_eq!(
        updated.rows_affected(),
        0,
        "il ruolo applicativo ha riportato indietro un contatore"
    );
}

/// Il riferimento riparte da uno in ogni organizzazione.
///
/// Prima della `0021` la sequenza era una sola per il cluster, e il numero
/// diceva quante segnalazioni esistono sull'intera piattaforma.
#[tokio::test]
async fn the_reference_is_numbered_per_organization() {
    let env = require_db!();
    let alfa = register_owner(&env, "riferimento-alfa").await;
    let beta = register_owner(&env, "riferimento-beta").await;

    let mut emitted = Vec::new();
    for context in [&alfa, &beta] {
        let property = a_property(&env, context, "Numerazione").await;
        let first = open_ticket(&env, context, property, None, "La prima").await;
        let second = open_ticket(&env, context, property, None, "La seconda").await;
        emitted.push((
            first
                .reference()
                .map(TicketReference::as_str)
                .map(str::to_owned),
            second
                .reference()
                .map(TicketReference::as_str)
                .map(str::to_owned),
        ));
    }

    assert_eq!(
        emitted[0],
        (Some("TK-000001".to_owned()), Some("TK-000002".to_owned()))
    );
    assert_eq!(
        emitted[0], emitted[1],
        "due organizzazioni indipendenti devono numerare allo stesso modo"
    );
}

/// Una segnalazione di un'altra organizzazione non esiste.
///
/// La Row Level Security la nasconde, e il caso d'uso traduce l'assenza in `404`
/// invece di confermare che esiste ma non è sua.
#[tokio::test]
async fn a_ticket_of_another_organization_is_invisible() {
    let env = require_db!();
    let alfa = register_owner(&env, "ticket-alfa").await;
    let beta = register_owner(&env, "ticket-beta").await;

    let property = a_property(&env, &alfa, "Genova").await;
    let ticket = open_ticket(&env, &alfa, property, None, "Solo di Alfa").await;

    let error = GetTicket::new(Arc::clone(&env.factory))
        .execute(&beta, ticket.id())
        .await
        .expect_err("Beta non deve vederla");
    assert_eq!(error.code(), "ticket.not_found");

    let page = ListTickets::new(Arc::clone(&env.factory))
        .execute(&beta, ListTicketsQuery::default())
        .await
        .expect("elenco riuscito");
    assert!(
        page.items.is_empty(),
        "Beta vede {} segnalazioni",
        page.items.len()
    );
}

/// Una stanza o un contratto di un **altro** immobile vengono rifiutati dal
/// service layer, perché lo schema non li rifiuta.
///
/// Non esiste alcun vincolo fra `room_id`, `lease_id` e `property_id`: il trigger
/// `assert_org_matches_property` verifica soltanto l'`org_id`. Riprodotto su
/// database reale: l'inserimento diretto di una segnalazione con la stanza di un
/// altro immobile passa senza una parola.
#[tokio::test]
async fn a_ticket_refuses_a_room_or_a_lease_of_another_property() {
    let env = require_db!();
    let context = register_owner(&env, "ticket-coerenza").await;

    let target = a_property(&env, &context, "Livorno").await;
    let other = a_property(&env, &context, "Pisa").await;

    let room = CreateRoom::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &context,
            CreateRoomCommand {
                property_id: other,
                name: "Camera altrove".to_owned(),
                attributes: RoomChanges::default(),
            },
        )
        .await
        .expect("stanza creata");
    let lease = draft_lease(
        &env,
        &context,
        other,
        None,
        days_from_today(0),
        Some(days_from_today(365)),
    )
    .await;

    let open = OpenTicket::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _);
    let command = |room_id, lease_id| OpenTicketCommand {
        property_id: target,
        room_id,
        lease_id,
        title: TicketTitle::parse("Incoerente").expect("titolo"),
        description: TicketDescription::parse("Riferimenti di un altro immobile")
            .expect("descrizione"),
        category: None,
        priority: None,
        visible_to_residents: None,
    };

    let error = open
        .execute(&context, command(Some(room.id()), None))
        .await
        .expect_err("una stanza di un altro immobile va rifiutata");
    assert_eq!(error.code(), "validation.failed");

    let error = open
        .execute(&context, command(None, Some(lease.id())))
        .await
        .expect_err("un contratto di un altro immobile va rifiutato");
    assert_eq!(error.code(), "validation.failed");
}

/// Comando ed evento sono atomici, e l'evento **non trasporta il testo**.
///
/// Titolo e descrizione sono ciò che una persona ha scritto su un guasto in casa
/// propria: un evento pubblicato su un broker è un dato che viaggia e si conserva
/// (regola 18).
#[tokio::test]
async fn opening_a_ticket_writes_the_event_without_the_free_text() {
    let env = require_db!();
    let context = register_owner(&env, "ticket-evento").await;
    let property = a_property(&env, &context, "Rimini").await;
    let ticket = open_ticket(
        &env,
        &context,
        property,
        None,
        "Perdita nel bagno di Mario Rossi",
    )
    .await;

    let row: (String, String, serde_json::Value) = sqlx::query_as(
        "SELECT event_type, aggregate_type, payload
           FROM app.domain_events
          WHERE aggregate_id = $1",
    )
    .bind(ticket.id().as_uuid())
    .fetch_one(env.pools.privileged())
    .await
    .expect("evento presente");

    assert_eq!(row.0, "ticket.opened");
    assert_eq!(row.1, "Ticket");
    let payload = row.2.to_string();
    assert!(
        !payload.contains("Mario") && !payload.contains("bagno"),
        "il testo libero è finito nell'evento: {payload}"
    );
    assert!(payload.contains("other"), "payload: {payload}");
}

/// Un messaggio del thread non si modifica e non si cancella, nemmeno in modo
/// soft: `app.ticket_messages` ha solo `_select` e `_insert`.
///
/// Il test non verifica il nostro codice — che non offre nemmeno il metodo — ma
/// **il database**: se un giorno qualcuno aggiungesse una policy di `UPDATE`, la
/// proprietà su cui il thread si regge cambierebbe senza che nulla lo dica.
#[tokio::test]
async fn a_thread_message_can_never_be_rewritten() {
    let env = require_db!();

    let policies: Vec<(String,)> = sqlx::query_as(
        "SELECT polname FROM pg_policy
          WHERE polrelid = 'app.ticket_messages'::regclass
          ORDER BY polname",
    )
    .fetch_all(env.pools.privileged())
    .await
    .expect("policy leggibili");

    let names: Vec<&str> = policies.iter().map(|row| row.0.as_str()).collect();
    assert_eq!(
        names,
        vec!["ticket_messages_insert", "ticket_messages_select"],
        "una policy in più su ticket_messages cambia una proprietà del thread"
    );
}

// ---------------------------------------------------------------------------
// Bollette e addebiti
// ---------------------------------------------------------------------------

/// Registra una bolletta sull'immobile, con numero di fattura irripetibile.
///
/// Il numero è unico per invocazione perché `utility_bills_dedup_uk` è unico su
/// `(org_id, supplier_vat, invoice_number)` e i test girano sullo stesso database:
/// un numero fisso farebbe fallire il secondo test perché il primo è passato.
async fn register_bill(
    env: &common::TestEnv,
    context: &RequestContext,
    property_id: PropertyId,
    lease_id: Option<propflow_domain::shared::LeaseId>,
    invoice_number: &str,
) -> propflow_domain::billing::UtilityBill {
    RegisterUtilityBill::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            context,
            RegisterUtilityBillCommand {
                property_id,
                room_id: None,
                lease_id,
                utility_type: UtilityType::Gas,
                supplier_name: None,
                supplier_vat: Some(SupplierVat::parse("IT12345678901").expect("valida")),
                invoice_number: Some(InvoiceNumber::parse(invoice_number).expect("valido")),
                issue_date: Some(days_from_today(-30)),
                due_date: Some(days_from_today(10)),
                period_start: Some(days_from_today(-90)),
                period_end: Some(days_from_today(-31)),
                total_amount: "184.20".parse().expect("decimale"),
                taxable_amount: None,
                vat_amount: None,
                consumption: None,
                notes: None,
            },
        )
        .await
        .expect("bolletta registrata")
        // I casi d'uso restituiscono ora la bolletta **con i nomi dei luoghi che
        // cita**: qui interessa il solo aggregato.
        .value
}

/// Porta un inquilino dentro l'organizzazione come parte del contratto indicato,
/// e restituisce il suo contesto.
///
/// Passa dall'invito vero e non da una `INSERT` a mano: è il solo modo in cui la
/// membership e il collegamento della parte contrattuale esistono davvero, ed è da
/// quel collegamento che dipende tutto ciò che l'inquilino vede.
///
/// **Su un contratto attivo l'invito va a una parte che esiste** (ADR-056): il
/// giro è «si attiva, poi si invita», con la persona già scritta sul contratto.
/// L'helper prende quindi la prima parte ancora senza utente e senza email — il
/// segnaposto di [`draft_lease`] — e le dà l'email dell'inquilino. Prima invitava
/// un'email nuova anche a contratto attivo, e creava una seconda parte a quota
/// zero accanto al segnaposto al 100%: lo stato che ADR-046 chiama un difetto, su
/// cui quindici test si reggevano senza saperlo. Se non c'è un segnaposto libero
/// l'invito crea la parte, come in bozza.
async fn tenant_on_lease(
    env: &common::TestEnv,
    owner: &RequestContext,
    lease_id: propflow_domain::shared::LeaseId,
    label: &str,
) -> RequestContext {
    let tenant = register_owner(env, label).await;
    let tenant_email = {
        let mut uow = env
            .factory
            .begin(&tenant)
            .await
            .expect("transazione aperta");
        uow.users()
            .find_by_id(tenant.user_id().expect("utente"))
            .await
            .expect("query riuscita")
            .expect("utente visibile a sé stesso")
            .email()
            .clone()
    };

    let placeholder = {
        let mut uow = env.factory.begin(owner).await.expect("transazione aperta");
        uow.lease_parties()
            .list_for_lease(lease_id)
            .await
            .expect("parti del contratto")
            .into_iter()
            .find(|party| party.user_id().is_none() && party.email().is_none())
    };
    if let Some(placeholder) = placeholder {
        CorrectLeaseParty::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
            .execute(
                owner,
                CorrectLeasePartyCommand {
                    id: placeholder.id(),
                    changes: LeasePartyChanges {
                        email: Some(Some(tenant_email.clone())),
                        ..LeasePartyChanges::default()
                    },
                },
            )
            .await
            .expect("email sul segnaposto");
    }

    let issued = InviteTenant::new(
        Arc::clone(&env.factory),
        Arc::new(OpaqueTokenGenerator),
        Arc::clone(&env.clock) as _,
        Arc::new(SilentNotifier),
    )
    .execute(
        owner,
        InviteTenantCommand {
            lease_id,
            email: tenant_email,
            full_name: format!("Inquilino {label}"),
        },
    )
    .await
    .expect("invito emesso");

    let accepted = AcceptInvitation::new(
        Arc::clone(&env.factory),
        Arc::new(OpaqueTokenGenerator),
        Arc::clone(&env.clock) as _,
    )
    .execute(
        &tenant,
        AcceptInvitationCommand {
            token: issued.token,
        },
    )
    .await
    .expect("invito accettato");

    RequestContext::authenticated(tenant.user_id().expect("utente"))
        .with_org(accepted.org_id, OrgRole::Tenant)
}

/// Il giro completo del mese del proprietario, contro il database vero.
#[tokio::test]
async fn a_bill_travels_from_draft_to_a_charge_the_tenant_can_see() {
    let env = require_db!();
    let owner = register_owner(&env, "bolletta-giro").await;
    let property_id = create_property(&env, &owner, "Immobile con gas", "Bologna").await;
    let lease = draft_lease(
        &env,
        &owner,
        property_id,
        None,
        days_from_today(-10),
        Some(days_from_today(355)),
    )
    .await;
    activate(&env, &owner, lease.id()).await.expect("attivato");
    let tenant = tenant_on_lease(&env, &owner, lease.id(), "bolletta-inquilino").await;

    let bill = register_bill(&env, &owner, property_id, Some(lease.id()), "2026/A/1").await;
    assert_eq!(bill.status(), BillStatus::Draft);

    // Correggere una bozza la lascia in bozza.
    let corrected = UpdateUtilityBill::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            bill.id(),
            UtilityBillChanges {
                invoice_number: Some(Some(InvoiceNumber::parse("2026/A/2").expect("valido"))),
                ..UtilityBillChanges::default()
            },
        )
        .await
        .expect("correzione riuscita");
    assert_eq!(corrected.status(), BillStatus::Draft);
    assert_eq!(
        corrected.invoice_number().map(|n| n.as_str()),
        Some("2026/A/2")
    );

    let validated = ValidateUtilityBill::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&owner, bill.id())
        .await
        .expect("validata");
    assert_eq!(validated.status(), BillStatus::Validated);
    assert_eq!(validated.validated_by(), owner.user_id());
    assert!(validated.validated_at().is_some());

    let recharge = RechargeUtilityBill::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            bill.id(),
            RechargeUtilityBillCommand {
                lease_id: lease.id(),
                amount: "120.00".parse().expect("decimale"),
                due_date: days_from_today(10),
                description: Some("Gas, bimestre".to_owned()),
                notes: None,
            },
        )
        .await
        .expect("riaddebitata");

    assert_eq!(recharge.bill.status(), BillStatus::Allocated);
    assert_eq!(recharge.charge.status(), ChargeStatus::Issued);
    assert_eq!(recharge.charge.charge_type(), ChargeType::UtilityRecharge);
    assert_eq!(recharge.charge.utility_bill_id(), Some(bill.id()));
    assert!(
        recharge.charge.issued_at().is_some(),
        "`charges_issued_ck` rifiuterebbe l'inserimento senza la data"
    );
    // Il periodo dell'addebito è quello della bolletta, non uno dichiarato a parte.
    assert_eq!(recharge.charge.period_start(), bill.period_start());

    // L'inquilino vede **entrambi**, e il saldo dice che non ha ancora pagato.
    let seen = GetCharge::new(Arc::clone(&env.factory))
        .execute(&tenant, recharge.charge.id())
        .await
        .expect("l'inquilino vede l'addebito");
    assert_eq!(
        seen.paid_amount.amount(),
        rust_decimal::Decimal::ZERO,
        "senza righe in app.payments il saldo è per forza zero"
    );
    assert_eq!(seen.outstanding_amount, seen.charge.amount());
    assert_eq!(seen.effective_status(), ChargeStatus::Issued);

    let bills = ListUtilityBills::new(Arc::clone(&env.factory))
        .execute(&tenant, ListUtilityBillsQuery::default())
        .await
        .expect("elenco riuscito");
    assert!(
        bills.items.iter().any(|item| item.id() == bill.id()),
        "l'inquilino non vede la bolletta imputata al proprio contratto"
    );
}

/// Lo stato scaduto **si deriva**: la vista lo calcola, la colonna resta `issued`.
#[tokio::test]
async fn an_overdue_charge_keeps_its_stored_state() {
    let env = require_db!();
    let owner = register_owner(&env, "addebito-scaduto").await;
    let property_id = create_property(&env, &owner, "Immobile scadenze", "Padova").await;
    let lease = draft_lease(
        &env,
        &owner,
        property_id,
        None,
        days_from_today(-30),
        Some(days_from_today(335)),
    )
    .await;

    let charge = IssueCharge::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            IssueChargeCommand {
                lease_id: lease.id(),
                charge_type: ChargeType::Rent,
                description: Some("Canone arretrato".to_owned()),
                period_start: None,
                period_end: None,
                amount: "800.00".parse().expect("decimale"),
                due_date: days_from_today(-5),
                notes: None,
            },
        )
        .await
        .expect("addebito emesso");

    let balance = GetCharge::new(Arc::clone(&env.factory))
        .execute(&owner, charge.id())
        .await
        .expect("saldo letto");

    assert!(
        balance.is_past_due,
        "la vista deve calcolare `is_past_due` su una scadenza nel passato"
    );
    assert_eq!(balance.effective_status(), ChargeStatus::Overdue);
    assert_eq!(
        balance.charge.status(),
        ChargeStatus::Issued,
        "lo stato memorizzato non cambia da sé: è ADR-019 applicato agli addebiti"
    );

    // E il dominio risponde la stessa cosa senza database, che è l'obbligo della
    // coppia `app.lease_in_force` / `Lease::is_in_force`.
    assert!(
        balance
            .charge
            .is_past_due(chrono::Utc::now().date_naive(), rust_decimal::Decimal::ZERO),
        "dominio e vista devono rispondere la stessa cosa"
    );

    // Il filtro sull'insoluto lo trova; quello sullo stato derivato è rifiutato dal
    // service layer prima di arrivare al database.
    let outstanding = ListCharges::new(Arc::clone(&env.factory))
        .execute(
            &owner,
            ListChargesQuery {
                outstanding: Some(true),
                ..ListChargesQuery::default()
            },
        )
        .await
        .expect("elenco riuscito");
    assert!(
        outstanding
            .items
            .iter()
            .any(|item| item.charge.id() == charge.id())
    );

    let refused = ListCharges::new(Arc::clone(&env.factory))
        .execute(
            &owner,
            ListChargesQuery {
                status: Some(ChargeStatus::Overdue),
                ..ListChargesQuery::default()
            },
        )
        .await;
    assert!(
        matches!(
            refused,
            Err(propflow_application::AppError::UnsupportedQuery(_))
        ),
        "`?status=overdue` deve dire come si fa, non restituire una pagina vuota"
    );
}

/// L'asimmetria fra i due annullamenti, verificata su database vero.
#[tokio::test]
async fn voiding_a_charge_keeps_it_visible_and_voiding_a_bill_frees_the_invoice_number() {
    let env = require_db!();
    let owner = register_owner(&env, "annullamenti").await;
    let property_id = create_property(&env, &owner, "Immobile annulli", "Genova").await;
    let lease = draft_lease(
        &env,
        &owner,
        property_id,
        None,
        days_from_today(-10),
        Some(days_from_today(355)),
    )
    .await;
    activate(&env, &owner, lease.id()).await.expect("attivato");
    let tenant = tenant_on_lease(&env, &owner, lease.id(), "annulli-inquilino").await;

    let charge = IssueCharge::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            IssueChargeCommand {
                lease_id: lease.id(),
                charge_type: ChargeType::Rent,
                description: None,
                period_start: None,
                period_end: None,
                amount: "800.00".parse().expect("decimale"),
                due_date: days_from_today(15),
                notes: None,
            },
        )
        .await
        .expect("emesso");

    VoidCharge::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&owner, charge.id())
        .await
        .expect("annullato");

    // **Resta visibile all'inquilino**: si è visto chiedere quella somma e deve
    // poter vedere che la richiesta è caduta.
    let seen = GetCharge::new(Arc::clone(&env.factory))
        .execute(&tenant, charge.id())
        .await
        .expect("l'addebito annullato resta leggibile");
    assert_eq!(seen.charge.status(), ChargeStatus::Void);
    assert_eq!(
        seen.effective_status(),
        ChargeStatus::Void,
        "un addebito annullato non diventa scaduto col passare del tempo"
    );

    // La bolletta invece sparisce, e libera la chiave di deduplica.
    let bill = register_bill(&env, &owner, property_id, Some(lease.id()), "2026/B/1").await;
    VoidUtilityBill::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&owner, bill.id())
        .await
        .expect("annullata");

    let bills = ListUtilityBills::new(Arc::clone(&env.factory))
        .execute(&owner, ListUtilityBillsQuery::default())
        .await
        .expect("elenco riuscito");
    assert!(
        !bills.items.iter().any(|item| item.id() == bill.id()),
        "una bolletta annullata deve sparire: il soft delete è ciò che libera la chiave"
    );

    // E la stessa fattura si reinserisce, che è la ragione tecnica del soft delete.
    let reinserted = register_bill(&env, &owner, property_id, Some(lease.id()), "2026/B/1").await;
    assert_ne!(reinserted.id(), bill.id());
}

/// La deduplica scatta e arriva come conflitto **parlante**, non come «vincolo
/// violato».
#[tokio::test]
async fn the_same_invoice_from_the_same_supplier_is_refused_once() {
    let env = require_db!();
    let owner = register_owner(&env, "deduplica").await;
    let property_id = create_property(&env, &owner, "Immobile doppioni", "Verona").await;

    register_bill(&env, &owner, property_id, None, "2026/C/1").await;

    let second = RegisterUtilityBill::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            RegisterUtilityBillCommand {
                property_id,
                room_id: None,
                lease_id: None,
                utility_type: UtilityType::Gas,
                supplier_name: None,
                supplier_vat: Some(SupplierVat::parse("IT12345678901").expect("valida")),
                invoice_number: Some(InvoiceNumber::parse("2026/C/1").expect("valido")),
                issue_date: None,
                due_date: None,
                period_start: None,
                period_end: None,
                total_amount: "10.00".parse().expect("decimale"),
                taxable_amount: None,
                vat_amount: None,
                consumption: None,
                notes: None,
            },
        )
        .await;

    assert!(
        matches!(
            second,
            Err(propflow_application::AppError::Conflict(
                propflow_application::Conflict::DuplicateUtilityBill
            ))
        ),
        "atteso un conflitto nominato, ottenuto {second:?}"
    );
}

/// La **riga fantasma**: `charges_insert` non chiama `can_access_lease`, quindi
/// senza il controllo applicativo si scriverebbe un addebito invisibile a tutti e
/// impossibile da cancellare — il ruolo applicativo non ha `DELETE`.
///
/// Il test verifica che lo use case si fermi prima: il contratto dell'altra
/// organizzazione, letto sotto la RLS di chi chiede, semplicemente non si trova.
#[tokio::test]
async fn a_charge_cannot_be_issued_against_a_lease_of_another_organization() {
    let env = require_db!();
    let stranger = register_owner(&env, "fantasma-estraneo").await;
    let stranger_property = create_property(&env, &stranger, "Immobile altrui", "Palermo").await;
    let stranger_lease = draft_lease(
        &env,
        &stranger,
        stranger_property,
        None,
        days_from_today(0),
        Some(days_from_today(365)),
    )
    .await;

    // Chi attacca conosce l'UUID del contratto altrui e usa il **proprio** org_id:
    // la `WITH CHECK` di `charges_insert` nomina l'organizzazione e non il
    // contratto, e la foreign key verso `app.leases` non guarda l'organizzazione.
    let attacker = register_owner(&env, "fantasma-attaccante").await;
    let attempt = IssueCharge::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &attacker,
            IssueChargeCommand {
                lease_id: stranger_lease.id(),
                charge_type: ChargeType::Rent,
                description: None,
                period_start: None,
                period_end: None,
                amount: "1.00".parse().expect("decimale"),
                due_date: days_from_today(30),
                notes: None,
            },
        )
        .await;

    assert!(
        matches!(
            attempt,
            Err(propflow_application::AppError::NotFound(
                propflow_domain::Entity::Lease
            ))
        ),
        "senza questo controllo la riga entrerebbe e non sarebbe più leggibile né \
         cancellabile: ottenuto {attempt:?}"
    );

    // E nulla è stato scritto: la prova che il controllo precede la scrittura.
    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM app.charges WHERE lease_id = $1")
        .bind(stranger_lease.id().as_uuid())
        .fetch_one(env.pools.privileged())
        .await
        .expect("conteggio riuscito");
    assert_eq!(count.0, 0, "una riga fantasma è finita in app.charges");
}

/// I privilegi di colonna della migration `0023`, verificati **al contrario**.
///
/// Il privilegio è controllato prima delle policy e prima che le righe vengano
/// esaminate, quindi il tentativo fallisce anche su un identificatore inesistente:
/// il test non ha bisogno di costruire lo scenario per dimostrare che la strada è
/// chiusa.
#[tokio::test]
async fn the_application_role_cannot_rewrite_a_claim_it_has_already_made() {
    let env = require_db!();

    // Le query sono letterali statici anche qui: `sqlx` 0.9 rifiuta le stringhe
    // composte a runtime, ed è la stessa regola che vale nel codice di produzione
    // (`CLAUDE.md` §2.15). Comporle con `format!` per brevità non compilerebbe.
    let refused: [(&str, &'static str); 5] = [
        (
            "amount",
            "UPDATE app.charges SET amount = 1.00 WHERE id = gen_random_uuid()",
        ),
        (
            "due_date",
            "UPDATE app.charges SET due_date = CURRENT_DATE WHERE id = gen_random_uuid()",
        ),
        (
            "lease_id",
            "UPDATE app.charges SET lease_id = gen_random_uuid() WHERE id = gen_random_uuid()",
        ),
        (
            "description",
            "UPDATE app.charges SET description = 'altro' WHERE id = gen_random_uuid()",
        ),
        (
            "deleted_at",
            "UPDATE app.charges SET deleted_at = now() WHERE id = gen_random_uuid()",
        ),
    ];
    for (column, statement) in refused {
        let attempt = sqlx::query(statement)
            .execute(env.pools.application())
            .await;
        let error = attempt.expect_err("l'aggiornamento doveva essere negato");
        assert!(
            error.to_string().contains("permission denied"),
            "`{column}` doveva essere negata, ottenuto: {error}"
        );
    }

    // `status` invece resta concessa: è ciò che l'applicazione scrive, ed è tutto
    // ciò di cui avrà bisogno anche il modulo dei pagamenti.
    let allowed =
        sqlx::query("UPDATE app.charges SET status = 'void' WHERE id = gen_random_uuid()")
            .execute(env.pools.application())
            .await;
    assert!(
        allowed.is_ok(),
        "l'aggiornamento di status non doveva essere negato: {:?}",
        allowed.err()
    );
}

/// L'isolamento fra organizzazioni sui due aggregati nuovi.
#[tokio::test]
async fn bills_and_charges_are_isolated_between_organizations() {
    let env = require_db!();
    let owner = register_owner(&env, "isolamento-contabile").await;
    let property_id = create_property(&env, &owner, "Immobile riservato", "Torino").await;
    let lease = draft_lease(
        &env,
        &owner,
        property_id,
        None,
        days_from_today(0),
        Some(days_from_today(365)),
    )
    .await;
    let bill = register_bill(&env, &owner, property_id, Some(lease.id()), "2026/D/1").await;
    let charge = IssueCharge::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            IssueChargeCommand {
                lease_id: lease.id(),
                charge_type: ChargeType::Deposit,
                description: None,
                period_start: None,
                period_end: None,
                amount: "1600.00".parse().expect("decimale"),
                due_date: days_from_today(7),
                notes: None,
            },
        )
        .await
        .expect("emesso");

    let stranger = register_owner(&env, "isolamento-estraneo").await;

    let mut uow = env
        .factory
        .begin(&stranger)
        .await
        .expect("transazione aperta");
    assert!(
        uow.utility_bills()
            .find_by_id(bill.id())
            .await
            .expect("query riuscita")
            .is_none(),
        "una bolletta di un'altra organizzazione non deve essere leggibile"
    );
    assert!(
        uow.charges()
            .find_by_id(charge.id())
            .await
            .expect("query riuscita")
            .is_none()
    );
    assert!(
        uow.charges()
            .find_balance_by_id(charge.id())
            .await
            .expect("query riuscita")
            .is_none(),
        "la vista dei saldi è security_invoker: applica la RLS del chiamante"
    );
}

/// La correzione di una bolletta validata **disfa la validazione**, e la scrittura
/// arriva davvero in database — non solo nell'aggregato in memoria.
#[tokio::test]
async fn correcting_a_validated_bill_sends_it_back_to_draft_in_the_database() {
    let env = require_db!();
    let owner = register_owner(&env, "disfa-validazione").await;
    let property_id = create_property(&env, &owner, "Immobile revisioni", "Ancona").await;
    let bill = register_bill(&env, &owner, property_id, None, "2026/E/1").await;

    ValidateUtilityBill::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&owner, bill.id())
        .await
        .expect("validata");

    UpdateUtilityBill::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            bill.id(),
            UtilityBillChanges {
                total_amount: Some("999.99".parse().expect("decimale")),
                ..UtilityBillChanges::default()
            },
        )
        .await
        .expect("corretta");

    // Riletta dal database: è il punto del test. `utility_bills_validated_ck`
    // vincola una sola direzione, quindi il database accetterebbe senza dire niente
    // una bolletta `draft` con `validated_at` valorizzato.
    let row: (
        String,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<uuid::Uuid>,
    ) = sqlx::query_as(
        "SELECT status::text, validated_at, validated_by
               FROM app.utility_bills WHERE id = $1",
    )
    .bind(bill.id().as_uuid())
    .fetch_one(env.pools.privileged())
    .await
    .expect("riga presente");

    assert_eq!(row.0, "draft");
    assert_eq!(
        row.1, None,
        "una bozza con la data di validazione è una riga che mente"
    );
    assert_eq!(row.2, None);
}

/// In un co-living con contratti per stanza, il coinquilino **non** scarica la
/// bolletta imputata al contratto dell'altro.
///
/// È la prova che la migration `0022` serviva: prima, `can_access_attachment`
/// risolveva `utility_bill` su `can_access_property`, vera per *ogni* inquilino
/// dell'immobile, mentre `utility_bills_select` chiede di essere parte del
/// contratto indicato. Il coinquilino non vedeva la riga della bolletta e ne
/// avrebbe scaricato la scansione — con nome, indirizzo e consumi dell'altro.
#[tokio::test]
async fn a_cotenant_cannot_reach_the_bill_of_the_other_room() {
    let env = require_db!();
    let owner = register_owner(&env, "coliving-bollette").await;
    let property_id = create_property(&env, &owner, "Co-living", "Milano").await;

    let room_a = CreateRoom::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            CreateRoomCommand {
                property_id,
                name: "Stanza A".to_owned(),
                attributes: RoomChanges::default(),
            },
        )
        .await
        .expect("stanza A");
    let room_b = CreateRoom::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            CreateRoomCommand {
                property_id,
                name: "Stanza B".to_owned(),
                attributes: RoomChanges::default(),
            },
        )
        .await
        .expect("stanza B");

    let lease_a = draft_lease(
        &env,
        &owner,
        property_id,
        Some(room_a.id()),
        days_from_today(-10),
        Some(days_from_today(355)),
    )
    .await;
    let lease_b = draft_lease(
        &env,
        &owner,
        property_id,
        Some(room_b.id()),
        days_from_today(-10),
        Some(days_from_today(355)),
    )
    .await;
    activate(&env, &owner, lease_a.id())
        .await
        .expect("A attivo");
    activate(&env, &owner, lease_b.id())
        .await
        .expect("B attivo");

    let tenant_a = tenant_on_lease(&env, &owner, lease_a.id(), "coliving-a").await;
    let _tenant_b = tenant_on_lease(&env, &owner, lease_b.id(), "coliving-b").await;

    // La bolletta è imputata al contratto di B.
    let bill_b = register_bill(&env, &owner, property_id, Some(lease_b.id()), "2026/F/1").await;

    // A non vede la riga: è la policy, e questo funzionava già.
    let mut uow = env
        .factory
        .begin(&tenant_a)
        .await
        .expect("transazione aperta");
    assert!(
        uow.utility_bills()
            .find_by_id(bill_b.id())
            .await
            .expect("query riuscita")
            .is_none(),
        "la policy della bolletta non lascia vedere quella dell'altro contratto"
    );
    drop(uow);

    // Il caso d'uso lo ferma: `ensure_owner_is_visible` rilegge la bolletta sotto
    // la policy, e per A non c'è.
    let listing = ListDocuments::new(Arc::clone(&env.factory))
        .execute(
            &tenant_a,
            ListDocumentsCommand {
                owner: AttachmentOwner::UtilityBill(bill_b.id()),
                after: None,
                limit: None,
            },
        )
        .await;
    assert!(
        matches!(
            listing,
            Err(propflow_application::AppError::NotFound(
                propflow_domain::Entity::UtilityBill
            ))
        ),
        "il coinquilino non deve raggiungere gli allegati della bolletta altrui: \
         ottenuto {listing:?}"
    );

    // **E lo ferma anche lo strato sotto**, che è il punto di questo test.
    //
    // Il controllo applicativo qui sopra passerebbe *comunque*, anche con la
    // funzione sbagliata: maschera il difetto invece di dimostrarlo — verificato
    // rimettendo la definizione della `0014` e vedendo il test restare verde. Per
    // sapere se la migration `0022` serviva davvero bisogna interrogare
    // `can_access_attachment` **direttamente, sotto il contesto RLS di A**, che è
    // ciò che fa la policy `documents_select` quando l'applicazione non c'è di
    // mezzo.
    //
    // Con la definizione originale questa asserzione fallisce: il ramo
    // `utility_bill` risolveva su `can_access_property`, vera per ogni inquilino
    // dell'immobile. È la difesa in profondità che ADR-016 chiede, e senza questa
    // riga non sapremmo di averla.
    let mut connection = env
        .pools
        .application()
        .begin()
        .await
        .expect("transazione aperta");
    sqlx::query("SELECT app.set_request_context($1, $2, false, NULL, NULL, NULL, $3)")
        .bind(tenant_a.user_id().expect("utente").as_uuid())
        .bind(tenant_a.org_id().expect("org").as_uuid())
        .bind("tenant")
        .execute(&mut *connection)
        .await
        .expect("contesto installato");

    let reachable: (bool,) = sqlx::query_as("SELECT app.can_access_attachment('utility_bill', $1)")
        .bind(bill_b.id().as_uuid())
        .fetch_one(&mut *connection)
        .await
        .expect("funzione interrogabile");
    assert!(
        !reachable.0,
        "can_access_attachment lascia il coinquilino raggiungere la bolletta \
         dell'altro contratto: è il difetto che la migration 0022 corregge"
    );

    // Il ramo `charge` era invece già allineato a `charges_select`, e il test lo
    // fissa: riscriverlo per simmetria avrebbe toccato una riga giusta.
    let charge_b = IssueCharge::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            IssueChargeCommand {
                lease_id: lease_b.id(),
                charge_type: ChargeType::Rent,
                description: None,
                period_start: None,
                period_end: None,
                amount: "500.00".parse().expect("decimale"),
                due_date: days_from_today(20),
                notes: None,
            },
        )
        .await
        .expect("emesso");

    let charge_reachable: (bool,) =
        sqlx::query_as("SELECT app.can_access_attachment('charge', $1)")
            .bind(charge_b.id().as_uuid())
            .fetch_one(&mut *connection)
            .await
            .expect("funzione interrogabile");
    assert!(
        !charge_reachable.0,
        "il ramo `charge` risolve su can_access_lease e non deve lasciar passare A"
    );
}

/// L'evento della bolletta non porta né importi né dati del fornitore.
#[tokio::test]
async fn registering_a_bill_writes_the_event_without_the_supplier_or_the_amount() {
    let env = require_db!();
    let owner = register_owner(&env, "bolletta-evento").await;
    let property_id = create_property(&env, &owner, "Immobile eventi", "Lecce").await;
    let bill = register_bill(&env, &owner, property_id, None, "2026/G/99887").await;

    let row: (String, String, serde_json::Value) = sqlx::query_as(
        "SELECT event_type, aggregate_type, payload
           FROM app.domain_events
          WHERE aggregate_id = $1",
    )
    .bind(bill.id().as_uuid())
    .fetch_one(env.pools.privileged())
    .await
    .expect("evento presente");

    assert_eq!(row.0, "billing.utility_bill.registered");
    assert_eq!(row.1, "UtilityBill");
    let payload = row.2.to_string();
    assert!(
        !payload.contains("184.20"),
        "l'importo è finito nell'evento: {payload}"
    );
    assert!(
        !payload.contains("IT12345678901") && !payload.contains("99887"),
        "il fornitore o il numero di fattura sono finiti nell'evento: {payload}"
    );
    assert!(payload.contains("gas"), "payload: {payload}");
}

// ---------------------------------------------------------------------------
// Pagamenti
// ---------------------------------------------------------------------------

/// Emette un addebito e ne restituisce l'identificatore.
async fn issue_charge(
    env: &common::TestEnv,
    owner: &RequestContext,
    lease_id: propflow_domain::shared::LeaseId,
    amount: &str,
    due_in_days: i64,
) -> propflow_domain::billing::Charge {
    IssueCharge::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            owner,
            IssueChargeCommand {
                lease_id,
                charge_type: ChargeType::Rent,
                description: Some(format!("Canone da {amount}")),
                period_start: None,
                period_end: None,
                amount: amount.parse().expect("decimale"),
                due_date: days_from_today(due_in_days),
                notes: None,
            },
        )
        .await
        .expect("addebito emesso")
        .value
}

fn key(label: &str) -> IdempotencyKey {
    IdempotencyKey::parse(&format!("{label}-{}", uuid::Uuid::now_v7())).expect("chiave valida")
}

/// **Il difetto delle viste, verificato al contrario.**
///
/// È il caso già documentato fra i debiti e in ADR-028, riprodotto su database vero
/// prima della migration `0024`: 800.00 emesso, 120.00 **annullato**, 40.00 scaduto
/// davano `total_outstanding = 960.00`, dove il numero vero è 840.00 —
/// all'inquilino era stato detto che quei 120 euro non li doveva più, e il numero di
/// testa della dashboard continuava a contarli come credito.
///
/// Una vista sola rimette a posto due numeri: `v_owner_dashboard` filtra già
/// `outstanding_amount > 0` e conta su `is_past_due`, quindi non è stata toccata.
#[tokio::test]
async fn a_dropped_claim_no_longer_counts_as_outstanding() {
    let env = require_db!();
    let owner = register_owner(&env, "saldo-vista").await;
    let property_id = create_property(&env, &owner, "Immobile dei saldi", "Genova").await;
    let lease = draft_lease(
        &env,
        &owner,
        property_id,
        None,
        days_from_today(-60),
        Some(days_from_today(305)),
    )
    .await;

    // L'annullato scade **nel passato**, e non è un dettaglio dello scenario: con
    // una scadenza futura `is_past_due` sarebbe falso anche con la vista vecchia, e
    // né quell'assert né `overdue_charges` distinguerebbero le due definizioni. Con
    // la scadenza passata la vista vecchia risponde 960.00 e **due** scaduti, la
    // nuova 840.00 e uno.
    let outstanding = issue_charge(&env, &owner, lease.id(), "800.00", 30).await;
    let to_void = issue_charge(&env, &owner, lease.id(), "120.00", -3).await;
    let overdue = issue_charge(&env, &owner, lease.id(), "40.00", -5).await;

    VoidCharge::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&owner, to_void.id())
        .await
        .expect("annullato");

    // La riga **resta**, ed è il punto: `find_balance_by_id` serve il dettaglio di
    // un addebito annullato, che l'inquilino deve poter vedere. Togliere le righe
    // dalla vista sarebbe stata la via ovvia e sbagliata.
    let voided_balance = GetCharge::new(Arc::clone(&env.factory))
        .execute(&owner, to_void.id())
        .await
        .expect("l'addebito annullato resta leggibile");
    assert_eq!(voided_balance.charge.status(), ChargeStatus::Void);
    assert_eq!(
        voided_balance.outstanding_amount.amount(),
        rust_decimal::Decimal::ZERO,
        "la pretesa è caduta: non è dovuto niente"
    );
    assert!(
        !voided_balance.is_past_due,
        "un addebito annullato non è scaduto, per quanto la data sia passata"
    );

    // E i due numeri della dashboard, letti dalla vista che li produce.
    let org_id = owner.org_id().expect("org").as_uuid();
    let (total, overdue_count): (rust_decimal::Decimal, i64) = sqlx::query_as(
        "SELECT total_outstanding::numeric, overdue_charges
           FROM app.v_owner_dashboard WHERE org_id = $1",
    )
    .bind(org_id)
    .fetch_one(env.pools.privileged())
    .await
    .expect("dashboard letta");

    assert_eq!(
        total,
        "840.00".parse::<rust_decimal::Decimal>().expect("decimale"),
        "prima della 0024 questo numero era 960.00: contava anche l'annullato"
    );
    assert_eq!(
        overdue_count, 1,
        "prima della 0024 erano due: l'annullato era scaduto quanto l'altro"
    );

    // I due che restano sono quelli attesi.
    assert!(outstanding.amount().amount() > rust_decimal::Decimal::ZERO);
    assert!(overdue.due_date() < chrono::Utc::now().date_naive());
}

/// Il giro a mano del modulo, contro il binario vero.
///
/// La dichiarazione **non** muove il saldo — è ciò che il flusso in due tempi
/// compra — e la conferma lo azzera portando l'addebito a `paid`.
#[tokio::test]
async fn a_declaration_does_not_move_the_balance_and_the_confirmation_does() {
    let env = require_db!();
    let owner = register_owner(&env, "pagamento-giro").await;
    let property_id = create_property(&env, &owner, "Immobile incassi", "Verona").await;
    let lease = draft_lease(
        &env,
        &owner,
        property_id,
        None,
        days_from_today(-10),
        Some(days_from_today(355)),
    )
    .await;
    activate(&env, &owner, lease.id()).await.expect("attivato");
    let tenant = tenant_on_lease(&env, &owner, lease.id(), "pagamento-inquilino").await;

    let charge = issue_charge(&env, &owner, lease.id(), "800.00", 20).await;

    let declared = DeclarePayment::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &tenant,
            DeclarePaymentCommand {
                charge_id: charge.id(),
                amount: "800.00".parse().expect("decimale"),
                method: PaymentMethod::BankTransfer,
                paid_at: None,
                reference: Some("Bonifico del 3".to_owned()),
                idempotency_key: key("dichiarazione"),
            },
        )
        .await
        .expect("dichiarazione accettata");
    assert!(!declared.replayed);
    assert_eq!(declared.payment.status(), PaymentStatus::Pending);
    assert_eq!(declared.payment.payer_user_id(), tenant.user_id());

    // Il saldo **non** si è mosso, e lo stato memorizzato nemmeno.
    let before = GetCharge::new(Arc::clone(&env.factory))
        .execute(&owner, charge.id())
        .await
        .expect("saldo letto");
    assert_eq!(before.paid_amount.amount(), rust_decimal::Decimal::ZERO);
    assert_eq!(before.outstanding_amount, charge.amount());
    assert_eq!(before.charge.status(), ChargeStatus::Issued);

    let confirmed = ConfirmPayment::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&owner, declared.payment.id())
        .await
        .expect("confermato");
    assert_eq!(confirmed.status(), PaymentStatus::Succeeded);
    assert!(
        confirmed.paid_at().is_some(),
        "`payments_succeeded_ck` rifiuterebbe la scrittura senza la data"
    );
    assert!(confirmed.reconciled_at().is_some());

    let after = GetCharge::new(Arc::clone(&env.factory))
        .execute(&tenant, charge.id())
        .await
        .expect("l'inquilino vede il proprio saldo");
    assert_eq!(after.paid_amount, charge.amount());
    assert_eq!(
        after.outstanding_amount.amount(),
        rust_decimal::Decimal::ZERO
    );
    assert_eq!(
        after.charge.status(),
        ChargeStatus::Paid,
        "lo stato si ricalcola dai pagamenti: è la prima scrittura che la GRANT della 0023 concede"
    );
    assert_eq!(after.effective_status(), ChargeStatus::Paid);

    // L'inquilino vede il proprio pagamento nell'elenco dell'addebito.
    let listed = ListPayments::new(Arc::clone(&env.factory))
        .execute(
            &tenant,
            ListPaymentsQuery {
                charge_id: Some(charge.id()),
                ..ListPaymentsQuery::default()
            },
        )
        .await
        .expect("elenco riuscito");
    assert_eq!(listed.items.len(), 1);
    assert_eq!(listed.items[0].id(), declared.payment.id());
}

/// Il predicato ridondante che rende utilizzabile `payments_pending_ix` **non
/// seleziona nulla per conto proprio**.
///
/// Esiste solo per il planner: il repository lo accende quando il filtro chiede
/// `pending` e lo spegne altrimenti, e su ogni altro stato deve restare vero per
/// tutte le righe. Se si accendesse per sbaglio, un elenco filtrato su `succeeded`
/// risponderebbe **zero righe senza alcun errore** — che è il modo peggiore di
/// sbagliare, perché sembra «non ci sono pagamenti».
///
/// Va sulla persistenza e non sull'unità perché è una proprietà della query, non
/// della condizione: a girare deve essere il SQL vero, contro il database vero.
#[tokio::test]
async fn the_redundant_predicate_selects_nothing_of_its_own() {
    let env = require_db!();
    let owner = register_owner(&env, "predicato-ridondante").await;
    let property_id = create_property(&env, &owner, "Immobile della coda", "Bologna").await;
    let lease = draft_lease(
        &env,
        &owner,
        property_id,
        None,
        days_from_today(-10),
        Some(days_from_today(355)),
    )
    .await;
    let charge = issue_charge(&env, &owner, lease.id(), "800.00", 20).await;

    // Un incasso `succeeded`, registrato dallo staff.
    RecordPayment::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            RecordPaymentCommand {
                charge_id: charge.id(),
                amount: "300.00".parse().expect("importo"),
                method: PaymentMethod::BankTransfer,
                paid_at: None,
                reference: None,
                payer_user_id: None,
                idempotency_key: key("coda-succeeded"),
            },
        )
        .await
        .expect("incasso registrato");

    let list = ListPayments::new(Arc::clone(&env.factory));

    let succeeded = list
        .execute(
            &owner,
            ListPaymentsQuery {
                status: Some(PaymentStatus::Succeeded),
                ..ListPaymentsQuery::default()
            },
        )
        .await
        .expect("elenco riuscito");
    assert_eq!(
        succeeded.items.len(),
        1,
        "il ramo `pending` è spento e non deve togliere righe"
    );

    let pending = list
        .execute(
            &owner,
            ListPaymentsQuery {
                status: Some(PaymentStatus::Pending),
                ..ListPaymentsQuery::default()
            },
        )
        .await
        .expect("elenco riuscito");
    assert!(
        pending.items.is_empty(),
        "un incasso registrato nasce `succeeded`, non entra nella coda"
    );

    let everything = list
        .execute(&owner, ListPaymentsQuery::default())
        .await
        .expect("elenco riuscito");
    assert_eq!(everything.items.len(), 1, "senza filtro ci sono tutti");
}

/// **La decisione più importante della fetta, verificata**: un secondo invio con la
/// stessa chiave restituisce il pagamento di prima e **non** fa scendere il saldo
/// una seconda volta.
#[tokio::test]
async fn the_same_idempotency_key_never_takes_the_money_twice() {
    let env = require_db!();
    let owner = register_owner(&env, "idempotenza").await;
    let property_id = create_property(&env, &owner, "Immobile idempotente", "Trieste").await;
    let lease = draft_lease(
        &env,
        &owner,
        property_id,
        None,
        days_from_today(-10),
        Some(days_from_today(355)),
    )
    .await;
    let charge = issue_charge(&env, &owner, lease.id(), "800.00", 20).await;

    let record = RecordPayment::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _);
    let idempotency_key = key("doppio-invio");
    let command = || RecordPaymentCommand {
        charge_id: charge.id(),
        amount: "400.00".parse().expect("decimale"),
        method: PaymentMethod::BankTransfer,
        paid_at: None,
        reference: None,
        payer_user_id: None,
        idempotency_key: idempotency_key.clone(),
    };

    let first = record
        .execute(&owner, command())
        .await
        .expect("primo invio");
    assert!(!first.replayed);
    assert_eq!(first.payment.status(), PaymentStatus::Succeeded);

    let second = record
        .execute(&owner, command())
        .await
        .expect("secondo invio: assorbito, non rifiutato");
    assert!(
        second.replayed,
        "una chiave ripetuta restituisce il pagamento già registrato, non un 409"
    );
    assert_eq!(second.payment.id(), first.payment.id());

    let balance = GetCharge::new(Arc::clone(&env.factory))
        .execute(&owner, charge.id())
        .await
        .expect("saldo letto");
    assert_eq!(
        balance.paid_amount.amount(),
        "400.00".parse::<rust_decimal::Decimal>().expect("decimale"),
        "il saldo è sceso una volta sola: senza l'idempotenza sarebbe sceso di 800"
    );
    assert_eq!(
        balance.charge.status(),
        ChargeStatus::PartiallyPaid,
        "400 su 800 è un pagamento parziale"
    );

    // La stessa chiave verso un **altro** addebito non è un doppio invio: è un
    // riuso, e un 200 con un pagamento diverso da quello chiesto sarebbe una bugia.
    let other = issue_charge(&env, &owner, lease.id(), "50.00", 20).await;
    let reused = record
        .execute(
            &owner,
            RecordPaymentCommand {
                charge_id: other.id(),
                ..command()
            },
        )
        .await;
    assert!(matches!(
        reused,
        Err(propflow_application::AppError::Conflict(
            propflow_application::error::Conflict::PaymentIdempotencyKeyReused
        ))
    ));
}

/// Una dichiarazione rifiutata lascia il saldo dov'era, e **perde la data
/// dichiarata**: `payments_succeeded_ck` vincola una sola direzione, quindi
/// l'azzeramento tocca al dominio.
#[tokio::test]
async fn a_rejected_declaration_leaves_the_balance_where_it_was() {
    let env = require_db!();
    let owner = register_owner(&env, "rifiuto").await;
    let property_id = create_property(&env, &owner, "Immobile del rifiuto", "Ancona").await;
    let lease = draft_lease(
        &env,
        &owner,
        property_id,
        None,
        days_from_today(-10),
        Some(days_from_today(355)),
    )
    .await;
    activate(&env, &owner, lease.id()).await.expect("attivato");
    let tenant = tenant_on_lease(&env, &owner, lease.id(), "rifiuto-inquilino").await;
    let charge = issue_charge(&env, &owner, lease.id(), "800.00", 20).await;

    let declared = DeclarePayment::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &tenant,
            DeclarePaymentCommand {
                charge_id: charge.id(),
                amount: "800.00".parse().expect("decimale"),
                method: PaymentMethod::Cash,
                paid_at: Some(chrono::Utc::now() - chrono::Duration::days(1)),
                reference: None,
                idempotency_key: key("da-rifiutare"),
            },
        )
        .await
        .expect("dichiarazione accettata");
    assert!(declared.payment.paid_at().is_some());

    let rejected = RejectPayment::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&owner, declared.payment.id())
        .await
        .expect("rifiutato");
    assert_eq!(rejected.status(), PaymentStatus::Cancelled);
    assert_eq!(
        rejected.paid_at(),
        None,
        "un pagamento che non è avvenuto non ha una data in cui è avvenuto"
    );

    let balance = GetCharge::new(Arc::clone(&env.factory))
        .execute(&owner, charge.id())
        .await
        .expect("saldo letto");
    assert_eq!(balance.paid_amount.amount(), rust_decimal::Decimal::ZERO);
    assert_eq!(
        balance.charge.status(),
        ChargeStatus::Issued,
        "il rifiuto non ricalcola lo stato: `pending` non contava"
    );

    // E non si riapre: `pending` ha due uscite e nessun ingresso di ritorno.
    let again = ConfirmPayment::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&owner, declared.payment.id())
        .await;
    assert!(again.is_err());
}

/// **La trappola del trigger polimorfico, riprodotta.**
///
/// `app.tg_check_polymorphic_ref` verifica che la riga referenziata esista ed è
/// `SECURITY DEFINER`: **non guarda l'organizzazione**. Il primo assert lo dimostra
/// inserendo a mano un pagamento di un'organizzazione verso l'addebito di un'altra e
/// vedendo che il trigger lo accetta. Il secondo mostra che a chiudere la strada è
/// lo use case, che rilegge l'addebito sotto la RLS del chiamante.
#[tokio::test]
async fn the_polymorphic_trigger_accepts_a_charge_of_another_organization_and_the_use_case_does_not()
 {
    let env = require_db!();
    let alfa = register_owner(&env, "pagamenti-alfa").await;
    let beta = register_owner(&env, "pagamenti-beta").await;

    let property_id = create_property(&env, &beta, "Immobile di Beta", "Bari").await;
    let lease = draft_lease(
        &env,
        &beta,
        property_id,
        None,
        days_from_today(-10),
        Some(days_from_today(355)),
    )
    .await;
    let charge = issue_charge(&env, &beta, lease.id(), "500.00", 20).await;

    // Il trigger accetta il riferimento incrociato: la riga di Beta **esiste**, e
    // il trigger non chiede altro. Lo stato è `pending` perché la riga non deve
    // sporcare il saldo di Beta: la dimostrazione riguarda il trigger, non il saldo.
    let accepted = sqlx::query(
        "INSERT INTO app.payments (org_id, payable_type, payable_id, amount, idempotency_key, status)
         VALUES ($1, 'charge', $2, 10.00, $3, 'pending')",
    )
    .bind(alfa.org_id().expect("org").as_uuid())
    .bind(charge.id().as_uuid())
    .bind(format!("trigger-{}", uuid::Uuid::now_v7()))
    .execute(env.pools.privileged())
    .await;
    assert!(
        accepted.is_ok(),
        "il trigger doveva accettare un riferimento a un'altra organizzazione: {:?}",
        accepted.err()
    );

    // Lo use case no: sotto la RLS di Alfa, l'addebito di Beta non si trova.
    let refused = RecordPayment::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &alfa,
            RecordPaymentCommand {
                charge_id: charge.id(),
                amount: "10.00".parse().expect("decimale"),
                method: PaymentMethod::BankTransfer,
                paid_at: None,
                reference: None,
                payer_user_id: None,
                idempotency_key: key("altra-organizzazione"),
            },
        )
        .await;
    assert!(matches!(
        refused,
        Err(propflow_application::AppError::NotFound(
            propflow_domain::Entity::Charge
        ))
    ));
}

/// Incassare su una pretesa caduta è un'incoerenza, non un caso d'uso.
#[tokio::test]
async fn a_dropped_claim_refuses_a_payment() {
    let env = require_db!();
    let owner = register_owner(&env, "pretesa-caduta").await;
    let property_id = create_property(&env, &owner, "Immobile annullato", "Perugia").await;
    let lease = draft_lease(
        &env,
        &owner,
        property_id,
        None,
        days_from_today(-10),
        Some(days_from_today(355)),
    )
    .await;
    let charge = issue_charge(&env, &owner, lease.id(), "200.00", 20).await;

    VoidCharge::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&owner, charge.id())
        .await
        .expect("annullato");

    let refused = RecordPayment::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            RecordPaymentCommand {
                charge_id: charge.id(),
                amount: "200.00".parse().expect("decimale"),
                method: PaymentMethod::BankTransfer,
                paid_at: None,
                reference: None,
                payer_user_id: None,
                idempotency_key: key("su-annullato"),
            },
        )
        .await;
    assert!(matches!(
        refused,
        Err(propflow_application::AppError::Validation(
            propflow_domain::DomainError::Invariant(propflow_domain::Invariant::ChargeNotPayable)
        ))
    ));

    // E nessuna riga è rimasta dietro: il rifiuto avviene prima dell'inserimento.
    let payments: i64 =
        sqlx::query_scalar("SELECT count(*) FROM app.payments WHERE payable_id = $1")
            .bind(charge.id().as_uuid())
            .fetch_one(env.pools.privileged())
            .await
            .expect("conteggio");
    assert_eq!(payments, 0);
}

/// Il sovrapagamento è possibile e non lo si impedisce: `outstanding_amount` diventa
/// **negativo**, che è un credito verso l'inquilino. Registrato invece che scoperto
/// dopo — e `app.money_amount` lo ammette, quindi la vista non si rompe.
#[tokio::test]
async fn an_overpayment_drives_the_outstanding_amount_negative() {
    let env = require_db!();
    let owner = register_owner(&env, "sovrapagamento").await;
    let property_id = create_property(&env, &owner, "Immobile pagato troppo", "Lecce").await;
    let lease = draft_lease(
        &env,
        &owner,
        property_id,
        None,
        days_from_today(-10),
        Some(days_from_today(355)),
    )
    .await;
    let charge = issue_charge(&env, &owner, lease.id(), "100.00", 20).await;

    RecordPayment::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            RecordPaymentCommand {
                charge_id: charge.id(),
                amount: "150.00".parse().expect("decimale"),
                method: PaymentMethod::BankTransfer,
                paid_at: None,
                reference: None,
                payer_user_id: None,
                idempotency_key: key("sovrapagamento"),
            },
        )
        .await
        .expect("registrato");

    let balance = GetCharge::new(Arc::clone(&env.factory))
        .execute(&owner, charge.id())
        .await
        .expect("saldo letto");
    assert_eq!(
        balance.outstanding_amount.amount(),
        "-50.00".parse::<rust_decimal::Decimal>().expect("decimale")
    );
    assert_eq!(balance.charge.status(), ChargeStatus::Paid);
    assert!(!balance.is_past_due);

    // La dashboard filtra `outstanding_amount > 0`: il credito **non compensa** gli
    // altri insoluti, che è probabilmente il comportamento giusto e va comunque
    // saputo.
    let total: rust_decimal::Decimal = sqlx::query_scalar(
        "SELECT total_outstanding::numeric FROM app.v_owner_dashboard WHERE org_id = $1",
    )
    .bind(owner.org_id().expect("org").as_uuid())
    .fetch_one(env.pools.privileged())
    .await
    .expect("dashboard letta");
    assert_eq!(total, rust_decimal::Decimal::ZERO);
}

/// I privilegi di colonna della migration `0025`, verificati **anche al contrario**.
///
/// Il privilegio viene verificato prima delle policy e prima che le righe vengano
/// esaminate, quindi il tentativo fallisce anche su un identificatore inesistente:
/// il test non ha bisogno di ricostruire lo scenario per dimostrare che la strada è
/// chiusa.
///
/// Le query sono letterali statici uno per uno e non composte in un ciclo: la
/// regola 15 non ammette `format!` dentro `sqlx::query`, e il tipo lo impedisce —
/// il che è esattamente ciò che deve fare, anche in un test.
#[tokio::test]
async fn the_application_role_cannot_rewrite_what_a_payment_records() {
    let env = require_db!();
    let id = uuid::Uuid::now_v7();

    // Ciò che il pagamento **registra**: chi ha pagato, quanto, come, con quale
    // chiave. Un fatto non si corregge riscrivendolo.
    for attempt in [
        sqlx::query("UPDATE app.payments SET amount = 1.00 WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.payments SET payer_user_id = $1 WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.payments SET idempotency_key = 'riscritta' WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.payments SET refunded_amount = 1.00 WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.payments SET org_id = $1 WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.payments SET method = 'cash' WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
    ] {
        let error = attempt.expect_err("l'aggiornamento doveva essere negato");
        assert!(
            error.to_string().contains("permission denied"),
            "atteso un rifiuto per privilegi, ottenuto: {error}"
        );
    }

    // Le tre colonne che il dominio muove restano concesse: la restrizione riguarda
    // ciò che il pagamento registra, non il suo avanzamento. `paid_at` è fra esse
    // perché il rifiuto la **azzera**.
    for allowed in [
        sqlx::query("UPDATE app.payments SET status = 'cancelled' WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.payments SET paid_at = NULL WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.payments SET reconciled_at = now() WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
    ] {
        assert!(
            allowed.is_ok(),
            "l'aggiornamento non doveva essere negato: {:?}",
            allowed.err()
        );
    }
}

// ---------------------------------------------------------------------------
// Spese co-living
// ---------------------------------------------------------------------------

/// Un inquilino dell'organizzazione, senza contratto.
///
/// Diverso da [`tenant_on_lease`]: là serviva un contratto perché la visibilità
/// passava da `can_access_lease`. Qui no — un gruppo di spesa esiste anche senza
/// immobile, e `can_access_expense_group` guarda la membership del **gruppo**.
/// Basta quindi l'appartenenza all'organizzazione, che si ottiene con un invito
/// su un contratto qualunque.
async fn coliving_tenant(
    env: &common::TestEnv,
    owner: &RequestContext,
    lease_id: propflow_domain::shared::LeaseId,
    label: &str,
) -> RequestContext {
    tenant_on_lease(env, owner, lease_id, label).await
}

/// Il palcoscenico del modulo: un proprietario, un immobile, un contratto, e
/// tre coinquilini che sono membri dell'organizzazione.
async fn coliving_stage(
    env: &common::TestEnv,
    label: &str,
) -> (RequestContext, PropertyId, Vec<RequestContext>) {
    let owner = register_owner(env, label).await;
    let property_id = create_property(env, &owner, "Casa condivisa", "Bologna").await;
    let lease = draft_lease(
        env,
        &owner,
        property_id,
        None,
        days_from_today(-30),
        Some(days_from_today(335)),
    )
    .await;
    // Tre posti sul contratto prima di attivarlo, uno per coinquilino: da attivo non
    // se ne aggiungono, e l'invito va a una parte che esiste (ADR-056).
    let parties = {
        let mut uow = env.factory.begin(&owner).await.expect("transazione aperta");
        uow.lease_parties()
            .list_for_lease(lease.id())
            .await
            .expect("parti del contratto")
    };
    CorrectLeaseParty::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            CorrectLeasePartyCommand {
                id: parties[0].id(),
                changes: LeasePartyChanges {
                    share_percent: Some(rust_decimal::Decimal::from(34)),
                    ..LeasePartyChanges::default()
                },
            },
        )
        .await
        .expect("quota del primo posto");
    for nth in 2..=3 {
        AddLeaseParty::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
            .execute(
                &owner,
                AddLeasePartyCommand {
                    lease_id: lease.id(),
                    role: LeasePartyRole::Cotenant,
                    full_name: format!("Coinquilino di prova {nth}"),
                    email: None,
                    share_percent: rust_decimal::Decimal::from(33),
                    is_primary: false,
                },
            )
            .await
            .expect("posto aggiunto");
    }
    activate(env, &owner, lease.id()).await.expect("attivato");

    let mut tenants = Vec::with_capacity(3);
    for nth in 1..=3 {
        tenants.push(coliving_tenant(env, &owner, lease.id(), &format!("{label}-{nth}")).await);
    }
    (owner, property_id, tenants)
}

fn user_of(context: &RequestContext) -> propflow_domain::shared::UserId {
    context.user_id().expect("utente autenticato")
}

async fn create_group(
    env: &common::TestEnv,
    context: &RequestContext,
    name: &str,
    property_id: Option<PropertyId>,
    method: SplitMethod,
) -> propflow_domain::expense::ExpenseGroup {
    CreateExpenseGroup::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            context,
            CreateExpenseGroupCommand {
                property_id,
                kind: ExpenseGroupKind::Coliving,
                name: name.to_owned(),
                default_split_method: method,
                weight: Weight::one(),
                display_name: None,
            },
        )
        .await
        .expect("gruppo creato")
        .0
}

async fn add_member(
    env: &common::TestEnv,
    context: &RequestContext,
    group_id: propflow_domain::shared::ExpenseGroupId,
    user: &RequestContext,
    weight: &str,
) {
    AddGroupMember::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            context,
            AddGroupMemberCommand {
                group_id,
                user_id: user_of(user),
                room_id: None,
                display_name: None,
                weight: Weight::parse(weight.parse().expect("decimale")).expect("peso valido"),
            },
        )
        .await
        .expect("membro aggiunto");
}

async fn balances_of(
    env: &common::TestEnv,
    context: &RequestContext,
    group_id: propflow_domain::shared::ExpenseGroupId,
) -> Vec<propflow_domain::expense::GroupBalance> {
    GetGroupBalances::new(Arc::clone(&env.factory))
        .execute(context, group_id)
        .await
        .expect("saldi letti")
}

fn balance_of(
    balances: &[propflow_domain::expense::GroupBalance],
    user: &RequestContext,
) -> rust_decimal::Decimal {
    balances
        .iter()
        .find(|balance| balance.user_id == user_of(user))
        .map(|balance| balance.net_balance.amount())
        .unwrap_or_else(|| panic!("nessun saldo per l'utente atteso"))
}

/// **La trappola più cara del piano, verificata sul binario vero.**
///
/// Lo schema, letto come usciva dalla `0014`, produceva un gruppo inutilizzabile
/// da chi lo aveva creato: `expense_groups_insert` chiede `is_org_member`, che un
/// inquilino soddisfa, ma `expense_group_members_insert` chiedeva
/// `can_access_expense_group`, che appena creato il gruppo è falsa proprio per lui.
///
/// La migration `0026` aggiunge il secondo ramo, e la funzione
/// `app.created_expense_group` è `SECURITY DEFINER` perché **una sottoquery dentro
/// una policy subisce la RLS della tabella che legge**: un `EXISTS` su
/// `app.expense_groups` scritto nella policy passerebbe da `expense_groups_select`,
/// che chiede l'accesso che a quel punto manca. Il test qui sotto lo dimostra
/// eseguendo entrambe le versioni.
#[tokio::test]
async fn whoever_creates_a_group_is_already_in_it() {
    let env = require_db!();
    let (owner, property_id, tenants) = coliving_stage(&env, "gruppo-creatore").await;
    let creator = &tenants[0];

    let group = create_group(
        &env,
        creator,
        "Casa di via Test",
        Some(property_id),
        SplitMethod::Equal,
    )
    .await;

    let (_, members) = GetExpenseGroup::new(Arc::clone(&env.factory))
        .execute(creator, group.id())
        .await
        .expect("il creatore vede il proprio gruppo");
    assert_eq!(members.len(), 1, "il gruppo non deve nascere vuoto");
    assert_eq!(members[0].user_id(), user_of(creator));

    // **La trappola, riprodotta.** Serve un gruppo di cui il creatore non è
    // ancora membro, cioè esattamente lo stato in cui si trova la policy mentre
    // valuta l'`INSERT` della prima membership. Si scrive con il pool
    // privilegiato, perché `CreateExpenseGroup` quello stato non lo lascia mai
    // esistere — ed è il suo scopo.
    let orphan = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO app.expense_groups (id, org_id, name, created_by)
         VALUES ($1, $2, 'Gruppo senza membri', $3)",
    )
    .bind(orphan)
    .bind(creator.org_id().expect("org").as_uuid())
    .bind(user_of(creator).as_uuid())
    .execute(env.pools.privileged())
    .await
    .expect("gruppo orfano scritto");

    let (subquery, definer, can_access): (bool, bool, bool) = {
        let mut tx = env
            .pools
            .application()
            .begin()
            .await
            .expect("transazione applicativa");
        sqlx::query("SELECT app.set_request_context($1, $2, false, NULL, NULL, NULL, 'tenant')")
            .bind(user_of(creator).as_uuid())
            .bind(creator.org_id().expect("org").as_uuid())
            .execute(&mut *tx)
            .await
            .expect("contesto installato");
        sqlx::query_as(
            // La prima è la sottoquery che si sarebbe scritta dentro la policy;
            // la seconda è la funzione `SECURITY DEFINER` della `0026`; la terza
            // è la ragione per cui la prima fallisce.
            "SELECT EXISTS (SELECT 1 FROM app.expense_groups g
                             WHERE g.id = $1 AND g.created_by = app.current_user_id()),
                    app.created_expense_group($1),
                    app.can_access_expense_group($1)",
        )
        .bind(orphan)
        .fetch_one(&mut *tx)
        .await
        .expect("query eseguita")
    };

    assert!(
        !can_access,
        "chi non è ancora membro non ha accesso al gruppo: è il punto di partenza"
    );
    assert!(
        !subquery,
        "una sottoquery dentro una policy subisce la RLS della tabella che legge: \
         passa da `expense_groups_select`, che chiede proprio l'accesso che manca. \
         Scritta così, il ramo nuovo della policy non scatterebbe mai"
    );
    assert!(
        definer,
        "la funzione `SECURITY DEFINER` legge il fatto vero, ed è la ragione per cui \
         la `0026` non si è potuta scrivere con un `EXISTS` in linea"
    );

    // E lo staff vede il gruppo dei propri inquilini: `can_access_expense_group`
    // risponde vero per `is_org_staff`. Guarda, e non ne è parte.
    let (_, seen_by_owner) = GetExpenseGroup::new(Arc::clone(&env.factory))
        .execute(&owner, group.id())
        .await
        .expect("lo staff vede il gruppo");
    assert_eq!(seen_by_owner.len(), 1);
}

/// **Il difetto della vista, verificato al contrario**, come per la `0024`.
///
/// Prima della `0027` chi lasciava il gruppo spariva dai saldi **portandosi via il
/// debito**: il coinquilino che deve 33,33 esce, e quei 33,33 non risultano più a
/// nessuno. La somma dei saldi netti del gruppo, che deve fare zero, smetteva di
/// farlo.
///
/// Il test verifica entrambe le direzioni: il debitore uscito **resta**, e chi
/// esce in pari **sparisce**.
#[tokio::test]
async fn whoever_leaves_with_an_open_account_stays_in_the_balances() {
    let env = require_db!();
    let (_, _, tenants) = coliving_stage(&env, "saldi-uscita").await;
    let (creator, second, third) = (&tenants[0], &tenants[1], &tenants[2]);

    let group = create_group(&env, creator, "Uscite", None, SplitMethod::Equal).await;
    add_member(&env, creator, group.id(), second, "1").await;
    add_member(&env, creator, group.id(), third, "1").await;

    // 100,00 anticipati dal creatore, divisi in tre: 33,34 / 33,33 / 33,33 con il
    // centesimo al primo in ordine di `user_id`.
    RecordExpense::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            creator,
            RecordExpenseCommand {
                group_id: group.id(),
                paid_by_user_id: None,
                category: ExpenseCategory::Groceries,
                description: "Spesa della settimana".to_owned(),
                amount: "100.00".parse().expect("decimale"),
                expense_date: None,
                split_method: None,
                notes: None,
            },
        )
        .await
        .expect("spesa registrata");

    // Il terzo salda il proprio debito e il creditore conferma: torna in pari.
    let settlement = DeclareSettlement::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            third,
            DeclareSettlementCommand {
                group_id: group.id(),
                to_user_id: user_of(creator),
                amount: balance_of(&balances_of(&env, third, group.id()).await, third).abs(),
                method: PaymentMethod::BankTransfer,
                settled_at: None,
                note: None,
            },
        )
        .await
        .expect("dichiarato");

    ConfirmSettlement::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(creator, settlement.id())
        .await
        .expect("confermato");

    let before = balances_of(&env, creator, group.id()).await;
    assert_eq!(before.len(), 3);
    assert_eq!(
        balance_of(&before, third),
        rust_decimal::Decimal::ZERO,
        "il terzo ha saldato"
    );

    // Escono entrambi: il secondo in debito, il terzo in pari.
    for leaving in [second, third] {
        LeaveGroup::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
            .execute(leaving, group.id(), user_of(leaving))
            .await
            .expect("uscito");
    }

    let after = balances_of(&env, creator, group.id()).await;
    assert_eq!(
        after.len(),
        2,
        "il debitore uscito resta, quello in pari sparisce"
    );
    assert!(
        balance_of(&after, second) < rust_decimal::Decimal::ZERO,
        "prima della 0027 questa riga spariva, e il debito con lei"
    );
    assert!(
        !after
            .iter()
            .any(|balance| balance.user_id == user_of(third)),
        "chi esce in pari non ha niente da dire e non compare"
    );

    // **L'invariante che la 0027 rimette in piedi**: la somma dei saldi fa zero.
    // Con la vista della 0015 questo assert falliva.
    let total: rust_decimal::Decimal = after
        .iter()
        .map(|balance| balance.net_balance.amount())
        .sum();
    assert_eq!(
        total,
        rust_decimal::Decimal::ZERO,
        "la somma dei saldi di un gruppo deve fare zero: un credito senza contropartita è un numero che mente"
    );
}

/// Il giro a mano del modulo, contro il binario vero.
///
/// La dichiarazione **non** muove i saldi — è ciò che il flusso in due tempi
/// compra — e la conferma li muove. A confermare è **chi riceve**, non lo staff.
#[tokio::test]
async fn a_declaration_does_not_move_the_balances_and_the_creditor_confirms() {
    let env = require_db!();
    let (owner, _, tenants) = coliving_stage(&env, "regolamento-giro").await;
    let (creator, second, third) = (&tenants[0], &tenants[1], &tenants[2]);

    let group = create_group(&env, creator, "Il giro", None, SplitMethod::Equal).await;
    add_member(&env, creator, group.id(), second, "1").await;
    add_member(&env, creator, group.id(), third, "1").await;

    let recorded = RecordExpense::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            creator,
            RecordExpenseCommand {
                group_id: group.id(),
                paid_by_user_id: None,
                category: ExpenseCategory::Groceries,
                description: "Cento euro tondi".to_owned(),
                amount: "100.00".parse().expect("decimale"),
                expense_date: None,
                split_method: None,
                notes: None,
            },
        )
        .await
        .expect("spesa registrata");

    // Le quote sommano esattamente all'importo: il trigger non ammette altro, e
    // l'avrebbe rifiutato al commit.
    let share_total: rust_decimal::Decimal = recorded
        .shares
        .iter()
        .map(|share| share.share_amount().amount())
        .sum();
    assert_eq!(share_total, recorded.expense.amount().amount());
    assert_eq!(recorded.shares.len(), 3);

    let start = balances_of(&env, creator, group.id()).await;
    let creditor_before = balance_of(&start, creator);
    assert!(creditor_before > rust_decimal::Decimal::ZERO);

    let debt = balance_of(&start, second).abs();
    let settlement = DeclareSettlement::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            second,
            DeclareSettlementCommand {
                group_id: group.id(),
                to_user_id: user_of(creator),
                amount: debt,
                method: PaymentMethod::BankTransfer,
                settled_at: None,
                note: Some("Bonifico di ieri".to_owned()),
            },
        )
        .await
        .expect("dichiarato");
    assert_eq!(settlement.status(), SettlementStatus::Pending);

    let pending = balances_of(&env, creator, group.id()).await;
    assert_eq!(
        balance_of(&pending, creator),
        creditor_before,
        "una dichiarazione pendente non muove i saldi: la vista somma i soli `confirmed`"
    );

    // **Chi ha dichiarato non può confermare a sé stesso.** La `USING` di
    // `settlements_update` lo lascerebbe scrivere — nomina anche `from_user_id` —
    // quindi a fermarlo è il service layer, con un codice che dice il rimedio
    // giusto: chiedere all'altro.
    let refused = ConfirmSettlement::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(second, settlement.id())
        .await
        .expect_err("chi paga non conferma");
    assert_eq!(refused.code(), "settlement.not_addressed_to_you");

    let confirmed = ConfirmSettlement::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(creator, settlement.id())
        .await
        .expect("il creditore conferma");
    assert_eq!(confirmed.status(), SettlementStatus::Confirmed);
    assert_eq!(confirmed.confirmed_by(), Some(user_of(creator)));

    let after = balances_of(&env, creator, group.id()).await;
    assert_eq!(
        balance_of(&after, second),
        rust_decimal::Decimal::ZERO,
        "chi ha saldato è in pari"
    );
    assert_eq!(balance_of(&after, creator), creditor_before - debt);

    // E il proprietario guarda senza esserne parte: vede i saldi, e non compare
    // fra loro.
    let seen_by_owner = balances_of(&env, &owner, group.id()).await;
    assert_eq!(seen_by_owner.len(), 3);
    assert!(
        !seen_by_owner
            .iter()
            .any(|balance| balance.user_id == user_of(&owner)),
        "lo staff guarda il conto dei coinquilini, non ne è parte"
    );
}

/// **Il trigger differito, riprodotto.**
///
/// `expense_shares_balanced` è `DEFERRABLE INITIALLY DEFERRED`: quote che non
/// sommano all'importo non fanno fallire l'`INSERT`, fanno fallire il **`COMMIT`**.
/// Il dominio non permette di costruire quello stato, quindi il test scrive le
/// righe a mano per dimostrare che la rete sotto c'è.
#[tokio::test]
async fn shares_that_do_not_add_up_are_refused_at_commit_and_not_before() {
    let env = require_db!();
    let (_, _, tenants) = coliving_stage(&env, "trigger-differito").await;
    let creator = &tenants[0];

    let group = create_group(&env, creator, "Quote sbilanciate", None, SplitMethod::Equal).await;

    let mut tx = env
        .pools
        .privileged()
        .begin()
        .await
        .expect("transazione privilegiata");
    let expense_id = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO app.expenses (id, org_id, group_id, paid_by_user_id, description, amount)
         VALUES ($1, $2, $3, $4, 'Spesa sbilanciata', 100.00)",
    )
    .bind(expense_id)
    .bind(creator.org_id().expect("org").as_uuid())
    .bind(group.id().as_uuid())
    .bind(user_of(creator).as_uuid())
    .execute(&mut *tx)
    .await
    .expect("la spesa entra");

    // 50,00 su 100,00: l'invariante è già violata, e **l'insert riesce lo stesso**.
    sqlx::query(
        "INSERT INTO app.expense_shares (org_id, expense_id, user_id, share_amount)
         VALUES ($1, $2, $3, 50.00)",
    )
    .bind(creator.org_id().expect("org").as_uuid())
    .bind(expense_id)
    .bind(user_of(creator).as_uuid())
    .execute(&mut *tx)
    .await
    .expect("l'insert delle quote riesce: il trigger è differito");

    let error = tx
        .commit()
        .await
        .expect_err("il commit doveva essere rifiutato");
    assert!(
        error.to_string().contains("does not match expense amount"),
        "atteso il rifiuto del trigger al commit, ottenuto: {error}"
    );
}

/// I privilegi di colonna della `0028`, verificati **al contrario**.
///
/// Il buco che chiudono non è teorico: `expense_shares_balanced` è un trigger su
/// `app.expense_shares`, quindi riscrivere `app.expenses.amount` **non lo fa
/// scattare** e sbilancia l'invariante contabile in silenzio.
#[tokio::test]
async fn the_application_role_cannot_rewrite_what_an_expense_records() {
    let env = require_db!();
    let id = uuid::Uuid::now_v7();

    // `amount` è la prima dell'elenco, ed è la ragione della migration.
    for attempt in [
        sqlx::query("UPDATE app.expenses SET amount = 1.00 WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.expenses SET group_id = $1 WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.expenses SET paid_by_user_id = $1 WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.expenses SET split_method = 'weighted' WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.expenses SET currency = 'USD' WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.expenses SET org_id = $1 WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.expenses SET utility_bill_id = $1 WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
    ] {
        let error = attempt.expect_err("l'aggiornamento doveva essere negato");
        assert!(
            error.to_string().contains("permission denied"),
            "atteso un rifiuto per privilegi, ottenuto: {error}"
        );
    }

    // Le cinque concesse: una correzione legittima passa. `deleted_at` è fra esse
    // al contrario degli addebiti, perché `app.expenses` non ha una colonna di
    // stato e il soft delete *è* l'annullamento di una spesa.
    for allowed in [
        sqlx::query("UPDATE app.expenses SET description = 'corretta' WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.expenses SET category = 'cleaning' WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.expenses SET expense_date = CURRENT_DATE WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.expenses SET notes = 'nota' WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.expenses SET deleted_at = now() WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
    ] {
        assert!(
            allowed.is_ok(),
            "l'aggiornamento non doveva essere negato: {:?}",
            allowed.err()
        );
    }
}

/// Un gruppo di un'altra organizzazione è **invisibile**, e la spesa che lo cita
/// riceve `404`.
///
/// Non è ridondante con l'isolamento generale: `app.expense_groups` **non ha** il
/// trigger `tg_assert_org_matches_property` che hanno `rooms`, `leases`, `tickets`
/// e `utility_bills`, quindi qui l'unico controllo è la rilettura del gruppo sotto
/// la RLS del chiamante.
#[tokio::test]
async fn a_group_of_another_organization_is_out_of_reach() {
    let env = require_db!();
    let (_, _, mine) = coliving_stage(&env, "isolamento-mio").await;
    let (_, _, theirs) = coliving_stage(&env, "isolamento-altrui").await;

    let group = create_group(&env, &theirs[0], "Casa altrui", None, SplitMethod::Equal).await;

    let seen = GetExpenseGroup::new(Arc::clone(&env.factory))
        .execute(&mine[0], group.id())
        .await
        .expect_err("il gruppo di un'altra organizzazione non si vede");
    assert_eq!(seen.code(), "expense_group.not_found");

    let recorded = RecordExpense::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &mine[0],
            RecordExpenseCommand {
                group_id: group.id(),
                paid_by_user_id: None,
                category: ExpenseCategory::Other,
                description: "Spesa in casa d'altri".to_owned(),
                amount: "10.00".parse().expect("decimale"),
                expense_date: None,
                split_method: None,
                notes: None,
            },
        )
        .await
        .expect_err("la spesa su un gruppo invisibile non si scrive");
    assert_eq!(recorded.code(), "expense_group.not_found");
}

/// Chi ha anticipato **deve essere un membro attivo**, e a imporlo è il service
/// layer perché lo schema non lo fa.
///
/// Il danno che evita è silenzioso: `app.v_expense_group_balances` parte dai
/// membri, quindi un credito intestato a un estraneo non comparirebbe a nessuno e
/// la somma dei saldi del gruppo smetterebbe di fare zero.
#[tokio::test]
async fn an_expense_cannot_be_advanced_by_someone_outside_the_group() {
    let env = require_db!();
    let (_, _, tenants) = coliving_stage(&env, "anticipo-estraneo").await;
    let (creator, outsider) = (&tenants[0], &tenants[1]);

    let group = create_group(&env, creator, "Solo io", None, SplitMethod::Equal).await;

    let refused = RecordExpense::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            creator,
            RecordExpenseCommand {
                group_id: group.id(),
                paid_by_user_id: Some(user_of(outsider)),
                category: ExpenseCategory::Other,
                description: "Ha pagato un estraneo".to_owned(),
                amount: "10.00".parse().expect("decimale"),
                expense_date: None,
                split_method: None,
                notes: None,
            },
        )
        .await
        .expect_err("chi non è del gruppo non può aver anticipato");
    assert_eq!(refused.code(), "expense.payer_not_in_group");
}

/// Il rientro è un `UPDATE` e non un `INSERT`.
///
/// `expense_group_members_group_user_uk` è unico su `(group_id, user_id)` e **non
/// è parziale**: chi scrive `join` come un inserimento incondizionato prende un
/// conflitto sul vincolo. `joined_at` segue il rientro, ed è la ragione per cui
/// l'ordine del resto nella ripartizione **non** è per quella colonna.
#[tokio::test]
async fn rejoining_a_group_is_an_update_and_moves_the_joining_date() {
    let env = require_db!();
    let (_, _, tenants) = coliving_stage(&env, "rientro").await;
    let (creator, second) = (&tenants[0], &tenants[1]);

    let group = create_group(&env, creator, "Va e viene", None, SplitMethod::Equal).await;
    add_member(&env, creator, group.id(), second, "1").await;

    let first_join = {
        let (_, members) = GetExpenseGroup::new(Arc::clone(&env.factory))
            .execute(creator, group.id())
            .await
            .expect("composizione letta");
        members
            .iter()
            .find(|member| member.user_id() == user_of(second))
            .expect("il secondo è dentro")
            .joined_at()
    };

    LeaveGroup::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(second, group.id(), user_of(second))
        .await
        .expect("uscito");

    add_member(&env, creator, group.id(), second, "2").await;

    let (_, members) = GetExpenseGroup::new(Arc::clone(&env.factory))
        .execute(creator, group.id())
        .await
        .expect("composizione letta");
    let rejoined: Vec<_> = members
        .iter()
        .filter(|member| member.user_id() == user_of(second))
        .collect();

    assert_eq!(
        rejoined.len(),
        1,
        "il vincolo unico non è parziale: una seconda riga non può esistere"
    );
    assert!(rejoined[0].is_active());
    assert!(
        rejoined[0].joined_at() > first_join,
        "`joined_at` segue il rientro, ed è per questo che il resto della ripartizione non si ordina per quella colonna"
    );
    assert_eq!(
        rejoined[0].weight().value(),
        rust_decimal::Decimal::TWO,
        "il rientro aggiorna anche il peso: la doppia adesso è sua"
    );
}

/// Si esce per sé stessi, e dalla migration `0029` lo dice **la policy**.
///
/// Il test fissa le due metà che contano, e la seconda è quella che rende la
/// prima una garanzia invece di una convenzione: il rifiuto applicativo con il
/// suo codice, e il fatto che la stessa scrittura fatta **a mano** con il
/// contesto del coinquilino sia rifiutata dalla Row Level Security.
///
/// Prima della `0029` la seconda metà asseriva l'opposto — la policy lasciava
/// passare — perché la `USING` ammette qualunque membro attivo del gruppo. A
/// stringere è la `WITH CHECK`, che vede la riga **nuova**: un membro non staff
/// può scriverla solo se il risultato lascia la persona dentro il gruppo.
#[tokio::test]
async fn a_cotenant_cannot_remove_another_and_the_policy_says_so_too() {
    let env = require_db!();
    let (owner, _, tenants) = coliving_stage(&env, "uscita-altrui").await;
    let (creator, second) = (&tenants[0], &tenants[1]);

    let group = create_group(&env, creator, "Chi esce", None, SplitMethod::Equal).await;
    add_member(&env, creator, group.id(), second, "1").await;

    let refused = LeaveGroup::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(creator, group.id(), user_of(second))
        .await
        .expect_err("un coinquilino non toglie un altro");
    assert_eq!(refused.code(), "expense_group.membership_not_yours");

    // **La stessa scrittura fatta a mano è rifiutata dalla policy**, ed è la metà
    // che rende la prima una garanzia. Prima della `0029` qui passava.
    let mut tx = env
        .pools
        .application()
        .begin()
        .await
        .expect("transazione applicativa");
    sqlx::query("SELECT app.set_request_context($1, $2, false, NULL, NULL, NULL, 'tenant')")
        .bind(user_of(creator).as_uuid())
        .bind(creator.org_id().expect("org").as_uuid())
        .execute(&mut *tx)
        .await
        .expect("contesto installato");
    let error = sqlx::query(
        "UPDATE app.expense_group_members SET left_at = now()
          WHERE group_id = $1 AND user_id = $2",
    )
    .bind(group.id().as_uuid())
    .bind(user_of(second).as_uuid())
    .execute(&mut *tx)
    .await
    .expect_err("la policy doveva rifiutare");
    assert!(
        error.to_string().contains("row-level security"),
        "atteso il rifiuto della policy, ottenuto: {error}"
    );
    tx.rollback().await.expect("annullata");

    // **Il rientro fatto da un altro deve continuare a funzionare.** È la trappola
    // della `0029`: stringere la `USING` invece della `WITH CHECK` avrebbe rotto
    // proprio questo, perché reintegrare qualcuno è un `UPDATE` sulla riga di un
    // altro.
    LeaveGroup::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(second, group.id(), user_of(second))
        .await
        .expect("il secondo esce da sé");
    add_member(&env, creator, group.id(), second, "2").await;

    // E lo staff toglie chiunque, perché è chi amministra.
    LeaveGroup::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&owner, group.id(), user_of(second))
        .await
        .expect("lo staff toglie chi se n'è andato davvero");
}

/// I privilegi di colonna della `0029`, verificati **al contrario**.
///
/// `user_id` è la più cara delle tre negate: intestare a un'altra persona una
/// membership lascerebbe `app.expense_shares` a puntare a quella vecchia, e i
/// saldi del gruppo smetterebbero di sommare a zero senza che nulla protesti.
#[tokio::test]
async fn the_application_role_cannot_rewrite_who_a_membership_belongs_to() {
    let env = require_db!();
    let id = uuid::Uuid::now_v7();

    for attempt in [
        sqlx::query("UPDATE app.expense_group_members SET user_id = $1 WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.expense_group_members SET group_id = $1 WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.expense_group_members SET org_id = $1 WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
    ] {
        let error = attempt.expect_err("l'aggiornamento doveva essere negato");
        assert!(
            error.to_string().contains("permission denied"),
            "atteso un rifiuto per privilegi, ottenuto: {error}"
        );
    }

    // Le cinque che il dominio muove restano concesse. `joined_at` è fra esse
    // perché il rientro la riscrive, e `weight` perché il gruppo deve poter
    // registrare che qualcuno si è spostato nella doppia.
    for allowed in [
        sqlx::query("UPDATE app.expense_group_members SET left_at = now() WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.expense_group_members SET joined_at = now() WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.expense_group_members SET weight = 2 WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.expense_group_members SET display_name = 'x' WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
        sqlx::query("UPDATE app.expense_group_members SET room_id = $1 WHERE id = $1")
            .bind(id)
            .execute(env.pools.application())
            .await,
    ] {
        assert!(
            allowed.is_ok(),
            "l'aggiornamento non doveva essere negato: {:?}",
            allowed.err()
        );
    }
}

/// Le tre uscite di `pending`, e la sola che i pagamenti non hanno.
///
/// `settlements_update` nomina anche `from_user_id`, mentre `payments_update` è
/// staff soltanto: **fra pari il ritiro è lecito, verso il proprietario no**. Il
/// test verifica che la scrittura passi davvero sul database, non solo nel
/// dominio.
#[tokio::test]
async fn whoever_declared_a_settlement_can_withdraw_it_and_the_creditor_can_refuse_it() {
    let env = require_db!();
    let (_, _, tenants) = coliving_stage(&env, "uscite-regolamento").await;
    let (creator, second) = (&tenants[0], &tenants[1]);

    let group = create_group(&env, creator, "Tre uscite", None, SplitMethod::Equal).await;
    add_member(&env, creator, group.id(), second, "1").await;

    // Lo use case vive fuori dalla chiusura: costruirlo dentro produrrebbe un
    // temporaneo che il future restituito continuerebbe a prendere in prestito.
    let declaring = DeclareSettlement::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _);
    let declare = || {
        declaring.execute(
            second,
            DeclareSettlementCommand {
                group_id: group.id(),
                to_user_id: user_of(creator),
                amount: "10.00".parse().expect("decimale"),
                method: PaymentMethod::Cash,
                settled_at: Some(chrono::Utc::now() - chrono::Duration::days(1)),
                note: None,
            },
        )
    };

    // Il ritiro di chi ha dichiarato.
    let to_cancel = declare().await.expect("dichiarato");
    let cancelled = CancelSettlement::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(second, to_cancel.id())
        .await
        .expect("chi ha dichiarato ritira");
    assert_eq!(cancelled.status(), SettlementStatus::Cancelled);
    assert_eq!(
        cancelled.settled_at(),
        None,
        "un trasferimento che non è avvenuto non ha una data in cui è avvenuto"
    );

    // Il rifiuto di chi riceve.
    let to_reject = declare().await.expect("dichiarato");
    let rejected = RejectSettlement::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(creator, to_reject.id())
        .await
        .expect("il creditore rifiuta");
    assert_eq!(rejected.status(), SettlementStatus::Rejected);
    assert_eq!(rejected.settled_at(), None);

    // E chi riceve **non** ritira: quello è il rifiuto, ed è un fatto diverso.
    let to_refuse = declare().await.expect("dichiarato");
    let refused = CancelSettlement::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(creator, to_refuse.id())
        .await
        .expect_err("chi riceve non ritira");
    assert_eq!(refused.code(), "settlement.not_declared_by_you");

    // I due terminali non si muovono più, e nessuno dei tre ha mosso i saldi.
    let stuck = ConfirmSettlement::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(creator, cancelled.id())
        .await
        .expect_err("da `cancelled` non si esce");
    assert_eq!(stuck.code(), "settlement.invalid_transition");
}

/// La ripartizione pesata, contro il database vero.
///
/// Chi occupa la doppia paga il doppio, e le quote sommano esattamente
/// all'importo — che è l'unico criterio, perché il resto lo impone il trigger.
#[tokio::test]
async fn a_weighted_split_makes_the_double_room_pay_more() {
    let env = require_db!();
    let (_, _, tenants) = coliving_stage(&env, "pesata").await;
    let (creator, second, third) = (&tenants[0], &tenants[1], &tenants[2]);

    let group = create_group(&env, creator, "Con la doppia", None, SplitMethod::Weighted).await;
    add_member(&env, creator, group.id(), second, "1").await;
    add_member(&env, creator, group.id(), third, "2").await;

    let recorded = RecordExpense::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            creator,
            RecordExpenseCommand {
                group_id: group.id(),
                paid_by_user_id: None,
                category: ExpenseCategory::Utilities,
                description: "Bolletta della luce".to_owned(),
                amount: "10.00".parse().expect("decimale"),
                expense_date: None,
                split_method: None,
                notes: None,
            },
        )
        .await
        .expect("spesa registrata");

    let share_of = |user: &RequestContext| {
        recorded
            .shares
            .iter()
            .find(|share| share.user_id() == user_of(user))
            .map(|share| share.share_amount().amount())
            .expect("quota presente")
    };
    assert_eq!(share_of(creator), "2.50".parse().expect("decimale"));
    assert_eq!(share_of(second), "2.50".parse().expect("decimale"));
    assert_eq!(share_of(third), "5.00".parse().expect("decimale"));

    let total: rust_decimal::Decimal = recorded
        .shares
        .iter()
        .map(|share| share.share_amount().amount())
        .sum();
    assert_eq!(total, "10.00".parse().expect("decimale"));

    // Il peso viaggia sulle quote **solo** perché la ripartizione è pesata.
    assert!(
        recorded
            .shares
            .iter()
            .all(|share| share.share_weight().is_some())
    );

    // E si rilegge dal database identico a come è stato scritto.
    let (_, from_db) = GetExpense::new(Arc::clone(&env.factory))
        .execute(creator, recorded.expense.id())
        .await
        .expect("spesa riletta");
    let reread: rust_decimal::Decimal = from_db
        .iter()
        .map(|share| share.share_amount().amount())
        .sum();
    assert_eq!(reread, total);
}

// ---------------------------------------------------------------------------
// I coinquilini si vedono
// ---------------------------------------------------------------------------

/// Una stanza sull'immobile indicato.
async fn a_room(
    env: &common::TestEnv,
    owner: &RequestContext,
    property_id: PropertyId,
    name: &str,
) -> RoomId {
    CreateRoom::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            owner,
            CreateRoomCommand {
                property_id,
                name: name.to_owned(),
                attributes: RoomChanges::default(),
            },
        )
        .await
        .expect("stanza creata")
        .id()
}

/// Un residente **per stanza**: contratto proprio, invito, accettazione,
/// attivazione.
///
/// È il co-living vero, e la ragione per cui questa fetta esiste: con un contratto
/// per stanza `can_access_lease` ammette solo il diretto interessato, quindi
/// nessuno dei coinquilini è parte del contratto degli altri.
///
/// Il contratto si redige **senza aggiungere una parte**: la crea `InviteTenant`,
/// con la quota residua — cioè 100 su un contratto vuoto — e `is_primary`. Una
/// parte aggiunta a mano prima dell'invito ne produrrebbe una seconda a quota
/// zero, senza utente, che comparirebbe nell'elenco dei residenti e non è ciò che
/// lo scenario vuole misurare.
async fn resident_of_room(
    env: &common::TestEnv,
    owner: &RequestContext,
    property_id: PropertyId,
    room_id: RoomId,
    label: &str,
) -> (RequestContext, propflow_domain::shared::LeaseId) {
    let view = DraftLease::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            owner,
            DraftLeaseCommand {
                property_id,
                room_id: Some(room_id),
                start_date: days_from_today(-30),
                end_date: Some(days_from_today(335)),
                rent: money("400.00"),
                deposit: None,
                rent_frequency: RentFrequency::Monthly,
                rent_due_day: 1,
                notice_months: None,
                auto_renew: false,
                renewal_months: None,
                utilities_included: false,
                istat_indexation: false,
                notes: None,
            },
        )
        .await
        .expect("bozza redatta");

    let tenant = tenant_on_lease(env, owner, view.lease.id(), label).await;
    activate(env, owner, view.lease.id())
        .await
        .expect("contratto attivato");
    (tenant, view.lease.id())
}

/// Lo scenario del piano: **due immobili**, cinque inquilini con contratti per
/// stanza. X, Y, Z nella Casa 1; A, B nella Casa 2, tutti nella stessa
/// organizzazione — che è ciò che l'accettazione di un invito produce.
struct TwoHomes {
    owner: RequestContext,
    home_one: PropertyId,
    home_two: PropertyId,
    /// X, Y, Z.
    first: Vec<RequestContext>,
    /// A, B.
    second: Vec<RequestContext>,
}

async fn two_homes(env: &common::TestEnv, label: &str) -> TwoHomes {
    let owner = register_owner(env, label).await;
    let home_one = create_property(env, &owner, "Casa 1", "Bologna").await;
    let home_two = create_property(env, &owner, "Casa 2", "Bologna").await;

    let mut first = Vec::with_capacity(3);
    for name in ["x", "y", "z"] {
        let room = a_room(env, &owner, home_one, &format!("Camera {name}")).await;
        let (tenant, _) =
            resident_of_room(env, &owner, home_one, room, &format!("{label}-{name}")).await;
        first.push(tenant);
    }
    let mut second = Vec::with_capacity(2);
    for name in ["a", "b"] {
        let room = a_room(env, &owner, home_two, &format!("Camera {name}")).await;
        let (tenant, _) =
            resident_of_room(env, &owner, home_two, room, &format!("{label}-{name}")).await;
        second.push(tenant);
    }

    TwoHomes {
        owner,
        home_one,
        home_two,
        first,
        second,
    }
}

async fn residents_seen_by(
    env: &common::TestEnv,
    context: &RequestContext,
    property_id: PropertyId,
) -> Vec<String> {
    let mut names: Vec<String> = ListPropertyResidents::new(Arc::clone(&env.factory))
        .execute(context, property_id)
        .await
        .expect("elenco dei residenti")
        .into_iter()
        .map(|resident| resident.full_name)
        .collect();
    names.sort();
    names
}

/// Chi non abita in una casa non ne ottiene un elenco vuoto: ottiene un `404`.
///
/// La distinzione conta ed è più forte di quella che il piano prevedeva. La
/// **funzione** restituisce zero righe — `can_access_property` è falsa — ma lo use
/// case rilegge l'immobile prima, e un immobile non visibile non esiste per chi
/// chiede. Un elenco vuoto direbbe «questa casa esiste e non ci abita nessuno»,
/// che è uno stato legittimo di un immobile sfitto e qui sarebbe una bugia utile a
/// chi va a tentoni.
async fn residents_denied_to(
    env: &common::TestEnv,
    context: &RequestContext,
    property_id: PropertyId,
) {
    let refused = ListPropertyResidents::new(Arc::clone(&env.factory))
        .execute(context, property_id)
        .await
        .expect_err("l'immobile non è visibile a questo contesto");
    assert!(
        matches!(refused, propflow_application::AppError::NotFound { .. }),
        "atteso 404, ottenuto {refused:?}"
    );
}

/// Quanti contratti vede questo contesto. È l'assert che dice che **nessun canone
/// altrui è diventato leggibile**: se restasse uno solo, nessuna riga di
/// `app.leases` in più è uscita, e `rent_amount` sta sulla riga.
async fn leases_seen_by(env: &common::TestEnv, context: &RequestContext) -> usize {
    ListLeases::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(context, ListLeasesQuery::default())
        .await
        .expect("elenco dei contratti")
        .items
        .len()
}

/// **La matrice dello scenario a due immobili, in entrambe le direzioni.**
///
/// È il criterio di completamento della fetta, e non la suite verde: X deve vedere
/// Y e Z e **non** vedere A e B, e in tutti e due i casi deve vedere **un solo
/// contratto**. Quest'ultimo assert è quello che dice che non si è pagato il
/// prezzo che la prima stesura del piano aveva accettato — allargare
/// `leases_select` avrebbe fatto funzionare l'elenco esponendo il canone dei
/// coinquilini, perché `leases.rent_amount` sta sulla riga e una policy è per riga
/// e non per colonna.
#[tokio::test]
async fn housemates_see_each_other_and_nobody_else() {
    let env = require_db!();
    let stage = two_homes(&env, "due-case").await;
    let (x, y, z) = (&stage.first[0], &stage.first[1], &stage.first[2]);
    let (a, b) = (&stage.second[0], &stage.second[1]);

    let seen_by_x = residents_seen_by(&env, x, stage.home_one).await;
    assert_eq!(
        seen_by_x.len(),
        3,
        "X deve vedere sé stesso, Y e Z: {seen_by_x:?}"
    );

    // L'altra casa **non esiste** per X: `can_access_property` è falsa, quindi la
    // rilettura dell'immobile dà `404` prima ancora che la funzione risponda zero
    // righe. Due confini in fila, e il più esterno è quello che parla.
    residents_denied_to(&env, x, stage.home_two).await;
    residents_denied_to(&env, a, stage.home_one).await;
    assert_eq!(
        residents_seen_by(&env, a, stage.home_two).await.len(),
        2,
        "A deve vedere sé stesso e B"
    );

    // Il proprietario vede entrambe le case, ed è staff: guarda, e non ne è parte.
    assert_eq!(
        residents_seen_by(&env, &stage.owner, stage.home_one)
            .await
            .len(),
        3
    );
    assert_eq!(
        residents_seen_by(&env, &stage.owner, stage.home_two)
            .await
            .len(),
        2
    );

    // **L'assert che dice che nessun canone è uscito.**
    for tenant in [x, y, z, a, b] {
        assert_eq!(
            leases_seen_by(&env, tenant).await,
            1,
            "ogni inquilino vede il proprio contratto e nessun altro: se ne vedesse \
             di più leggerebbe il canone dei coinquilini"
        );
    }
    assert_eq!(
        leases_seen_by(&env, &stage.owner).await,
        5,
        "lo staff vede tutto, come prima"
    );
}

/// **La trappola più cara del piano, riprodotta sul binario vero.**
///
/// Allargare una policy di lettura non basta se la query fa un join, e il modo in
/// cui non basta è silenzioso: nessun errore, un elenco che sembra corto.
///
/// Il test esegue nella stessa transazione le due versioni della stessa domanda:
/// la query con i join che si sarebbe scritta, e la funzione della `0030`. La
/// prima risponde **una** riga — il chiamante — perché `leases_select` è
/// `can_access_lease(id)` e rifiltra l'unione; la seconda ne risponde tre.
#[tokio::test]
async fn the_join_that_looks_right_refilters_itself() {
    let env = require_db!();
    let stage = two_homes(&env, "trappola-join").await;
    let x = &stage.first[0];

    let mut tx = env
        .pools
        .application()
        .begin()
        .await
        .expect("transazione applicativa");
    sqlx::query("SELECT app.set_request_context($1, $2, false, NULL, NULL, NULL, 'tenant')")
        .bind(user_of(x).as_uuid())
        .bind(x.org_id().expect("org").as_uuid())
        .execute(&mut *tx)
        .await
        .expect("contesto installato");

    let (with_join, from_function): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*)
                   FROM app.lease_parties lp
                   JOIN app.leases l ON l.id = lp.lease_id
                  WHERE l.property_id = $1
                    AND l.deleted_at IS NULL
                    AND lp.deleted_at IS NULL
                    AND app.lease_in_force(l.status, l.start_date, l.end_date)),
                (SELECT count(*) FROM app.property_residents($1))",
    )
    .bind(stage.home_one.as_uuid())
    .fetch_one(&mut *tx)
    .await
    .expect("query eseguita");

    assert_eq!(
        with_join, 1,
        "la query con i join passa da `leases_select`, che è per contratto: \
         qualunque allargamento di `lease_parties_select` viene rifiltrato qui, \
         e il sintomo è un elenco corto senza alcun errore"
    );
    assert_eq!(
        from_function, 3,
        "la funzione `SECURITY DEFINER` legge il fatto vero e ne proietta tre \
         colonne: è la ragione per cui la `0030` non è un `ALTER POLICY`"
    );
}

/// **La seconda metà, verificata al contrario.**
///
/// Restringere `users_select` non fa fallire nessun test, e questo è il pericolo:
/// il verde prova che nessuno usa quella visibilità, non che restringere sia
/// sicuro. La verifica vera è questa — il predicato vecchio e quello nuovo,
/// valutati fianco a fianco sulla stessa persona.
///
/// `shares_org_with(A)` resta **vero** per X: la funzione della `0014` esiste
/// ancora e A è nella stessa organizzazione. `shares_home_with(A)` è **falso**,
/// perché A abita altrove. Prima della `0030` la policy usava il primo, e X
/// leggeva la riga di A con nome, **email** e telefono.
#[tokio::test]
async fn the_anagraphic_perimeter_is_the_home_and_not_the_organization() {
    let env = require_db!();
    let stage = two_homes(&env, "perimetro-casa").await;
    let (x, y) = (&stage.first[0], &stage.first[1]);
    let a = &stage.second[0];

    let mut tx = env
        .pools
        .application()
        .begin()
        .await
        .expect("transazione applicativa");
    sqlx::query("SELECT app.set_request_context($1, $2, false, NULL, NULL, NULL, 'tenant')")
        .bind(user_of(x).as_uuid())
        .bind(x.org_id().expect("org").as_uuid())
        .execute(&mut *tx)
        .await
        .expect("contesto installato");

    let (org_with_a, home_with_a, home_with_y, staff_of_owner): (bool, bool, bool, bool) =
        sqlx::query_as(
            "SELECT app.shares_org_with($1),
                    app.shares_home_with($1),
                    app.shares_home_with($2),
                    app.is_my_staff($3)",
        )
        .bind(user_of(a).as_uuid())
        .bind(user_of(y).as_uuid())
        .bind(user_of(&stage.owner).as_uuid())
        .fetch_one(&mut *tx)
        .await
        .expect("query eseguita");

    assert!(
        org_with_a,
        "il predicato vecchio è ancora vero: A è nella stessa organizzazione, ed è \
         esattamente ciò che rendeva la sua riga leggibile a X"
    );
    assert!(
        !home_with_a,
        "il predicato nuovo è falso: A abita altrove, e la sua riga porta nome, \
         email e telefono"
    );
    assert!(home_with_y, "Y abita con X");
    assert!(
        staff_of_owner,
        "senza questo ramo un inquilino non potrebbe mai sapere il nome del \
         proprio proprietario, e `properties.owner_id` punterebbe a una riga \
         illeggibile"
    );

    // E la policy, non solo le funzioni: chi è leggibile davvero.
    let readable: Vec<uuid::Uuid> =
        sqlx::query_scalar("SELECT id FROM app.users WHERE deleted_at IS NULL AND id = ANY($1)")
            .bind(
                [x, y, a, &stage.owner]
                    .map(|context| user_of(context).as_uuid())
                    .to_vec(),
            )
            .fetch_all(&mut *tx)
            .await
            .expect("query eseguita");

    assert!(readable.contains(&user_of(x).as_uuid()), "sé stesso");
    assert!(readable.contains(&user_of(y).as_uuid()), "il coinquilino");
    assert!(
        readable.contains(&user_of(&stage.owner).as_uuid()),
        "il proprio proprietario"
    );
    assert!(
        !readable.contains(&user_of(a).as_uuid()),
        "chi abita in un'altra casa: è la riga che prima usciva, ed è il pavimento \
         che la `0030` mette"
    );
}

/// **`is_my_staff` e `is_staff_of_user` non sono la stessa funzione al contrario.**
///
/// Scambiarle produce una policy che sembra funzionare finché il chiamante non è
/// un inquilino, ed è la trappola che il piano segnalava. Qui le due si valutano
/// dal contesto di un inquilino, dove danno risposte **opposte** sulla stessa
/// persona.
#[tokio::test]
async fn the_two_staff_predicates_are_not_each_other_backwards() {
    let env = require_db!();
    let stage = two_homes(&env, "due-predicati").await;
    let x = &stage.first[0];

    let mut tx = env
        .pools
        .application()
        .begin()
        .await
        .expect("transazione applicativa");
    sqlx::query("SELECT app.set_request_context($1, $2, false, NULL, NULL, NULL, 'tenant')")
        .bind(user_of(x).as_uuid())
        .bind(x.org_id().expect("org").as_uuid())
        .execute(&mut *tx)
        .await
        .expect("contesto installato");

    let (my_staff, staff_of): (bool, bool) =
        sqlx::query_as("SELECT app.is_my_staff($1), app.is_staff_of_user($1)")
            .bind(user_of(&stage.owner).as_uuid())
            .fetch_one(&mut *tx)
            .await
            .expect("query eseguita");

    assert!(
        my_staff,
        "il proprietario è staff di un'organizzazione a cui l'inquilino appartiene"
    );
    assert!(
        !staff_of,
        "l'inquilino non è staff di nulla: se le due funzioni fossero scambiate, \
         questa policy negherebbe all'inquilino il nome del proprio proprietario"
    );
}

/// **Un contratto scaduto non toglie la parte contrattuale**, ma toglie il
/// residente.
///
/// Senza il filtro su `app.lease_in_force` un inquilino di due anni fa comparirebbe
/// ancora fra i residenti: lo stato memorizzato resta `active` anche dopo la
/// scadenza, perché nessuno scheduler lo aggiorna (ADR-019). Il test verifica
/// **entrambe** le direzioni: la parte contrattuale c'è ancora, e il residente no.
#[tokio::test]
async fn an_expired_lease_leaves_the_party_and_takes_the_resident() {
    let env = require_db!();
    let owner = register_owner(&env, "residente-scaduto").await;
    let property_id = create_property(&env, &owner, "Casa con storia", "Bologna").await;
    let room = a_room(&env, &owner, property_id, "Camera scaduta").await;

    let view = DraftLease::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            DraftLeaseCommand {
                property_id,
                room_id: Some(room),
                start_date: days_from_today(-800),
                end_date: Some(days_from_today(-400)),
                rent: money("400.00"),
                deposit: None,
                rent_frequency: RentFrequency::Monthly,
                rent_due_day: 1,
                notice_months: None,
                auto_renew: false,
                renewal_months: None,
                utilities_included: false,
                istat_indexation: false,
                notes: None,
            },
        )
        .await
        .expect("bozza redatta");
    tenant_on_lease(&env, &owner, view.lease.id(), "residente-scaduto-uno").await;
    activate(&env, &owner, view.lease.id())
        .await
        .expect("attivato: l'attivazione non guarda le date");

    // La parte contrattuale c'è, e lo stato memorizzato è ancora `active`.
    let mut uow = env.factory.begin(&owner).await.expect("transazione aperta");
    let parties = uow
        .lease_parties()
        .list_for_lease(view.lease.id())
        .await
        .expect("parti lette");
    assert_eq!(parties.len(), 1, "la parte contrattuale resta");
    let stored = uow
        .leases()
        .find_by_id(view.lease.id())
        .await
        .expect("query riuscita")
        .expect("contratto visibile");
    assert_eq!(
        stored.status(),
        LeaseStatus::Active,
        "nessuno scheduler porta lo stato a `expired`: è il difetto per cui \
         `app.lease_in_force` esiste"
    );
    drop(uow);

    assert!(
        residents_seen_by(&env, &owner, property_id)
            .await
            .is_empty(),
        "un contratto terminato non produce residenti"
    );
}

/// **`lease_parties.user_id` è annullabile, e l'elenco lo dice invece di
/// ometterlo.**
///
/// Un residente «invitato e non ancora entrato» ha un nome e nessun
/// identificatore, quindi non è aggiungibile a un gruppo di spesa. Ometterlo
/// lascerebbe chi guarda senza una spiegazione per una persona che sa di avere in
/// casa e non trova nell'elenco.
///
/// Nello stesso scenario si verificano le altre due assenze: il **garante** non
/// abita lì, e il **proprietario** nemmeno — quest'ultimo vede l'elenco perché è
/// staff, e non ne fa parte.
#[tokio::test]
async fn the_invited_appear_without_an_identifier_and_the_guarantor_not_at_all() {
    let env = require_db!();
    let owner = register_owner(&env, "invitato-e-garante").await;
    let property_id = create_property(&env, &owner, "Casa con garante", "Bologna").await;
    let room = a_room(&env, &owner, property_id, "Camera unica").await;

    let view = DraftLease::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            DraftLeaseCommand {
                property_id,
                room_id: Some(room),
                start_date: days_from_today(-30),
                end_date: Some(days_from_today(335)),
                rent: money("400.00"),
                deposit: None,
                rent_frequency: RentFrequency::Monthly,
                rent_due_day: 1,
                notice_months: None,
                auto_renew: false,
                renewal_months: None,
                utilities_included: false,
                istat_indexation: false,
                notes: None,
            },
        )
        .await
        .expect("bozza redatta");

    // L'invitato: la parte esiste con nome ed email, `user_id` resta nullo finché
    // non accetta. L'invito si emette e **non** si accetta.
    InviteTenant::new(
        Arc::clone(&env.factory),
        Arc::new(OpaqueTokenGenerator),
        Arc::clone(&env.clock) as _,
        Arc::new(SilentNotifier),
    )
    .execute(
        &owner,
        InviteTenantCommand {
            lease_id: view.lease.id(),
            email: propflow_domain::shared::Email::parse(common::unique_email("mai-accettato"))
                .expect("email valida"),
            full_name: "Invitato Non Registrato".to_owned(),
        },
    )
    .await
    .expect("invito emesso");

    AddLeaseParty::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &owner,
            AddLeasePartyCommand {
                lease_id: view.lease.id(),
                role: LeasePartyRole::Guarantor,
                full_name: "Garante Che Non Abita".to_owned(),
                email: None,
                share_percent: rust_decimal::Decimal::ZERO,
                is_primary: false,
            },
        )
        .await
        .expect("garante aggiunto");
    activate(&env, &owner, view.lease.id())
        .await
        .expect("attivato");

    let residents = ListPropertyResidents::new(Arc::clone(&env.factory))
        .execute(&owner, property_id)
        .await
        .expect("elenco dei residenti");

    assert_eq!(
        residents.len(),
        1,
        "un solo residente: l'invitato. Il garante non abita lì e il proprietario \
         nemmeno — lo vede perché è staff, non perché ne fa parte: {residents:?}"
    );
    let invited = &residents[0];
    assert_eq!(invited.full_name, "Invitato Non Registrato");
    assert!(
        invited.user_id.is_none(),
        "`user_id` resta nullo finché l'invito non è accettato"
    );
    assert!(
        !invited.is_addressable(),
        "senza identificatore non è aggiungibile a un gruppo di spesa, e l'elenco \
         deve poterlo dire"
    );
    assert_eq!(
        invited.room_name.as_deref(),
        Some("Camera unica"),
        "la stanza viene dal contratto e serve a riconoscere chi si ha in casa"
    );
}

/// L'immobile di un'altra organizzazione non ha residenti, e non perché sia vuoto.
///
/// La funzione è `SECURITY DEFINER`, quindi legge senza policy: senza l'ancora
/// `p.org_id = app.current_org_id()` e senza `can_access_property` risponderebbe
/// con le righe di un immobile altrui. È lo stesso criterio della `0026`.
#[tokio::test]
async fn a_property_of_another_organization_has_no_residents() {
    let env = require_db!();
    let stage = two_homes(&env, "iso-residenti").await;
    let stranger = register_owner(&env, "estraneo-residenti").await;

    // `404` e non un elenco vuoto: le due cose sono diverse per il client, e un
    // elenco vuoto suggerirebbe che l'immobile esiste e non ha residenti.
    residents_denied_to(&env, &stranger, stage.home_one).await;

    // E la funzione da sola, senza la rilettura dello use case, risponde comunque
    // zero righe: il confine è dentro, non nel service layer.
    let mut tx = env
        .pools
        .application()
        .begin()
        .await
        .expect("transazione applicativa");
    sqlx::query("SELECT app.set_request_context($1, $2, false, NULL, NULL, NULL, 'owner')")
        .bind(user_of(&stranger).as_uuid())
        .bind(stranger.org_id().expect("org").as_uuid())
        .execute(&mut *tx)
        .await
        .expect("contesto installato");
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM app.property_residents($1)")
        .bind(stage.home_one.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .expect("query eseguita");
    assert_eq!(
        rows, 0,
        "la funzione è SECURITY DEFINER e legge senza policy: se il confine non          fosse nel suo corpo, questa query restituirebbe i residenti di casa altrui"
    );
}

// ---------------------------------------------------------------------------
// Lettura automatica delle bollette
// ---------------------------------------------------------------------------

/// Carica una scansione su una bolletta e restituisce il documento.
///
/// Passa da `UploadDocument` e non da una `INSERT` a mano perché **è
/// l'innesco**: il job nasce dentro quella transazione, e scriverlo a mano
/// verificherebbe una cosa che il codice non fa.
async fn attach_scan(
    env: &common::TestEnv,
    storage: &Arc<MemoryStorage>,
    context: &RequestContext,
    bill_id: propflow_domain::shared::UtilityBillId,
    content: &str,
) -> propflow_domain::document::Document {
    UploadDocument::new(
        Arc::clone(&env.factory),
        Arc::clone(storage) as Arc<dyn propflow_application::ports::ObjectStorage>,
        Arc::clone(&env.clock) as _,
        propflow_infrastructure::DeterministicOcrProvider::NAME,
    )
    .execute(
        context,
        UploadDocumentCommand {
            owner: AttachmentOwner::UtilityBill(bill_id),
            kind: DocumentKind::UtilityBill,
            original_filename: "bolletta.txt".to_owned(),
            mime_type: "text/plain".to_owned(),
            content: content.as_bytes().to_vec(),
        },
    )
    .await
    .expect("scansione allegata")
}

/// Una bolletta pulita, che l'adattatore deterministico legge per intero.
const CLEAN_SCAN: &str = "\
Fornitore: Enel Energia
Data emissione: 01/08/2026
Scadenza: 31/08/2026
Totale: 184,20 €
Consumo: 123,4567 Smc
";

fn ocr_loop(env: &common::TestEnv, storage: &Arc<MemoryStorage>) -> ProcessAiJob {
    ProcessAiJob::new(
        Arc::new(PgWorkerUnitOfWorkFactory::new(
            env.pools.privileged().clone(),
        )),
        Arc::clone(storage) as Arc<dyn propflow_application::ports::ObjectStorage>,
        Arc::new(propflow_infrastructure::DeterministicOcrProvider),
        Arc::clone(&env.clock) as _,
    )
}

/// Fa girare il ciclo finché il job di **questa** bolletta non è concluso, e lo
/// restituisce.
///
/// Un giro solo non basta, e la ragione vale la pena scriverla perché è la stessa
/// che rende il worker corretto: `claim_next` **non filtra per organizzazione** —
/// il worker serve tutte, e deve — quindi in una suite che gira in parallelo sullo
/// stesso database un giro può benissimo prendere il job di un altro test. Un test
/// che contasse i giri verificherebbe l'ordine di esecuzione della suite invece
/// del comportamento del ciclo.
///
/// Aspettare il proprio job è anche più fedele a come il worker si comporta
/// davvero: un ciclo che non si ferma finché la coda non è vuota. Il tetto sui
/// giri c'è perché un test che non finisce è peggio di un test che fallisce.
async fn work_until_settled(
    env: &common::TestEnv,
    storage: &Arc<MemoryStorage>,
    context: &RequestContext,
    bill_id: propflow_domain::shared::UtilityBillId,
) -> propflow_domain::ai::AiJob {
    const MAX_TURNS: usize = 64;
    let ocr = ocr_loop(env, storage);
    for _ in 0..MAX_TURNS {
        if let Some(job) = job_of(env, context, bill_id).await
            && job.status().is_terminal()
        {
            return job;
        }
        if ocr
            .execute(AiTask::BillOcr)
            .await
            .expect("il ciclo ha girato")
            == Processed::Nothing
        {
            // Coda vuota e il nostro job non è concluso: o non è mai stato
            // accodato, o qualcuno lo ha lasciato `running`. In entrambi i casi
            // il test deve dirlo invece di girare a vuoto.
            break;
        }
    }
    panic!("il job della bolletta {bill_id} non si è concluso in {MAX_TURNS} giri")
}

async fn job_of(
    env: &common::TestEnv,
    context: &RequestContext,
    bill_id: propflow_domain::shared::UtilityBillId,
) -> Option<propflow_domain::ai::AiJob> {
    let mut uow = env.factory.begin(context).await.expect("transazione");
    uow.ai_jobs()
        .find_latest_for_subject(AttachmentOwner::UtilityBill(bill_id))
        .await
        .expect("job letto")
}

async fn bill_of(
    env: &common::TestEnv,
    context: &RequestContext,
    bill_id: propflow_domain::shared::UtilityBillId,
) -> propflow_domain::billing::UtilityBill {
    let mut uow = env.factory.begin(context).await.expect("transazione");
    uow.utility_bills()
        .find_by_id(bill_id)
        .await
        .expect("bolletta letta")
        .expect("la bolletta c'è")
}

/// **La trappola della `0031`, verificata al contrario.**
///
/// Non basta che l'accodamento funzioni: bisogna vedere che ciò che la policy
/// esclude viene davvero escluso. Il ruolo applicativo ha il privilegio `INSERT`
/// dalla `0015` e, prima di questa migration, **nessuna policy** — quindi
/// l'inserimento era negato mentre `has_table_privilege` diceva di sì.
///
/// Qui si verificano i tre confini che la policy disegna, scritti in SQL diretto
/// e non attraverso il repository: il repository può solo produrre job `queued`
/// nella propria organizzazione, quindi non saprebbe nemmeno tentare gli altri due.
#[tokio::test]
async fn the_enqueue_policy_admits_a_queued_job_and_nothing_else() {
    let env = require_db!();
    let storage = Arc::new(MemoryStorage::default());
    let context = register_owner(&env, "ocr-policy").await;
    let property = a_property(&env, &context, "Bologna").await;
    let bill = register_bill(&env, &context, property, None, "2026/OCR/0001").await;
    let _ = &storage;

    let org_id = context.org_id().expect("org").as_uuid();
    let user_id = user_of(&context).as_uuid();

    // (a) un job `queued` dello staff, nella propria organizzazione: ammesso.
    let mut tx = env
        .pools
        .application()
        .begin()
        .await
        .expect("transazione applicativa");
    sqlx::query("SELECT app.set_request_context($1, $2, false, NULL, NULL, NULL, 'owner')")
        .bind(user_id)
        .bind(org_id)
        .execute(&mut *tx)
        .await
        .expect("contesto installato");
    let accepted = sqlx::query(
        "INSERT INTO app.ai_jobs (org_id, capability, task, status, provider_name,
                                  subject_type, subject_id)
         VALUES ($1, 'ocr', 'bill_ocr', 'queued', 'deterministic',
                 'utility_bill', $2)",
    )
    .bind(org_id)
    .bind(bill.id().as_uuid())
    .execute(&mut *tx)
    .await;
    assert!(accepted.is_ok(), "l'accodamento dello staff deve riuscire");

    // (b) lo stesso job che si dichiara già concluso: rifiutato. È la ragione per
    // cui `status = 'queued'` sta nella WITH CHECK e non solo nel codice — senza,
    // una query scritta a mano potrebbe far dire alla macchina quello che vuole
    // chi chiama.
    let declared_done = sqlx::query(
        "INSERT INTO app.ai_jobs (org_id, capability, task, status, provider_name,
                                  subject_type, subject_id, finished_at, output)
         VALUES ($1, 'ocr', 'bill_ocr', 'succeeded', 'deterministic',
                 'utility_bill', $2, now(), '{\"scritto\":\"a mano\"}'::jsonb)",
    )
    .bind(org_id)
    .bind(bill.id().as_uuid())
    .execute(&mut *tx)
    .await;
    assert!(
        declared_done.is_err(),
        "un job che nasce `succeeded` deve essere rifiutato dalla policy"
    );
    drop(tx);

    // (c) un job in un'altra organizzazione: rifiutato.
    let stranger = register_owner(&env, "ocr-policy-altrove").await;
    let mut tx = env
        .pools
        .application()
        .begin()
        .await
        .expect("transazione applicativa");
    sqlx::query("SELECT app.set_request_context($1, $2, false, NULL, NULL, NULL, 'owner')")
        .bind(user_of(&stranger).as_uuid())
        .bind(stranger.org_id().expect("org").as_uuid())
        .execute(&mut *tx)
        .await
        .expect("contesto installato");
    let cross_org = sqlx::query(
        "INSERT INTO app.ai_jobs (org_id, capability, task, status, provider_name,
                                  subject_type, subject_id)
         VALUES ($1, 'ocr', 'bill_ocr', 'queued', 'deterministic',
                 'utility_bill', $2)",
    )
    .bind(org_id)
    .bind(bill.id().as_uuid())
    .execute(&mut *tx)
    .await;
    assert!(
        cross_org.is_err(),
        "accodare lavoro nell'organizzazione di un altro deve essere rifiutato"
    );
}

/// L'altra metà della policy: **il ruolo applicativo non può aggiornare un job**,
/// e il rifiuto è silenzioso.
///
/// La `0031` aggiunge una policy di `INSERT` e nessuna di `UPDATE`. Con RLS attiva
/// e nessuna policy per un comando, quel comando non solleva: la riga
/// semplicemente non è visibile all'aggiornamento, e il risultato è `UPDATE 0`.
/// Va verificato apposta, perché un `UPDATE` che non fa niente e non protesta è
/// il modo più facile di credere che una difesa non ci sia.
#[tokio::test]
async fn the_application_role_cannot_rewrite_what_a_machine_recorded() {
    let env = require_db!();
    let context = register_owner(&env, "ocr-update").await;
    let property = a_property(&env, &context, "Ancona").await;
    let bill = register_bill(&env, &context, property, None, "2026/OCR/0002").await;

    let mut tx = env
        .pools
        .application()
        .begin()
        .await
        .expect("transazione applicativa");
    sqlx::query("SELECT app.set_request_context($1, $2, false, NULL, NULL, NULL, 'owner')")
        .bind(user_of(&context).as_uuid())
        .bind(context.org_id().expect("org").as_uuid())
        .execute(&mut *tx)
        .await
        .expect("contesto installato");
    sqlx::query(
        "INSERT INTO app.ai_jobs (id, org_id, capability, task, status, provider_name,
                                  subject_type, subject_id)
         VALUES ($1, $2, 'ocr', 'bill_ocr', 'queued', 'deterministic',
                 'utility_bill', $3)",
    )
    .bind(uuid::Uuid::now_v7())
    .bind(context.org_id().expect("org").as_uuid())
    .bind(bill.id().as_uuid())
    .execute(&mut *tx)
    .await
    .expect("accodato");

    let updated = sqlx::query(
        "UPDATE app.ai_jobs SET status = 'succeeded', finished_at = now()
          WHERE subject_id = $1",
    )
    .bind(bill.id().as_uuid())
    .execute(&mut *tx)
    .await
    .expect("l'UPDATE non solleva: nega producendo zero righe");
    assert_eq!(
        updated.rows_affected(),
        0,
        "senza una policy di UPDATE la riga non è aggiornabile, e il rifiuto è muto"
    );
}

/// Il giro completo, attraverso i due strati che lo compongono davvero:
/// l'accodamento dall'API e il ciclo dal worker.
#[tokio::test]
async fn a_scan_uploaded_to_a_draft_brings_the_bill_under_review() {
    let env = require_db!();
    let _gate = common::lock_ocr_queue(&env.pools).await;
    let storage = Arc::new(MemoryStorage::default());
    let context = register_owner(&env, "ocr-giro").await;
    let property = a_property(&env, &context, "Padova").await;
    let bill = register_bill(&env, &context, property, None, "2026/OCR/0010").await;

    let document = attach_scan(&env, &storage, &context, bill.id(), CLEAN_SCAN).await;

    // Accodato **nella stessa transazione** del documento: appena il caricamento
    // è tornato, il job c'è già.
    let queued = job_of(&env, &context, bill.id())
        .await
        .expect("job accodato");
    assert_eq!(queued.status(), AiJobStatus::Queued);
    assert_eq!(queued.task(), AiTask::BillOcr);
    assert_eq!(queued.input_document_id(), Some(document.id()));
    assert_eq!(queued.attempts(), 0, "nessuno lo ha ancora preso in carico");
    // La bolletta **non si è mossa**: il caricamento non la tocca.
    assert_eq!(
        bill_of(&env, &context, bill.id()).await.status(),
        BillStatus::Draft
    );

    let job = work_until_settled(&env, &storage, &context, bill.id()).await;
    assert_eq!(job.status(), AiJobStatus::Succeeded);
    assert_eq!(job.attempts(), 1);
    assert!(job.started_at().is_some());
    assert!(
        job.finished_at().is_some(),
        "ai_jobs_finished_ck rifiuterebbe la riga senza la data"
    );
    assert!(job.failure().is_none());

    let reviewed = bill_of(&env, &context, bill.id()).await;
    assert_eq!(reviewed.status(), BillStatus::PendingReview);
    assert_eq!(reviewed.ai_job_id(), Some(job.id()));
    let reading = reviewed
        .ocr_extracted()
        .expect("la proposta è sulla bolletta");
    assert_eq!(reading.fields().len(), 5);
    assert_eq!(
        reviewed.ocr_confidence(),
        reading.overall_confidence(),
        "la confidenza della bolletta è quella complessiva della lettura"
    );

    // **Il confine fra le due colonne JSONB**, verificato su righe vere: il job
    // porta la forma, la bolletta porta i valori.
    assert_eq!(job.output().len(), 5);
    let stored_output: serde_json::Value =
        sqlx::query_scalar("SELECT output FROM app.ai_jobs WHERE id = $1")
            .bind(job.id().as_uuid())
            .fetch_one(env.pools.privileged())
            .await
            .expect("output letto");
    let rendered = stored_output.to_string();
    assert!(
        !rendered.contains("Enel Energia") && !rendered.contains("184.20"),
        "un valore della bolletta è finito in ai_jobs.output: {rendered}"
    );
    // …e la forma è interrogabile con il contenimento, che è ciò per cui
    // `ai_jobs_output_ix` è `gin (output jsonb_path_ops)`.
    let found: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM app.ai_jobs
          WHERE output @> '{\"fields\":[\"total_amount\"]}'::jsonb AND id = $1",
    )
    .bind(job.id().as_uuid())
    .fetch_one(env.pools.privileged())
    .await
    .expect("query di contenimento");
    assert_eq!(found, 1);

    // **I campi della bolletta non sono stati toccati.** È la decisione 4 del
    // piano, ed è la ragione per cui l'accuratezza si potrà misurare.
    assert_eq!(reviewed.supplier_name(), bill.supplier_name());
    assert_eq!(reviewed.total_amount(), bill.total_amount());
    assert_eq!(reviewed.invoice_number(), bill.invoice_number());

    // Il job non si riprende: è concluso, e `claim_next` filtra su `queued`.
    assert!(job.status().is_terminal());
}

/// **Il fallimento verificato quanto il successo.**
///
/// Un documento che il fornitore rifiuta lascia il job `failed` con il suo
/// `error_code` e la bolletta **in `draft`** — dove era, con il suo allegato,
/// correggibile a mano come se l'OCR non esistesse.
#[tokio::test]
async fn a_refused_document_fails_the_job_and_leaves_the_bill_in_draft() {
    let env = require_db!();
    let _gate = common::lock_ocr_queue(&env.pools).await;
    let storage = Arc::new(MemoryStorage::default());
    let context = register_owner(&env, "ocr-fallimento").await;
    let property = a_property(&env, &context, "Genova").await;
    let bill = register_bill(&env, &context, property, None, "2026/OCR/0020").await;

    attach_scan(
        &env,
        &storage,
        &context,
        bill.id(),
        "Questa non è una bolletta, è una lettera.\n",
    )
    .await;

    let job = work_until_settled(&env, &storage, &context, bill.id()).await;
    assert_eq!(job.status(), AiJobStatus::Failed);
    assert_eq!(
        job.failure().map(|failure| failure.code),
        Some(AiJobErrorCode::UnsupportedDocument)
    );
    assert!(
        job.failure().is_some_and(|f| !f.message.is_empty()),
        "il messaggio del fornitore serve a capire perché"
    );
    assert!(job.finished_at().is_some());
    assert_eq!(job.attempts(), 1, "ci ha provato una volta");
    assert!(
        job.confidence().is_none() && job.output().is_empty(),
        "un fallimento non è una lettura a confidenza zero"
    );

    let untouched = bill_of(&env, &context, bill.id()).await;
    assert_eq!(
        untouched.status(),
        BillStatus::Draft,
        "la bolletta resta dov'era"
    );
    assert_eq!(untouched.ai_job_id(), None);
    assert!(untouched.ocr_extracted().is_none());
    assert!(untouched.ocr_confidence().is_none());
}

/// **La decisione che il piano lasciava aperta**: un job su un documento
/// archiviato *fallisce*.
///
/// Il worker gira con `BYPASSRLS` e la riga la vedrebbe comunque. Fallire è la
/// scelta, perché altrimenti la macchina legge ciò che una persona ha deciso di
/// togliere — e glielo ripropone come se fosse ancora lì.
#[tokio::test]
async fn a_job_on_an_archived_document_fails_instead_of_reading_it_anyway() {
    let env = require_db!();
    let _gate = common::lock_ocr_queue(&env.pools).await;
    let storage = Arc::new(MemoryStorage::default());
    let context = register_owner(&env, "ocr-archiviato").await;
    let property = a_property(&env, &context, "Verona").await;
    let bill = register_bill(&env, &context, property, None, "2026/OCR/0030").await;

    let document = attach_scan(&env, &storage, &context, bill.id(), CLEAN_SCAN).await;

    // Archiviato **dopo** l'accodamento, cioè mentre il job era in coda.
    ArchiveDocument::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&context, ArchiveDocumentCommand { id: document.id() })
        .await
        .expect("documento archiviato");

    // I byte sono ancora nel bucket: il soft delete non tocca lo storage. Se il
    // worker leggesse senza guardare la riga, la lettura riuscirebbe.
    assert!(
        storage.get(document.storage().object_key()).await.is_ok(),
        "l'oggetto c'è ancora: è la riga a essere archiviata"
    );

    let job = work_until_settled(&env, &storage, &context, bill.id()).await;
    assert_eq!(job.status(), AiJobStatus::Failed);
    assert_eq!(
        job.failure().map(|failure| failure.code),
        Some(AiJobErrorCode::DocumentArchived)
    );
    assert_eq!(
        bill_of(&env, &context, bill.id()).await.status(),
        BillStatus::Draft
    );
}

/// **`SKIP LOCKED` verificato con DUE worker**, che è l'unico modo di verificarlo.
///
/// Due transazioni concorrenti che chiamano `claim_next`:
/// - con **due** job in coda devono prenderne uno ciascuno, diversi;
/// - con **uno** solo, il secondo deve ottenere `None` invece di aspettare.
///
/// Con un worker solo questo test passerebbe anche senza `SKIP LOCKED`, ed è
/// precisamente la verifica che non verificherebbe niente.
#[tokio::test]
async fn two_workers_never_claim_the_same_job() {
    let env = require_db!();
    let _gate = common::lock_ocr_queue(&env.pools).await;
    let storage = Arc::new(MemoryStorage::default());
    let context = register_owner(&env, "ocr-skip-locked").await;
    let property = a_property(&env, &context, "Livorno").await;

    let first = register_bill(&env, &context, property, None, "2026/OCR/0040").await;
    let second = register_bill(&env, &context, property, None, "2026/OCR/0041").await;
    attach_scan(&env, &storage, &context, first.id(), CLEAN_SCAN).await;
    attach_scan(&env, &storage, &context, second.id(), CLEAN_SCAN).await;

    let factory = PgWorkerUnitOfWorkFactory::new(env.pools.privileged().clone());

    // Due transazioni **aperte insieme**: è la simultaneità a rendere il test
    // significativo, non due chiamate in sequenza.
    let mut worker_one = factory.begin().await.expect("primo worker");
    let mut worker_two = factory.begin().await.expect("secondo worker");

    let claimed_one = worker_one
        .ai_jobs()
        .claim_next(AiTask::BillOcr)
        .await
        .expect("primo claim")
        .expect("un job c'è");
    let claimed_two = worker_two
        .ai_jobs()
        .claim_next(AiTask::BillOcr)
        .await
        .expect("secondo claim")
        .expect("il secondo job c'è");

    assert_ne!(
        claimed_one.id(),
        claimed_two.id(),
        "due worker hanno preso lo stesso job: SKIP LOCKED non sta funzionando"
    );

    // Con entrambi i job bloccati, un terzo worker non trova niente — e **non si
    // blocca ad aspettare**, che è l'altra metà di ciò che SKIP LOCKED compra.
    let mut worker_three = factory.begin().await.expect("terzo worker");
    let nothing = worker_three
        .ai_jobs()
        .claim_next(AiTask::BillOcr)
        .await
        .expect("terzo claim");
    assert!(
        nothing.is_none(),
        "la coda è tutta bloccata: il terzo worker deve saltare, non aspettare"
    );

    drop(worker_one);
    drop(worker_two);
    drop(worker_three);
}

/// Il piano diceva «l'OCR non scrive i campi della bolletta»: qui si verifica
/// l'altra metà, cioè che **correggere a mano non cancella la proposta** — e che
/// la bolletta **resta** in `pending_review`, che è la decisione presa eseguendo.
#[tokio::test]
async fn correcting_a_proposal_keeps_it_and_leaves_the_bill_under_review() {
    let env = require_db!();
    let _gate = common::lock_ocr_queue(&env.pools).await;
    let storage = Arc::new(MemoryStorage::default());
    let context = register_owner(&env, "ocr-correzione").await;
    let property = a_property(&env, &context, "Parma").await;
    let bill = register_bill(&env, &context, property, None, "2026/OCR/0050").await;

    attach_scan(&env, &storage, &context, bill.id(), CLEAN_SCAN).await;
    work_until_settled(&env, &storage, &context, bill.id()).await;

    let proposal = bill_of(&env, &context, bill.id())
        .await
        .ocr_extracted()
        .cloned()
        .expect("la proposta c'è");

    // Una persona corregge il totale.
    let corrected = UpdateUtilityBill::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &context,
            bill.id(),
            UtilityBillChanges {
                total_amount: Some("190.00".parse().expect("decimale")),
                ..UtilityBillChanges::default()
            },
        )
        .await
        .expect("correzione ammessa");

    assert_eq!(
        corrected.status(),
        BillStatus::PendingReview,
        "correggere una proposta è la revisione, non il suo annullamento"
    );
    // La proposta sopravvive alla scrittura, riletta dal database.
    let reread = bill_of(&env, &context, bill.id()).await;
    assert_eq!(reread.ocr_extracted(), Some(&proposal));
    assert_eq!(
        reread.total_amount().amount(),
        "190.00".parse().expect("dec")
    );

    // …e da lì si valida, che prima del passo 3 era impossibile.
    let validated = ValidateUtilityBill::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&context, bill.id())
        .await
        .expect("validazione ammessa da pending_review");
    assert_eq!(validated.status(), BillStatus::Validated);
    assert!(validated.validated_at().is_some());
    assert!(
        bill_of(&env, &context, bill.id())
            .await
            .ocr_extracted()
            .is_some(),
        "validare non cancella ciò che la macchina aveva proposto"
    );
}

/// Una bolletta che **non è più una bozza** quando il job arriva in fondo: il job
/// si chiude `failed` con `subject_not_processable`, e nessuno riporta indietro
/// una decisione già presa da una persona.
#[tokio::test]
async fn a_bill_validated_while_the_job_was_queued_is_not_dragged_back() {
    let env = require_db!();
    let _gate = common::lock_ocr_queue(&env.pools).await;
    let storage = Arc::new(MemoryStorage::default());
    let context = register_owner(&env, "ocr-corsa").await;
    let property = a_property(&env, &context, "Rimini").await;
    let bill = register_bill(&env, &context, property, None, "2026/OCR/0060").await;

    attach_scan(&env, &storage, &context, bill.id(), CLEAN_SCAN).await;

    // Una persona valida **mentre il job è in coda**.
    ValidateUtilityBill::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&context, bill.id())
        .await
        .expect("validata a mano");

    let job = work_until_settled(&env, &storage, &context, bill.id()).await;
    assert_eq!(job.status(), AiJobStatus::Failed);
    assert_eq!(
        job.failure().map(|failure| failure.code),
        Some(AiJobErrorCode::SubjectNotProcessable)
    );
    let untouched = bill_of(&env, &context, bill.id()).await;
    assert_eq!(untouched.status(), BillStatus::Validated);
    assert!(untouched.ocr_extracted().is_none());
}

/// L'accodamento **non scatta** su una bolletta che non è in bozza, né su un
/// documento allegato ad altro. Sono le due condizioni che evitano il lavoro
/// inutile e il lavoro dannoso.
#[tokio::test]
async fn nothing_is_queued_for_a_photo_or_for_a_bill_that_is_no_longer_a_draft() {
    let env = require_db!();
    let _gate = common::lock_ocr_queue(&env.pools).await;
    let storage = Arc::new(MemoryStorage::default());
    let context = register_owner(&env, "ocr-innesco").await;
    let property = a_property(&env, &context, "Siena").await;

    // (a) un documento allegato a un **immobile**: nessun job.
    upload(
        &env,
        &storage,
        &context,
        AttachmentOwner::Property(property),
        "planimetria.pdf",
    )
    .await;

    // (b) una scansione su una bolletta **già validata**: nessun job.
    let bill = register_bill(&env, &context, property, None, "2026/OCR/0070").await;
    ValidateUtilityBill::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&context, bill.id())
        .await
        .expect("validata");
    attach_scan(&env, &storage, &context, bill.id(), CLEAN_SCAN).await;

    assert!(
        job_of(&env, &context, bill.id()).await.is_none(),
        "una bolletta già validata non ha niente da farsi proporre"
    );
}

/// Il job di un'altra organizzazione **non si vede**: `ai_jobs_select` chiede
/// `org_id = current_org_id() AND is_org_staff(org_id)`.
#[tokio::test]
async fn a_job_of_another_organization_is_out_of_reach() {
    let env = require_db!();
    // Anche qui, benché non faccia girare il ciclo: accoda un job e lo lascia in
    // coda, e senza il lock lo troverebbe il worker del test successivo.
    let _gate = common::lock_ocr_queue(&env.pools).await;
    let storage = Arc::new(MemoryStorage::default());
    let context = register_owner(&env, "ocr-isolamento").await;
    let property = a_property(&env, &context, "Lecce").await;
    let bill = register_bill(&env, &context, property, None, "2026/OCR/0080").await;
    attach_scan(&env, &storage, &context, bill.id(), CLEAN_SCAN).await;

    assert!(job_of(&env, &context, bill.id()).await.is_some());

    let stranger = register_owner(&env, "ocr-isolamento-altro").await;
    assert!(
        job_of(&env, &stranger, bill.id()).await.is_none(),
        "il job di un'altra organizzazione non deve essere visibile"
    );
}

// ---------------------------------------------------------------------------
// Sezione 5 — le segnalazioni: visibilità, correzione, nomi, thread chiuso
// ---------------------------------------------------------------------------

/// Apre una segnalazione **sull'immobile intero**, senza contratto indicato.
///
/// Senza `lease_id` il quarto ramo di `app.can_access_ticket` non può concedere
/// niente a nessuno: è la forma che rende il test una prova del **quinto** ramo e
/// non di un altro.
async fn open_shared_ticket(
    env: &common::TestEnv,
    context: &RequestContext,
    property_id: PropertyId,
    title: &str,
    shared: bool,
) -> propflow_domain::ticket::Ticket {
    OpenTicket::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            context,
            OpenTicketCommand {
                property_id,
                room_id: None,
                lease_id: None,
                title: TicketTitle::parse(title).expect("titolo valido"),
                description: TicketDescription::parse("La caldaia condominiale non parte")
                    .expect("descrizione valida"),
                category: None,
                priority: None,
                visible_to_residents: Some(shared),
            },
        )
        .await
        .expect("segnalazione aperta")
        .ticket
}

/// **Il quinto ramo di `app.can_access_ticket`, sul binario vero.**
///
/// X apre una segnalazione sull'immobile intero — nessun contratto indicato,
/// quindi il ramo `lease_id` è fuori gioco per costruzione — e la dichiara di
/// interesse comune. Y, che abita la stessa casa con un **contratto proprio** e
/// non è né chi ha segnalato né chi ha in carico, deve leggerla.
///
/// E deve leggere **anche il thread e lo storico**: è la ragione per cui il ramo
/// sta nella funzione e non in `tickets_select`. Con la policy allargata da sola,
/// questo test passerebbe sulla prima riga e fallirebbe sulle altre due — cioè
/// esattamente la segnalazione che si apre e non si legge.
#[tokio::test]
async fn a_shared_ticket_reaches_the_housemates_with_its_thread_and_history() {
    let env = require_db!();
    let stage = two_homes(&env, "ticket-condiviso").await;
    let (x, y) = (&stage.first[0], &stage.first[1]);
    let a = &stage.second[0];

    let shared = open_shared_ticket(&env, x, stage.home_one, "La caldaia è ferma", true).await;

    PostTicketMessage::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            x,
            PostTicketMessageCommand {
                ticket_id: shared.id(),
                body: MessageBody::parse("Da stamattina niente acqua calda").expect("corpo"),
                is_internal: false,
            },
        )
        .await
        .expect("messaggio scritto");

    let get = GetTicket::new(Arc::clone(&env.factory));
    let detail = get
        .execute(y, shared.id())
        .await
        .expect("il coinquilino legge la segnalazione condivisa");
    assert_eq!(detail.ticket.id(), shared.id());
    assert!(detail.ticket.visible_to_residents());
    assert!(
        !detail.history.is_empty(),
        "lo storico passa dalla stessa funzione: se fosse vuoto il ramo sarebbe \
         finito in `tickets_select` invece che in `can_access_ticket`"
    );

    let thread = ListTicketMessages::new(Arc::clone(&env.factory))
        .execute(
            y,
            ListTicketMessagesCommand {
                ticket_id: shared.id(),
                after: None,
                limit: None,
            },
        )
        .await
        .expect("il coinquilino legge il thread");
    assert_eq!(thread.items.len(), 1);

    // L'altra casa non c'entra: A è residente, ma di un immobile diverso.
    let denied = get
        .execute(a, shared.id())
        .await
        .expect_err("chi abita altrove non deve vederla");
    assert_eq!(denied.code(), "ticket.not_found");

    // E il default resta quello di sempre.
    let private =
        open_shared_ticket(&env, x, stage.home_one, "Perde il mio lavandino", false).await;
    let denied = get
        .execute(y, private.id())
        .await
        .expect_err("senza la casella, la segnalazione resta privata");
    assert_eq!(
        denied.code(),
        "ticket.not_found",
        "è il comportamento di prima della migration 0035, e resta il default"
    );
}

/// **Condividere non concede una riga di scrittura.**
///
/// `tickets_update` ha un predicato in linea — staff, `reported_by`,
/// `assigned_to` — e **non** passa da `can_access_ticket`: è la ragione per cui la
/// `0035` è più piccola di quanto sembri. Il coinquilino che ora *vede* la
/// segnalazione non deve poterla riaprire, annullare o correggere.
#[tokio::test]
async fn seeing_a_shared_ticket_does_not_make_it_writable() {
    let env = require_db!();
    let stage = two_homes(&env, "ticket-sola-lettura").await;
    let (x, y) = (&stage.first[0], &stage.first[1]);

    let shared = open_shared_ticket(&env, x, stage.home_one, "Il portone non chiude", true).await;

    let refused = CancelTicket::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(y, CancelTicketCommand { id: shared.id() })
        .await
        .expect_err("il coinquilino non annulla");
    assert_eq!(refused.code(), "ticket.not_yours");

    let refused = CorrectTicketReport::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            y,
            CorrectTicketReportCommand {
                id: shared.id(),
                correction: TicketCorrection {
                    title: Some(TicketTitle::parse("Riscritto da un altro").expect("titolo")),
                    ..TicketCorrection::default()
                },
            },
        )
        .await
        .expect_err("il coinquilino non corregge il testo di un altro");
    assert_eq!(
        refused.code(),
        "ticket.not_yours",
        "non gli manca un ruolo: non è stato lui a segnalare"
    );
}

/// **La correzione ha la forma dell'annullamento, e non è una copia.**
///
/// Chi ha segnalato corregge finché è `open`; da `assigned` in poi il rifiuto è
/// `org.staff_role_required`, perché lì il ruolo serve davvero. Lo staff corregge
/// sempre. Su una annullata non corregge nessuno.
///
/// Fino alla migration `0036` la scrittura sarebbe fallita con `permission
/// denied` prima ancora di arrivare qui: questo test è anche la prova che quel
/// privilegio è stato concesso.
#[tokio::test]
async fn the_reporter_corrects_the_report_only_while_nobody_has_touched_it() {
    let env = require_db!();
    let stage = two_homes(&env, "ticket-correzione").await;
    let x = &stage.first[0];

    let correct = CorrectTicketReport::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _);
    let ticket = open_shared_ticket(&env, x, stage.home_one, "Il bolier perde", false).await;

    let fixed = correct
        .execute(
            x,
            CorrectTicketReportCommand {
                id: ticket.id(),
                correction: TicketCorrection {
                    title: Some(TicketTitle::parse("Il boiler perde").expect("titolo")),
                    visible_to_residents: Some(true),
                    ..TicketCorrection::default()
                },
            },
        )
        .await
        .expect("chi ha segnalato corregge finché è aperta");
    assert_eq!(fixed.ticket.title().as_str(), "Il boiler perde");
    assert!(
        fixed.ticket.visible_to_residents(),
        "la casella si cambia anche dopo l'apertura: è uno dei tre campi della rotta"
    );

    // La descrizione non è stata nominata: resta com'era.
    assert_eq!(
        fixed.ticket.description().as_str(),
        "La caldaia condominiale non parte",
        "assente = invariato, che è la metà di PATCH che questa rotta usa"
    );

    AssignTicket::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &stage.owner,
            AssignTicketCommand {
                id: ticket.id(),
                assignee: user_of(&stage.owner),
            },
        )
        .await
        .expect("presa in carico");

    let refused = correct
        .execute(
            x,
            CorrectTicketReportCommand {
                id: ticket.id(),
                correction: TicketCorrection {
                    title: Some(TicketTitle::parse("Troppo tardi").expect("titolo")),
                    ..TicketCorrection::default()
                },
            },
        )
        .await
        .expect_err("da qui in poi corregge chi amministra");
    assert_eq!(
        refused.code(),
        "org.staff_role_required",
        "la segnalazione è sua, ma qualcuno ci ha messo mano"
    );

    correct
        .execute(
            &stage.owner,
            CorrectTicketReportCommand {
                id: ticket.id(),
                correction: TicketCorrection {
                    description: Some(
                        TicketDescription::parse("Riformulata da chi amministra")
                            .expect("descrizione"),
                    ),
                    ..TicketCorrection::default()
                },
            },
        )
        .await
        .expect("lo staff corregge sempre");

    CancelTicket::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(&stage.owner, CancelTicketCommand { id: ticket.id() })
        .await
        .expect("annullata");
    let refused = correct
        .execute(
            &stage.owner,
            CorrectTicketReportCommand {
                id: ticket.id(),
                correction: TicketCorrection {
                    title: Some(TicketTitle::parse("Su una annullata no").expect("titolo")),
                    ..TicketCorrection::default()
                },
            },
        )
        .await
        .expect_err("una segnalazione annullata non si tocca più");
    assert_eq!(refused.code(), "ticket.cancelled");
}

/// **Un thread terminato si legge e non si scrive, e riaprire dice perché.**
///
/// Le due metà sono una decisione sola: se dopo la chiusura non si scrive più,
/// riaprire è l'unico modo di dire «non è risolto», e allora il motivo è l'unica
/// cosa che spiega perché la segnalazione è tornata viva. Il motivo finisce nel
/// thread, che non si riscrive mai — e questo test lo va a rileggere lì.
#[tokio::test]
async fn a_closed_thread_is_read_but_not_written_and_reopening_says_why() {
    let env = require_db!();
    let stage = two_homes(&env, "ticket-thread-chiuso").await;
    let x = &stage.first[0];

    let ticket =
        open_shared_ticket(&env, x, stage.home_one, "Il termosifone è freddo", false).await;
    let post = PostTicketMessage::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _);
    let list = ListTicketMessages::new(Arc::clone(&env.factory));

    post.execute(
        x,
        PostTicketMessageCommand {
            ticket_id: ticket.id(),
            body: MessageBody::parse("Non scalda da lunedì").expect("corpo"),
            is_internal: false,
        },
    )
    .await
    .expect("finché è aperta si scrive");

    CloseTicket::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &stage.owner,
            CloseTicketCommand {
                id: ticket.id(),
                resolution_notes: Some("spurgato".to_owned()),
            },
        )
        .await
        .expect("chiusa");

    let refused = post
        .execute(
            x,
            PostTicketMessageCommand {
                ticket_id: ticket.id(),
                body: MessageBody::parse("Non è risolto").expect("corpo"),
                is_internal: false,
            },
        )
        .await
        .expect_err("su una chiusa non si scrive");
    assert_eq!(
        refused.code(),
        "ticket.thread_closed",
        "codice proprio: non è una transizione rifiutata, e il rimedio è riaprire"
    );

    // **Leggere resta permesso.** È metà della regola, ed è quella che si
    // romperebbe mettendo la guardia nel posto sbagliato.
    let thread = list
        .execute(
            x,
            ListTicketMessagesCommand {
                ticket_id: ticket.id(),
                after: None,
                limit: Some(50),
            },
        )
        .await
        .expect("un thread chiuso si rilegge per intero");
    assert_eq!(thread.items.len(), 1);

    ReopenTicket::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            x,
            ReopenTicketCommand {
                id: ticket.id(),
                reason: MessageBody::parse("Non è risolto: è tornato freddo").expect("motivo"),
            },
        )
        .await
        .expect("chi ha segnalato riapre");

    let thread = list
        .execute(
            x,
            ListTicketMessagesCommand {
                ticket_id: ticket.id(),
                after: None,
                limit: Some(50),
            },
        )
        .await
        .expect("thread riletto");
    assert_eq!(
        thread.items.len(),
        2,
        "il motivo della riapertura è il primo messaggio del giro nuovo"
    );
    let last = thread.items.last().expect("il messaggio c'è");
    assert_eq!(last.body().as_str(), "Non è risolto: è tornato freddo");
    assert!(
        !last.is_internal(),
        "il punto del motivo è che lo legga l'altra parte"
    );

    // E da riaperta si torna a scrivere.
    post.execute(
        x,
        PostTicketMessageCommand {
            ticket_id: ticket.id(),
            body: MessageBody::parse("Aggiungo una foto").expect("corpo"),
            is_internal: false,
        },
    )
    .await
    .expect("riaperta, il thread riprende");
}

/// **I nomi arrivano dalla policy `users_select`, e questa query è il suo primo
/// lettore.**
///
/// La `0030` aveva scritto che la finestra per restringere quella policy la
/// avrebbe chiusa il primo client che leggesse un nome da lì. È questo. I due rami
/// che reggono la sezione si provano insieme: `is_my_staff` — l'inquilino legge il
/// nome di chi ha in carico la sua segnalazione — e `shares_home_with` — il
/// coinquilino legge il nome di chi ha segnalato.
#[tokio::test]
async fn the_names_come_from_the_policy_that_0030_left_in_place() {
    let env = require_db!();
    let stage = two_homes(&env, "ticket-nomi").await;
    let (x, y) = (&stage.first[0], &stage.first[1]);

    let ticket = open_shared_ticket(&env, x, stage.home_one, "La caldaia è ferma", true).await;
    AssignTicket::new(Arc::clone(&env.factory), Arc::clone(&env.clock) as _)
        .execute(
            &stage.owner,
            AssignTicketCommand {
                id: ticket.id(),
                assignee: user_of(&stage.owner),
            },
        )
        .await
        .expect("presa in carico");

    let get = GetTicket::new(Arc::clone(&env.factory));

    // `is_my_staff`: l'inquilino vede il nome del proprietario.
    let seen_by_x = get
        .execute(x, ticket.id())
        .await
        .expect("visibile a chi ha segnalato");
    assert!(
        seen_by_x
            .people
            .name_of(seen_by_x.ticket.assigned_to())
            .is_some(),
        "«In carico a» deve dire chi, non solo quando"
    );

    // `shares_home_with`: il coinquilino vede il nome di chi ha segnalato.
    let seen_by_y = get.execute(y, ticket.id()).await.expect("condivisa");
    assert!(
        seen_by_y
            .people
            .name_of(seen_by_y.ticket.reported_by())
            .is_some(),
        "in un co-living «Inquilino» non distingue due coinquilini: serve il nome"
    );

    // E lo staff vede i nomi di entrambi, dal ramo `is_staff_of_user`.
    let seen_by_owner = get
        .execute(&stage.owner, ticket.id())
        .await
        .expect("visibile allo staff");
    assert!(
        seen_by_owner
            .people
            .name_of(seen_by_owner.ticket.reported_by())
            .is_some()
    );
}

// ---------------------------------------------------------------------------
// Inviti da proprietario
// ---------------------------------------------------------------------------

/// `app.owner_invitations` non ha una policy che ammetta il ruolo applicativo:
/// `owner_invitations_none` è `USING (false)` su ogni comando (migration `0040`,
/// ADR-053). Il ruolo applicativo non ha nemmeno il privilegio `INSERT` — la riga
/// la scrive solo lo script in `ops/`, con il ruolo del migratore — quindi il
/// tentativo è respinto **prima** che la RLS entri in gioco, con
/// `permission denied` e non con zero righe.
#[tokio::test]
async fn the_application_role_cannot_insert_an_owner_invitation() {
    let env = require_db!();
    let hash = OpaqueTokenGenerator.generate().expect("hash di prova").hash;

    let attempt = sqlx::query(
        "INSERT INTO app.owner_invitations (id, email, token_hash, expires_at)
         VALUES ($1, $2::app.email_address, $3, now() + interval '7 days')",
    )
    .bind(uuid::Uuid::now_v7())
    .bind(common::unique_email("proprietario-insert"))
    .bind(hash.as_bytes().as_slice())
    .execute(env.pools.application())
    .await;

    let error = attempt.expect_err("l'inserimento doveva essere negato");
    assert!(
        error.to_string().contains("permission denied"),
        "atteso un rifiuto per privilegi, ottenuto: {error}"
    );
}

/// Il ruolo applicativo ha il privilegio `SELECT` — la migration lo concede per
/// esteso, dopo un `REVOKE ALL` che disfa il default della `0015` — e
/// `owner_invitations_none` nega comunque ogni riga:
/// una riga inserita dal pool privilegiato resta invisibile. La differenza con il
/// test precedente è voluta: lì il privilegio manca e l'errore è
/// `permission denied`; qui il privilegio c'è e la riga sparisce senza errore,
/// perché è la RLS e non il sistema dei privilegi a fermare la lettura.
#[tokio::test]
async fn the_application_role_cannot_see_an_owner_invitation_inserted_by_the_privileged_role() {
    let env = require_db!();
    let hash = OpaqueTokenGenerator.generate().expect("hash di prova").hash;
    let id = uuid::Uuid::now_v7();

    // Il pool privilegiato è quello che lo script userebbe in produzione (qui
    // con il ruolo `propflow_worker`, l'unica connessione che i test hanno con il
    // privilegio di scrivere questa tabella: vedi la nota nella migration `0040`
    // sul perché `propflow_worker` riceve anche `INSERT`).
    sqlx::query(
        "INSERT INTO app.owner_invitations (id, email, token_hash, expires_at)
         VALUES ($1, $2::app.email_address, $3, now() + interval '7 days')",
    )
    .bind(id)
    .bind(common::unique_email("proprietario-select"))
    .bind(hash.as_bytes().as_slice())
    .execute(env.pools.privileged())
    .await
    .expect("inserimento privilegiato riuscito");

    let seen_by_app: i64 =
        sqlx::query_scalar("SELECT count(*) FROM app.owner_invitations WHERE id = $1")
            .bind(id)
            .fetch_one(env.pools.application())
            .await
            .expect("la SELECT in sé non deve fallire: il privilegio SELECT c'è");
    assert_eq!(
        seen_by_app, 0,
        "il ruolo applicativo ha visto una riga che `USING (false)` deve negare"
    );

    // Il pool privilegiato, che ha BYPASSRLS, la vede: prova che la riga esiste
    // davvero e che il test precedente non passava semplicemente a vuoto.
    let seen_by_privileged: i64 =
        sqlx::query_scalar("SELECT count(*) FROM app.owner_invitations WHERE id = $1")
            .bind(id)
            .fetch_one(env.pools.privileged())
            .await
            .expect("query privilegiata");
    assert_eq!(seen_by_privileged, 1, "la riga inserita non si trova più");
}

/// Le colonne che identificano l'invito — cosa promette e a chi — non sono
/// aggiornabili dal ruolo applicativo, come per `org_invitations` (ADR-021) e per
/// le altre tabelle di credenziali. Solo `accepted_at` e `accepted_by`, ciò che
/// l'accettazione scrive, restano concesse: è la coppia del controllo 12.
///
/// Il privilegio si verifica **prima** delle policy e prima che le righe vengano
/// esaminate, quindi il rifiuto arriva come `permission denied` anche su un
/// identificatore inesistente — stessa forma di
/// `the_application_role_cannot_rewrite_what_an_account_is`.
#[tokio::test]
async fn the_application_role_cannot_rewrite_the_identity_of_an_owner_invitation() {
    let env = require_db!();
    let id = uuid::Uuid::now_v7();

    for statement in [
        "UPDATE app.owner_invitations SET email = 'altro@example.test' WHERE id = $1",
        "UPDATE app.owner_invitations SET token_hash = '\\xdeadbeef' WHERE id = $1",
        "UPDATE app.owner_invitations SET expires_at = now() WHERE id = $1",
        "UPDATE app.owner_invitations SET revoked_at = now() WHERE id = $1",
        "UPDATE app.owner_invitations SET created_at = now() WHERE id = $1",
    ] {
        let error = sqlx::query(statement)
            .bind(id)
            .execute(env.pools.application())
            .await
            .expect_err("l'aggiornamento doveva essere negato");
        assert!(
            error.to_string().contains("permission denied"),
            "atteso un rifiuto per privilegi su `{statement}`, ottenuto: {error}"
        );
    }

    // Le due concesse non falliscono per privilegio — `USING (false)` le negherà
    // comunque a livello di riga, ma è la RLS a fermarle, non la `GRANT`, ed è
    // esattamente la distinzione che il test precedente stabilisce.
    for statement in [
        "UPDATE app.owner_invitations SET accepted_at = now() WHERE id = $1",
        "UPDATE app.owner_invitations SET accepted_by = NULL WHERE id = $1",
    ] {
        let allowed = sqlx::query(statement)
            .bind(id)
            .execute(env.pools.application())
            .await;
        assert!(
            allowed.is_ok(),
            "l'aggiornamento di `{statement}` non doveva essere negato per privilegio: {:?}",
            allowed.err()
        );
    }
}
