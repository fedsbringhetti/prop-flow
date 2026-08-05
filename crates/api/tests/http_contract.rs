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
