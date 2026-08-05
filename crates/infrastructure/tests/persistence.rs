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

use propflow_application::use_cases::auth::{RegisterOwner, RegisterOwnerCommand};
use propflow_application::use_cases::property::{
    ArchiveProperty, ArchivePropertyCommand, CreateProperty, CreatePropertyCommand, ListProperties,
    ListPropertiesQuery,
};
use propflow_application::use_cases::room::{CreateRoom, CreateRoomCommand, ListRooms};
use propflow_domain::RequestContext;
use propflow_domain::identity::OrgRole;
use propflow_domain::ports::RepositoryError;
use propflow_domain::property::{PropertyType, Room, RoomChanges};
use propflow_domain::shared::{Address, CountryCode, PropertyId, RoomId};

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
async fn register_owner(env: &common::TestEnv, label: &str) -> RequestContext {
    let use_case = RegisterOwner::new(
        Arc::clone(&env.factory),
        Arc::clone(&env.hasher),
        Arc::clone(&env.clock) as Arc<dyn propflow_domain::ports::Clock>,
    );
    let result = use_case
        .execute(
            &RequestContext::anonymous(),
            RegisterOwnerCommand {
                email: common::unique_email(label),
                full_name: format!("Proprietario {label}"),
                password: "cavallo batteria graffetta".to_owned(),
                organization_name: Some(format!("Org {label}")),
            },
        )
        .await
        .expect("registrazione riuscita");

    RequestContext::authenticated(result.user_id).with_org(result.org_id, OrgRole::Owner)
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
    assert!(own.items.iter().any(|item| item.id() == property.id()));

    // L'organizzazione Beta non vede nulla di Alfa.
    let other = list
        .execute(&beta, ListPropertiesQuery::default())
        .await
        .expect("elenco riuscito");
    assert!(
        !other.items.iter().any(|item| item.id() == property.id()),
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
        .find(|item| item.id() == room.id())
        .expect("stanza presente nell'elenco");
    assert_eq!(stored.name(), "Camera A");
    assert_eq!(
        stored
            .monthly_rent()
            .map(|money| money.amount().to_string()),
        Some("450.00".to_owned())
    );
    assert_eq!(
        stored
            .utility_share_percent()
            .map(|value| value.to_string()),
        Some("33.33".to_owned())
    );
    assert!(stored.has_private_bathroom());
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
