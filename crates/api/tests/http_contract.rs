// Crate di test: qui `expect` è lo strumento corretto. Un panic segnala una
// precondizione di test non soddisfatta e fa fallire il test, che è il
// comportamento desiderato; gestire l'errore lo renderebbe verde a vuoto.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

//! Contratto HTTP: codici di stato, formato degli errori, autorizzazione.

mod common;

use axum::http::StatusCode;
use common::TestApp;

macro_rules! require_app {
    () => {
        match TestApp::spawn().await {
            Some(app) => app,
            None => return,
        }
    };
}

#[tokio::test]
async fn protected_routes_require_a_token() {
    let app = require_app!();
    let (status, body) = app.send("GET", "/api/v1/properties", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["status"], 401);
    assert!(
        body["type"]
            .as_str()
            .is_some_and(|t| t.contains("unauthorized")),
        "atteso un Problem Details: {body}"
    );
}

#[tokio::test]
async fn an_invalid_token_is_refused() {
    let app = require_app!();
    let (status, _) = app
        .send("GET", "/api/v1/properties", Some("non-un-jwt"), None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn weak_passwords_are_rejected_with_field_errors() {
    let app = require_app!();
    let (status, body) = app
        .post(
            "/api/v1/auth/register",
            serde_json::json!({
                "email": common::unique_email("debole"),
                "full_name": "Tizio",
                "password": "corta",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "password", "corpo: {body}");
}

#[tokio::test]
async fn malformed_email_is_rejected() {
    let app = require_app!();
    let (status, body) = app
        .post(
            "/api/v1/auth/register",
            serde_json::json!({
                "email": "non-una-email",
                "full_name": "Tizio",
                "password": "cavallo batteria graffetta",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "email", "corpo: {body}");
}

#[tokio::test]
async fn registering_the_same_email_twice_is_a_conflict() {
    let app = require_app!();
    let email = common::unique_email("duplicata");
    let payload = serde_json::json!({
        "email": email,
        "full_name": "Tizio",
        "password": "cavallo batteria graffetta",
    });

    let (first, _) = app.post("/api/v1/auth/register", payload.clone()).await;
    assert_eq!(first, StatusCode::CREATED);

    let (second, body) = app.post("/api/v1/auth/register", payload).await;
    assert_eq!(second, StatusCode::CONFLICT, "corpo: {body}");
}

#[tokio::test]
async fn wrong_credentials_are_indistinguishable_from_an_unknown_account() {
    let app = require_app!();
    let email = common::unique_email("credenziali");
    app.post(
        "/api/v1/auth/register",
        serde_json::json!({
            "email": email,
            "full_name": "Tizio",
            "password": "cavallo batteria graffetta",
        }),
    )
    .await;

    let (existing_user, body_existing) = app
        .post(
            "/api/v1/auth/login",
            serde_json::json!({ "email": email, "password": "password sbagliata ma lunga" }),
        )
        .await;
    let (unknown_user, body_unknown) = app
        .post(
            "/api/v1/auth/login",
            serde_json::json!({ "email": common::unique_email("mai-visto"), "password": "password sbagliata ma lunga" }),
        )
        .await;

    // Stesso stato e stesso corpo: la risposta non permette di sapere se l'account
    // esiste, che è il punto di non avere un oracolo di enumerazione.
    assert_eq!(existing_user, StatusCode::UNAUTHORIZED);
    assert_eq!(unknown_user, StatusCode::UNAUTHORIZED);
    assert_eq!(body_existing["title"], body_unknown["title"]);
}

#[tokio::test]
async fn full_property_lifecycle() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "ciclo").await;

    // Creazione
    let (status, created) = app
        .post_auth(
            "/api/v1/properties",
            &token,
            serde_json::json!({
                "label": "Bilocale centro",
                "property_type": "apartment",
                "address": { "street": "Via Roma 1", "city": "Milano", "province": "MI" }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {created}");
    assert_eq!(created["status"], "draft");
    let id = created["id"].as_str().expect("id").to_owned();

    // Lettura
    let (status, fetched) = app
        .send(
            "GET",
            &format!("/api/v1/properties/{id}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched["label"], "Bilocale centro");

    // Elenco
    let (status, page) = app
        .send("GET", "/api/v1/properties", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        page["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "elenco vuoto: {page}"
    );

    // Modifica parziale, con azzeramento esplicito di un campo
    let (status, patched) = app
        .send(
            "PATCH",
            &format!("/api/v1/properties/{id}"),
            Some(&token),
            Some(serde_json::json!({ "status": "available", "notes": "da imbiancare" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {patched}");
    assert_eq!(patched["status"], "available");
    assert_eq!(patched["notes"], "da imbiancare");

    let (status, cleared) = app
        .send(
            "PATCH",
            &format!("/api/v1/properties/{id}"),
            Some(&token),
            Some(serde_json::json!({ "notes": null })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        cleared.get("notes").is_none(),
        "notes non azzerato: {cleared}"
    );

    // Archiviazione
    let (status, body) = app
        .send(
            "DELETE",
            &format!("/api/v1/properties/{id}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "corpo: {body}");

    // Dopo l'archiviazione non è più leggibile
    let (status, _) = app
        .send(
            "GET",
            &format!("/api/v1/properties/{id}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn refresh_rotates_the_token_and_reuse_revokes_the_chain() {
    let app = require_app!();
    let email = common::unique_email("rotazione");
    let password = "cavallo batteria graffetta";
    app.post(
        "/api/v1/auth/register",
        serde_json::json!({ "email": email, "full_name": "Tizio", "password": password }),
    )
    .await;
    let (_, session) = app
        .post(
            "/api/v1/auth/login",
            serde_json::json!({ "email": email, "password": password }),
        )
        .await;
    let first_refresh = session["refresh_token"]
        .as_str()
        .expect("refresh")
        .to_owned();

    let (status, rotated) = app
        .post(
            "/api/v1/auth/refresh",
            serde_json::json!({ "refresh_token": first_refresh }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {rotated}");
    let second_refresh = rotated["refresh_token"]
        .as_str()
        .expect("refresh")
        .to_owned();
    assert_ne!(
        second_refresh, first_refresh,
        "il token non è stato ruotato"
    );

    // Riuso del token già consumato: rifiutato, e la catena viene revocata.
    let (status, body) = app
        .post(
            "/api/v1/auth/refresh",
            serde_json::json!({ "refresh_token": first_refresh }),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "corpo: {body}");

    // Anche il successore cade, perché non si sa quale delle due parti sia
    // legittima.
    let (status, body) = app
        .post(
            "/api/v1/auth/refresh",
            serde_json::json!({ "refresh_token": second_refresh }),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "il rilevamento del riuso deve revocare tutte le sessioni, corpo: {body}"
    );
}

#[tokio::test]
async fn unknown_routes_return_problem_details_too() {
    let app = require_app!();
    let (status, body) = app.send("GET", "/api/v1/non-esiste", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404, "corpo: {body}");
}

#[tokio::test]
async fn openapi_document_is_served() {
    let app = require_app!();
    let (status, body) = app.send("GET", "/openapi.json", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["info"]["title"], "PropFlow API");
    assert!(
        body["paths"]["/api/v1/properties"]["post"].is_object(),
        "rotta assente dal documento"
    );
}

/// Crea un immobile e restituisce il suo id.
async fn create_property(app: &TestApp, token: &str, label: &str) -> String {
    let (status, created) = app
        .post_auth(
            "/api/v1/properties",
            token,
            serde_json::json!({
                "label": label,
                "property_type": "apartment",
                "address": { "street": "Via Roma 1", "city": "Milano", "province": "MI" }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {created}");
    created["id"].as_str().expect("id").to_owned()
}

#[tokio::test]
async fn full_room_lifecycle() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "stanze").await;
    let property = create_property(&app, &token, "Casa in condivisione").await;

    let (status, created) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/rooms"),
            &token,
            serde_json::json!({
                "name": "Camera A",
                "monthly_rent": "450.00",
                "has_private_bathroom": true,
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {created}");
    assert_eq!(created["status"], "available");
    // Gli importi sul filo sono stringhe, non float.
    assert_eq!(created["monthly_rent"], "450.00");
    assert_eq!(created["has_private_bathroom"], true);
    let room = created["id"].as_str().expect("id").to_owned();

    let (status, page) = app
        .send(
            "GET",
            &format!("/api/v1/properties/{property}/rooms"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        page["items"].as_array().map(Vec::len),
        Some(1),
        "corpo: {page}"
    );

    // Modifica parziale, con azzeramento esplicito del canone.
    let (status, patched) = app
        .send(
            "PATCH",
            &format!("/api/v1/rooms/{room}"),
            Some(&token),
            Some(serde_json::json!({ "status": "occupied", "monthly_rent": null })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {patched}");
    assert_eq!(patched["status"], "occupied");
    assert!(
        patched.get("monthly_rent").is_none(),
        "canone non azzerato: {patched}"
    );

    let (status, body) = app
        .send(
            "DELETE",
            &format!("/api/v1/rooms/{room}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "corpo: {body}");

    // Dopo l'archiviazione la stanza non compare più nell'elenco.
    let (_, page) = app
        .send(
            "GET",
            &format!("/api/v1/properties/{property}/rooms"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(
        page["items"].as_array().map(Vec::len),
        Some(0),
        "corpo: {page}"
    );
}

#[tokio::test]
async fn a_room_on_someone_elses_property_is_not_found() {
    let app = require_app!();
    let (alfa, _) = common::register_and_login(&app, "stanza-alfa").await;
    let (beta, _) = common::register_and_login(&app, "stanza-beta").await;
    let property = create_property(&app, &alfa, "Immobile di Alfa").await;

    // 404 e non 403: un 403 confermerebbe che l'immobile esiste.
    let (status, body) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/rooms"),
            &beta,
            serde_json::json!({ "name": "Camera abusiva" }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {body}");
}

#[tokio::test]
async fn two_rooms_cannot_share_a_name_in_the_same_property() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "nomi").await;
    let property = create_property(&app, &token, "Casa con nomi ripetuti").await;
    let body = serde_json::json!({ "name": "Camera A" });

    let (first, _) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/rooms"),
            &token,
            body.clone(),
        )
        .await;
    assert_eq!(first, StatusCode::CREATED);

    let (second, problem) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/rooms"),
            &token,
            body,
        )
        .await;
    assert_eq!(second, StatusCode::CONFLICT, "corpo: {problem}");
    assert!(
        problem["detail"]
            .as_str()
            .is_some_and(|detail| detail.contains("stanza")),
        "il conflitto deve dire cosa è duplicato: {problem}"
    );
}

fn day(offset: i64) -> String {
    (chrono::Utc::now().date_naive() + chrono::Duration::days(offset)).to_string()
}

/// Redige un contratto sull'immobile indicato e restituisce il suo id.
async fn draft_lease(
    app: &TestApp,
    token: &str,
    property: &str,
    room: Option<&str>,
    start_offset: i64,
    end_offset: Option<i64>,
) -> String {
    let mut body = serde_json::json!({
        "property_id": property,
        "start_date": day(start_offset),
        "rent_amount": "800.00",
    });
    if let Some(room) = room {
        body["room_id"] = serde_json::json!(room);
    }
    if let Some(offset) = end_offset {
        body["end_date"] = serde_json::json!(day(offset));
    }

    let (status, created) = app.post_auth("/api/v1/leases", token, body).await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {created}");
    created["id"].as_str().expect("id").to_owned()
}

/// Aggiunge un inquilino al 100% al contratto indicato.
async fn add_tenant(app: &TestApp, token: &str, lease: &str, name: &str, email: Option<&str>) {
    let mut body = serde_json::json!({ "full_name": name, "is_primary": true });
    if let Some(email) = email {
        body["email"] = serde_json::json!(email);
    }
    let (status, created) = app
        .post_auth(&format!("/api/v1/leases/{lease}/parties"), token, body)
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {created}");
}

#[tokio::test]
async fn full_lease_lifecycle() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "contratto").await;
    let property = create_property(&app, &token, "Immobile da affittare").await;

    let lease = draft_lease(&app, &token, &property, None, 0, Some(365)).await;

    // Una bozza non è in essere, e il tipo si deduce dall'assenza della stanza.
    let (status, drafted) = app
        .send(
            "GET",
            &format!("/api/v1/leases/{lease}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {drafted}");
    assert_eq!(drafted["status"], "draft");
    assert_eq!(drafted["effective_status"], "draft");
    assert_eq!(drafted["lease_type"], "whole_property");
    // Gli importi sul filo sono stringhe; il deposito assente diventa zero.
    assert_eq!(drafted["rent_amount"], "800.00");
    assert_eq!(drafted["deposit_amount"], "0.00");
    assert_eq!(drafted["parties"].as_array().map(Vec::len), Some(0));

    // Senza inquilini l'attivazione è rifiutata: un contratto attivo senza parti
    // non sarebbe addebitabile.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");

    add_tenant(&app, &token, &lease, "Mario Rossi", None).await;

    let (status, activated) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {activated}");
    assert_eq!(activated["status"], "active");
    assert_eq!(activated["effective_status"], "active");
    assert!(activated["signed_at"].is_string(), "corpo: {activated}");

    // L'elenco filtrato per contratti in essere lo contiene.
    let (status, page) = app
        .send(
            "GET",
            &format!("/api/v1/leases?property_id={property}&in_force=true"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        page["items"].as_array().map(Vec::len),
        Some(1),
        "corpo: {page}"
    );

    let (status, terminated) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/terminate"),
            &token,
            serde_json::json!({ "reason": "trasferimento" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {terminated}");
    assert_eq!(terminated["status"], "terminated");
    assert!(terminated["terminated_at"].is_string());

    // Disdirlo due volte non è ammesso.
    let (status, _) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/terminate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn an_overlapping_activation_says_what_is_occupied() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "conflitto").await;
    let property = create_property(&app, &token, "Immobile conteso").await;

    let first = draft_lease(&app, &token, &property, None, 0, Some(365)).await;
    add_tenant(&app, &token, &first, "Primo Inquilino", None).await;
    let (status, _) = app
        .post_auth(
            &format!("/api/v1/leases/{first}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let second = draft_lease(&app, &token, &property, None, 180, Some(540)).await;
    add_tenant(&app, &token, &second, "Secondo Inquilino", None).await;
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/leases/{second}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    // Il nome del vincolo resta nei log: la risposta dice cosa è occupato.
    let detail = problem["detail"].as_str().unwrap_or_default();
    assert!(
        detail.contains("immobile") && detail.contains("periodo"),
        "il 409 deve essere parlante: {problem}"
    );
}

#[tokio::test]
async fn an_expired_lease_reports_its_derived_status() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "scaduto").await;
    let property = create_property(&app, &token, "Immobile con contratto finito").await;

    let lease = draft_lease(&app, &token, &property, None, -730, Some(-30)).await;
    add_tenant(&app, &token, &lease, "Ex Inquilino", None).await;
    app.post_auth(
        &format!("/api/v1/leases/{lease}/activate"),
        &token,
        serde_json::json!({}),
    )
    .await;

    let (status, body) = app
        .send(
            "GET",
            &format!("/api/v1/leases/{lease}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "active", "lo stato memorizzato non cambia");
    assert_eq!(body["effective_status"], "expired", "corpo: {body}");

    // E non compare fra quelli in essere.
    let (_, page) = app
        .send(
            "GET",
            &format!("/api/v1/leases?property_id={property}&in_force=true"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(
        page["items"].as_array().map(Vec::len),
        Some(0),
        "corpo: {page}"
    );
}

#[tokio::test]
async fn filtering_by_a_derived_status_is_refused_with_a_hint() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "filtro").await;

    let (status, problem) = app
        .send("GET", "/api/v1/leases?status=expired", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");
    assert_eq!(problem["errors"][0]["field"], "status");
    assert!(
        problem["errors"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("in_force")),
        "l'errore deve indicare il filtro corretto: {problem}"
    );
}

#[tokio::test]
async fn a_lease_of_another_organization_is_not_found() {
    let app = require_app!();
    let (alfa, _) = common::register_and_login(&app, "contratto-alfa").await;
    let (beta, _) = common::register_and_login(&app, "contratto-beta").await;
    let property = create_property(&app, &alfa, "Immobile di Alfa").await;
    let lease = draft_lease(&app, &alfa, &property, None, 0, Some(365)).await;

    let (status, body) = app
        .send("GET", &format!("/api/v1/leases/{lease}"), Some(&beta), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {body}");

    // E nemmeno redigerne uno sull'immobile altrui.
    let (status, body) = app
        .post_auth(
            "/api/v1/leases",
            &beta,
            serde_json::json!({
                "property_id": property,
                "start_date": day(0),
                "rent_amount": "800.00",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {body}");
}

#[tokio::test]
async fn a_room_of_another_property_cannot_be_let() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "stanza-altrui").await;
    let first = create_property(&app, &token, "Prima casa").await;
    let second = create_property(&app, &token, "Seconda casa").await;

    let (_, room) = app
        .post_auth(
            &format!("/api/v1/properties/{second}/rooms"),
            &token,
            serde_json::json!({ "name": "Camera della seconda casa" }),
        )
        .await;
    let room = room["id"].as_str().expect("id");

    // Entrambi gli immobili sono visibili al proprietario, quindi la RLS non
    // fermerebbe l'accostamento: il legame stanza-immobile va verificato.
    let (status, problem) = app
        .post_auth(
            "/api/v1/leases",
            &token,
            serde_json::json!({
                "property_id": first,
                "room_id": room,
                "start_date": day(0),
                "rent_amount": "400.00",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");
    assert_eq!(problem["errors"][0]["field"], "room_id");
}

/// Emette un invito su un contratto e restituisce (`invitation_id`, `token`).
async fn invite_tenant(app: &TestApp, token: &str, lease: &str, email: &str) -> (String, String) {
    let (status, body) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/invitations"),
            token,
            serde_json::json!({ "email": email, "full_name": "Inquilino Invitato" }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {body}");
    (
        body["id"].as_str().expect("id").to_owned(),
        body["accept_token"].as_str().expect("token").to_owned(),
    )
}

/// Il test che dimostra la bidirezionalità: prima dello step 3 era impossibile,
/// perché nessuno poteva diventare inquilino dall'API.
#[tokio::test]
async fn a_tenant_becomes_real_and_sees_only_what_is_theirs() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "proprietario").await;

    // Due immobili: quello affittato e uno che l'inquilino non deve vedere.
    let rented = create_property(&app, &owner, "Immobile affittato").await;
    let other = create_property(&app, &owner, "Immobile non affittato").await;

    let (_, room) = app
        .post_auth(
            &format!("/api/v1/properties/{rented}/rooms"),
            &owner,
            serde_json::json!({ "name": "Camera dell'inquilino", "monthly_rent": "400.00" }),
        )
        .await;
    let room = room["id"].as_str().expect("id").to_owned();

    let lease = draft_lease(&app, &owner, &rented, Some(&room), 0, Some(365)).await;
    // Un secondo contratto sull'altro immobile: l'inquilino non deve vederlo.
    let foreign_lease = draft_lease(&app, &owner, &other, None, 0, Some(365)).await;

    let tenant_email = common::unique_email("inquilino");
    let (invitation, accept_token) = invite_tenant(&app, &owner, &lease, &tenant_email).await;
    assert!(!invitation.is_empty());

    // L'invito ha creato la parte contrattuale, ancora senza utente collegato.
    let (_, detail) = app
        .send(
            "GET",
            &format!("/api/v1/leases/{lease}"),
            Some(&owner),
            None,
        )
        .await;
    let party = &detail["parties"][0];
    assert_eq!(party["email"], tenant_email);
    assert!(
        party.get("user_id").is_none(),
        "la parte non deve avere un utente prima dell'accettazione: {detail}"
    );
    // `Decimal` va sul filo come stringa, con la scala che ha nel database.
    assert_eq!(party["share_percent"], "100.00");

    // L'inquilino si registra con la stessa email e accetta.
    let (tenant, tenant_org, _) = common::register_and_login_with_email(&app, &tenant_email).await;
    assert_ne!(tenant_org, owner_org);

    let (status, accepted) = app
        .post_auth(
            "/api/v1/auth/accept-invitation",
            &tenant,
            serde_json::json!({ "token": accept_token }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {accepted}");
    assert_eq!(accepted["organization_id"], owner_org);
    assert_eq!(accepted["role"], "tenant");
    assert_eq!(accepted["lease_id"], lease);

    // Da qui l'inquilino vede il proprio contratto nell'organizzazione del
    // proprietario: la visibilità arriva da `lease_parties.user_id`.
    let (status, page) = app
        .send_in_org(
            "GET",
            "/api/v1/leases",
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");
    let visible: Vec<&str> = page["items"]
        .as_array()
        .expect("elenco")
        .iter()
        .filter_map(|item| item["id"].as_str())
        .collect();
    assert_eq!(visible, vec![lease.as_str()], "corpo: {page}");
    assert!(
        !visible.contains(&foreign_lease.as_str()),
        "l'inquilino vede un contratto che non è suo"
    );

    // E vede l'immobile in cui abita, non l'altro dello stesso proprietario.
    let (status, properties) = app
        .send_in_org(
            "GET",
            "/api/v1/properties",
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {properties}");
    let visible: Vec<&str> = properties["items"]
        .as_array()
        .expect("elenco")
        .iter()
        .filter_map(|item| item["id"].as_str())
        .collect();
    assert_eq!(visible, vec![rented.as_str()], "corpo: {properties}");
    assert!(
        !visible.contains(&other.as_str()),
        "l'inquilino vede il secondo immobile del proprietario"
    );

    // La parte contrattuale è collegata al suo utente.
    let (_, detail) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/leases/{lease}"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert!(
        detail["parties"][0]["user_id"].is_string(),
        "la parte non è stata collegata: {detail}"
    );
    assert!(detail["parties"][0]["joined_at"].is_string());

    // L'inquilino è membro, non staff: non crea contratti né stanze.
    let (status, body) = app
        .send_in_org(
            "POST",
            "/api/v1/leases",
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({
                "property_id": rented,
                "start_date": day(400),
                "rent_amount": "100.00",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {body}");

    let (status, body) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/properties/{rented}/rooms"),
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({ "name": "Camera abusiva" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {body}");

    // E non invita nessun altro.
    let (status, body) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/leases/{lease}/invitations"),
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({
                "email": common::unique_email("amico"),
                "full_name": "Un Amico",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {body}");

    // Riaccettare lo stesso invito è un conflitto, non un secondo ingresso.
    let (status, body) = app
        .post_auth(
            "/api/v1/auth/accept-invitation",
            &tenant,
            serde_json::json!({ "token": accept_token }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {body}");
}

#[tokio::test]
async fn an_unknown_token_is_indistinguishable_from_someone_elses() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "invito-ignoto").await;
    let property = create_property(&app, &owner, "Immobile con invito").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    let invited = common::unique_email("invitato");
    let (_, accept_token) = invite_tenant(&app, &owner, &lease, &invited).await;

    // Un estraneo con il token in mano: l'email non corrisponde, quindi 401 —
    // lo stesso di un token inventato, perché distinguerli direbbe quali inviti
    // esistono.
    let (intruder, _) = common::register_and_login(&app, "estraneo").await;
    let (with_stolen_token, body) = app
        .post_auth(
            "/api/v1/auth/accept-invitation",
            &intruder,
            serde_json::json!({ "token": accept_token }),
        )
        .await;
    let (with_invented_token, other_body) = app
        .post_auth(
            "/api/v1/auth/accept-invitation",
            &intruder,
            serde_json::json!({ "token": "token-mai-emesso" }),
        )
        .await;

    assert_eq!(with_stolen_token, StatusCode::UNAUTHORIZED, "corpo: {body}");
    assert_eq!(with_invented_token, StatusCode::UNAUTHORIZED);
    assert_eq!(body["title"], other_body["title"]);
}

#[tokio::test]
async fn a_revoked_invitation_no_longer_works_and_leaves_no_membership() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "invito-revocato").await;
    let property = create_property(&app, &owner, "Immobile con invito revocato").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    let tenant_email = common::unique_email("revocato");
    let (invitation, accept_token) = invite_tenant(&app, &owner, &lease, &tenant_email).await;

    let (status, body) = app
        .send(
            "DELETE",
            &format!("/api/v1/invitations/{invitation}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "corpo: {body}");

    let (tenant, _, _) = common::register_and_login_with_email(&app, &tenant_email).await;
    let (status, body) = app
        .post_auth(
            "/api/v1/auth/accept-invitation",
            &tenant,
            serde_json::json!({ "token": accept_token }),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "corpo: {body}");

    // Nessuna scrittura parziale: non è diventato membro, quindi non vede nulla
    // dell'organizzazione che lo aveva invitato.
    let (status, body) = app
        .send_in_org(
            "GET",
            "/api/v1/leases",
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {body}");
}

#[tokio::test]
async fn a_second_pending_invitation_for_the_same_email_is_a_conflict() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "doppio-invito").await;
    let property = create_property(&app, &owner, "Immobile invitato due volte").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;
    let email = common::unique_email("una-volta-sola");

    invite_tenant(&app, &owner, &lease, &email).await;
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/invitations"),
            &owner,
            serde_json::json!({ "email": email, "full_name": "Inquilino Invitato" }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert!(
        problem["detail"]
            .as_str()
            .is_some_and(|detail| detail.contains("invito")),
        "il conflitto deve dire cosa è duplicato: {problem}"
    );
}
