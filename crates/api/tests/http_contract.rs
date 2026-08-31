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
    assert_eq!(body["code"], "auth.token_missing", "corpo: {body}");
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
    let (status, body) = app
        .send("GET", "/api/v1/properties", Some("non-un-jwt"), None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // Non `auth.token_expired`: un token che non abbiamo firmato noi non può
    // ottenere la diagnosi riservata a chi ne aveva uno legittimo (ADR-022).
    assert_eq!(body["code"], "auth.token_invalid", "corpo: {body}");
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
    assert_eq!(body["code"], "validation.failed", "corpo: {body}");
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
    // Il codice non deve reintrodurre l'oracolo che il titolo unico evita.
    assert_eq!(body_existing["code"], body_unknown["code"]);
    assert_eq!(body_existing["code"], "auth.invalid_credentials");
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
    // Una rotta sconosciuta ha la stessa forma di una risorsa non trovata, ma un
    // codice suo: un client non deve scambiare un errore di indirizzo per una
    // risorsa cancellata.
    assert_eq!(body["code"], "endpoint.not_found", "corpo: {body}");
}

/// Ogni risposta d'errore porta un `code`, e ogni `code` è nella forma
/// `<dominio>.<condizione>`. Senza questo, una variante nuova può nascere senza
/// codice e nessuno se ne accorge finché non lo cerca un client.
#[tokio::test]
async fn every_error_response_carries_a_dotted_code() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "codici").await;

    let cases: Vec<(StatusCode, serde_json::Value)> = vec![
        app.send("GET", "/api/v1/properties", None, None).await,
        app.send("GET", "/api/v1/non-esiste", None, None).await,
        app.send("GET", "/api/v1/leases?status=expired", Some(&token), None)
            .await,
        app.send(
            "GET",
            &format!("/api/v1/properties/{}", uuid::Uuid::now_v7()),
            Some(&token),
            None,
        )
        .await,
    ];

    for (status, body) in cases {
        let code = body["code"]
            .as_str()
            .unwrap_or_else(|| panic!("risposta {status} senza `code`: {body}"));
        let (domain, condition) = code
            .split_once('.')
            .unwrap_or_else(|| panic!("`{code}` non è <dominio>.<condizione>"));
        assert!(!domain.is_empty() && !condition.is_empty(), "`{code}`");
    }
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

    // `code` è parte del contratto, quindi deve stare nello schema pubblicato e
    // non solo nelle risposte: un client che genera il proprio modello dallo
    // OpenAPI non lo vedrebbe mai.
    let problem = &body["components"]["schemas"]["ProblemDetails"];
    assert!(
        problem["properties"]["code"].is_object(),
        "ProblemDetails senza `code`: {problem}"
    );
    let required = problem["required"]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    assert!(
        required.contains(&"code"),
        "`code` deve essere obbligatorio: {problem}"
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
    // Sul codice e non sul testo: il `detail` può essere riscritto, `code` no
    // (ADR-022). Un test che confronta prosa si rompe alla prima riformulazione.
    assert_eq!(problem["code"], "room.name_taken", "corpo: {problem}");
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
    assert_eq!(problem["code"], "lease.period_overlap", "corpo: {problem}");
    // Il nome del vincolo resta nei log: la risposta dice cosa è occupato.
    let detail = problem["detail"].as_str().unwrap_or_default();
    assert!(
        detail.contains("property") && detail.contains("period"),
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
    // Codice proprio, non `validation.failed`: un valore derivato rifiutato è una
    // condizione diversa da un valore non ammesso, e sullo stesso campo.
    assert_eq!(
        problem["code"], "lease.derived_status_filter",
        "corpo: {problem}"
    );
    assert_eq!(problem["errors"][0]["field"], "status");
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
    assert_eq!(problem["code"], "validation.failed", "corpo: {problem}");
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
    // Un token altrui e uno mai emesso danno lo stesso codice: distinguerli
    // direbbe a chi prova se un invito esiste.
    assert_eq!(body["code"], other_body["code"]);
    assert_eq!(body["code"], "auth.invalid_credentials");
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
    assert_eq!(
        problem["code"], "invitation.already_pending",
        "corpo: {problem}"
    );
}

// ---------------------------------------------------------------------------
// Documenti
// ---------------------------------------------------------------------------

/// Un PDF minimo ma riconoscibile: i byte non contano, conta che siano gli stessi
/// che il checksum deve coprire.
const PDF: &[u8] = b"%PDF-1.7\n1 0 obj\n<<>>\nendobj\ntrailer\n<<>>\n%%EOF\n";

/// Il giro completo: si carica, si rilegge nell'elenco, si scarica seguendo il
/// `307`, si archivia. È il criterio con cui il piano dichiara finita la fetta.
#[tokio::test]
async fn full_document_lifecycle() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "documenti").await;
    let property = create_property(&app, &owner, "Immobile con documenti").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    let (status, created) = app
        .upload(common::Upload {
            path: &format!("/api/v1/leases/{lease}/documents"),
            token: &owner,
            org: None,
            filename: "contratto-mario-rossi.pdf",
            content_type: "application/pdf",
            content: PDF,
            kind: Some("lease_contract"),
        })
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {created}");
    assert_eq!(created["kind"], "lease_contract", "corpo: {created}");
    assert_eq!(created["owner_type"], "lease", "corpo: {created}");
    assert_eq!(created["owner_id"], lease, "corpo: {created}");
    assert_eq!(created["mime_type"], "application/pdf", "corpo: {created}");
    assert_eq!(created["size_bytes"], PDF.len(), "corpo: {created}");
    assert_eq!(
        created["original_filename"], "contratto-mario-rossi.pdf",
        "corpo: {created}"
    );
    // Il checksum lo calcola l'API sui byte ricevuti, non il client: 64 caratteri
    // esadecimali di SHA-256.
    let checksum = created["checksum_sha256"].as_str().expect("checksum");
    assert_eq!(checksum.len(), 64, "corpo: {created}");
    assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
    // Il bucket non è configurato per la cifratura, e la colonna lo dice.
    assert_eq!(created["is_encrypted"], false, "corpo: {created}");
    let document = created["id"].as_str().expect("id").to_owned();

    // La trappola più cara del modulo, verificata anche da qui: la chiave
    // dell'oggetto si deriva dagli identificatori, e il nome del file — che è un
    // dato personale — non ci compare.
    let keys = app.storage.keys();
    assert_eq!(keys.len(), 1, "chiavi: {keys:?}");
    assert!(
        !keys[0].contains("mario") && !keys[0].contains(".pdf"),
        "il nome del file è finito nella chiave: {keys:?}"
    );

    let (status, page) = app
        .send(
            "GET",
            &format!("/api/v1/leases/{lease}/documents"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");
    let listed: Vec<&str> = page["items"]
        .as_array()
        .expect("elenco")
        .iter()
        .filter_map(|item| item["id"].as_str())
        .collect();
    assert_eq!(listed, vec![document.as_str()], "corpo: {page}");

    // Download: `307` verso l'URL pre-firmato, che sta in `Location` e non nel
    // corpo — il contenuto non transita dall'API.
    let (status, response) = app
        .send_raw(
            "GET",
            &format!("/api/v1/documents/{document}/content"),
            &owner,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
    let location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .expect("intestazione Location");
    assert!(location.contains("X-Amz-Signature="), "{location}");
    assert!(
        location.contains(&format!("X-Amz-Expires={}", common::PRESIGN_TTL_SECONDS)),
        "{location}"
    );
    // Un permesso a tempo non va messo in cache da nessuno.
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("no-store, private")
    );

    let (status, body) = app
        .send(
            "DELETE",
            &format!("/api/v1/documents/{document}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "corpo: {body}");

    // Archiviato è invisibile all'elenco, ma **l'oggetto resta nel bucket**: la
    // cancellazione dello storage è un debito dichiarato, non una dimenticanza.
    let (_, page) = app
        .send(
            "GET",
            &format!("/api/v1/leases/{lease}/documents"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(
        page["items"].as_array().map(Vec::len),
        Some(0),
        "corpo: {page}"
    );
    assert_eq!(
        app.storage.len(),
        1,
        "l'oggetto non deve essere stato rimosso"
    );

    // Un documento archiviato non si scarica più: sotto RLS "archiviato" e "non
    // esiste" sono la stessa cosa per il chiamante.
    let (status, problem) = app
        .send(
            "GET",
            &format!("/api/v1/documents/{document}/content"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {problem}");
    assert_eq!(problem["code"], "document.not_found", "corpo: {problem}");
}

/// Un contratto di un'altra organizzazione dà `404` e **non lascia byte nel
/// bucket**: se l'owner si verificasse dopo la scrittura, chiunque potrebbe
/// riempire lo storage puntando a identificatori a caso.
#[tokio::test]
async fn an_upload_to_an_invisible_owner_leaves_no_object_behind() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "proprietario-doc").await;
    let (stranger, _) = common::register_and_login(&app, "estraneo-doc").await;

    let property = create_property(&app, &owner, "Immobile riservato").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    let before = app.storage.len();
    let (status, problem) = app
        .upload(common::Upload {
            path: &format!("/api/v1/leases/{lease}/documents"),
            token: &stranger,
            org: None,
            filename: "curioso.pdf",
            content_type: "application/pdf",
            content: PDF,
            kind: None,
        })
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {problem}");
    assert_eq!(problem["code"], "lease.not_found", "corpo: {problem}");
    assert_eq!(
        app.storage.len(),
        before,
        "un caricamento rifiutato ha lasciato un oggetto orfano"
    );
}

/// L'inquilino allega al **proprio** contratto e vede solo i propri documenti.
///
/// È il ramo `can_access_attachment` delle policy: senza, `documents_insert`
/// pretenderebbe il ruolo di staff e l'inquilino non potrebbe allegare nulla.
#[tokio::test]
async fn a_tenant_attaches_to_their_own_lease_and_sees_nothing_else() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "proprietario-doc2").await;

    let property = create_property(&app, &owner, "Immobile dell'inquilino").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;
    let other_property = create_property(&app, &owner, "Immobile estraneo").await;
    let other_lease = draft_lease(&app, &owner, &other_property, None, 0, Some(365)).await;

    let tenant_email = common::unique_email("inquilino-doc");
    let (_, accept_token) = invite_tenant(&app, &owner, &lease, &tenant_email).await;
    let (tenant, _, _) = common::register_and_login_with_email(&app, &tenant_email).await;
    let (status, accepted) = app
        .post_auth(
            "/api/v1/auth/accept-invitation",
            &tenant,
            serde_json::json!({ "token": accept_token }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {accepted}");

    // Il proprietario allega un documento al contratto dell'inquilino.
    let (status, by_owner) = app
        .upload(common::Upload {
            path: &format!("/api/v1/leases/{lease}/documents"),
            token: &owner,
            org: None,
            filename: "regolamento.pdf",
            content_type: "application/pdf",
            content: PDF,
            kind: Some("other"),
        })
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {by_owner}");
    let owner_document = by_owner["id"].as_str().expect("id").to_owned();

    // L'inquilino allega il proprio. Serve `X-Org-Id`: la sua organizzazione
    // predefinita resta quella personale (debito noto in docs/09).
    let (status, by_tenant) = app
        .upload(common::Upload {
            path: &format!("/api/v1/leases/{lease}/documents"),
            token: &tenant,
            org: Some(&owner_org),
            filename: "carta-identita.pdf",
            content_type: "application/pdf",
            content: PDF,
            kind: Some("id_document"),
        })
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {by_tenant}");
    let tenant_document = by_tenant["id"].as_str().expect("id").to_owned();
    assert_eq!(by_tenant["kind"], "id_document", "corpo: {by_tenant}");

    // Entrambi i documenti sono visibili all'inquilino: sono del suo contratto.
    let (status, page) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/leases/{lease}/documents"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");
    let mut visible: Vec<&str> = page["items"]
        .as_array()
        .expect("elenco")
        .iter()
        .filter_map(|item| item["id"].as_str())
        .collect();
    visible.sort_unstable();
    let mut expected = vec![owner_document.as_str(), tenant_document.as_str()];
    expected.sort_unstable();
    assert_eq!(visible, expected, "corpo: {page}");

    // Il contratto di un altro non esiste, per lui.
    let (status, problem) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/leases/{other_lease}/documents"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {problem}");
    assert_eq!(problem["code"], "lease.not_found", "corpo: {problem}");

    // Può archiviare ciò che ha caricato — è la `USING` di `documents_update` —
    // ma non il documento del proprietario.
    let (status, problem) = app
        .send_in_org(
            "DELETE",
            &format!("/api/v1/documents/{owner_document}"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {problem}");
    // `org.staff_role_required` e non `auth.denied_by_policy`: quest'ultimo
    // significa "difetto nostro", e qui il rifiuto è corretto e previsto.
    assert_eq!(
        problem["code"], "org.staff_role_required",
        "corpo: {problem}"
    );

    let (status, body) = app
        .send_in_org(
            "DELETE",
            &format!("/api/v1/documents/{tenant_document}"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "corpo: {body}");
}

/// Il limite di corpo delle rotte di caricamento è **più largo** di quello
/// generale, e non è un limite dichiarato e mai raggiunto.
///
/// Tre MiB: oltre il limite generale (1 MiB) e oltre il `DefaultBodyLimit` di
/// `Multipart` (2 MiB). Un caricamento che passa dimostra che sono stati alzati
/// entrambi — alzarne uno solo lo farebbe fallire.
#[tokio::test]
async fn the_upload_route_accepts_more_than_the_general_body_limit() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "documenti-grandi").await;
    let property = create_property(&app, &owner, "Immobile con foto").await;

    let big = vec![b'x'; 3 * 1024 * 1024];
    assert!(big.len() > common::BODY_LIMIT_BYTES);

    let (status, created) = app
        .upload(common::Upload {
            path: &format!("/api/v1/properties/{property}/documents"),
            token: &owner,
            org: None,
            filename: "planimetria.png",
            content_type: "image/png",
            content: &big,
            kind: Some("photo"),
        })
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {created}");
    assert_eq!(created["size_bytes"], big.len(), "corpo: {created}");
}

/// …e il limite generale resta stretto su tutto il resto: alzarlo per servire
/// l'upload avrebbe indebolito ogni endpoint, ed è la via che il piano indica
/// come sbagliata.
#[tokio::test]
async fn the_general_body_limit_still_applies_outside_the_upload_routes() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "corpo-grande").await;

    let oversized = "x".repeat(common::BODY_LIMIT_BYTES + 1024);
    let (status, _) = app
        .post_auth(
            "/api/v1/properties",
            &owner,
            serde_json::json!({
                "label": oversized,
                "property_type": "apartment",
                "address": { "street": "Via Roma 1", "city": "Milano", "province": "MI", "postal_code": "20121" }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

/// Un corpo multipart senza la parte `file` è una validazione, non un `500`.
#[tokio::test]
async fn an_upload_without_a_file_part_is_a_validation_error() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "senza-file").await;
    let property = create_property(&app, &owner, "Immobile senza allegati").await;

    let request = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/api/v1/properties/{property}/documents"))
        .header("content-type", "multipart/form-data; boundary=xyz")
        .header("authorization", format!("Bearer {owner}"))
        .body(axum::body::Body::from(
            "--xyz\r\nContent-Disposition: form-data; name=\"kind\"\r\n\r\nphoto\r\n--xyz--\r\n",
        ))
        .expect("richiesta costruita");
    let (status, problem, _) = app.request(request).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");
    assert_eq!(problem["code"], "validation.failed", "corpo: {problem}");
    assert_eq!(problem["errors"][0]["field"], "file", "corpo: {problem}");
}

/// Una classificazione sconosciuta è un `422` con il campo che la nomina, non un
/// ripiego silenzioso su `other`: un enum esteso altrove e non qui deve essere
/// rumoroso.
#[tokio::test]
async fn an_unknown_document_kind_is_refused() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "tipo-ignoto").await;
    let property = create_property(&app, &owner, "Immobile con tipo ignoto").await;

    let (status, problem) = app
        .upload(common::Upload {
            path: &format!("/api/v1/properties/{property}/documents"),
            token: &owner,
            org: None,
            filename: "qualcosa.pdf",
            content_type: "application/pdf",
            content: PDF,
            kind: Some("perizia_giurata"),
        })
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");
    assert_eq!(problem["errors"][0]["field"], "kind", "corpo: {problem}");
}

// ---------------------------------------------------------------------------
// Segnalazioni
// ---------------------------------------------------------------------------

/// Apre una segnalazione e restituisce (id, reference).
async fn open_ticket(
    app: &TestApp,
    token: &str,
    org: Option<&str>,
    property: &str,
    lease: Option<&str>,
    title: &str,
) -> (String, String) {
    let mut body = serde_json::json!({ "title": title, "description": "Descrizione del guasto" });
    if let Some(lease) = lease {
        body["lease_id"] = serde_json::json!(lease);
    }
    let (status, created) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/properties/{property}/tickets"),
            Some(token),
            org,
            Some(body),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {created}");
    (
        created["id"].as_str().expect("id").to_owned(),
        created["reference"].as_str().expect("reference").to_owned(),
    )
}

/// Il giro che dimostra la bidirezionalità: l'inquilino apre, allega una foto,
/// commenta, **non vede la nota interna**, e riapre dopo la chiusura.
///
/// È il test che il piano chiama «come si saprà che è finita», scritto come test
/// invece che come sequenza di `curl`: qui non si può dimenticare di eseguirlo.
#[tokio::test]
async fn the_relationship_runs_in_both_directions() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "proprietario-ticket").await;

    let property = create_property(&app, &owner, "Immobile con segnalazioni").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    // L'inquilino diventa reale.
    let tenant_email = common::unique_email("inquilino-ticket");
    let (_, accept_token) = invite_tenant(&app, &owner, &lease, &tenant_email).await;
    let (tenant, _, _) = common::register_and_login_with_email(&app, &tenant_email).await;
    let (status, accepted) = app
        .post_auth(
            "/api/v1/auth/accept-invitation",
            &tenant,
            serde_json::json!({ "token": accept_token }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {accepted}");

    // 1. L'inquilino apre la segnalazione sul proprio immobile. Serve `X-Org-Id`:
    //    la sua organizzazione predefinita resta quella personale (debito noto).
    let (ticket, reference) = open_ticket(
        &app,
        &tenant,
        Some(&owner_org),
        &property,
        Some(&lease),
        "Il boiler perde",
    )
    .await;
    assert!(
        reference.starts_with("TK-"),
        "il riferimento lo assegna il database: {reference}"
    );

    // 2. Ci allega la foto del guasto. La rotta riusa gli use case dei documenti
    //    senza modificarli: `can_access_attachment` risolveva già `ticket`.
    let (status, photo) = app
        .upload(common::Upload {
            path: &format!("/api/v1/tickets/{ticket}/documents"),
            token: &tenant,
            org: Some(&owner_org),
            filename: "boiler.pdf",
            content_type: "application/pdf",
            content: PDF,
            kind: Some("photo"),
        })
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {photo}");
    assert_eq!(photo["owner_type"], "ticket", "corpo: {photo}");

    let (status, attachments) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/tickets/{ticket}/documents"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {attachments}");
    assert_eq!(attachments["items"].as_array().map(Vec::len), Some(1));

    // 3. Il proprietario classifica e assegna.
    let (status, triaged) = app
        .post_auth(
            &format!("/api/v1/tickets/{ticket}/triage"),
            &owner,
            serde_json::json!({ "category": "plumbing", "priority": "high" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {triaged}");
    assert_eq!(triaged["status"], "triaged", "corpo: {triaged}");
    assert_eq!(triaged["category"], "plumbing");

    let owner_id = triaged["reported_by"]
        .as_str()
        .map(str::to_owned)
        .expect("chi ha segnalato è l'inquilino");
    let (status, assigned) = app
        .post_auth(
            &format!("/api/v1/tickets/{ticket}/assign"),
            &owner,
            serde_json::json!({ "assignee_id": owner_id }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {assigned}");
    assert_eq!(assigned["status"], "assigned");

    // 4. Due messaggi, uno per parte.
    for (token, org, body) in [
        (&tenant, Some(owner_org.as_str()), "Sono in casa dopo le 18"),
        (&owner, None, "Mando l'idraulico giovedì"),
    ] {
        let (status, message) = app
            .send_in_org(
                "POST",
                &format!("/api/v1/tickets/{ticket}/messages"),
                Some(token),
                org,
                Some(serde_json::json!({ "body": body })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "corpo: {message}");
    }

    // 5. Il proprietario scrive una nota interna. L'inquilino non la vede: la
    //    filtra `ticket_messages_select`, non un ramo applicativo.
    let (status, note) = app
        .post_auth(
            &format!("/api/v1/tickets/{ticket}/messages"),
            &owner,
            serde_json::json!({ "body": "Preventivo gonfiato, trattare", "is_internal": true }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {note}");

    let (status, thread) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/tickets/{ticket}/messages"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {thread}");
    let visible = thread["items"].as_array().expect("elenco");
    assert_eq!(visible.len(), 2, "l'inquilino vede due messaggi: {thread}");
    assert!(
        !thread.to_string().contains("gonfiato"),
        "la nota interna è arrivata all'inquilino: {thread}"
    );

    // Il proprietario invece la vede, ed è lo stesso thread.
    let (status, staff_thread) = app
        .send(
            "GET",
            &format!("/api/v1/tickets/{ticket}/messages"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {staff_thread}");
    assert_eq!(staff_thread["items"].as_array().map(Vec::len), Some(3));

    // 6. Il proprietario registra stima e costo sostenuto, e chiude dicendo cosa
    //    è stato fatto.
    let (status, priced) = app
        .send(
            "PATCH",
            &format!("/api/v1/tickets/{ticket}"),
            Some(&owner),
            Some(serde_json::json!({ "estimated_cost": "200.00", "actual_cost": "180.00" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {priced}");
    assert_eq!(priced["estimated_cost"], "200.00", "corpo: {priced}");
    assert_eq!(priced["actual_cost"], "180.00", "corpo: {priced}");
    assert_eq!(priced["currency"], "EUR");

    // L'inquilino vede il costo sostenuto e **non** la stima: quella è la cifra
    // di una trattativa con un fornitore e non riguarda chi abita la casa.
    let (status, seen_by_tenant) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/tickets/{ticket}"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {seen_by_tenant}");
    assert_eq!(
        seen_by_tenant["actual_cost"], "180.00",
        "corpo: {seen_by_tenant}"
    );
    assert!(
        seen_by_tenant["estimated_cost"].is_null(),
        "la stima è arrivata all'inquilino: {seen_by_tenant}"
    );

    let (status, closed) = app
        .post_auth(
            &format!("/api/v1/tickets/{ticket}/close"),
            &owner,
            serde_json::json!({ "resolution_notes": "sostituita la valvola di sicurezza" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {closed}");
    assert_eq!(closed["status"], "closed");
    assert!(closed["closed_at"].is_string(), "corpo: {closed}");
    assert_eq!(
        closed["resolution_notes"],
        "sostituita la valvola di sicurezza"
    );

    // 7. Il guasto torna: **l'inquilino** riapre. È la sola transizione che può
    //    eseguire, ed è ciò che rende la segnalazione un rapporto.
    let (status, reopened) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/tickets/{ticket}/reopen"),
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {reopened}");
    assert_eq!(reopened["status"], "open");
    assert!(
        reopened["closed_at"].is_null(),
        "una segnalazione riaperta con una data di chiusura è una riga che mente: {reopened}"
    );
    // Riaprire disfa la chiusura, e la nota di risoluzione ne fa parte: una
    // segnalazione aperta che dichiara come è stata risolta direbbe il falso.
    assert!(
        reopened.get("resolution_notes").is_none(),
        "la nota di risoluzione è sopravvissuta alla riapertura: {reopened}"
    );

    // 8. Lo storico mostra la catena intera, riapertura compresa.
    let (status, detail) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/tickets/{ticket}"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {detail}");
    let chain: Vec<&str> = detail["history"]
        .as_array()
        .expect("storico")
        .iter()
        .filter_map(|row| row["to_status"].as_str())
        .collect();
    assert_eq!(
        chain,
        vec!["open", "triaged", "assigned", "closed", "open"],
        "corpo: {detail}"
    );
    // Lo storico lo scrive il trigger, e la prima riga non ha uno stato di
    // partenza: la produce l'`INSERT`, non una transizione.
    assert!(
        detail["history"][0]["from_status"].is_null(),
        "corpo: {detail}"
    );
}

/// L'inquilino non guida la lavorazione, e non scrive note interne.
#[tokio::test]
async fn a_tenant_can_speak_but_not_drive_the_handling() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "proprietario-ruoli").await;
    let property = create_property(&app, &owner, "Immobile dei ruoli").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    let tenant_email = common::unique_email("inquilino-ruoli");
    let (_, accept_token) = invite_tenant(&app, &owner, &lease, &tenant_email).await;
    let (tenant, _, _) = common::register_and_login_with_email(&app, &tenant_email).await;
    app.post_auth(
        "/api/v1/auth/accept-invitation",
        &tenant,
        serde_json::json!({ "token": accept_token }),
    )
    .await;

    let (ticket, _) = open_ticket(
        &app,
        &tenant,
        Some(&owner_org),
        &property,
        Some(&lease),
        "Guasto dell'inquilino",
    )
    .await;

    // Le transizioni dello staff gli sono precluse.
    for path in ["triage", "assign", "start", "close"] {
        let body = match path {
            "triage" => serde_json::json!({ "category": "other", "priority": "low" }),
            "assign" => serde_json::json!({ "assignee_id": uuid::Uuid::now_v7().to_string() }),
            _ => serde_json::json!({}),
        };
        let (status, problem) = app
            .send_in_org(
                "POST",
                &format!("/api/v1/tickets/{ticket}/{path}"),
                Some(&tenant),
                Some(&owner_org),
                Some(body),
            )
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "su `{path}`: {problem}");
        // `org.staff_role_required` e non `auth.denied_by_policy`: quest'ultimo
        // significa "difetto nostro", e qui il rifiuto è corretto e previsto.
        assert_eq!(problem["code"], "org.staff_role_required", "su `{path}`");
    }

    // Nemmeno il PATCH sui costi: il costo di una riparazione lo registra chi paga.
    let (status, problem) = app
        .send_in_org(
            "PATCH",
            &format!("/api/v1/tickets/{ticket}"),
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({ "actual_cost": "1.00" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {problem}");

    // E nemmeno una nota interna: il service layer la rifiuta **prima** della
    // policy, che darebbe `auth.denied_by_policy`.
    let (status, problem) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/tickets/{ticket}/messages"),
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({ "body": "nota", "is_internal": true })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {problem}");
    assert_eq!(problem["code"], "org.staff_role_required");

    // Annullare invece sì, finché nessuno ci ha messo mano.
    let (status, cancelled) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/tickets/{ticket}/cancel"),
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {cancelled}");
    assert_eq!(cancelled["status"], "cancelled");
}

/// Chi condivide il contratto vede la segnalazione ma non la riapre, e il codice
/// glielo dice per quello che è.
///
/// La `USING` di `tickets_update` ammette staff, chi ha segnalato e chi è
/// assegnato: non passa dal contratto. `org.staff_role_required` direbbe al
/// coinquilino che gli manca un ruolo, mentre gli manca l'essere stato lui a
/// segnalare — e nessun ruolo glielo darebbe.
#[tokio::test]
async fn a_cotenant_sees_the_ticket_but_cannot_reopen_it() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "proprietario-coinquilini").await;
    let property = create_property(&app, &owner, "Casa condivisa").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    // Due inquilini sullo stesso contratto.
    let mut tokens = Vec::new();
    for prefix in ["coinquilino-uno", "coinquilino-due"] {
        let email = common::unique_email(prefix);
        let (_, accept_token) = invite_tenant(&app, &owner, &lease, &email).await;
        let (token, _, _) = common::register_and_login_with_email(&app, &email).await;
        let (status, accepted) = app
            .post_auth(
                "/api/v1/auth/accept-invitation",
                &token,
                serde_json::json!({ "token": accept_token }),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "corpo: {accepted}");
        tokens.push(token);
    }
    let (reporter, cotenant) = (&tokens[0], &tokens[1]);

    let (ticket, _) = open_ticket(
        &app,
        reporter,
        Some(&owner_org),
        &property,
        Some(&lease),
        "Perdita in cucina",
    )
    .await;
    app.post_auth(
        &format!("/api/v1/tickets/{ticket}/close"),
        &owner,
        serde_json::json!({}),
    )
    .await;

    // Il coinquilino la vede: gliela concede can_access_ticket attraverso il
    // contratto.
    let (status, detail) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/tickets/{ticket}"),
            Some(cotenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {detail}");

    // Ma non la riapre, e il codice non parla di ruoli.
    let (status, problem) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/tickets/{ticket}/reopen"),
            Some(cotenant),
            Some(&owner_org),
            Some(serde_json::json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {problem}");
    assert_eq!(problem["code"], "ticket.not_yours", "corpo: {problem}");

    // Chi ha segnalato invece sì.
    let (status, reopened) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/tickets/{ticket}/reopen"),
            Some(reporter),
            Some(&owner_org),
            Some(serde_json::json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {reopened}");
}

/// Il riferimento riparte da uno in ogni organizzazione, e non dice quante
/// segnalazioni esistono sulla piattaforma.
#[tokio::test]
async fn the_reference_restarts_from_one_in_every_organization() {
    let app = require_app!();

    let mut references = Vec::new();
    for prefix in ["riferimenti-alfa", "riferimenti-beta"] {
        let (owner, _) = common::register_and_login(&app, prefix).await;
        let property = create_property(&app, &owner, "Immobile numerato").await;
        let (_, first) = open_ticket(&app, &owner, None, &property, None, "La prima").await;
        let (_, second) = open_ticket(&app, &owner, None, &property, None, "La seconda").await;
        references.push((first, second));
    }

    for (first, second) in &references {
        assert_eq!(first, "TK-000001", "ogni organizzazione riparte da uno");
        assert_eq!(second, "TK-000002");
    }
    assert_eq!(
        references[0], references[1],
        "due organizzazioni indipendenti devono numerare allo stesso modo"
    );
}

/// Una segnalazione di un'altra organizzazione non esiste, e l'elenco non la
/// nomina.
#[tokio::test]
async fn a_ticket_of_another_organization_is_not_found() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "ticket-mio").await;
    let (stranger, _) = common::register_and_login(&app, "ticket-estraneo").await;

    let property = create_property(&app, &owner, "Immobile riservato").await;
    let (ticket, _) = open_ticket(&app, &owner, None, &property, None, "Solo mio").await;

    for (method, path, body) in [
        ("GET", format!("/api/v1/tickets/{ticket}"), None),
        (
            "GET",
            format!("/api/v1/tickets/{ticket}/messages"),
            None::<serde_json::Value>,
        ),
        (
            "POST",
            format!("/api/v1/tickets/{ticket}/close"),
            Some(serde_json::json!({})),
        ),
    ] {
        let (status, problem) = app.send(method, &path, Some(&stranger), body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "su `{path}`: {problem}");
        assert_eq!(problem["code"], "ticket.not_found", "su `{path}`");
    }

    let (status, page) = app
        .send("GET", "/api/v1/tickets", Some(&stranger), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");
    assert_eq!(page["items"].as_array().map(Vec::len), Some(0));
}

/// Il filtro della coda usa i sei stati di lavorazione, non `status <> 'closed'`.
#[tokio::test]
async fn the_queue_filter_separates_working_from_terminal() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "ticket-coda").await;
    let property = create_property(&app, &owner, "Immobile della coda").await;

    let (aperta, _) = open_ticket(&app, &owner, None, &property, None, "Ancora aperta").await;
    let (chiusa, _) = open_ticket(&app, &owner, None, &property, None, "Da chiudere").await;
    app.post_auth(
        &format!("/api/v1/tickets/{chiusa}/close"),
        &owner,
        serde_json::json!({}),
    )
    .await;

    let ids = |page: &serde_json::Value| -> Vec<String> {
        page["items"]
            .as_array()
            .expect("elenco")
            .iter()
            .filter_map(|item| item["id"].as_str().map(str::to_owned))
            .collect()
    };

    let (status, queue) = app
        .send(
            "GET",
            &format!("/api/v1/tickets?property_id={property}&open=true"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {queue}");
    assert_eq!(ids(&queue), vec![aperta.clone()], "corpo: {queue}");

    let (status, done) = app
        .send(
            "GET",
            &format!("/api/v1/tickets?property_id={property}&open=false"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {done}");
    assert_eq!(ids(&done), vec![chiusa], "corpo: {done}");

    // E il filtro per stato memorizzato resta disponibile.
    let (status, by_status) = app
        .send(
            "GET",
            &format!("/api/v1/tickets?property_id={property}&status=open"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {by_status}");
    assert_eq!(ids(&by_status), vec![aperta]);
}

/// Le transizioni non ammesse danno `409` con un codice che il client può
/// instradare, e non un successo silenzioso.
#[tokio::test]
async fn a_transition_that_is_not_allowed_is_a_conflict() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "ticket-transizioni").await;
    let property = create_property(&app, &owner, "Immobile delle transizioni").await;
    let (ticket, _) = open_ticket(&app, &owner, None, &property, None, "Appena aperta").await;

    // Da `open` non si avvia la lavorazione: prima si assegna.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/tickets/{ticket}/start"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "ticket.invalid_transition");

    // Né si riapre ciò che non è chiuso.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/tickets/{ticket}/reopen"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "ticket.invalid_transition");

    // Una segnalazione annullata non ha costi.
    app.post_auth(
        &format!("/api/v1/tickets/{ticket}/cancel"),
        &owner,
        serde_json::json!({}),
    )
    .await;
    let (status, problem) = app
        .send(
            "PATCH",
            &format!("/api/v1/tickets/{ticket}"),
            Some(&owner),
            Some(serde_json::json!({ "actual_cost": "10.00" })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "ticket.cancelled");
}

/// Un `PATCH` vuoto non è un errore, e non produce una modifica.
///
/// `null` invece azzera: è la differenza che rende un PATCH capace di cancellare
/// un valore, e non solo di sovrascriverlo.
#[tokio::test]
async fn the_cost_patch_tells_absent_from_null() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "ticket-costi").await;
    let property = create_property(&app, &owner, "Immobile dei costi").await;
    let (ticket, _) = open_ticket(&app, &owner, None, &property, None, "Con preventivo").await;

    let path = format!("/api/v1/tickets/{ticket}");
    let patch = |body: serde_json::Value| app.send("PATCH", &path, Some(&owner), Some(body));

    let (status, set) = patch(serde_json::json!({ "estimated_cost": "120.00" })).await;
    assert_eq!(status, StatusCode::OK, "corpo: {set}");
    assert_eq!(set["estimated_cost"], "120.00");

    let (status, untouched) = patch(serde_json::json!({})).await;
    assert_eq!(status, StatusCode::OK, "corpo: {untouched}");
    assert_eq!(
        untouched["estimated_cost"], "120.00",
        "un campo assente resta com'era: {untouched}"
    );

    let (status, cleared) = patch(serde_json::json!({ "estimated_cost": null })).await;
    assert_eq!(status, StatusCode::OK, "corpo: {cleared}");
    assert!(
        cleared["estimated_cost"].is_null(),
        "`null` azzera: {cleared}"
    );

    let (status, problem) = patch(serde_json::json!({ "actual_cost": "-1.00" })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");
    assert_eq!(problem["errors"][0]["field"], "actual_cost");
}

/// Stanza e contratto di un **altro** immobile vengono rifiutati all'apertura,
/// che è l'unico momento in cui l'incoerenza può nascere: dopo l'inserimento
/// quelle colonne non sono più scrivibili (migration `0020`).
#[tokio::test]
async fn a_ticket_refuses_references_of_another_property() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "ticket-riferimenti").await;
    let target = create_property(&app, &owner, "Immobile bersaglio").await;
    let other = create_property(&app, &owner, "Immobile estraneo").await;

    let (_, room) = app
        .post_auth(
            &format!("/api/v1/properties/{other}/rooms"),
            &owner,
            serde_json::json!({ "name": "Camera altrove" }),
        )
        .await;
    let room = room["id"].as_str().expect("id").to_owned();
    let foreign_lease = draft_lease(&app, &owner, &other, None, 0, Some(365)).await;

    for (field, value) in [("room_id", room), ("lease_id", foreign_lease)] {
        let (status, problem) = app
            .post_auth(
                &format!("/api/v1/properties/{target}/tickets"),
                &owner,
                serde_json::json!({
                    "title": "Incoerente",
                    "description": "Riferimento di un altro immobile",
                    field: value,
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "su `{field}`: {problem}"
        );
        assert_eq!(problem["code"], "validation.failed", "su `{field}`");
        assert_eq!(problem["errors"][0]["field"], field);
    }
}

/// Il titolo rispetta i limiti dello schema, e il corpo di un messaggio non può
/// essere vuoto.
#[tokio::test]
async fn a_ticket_validates_its_text() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "ticket-validazione").await;
    let property = create_property(&app, &owner, "Immobile della validazione").await;

    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/tickets"),
            &owner,
            serde_json::json!({ "title": "   ", "description": "c'è" }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");
    assert_eq!(problem["errors"][0]["field"], "title");

    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/tickets"),
            &owner,
            serde_json::json!({ "title": "a".repeat(201), "description": "c'è" }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");

    let (ticket, _) = open_ticket(&app, &owner, None, &property, None, "Valida").await;
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/tickets/{ticket}/messages"),
            &owner,
            serde_json::json!({ "body": "  " }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");
    assert_eq!(problem["errors"][0]["field"], "body");
}

// ---------------------------------------------------------------------------
// Bollette e addebiti
// ---------------------------------------------------------------------------

/// Registra una bolletta sull'immobile e restituisce il suo id.
///
/// Il numero di fattura è irripetibile: `utility_bills_dedup_uk` è unico su
/// `(org_id, supplier_vat, invoice_number)` e i test girano sullo stesso
/// database.
async fn register_bill(
    app: &TestApp,
    token: &str,
    property: &str,
    lease: Option<&str>,
    invoice_number: &str,
) -> String {
    let mut body = serde_json::json!({
        "utility_type": "gas",
        "supplier_vat": "IT12345678901",
        "invoice_number": invoice_number,
        "period_start": day(-90),
        "period_end": day(-31),
        "total_amount": "184.20",
    });
    if let Some(lease) = lease {
        body["lease_id"] = serde_json::json!(lease);
    }

    let (status, created) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/utility-bills"),
            token,
            body,
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {created}");
    assert_eq!(created["status"], "draft", "corpo: {created}");
    created["id"].as_str().expect("id").to_owned()
}

/// Il mese del proprietario, dal bordo HTTP: registra, corregge, valida,
/// riaddebita; l'inquilino vede addebito e bolletta con il saldo a zero.
#[tokio::test]
async fn a_bill_becomes_a_charge_the_tenant_can_read() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "bollette-proprietario").await;
    let property = create_property(&app, &owner, "Immobile con utenze").await;
    let lease = draft_lease(&app, &owner, &property, None, -10, Some(355)).await;

    let email = common::unique_email("bollette-inquilino");
    add_tenant(&app, &owner, &lease, "Inquilino Bollette", Some(&email)).await;
    let (status, activated) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {activated}");

    let (_, accept_token) = invite_tenant(&app, &owner, &lease, &email).await;
    let (tenant, _, _) = common::register_and_login_with_email(&app, &email).await;
    let (status, accepted) = app
        .post_auth(
            "/api/v1/auth/accept-invitation",
            &tenant,
            serde_json::json!({ "token": accept_token }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {accepted}");

    let bill = register_bill(&app, &owner, &property, Some(&lease), "2026/H/1").await;

    // Il numero di fattura era sbagliato: si corregge, e la bolletta resta in
    // bozza perché nessuno l'aveva ancora validata.
    let (status, corrected) = app
        .send(
            "PATCH",
            &format!("/api/v1/utility-bills/{bill}"),
            Some(&owner),
            Some(serde_json::json!({ "invoice_number": "2026/H/2" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {corrected}");
    assert_eq!(corrected["invoice_number"], "2026/H/2");
    assert_eq!(corrected["status"], "draft");

    let (status, validated) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/validate"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {validated}");
    assert_eq!(validated["status"], "validated");
    assert!(validated["validated_at"].is_string(), "corpo: {validated}");

    let (status, recharged) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/recharge"),
            &owner,
            serde_json::json!({
                "lease_id": lease,
                "amount": "120.00",
                "due_date": day(10),
                "description": "Gas, bimestre",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {recharged}");
    assert_eq!(recharged["utility_bill"]["status"], "allocated");
    assert_eq!(recharged["charge"]["charge_type"], "utility_recharge");
    assert_eq!(recharged["charge"]["utility_bill_id"], bill);
    let charge = recharged["charge"]["id"].as_str().expect("id").to_owned();

    // L'inquilino legge l'addebito, con il saldo che dice che non ha pagato.
    let (status, seen) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/charges/{charge}"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {seen}");
    assert_eq!(seen["amount"], "120.00");
    assert_eq!(
        seen["paid_amount"], "0.00",
        "senza il modulo dei pagamenti il saldo è per forza zero: {seen}"
    );
    assert_eq!(seen["outstanding_amount"], "120.00");
    assert_eq!(seen["status"], "issued");
    assert_eq!(seen["effective_status"], "issued");

    // E vede anche la bolletta, perché è imputata al suo contratto.
    let (status, bill_seen) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/utility-bills/{bill}"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {bill_seen}");
    assert_eq!(bill_seen["total_amount"], "184.20");
}

/// Lo stato scaduto compare in `effective_status` e **non** in `status`.
#[tokio::test]
async fn an_overdue_charge_says_so_without_the_column_ever_saying_it() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "addebiti-scaduti").await;
    let property = create_property(&app, &owner, "Immobile morosi").await;
    let lease = draft_lease(&app, &owner, &property, None, -30, Some(335)).await;

    let (status, charge) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/charges"),
            &owner,
            serde_json::json!({
                "charge_type": "rent",
                "amount": "800.00",
                "due_date": day(-5),
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {charge}");
    // Nasce **emesso**: non esiste uno stato di bozza.
    assert_eq!(charge["status"], "issued", "corpo: {charge}");
    assert!(charge["issued_at"].is_string(), "corpo: {charge}");
    let id = charge["id"].as_str().expect("id").to_owned();

    let (status, read) = app
        .send("GET", &format!("/api/v1/charges/{id}"), Some(&owner), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {read}");
    assert_eq!(read["effective_status"], "overdue", "corpo: {read}");
    assert_eq!(
        read["status"], "issued",
        "la colonna non porta mai `overdue`: lo stato si deriva (ADR-019)"
    );
}

/// `?status=overdue` è un `422` con il rimedio, non una pagina vuota.
///
/// Stessa forma di `?status=expiring` sui contratti: restituire zero righe sarebbe
/// il modo peggiore di dire che in database quel valore non esiste.
#[tokio::test]
async fn filtering_charges_by_the_derived_status_is_refused_with_a_hint() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "filtro-derivato").await;

    let (status, problem) = app
        .send("GET", "/api/v1/charges?status=overdue", Some(&owner), None)
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");
    assert_eq!(
        problem["code"], "charge.derived_status_filter",
        "corpo: {problem}"
    );
    // Il rimedio viaggia in `errors[].message`, dove sta l'errore per campo: la
    // forma della risposta resta quella di una validazione, cambia solo il codice.
    assert_eq!(problem["errors"][0]["field"], "status", "corpo: {problem}");
    assert!(
        problem["errors"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("outstanding")),
        "il messaggio deve dire come si fa: {problem}"
    );

    // Il filtro giusto risponde invece `200`.
    let (status, page) = app
        .send(
            "GET",
            "/api/v1/charges?outstanding=true",
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");
}

/// L'asimmetria fra i due annullamenti, dal bordo HTTP.
#[tokio::test]
async fn a_voided_charge_stays_visible_and_a_voided_bill_frees_its_invoice_number() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "annulli-http").await;
    let property = create_property(&app, &owner, "Immobile annulli HTTP").await;
    let lease = draft_lease(&app, &owner, &property, None, -10, Some(355)).await;

    let email = common::unique_email("annulli-inquilino");
    add_tenant(&app, &owner, &lease, "Inquilino Annulli", Some(&email)).await;
    app.post_auth(
        &format!("/api/v1/leases/{lease}/activate"),
        &owner,
        serde_json::json!({}),
    )
    .await;
    let (_, accept_token) = invite_tenant(&app, &owner, &lease, &email).await;
    let (tenant, _, _) = common::register_and_login_with_email(&app, &email).await;
    app.post_auth(
        "/api/v1/auth/accept-invitation",
        &tenant,
        serde_json::json!({ "token": accept_token }),
    )
    .await;

    let (_, charge) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/charges"),
            &owner,
            serde_json::json!({
                "charge_type": "rent",
                "amount": "800.00",
                "due_date": day(15),
            }),
        )
        .await;
    let charge_id = charge["id"].as_str().expect("id").to_owned();

    let (status, voided) = app
        .post_auth(
            &format!("/api/v1/charges/{charge_id}/void"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {voided}");
    assert_eq!(voided["status"], "void");

    // L'inquilino continua a vederlo: si era visto chiedere quella somma.
    let (status, seen) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/charges/{charge_id}"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {seen}");
    assert_eq!(seen["status"], "void");

    // Annullare due volte è una transizione non ammessa.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/charges/{charge_id}/void"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "charge.invalid_transition");

    // La bolletta invece sparisce, e il numero di fattura torna libero.
    let bill = register_bill(&app, &owner, &property, Some(&lease), "2026/I/1").await;
    let (status, voided_bill) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/void"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {voided_bill}");

    let (status, gone) = app
        .send(
            "GET",
            &format!("/api/v1/utility-bills/{bill}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "una bolletta annullata sparisce: {gone}"
    );

    let reinserted = register_bill(&app, &owner, &property, Some(&lease), "2026/I/1").await;
    assert_ne!(
        reinserted, bill,
        "il numero di fattura doveva tornare libero"
    );
}

/// La stessa fattura due volte è un conflitto **nominato**.
#[tokio::test]
async fn the_same_invoice_twice_is_a_named_conflict() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "deduplica-http").await;
    let property = create_property(&app, &owner, "Immobile doppioni HTTP").await;

    register_bill(&app, &owner, &property, None, "2026/L/1").await;

    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/utility-bills"),
            &owner,
            serde_json::json!({
                "utility_type": "gas",
                "supplier_vat": "IT12345678901",
                "invoice_number": "2026/L/1",
                "total_amount": "10.00",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(
        problem["code"], "utility_bill.duplicate",
        "corpo: {problem}"
    );
}

/// Da `allocated` in poi la bolletta è ferma, e la correzione è un `409`.
#[tokio::test]
async fn a_recharged_bill_can_no_longer_be_corrected_or_voided() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "bolletta-ferma").await;
    let property = create_property(&app, &owner, "Immobile fermo").await;
    let lease = draft_lease(&app, &owner, &property, None, -10, Some(355)).await;
    let bill = register_bill(&app, &owner, &property, Some(&lease), "2026/M/1").await;

    app.post_auth(
        &format!("/api/v1/utility-bills/{bill}/validate"),
        &owner,
        serde_json::json!({}),
    )
    .await;
    let (status, recharged) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/recharge"),
            &owner,
            serde_json::json!({
                "lease_id": lease,
                "amount": "50.00",
                "due_date": day(20),
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {recharged}");

    let (status, problem) = app
        .send(
            "PATCH",
            &format!("/api/v1/utility-bills/{bill}"),
            Some(&owner),
            Some(serde_json::json!({ "total_amount": "1.00" })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "utility_bill.invalid_transition");

    // E non si annulla: c'è un addebito che la cita.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/void"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");

    // Riaddebitarla ancora invece sì: è il caso del co-living per stanza.
    let (status, again) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/recharge"),
            &owner,
            serde_json::json!({
                "lease_id": lease,
                "amount": "70.00",
                "due_date": day(20),
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {again}");
}

/// Correggere una bolletta **validata** la riporta in bozza.
#[tokio::test]
async fn correcting_a_validated_bill_takes_the_validation_away() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "revoca-validazione").await;
    let property = create_property(&app, &owner, "Immobile revisioni HTTP").await;
    let bill = register_bill(&app, &owner, &property, None, "2026/N/1").await;

    app.post_auth(
        &format!("/api/v1/utility-bills/{bill}/validate"),
        &owner,
        serde_json::json!({}),
    )
    .await;

    let (status, corrected) = app
        .send(
            "PATCH",
            &format!("/api/v1/utility-bills/{bill}"),
            Some(&owner),
            Some(serde_json::json!({ "total_amount": "200.00" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {corrected}");
    assert_eq!(
        corrected["status"], "draft",
        "cambiare i numeri smentisce la validazione: {corrected}"
    );
    assert!(corrected["validated_at"].is_null(), "corpo: {corrected}");
    assert!(corrected["validated_by"].is_null(), "corpo: {corrected}");
}

/// L'inquilino legge ma non scrive: la contabilità è dello staff.
#[tokio::test]
async fn a_tenant_reads_the_accounting_but_never_writes_it() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "contabilita-inquilino").await;
    let property = create_property(&app, &owner, "Immobile in affitto").await;
    let lease = draft_lease(&app, &owner, &property, None, -10, Some(355)).await;

    let email = common::unique_email("contabilita-tenant");
    add_tenant(&app, &owner, &lease, "Inquilino Contabile", Some(&email)).await;
    app.post_auth(
        &format!("/api/v1/leases/{lease}/activate"),
        &owner,
        serde_json::json!({}),
    )
    .await;
    let (_, accept_token) = invite_tenant(&app, &owner, &lease, &email).await;
    let (tenant, _, _) = common::register_and_login_with_email(&app, &email).await;
    app.post_auth(
        "/api/v1/auth/accept-invitation",
        &tenant,
        serde_json::json!({ "token": accept_token }),
    )
    .await;

    // Registrare una bolletta: `403`, e il codice parla di ruolo.
    let (status, problem) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/properties/{property}/utility-bills"),
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({
                "utility_type": "water",
                "total_amount": "10.00",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {problem}");
    assert_eq!(problem["code"], "org.staff_role_required");

    // Emettere un addebito a sé stesso: idem.
    let (status, problem) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/leases/{lease}/charges"),
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({
                "charge_type": "rent",
                "amount": "1.00",
                "due_date": day(30),
            })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {problem}");

    // Ma l'elenco degli addebiti lo legge.
    let (status, page) = app
        .send_in_org(
            "GET",
            "/api/v1/charges",
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");
}

/// Un'altra organizzazione riceve `404` su tutto, e l'elenco vuoto.
#[tokio::test]
async fn billing_of_another_organization_is_not_found() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "contabilita-riservata").await;
    let property = create_property(&app, &owner, "Immobile riservato HTTP").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;
    let bill = register_bill(&app, &owner, &property, Some(&lease), "2026/O/1").await;
    let (_, charge) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/charges"),
            &owner,
            serde_json::json!({
                "charge_type": "deposit",
                "amount": "1600.00",
                "due_date": day(7),
            }),
        )
        .await;
    let charge_id = charge["id"].as_str().expect("id").to_owned();

    let (stranger, _) = common::register_and_login(&app, "contabilita-estranea").await;

    for path in [
        format!("/api/v1/utility-bills/{bill}"),
        format!("/api/v1/charges/{charge_id}"),
        format!("/api/v1/utility-bills/{bill}/documents"),
        format!("/api/v1/charges/{charge_id}/documents"),
    ] {
        let (status, problem) = app.send("GET", &path, Some(&stranger), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path} → {problem}");
    }

    for path in [
        format!("/api/v1/utility-bills/{bill}/validate"),
        format!("/api/v1/utility-bills/{bill}/void"),
        format!("/api/v1/charges/{charge_id}/void"),
        format!("/api/v1/charges/{charge_id}/write-off"),
    ] {
        let (status, problem) = app.post_auth(&path, &stranger, serde_json::json!({})).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path} → {problem}");
    }

    let (status, page) = app
        .send("GET", "/api/v1/utility-bills", Some(&stranger), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");
    assert_eq!(
        page["items"].as_array().map(Vec::len),
        Some(0),
        "l'elenco di un estraneo deve essere vuoto: {page}"
    );
}

/// La scansione si allega alla bolletta e si scarica.
#[tokio::test]
async fn a_bill_carries_its_scan() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "scansione-bolletta").await;
    let property = create_property(&app, &owner, "Immobile con scansioni").await;
    let bill = register_bill(&app, &owner, &property, None, "2026/P/1").await;

    let (status, uploaded) = app
        .upload(common::Upload {
            path: &format!("/api/v1/utility-bills/{bill}/documents"),
            token: &owner,
            org: None,
            filename: "bolletta-gas.pdf",
            content_type: "application/pdf",
            content: b"%PDF-1.7 finto",
            kind: Some("utility_bill"),
        })
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {uploaded}");
    assert_eq!(uploaded["owner_type"], "utility_bill", "corpo: {uploaded}");
    assert_eq!(uploaded["owner_id"], bill);

    let (status, page) = app
        .send(
            "GET",
            &format!("/api/v1/utility-bills/{bill}/documents"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");
    assert_eq!(page["items"].as_array().map(Vec::len), Some(1));
}

/// Un `PATCH` su un addebito **non esiste**: la rotta non c'è.
///
/// Non è un dettaglio di routing: la migration `0023` toglie al ruolo applicativo
/// il privilegio di riscrivere qualunque colonna dell'addebito che non sia
/// `status`, quindi un `PATCH` non avrebbe nulla da scrivere. Prorogare una
/// scadenza si fa annullando e riemettendo.
#[tokio::test]
async fn a_charge_has_no_patch_route_at_all() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "niente-patch").await;
    let property = create_property(&app, &owner, "Immobile senza patch").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;
    let (_, charge) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/charges"),
            &owner,
            serde_json::json!({
                "charge_type": "rent",
                "amount": "800.00",
                "due_date": day(10),
            }),
        )
        .await;
    let id = charge["id"].as_str().expect("id").to_owned();

    let (status, _) = app
        .send(
            "PATCH",
            &format!("/api/v1/charges/{id}"),
            Some(&owner),
            Some(serde_json::json!({ "amount": "1.00" })),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::METHOD_NOT_ALLOWED,
        "l'API non deve offrire un modo di riscrivere una pretesa già comunicata"
    );
}

/// La rinuncia all'incasso è diversa dall'annullamento, e i due si escludono.
#[tokio::test]
async fn writing_off_is_not_the_same_as_voiding() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "rinuncia").await;
    let property = create_property(&app, &owner, "Immobile perdite").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;
    let (_, charge) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/charges"),
            &owner,
            serde_json::json!({
                "charge_type": "late_fee",
                "amount": "40.00",
                "due_date": day(-2),
            }),
        )
        .await;
    let id = charge["id"].as_str().expect("id").to_owned();

    let (status, written_off) = app
        .post_auth(
            &format!("/api/v1/charges/{id}/write-off"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {written_off}");
    assert_eq!(written_off["status"], "written_off");
    assert_eq!(
        written_off["effective_status"], "written_off",
        "una rinuncia non diventa scaduta col passare del tempo: {written_off}"
    );

    // E ora non si annulla più: le due uscite si escludono.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/charges/{id}/void"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
}

/// Il consumo ammette **quattro** decimali, gli importi due.
///
/// È la trappola del value object: passare il consumo come importo darebbe un
/// `422` che parla di «amount», mandando a cercare l'errore fra le cifre della
/// bolletta invece che nel tipo sbagliato.
#[tokio::test]
async fn a_consumption_keeps_four_decimals_while_amounts_keep_two() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "consumo").await;
    let property = create_property(&app, &owner, "Immobile consumi").await;

    let (status, created) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/utility-bills"),
            &owner,
            serde_json::json!({
                "utility_type": "electricity",
                "total_amount": "184.20",
                "consumption": { "value": "123.4567", "unit": "kWh" },
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {created}");
    assert_eq!(created["consumption"]["value"], "123.4567");
    assert_eq!(created["consumption"]["unit"], "kWh");

    // Un importo con quattro decimali invece è rifiutato.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/utility-bills"),
            &owner,
            serde_json::json!({
                "utility_type": "electricity",
                "total_amount": "184.2000000",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");
}

/// Stanza e contratto devono appartenere all'immobile: lo schema non lo verifica.
#[tokio::test]
async fn a_bill_cannot_point_at_a_lease_of_another_property() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "riferimenti-bolletta").await;
    let first = create_property(&app, &owner, "Primo immobile").await;
    let second = create_property(&app, &owner, "Secondo immobile").await;
    let lease_on_second = draft_lease(&app, &owner, &second, None, 0, Some(365)).await;

    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/properties/{first}/utility-bills"),
            &owner,
            serde_json::json!({
                "utility_type": "water",
                "total_amount": "50.00",
                "lease_id": lease_on_second,
            }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");

    // E lo stesso vale sul `PATCH`, dove quelle colonne restano scrivibili: è la
    // differenza con le segnalazioni, dove il controllo basta all'apertura.
    let bill = register_bill(&app, &owner, &first, None, "2026/Q/1").await;
    let (status, problem) = app
        .send(
            "PATCH",
            &format!("/api/v1/utility-bills/{bill}"),
            Some(&owner),
            Some(serde_json::json!({ "lease_id": lease_on_second })),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "il controllo va rifatto a ogni correzione: {problem}"
    );
}
