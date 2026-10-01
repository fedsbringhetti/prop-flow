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

/// Le sonde rispondono uguali alla radice e sotto `/api/v1` (`R50`).
///
/// La radice è per l'`healthcheck` del container, che parla all'API senza proxy;
/// il prefisso è l'unica forma che attraversa il reverse proxy, e quella che il
/// runbook del primo deploy usa per dichiararsi riuscito. Senza la seconda,
/// `…/api/v1/ready` cadrebbe nel `fallback` con `404 endpoint.not_found` — e il
/// confronto sul corpo lo vedrebbe. Il `200` si afferma a parte: due `404`
/// uguali sarebbero «uguali» anche loro.
#[tokio::test]
async fn the_probes_answer_the_same_at_the_root_and_under_the_prefix() {
    let app = require_app!();
    for probe in ["health", "ready"] {
        let (root_status, root_body) = app.send("GET", &format!("/{probe}"), None, None).await;
        let (prefixed_status, prefixed_body) = app
            .send("GET", &format!("/api/v1/{probe}"), None, None)
            .await;
        assert_eq!(root_status, StatusCode::OK, "`/{probe}`: {root_body}");
        assert_eq!(
            prefixed_status, root_status,
            "`/api/v1/{probe}` non risponde come `/{probe}`: {prefixed_body}"
        );
        assert_eq!(prefixed_body, root_body, "su `{probe}`");
    }
}

/// `F6` ha chiuso `POST /auth/register`: la validazione della password alla
/// registrazione si prova adesso sulle due rotte da invito (ADR-053), che sono
/// le sole rimaste. `password_policy::validate` è condivisa da entrambe —
/// `create_owner_account`, dietro `accept-owner-invitation`, e
/// `RegisterTenantFromInvitation` — quindi un `422` mancante su una delle due
/// sarebbe una regressione della condivisione, non solo del test.
#[tokio::test]
async fn weak_passwords_are_rejected_with_field_errors() {
    let app = require_app!();

    let owner_email = common::unique_email("debole-proprietario");
    let owner_token = common::issue_owner_invitation(&app, &owner_email).await;
    let (status, body) = app
        .post(
            "/api/v1/auth/accept-owner-invitation",
            serde_json::json!({
                "token": owner_token,
                "full_name": "Tizio",
                "password": "corta",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {body}");
    assert_eq!(body["code"], "validation.failed", "corpo: {body}");
    assert_eq!(body["errors"][0]["field"], "password", "corpo: {body}");

    let (owner, _) = common::register_and_login(&app, "debole-inquilino").await;
    let property = create_property(&app, &owner, "Immobile per password debole").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;
    let tenant_email = common::unique_email("debole-inquilino-invitato");
    let (_, tenant_invite_token) = invite_tenant(&app, &owner, &lease, &tenant_email).await;
    let (status, body) = app
        .post(
            "/api/v1/auth/register-from-invitation",
            serde_json::json!({
                "token": tenant_invite_token,
                "full_name": "Caio",
                "password": "corta",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {body}");
    assert_eq!(body["code"], "validation.failed", "corpo: {body}");
    assert_eq!(body["errors"][0]["field"], "password", "corpo: {body}");
}

/// Sulle rotte da invito l'email **non è un campo**: è quella dell'invito
/// (ADR-053, decisione 6), quindi un indirizzo malformato non può più
/// arrivarci — non c'è dove scriverlo. La validazione del formato vive adesso
/// dove un invito nasce, `POST /leases/{id}/invitations`, che prima non aveva
/// un test di contratto dedicato.
#[tokio::test]
async fn malformed_email_is_rejected_where_an_invitation_is_issued() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "email-malformata").await;
    let property = create_property(&app, &owner, "Immobile per email malformata").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    let (status, body) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/invitations"),
            &owner,
            serde_json::json!({
                "email": "non-una-email",
                "full_name": "Tizio",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {body}");
    assert_eq!(body["errors"][0]["field"], "email", "corpo: {body}");
}

/// Nessun test di F4 provava già questo caso sulle rotte nuove: l'oracolo
/// dell'email esiste ed è voluto (ADR-053, «l'oracolo delle email esiste
/// già»), ma resta uno solo — `auth.email_taken` — anche quando ad accettare
/// due inviti da proprietario distinti è la stessa persona.
#[tokio::test]
async fn accepting_two_owner_invitations_for_the_same_email_is_a_conflict() {
    let app = require_app!();
    let email = common::unique_email("proprietario-duplicato");
    let full_name = "Proprietario Duplicato";
    let password = "cavallo batteria graffetta";

    let first_token = common::issue_owner_invitation(&app, &email).await;
    let (first, body) = app
        .post(
            "/api/v1/auth/accept-owner-invitation",
            serde_json::json!({ "token": first_token, "full_name": full_name, "password": password }),
        )
        .await;
    assert_eq!(first, StatusCode::CREATED, "corpo: {body}");

    // L'indice unico su `owner_invitations` è parziale sui soli inviti
    // pendenti: il primo è ormai accettato, quindi un secondo invito per lo
    // stesso indirizzo si emette senza sbattere contro di lui — è il
    // `409 auth.email_taken` dell'account, non un conflitto sull'invito.
    let second_token = common::issue_owner_invitation(&app, &email).await;
    let (second, body) = app
        .post(
            "/api/v1/auth/accept-owner-invitation",
            serde_json::json!({ "token": second_token, "full_name": full_name, "password": password }),
        )
        .await;
    assert_eq!(second, StatusCode::CONFLICT, "corpo: {body}");
    assert_eq!(body["code"], "auth.email_taken", "corpo: {body}");
}

#[tokio::test]
async fn wrong_credentials_are_indistinguishable_from_an_unknown_account() {
    let app = require_app!();
    let email = common::unique_email("credenziali");
    let invitation_token = common::issue_owner_invitation(&app, &email).await;
    app.post(
        "/api/v1/auth/accept-owner-invitation",
        serde_json::json!({
            "token": invitation_token,
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
    let invitation_token = common::issue_owner_invitation(&app, &email).await;
    app.post(
        "/api/v1/auth/accept-owner-invitation",
        serde_json::json!({ "token": invitation_token, "full_name": "Tizio", "password": password }),
    )
    .await;
    let (_, session, first_refresh) = app
        .post_returning_cookie(
            "/api/v1/auth/login",
            serde_json::json!({ "email": email, "password": password }),
        )
        .await;
    // Il refresh token esce **solo** dal cookie: se ricomparisse nel corpo, il
    // cookie `httpOnly` non proteggerebbe più niente, perché un XSS lo leggerebbe
    // dalla risposta del login.
    assert!(
        session.get("refresh_token").is_none(),
        "il refresh token non deve comparire nel corpo: {session}"
    );
    let first_refresh = first_refresh.expect("il login deve posare il cookie di sessione");

    let (status, rotated, second_refresh) = app
        .post_with_cookie("/api/v1/auth/refresh", Some(&first_refresh), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {rotated}");
    let second_refresh = second_refresh.expect("il refresh deve posare il cookie nuovo");
    assert_ne!(
        second_refresh, first_refresh,
        "il token non è stato ruotato"
    );

    // Riuso del token già consumato: rifiutato, e la catena viene revocata.
    let (status, body, _) = app
        .post_with_cookie("/api/v1/auth/refresh", Some(&first_refresh), None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "corpo: {body}");

    // Anche il successore cade, perché non si sa quale delle due parti sia
    // legittima.
    let (status, body, _) = app
        .post_with_cookie("/api/v1/auth/refresh", Some(&second_refresh), None)
        .await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "il rilevamento del riuso deve revocare tutte le sessioni, corpo: {body}"
    );
}

/// Senza cookie il refresh è `401`, con lo **stesso codice** di un cookie
/// rifiutato.
///
/// Distinguerli direbbe a chi ha rubato un token se è ancora vivo, ed è la stessa
/// ragione per cui il refresh non separa «scaduto» da «riusato» (ADR-022).
#[tokio::test]
async fn refresh_without_the_cookie_is_indistinguishable_from_a_bad_one() {
    let app = require_app!();

    let (missing_status, missing_body, _) = app
        .post_with_cookie("/api/v1/auth/refresh", None, None)
        .await;
    let (bad_status, bad_body, _) = app
        .post_with_cookie(
            "/api/v1/auth/refresh",
            Some("un-token-che-non-esiste"),
            None,
        )
        .await;

    assert_eq!(
        missing_status,
        StatusCode::UNAUTHORIZED,
        "corpo: {missing_body}"
    );
    assert_eq!(bad_status, StatusCode::UNAUTHORIZED, "corpo: {bad_body}");
    assert_eq!(
        missing_body["code"], bad_body["code"],
        "cookie assente e cookie rifiutato devono essere indistinguibili"
    );
}

/// **Il test che tiene in piedi l'argomento CSRF.**
///
/// Nessuna difesa CSRF è montata, e la conclusione regge su una condizione
/// precisa: che *nessuna rotta di business sia autenticata da un cookie*. Ogni
/// rotta di business chiede `Authorization`, che un sito attaccante non può
/// impostare cross-origin senza il permesso di CORS — che non concediamo, perché
/// CORS non è montato affatto — e il cookie del refresh è `SameSite=Strict` su
/// `Path=/api/v1/auth`.
///
/// Il giorno in cui qualcuno aggiungesse una sessione a cookie per comodità,
/// quell'argomento cadrebbe **in silenzio**: nessun altro test fallirebbe. Questo
/// sì. Non verifica una funzionalità: verifica che una porta resti chiusa.
#[tokio::test]
async fn the_session_cookie_is_refused_by_business_routes() {
    let app = require_app!();
    let email = common::unique_email("csrf");
    let password = "cavallo batteria graffetta";
    let invitation_token = common::issue_owner_invitation(&app, &email).await;
    app.post(
        "/api/v1/auth/accept-owner-invitation",
        serde_json::json!({ "token": invitation_token, "full_name": "Tizio", "password": password }),
    )
    .await;
    let (_, _, cookie) = app
        .post_returning_cookie(
            "/api/v1/auth/login",
            serde_json::json!({ "email": email, "password": password }),
        )
        .await;
    let cookie = cookie.expect("il login deve posare il cookie di sessione");

    // Un cookie **valido**, presentato su rotte di business senza `Authorization`.
    // Sono lettura, scrittura e la proiezione che attraversa tutti i moduli: se
    // una sola di queste accettasse il cookie, l'argomento sarebbe già caduto.
    for (method, path) in [
        ("GET", "/api/v1/dashboard"),
        ("GET", "/api/v1/properties"),
        ("GET", "/api/v1/leases"),
        ("GET", "/api/v1/charges"),
        (
            "DELETE",
            "/api/v1/properties/019fce39-0e43-7011-84bf-1d03f6764daf",
        ),
    ] {
        let (status, body) = app.send_with_cookie_only(method, path, &cookie).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "{method} {path} ha accettato il solo cookie: l'argomento CSRF è caduto. Corpo: {body}"
        );
        assert_eq!(
            body["code"], "auth.token_missing",
            "{method} {path}: il rifiuto deve essere per assenza di Authorization, corpo: {body}"
        );
    }
}

/// L'uscita cancella il cookie, e lo fa anche quando non c'era.
#[tokio::test]
async fn logout_clears_the_cookie_and_is_idempotent() {
    let app = require_app!();
    let email = common::unique_email("uscita");
    let password = "cavallo batteria graffetta";
    let invitation_token = common::issue_owner_invitation(&app, &email).await;
    app.post(
        "/api/v1/auth/accept-owner-invitation",
        serde_json::json!({ "token": invitation_token, "full_name": "Tizio", "password": password }),
    )
    .await;
    let (_, session, cookie) = app
        .post_returning_cookie(
            "/api/v1/auth/login",
            serde_json::json!({ "email": email, "password": password }),
        )
        .await;
    let token = session["access_token"].as_str().expect("access token");
    let cookie = cookie.expect("cookie di sessione");

    let (status, _, cleared) = app
        .post_with_cookie("/api/v1/auth/logout", Some(&cookie), Some(token))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(cleared, None, "il logout deve cancellare il cookie");

    // La sessione è davvero caduta: il token revocato non rinnova più.
    let (status, body, _) = app
        .post_with_cookie("/api/v1/auth/refresh", Some(&cookie), None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "corpo: {body}");

    // Un secondo logout, ormai senza cookie, resta `204`: non c'è nulla da
    // revocare, e costringere il client a distinguere i due casi non lo aiuta.
    let (status, _, _) = app
        .post_with_cookie("/api/v1/auth/logout", None, Some(token))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
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

/// Crea un immobile **disponibile** e restituisce il suo id.
///
/// Disponibile e non in bozza perché la gran parte di chi lo chiama ci attiva poi
/// un contratto, e dalla sezione 12 un immobile in bozza lo rifiuta
/// (`property.not_lettable`, ADR-015). Che il default resti `draft` lo verifica
/// `full_property_lifecycle`, che crea senza passare da qui.
async fn create_property(app: &TestApp, token: &str, label: &str) -> String {
    let (status, created) = app
        .post_auth(
            "/api/v1/properties",
            token,
            serde_json::json!({
                "label": label,
                "property_type": "apartment",
                "status": "available",
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

/// Lo stato si può scegliere alla creazione, e il **default non si sposta**.
///
/// `R44`: il form non offriva lo stato, quindi si atterrava sempre in bozza, e lo
/// stato si cambiava solo dal dialogo «Modifica» della scheda — che non è dove si
/// va a cercarlo. Il campo è opzionale: chi non lo dice ottiene `draft` come
/// prima, perché spostare il default nel dominio deciderebbe al posto di chi crea
/// senza guardare.
#[tokio::test]
async fn a_property_can_be_born_available_and_stays_a_draft_when_nobody_says_otherwise() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "stato-alla-creazione").await;

    let create = |body: serde_json::Value| app.post_auth("/api/v1/properties", &token, body);

    let (status, implicit) = create(serde_json::json!({
        "label": "Senza stato",
        "property_type": "apartment",
        "address": { "street": "Via Roma 1", "city": "Milano", "province": "MI" }
    }))
    .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {implicit}");
    assert_eq!(
        implicit["status"], "draft",
        "il default del dominio non si sposta"
    );

    let (status, available) = create(serde_json::json!({
        "label": "Disponibile da subito",
        "property_type": "apartment",
        "status": "available",
        "address": { "street": "Via Roma 2", "city": "Milano", "province": "MI" }
    }))
    .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {available}");
    assert_eq!(available["status"], "available");

    // `archived` lo rifiuta `set_status`, che è lo stesso controllo del `PATCH`:
    // archiviare ha l'operazione sua, e passa da `DELETE`.
    let (status, problem) = create(serde_json::json!({
        "label": "Nato archiviato",
        "property_type": "apartment",
        "status": "archived",
        "address": { "street": "Via Roma 3", "city": "Milano", "province": "MI" }
    }))
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(
        problem["code"],
        "property.archive_requires_dedicated_operation"
    );
}

/// **In bozza non si affitta**, e il rifiuto è all'attivazione.
///
/// `R44`: un immobile nasce in bozza e ci resta, e fino alla sezione 12 nessuna
/// regola lo guardava — si poteva attivare un contratto su un immobile che nessuno
/// aveva mai dichiarato disponibile, e il doppio badge mostrava «Affittato (Bozza)»
/// per sempre. Il rifiuto sta su `ActivateLease` e non su `DraftLease`: una bozza
/// di contratto su una bozza di immobile sono due cose che si preparano insieme
/// (ADR-015).
#[tokio::test]
async fn a_draft_property_refuses_to_be_let_until_someone_declares_it_available() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "bozza-non-si-affitta").await;

    // Creato senza passare dall'helper: qui la bozza è la premessa, non un caso.
    let (status, created) = app
        .post_auth(
            "/api/v1/properties",
            &token,
            serde_json::json!({
                "label": "Immobile mai dichiarato",
                "property_type": "apartment",
                "address": { "street": "Via Roma 1", "city": "Milano", "province": "MI" }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {created}");
    assert_eq!(created["status"], "draft", "il default non cambia");
    let property = created["id"].as_str().expect("id").to_owned();

    // La bozza di contratto resta possibile: è l'ordine di compilazione che non si
    // vuole imporre.
    let lease = draft_lease(&app, &token, &property, None, 0, Some(365)).await;
    add_tenant(&app, &token, &lease, "Inquilina Pronta", None).await;

    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "property.not_lettable");

    // Dichiarato disponibile, la stessa chiamata passa: è l'unica cosa che
    // mancava.
    let (status, updated) = app
        .send(
            "PATCH",
            &format!("/api/v1/properties/{property}"),
            Some(&token),
            Some(serde_json::json!({ "status": "available" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {updated}");

    let (status, activated) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {activated}");
    assert_eq!(activated["status"], "active");
}

/// **Con un contratto in essere non si torna in bozza**, e `maintenance` sì.
///
/// `R44`, la porta di servizio: la regola dell'attivazione non basta da sola. Se
/// un `PATCH` accettasse `status: draft` su un immobile affittato, «Affittato
/// (Bozza)» rientrerebbe dall'altra parte. `maintenance` resta ammesso di
/// proposito: «In manutenzione (Affittato)» è un'anomalia vera, che il doppio
/// badge della 5-ter esiste per mostrare.
#[tokio::test]
async fn a_property_with_a_lease_in_force_cannot_go_back_to_draft_but_can_go_to_maintenance() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "porta-di-servizio").await;
    let property = create_property(&app, &token, "Immobile affittato").await;

    let lease = draft_lease(&app, &token, &property, None, 0, Some(365)).await;
    add_tenant(&app, &token, &lease, "Inquilina Presente", None).await;
    let (status, activated) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {activated}");

    let (status, problem) = app
        .send(
            "PATCH",
            &format!("/api/v1/properties/{property}"),
            Some(&token),
            Some(serde_json::json!({ "status": "draft" })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "property.lease_in_force");

    // `maintenance` passa: la domanda si fa **solo** quando si chiede la bozza.
    let (status, updated) = app
        .send(
            "PATCH",
            &format!("/api/v1/properties/{property}"),
            Some(&token),
            Some(serde_json::json!({ "status": "maintenance" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {updated}");
    assert_eq!(updated["status"], "maintenance");

    // E un `PATCH` che non tocca lo stato nemmeno: non c'è nessuna domanda da
    // fare, e farla renderebbe immodificabile un immobile affittato.
    let (status, renamed) = app
        .send(
            "PATCH",
            &format!("/api/v1/properties/{property}"),
            Some(&token),
            Some(serde_json::json!({ "label": "Immobile affittato, rinominato" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {renamed}");
    assert_eq!(renamed["label"], "Immobile affittato, rinominato");
}

/// **Con un contratto in essere non si archivia**, e disdire è la via d'uscita.
///
/// `R45`: archiviare è un soft delete, e prima della sezione 12 portava via
/// l'immobile lasciando il contratto attaccato a una riga che le risposte chiamano
/// «Immobile archiviato» — con gli addebiti che continuavano a maturare e
/// l'inquilino che non vedeva nulla di strano. Si rifiuta, **non si disdice**: un
/// atto con una data e delle conseguenze non si nasconde dentro un'archiviazione.
#[tokio::test]
async fn a_property_with_a_lease_in_force_is_archived_only_after_the_lease_is_terminated() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "archivia-affittato").await;
    let property = create_property(&app, &token, "Immobile da archiviare").await;

    let lease = draft_lease(&app, &token, &property, None, 0, Some(365)).await;
    add_tenant(&app, &token, &lease, "Inquilino Residente", None).await;
    let (status, activated) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {activated}");

    let (status, problem) = app
        .send(
            "DELETE",
            &format!("/api/v1/properties/{property}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "property.lease_in_force");

    // L'immobile è ancora là: il rifiuto non ha archiviato a metà.
    let (status, still_there) = app
        .send(
            "GET",
            &format!("/api/v1/properties/{property}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {still_there}");

    // E il contratto **non** è stato disdetto dall'archiviazione rifiutata.
    let (status, lease_now) = app
        .send(
            "GET",
            &format!("/api/v1/leases/{lease}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {lease_now}");
    assert_eq!(
        lease_now["status"], "active",
        "archiviare non disdice, nemmeno quando fallisce"
    );

    // Disdetto il contratto, la stessa chiamata archivia.
    let (status, terminated) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/terminate"),
            &token,
            serde_json::json!({ "reason": "Fine del rapporto" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {terminated}");

    let (status, body) = app
        .send(
            "DELETE",
            &format!("/api/v1/properties/{property}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "corpo: {body}");

    // E fuori dall'elenco, che è ciò che archiviare significa.
    let (status, page) = app
        .send("GET", "/api/v1/properties", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let visible: Vec<&str> = page["items"]
        .as_array()
        .expect("elenco")
        .iter()
        .filter_map(|item| item["id"].as_str())
        .collect();
    assert!(!visible.contains(&property.as_str()), "corpo: {page}");
}

/// L'immobile si legge **prima** delle parti, e il messaggio è la ragione.
///
/// Un contratto senza inquilino su un immobile in bozza è sbagliato due volte. Se
/// rispondesse `lease.without_tenant_party`, chi lo legge aggiungerebbe una parte
/// per poi ricevere `property.not_lettable` al secondo tentativo: due giri per
/// arrivare alla cosa da fare. Per il database l'ordine non conta — è tutto in una
/// transazione — per chi legge la risposta sì.
#[tokio::test]
async fn a_draft_property_is_refused_before_the_parties_are_even_looked_at() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "ordine-dei-rifiuti").await;

    let (status, created) = app
        .post_auth(
            "/api/v1/properties",
            &token,
            serde_json::json!({
                "label": "Immobile in bozza",
                "property_type": "apartment",
                "address": { "street": "Via Roma 2", "city": "Milano", "province": "MI" }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {created}");
    let property = created["id"].as_str().expect("id").to_owned();

    // Nessuna parte: senza la lettura dell'immobile il rifiuto sarebbe quello
    // dell'inquilino mancante.
    let lease = draft_lease(&app, &token, &property, None, 0, Some(365)).await;

    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(
        problem["code"], "property.not_lettable",
        "l'immobile va letto prima delle parti, o il rifiuto manda a sistemare \
         la cosa sbagliata"
    );
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

/// **Annullare una bozza**, e le due metà che rendono utile l'operazione.
///
/// La transizione esisteva nel dominio da sempre e non la esponeva nessuna rotta:
/// un contratto redatto per sbaglio restava lì, perché l'unica uscita sarebbe
/// stata attivarlo e disdirlo — che è ciò che non riesce quando le quote sono
/// sbagliate.
///
/// Il test guarda **tre** cose, e la seconda è quella che una cancellazione soft
/// avrebbe superato in silenzio fallendo lo scopo:
///
/// 1. la bozza diventa `cancelled`;
/// 2. **resta leggibile**, e il filtro per quello stato la trova. Con `deleted_at`
///    sarebbe sparita da ogni elenco, e il filtro sarebbe rimasto quello che era:
///    un'opzione che non può corrispondere a niente;
/// 3. il periodo **torna libero**, perché i vincoli di esclusione della `0005`
///    coprono `pending_signature`, `active` ed `expiring`, e `cancelled` non è
///    fra questi.
#[tokio::test]
async fn a_cancelled_draft_stays_readable_and_frees_its_period() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "annullamento").await;
    let property = create_property(&app, &token, "Immobile ripensato").await;

    let wrong = draft_lease(&app, &token, &property, None, 0, Some(365)).await;
    let (status, cancelled) = app
        .post_auth(
            &format!("/api/v1/leases/{wrong}/cancel"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {cancelled}");
    assert_eq!(cancelled["status"], "cancelled");

    // Resta negli elenchi: è la metà che distingue «annullare» da «cancellare».
    let (status, page) = app
        .send(
            "GET",
            &format!("/api/v1/leases?property_id={property}&status=cancelled"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let items = page["items"].as_array().expect("elenco");
    assert_eq!(items.len(), 1, "corpo: {page}");
    assert_eq!(items[0]["id"], serde_json::json!(wrong));

    // Annullarlo due volte non è ammesso.
    let (status, _) = app
        .post_auth(
            &format!("/api/v1/leases/{wrong}/cancel"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Lo stesso periodo si può rifare, ed è il motivo per cui si annulla.
    let good = draft_lease(&app, &token, &property, None, 0, Some(365)).await;
    add_tenant(&app, &token, &good, "Inquilino Vero", None).await;
    let (status, activated) = app
        .post_auth(
            &format!("/api/v1/leases/{good}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {activated}");

    // E un contratto in vigore non si annulla: per quello c'è la disdetta, che
    // registra un fatto invece di dire che non è mai successo niente.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/leases/{good}/cancel"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "lease.invalid_transition");
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

/// **Il vicolo cieco, percorso da capo a fondo.**
///
/// Era questo: una parte inserita con la quota sbagliata non si correggeva, quindi
/// il contratto non si attivava — le quote degli inquilini devono sommare a 100 —
/// e non si poteva nemmeno rifare, perché annullarlo non era esposto. Il contratto
/// restava lì per sempre, non attivabile e non rimovibile, e l'unico rimedio era
/// crearne un altro lasciando in elenco quello rotto.
///
/// Il test cammina l'uscita: si sbaglia, si toglie ciò che è di troppo, si
/// corregge, si attiva.
#[tokio::test]
async fn a_wrong_share_can_be_corrected_and_the_lease_activates() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "correzione").await;
    let property = create_property(&app, &token, "Immobile con le quote sbagliate").await;
    let lease = draft_lease(&app, &token, &property, None, 0, Some(365)).await;

    let (status, first) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/parties"),
            &token,
            serde_json::json!({
                "full_name": "Inquilino Uno",
                "share_percent": "30",
                "is_primary": true,
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {first}");
    let first_party = first["id"].as_str().expect("id").to_owned();

    // Il muro: la somma non fa 100, e prima non c'era modo di uscirne.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "lease.shares_not_hundred");

    // Una parte aggiunta per sbaglio, che va tolta e non corretta.
    let (status, second) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/parties"),
            &token,
            serde_json::json!({ "full_name": "Aggiunta Per Sbaglio", "share_percent": "70" }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {second}");
    let second_party = second["id"].as_str().expect("id").to_owned();

    // Un contratto ha un solo intestatario principale, e il vincolo parziale lo
    // dice con il suo codice invece che come "vincolo violato".
    let (status, problem) = app
        .send(
            "PATCH",
            &format!("/api/v1/lease-parties/{second_party}"),
            Some(&token),
            Some(serde_json::json!({ "is_primary": true })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "lease.primary_party_exists");

    let (status, _) = app
        .send(
            "DELETE",
            &format!("/api/v1/lease-parties/{second_party}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Tolta davvero: l'elenco delle parti torna a una.
    let (_, detail) = app
        .send(
            "GET",
            &format!("/api/v1/leases/{lease}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(
        detail["parties"].as_array().map(Vec::len),
        Some(1),
        "corpo: {detail}"
    );

    // La correzione, che è l'operazione che mancava.
    let (status, corrected) = app
        .send(
            "PATCH",
            &format!("/api/v1/lease-parties/{first_party}"),
            Some(&token),
            Some(serde_json::json!({ "share_percent": "100", "phone": "+39 011 0000000" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {corrected}");
    assert_eq!(corrected["share_percent"], "100.00");
    assert_eq!(corrected["phone"], "+39 011 0000000");

    let (status, activated) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {activated}");
    assert_eq!(activated["status"], "active");
}

/// **Le due regole che impediscono di rifare il danno.** Vedi ADR-046.
///
/// La prima: `100 + 0` fa 100, e passava. Era il caso osservato in database — il
/// 100% su una parte che non aveva mai accettato, e l'inquilino vero a zero — e la
/// somma non ha nulla da eccepire, perché il difetto non è nella somma.
///
/// La seconda: aggiungere una parte a un contratto **attivo** era ammesso, e da lì
/// le quote smettevano di sommare a 100 senza che niente protestasse. Il client
/// offriva il bottone solo in bozza, ma quella è una scelta di che cosa offrire.
#[tokio::test]
async fn a_zero_share_blocks_activation_and_an_active_lease_takes_no_new_parties() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "quota-zero").await;
    let property = create_property(&app, &token, "Immobile in coabitazione").await;
    let lease = draft_lease(&app, &token, &property, None, 0, Some(365)).await;

    // Il concorso che produceva il danno: il primo si prende tutto, al secondo
    // resta zero, e la somma torna.
    let (status, first) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/parties"),
            &token,
            serde_json::json!({
                "full_name": "Primo Inquilino",
                "share_percent": "100",
                "is_primary": true,
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {first}");
    let first_party = first["id"].as_str().expect("id").to_owned();

    let (status, second) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/parties"),
            &token,
            serde_json::json!({ "full_name": "Secondo Inquilino", "share_percent": "0" }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {second}");

    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "lease.tenant_without_share");

    // Corrette le due quote, l'attivazione passa.
    let (status, _) = app
        .send(
            "PATCH",
            &format!("/api/v1/lease-parties/{first_party}"),
            Some(&token),
            Some(serde_json::json!({ "share_percent": "50" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let second_party = second["id"].as_str().expect("id").to_owned();
    let (status, _) = app
        .send(
            "PATCH",
            &format!("/api/v1/lease-parties/{second_party}"),
            Some(&token),
            Some(serde_json::json!({ "share_percent": "50" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, activated) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {activated}");

    // E da qui non si aggiungono parti: la somma non verrebbe più riverificata.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/parties"),
            &token,
            serde_json::json!({ "full_name": "Terzo Inquilino", "share_percent": "10" }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "lease.parties_locked");
}

/// Un contratto attivo da dieci giorni, con due parti al 50%: Anna, intestataria e
/// collegata a un account, e Marco, senza account. Restituisce l'id del contratto,
/// della parte di Anna, di quella di Marco, il token di Anna e l'email di Marco.
async fn active_lease_of_two(
    app: &TestApp,
    owner: &str,
    label: &str,
) -> (String, String, String, String, String) {
    let property = create_property(app, owner, label).await;
    let lease = draft_lease(app, owner, &property, None, -10, Some(355)).await;

    // L'invito su una bozza crea la parte: al 100%, e intestataria perché è la prima.
    let anna_email = common::unique_email("subentro-anna");
    let (_, accept_token) = invite_tenant(app, owner, &lease, &anna_email).await;
    let (anna, _) = common::register_tenant_from_invitation(
        app,
        &accept_token,
        "Anna Che Esce",
        "cavallo batteria graffetta",
    )
    .await;
    let (_, detail) = app
        .send("GET", &format!("/api/v1/leases/{lease}"), Some(owner), None)
        .await;
    let anna_party = detail["parties"][0]["id"].as_str().expect("id").to_owned();
    let (status, _) = app
        .send(
            "PATCH",
            &format!("/api/v1/lease-parties/{anna_party}"),
            Some(owner),
            Some(serde_json::json!({ "share_percent": "50" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let marco_email = common::unique_email("subentro-marco");
    let (status, marco) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/parties"),
            owner,
            serde_json::json!({
                "full_name": "Marco Che Resta",
                "email": marco_email,
                "role": "cotenant",
                "share_percent": "50",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {marco}");
    let marco_party = marco["id"].as_str().expect("id").to_owned();

    let (status, activated) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {activated}");
    (lease, anna_party, marco_party, anna, marco_email)
}

fn party_in<'a>(parties: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    parties
        .as_array()
        .expect("elenco delle parti")
        .iter()
        .find(|party| party["id"] == id)
        .expect("la parte c'è")
}

/// **Il subentro fa le due cose che contano**, e le fa vedere dal bordo (ADR-056).
///
/// Chi esce **smette di essere residente**: dopo il subentro può uscire
/// dall'organizzazione, prima riceveva `409 membership.lease_in_force` — è
/// `app.has_lease_in_force` che filtra `left_at`. **Non smette di essere
/// debitore**: finché è membro continua a leggere il contratto, perché
/// `app.can_access_lease` non guarda `left_at`, ed è voluto. Chi entra nasce come
/// segnaposto in attesa, con la quota e l'intestazione di chi esce, e accettando
/// l'invito **diventa lui** quello che non può uscire.
#[tokio::test]
async fn a_handover_lets_the_leaving_tenant_out_and_puts_the_joining_one_in_its_place() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "subentro-proprietario").await;
    let (lease, anna_party, marco_party, anna, _) =
        active_lease_of_two(&app, &owner, "Casa del subentro").await;

    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/organizations/{owner_org}/leave"),
            &anna,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "membership.lease_in_force");

    let giulia_email = common::unique_email("subentro-giulia");
    let body = serde_json::json!({
        "leaving_party_id": anna_party,
        "joining": { "full_name": "Giulia Che Entra", "email": giulia_email, "share_percent": "50" },
    });

    // Il subentro è di chi amministra: chi abita non fa uscire nessuno. Il codice e
    // non solo lo stato, perché senza `ensure_staff` il `403` arriverebbe lo stesso
    // dalla RLS (`auth.denied_by_policy`) — provato al contrario — e il test non
    // saprebbe distinguere il controllo che c'è da quello che manca.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/handover"),
            &anna,
            body.clone(),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {problem}");
    assert_eq!(problem["code"], "org.staff_role_required");

    let (status, handed) = app
        .post_auth(&format!("/api/v1/leases/{lease}/handover"), &owner, body)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {handed}");

    let parties = &handed["lease"]["parties"];
    let left = party_in(parties, &anna_party);
    assert_eq!(left["left_at"], day(0), "esce oggi: {left}");
    assert_eq!(left["is_primary"], false, "chi esce non intesta più niente");
    let joined = parties
        .as_array()
        .expect("parti")
        .iter()
        .find(|party| party["email"] == giulia_email.as_str())
        .expect("la parte di chi entra");
    assert_eq!(
        joined["is_primary"], true,
        "l'intestazione passa a chi entra"
    );
    assert_eq!(joined["share_percent"], "50.00");
    assert_eq!(joined["role"], "tenant", "il ruolo di chi esce");
    assert!(
        joined.get("user_id").is_none(),
        "un segnaposto in attesa: {joined}"
    );
    assert_eq!(party_in(parties, &marco_party)["share_percent"], "50.00");
    assert_eq!(handed["invitation"]["email"], giulia_email.as_str());
    let giulia_token = handed["invitation"]["accept_token"]
        .as_str()
        .expect("token dell'invito")
        .to_owned();

    // Anna continua a leggere il contratto: ha degli addebiti da pagare.
    let (status, seen) = app
        .send("GET", &format!("/api/v1/leases/{lease}"), Some(&anna), None)
        .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "chi esce continua a leggere: {seen}"
    );

    // E ora può uscire dall'organizzazione.
    let (status, body) = app
        .post_auth(
            &format!("/api/v1/organizations/{owner_org}/leave"),
            &anna,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {body}");

    // Giulia accetta: l'invito trova la sua riga, e da qui è lei ad avere un
    // contratto in essere.
    let (giulia, giulia_id) = common::register_tenant_from_invitation(
        &app,
        &giulia_token,
        "Giulia Che Entra",
        "lampada fiume cartolina",
    )
    .await;
    let (_, detail) = app
        .send(
            "GET",
            &format!("/api/v1/leases/{lease}"),
            Some(&owner),
            None,
        )
        .await;
    let joined_id = joined["id"].as_str().expect("id");
    assert_eq!(
        party_in(&detail["parties"], joined_id)["user_id"],
        giulia_id.as_str()
    );
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/organizations/{owner_org}/leave"),
            &giulia,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "membership.lease_in_force");
}

/// **Un rifiuto non lascia niente a metà**: né l'uscita, né la parte nuova, né le
/// quote. È la ragione per cui il subentro è una richiesta sola e non una sequenza
/// di `PATCH`.
#[tokio::test]
async fn a_handover_that_does_not_add_up_is_refused_and_writes_nothing() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "subentro-rifiuti").await;
    let (lease, anna_party, marco_party, _, marco_email) =
        active_lease_of_two(&app, &owner, "Casa dei subentri rifiutati").await;
    let url = format!("/api/v1/leases/{lease}/handover");
    let giulia_email = common::unique_email("subentro-rifiuti-giulia");
    let handover = |leaving: &str, email: &str, joining: &str, shares: serde_json::Value| {
        serde_json::json!({
            "leaving_party_id": leaving,
            "joining": { "full_name": "Giulia Che Entra", "email": email, "share_percent": joining },
            "shares": shares,
        })
    };

    // Anna esce con il suo 50, e 50 + 40 non fa 100.
    let marco_40 = serde_json::json!([{ "party_id": marco_party, "share_percent": "40" }]);
    let (status, problem) = app
        .post_auth(
            &url,
            &owner,
            handover(&anna_party, &giulia_email, "50", marco_40.clone()),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "lease.shares_not_hundred");

    // Un'email che è già di una parte: l'accettazione troverebbe quella riga.
    let (status, problem) = app
        .post_auth(
            &url,
            &owner,
            handover(&anna_party, &marco_email, "50", serde_json::json!([])),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "lease.party_email_taken");

    // Una quota su chi esce non è una quota di chi resta.
    let on_anna = serde_json::json!([{ "party_id": anna_party, "share_percent": "50" }]);
    let (status, problem) = app
        .post_auth(
            &url,
            &owner,
            handover(&anna_party, &giulia_email, "50", on_anna),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");

    let stranger = uuid::Uuid::now_v7().to_string();
    let (status, problem) = app
        .post_auth(
            &url,
            &owner,
            handover(&stranger, &giulia_email, "50", serde_json::json!([])),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {problem}");
    assert_eq!(problem["code"], "lease.party_not_found");

    // Un invito già pendente per quell'email, su un altro contratto della stessa
    // organizzazione, annulla **tutto**: anche l'uscita di Anna.
    let other_property = create_property(&app, &owner, "Casa dell'invito pendente").await;
    let other = draft_lease(&app, &owner, &other_property, None, 0, Some(365)).await;
    let pending_email = common::unique_email("subentro-pendente");
    invite_tenant(&app, &owner, &other, &pending_email).await;
    let (status, problem) = app
        .post_auth(
            &url,
            &owner,
            handover(&anna_party, &pending_email, "50", serde_json::json!([])),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "invitation.already_pending");

    let (_, detail) = app
        .send(
            "GET",
            &format!("/api/v1/leases/{lease}"),
            Some(&owner),
            None,
        )
        .await;
    let parties = detail["parties"].as_array().expect("parti");
    assert_eq!(parties.len(), 2, "nessuna parte nuova: {detail}");
    let anna = party_in(&detail["parties"], &anna_party);
    assert_eq!(
        anna["left_at"],
        serde_json::Value::Null,
        "Anna è ancora dentro"
    );
    assert_eq!(anna["is_primary"], true);
    assert_eq!(
        party_in(&detail["parties"], &marco_party)["share_percent"],
        "50.00"
    );

    // Con le quote giuste passa, e le quote di chi resta cambiano.
    let marco_40 = serde_json::json!([{ "party_id": marco_party, "share_percent": "40" }]);
    let (status, handed) = app
        .post_auth(
            &url,
            &owner,
            handover(&anna_party, &giulia_email, "60", marco_40),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {handed}");
    assert_eq!(
        party_in(&handed["lease"]["parties"], &marco_party)["share_percent"],
        "40.00"
    );

    // E Anna non esce due volte.
    let (status, problem) = app
        .post_auth(
            &url,
            &owner,
            handover(
                &anna_party,
                &common::unique_email("subentro-luca"),
                "60",
                serde_json::json!([]),
            ),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "lease.party_already_left");
}

/// Il subentro esce da un rapporto **in corso**: in bozza si aggiunge una parte, e
/// un contratto firmato che comincia fra un mese non ha ancora nessuno che esca.
#[tokio::test]
async fn a_handover_needs_a_lease_in_force() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "subentro-stati").await;

    for (label, start, activate) in [
        ("in bozza", -10, false),
        ("non ancora cominciato", 30, true),
    ] {
        let property = create_property(&app, &owner, &format!("Casa {label}")).await;
        let lease = draft_lease(&app, &owner, &property, None, start, Some(start + 365)).await;
        let email = common::unique_email("subentro-stati-parte");
        add_tenant(&app, &owner, &lease, "Parte Unica", Some(&email)).await;
        if activate {
            let (status, body) = app
                .post_auth(
                    &format!("/api/v1/leases/{lease}/activate"),
                    &owner,
                    serde_json::json!({}),
                )
                .await;
            assert_eq!(status, StatusCode::OK, "corpo: {body}");
        }
        let (_, detail) = app
            .send(
                "GET",
                &format!("/api/v1/leases/{lease}"),
                Some(&owner),
                None,
            )
            .await;
        let party = detail["parties"][0]["id"].as_str().expect("id");

        let (status, problem) = app
            .post_auth(
                &format!("/api/v1/leases/{lease}/handover"),
                &owner,
                serde_json::json!({
                    "leaving_party_id": party,
                    "joining": {
                        "full_name": "Chi Entra",
                        "email": common::unique_email("subentro-stati-entra"),
                        "share_percent": "100",
                    },
                }),
            )
            .await;
        assert_eq!(status, StatusCode::CONFLICT, "{label}: {problem}");
        assert_eq!(problem["code"], "lease.invalid_transition", "{label}");
    }
}

/// **La seconda porta di ADR-046**, chiusa (ADR-056): un invito a un'email nuova
/// su un contratto attivo creava una parte a `remaining_share`, cioè a zero, con
/// chi doveva uscire ancora dentro. Invitare una parte che esiste resta ammesso:
/// «si attiva, poi si invita».
#[tokio::test]
async fn an_invitation_reaches_an_existing_party_of_an_active_lease_but_adds_no_new_one() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "invito-a-contratto-attivo").await;
    let property = create_property(&app, &owner, "Casa dell'invito tardivo").await;
    let lease = draft_lease(&app, &owner, &property, None, -10, Some(355)).await;
    let email = common::unique_email("invito-tardivo");
    add_tenant(&app, &owner, &lease, "Inquilino Tardivo", Some(&email)).await;
    let (status, body) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {body}");

    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/invitations"),
            &owner,
            serde_json::json!({
                "email": common::unique_email("invito-intruso"),
                "full_name": "Chi Non C'era",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "lease.parties_locked");

    invite_tenant(&app, &owner, &lease, &email).await;
    let (_, detail) = app
        .send(
            "GET",
            &format!("/api/v1/leases/{lease}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(
        detail["parties"].as_array().expect("parti").len(),
        1,
        "corpo: {detail}"
    );
}

/// **Una parte con una persona dietro non è una riga sbagliata.**
///
/// Due rifiuti, e sono la ragione per cui correggere e togliere non sono la stessa
/// operazione: l'email è ciò a cui l'invito è intestato, e cambiarla dopo
/// l'accettazione sposterebbe il rapporto sotto i piedi di chi l'ha usata;
/// togliere una parte collegata farebbe sparire un inquilino vero senza traccia,
/// mentre uscire di casa è un fatto con una data.
///
/// La quota invece **si corregge anche a contratto attivo**, ed è deliberato: i
/// contratti con le quote sbagliate che esistono già sono attivi, e rifiutare lì
/// lascerebbe irreparabili proprio quelli.
#[tokio::test]
async fn a_party_with_a_user_keeps_its_email_and_its_place() {
    let app = require_app!();
    let (owner, _org) = common::register_and_login(&app, "parte-collegata").await;
    let property = create_property(&app, &owner, "Immobile con inquilino vero").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    let tenant_email = common::unique_email("inquilino-collegato");
    let (_, accept_token) = invite_tenant(&app, &owner, &lease, &tenant_email).await;
    common::register_tenant_from_invitation(
        &app,
        &accept_token,
        "Inquilino Collegato",
        "cavallo batteria graffetta",
    )
    .await;

    let (_, detail) = app
        .send(
            "GET",
            &format!("/api/v1/leases/{lease}"),
            Some(&owner),
            None,
        )
        .await;
    let party = detail["parties"][0]["id"].as_str().expect("id").to_owned();

    let (status, problem) = app
        .send(
            "PATCH",
            &format!("/api/v1/lease-parties/{party}"),
            Some(&owner),
            Some(serde_json::json!({ "email": "un-altro@example.it" })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "lease.party_email_locked");

    let (status, problem) = app
        .send(
            "DELETE",
            &format!("/api/v1/lease-parties/{party}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "lease.party_has_user");

    // Il contratto si attiva — la quota dell'invito è 100 — e la correzione
    // continua a funzionare da lì in avanti.
    let (status, activated) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {activated}");

    let (status, corrected) = app
        .send(
            "PATCH",
            &format!("/api/v1/lease-parties/{party}"),
            Some(&owner),
            Some(serde_json::json!({ "full_name": "Nome Corretto" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {corrected}");
    assert_eq!(corrected["full_name"], "Nome Corretto");
}

/// **L'occupazione la decide il calendario, e per questo non è una colonna.**
///
/// È la prova che un trigger su attivazione e disdetta sarebbe sbagliato: il
/// secondo contratto è **attivo** e comincia fra un mese, quindi la sua stanza
/// oggi è libera. Un `UPDATE` all'attivazione l'avrebbe già marcata occupata, e
/// nessun trigger scatterebbe la notte in cui il contratto comincia davvero.
///
/// Guarda anche la seconda metà della decisione: lo stato **dichiarato** resta
/// quello che qualcuno ha scritto, e i due valori viaggiano insieme — una stanza
/// dichiarata fuori uso mentre ci abita una persona è un'anomalia da vedere, non
/// da nascondere dietro la precedenza di un campo sull'altro.
#[tokio::test]
async fn a_room_is_occupied_by_the_calendar_and_not_by_a_column() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "occupazione").await;
    let property = create_property(&app, &token, "Immobile in co-living").await;

    let mut ids = Vec::new();
    for nome in ["Stanza abitata", "Stanza che lo sarà"] {
        let (status, room) = app
            .post_auth(
                &format!("/api/v1/properties/{property}/rooms"),
                &token,
                serde_json::json!({ "name": nome, "monthly_rent": "400.00" }),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "corpo: {room}");
        // Appena creata, nessuna stanza è occupata.
        assert_eq!(room["status"], "available");
        assert_eq!(room["effective_status"], "available");
        ids.push(room["id"].as_str().expect("id").to_owned());
    }
    let (abitata, futura) = (&ids[0], &ids[1]);

    // Il primo contratto è in essere adesso.
    let ora = draft_lease(&app, &token, &property, Some(abitata), -10, Some(355)).await;
    add_tenant(&app, &token, &ora, "Inquilino Presente", None).await;
    let (status, _) = app
        .post_auth(
            &format!("/api/v1/leases/{ora}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Il secondo è attivo e comincia **fra un mese**.
    let domani = draft_lease(&app, &token, &property, Some(futura), 30, Some(395)).await;
    add_tenant(&app, &token, &domani, "Inquilino Futuro", None).await;
    let (status, _) = app
        .post_auth(
            &format!("/api/v1/leases/{domani}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, page) = app
        .send(
            "GET",
            &format!("/api/v1/properties/{property}/rooms"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");
    let rooms = page["items"].as_array().expect("stanze");
    let occupata = rooms
        .iter()
        .find(|room| room["id"] == serde_json::json!(abitata))
        .expect("stanza abitata");
    let libera = rooms
        .iter()
        .find(|room| room["id"] == serde_json::json!(futura))
        .expect("stanza futura");

    assert_eq!(occupata["effective_status"], "occupied", "corpo: {page}");
    // Lo stato dichiarato **non** è cambiato: nessuno lo ha scritto.
    assert_eq!(occupata["status"], "available", "corpo: {page}");
    // E la stanza del contratto che comincia fra un mese è libera oggi.
    assert_eq!(libera["effective_status"], "available", "corpo: {page}");
}

/// **Lo stato dell'immobile segue le sue stanze**, e il filtro che serve è
/// l'occupazione.
///
/// `?status=rented` non può funzionare — quel valore non è memorizzato — e
/// restituire zero righe sarebbe il modo peggiore di dirlo: risponde `422`
/// indicando il filtro giusto, come fanno i contratti con `in_force` e gli
/// addebiti con `outstanding`.
#[tokio::test]
async fn a_property_is_partially_rented_when_one_of_its_rooms_is() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "occupazione-immobile").await;
    let property = create_property(&app, &token, "Immobile a metà").await;

    let mut ids = Vec::new();
    for nome in ["Prima stanza", "Seconda stanza"] {
        let (_, room) = app
            .post_auth(
                &format!("/api/v1/properties/{property}/rooms"),
                &token,
                serde_json::json!({ "name": nome }),
            )
            .await;
        ids.push(room["id"].as_str().expect("id").to_owned());
    }

    let lease = draft_lease(&app, &token, &property, Some(&ids[0]), -1, Some(364)).await;
    add_tenant(&app, &token, &lease, "Inquilino", None).await;
    let (status, _) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, detail) = app
        .send(
            "GET",
            &format!("/api/v1/properties/{property}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {detail}");
    assert_eq!(
        detail["status"], "available",
        "lo stato dichiarato non cambia: l'attivazione muove il derivato, e \
         `available` è quello con cui l'immobile è nato"
    );
    assert_eq!(
        detail["effective_status"], "partially_rented",
        "corpo: {detail}"
    );

    // Il filtro che funziona.
    let (status, page) = app
        .send(
            "GET",
            "/api/v1/properties?occupied=true",
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

    let (status, page) = app
        .send(
            "GET",
            "/api/v1/properties?occupied=false",
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        page["items"].as_array().map(Vec::len),
        Some(0),
        "corpo: {page}"
    );

    // E quello che non può funzionare, che lo dice invece di rispondere vuoto.
    let (status, problem) = app
        .send(
            "GET",
            "/api/v1/properties?status=rented",
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");
    assert_eq!(problem["code"], "property.derived_status_filter");
}

/// **Una bolletta pagata si può dire pagata**, e non è un pagamento.
///
/// `app.bill_status` contiene `'paid'` da sempre e nessuna transizione lo
/// scriveva: l'elenco non distingueva le bollette saldate da quelle da saldare,
/// che è la sola domanda che ci si fa guardandolo.
#[tokio::test]
async fn a_validated_bill_can_be_declared_paid() {
    let app = require_app!();
    let (token, _org) = common::register_and_login(&app, "bolletta-pagata").await;
    let property = create_property(&app, &token, "Immobile con gas").await;
    let bill = register_bill(&app, &token, &property, None, "PAGATA-1").await;

    // In bozza non si dichiara: i numeri non li ha confermati nessuno.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/pay"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "utility_bill.invalid_transition");

    let (status, _) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/validate"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, pagata) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/pay"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {pagata}");
    assert_eq!(pagata["status"], "paid");
    // La data è obbligatoria a schema quando lo stato c'è: senza, «pagata» non
    // direbbe quando.
    assert!(pagata["paid_at"].is_string(), "corpo: {pagata}");

    // Non si dichiara due volte: una bolletta pagata resta pagata.
    let (status, _) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/pay"),
            &token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
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

    // L'inquilino si registra dall'invito: nasce direttamente dentro
    // l'organizzazione di chi lo ospita, senza una propria (ADR-053).
    let (tenant, _tenant_id) = common::register_tenant_from_invitation(
        &app,
        &accept_token,
        "Inquilino Reale",
        "cavallo batteria graffetta",
    )
    .await;

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

/// L'elenco delle proprie organizzazioni, che è la sola rotta che risponde
/// **senza un'organizzazione attiva**.
///
/// Le tre asserzioni sono le tre ragioni per cui esiste: il **nome** — senza cui
/// un selettore mostra due UUID fra cui indovinare — l'isolamento, e la risposta
/// a chi non ha dichiarato `X-Org-Id`. La terza è la decisione «`B0` è
/// `ensure_authenticated` e non `ensure_member`» verificata invece che dichiarata:
/// con `ensure_member` questa chiamata darebbe `403 org.not_selected` quando il
/// token non porta un'organizzazione, cioè proprio nel caso che l'endpoint serve.
#[tokio::test]
async fn the_organizations_of_a_user_come_back_with_their_names() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "proprietario").await;
    // Un secondo proprietario, del tutto estraneo: la sua organizzazione non deve
    // comparire nell'elenco di nessun altro.
    let (stranger, stranger_org) = common::register_and_login(&app, "estraneo").await;

    let property = create_property(&app, &owner, "Immobile affittato").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    let tenant_email = common::unique_email("inquilino");
    let (_, accept_token) = invite_tenant(&app, &owner, &lease, &tenant_email).await;
    let (tenant, _tenant_id) = common::register_tenant_from_invitation(
        &app,
        &accept_token,
        "Inquilino Test",
        "cavallo batteria graffetta",
    )
    .await;

    // **Senza `X-Org-Id`**: è la condizione che la decisione dichiara possibile.
    let (status, page) = app
        .send("GET", "/api/v1/organizations", Some(&tenant), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");

    let items = page["items"].as_array().expect("elenco");
    // Una sola organizzazione, quella di chi ha invitato: prima di ADR-053
    // l'inquilino ne aveva anche una propria vuota, e questo elenco ne contava
    // **due**.
    assert_eq!(
        items.len(),
        1,
        "un inquilino ha una sola organizzazione, quella di chi lo ha invitato: prima di ADR-053 ne aveva due (la propria e quella di chi lo ha invitato). corpo: {page}"
    );

    let rented = items
        .iter()
        .find(|item| item["id"] == serde_json::Value::String(owner_org.clone()))
        .expect("l'organizzazione del proprietario");
    assert_eq!(rented["role"], "tenant");
    // Il nome, che è il punto: prima di questa rotta nessun endpoint lo dava a
    // chi non fosse staff dell'organizzazione.
    assert_eq!(
        rented["name"], "Proprietario Test",
        "il nome dell'organizzazione manca: {rented}"
    );
    assert!(
        rented["slug"].as_str().is_some_and(|slug| !slug.is_empty()),
        "lo slug manca: {rented}"
    );

    // L'organizzazione dell'estraneo non compare, e non perché la si filtri: la
    // policy `organizations_select` è `app.is_org_member(id)`.
    assert!(
        !items
            .iter()
            .any(|item| item["id"] == serde_json::Value::String(stranger_org.clone())),
        "un'organizzazione altrui è finita nell'elenco: {page}"
    );

    // E la simmetria: l'estraneo vede la propria e nient'altro.
    let (status, page) = app
        .send("GET", "/api/v1/organizations", Some(&stranger), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");
    let ids: Vec<&str> = page["items"]
        .as_array()
        .expect("elenco")
        .iter()
        .filter_map(|item| item["id"].as_str())
        .collect();
    assert_eq!(ids, vec![stranger_org.as_str()], "corpo: {page}");
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

    // Serve un account che esiste già a prescindere dal token revocato:
    // `register_tenant_from_invitation` non ne creerebbe uno da un token non
    // valido, e non resterebbe nulla su cui verificare «nessuna scrittura
    // parziale» qui sotto.
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

/// **Il recupero della password, da capo a fondo**, che è `Z2`.
///
/// Prima di questo passo le rotte di autenticazione erano quattro e il primo «non mi
/// ricordo la password» di un utente vero finiva in una `UPDATE` scritta a mano sul
/// database di produzione.
///
/// Le tre cose che il test fissa sono le tre che si perdono, e nessuna delle tre si
/// vede guardando la schermata:
///
/// 1. **la conferma revoca le sessioni**, comprese quelle aperte prima. Senza,
///    cambiare la password non butta fuori chi era entrato con quella vecchia — e un
///    refresh token vive trenta giorni;
/// 2. **un indirizzo sconosciuto riceve la stessa risposta di uno noto**, e non manda
///    niente: distinguerli sarebbe un oracolo per sapere chi ha un account;
/// 3. **un codice già usato non vale due volte.**
#[tokio::test]
async fn a_forgotten_password_can_be_recovered_and_the_old_sessions_die() {
    let app = require_app!();
    let email = common::unique_email("smemorato");
    let vecchia = "cavallo batteria graffetta";
    let invitation_token = common::issue_owner_invitation(&app, &email).await;
    let (status, _) = app
        .post(
            "/api/v1/auth/accept-owner-invitation",
            serde_json::json!({
                "token": invitation_token,
                "full_name": "Proprietario Test",
                "password": vecchia,
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);

    // La sessione di prima, con il **cookie di refresh**: è ciò che la revoca
    // uccide, e l'unica cosa che si possa verificare subito. L'access token è un
    // JWT e resta valido i suoi quindici minuti — vedi l'asserzione più sotto.
    let (status, _, cookie) = app
        .post_returning_cookie(
            "/api/v1/auth/login",
            serde_json::json!({ "email": email, "password": vecchia }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let cookie = cookie.expect("cookie di refresh");

    let (status, body) = app
        .post(
            "/api/v1/auth/password-reset",
            serde_json::json!({ "email": email }),
        )
        .await;
    assert_eq!(status, StatusCode::ACCEPTED, "corpo: {body}");

    let code = match app.mail.sent().last() {
        Some((
            recipient,
            propflow_application::ports::NotificationContent::PasswordReset {
                code,
                valid_minutes,
            },
        )) => {
            assert_eq!(recipient, &email);
            // La durata è nel testo perché chi legge deve sapere quanto ha: un
            // codice che scade senza dirlo è un codice che scade addosso.
            assert!(*valid_minutes > 0);
            code.clone()
        }
        other => panic!("il codice di recupero non è partito: {other:?}"),
    };

    // Una password debole viene rifiutata **prima** di consumare il codice: chi
    // sbaglia la password non deve doversi rifare mandare il codice.
    let (status, problem) = app
        .post(
            "/api/v1/auth/password-reset/confirm",
            serde_json::json!({ "code": code, "new_password": "corta" }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");

    let nuova = "cavallo batteria graffetta nuova";
    let (status, body) = app
        .post(
            "/api/v1/auth/password-reset/confirm",
            serde_json::json!({ "code": code, "new_password": nuova }),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "corpo: {body}");

    // 1. **La sessione di prima è morta**: il refresh non rinnova più. È la parte
    //    che si dimentica, ed è quella che rende il recupero una difesa invece di
    //    una comodità — un refresh token vive trenta giorni.
    //
    //    Quello che **non** muore è l'access token già emesso, che è un JWT e non è
    //    revocabile: chi ce l'ha resta dentro fino alla scadenza, quindici minuti.
    //    È il baratto di ADR-012 e va saputo, perché «revoca tutte le sessioni» si
    //    legge come «lo butta fuori adesso» e non è vero.
    let (status, body, _) = app
        .post_with_cookie("/api/v1/auth/refresh", Some(&cookie), None)
        .await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "il refresh della vecchia sessione funziona ancora: {body}"
    );

    // La password nuova funziona, la vecchia no.
    let (status, body) = app
        .post(
            "/api/v1/auth/login",
            serde_json::json!({ "email": email, "password": nuova }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {body}");
    let (status, _) = app
        .post(
            "/api/v1/auth/login",
            serde_json::json!({ "email": email, "password": vecchia }),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // 3. Lo stesso codice non vale due volte, e la risposta non dice perché.
    let (status, problem) = app
        .post(
            "/api/v1/auth/password-reset/confirm",
            serde_json::json!({ "code": code, "new_password": "un altro cavallo lungo" }),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "corpo: {problem}");

    // 2. Un indirizzo che non esiste riceve **la stessa** risposta, e non fa partire
    //    niente: è la regola di ADR-017 applicata al recupero.
    let prima = app.mail.sent().len();
    let (status, body) = app
        .post(
            "/api/v1/auth/password-reset",
            serde_json::json!({ "email": common::unique_email("mai-registrato") }),
        )
        .await;
    assert_eq!(status, StatusCode::ACCEPTED, "corpo: {body}");
    assert_eq!(
        app.mail.sent().len(),
        prima,
        "un indirizzo sconosciuto ha fatto partire un'email"
    );
}

/// **Il limite agli invii è per email**, e rifiuta in silenzio.
///
/// Un recupero senza freno è un modo per far arrivare cinquanta email a un indirizzo
/// altrui. Il rifiuto è silenzioso — `202` come sempre — perché dirlo sarebbe di
/// nuovo l'oracolo: «troppo presto» conferma che l'account esiste.
#[tokio::test]
async fn asking_for_a_second_code_too_soon_sends_nothing_and_says_nothing() {
    let app = require_app!();
    let email = common::unique_email("insistente");
    common::register_and_login_with_email(&app, &email).await;

    let (status, _) = app
        .post(
            "/api/v1/auth/password-reset",
            serde_json::json!({ "email": email }),
        )
        .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let dopo_il_primo = app.mail.sent().len();

    let (status, body) = app
        .post(
            "/api/v1/auth/password-reset",
            serde_json::json!({ "email": email }),
        )
        .await;
    // Stessa risposta del primo, e nessuna seconda email.
    assert_eq!(status, StatusCode::ACCEPTED, "corpo: {body}");
    assert_eq!(
        app.mail.sent().len(),
        dopo_il_primo,
        "il limite agli invii non ha fermato il secondo codice"
    );
}

/// **L'invito parte per posta**, che è il criterio di fine di `Z1`.
///
/// Prima di questo passo il token tornava solo nella risposta HTTP e il
/// proprietario lo consegnava a mano: era deliberato finché `Notifier` era una porta
/// senza adattatore, e smette di bastare quando la posta diventa l'unico canale
/// verso chi è chiuso fuori dal proprio account.
///
/// Il test guarda **il contenuto** e non il fatto che qualcosa sia partito: nel corpo
/// c'è lo stesso codice che la risposta consegna, e c'è il nome dell'organizzazione,
/// senza cui chi riceve un codice non sa da chi arriva.
///
/// **Il token resta anche nella risposta**, ed è la metà che si toglierebbe
/// «pulendo»: l'invio è best-effort dopo il commit, quindi con il relay fermo quella
/// è l'unica copia che esiste.
#[tokio::test]
async fn inviting_a_tenant_sends_the_code_by_mail() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "invito-per-posta").await;
    let property = create_property(&app, &owner, "Immobile con posta").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    let email = common::unique_email("destinatario");
    let (_, accept_token) = invite_tenant(&app, &owner, &lease, &email).await;

    let sent = app.mail.sent();
    assert_eq!(sent.len(), 1, "l'invito non è stato recapitato per posta");
    let (recipient, content) = &sent[0];
    assert_eq!(recipient, &email);
    match content {
        propflow_application::ports::NotificationContent::TenantInvitation {
            organization_name,
            accept_token: sent_token,
        } => {
            // Lo **stesso** codice della risposta: se fossero due, uno dei due non
            // funzionerebbe e nessuno saprebbe quale.
            assert_eq!(sent_token, &accept_token);
            assert_eq!(
                organization_name, "Proprietario Test",
                "l'email non dice da chi arriva l'invito"
            );
        }
        other => panic!("notifica inattesa: {other:?}"),
    }

    // La risposta continua a portarlo: con il relay fermo è l'unica copia.
    assert!(!accept_token.is_empty());
}

/// **L'inquilino che torna**, che è `R31`: il ciclo di vita normale del prodotto.
///
/// Un contratto finisce, con la stessa persona se ne fa un altro. Prima di questo
/// test la seconda accettazione rispondeva `409 membership.already_exists` — su un
/// codice che il proprietario aveva appena ricevuto senza un avviso — e siccome
/// `lease_parties.user_id` lo scrive **solo** l'accettazione, l'inquilino restava
/// fuori dal contratto nuovo senza nessun modo di rientrare.
///
/// **Nessun test poteva trovarlo**, e la ragione va tenuta: ogni altro test
/// costruisce un inquilino **nuovo**. Il difetto viveva nella *seconda* volta che
/// qualcosa succede alla stessa persona, che è lo spazio che una suite verde non
/// attraversa. Questo test è quella seconda volta.
#[tokio::test]
async fn a_returning_tenant_can_accept_a_second_invitation() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "inquilino-che-torna").await;
    let property = create_property(&app, &owner, "Casa con due stagioni").await;

    let (_, first_room) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/rooms"),
            &owner,
            serde_json::json!({ "name": "Camera del primo anno", "monthly_rent": "400.00" }),
        )
        .await;
    let first_room = first_room["id"].as_str().expect("id").to_owned();
    let (_, second_room) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/rooms"),
            &owner,
            serde_json::json!({ "name": "Camera del secondo anno", "monthly_rent": "450.00" }),
        )
        .await;
    let second_room = second_room["id"].as_str().expect("id").to_owned();

    // CASO 1 — l'inquilino nuovo. È il lato che non deve rompersi.
    let first_lease = draft_lease(&app, &owner, &property, Some(&first_room), 0, Some(365)).await;
    let tenant_email = common::unique_email("torna");
    let (_, first_token) = invite_tenant(&app, &owner, &first_lease, &tenant_email).await;
    let (tenant, _tenant_id) = common::register_tenant_from_invitation(
        &app,
        &first_token,
        "Inquilino Che Torna",
        "cavallo batteria graffetta",
    )
    .await;
    assert_eq!(visible_leases(&app, &tenant, &owner_org).await.len(), 1);

    // CASO 2 — la stessa persona, un contratto nuovo. Qui rispondeva `409`.
    let second_lease =
        draft_lease(&app, &owner, &property, Some(&second_room), 400, Some(730)).await;
    let (_, second_token) = invite_tenant(&app, &owner, &second_lease, &tenant_email).await;

    let (status, accepted) = app
        .post_auth(
            "/api/v1/auth/accept-invitation",
            &tenant,
            serde_json::json!({ "token": second_token }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {accepted}");
    assert_eq!(accepted["lease_id"], second_lease, "corpo: {accepted}");

    // **Tutti e due**, non l'ultimo: il primo contratto non si perde per strada.
    let mut visible = visible_leases(&app, &tenant, &owner_org).await;
    visible.sort();
    let mut expected = vec![first_lease, second_lease.clone()];
    expected.sort();
    assert_eq!(
        visible, expected,
        "l'inquilino non vede entrambi i contratti"
    );

    // La parte contrattuale del secondo è collegata: è ciò che rende visibile il
    // contratto, e la ragione per cui il `409` era irreparabile.
    let (_, detail) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/leases/{second_lease}"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert!(
        detail["parties"][0]["user_id"].is_string(),
        "la parte del secondo contratto non è stata collegata: {detail}"
    );

    // Una sola appartenenza, non due: la membership non è stata reinserita.
    let (status, orgs) = app
        .send("GET", "/api/v1/organizations", Some(&tenant), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {orgs}");
    let in_owner_org = orgs["items"]
        .as_array()
        .expect("elenco")
        .iter()
        .filter(|item| item["id"] == serde_json::Value::String(owner_org.clone()))
        .count();
    assert_eq!(in_owner_org, 1, "appartenenza duplicata: {orgs}");
}

/// **Il ruolo non si tocca.** Chi è già staff e viene invitato come inquilino
/// resta staff.
///
/// È il caso vero di chi affitta una casa e ne abita un'altra, ed è anche l'assert
/// che qualcuno toglierebbe «uniformando» il ramo: far coincidere il ruolo con
/// quello dell'invito retrocederebbe il proprietario a `tenant`
/// nella **propria** organizzazione.
#[tokio::test]
async fn accepting_a_tenant_invitation_does_not_demote_an_existing_member() {
    let app = require_app!();
    let email = common::unique_email("proprietario-inquilino");
    // Un proprietario con un'organizzazione propria, non un inquilino al primo
    // ingresso: è lui il caso che il test prova, quindi passa da
    // `register_and_login_with_email` (invito da proprietario) e non da
    // `register_tenant_from_invitation`.
    let (owner, owner_org, _) = common::register_and_login_with_email(&app, &email).await;
    let property = create_property(&app, &owner, "Casa dove abita chi la possiede").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    let (_, accept_token) = invite_tenant(&app, &owner, &lease, &email).await;
    let (status, accepted) = app
        .post_auth(
            "/api/v1/auth/accept-invitation",
            &owner,
            serde_json::json!({ "token": accept_token }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {accepted}");

    let (status, orgs) = app
        .send("GET", "/api/v1/organizations", Some(&owner), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {orgs}");
    let membership = orgs["items"]
        .as_array()
        .expect("elenco")
        .iter()
        .find(|item| item["id"] == serde_json::Value::String(owner_org.clone()))
        .expect("la propria organizzazione");
    assert_eq!(
        membership["role"], "owner",
        "l'invito ha retrocesso il proprietario: {orgs}"
    );
}

// ---------------------------------------------------------------------------
// Il proprio account e l'uscita da un'organizzazione (sezione 6)
// ---------------------------------------------------------------------------

/// `/api/v1/me` si legge, si corregge, e **non prende mai un identificatore**.
///
/// Tre cose in un giro solo: la lettura non porta il ruolo di piattaforma; il
/// `PATCH` è parziale, con `null` che azzera; e l'email si riconosce e si rifiuta
/// con un `422` che nomina il campo — non un `200` con l'indirizzo di prima, che
/// sarebbe il sintomo muto.
#[tokio::test]
async fn the_account_reads_and_corrects_itself_and_the_email_stays_put() {
    let app = require_app!();
    let email = common::unique_email("me");
    let (token, _, _) = common::register_and_login_with_email(&app, &email).await;

    let (status, me) = app.send("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK, "corpo: {me}");
    assert_eq!(me["email"], email);
    assert_eq!(me["full_name"], "Proprietario Test");
    assert_eq!(me["locale"], "it-IT");
    assert!(me.get("phone").is_none(), "corpo: {me}");
    // Non è affare di chi guarda, e non deve nemmeno comparire a `null`.
    assert!(me.get("platform_role").is_none(), "corpo: {me}");
    assert!(me.get("failed_login_count").is_none(), "corpo: {me}");

    // Senza `Authorization` non c'è nessuno da leggere.
    let (status, _) = app.send("GET", "/api/v1/me", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, updated) = app
        .send(
            "PATCH",
            "/api/v1/me",
            Some(&token),
            Some(serde_json::json!({ "full_name": "  Maria Rossi ", "phone": "+39 333 1234567" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {updated}");
    assert_eq!(updated["full_name"], "Maria Rossi");
    assert_eq!(updated["phone"], "+39 333 1234567");

    // Si ricarica, e il telefono è quello: è il criterio di fine del piano.
    let (_, again) = app.send("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(again["phone"], "+39 333 1234567");

    // `null` azzera, e un campo non menzionato resta.
    let (status, cleared) = app
        .send(
            "PATCH",
            "/api/v1/me",
            Some(&token),
            Some(serde_json::json!({ "phone": null })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {cleared}");
    assert!(cleared.get("phone").is_none(), "corpo: {cleared}");
    assert_eq!(cleared["full_name"], "Maria Rossi");

    // Un corpo vuoto non è un errore: risponde con i dati invariati.
    let (status, _) = app
        .send(
            "PATCH",
            "/api/v1/me",
            Some(&token),
            Some(serde_json::json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Un telefono malformato è un `422` che nomina il campo, non un `409` di
    // vincolo: la regola del `CHECK` è ripetuta nel dominio apposta.
    let (status, problem) = app
        .send(
            "PATCH",
            "/api/v1/me",
            Some(&token),
            Some(serde_json::json!({ "phone": "chiamami" })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");
    assert_eq!(problem["errors"][0]["field"], "phone");

    // **L'email non si cambia, e il tentativo lo dice.**
    let (status, problem) = app
        .send(
            "PATCH",
            "/api/v1/me",
            Some(&token),
            Some(serde_json::json!({ "email": common::unique_email("altra") })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");
    assert_eq!(problem["errors"][0]["field"], "email");
    let (_, me) = app.send("GET", "/api/v1/me", Some(&token), None).await;
    assert_eq!(me["email"], email, "l'email è cambiata: {me}");

    // **Nessuna rotta proietta un utente per identificatore** (ADR-036): la forma
    // con l'id nel percorso non esiste, nemmeno per il proprio.
    let id = me["id"].as_str().expect("id");
    let (status, _) = app
        .send("GET", &format!("/api/v1/users/{id}"), Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Cambiare la password da dentro pretende quella vecchia e **chiude tutto**.
///
/// Il secondo assert è quello che si scrive sbagliato, ed è l'inciampo della
/// 5-quater: asserire che la vecchia sessione smetta di *leggere* fallirebbe,
/// perché l'access token è un JWT e vive i suoi quindici minuti. Ciò che muore
/// è il **rinnovo**, e il test verifica quello — su **entrambe** le sessioni,
/// compresa quella da cui si è chiesto il cambio.
#[tokio::test]
async fn changing_the_password_needs_the_old_one_and_no_session_renews_anymore() {
    let app = require_app!();
    let email = common::unique_email("cambia-password");
    let vecchia = "cavallo batteria graffetta";
    let invitation_token = common::issue_owner_invitation(&app, &email).await;
    let (status, _) = app
        .post(
            "/api/v1/auth/accept-owner-invitation",
            serde_json::json!({ "token": invitation_token, "full_name": "Proprietario Test", "password": vecchia }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);

    // Due sessioni: quella da cui si cambia e «il telefono».
    let (status, body, desk_cookie) = app
        .post_returning_cookie(
            "/api/v1/auth/login",
            serde_json::json!({ "email": email, "password": vecchia }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {body}");
    let desk = body["access_token"].as_str().expect("token").to_owned();
    let desk_cookie = desk_cookie.expect("cookie di refresh");
    let (_, _, phone_cookie) = app
        .post_returning_cookie(
            "/api/v1/auth/login",
            serde_json::json!({ "email": email, "password": vecchia }),
        )
        .await;
    let phone_cookie = phone_cookie.expect("cookie di refresh");

    let nuova = "cavallo batteria graffetta nuova";

    // La vecchia sbagliata: `401`, e **niente è cambiato** — la vecchia giusta
    // funziona ancora al login, quindi nessuna scrittura è passata.
    let (status, problem) = app
        .post_auth(
            "/api/v1/me/password",
            &desk,
            serde_json::json!({ "current_password": "non è questa, di sicuro", "new_password": nuova }),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "corpo: {problem}");
    let (status, _) = app
        .post(
            "/api/v1/auth/login",
            serde_json::json!({ "email": email, "password": vecchia }),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "la vecchia password non vale più dopo un tentativo rifiutato"
    );
    // E il rifiuto non ha consumato il rinnovo di nessuno.
    let (status, _, desk_cookie) = app
        .post_with_cookie("/api/v1/auth/refresh", Some(&desk_cookie), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let desk_cookie = desk_cookie.expect("cookie ruotato");

    // Una password nuova debole è un `422` che nomina il campo, prima di toccare
    // qualunque cosa.
    let (status, problem) = app
        .post_auth(
            "/api/v1/me/password",
            &desk,
            serde_json::json!({ "current_password": vecchia, "new_password": "corta" }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");

    let (status, body) = app
        .post_auth(
            "/api/v1/me/password",
            &desk,
            serde_json::json!({ "current_password": vecchia, "new_password": nuova }),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "corpo: {body}");

    // **Nessuna sessione rinnova più**, compresa quella da cui si è cambiata: è la
    // decisione 3 della sezione, uniforme ad ADR-052.
    for (name, cookie) in [("scrivania", &desk_cookie), ("telefono", &phone_cookie)] {
        let (status, body, _) = app
            .post_with_cookie("/api/v1/auth/refresh", Some(cookie), None)
            .await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "la sessione «{name}» rinnova ancora: {body}"
        );
    }

    // L'access token già emesso invece **legge ancora**: è un JWT, non è
    // revocabile, e vive i suoi quindici minuti. È il baratto di ADR-012, e
    // asserire il contrario è l'errore che questo test esiste per non rifare.
    let (status, _) = app.send("GET", "/api/v1/me", Some(&desk), None).await;
    assert_eq!(status, StatusCode::OK);

    // Si rientra con la nuova, non con la vecchia.
    let (status, _) = app
        .post(
            "/api/v1/auth/login",
            serde_json::json!({ "email": email, "password": nuova }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app
        .post(
            "/api/v1/auth/login",
            serde_json::json!({ "email": email, "password": vecchia }),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

/// Si esce, non si esce con un contratto in essere, e chi è uscito **rientra**.
///
/// Il secondo rifiuto arriva dal database e non da un `if`: il test lo prova
/// due volte, prima dalla rotta — che dà la versione parlante — e poi con
/// un'`UPDATE` scritta a mano sotto la Row Level Security dell'inquilino, che è
/// l'unica prova che il confine non sia solo nel service layer.
///
/// Il terzo è la porta di servizio di ADR-051, ora percorribile: senza il ramo di
/// riattivazione l'accettazione morirebbe con `409 membership.already_exists`.
///
/// Fra il secondo e il terzo, **da fuori si guarda con il token di prima**: è il
/// token che chi esce ha davvero in mano per un quarto d'ora, e fino alla `0041`
/// gli mostrava ancora il contratto (`R46`).
#[tokio::test]
async fn a_tenant_leaves_an_organization_unless_a_lease_is_in_force_and_can_come_back() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "casa-da-cui-si-esce").await;
    let property = create_property(&app, &owner, "Casa da cui si esce").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    let tenant_email = common::unique_email("chi-esce");
    let (_, accept_token) = invite_tenant(&app, &owner, &lease, &tenant_email).await;
    let (tenant, _tenant_id) = common::register_tenant_from_invitation(
        &app,
        &accept_token,
        "Inquilino Che Esce",
        "cavallo batteria graffetta",
    )
    .await;

    // Il contratto entra in essere.
    let (status, _) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 1. **Con un contratto in essere non si esce**, e la risposta dice perché.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/organizations/{owner_org}/leave"),
            &tenant,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "membership.lease_in_force");

    // La stessa scrittura a mano, sotto la RLS dell'inquilino: **zero righe**, che
    // è il rifiuto della `WITH CHECK` e non di un `if`.
    let tenant_id = app.send("GET", "/api/v1/me", Some(&tenant), None).await.1["id"]
        .as_str()
        .expect("id")
        .to_owned();
    let mut tx = app
        .pools
        .application()
        .begin()
        .await
        .expect("transazione applicativa");
    sqlx::query("SELECT app.set_request_context($1, $2, false, NULL, NULL, NULL, 'tenant')")
        .bind(uuid::Uuid::parse_str(&tenant_id).expect("uuid"))
        .bind(uuid::Uuid::parse_str(&owner_org).expect("uuid"))
        .execute(&mut *tx)
        .await
        .expect("contesto installato");
    let error = sqlx::query(
        "UPDATE app.org_members SET status = 'left' WHERE org_id = $1 AND user_id = $2",
    )
    .bind(uuid::Uuid::parse_str(&owner_org).expect("uuid"))
    .bind(uuid::Uuid::parse_str(&tenant_id).expect("uuid"))
    .execute(&mut *tx)
    .await
    .expect_err("la policy doveva rifiutare l'uscita con un contratto in essere");
    assert!(
        error.to_string().contains("row-level security"),
        "atteso il rifiuto della policy, ottenuto: {error}"
    );
    tx.rollback().await.expect("annullata");

    // Il contratto finisce: si disdice.
    let (status, body) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/terminate"),
            &owner,
            serde_json::json!({ "reason": "Trasloco" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {body}");

    // 2. **Si esce.** La risposta dice da dove e con che ruolo.
    let (status, left) = app
        .post_auth(
            &format!("/api/v1/organizations/{owner_org}/leave"),
            &tenant,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {left}");
    assert_eq!(left["organization_id"], owner_org);
    assert_eq!(left["role"], "tenant");

    // Da fuori: l'organizzazione non è più fra le proprie, e ciò che era dentro
    // non si legge più. **Nessuna** organizzazione resta: prima di ADR-053
    // l'inquilino aveva una propria organizzazione personale vuota e ci
    // ricadeva uscendo dall'unica che aveva; dopo, un inquilino non ne ha mai
    // avuta una.
    let (_, orgs) = app
        .send("GET", "/api/v1/organizations", Some(&tenant), None)
        .await;
    let ids: Vec<&str> = orgs["items"]
        .as_array()
        .expect("elenco")
        .iter()
        .filter_map(|item| item["id"].as_str())
        .collect();
    assert_eq!(
        ids,
        Vec::<&str>::new(),
        "un inquilino senza organizzazioni non ne ha più nessuna: prima di ADR-053 gli restava la propria organizzazione personale, qui rappresentata da un id che non esiste più. corpo: {orgs}"
    );

    // **Con il token vecchio**, e questo è il punto (`R46`). I suoi claim
    // portano ancora `org_id: owner_org`, e `middleware::context` si fida del
    // ruolo del token quando l'organizzazione richiesta coincide con quella dei
    // claim — un access token già emesso non si revoca a metà vita, il baratto
    // di ADR-012. Fino alla `0041` il ramo dell'inquilino di `can_access_lease`
    // e `can_access_property` guardava solo `lease_parties`, che l'uscita non
    // tocca: `GET /leases` rispondeva `200` **con il contratto disdetto nel
    // corpo**, e una segnalazione si apriva nell'organizzazione lasciata. Il
    // test faceva un login nuovo per non vederlo. Ora le due funzioni chiedono
    // anche la membership, e il confine tiene dalla Row Level Security, non dal
    // token: la risposta è `200` con un elenco che non porta più il contratto.
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
    let still_visible = page["items"]
        .as_array()
        .expect("elenco")
        .iter()
        .any(|item| item["id"] == serde_json::Value::String(lease.clone()));
    assert!(
        !still_visible,
        "chi è uscito legge ancora il contratto disdetto con il token vecchio: {page}"
    );

    // E non scrive: aprire una segnalazione nell'organizzazione lasciata passa da
    // `tickets_insert`, che consulta `can_access_property`. Il codice è quello che
    // l'esecuzione mostra, non uno deciso prima.
    let (status, problem) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/properties/{property}/tickets"),
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({
                "title": "Da fuori",
                "description": "Aperta con un token emesso prima dell'uscita",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {problem}");
    assert_eq!(problem["code"], "property.not_found", "corpo: {problem}");

    // Due volte no: la riga resta, ed è osservabile.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/organizations/{owner_org}/leave"),
            &tenant,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "membership.already_left");

    // Un'organizzazione di cui non si è mai stati membri: la riga non torna.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/organizations/{}/leave", uuid::Uuid::now_v7()),
            &tenant,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {problem}");
    assert_eq!(problem["code"], "membership.not_found");

    // 3. **Chi è uscito rientra**, accettando un invito nuovo. È la porta di
    //    servizio di ADR-051: senza la riattivazione, qui risponderebbe `409`.
    let second_lease = draft_lease(&app, &owner, &property, None, 30, Some(400)).await;
    let (_, second_token) = invite_tenant(&app, &owner, &second_lease, &tenant_email).await;
    let (status, accepted) = app
        .post_auth(
            "/api/v1/auth/accept-invitation",
            &tenant,
            serde_json::json!({ "token": second_token }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {accepted}");
    assert_eq!(accepted["organization_id"], owner_org);

    let (_, orgs) = app
        .send("GET", "/api/v1/organizations", Some(&tenant), None)
        .await;
    let back = orgs["items"]
        .as_array()
        .expect("elenco")
        .iter()
        .filter(|item| item["id"] == serde_json::Value::String(owner_org.clone()))
        .count();
    assert_eq!(
        back, 1,
        "chi è rientrato non compare, o compare due volte: {orgs}"
    );
    // E vede il contratto nuovo: la parte è collegata, la membership è attiva.
    assert!(
        visible_leases(&app, &tenant, &owner_org)
            .await
            .contains(&second_lease),
        "chi è rientrato non vede il contratto nuovo"
    );
}

/// **L'ultimo proprietario non se ne va**, e il ruolo non è scrivibile da chi esce.
///
/// La prima metà è l'invariante che lo schema delega al service layer: senza,
/// uscire dalla propria organizzazione personale la lascerebbe senza nessuno che la
/// amministri, e `organizations_select` non la mostrerebbe più a nessuno.
///
/// La seconda è la colonna che si dimentica: aprire la propria riga senza negare
/// `role` è dare a chiunque la promozione a `owner`. Il privilegio lo nega **prima**
/// della policy, quindi anche l'`UPDATE` su una riga inesistente fallisce.
#[tokio::test]
async fn the_last_owner_cannot_leave_and_nobody_can_promote_themselves() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "ultimo-proprietario").await;

    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/organizations/{owner_org}/leave"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(problem["code"], "membership.last_owner");

    // Ed è ancora dentro.
    let (_, orgs) = app
        .send("GET", "/api/v1/organizations", Some(&owner), None)
        .await;
    assert_eq!(orgs["items"][0]["id"], owner_org, "corpo: {orgs}");
    assert_eq!(orgs["items"][0]["role"], "owner");

    for denied in [
        "UPDATE app.org_members SET role = 'owner' WHERE id = $1",
        "UPDATE app.org_members SET org_id = $1 WHERE id = $1",
        "UPDATE app.org_members SET user_id = $1 WHERE id = $1",
        "UPDATE app.org_members SET deleted_at = now() WHERE id = $1",
    ] {
        let error = sqlx::query(denied)
            .bind(uuid::Uuid::now_v7())
            .execute(app.pools.application())
            .await
            .expect_err("l'aggiornamento doveva essere negato");
        assert!(
            error.to_string().contains("permission denied"),
            "atteso un rifiuto per privilegi su `{denied}`, ottenuto: {error}"
        );
    }
}

/// Gli identificatori dei contratti che un utente vede in un'organizzazione.
async fn visible_leases(app: &TestApp, token: &str, org: &str) -> Vec<String> {
    let (status, page) = app
        .send_in_org("GET", "/api/v1/leases", Some(token), Some(org), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");
    page["items"]
        .as_array()
        .expect("elenco")
        .iter()
        .filter_map(|item| item["id"].as_str())
        .map(str::to_owned)
        .collect()
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
    let (tenant, _tenant_id) = common::register_tenant_from_invitation(
        &app,
        &accept_token,
        "Inquilino Documenti",
        "cavallo batteria graffetta",
    )
    .await;

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

    // L'inquilino allega il proprio. Serve `X-Org-Id` per la regola 24: sempre,
    // anche quando quella dichiarata è l'unica organizzazione che ha.
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
    let (tenant, _tenant_id) = common::register_tenant_from_invitation(
        &app,
        &accept_token,
        "Inquilino Segnalazioni",
        "cavallo batteria graffetta",
    )
    .await;

    // 1. L'inquilino apre la segnalazione sul proprio immobile. Serve `X-Org-Id`
    //    per la regola 24: sempre, anche quando è l'unica organizzazione che ha.
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
    //    E **dice perché**: il motivo è obbligatorio da quando un thread chiuso non
    //    riceve più messaggi, perché riaprire è l'unica via per dire «non è
    //    risolto». Senza, l'API risponde `422`.
    let (status, problem) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/tickets/{ticket}/reopen"),
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({})),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "una riapertura senza motivo non lascia niente che la spieghi: {problem}"
    );

    let (status, reopened) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/tickets/{ticket}/reopen"),
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({ "reason": "Ha ricominciato a perdere due giorni dopo" })),
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
    let (tenant, _tenant_id) = common::register_tenant_from_invitation(
        &app,
        &accept_token,
        "Inquilino Ruoli",
        "cavallo batteria graffetta",
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
    for (prefix, name) in [
        ("coinquilino-uno", "Coinquilino Uno"),
        ("coinquilino-due", "Coinquilino Due"),
    ] {
        let email = common::unique_email(prefix);
        let (_, accept_token) = invite_tenant(&app, &owner, &lease, &email).await;
        let (token, _tenant_id) = common::register_tenant_from_invitation(
            &app,
            &accept_token,
            name,
            "cavallo batteria graffetta",
        )
        .await;
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
            Some(serde_json::json!({ "reason": "Secondo me non è risolto" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {problem}");
    assert_eq!(problem["code"], "ticket.not_yours", "corpo: {problem}");

    // Chi ha segnalato invece sì, e il suo motivo finisce nel thread.
    let (status, reopened) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/tickets/{ticket}/reopen"),
            Some(reporter),
            Some(&owner_org),
            Some(serde_json::json!({ "reason": "Perde ancora, sotto il lavello" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {reopened}");

    let (status, thread) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/tickets/{ticket}/messages"),
            Some(cotenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {thread}");
    let bodies: Vec<&str> = thread["items"]
        .as_array()
        .expect("messaggi")
        .iter()
        .filter_map(|message| message["body"].as_str())
        .collect();
    assert!(
        bodies.contains(&"Perde ancora, sotto il lavello"),
        "il motivo della riapertura vive nel thread, che non si riscrive mai: {thread}"
    );

    // E il thread dice **chi** ha scritto, non solo con che ruolo: in un
    // co-living i due inquilini sono entrambi `tenant`.
    let author = thread["items"]
        .as_array()
        .expect("messaggi")
        .iter()
        .find(|message| message["body"] == "Perde ancora, sotto il lavello")
        .expect("il messaggio della riapertura");
    assert_eq!(author["author_kind"], "tenant", "corpo: {thread}");
    assert!(
        author["author_name"].is_string(),
        "il coinquilino legge il nome dell'altro dal ramo `shares_home_with` di          `users_select`, che la 0030 aveva lasciato in piedi: {thread}"
    );
}

/// La correzione del testo è una rotta a sé, con **una sola** autorizzazione.
///
/// `PATCH /tickets/{id}` resta i costi e resta solo staff; `PATCH
/// /tickets/{id}/report` è il testo e la casella di visibilità, e vale per lo
/// staff sempre e per chi ha segnalato finché è aperta. Sono due rotte perché una
/// sola avrebbe due autorizzazioni secondo i campi presenti nel corpo, che è la
/// cosa che si sbaglia in silenzio (ADR-048).
#[tokio::test]
async fn the_report_is_corrected_from_a_route_of_its_own() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "proprietario-correzione").await;
    let property = create_property(&app, &owner, "Casa da correggere").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    let email = common::unique_email("inquilino-correzione");
    let (_, accept_token) = invite_tenant(&app, &owner, &lease, &email).await;
    let (tenant, _tenant_id) = common::register_tenant_from_invitation(
        &app,
        &accept_token,
        "Inquilino Correzione",
        "cavallo batteria graffetta",
    )
    .await;

    let (ticket, _) = open_ticket(
        &app,
        &tenant,
        Some(&owner_org),
        &property,
        Some(&lease),
        "Il bolier perde",
    )
    .await;

    // Chi ha segnalato corregge il refuso e dichiara la segnalazione di interesse
    // comune, nella stessa richiesta: i tre campi hanno la stessa autorizzazione.
    let (status, fixed) = app
        .send_in_org(
            "PATCH",
            &format!("/api/v1/tickets/{ticket}/report"),
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({
                "title": "Il boiler perde",
                "visible_to_residents": true
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {fixed}");
    assert_eq!(fixed["title"], "Il boiler perde");
    assert_eq!(fixed["visible_to_residents"], true);
    assert_eq!(
        fixed["description"], "Descrizione del guasto",
        "assente = invariato: il campo non nominato non si azzera, {fixed}"
    );

    // Il `PATCH` dei costi resta solo staff, e non ha imparato a scrivere il testo.
    let (status, problem) = app
        .send_in_org(
            "PATCH",
            &format!("/api/v1/tickets/{ticket}"),
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({ "actual_cost": "10.00" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {problem}");
    assert_eq!(problem["code"], "org.staff_role_required");

    // Preso in carico, il testo passa a chi amministra. L'assegnatario è chi ha
    // segnalato — l'unico identificatore che questa risposta porta — e basta a
    // far uscire la segnalazione da `open`, che è ciò che il test misura.
    let reporter_id = fixed["reported_by"]
        .as_str()
        .map(str::to_owned)
        .expect("chi ha segnalato");
    let (status, assigned) = app
        .post_auth(
            &format!("/api/v1/tickets/{ticket}/assign"),
            &owner,
            serde_json::json!({ "assignee_id": reporter_id }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {assigned}");

    let (status, problem) = app
        .send_in_org(
            "PATCH",
            &format!("/api/v1/tickets/{ticket}/report"),
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({ "title": "Troppo tardi" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {problem}");
    assert_eq!(
        problem["code"], "org.staff_role_required",
        "la segnalazione è sua, ma qualcuno ci ha messo mano: {problem}"
    );

    let (status, corrected) = app
        .send_in_org(
            "PATCH",
            &format!("/api/v1/tickets/{ticket}/report"),
            Some(&owner),
            None,
            Some(serde_json::json!({ "description": "Riformulata da chi amministra" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {corrected}");
    assert_eq!(corrected["description"], "Riformulata da chi amministra");
    assert_eq!(
        corrected["title"], "Il boiler perde",
        "il titolo non era nel corpo e non doveva cambiare: {corrected}"
    );
}

/// Su una segnalazione chiusa il thread **si legge e non si scrive**.
///
/// È una regola di dominio e non un controllo di interfaccia: nasconderla nel
/// client lascerebbe la rotta aperta, e questo test bussa direttamente alla rotta.
#[tokio::test]
async fn a_closed_ticket_does_not_take_messages_but_still_gives_them() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "proprietario-thread-chiuso").await;
    let property = create_property(&app, &owner, "Casa chiusa").await;
    let (ticket, _) = open_ticket(&app, &owner, None, &property, None, "Guasto risolto").await;

    app.post_auth(
        &format!("/api/v1/tickets/{ticket}/messages"),
        &owner,
        serde_json::json!({ "body": "Passo domani" }),
    )
    .await;
    app.post_auth(
        &format!("/api/v1/tickets/{ticket}/close"),
        &owner,
        serde_json::json!({}),
    )
    .await;

    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/tickets/{ticket}/messages"),
            &owner,
            serde_json::json!({ "body": "Un'altra cosa" }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(
        problem["code"], "ticket.thread_closed",
        "codice proprio: non è una transizione rifiutata, e il rimedio è riaprire — {problem}"
    );

    let (status, thread) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/tickets/{ticket}/messages"),
            Some(&owner),
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {thread}");
    assert_eq!(
        thread["items"].as_array().expect("messaggi").len(),
        1,
        "leggere resta permesso: {thread}"
    );
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

    // Né si riapre ciò che non è chiuso. Il motivo va mandato lo stesso: la
    // validazione del corpo viene **prima** dello stato, quindi senza si otterrebbe
    // un `422` e la transizione rifiutata non si vedrebbe affatto.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/tickets/{ticket}/reopen"),
            &owner,
            serde_json::json!({ "reason": "Non è mai stata chiusa" }),
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

/// **I nomi di ciò a cui una risposta porta**, che è `R32` — terza occorrenza dello
/// stesso difetto dopo `R14` sui contratti e `R21`/`R22` sulle persone.
///
/// Tre schede portavano un collegamento etichettato con il nome della *categoria*
/// — «Immobili», «Contratti» — invece che con quello della cosa. Il test guarda le
/// tre risposte insieme, perché è insieme che il difetto si vede: chi ne sistemasse
/// una sola lascerebbe una schermata in cui un nome compare e l'altro no.
///
/// Copre anche i due modi in cui un nome **manca**, che sono diversi fra loro: la
/// stanza assente è il caso normale di un contratto sull'immobile intero, il nome
/// dell'immobile assente è un immobile archiviato — e sono due assenze che il
/// client racconta in modo diverso.
#[tokio::test]
async fn tickets_charges_and_bills_carry_the_names_of_what_they_point_at() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "i-nomi").await;
    let property = create_property(&app, &owner, "Villa Aurora").await;

    let (_, room) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/rooms"),
            &owner,
            serde_json::json!({ "name": "Camera azzurra", "monthly_rent": "400.00" }),
        )
        .await;
    let room = room["id"].as_str().expect("id").to_owned();
    let lease = draft_lease(&app, &owner, &property, Some(&room), -10, Some(355)).await;

    // --- la segnalazione: l'immobile e, quando c'è, la stanza ---
    let (status, ticket) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/tickets"),
            &owner,
            serde_json::json!({
                "title": "La caldaia perde",
                "description": "Perde dal raccordo.",
                "room_id": room,
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {ticket}");
    // **Anche sulla creazione**, non solo in lettura: una risposta che portasse i
    // nomi a volte sì e a volte no non permetterebbe di distinguere «non
    // risolto» da «archiviato».
    assert_eq!(ticket["property_label"], "Villa Aurora", "corpo: {ticket}");
    assert_eq!(ticket["room_name"], "Camera azzurra", "corpo: {ticket}");
    let ticket_id = ticket["id"].as_str().expect("id").to_owned();

    let (status, detail) = app
        .send(
            "GET",
            &format!("/api/v1/tickets/{ticket_id}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {detail}");
    assert_eq!(detail["property_label"], "Villa Aurora", "corpo: {detail}");

    let (status, page) = app.send("GET", "/api/v1/tickets", Some(&owner), None).await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");
    let listed = page["items"]
        .as_array()
        .expect("elenco")
        .iter()
        .find(|item| item["id"] == serde_json::Value::String(ticket_id.clone()))
        .expect("la segnalazione appena aperta");
    assert_eq!(listed["property_label"], "Villa Aurora", "corpo: {page}");

    // Una transizione risponde con gli stessi campi: è la ragione per cui i nomi
    // si risolvono in `view_of` e non nel solo caso d'uso di lettura.
    let (status, triaged) = app
        .post_auth(
            &format!("/api/v1/tickets/{ticket_id}/triage"),
            &owner,
            serde_json::json!({ "category": "plumbing", "priority": "high" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {triaged}");
    assert_eq!(
        triaged["property_label"], "Villa Aurora",
        "corpo: {triaged}"
    );
    assert_eq!(triaged["room_name"], "Camera azzurra", "corpo: {triaged}");

    // --- l'addebito: il contratto si nomina con la casa e la stanza ---
    let (status, charge) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/charges"),
            &owner,
            serde_json::json!({
                "charge_type": "rent",
                "amount": "400.00",
                "due_date": day(5),
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {charge}");
    assert_eq!(charge["property_label"], "Villa Aurora", "corpo: {charge}");
    assert_eq!(charge["room_name"], "Camera azzurra", "corpo: {charge}");
    let charge_id = charge["id"].as_str().expect("id").to_owned();

    let (status, detail) = app
        .send(
            "GET",
            &format!("/api/v1/charges/{charge_id}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {detail}");
    assert_eq!(detail["property_label"], "Villa Aurora", "corpo: {detail}");
    assert_eq!(detail["room_name"], "Camera azzurra", "corpo: {detail}");

    // --- la bolletta: il **contratto imputato**, e il riaddebito ---
    let bill = register_bill(&app, &owner, &property, Some(&lease), "2026/N/1").await;
    let (status, read) = app
        .send(
            "GET",
            &format!("/api/v1/utility-bills/{bill}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {read}");
    assert_eq!(read["property_label"], "Villa Aurora", "corpo: {read}");
    assert_eq!(read["room_name"], "Camera azzurra", "corpo: {read}");

    // Una bolletta **senza contratto imputato** non ha niente da nominare, e il
    // campo resta assente invece di prendere il nome dell'immobile della bolletta:
    // ciò che il collegamento nomina è il contratto, e lì non ce n'è uno.
    let orphan = register_bill(&app, &owner, &property, None, "2026/N/2").await;
    let (status, read) = app
        .send(
            "GET",
            &format!("/api/v1/utility-bills/{orphan}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {read}");
    assert!(read.get("property_label").is_none(), "corpo: {read}");

    let (status, validated) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/validate"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {validated}");
    assert_eq!(
        validated["property_label"], "Villa Aurora",
        "una transizione risponde senza i nomi: {validated}"
    );

    let (status, recharged) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/recharge"),
            &owner,
            serde_json::json!({ "lease_id": lease, "amount": "90.00", "due_date": day(20) }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {recharged}");
    assert_eq!(
        recharged["charge"]["property_label"], "Villa Aurora",
        "corpo: {recharged}"
    );
    assert_eq!(
        recharged["utility_bill"]["room_name"], "Camera azzurra",
        "corpo: {recharged}"
    );
}

/// **Quanto resta da riaddebitare**, che è la metà di `R33` che riguarda i numeri.
///
/// Riaddebitare una bolletta d'immobile a tre contratti erano tre sottrazioni a
/// mente. Il test fissa le tre regole che quel numero deve rispettare, e la terza è
/// quella che si perde: **un addebito annullato non conta**, perché `void` dice «non
/// doveva essere chiesto» e quella quota torna disponibile.
///
/// Fissa anche ciò che **non** cambia: superare il totale resta possibile. È scritto
/// come deliberato — lo schema non ha il vincolo e uno applicativo cadrebbe fra due
/// riaddebiti concorrenti — e un test che lo vietasse trasformerebbe un aiuto in un
/// cancello.
#[tokio::test]
async fn a_bill_says_how_much_of_it_has_already_been_recharged() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "quanto-resta").await;
    let property = create_property(&app, &owner, "Casa da dividere").await;

    let (_, first_room) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/rooms"),
            &owner,
            serde_json::json!({ "name": "Camera uno", "monthly_rent": "400.00" }),
        )
        .await;
    let first_room = first_room["id"].as_str().expect("id").to_owned();
    let (_, second_room) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/rooms"),
            &owner,
            serde_json::json!({ "name": "Camera due", "monthly_rent": "400.00" }),
        )
        .await;
    let second_room = second_room["id"].as_str().expect("id").to_owned();

    let first = draft_lease(&app, &owner, &property, Some(&first_room), -10, Some(355)).await;
    let second = draft_lease(&app, &owner, &property, Some(&second_room), -10, Some(355)).await;
    let email = common::unique_email("intestatario");
    add_tenant(&app, &owner, &first, "Giulia Neri", Some(&email)).await;

    // **L'intestatario viaggia con i nomi del posto**: è ciò che distingue due
    // contratti sulla stessa stanza in due anni diversi, e senza di lui una tendina
    // di contratti resta un elenco di date.
    let (status, page) = app.send("GET", "/api/v1/leases", Some(&owner), None).await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");
    let listed = page["items"]
        .as_array()
        .expect("elenco")
        .iter()
        .find(|item| item["id"] == serde_json::Value::String(first.clone()))
        .expect("il primo contratto");
    assert_eq!(listed["primary_tenant"], "Giulia Neri", "corpo: {page}");
    assert_eq!(listed["room_name"], "Camera uno", "corpo: {page}");
    // Un contratto senza parti non ha un intestatario, e il campo manca.
    let bare = page["items"]
        .as_array()
        .expect("elenco")
        .iter()
        .find(|item| item["id"] == serde_json::Value::String(second.clone()))
        .expect("il secondo contratto");
    assert!(bare.get("primary_tenant").is_none(), "corpo: {page}");

    // La bolletta è dell'immobile intero: non è imputata a nessun contratto, ed è
    // il caso in cui il residuo serve davvero.
    let bill = register_bill(&app, &owner, &property, None, "2026/Q/1").await;
    let (status, read) = app
        .send(
            "GET",
            &format!("/api/v1/utility-bills/{bill}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {read}");
    assert_eq!(read["total_amount"], "184.20", "corpo: {read}");
    assert_eq!(read["recharged_amount"], "0.00", "mai riaddebitata: {read}");

    let (status, body) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/validate"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {body}");

    let (status, first_recharge) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/recharge"),
            &owner,
            serde_json::json!({ "lease_id": first, "amount": "92.10", "due_date": day(20) }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {first_recharge}");
    assert_eq!(
        first_recharge["utility_bill"]["recharged_amount"], "92.10",
        "corpo: {first_recharge}"
    );
    let first_charge = first_recharge["charge"]["id"]
        .as_str()
        .expect("id")
        .to_owned();

    let (status, second_recharge) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/recharge"),
            &owner,
            serde_json::json!({ "lease_id": second, "amount": "92.10", "due_date": day(20) }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {second_recharge}");
    assert_eq!(
        second_recharge["utility_bill"]["recharged_amount"], "184.20",
        "la somma non torna: {second_recharge}"
    );

    // Annullare il primo addebito **libera la sua quota**: era una richiesta che non
    // doveva essere fatta, e il residuo torna a coprirla.
    let (status, body) = app
        .post_auth(
            &format!("/api/v1/charges/{first_charge}/void"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {body}");

    let (status, read) = app
        .send(
            "GET",
            &format!("/api/v1/utility-bills/{bill}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {read}");
    assert_eq!(
        read["recharged_amount"], "92.10",
        "un addebito annullato non deve contare: {read}"
    );

    // Superare il totale resta ammesso: è scritto come deliberato, e qui il numero
    // lo dice invece di impedirlo.
    let (status, over) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/recharge"),
            &owner,
            serde_json::json!({ "lease_id": first, "amount": "1000.00", "due_date": day(20) }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {over}");
    assert_eq!(
        over["utility_bill"]["recharged_amount"], "1092.10",
        "corpo: {over}"
    );

    // **Per chi abita, il campo non c'è.** La somma si legge sotto RLS e a lui
    // tornerebbe parziale: un numero sbagliato che sembra un totale.
    let tenant_bill = register_bill(&app, &owner, &property, Some(&first), "2026/Q/2").await;
    let (status, activated) = app
        .post_auth(
            &format!("/api/v1/leases/{first}/activate"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {activated}");
    let (_, accept_token) = invite_tenant(&app, &owner, &first, &email).await;
    let (tenant, _tenant_id) = common::register_tenant_from_invitation(
        &app,
        &accept_token,
        "Giulia Neri",
        "cavallo batteria graffetta",
    )
    .await;

    let (status, read) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/utility-bills/{tenant_bill}"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {read}");
    assert!(
        read["recharged_amount"].is_null(),
        "l'inquilino non deve ricevere una somma parziale: {read}"
    );
}

/// **Un immobile archiviato lascia il campo assente, e la riga resta.**
///
/// È la trappola che la sezione 3 aveva già pagato con un `LEFT JOIN`: la query dei
/// nomi filtra `deleted_at IS NULL` come impone la regola 10, quindi con un `INNER`
/// sparirebbero dall'elenco *tutte* le righe di ogni immobile archiviato. Il campo
/// manca, e chi legge dice «Immobile archiviato» invece di mostrare uno spazio.
#[tokio::test]
async fn an_archived_property_leaves_the_name_out_without_hiding_the_charge() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "immobile-archiviato").await;
    let property = create_property(&app, &owner, "Casa che sparirà").await;
    let lease = draft_lease(&app, &owner, &property, None, -10, Some(355)).await;

    let (status, charge) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/charges"),
            &owner,
            serde_json::json!({
                "charge_type": "rent",
                "amount": "500.00",
                "due_date": day(5),
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {charge}");
    assert_eq!(charge["property_label"], "Casa che sparirà");
    let charge_id = charge["id"].as_str().expect("id").to_owned();

    let (status, body) = app
        .send(
            "DELETE",
            &format!("/api/v1/properties/{property}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "corpo: {body}");

    let (status, detail) = app
        .send(
            "GET",
            &format!("/api/v1/charges/{charge_id}"),
            Some(&owner),
            None,
        )
        .await;
    // La riga c'è ancora — è la cosa che l'`INNER JOIN` avrebbe rotto — e il nome
    // non c'è più.
    assert_eq!(status, StatusCode::OK, "corpo: {detail}");
    assert_eq!(detail["id"], charge_id, "corpo: {detail}");
    assert!(
        detail.get("property_label").is_none(),
        "il nome di un immobile archiviato non deve tornare: {detail}"
    );
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
    let (tenant, _tenant_id) = common::register_tenant_from_invitation(
        &app,
        &accept_token,
        "Inquilino Bollette",
        "cavallo batteria graffetta",
    )
    .await;

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
    let (tenant, _tenant_id) = common::register_tenant_from_invitation(
        &app,
        &accept_token,
        "Inquilino Annulli",
        "cavallo batteria graffetta",
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
    let (tenant, _tenant_id) = common::register_tenant_from_invitation(
        &app,
        &accept_token,
        "Inquilino Contabile",
        "cavallo batteria graffetta",
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

// ---------------------------------------------------------------------------
// Pagamenti
// ---------------------------------------------------------------------------

/// Prepara un addebito da 800.00 su un contratto attivo con un inquilino vero, e
/// restituisce `(token del proprietario, org, token dell'inquilino, id addebito)`.
async fn charge_with_a_real_tenant(app: &TestApp, label: &str) -> (String, String, String, String) {
    let (owner, owner_org) = common::register_and_login(app, label).await;
    let property = create_property(app, &owner, &format!("Immobile {label}")).await;
    let lease = draft_lease(app, &owner, &property, None, -10, Some(355)).await;

    let email = common::unique_email(&format!("{label}-inquilino"));
    add_tenant(app, &owner, &lease, "Inquilino Pagante", Some(&email)).await;
    app.post_auth(
        &format!("/api/v1/leases/{lease}/activate"),
        &owner,
        serde_json::json!({}),
    )
    .await;
    let (_, accept_token) = invite_tenant(app, &owner, &lease, &email).await;
    let (tenant, _tenant_id) = common::register_tenant_from_invitation(
        app,
        &accept_token,
        "Inquilino Pagante",
        "cavallo batteria graffetta",
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

    (owner, owner_org, tenant, charge_id)
}

/// **Un incasso non attribuito non fa più leggere un residuo falso.**
///
/// `R29`, misurata: il proprietario registra `150,00` su un addebito da `420,00`
/// **senza** `payer_user_id` — il bonifico da un conto che non si sa a chi
/// ricondurre, che è deliberato — e prima della `0042` l'inquilino leggeva sulla
/// stessa riga `status: partially_paid` con `paid_amount: 0.00` e
/// `outstanding_amount: 420.00`. Pagherebbe due volte.
///
/// La causa è a due piani: `payments_select` nasconde il pagamento non
/// attribuito, e `v_charge_balances` è `security_invoker`, quindi la somma
/// scendeva con la Row Level Security di chi legge. `app.charge_paid_amount`
/// somma oltre la policy e proietta il **solo totale** (ADR-055): l'elenco dei
/// pagamenti continua a non mostrargli quella riga, e il saldo dice il vero.
#[tokio::test]
async fn an_unattributed_payment_still_counts_in_the_balance_the_tenant_reads() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "incasso-anonimo").await;
    let property = create_property(&app, &owner, "Immobile con bonifico anonimo").await;
    let lease = draft_lease(&app, &owner, &property, None, -10, Some(355)).await;

    let email = common::unique_email("incasso-anonimo-inquilino");
    add_tenant(&app, &owner, &lease, "Inquilina Pagante", Some(&email)).await;
    let (status, activated) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {activated}");
    let (_, accept_token) = invite_tenant(&app, &owner, &lease, &email).await;
    let (tenant, _tenant_id) = common::register_tenant_from_invitation(
        &app,
        &accept_token,
        "Inquilina Pagante",
        "cavallo batteria graffetta",
    )
    .await;

    let (status, charge) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/charges"),
            &owner,
            serde_json::json!({
                "charge_type": "rent",
                "amount": "420.00",
                "due_date": day(15),
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {charge}");
    let charge = charge["id"].as_str().expect("id").to_owned();

    // **Senza `payer_user_id`**: è la premessa del caso, non una dimenticanza.
    let (status, payment) = app
        .post_idempotent(
            &format!("/api/v1/charges/{charge}/payments"),
            &owner,
            None,
            Some(&idempotency_key("bonifico-anonimo")),
            serde_json::json!({ "amount": "150.00", "method": "bank_transfer" }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {payment}");

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
    assert_eq!(seen["amount"], "420.00");
    assert_eq!(
        seen["paid_amount"], "150.00",
        "il totale incassato si somma oltre la RLS di chi legge: {seen}"
    );
    assert_eq!(
        seen["outstanding_amount"], "270.00",
        "il residuo che chi abita legge non deve mentire: {seen}"
    );
    assert_eq!(
        seen["status"], "partially_paid",
        "lo stato e il saldo devono dire la stessa cosa: {seen}"
    );

    // **E il perimetro non si è allargato.** La funzione proietta un aggregato:
    // l'elenco dei pagamenti continua a non mostrargli la riga che non è sua, e
    // chi ha pagato resta ignoto.
    let (status, page) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/charges/{charge}/payments"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");
    assert_eq!(
        page["items"].as_array().expect("items").len(),
        0,
        "`payments_select` non si tocca: il pagamento non attribuito resta \
         invisibile a chi non lo ha fatto — {page}"
    );

    // Il proprietario, che le righe le vede tutte, legge lo stesso saldo: la
    // funzione somma lo stesso denaro per entrambi.
    let (status, owner_view) = app
        .send(
            "GET",
            &format!("/api/v1/charges/{charge}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {owner_view}");
    assert_eq!(owner_view["paid_amount"], "150.00");
    assert_eq!(owner_view["outstanding_amount"], "270.00");
}

fn idempotency_key(label: &str) -> String {
    format!("{label}-{}", uuid::Uuid::now_v7())
}

/// Il giro a mano della fetta, dal bordo HTTP: l'inquilino dichiara e il saldo non
/// si muove, lo staff conferma e si muove.
#[tokio::test]
async fn a_declaration_becomes_real_only_when_the_staff_confirms_it() {
    let app = require_app!();
    let (owner, owner_org, tenant, charge) = charge_with_a_real_tenant(&app, "conferma-http").await;

    let (status, declared) = app
        .post_idempotent(
            &format!("/api/v1/charges/{charge}/payment-declarations"),
            &tenant,
            Some(&owner_org),
            Some(&idempotency_key("dichiaro")),
            serde_json::json!({
                "amount": "800.00",
                "method": "bank_transfer",
                "reference": "Bonifico del 3",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {declared}");
    assert_eq!(declared["status"], "pending", "corpo: {declared}");
    assert_eq!(declared["charge_id"], charge);

    // Il saldo non si è mosso: `pending` rende la dichiarazione visibile senza
    // renderla vera.
    let (status, before) = app
        .send(
            "GET",
            &format!("/api/v1/charges/{charge}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {before}");
    assert_eq!(before["paid_amount"], "0.00", "corpo: {before}");
    assert_eq!(before["outstanding_amount"], "800.00");
    assert_eq!(before["status"], "issued");

    // L'inquilino **non** può confermare la propria dichiarazione, e nemmeno
    // ritirarla: `payments_update` è staff soltanto, e non esiste una rotta di
    // ritiro.
    let payment = declared["id"].as_str().expect("id").to_owned();
    let (status, refused) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/payments/{payment}/confirm"),
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {refused}");
    assert_eq!(
        refused["code"], "org.staff_role_required",
        "corpo: {refused}"
    );

    let (status, confirmed) = app
        .post_auth(
            &format!("/api/v1/payments/{payment}/confirm"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {confirmed}");
    assert_eq!(confirmed["status"], "succeeded");
    assert!(confirmed["paid_at"].is_string(), "corpo: {confirmed}");
    assert!(confirmed["reconciled_at"].is_string());

    let (_, after) = app
        .send(
            "GET",
            &format!("/api/v1/charges/{charge}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(after["paid_amount"], "800.00", "corpo: {after}");
    assert_eq!(after["outstanding_amount"], "0.00");
    assert_eq!(
        after["status"], "paid",
        "lo stato dell'addebito si ricalcola dai pagamenti: {after}"
    );
    assert_eq!(after["effective_status"], "paid");

    // L'inquilino vede il proprio pagamento nell'elenco dell'addebito.
    let (status, page) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/charges/{charge}/payments"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");
    assert_eq!(page["items"].as_array().expect("items").len(), 1);
    assert_eq!(page["items"][0]["id"], payment);
}

/// L'header `Idempotency-Key` è **obbligatorio**, e una chiave ripetuta dà `200`
/// con il pagamento già registrato — non `201` e non `409`.
#[tokio::test]
async fn the_idempotency_key_is_required_and_a_repeat_never_takes_the_money_twice() {
    let app = require_app!();
    let (owner, _, _, charge) = charge_with_a_real_tenant(&app, "idempotenza-http").await;
    let body = serde_json::json!({ "amount": "400.00", "method": "bank_transfer" });

    // Assente: `400`, non `422`. Non c'è nulla di malformato nel corpo, manca un
    // pezzo del trasporto.
    let (status, problem) = app
        .post_idempotent(
            &format!("/api/v1/charges/{charge}/payments"),
            &owner,
            None,
            None,
            body.clone(),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "corpo: {problem}");
    assert_eq!(
        problem["code"], "request.missing_header",
        "corpo: {problem}"
    );

    // Malformata: la chiave viene da un header, e uno spazio non sopravvive intatto
    // al primo proxy.
    let (status, problem) = app
        .post_idempotent(
            &format!("/api/v1/charges/{charge}/payments"),
            &owner,
            None,
            Some("chiave con spazi"),
            body.clone(),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {problem}");
    assert_eq!(problem["errors"][0]["field"], "idempotency_key");

    let key = idempotency_key("doppio-invio");
    let (status, first) = app
        .post_idempotent(
            &format!("/api/v1/charges/{charge}/payments"),
            &owner,
            None,
            Some(&key),
            body.clone(),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {first}");
    assert_eq!(first["status"], "succeeded");

    let (status, second) = app
        .post_idempotent(
            &format!("/api/v1/charges/{charge}/payments"),
            &owner,
            None,
            Some(&key),
            body,
        )
        .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "una chiave ripetuta risponde 200 con il pagamento di prima: {second}"
    );
    assert_eq!(second["id"], first["id"]);

    let (_, balance) = app
        .send(
            "GET",
            &format!("/api/v1/charges/{charge}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(
        balance["paid_amount"], "400.00",
        "il saldo è sceso una volta sola: {balance}"
    );
    assert_eq!(balance["outstanding_amount"], "400.00");
    assert_eq!(balance["status"], "partially_paid");
}

/// Una dichiarazione rifiutata lascia il saldo dov'era e perde la data dichiarata.
#[tokio::test]
async fn a_rejected_declaration_leaves_the_balance_alone() {
    let app = require_app!();
    let (owner, owner_org, tenant, charge) = charge_with_a_real_tenant(&app, "rifiuto-http").await;

    let (_, declared) = app
        .post_idempotent(
            &format!("/api/v1/charges/{charge}/payment-declarations"),
            &tenant,
            Some(&owner_org),
            Some(&idempotency_key("da-rifiutare")),
            serde_json::json!({
                "amount": "800.00",
                "method": "cash",
                "paid_at": "2026-08-30T10:00:00Z",
            }),
        )
        .await;
    let payment = declared["id"].as_str().expect("id").to_owned();
    assert_eq!(declared["paid_at"], "2026-08-30T10:00:00Z");

    let (status, rejected) = app
        .post_auth(
            &format!("/api/v1/payments/{payment}/reject"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {rejected}");
    assert_eq!(rejected["status"], "cancelled");
    assert!(
        rejected["paid_at"].is_null(),
        "un pagamento che non è avvenuto non ha una data in cui è avvenuto: {rejected}"
    );

    let (_, balance) = app
        .send(
            "GET",
            &format!("/api/v1/charges/{charge}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(balance["paid_amount"], "0.00", "corpo: {balance}");
    assert_eq!(balance["status"], "issued");

    // `pending` ha due uscite e nessun ingresso di ritorno.
    let (status, problem) = app
        .post_auth(
            &format!("/api/v1/payments/{payment}/confirm"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(
        problem["code"], "payment.invalid_transition",
        "corpo: {problem}"
    );
}

/// Un pagamento verso l'addebito di un'altra organizzazione riceve `404`: sotto la
/// RLS del chiamante quell'addebito non esiste, e il trigger polimorfico da solo lo
/// avrebbe accettato.
#[tokio::test]
async fn a_payment_towards_a_charge_of_another_organization_is_not_found() {
    let app = require_app!();
    let (_, _, _, charge) = charge_with_a_real_tenant(&app, "altra-org-http").await;
    let (stranger, _) = common::register_and_login(&app, "estraneo-pagamenti").await;

    let (status, problem) = app
        .post_idempotent(
            &format!("/api/v1/charges/{charge}/payments"),
            &stranger,
            None,
            Some(&idempotency_key("estraneo")),
            serde_json::json!({ "amount": "10.00", "method": "bank_transfer" }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {problem}");
    assert_eq!(problem["code"], "charge.not_found", "corpo: {problem}");
}

/// Un addebito annullato non accetta incassi, e il suo residuo è `"0.00"` — che è
/// il cambiamento della migration `0024` visto dal filo.
#[tokio::test]
async fn a_voided_charge_shows_no_outstanding_amount_and_takes_no_payment() {
    let app = require_app!();
    let (owner, _, _, charge) = charge_with_a_real_tenant(&app, "annullato-http").await;

    app.post_auth(
        &format!("/api/v1/charges/{charge}/void"),
        &owner,
        serde_json::json!({}),
    )
    .await;

    let (status, voided) = app
        .send(
            "GET",
            &format!("/api/v1/charges/{charge}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {voided}");
    assert_eq!(voided["status"], "void");
    assert_eq!(
        voided["outstanding_amount"], "0.00",
        "prima della 0024 qui c'era ancora l'intero importo: {voided}"
    );
    assert_eq!(voided["amount"], "800.00", "l'importo resta quello che era");

    let (status, problem) = app
        .post_idempotent(
            &format!("/api/v1/charges/{charge}/payments"),
            &owner,
            None,
            Some(&idempotency_key("su-annullato")),
            serde_json::json!({ "amount": "800.00", "method": "bank_transfer" }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {problem}");
    assert_eq!(
        problem["code"], "charge.invalid_transition",
        "corpo: {problem}"
    );
}

/// Il giro che il prodotto non sapeva fare: l'insoluto **scende**.
///
/// Ottocento euro di canone, trecento incassati, e la dashboard passa da 800 a 500
/// mentre lo stato dell'addebito diventa `partially_paid`. Prima di questa fetta
/// l'unico modo di far scendere quel numero era rinunciare ai soldi.
#[tokio::test]
async fn recording_a_partial_payment_lowers_the_outstanding_total() {
    let app = require_app!();
    let (owner, _org, _tenant, charge) = charge_with_a_real_tenant(&app, "insoluto-scende").await;

    let (_, before) = app
        .send("GET", "/api/v1/dashboard", Some(&owner), None)
        .await;
    assert_eq!(before["total_outstanding"], "800.00", "corpo: {before}");
    assert_eq!(
        before["written_off_total"], "0.00",
        "niente è stato stornato: {before}"
    );

    let (status, recorded) = app
        .post_idempotent(
            &format!("/api/v1/charges/{charge}/payments"),
            &owner,
            None,
            Some(&idempotency_key("acconto")),
            serde_json::json!({ "amount": "300.00", "method": "bank_transfer" }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {recorded}");
    assert_eq!(recorded["status"], "succeeded", "corpo: {recorded}");

    let (_, after) = app
        .send("GET", "/api/v1/dashboard", Some(&owner), None)
        .await;
    assert_eq!(
        after["total_outstanding"], "500.00",
        "l'insoluto scende dell'incassato: {after}"
    );

    let (_, balance) = app
        .send(
            "GET",
            &format!("/api/v1/charges/{charge}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(balance["status"], "partially_paid", "corpo: {balance}");
    assert_eq!(balance["paid_amount"], "300.00");
    assert_eq!(balance["outstanding_amount"], "500.00");
}

/// Il nono numero dice il **residuo** e non l'importo.
///
/// È il caso della decisione 3 del piano, e il modo più facile di sbagliarlo:
/// ottocento euro con trecento incassati e poi stornati fanno una perdita di
/// **500**, non di 800. Gli altri trecento sono entrati.
///
/// Il test verifica anche le due cose che rendono il numero possibile: che da
/// `partially_paid` si **storni** (e non si annulli), e che `outstanding_amount`
/// vada a zero — cioè che la colonna che rende il numero necessario non sia la
/// colonna che lo produce.
#[tokio::test]
async fn the_ninth_number_counts_the_remainder_of_a_partially_paid_charge() {
    let app = require_app!();
    let (owner, _org, _tenant, charge) = charge_with_a_real_tenant(&app, "nono-numero").await;

    app.post_idempotent(
        &format!("/api/v1/charges/{charge}/payments"),
        &owner,
        None,
        Some(&idempotency_key("parziale")),
        serde_json::json!({ "amount": "300.00", "method": "bank_transfer" }),
    )
    .await;

    // Annullare no: del denaro è entrato, e dire che la richiesta non è mai
    // esistita sarebbe falso.
    let (status, refused) = app
        .post_auth(
            &format!("/api/v1/charges/{charge}/void"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {refused}");
    assert_eq!(
        refused["code"], "charge.invalid_transition",
        "corpo: {refused}"
    );

    // Stornare sì: l'incassato resta incassato e si rinuncia al resto.
    let (status, written_off) = app
        .post_auth(
            &format!("/api/v1/charges/{charge}/write-off"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "da `partially_paid` si storna: {written_off}"
    );
    assert_eq!(written_off["status"], "written_off", "corpo: {written_off}");

    let (_, balance) = app
        .send(
            "GET",
            &format!("/api/v1/charges/{charge}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(
        balance["outstanding_amount"], "0.00",
        "la 0024 azzera il residuo su `written_off`, ed è la ragione per cui il nono numero \
         non si può calcolare da qui: {balance}"
    );
    assert_eq!(balance["paid_amount"], "300.00", "il pagato resta un fatto");

    let (status, dashboard) = app
        .send("GET", "/api/v1/dashboard", Some(&owner), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {dashboard}");
    assert_eq!(
        dashboard["written_off_total"], "500.00",
        "la perdita è il residuo, non l'importo: 800 meno i 300 incassati. \
         Se qui c'è 800.00, la vista somma `amount`: {dashboard}"
    );
    assert_eq!(
        dashboard["total_outstanding"], "0.00",
        "lo storno esce dall'insoluto, come `void`: {dashboard}"
    );
}

/// Un addebito **annullato** non è una perdita, e non entra nel nono numero.
///
/// È la distinzione per cui il numero esiste: `void` dice «non doveva essere
/// chiesto», `written_off` dice «era dovuto e ci rinunciamo». Se entrambi
/// contassero, il numero non aggiungerebbe niente a `total_outstanding`.
#[tokio::test]
async fn a_voided_charge_is_not_a_loss() {
    let app = require_app!();
    let (owner, _org, _tenant, charge) =
        charge_with_a_real_tenant(&app, "annullato-non-perdita").await;

    app.post_auth(
        &format!("/api/v1/charges/{charge}/void"),
        &owner,
        serde_json::json!({}),
    )
    .await;

    let (_, dashboard) = app
        .send("GET", "/api/v1/dashboard", Some(&owner), None)
        .await;
    assert_eq!(
        dashboard["written_off_total"], "0.00",
        "un annullamento non è una perdita: {dashboard}"
    );
    assert_eq!(dashboard["total_outstanding"], "0.00");
}

/// La coda: il proprietario trova una dichiarazione **senza sapere quale addebito
/// guardare**.
///
/// È la ragione per cui `GET /api/v1/payments` esiste. Senza, l'unica via sarebbe
/// aprire gli addebiti a uno a uno, e una notifica avviserebbe di qualcosa che poi
/// non si trova.
#[tokio::test]
async fn the_queue_finds_a_declaration_without_knowing_its_charge() {
    let app = require_app!();
    let (owner, owner_org, tenant, charge) = charge_with_a_real_tenant(&app, "coda").await;

    // Un incasso già registrato: c'è, ed è `succeeded`. Serve a provare che il
    // filtro sullo stato seleziona e non prende tutto.
    app.post_idempotent(
        &format!("/api/v1/charges/{charge}/payments"),
        &owner,
        None,
        Some(&idempotency_key("gia-incassato")),
        serde_json::json!({ "amount": "100.00", "method": "bank_transfer" }),
    )
    .await;

    let (status, declared) = app
        .post_idempotent(
            &format!("/api/v1/charges/{charge}/payment-declarations"),
            &tenant,
            Some(&owner_org),
            Some(&idempotency_key("dichiarazione-in-coda")),
            serde_json::json!({ "amount": "700.00", "method": "bank_transfer" }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {declared}");
    let declaration = declared["id"].as_str().expect("id").to_owned();

    // La coda, senza nominare l'addebito.
    let (status, queue) = app
        .send("GET", "/api/v1/payments?status=pending", Some(&owner), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {queue}");
    let items = queue["items"].as_array().expect("items");
    assert_eq!(
        items.len(),
        1,
        "solo la dichiarazione, non l'incasso: {queue}"
    );
    assert_eq!(items[0]["id"], declaration, "corpo: {queue}");
    assert_eq!(
        items[0]["charge_id"], charge,
        "la coda dice quale addebito guardare"
    );

    // Senza filtro ci sono entrambi.
    let (_, all) = app
        .send("GET", "/api/v1/payments", Some(&owner), None)
        .await;
    assert_eq!(
        all["items"].as_array().expect("items").len(),
        2,
        "corpo: {all}"
    );

    // Confermata, esce dalla coda: uno stato in cui si entra e da cui non si esce
    // sarebbe peggio di nessuno stato.
    app.post_auth(
        &format!("/api/v1/payments/{declaration}/confirm"),
        &owner,
        serde_json::json!({}),
    )
    .await;
    let (_, empty) = app
        .send("GET", "/api/v1/payments?status=pending", Some(&owner), None)
        .await;
    assert!(
        empty["items"].as_array().expect("items").is_empty(),
        "corpo: {empty}"
    );
}

/// La collezione è ristretta dalla **policy**, non da un ramo sul ruolo.
///
/// A un inquilino risponde `200` con i **propri** pagamenti — non `403`, e non
/// quelli altrui. È `payments_select` a scegliere le righe, e la conseguenza da
/// dire ad alta voce è che un incasso registrato senza `payer_user_id` **non
/// compare** a chi l'ha pagato, pur muovendogli il saldo.
#[tokio::test]
async fn the_payment_collection_shows_a_tenant_only_their_own() {
    let app = require_app!();
    let (owner, owner_org, tenant, charge) =
        charge_with_a_real_tenant(&app, "coda-inquilino").await;

    // Un incasso **non attribuito**: lo staff lo vede, il pagante no.
    app.post_idempotent(
        &format!("/api/v1/charges/{charge}/payments"),
        &owner,
        None,
        Some(&idempotency_key("non-attribuito")),
        serde_json::json!({ "amount": "50.00", "method": "cash" }),
    )
    .await;

    let (status, declared) = app
        .post_idempotent(
            &format!("/api/v1/charges/{charge}/payment-declarations"),
            &tenant,
            Some(&owner_org),
            Some(&idempotency_key("sua-dichiarazione")),
            serde_json::json!({ "amount": "750.00", "method": "bank_transfer" }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {declared}");
    let mine = declared["id"].as_str().expect("id").to_owned();

    let (status, seen) = app
        .send_in_org(
            "GET",
            "/api/v1/payments",
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "la collezione non è staff soltanto: {seen}"
    );
    let items = seen["items"].as_array().expect("items");
    assert_eq!(
        items.len(),
        1,
        "l'incasso non attribuito non è suo, e non lo vede: {seen}"
    );
    assert_eq!(items[0]["id"], mine, "corpo: {seen}");

    // Allo staff si vedono entrambi, così il conteggio di sopra è della policy e
    // non della rotta.
    let (_, both) = app
        .send("GET", "/api/v1/payments", Some(&owner), None)
        .await;
    assert_eq!(
        both["items"].as_array().expect("items").len(),
        2,
        "corpo: {both}"
    );
}

/// Il filtro sul contratto passa dall'addebito, e l'elenco annidato non è cambiato.
///
/// `lease_id` non è una colonna di `app.payments`: il legame è
/// `payable_id → app.charges.lease_id`. E la rotta annidata continua a rispondere
/// `404` su un addebito che non si vede, invece di una pagina vuota — che è il
/// comportamento che rendere `charge_id` opzionale non doveva toccare.
#[tokio::test]
async fn filtering_by_lease_goes_through_the_charge_and_the_nested_route_is_unchanged() {
    let app = require_app!();
    let (owner, _org, _tenant, charge) = charge_with_a_real_tenant(&app, "per-contratto").await;

    let (_, detail) = app
        .send(
            "GET",
            &format!("/api/v1/charges/{charge}"),
            Some(&owner),
            None,
        )
        .await;
    let lease = detail["lease_id"].as_str().expect("lease_id").to_owned();

    app.post_idempotent(
        &format!("/api/v1/charges/{charge}/payments"),
        &owner,
        None,
        Some(&idempotency_key("sul-contratto")),
        serde_json::json!({ "amount": "120.00", "method": "bank_transfer" }),
    )
    .await;

    let (status, filtered) = app
        .send(
            "GET",
            &format!("/api/v1/payments?lease_id={lease}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {filtered}");
    assert_eq!(
        filtered["items"].as_array().expect("items").len(),
        1,
        "corpo: {filtered}"
    );

    // Un altro contratto: zero righe, non un errore.
    let other_property = create_property(&app, &owner, "Immobile senza pagamenti").await;
    let other_lease = draft_lease(&app, &owner, &other_property, None, -5, Some(300)).await;
    let (_, none) = app
        .send(
            "GET",
            &format!("/api/v1/payments?lease_id={other_lease}"),
            Some(&owner),
            None,
        )
        .await;
    assert!(
        none["items"].as_array().expect("items").is_empty(),
        "corpo: {none}"
    );

    // La rotta annidata: invariata, `404` e non una pagina vuota.
    let (stranger, _) = common::register_and_login(&app, "estraneo-elenco").await;
    let (status, problem) = app
        .send(
            "GET",
            &format!("/api/v1/charges/{charge}/payments"),
            Some(&stranger),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {problem}");
    assert_eq!(problem["code"], "charge.not_found", "corpo: {problem}");

    // E al proprietario risponde con l'elenco di quell'addebito soltanto.
    let (status, nested) = app
        .send(
            "GET",
            &format!("/api/v1/charges/{charge}/payments"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {nested}");
    assert_eq!(
        nested["items"].as_array().expect("items").len(),
        1,
        "corpo: {nested}"
    );
}

// ---------------------------------------------------------------------------
// Spese co-living
// ---------------------------------------------------------------------------

/// Registra un inquilino, lo invita sul contratto e restituisce
/// `(access_token, user_id)`.
///
/// Serve l'identificatore dell'utente e non solo il token: i corpi di questo
/// modulo nominano le persone — chi ha anticipato, a chi si è pagato — e senza
/// `user_id` non si scriverebbero.
async fn coliving_tenant(app: &TestApp, owner: &str, lease: &str, label: &str) -> (String, String) {
    let email = common::unique_email(label);
    let (_, accept_token) = invite_tenant(app, owner, lease, &email).await;
    // `register_tenant_from_invitation` restituisce già (access_token, user_id):
    // prima di ADR-053 l'utente si otteneva registrandosi come proprietario e
    // poi accettando, e il `user_id` andava riletto dalle parti del contratto
    // perché `POST /auth/accept-invitation` non lo restituisce.
    common::register_tenant_from_invitation(app, &accept_token, label, "cavallo batteria graffetta")
        .await
}

/// Il giro a mano del modulo, contro il router vero.
///
/// Un inquilino crea un gruppo e **si ritrova dentro**; aggiunge due coinquilini;
/// registra una spesa da 100,00 divisa in tre e i saldi dicono +66,67 / −33,33 /
/// −33,34; uno dichiara di aver saldato e **il saldo non si muove** finché il
/// creditore non conferma.
#[tokio::test]
async fn a_coliving_group_splits_a_hundred_euros_and_closes_the_account() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "proprietario-coliving").await;
    let property = create_property(&app, &owner, "Casa condivisa").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    let (first, first_id) = coliving_tenant(&app, &owner, &lease, "coinquilino-uno").await;
    let (second, second_id) = coliving_tenant(&app, &owner, &lease, "coinquilino-due").await;
    let (third, third_id) = coliving_tenant(&app, &owner, &lease, "coinquilino-tre").await;

    // **Chi crea si ritrova dentro**: senza la migration `0026` il gruppo
    // nascerebbe vuoto e resterebbe vuoto.
    let (status, group) = app
        .send_in_org(
            "POST",
            "/api/v1/expense-groups",
            Some(&first),
            Some(&owner_org),
            Some(serde_json::json!({
                "name": "Casa di via Test",
                "property_id": property,
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {group}");
    assert_eq!(group["kind"], "coliving");
    assert_eq!(group["default_split_method"], "equal");
    assert_eq!(group["currency"], "EUR");
    assert_eq!(group["is_archived"], false);
    let members = group["members"].as_array().expect("composizione presente");
    assert_eq!(members.len(), 1, "il gruppo non nasce vuoto");
    assert_eq!(members[0]["user_id"], first_id);
    assert_eq!(members[0]["weight"], "1.000");
    let group_id = group["id"].as_str().expect("id").to_owned();

    // I due coinquilini.
    for user in [&second_id, &third_id] {
        let (status, added) = app
            .send_in_org(
                "POST",
                &format!("/api/v1/expense-groups/{group_id}/members"),
                Some(&first),
                Some(&owner_org),
                Some(serde_json::json!({ "user_id": user })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "corpo: {added}");
        assert_eq!(added["left_at"], serde_json::Value::Null);
    }

    // Cento euro tondi divisi in tre: il centesimo di resto va al primo in ordine
    // di `user_id`, e **la somma delle quote è esattamente l'importo**.
    let (status, expense) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expense-groups/{group_id}/expenses"),
            Some(&first),
            Some(&owner_org),
            Some(serde_json::json!({
                "description": "Spesa della settimana",
                "amount": "100.00",
                "category": "groceries",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {expense}");
    assert_eq!(expense["amount"], "100.00");
    assert_eq!(expense["paid_by_user_id"], first_id);
    let shares = expense["shares"].as_array().expect("quote presenti");
    assert_eq!(shares.len(), 3);
    let total: rust_decimal::Decimal = shares
        .iter()
        .map(|share| {
            share["share_amount"]
                .as_str()
                .expect("importo")
                .parse::<rust_decimal::Decimal>()
                .expect("decimale")
        })
        .sum();
    assert_eq!(
        total,
        "100.00".parse::<rust_decimal::Decimal>().expect("decimale"),
        "le quote devono sommare all'importo: il trigger non ammette altro"
    );
    // Su una divisione in parti uguali il peso **non** viaggia con le quote.
    assert!(
        shares.iter().all(|share| share["share_weight"].is_null()),
        "il peso non ha partecipato al calcolo: scriverlo direbbe il contrario"
    );

    // I saldi: chi ha anticipato è in credito, gli altri due in debito, e la
    // somma fa zero.
    let balance_of = |body: &serde_json::Value, user: &str| -> rust_decimal::Decimal {
        body["items"]
            .as_array()
            .expect("saldi")
            .iter()
            .find(|item| item["user_id"] == user)
            .map(|item| {
                item["net_balance"]
                    .as_str()
                    .expect("saldo")
                    .parse::<rust_decimal::Decimal>()
                    .expect("decimale")
            })
            .unwrap_or_else(|| panic!("nessun saldo per {user}"))
    };

    let (status, balances) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/expense-groups/{group_id}/balances"),
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {balances}");
    let creditor = balance_of(&balances, &first_id);
    assert!(creditor > rust_decimal::Decimal::ZERO);
    let sum: rust_decimal::Decimal = balances["items"]
        .as_array()
        .expect("saldi")
        .iter()
        .map(|item| {
            item["net_balance"]
                .as_str()
                .expect("saldo")
                .parse::<rust_decimal::Decimal>()
                .expect("decimale")
        })
        .sum();
    assert_eq!(
        sum,
        rust_decimal::Decimal::ZERO,
        "la somma dei saldi di un gruppo fa zero"
    );

    // Il secondo dichiara di aver saldato. **Il saldo non si muove.**
    let debt = balance_of(&balances, &second_id).abs().to_string();
    let (status, settlement) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expense-groups/{group_id}/settlements"),
            Some(&second),
            Some(&owner_org),
            Some(serde_json::json!({
                "to_user_id": first_id,
                "amount": debt,
                "method": "bank_transfer",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {settlement}");
    assert_eq!(settlement["status"], "pending");
    assert_eq!(settlement["from_user_id"], second_id);
    assert_eq!(settlement["confirmed_at"], serde_json::Value::Null);
    let settlement_id = settlement["id"].as_str().expect("id").to_owned();

    let (_, pending) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/expense-groups/{group_id}/balances"),
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(
        balance_of(&pending, &first_id),
        creditor,
        "una dichiarazione pendente non muove i saldi: la vista somma i soli `confirmed`"
    );

    // **Chi paga non conferma a sé stesso**: conferma chi riceve.
    let (status, refused) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/settlements/{settlement_id}/confirm"),
            Some(&second),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {refused}");
    assert_eq!(refused["code"], "settlement.not_addressed_to_you");

    // Il creditore trova la dichiarazione da confermare filtrando per stato.
    let (status, queue) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/expense-groups/{group_id}/settlements?status=pending"),
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {queue}");
    assert_eq!(queue["items"].as_array().expect("elenco").len(), 1);

    let (status, confirmed) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/settlements/{settlement_id}/confirm"),
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {confirmed}");
    assert_eq!(confirmed["status"], "confirmed");
    assert_eq!(confirmed["confirmed_by"], first_id);

    let (_, after) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/expense-groups/{group_id}/balances"),
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(
        balance_of(&after, &second_id),
        rust_decimal::Decimal::ZERO,
        "chi ha saldato è in pari"
    );

    // **Il difetto della vista, dal bordo HTTP**: chi esce con un conto aperto
    // resta nei saldi, chi esce in pari sparisce.
    for (token, user) in [(&third, &third_id), (&second, &second_id)] {
        let (status, left) = app
            .send_in_org(
                "DELETE",
                &format!("/api/v1/expense-groups/{group_id}/members/{user}"),
                Some(token),
                Some(&owner_org),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK, "corpo: {left}");
        assert!(
            !left["left_at"].is_null(),
            "uscire valorizza `left_at` e non cancella la riga"
        );
    }

    let (_, departed) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/expense-groups/{group_id}/balances"),
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    let items = departed["items"].as_array().expect("saldi");
    assert_eq!(
        items.len(),
        2,
        "il debitore uscito resta, quello in pari sparisce: prima della 0027 sparivano entrambi"
    );
    assert!(balance_of(&departed, &third_id) < rust_decimal::Decimal::ZERO);
    assert!(
        !items.iter().any(|item| item["user_id"] == second_id),
        "chi esce in pari non ha niente da dire e non compare"
    );

    // Il conto si chiude anche con dei conti aperti: chi se n'è andato senza
    // pagare non tiene il gruppo in ostaggio.
    let (status, archived) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expense-groups/{group_id}/archive"),
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {archived}");
    assert_eq!(archived["is_archived"], true);

    // E un gruppo archiviato non accetta più spese, ma i saldi restano leggibili.
    let (status, refused) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expense-groups/{group_id}/expenses"),
            Some(&first),
            Some(&owner_org),
            Some(serde_json::json!({ "description": "Tardi", "amount": "1.00" })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {refused}");
    assert_eq!(refused["code"], "expense_group.archived");

    let (status, _) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/expense-groups/{group_id}/balances"),
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "un conto chiuso si consulta");
}

/// Annullare una spesa **fa scendere i saldi**, anche dopo un regolamento.
///
/// È il criterio di fine della sezione 7 per il suo unico passo con backend: la
/// migration `0028` concedeva `deleted_at` «per l'annullamento» e nessuna rotta
/// lo faceva. Il giro: cento euro divisi in tre, il secondo salda e il primo
/// conferma, poi la spesa viene annullata — e il saldo del primo scende da
/// +33,34 a **−33,33**, perché il regolamento resta e la spesa no. Un regolamento
/// salda un saldo, non una spesa.
///
/// Intorno, i tre rifiuti: il terzo coinquilino — che non l'ha registrata né
/// anticipata — riceve `403 expense.not_yours`; il secondo annullamento trova
/// `404`, non `409`, perché la riga non si legge più; e la spesa è sparita
/// dall'elenco.
#[tokio::test]
async fn cancelling_an_expense_lowers_the_balances_even_after_a_settlement() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "proprietario-annulla").await;
    let property = create_property(&app, &owner, "Casa che annulla").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    let (first, first_id) = coliving_tenant(&app, &owner, &lease, "annulla-uno").await;
    let (second, second_id) = coliving_tenant(&app, &owner, &lease, "annulla-due").await;
    let (third, third_id) = coliving_tenant(&app, &owner, &lease, "annulla-tre").await;

    let (status, group) = app
        .send_in_org(
            "POST",
            "/api/v1/expense-groups",
            Some(&first),
            Some(&owner_org),
            Some(serde_json::json!({ "name": "Casa di via Annulla", "property_id": property })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {group}");
    let group_id = group["id"].as_str().expect("id").to_owned();
    for user in [&second_id, &third_id] {
        let (status, added) = app
            .send_in_org(
                "POST",
                &format!("/api/v1/expense-groups/{group_id}/members"),
                Some(&first),
                Some(&owner_org),
                Some(serde_json::json!({ "user_id": user })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "corpo: {added}");
    }

    let (status, expense) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expense-groups/{group_id}/expenses"),
            Some(&first),
            Some(&owner_org),
            Some(serde_json::json!({
                "description": "Spesa sbagliata",
                "amount": "100.00",
                "category": "groceries",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {expense}");
    let expense_id = expense["id"].as_str().expect("id").to_owned();

    let balance_of = |body: &serde_json::Value, user: &str| -> rust_decimal::Decimal {
        body["items"]
            .as_array()
            .expect("saldi")
            .iter()
            .find(|item| item["user_id"] == user)
            .map(|item| {
                item["net_balance"]
                    .as_str()
                    .expect("saldo")
                    .parse::<rust_decimal::Decimal>()
                    .expect("decimale")
            })
            .unwrap_or_else(|| panic!("nessun saldo per {user}"))
    };
    let balances = |token: &str| {
        let path = format!("/api/v1/expense-groups/{group_id}/balances");
        let token = token.to_owned();
        let owner_org = owner_org.clone();
        let app = &app;
        async move {
            let (status, body) = app
                .send_in_org("GET", &path, Some(&token), Some(&owner_org), None)
                .await;
            assert_eq!(status, StatusCode::OK, "corpo: {body}");
            body
        }
    };

    // Il secondo salda la propria quota e il primo conferma: il saldo del primo
    // scende da +66,67 a +33,34, e il secondo è in pari.
    let before = balances(&first).await;
    let debt = balance_of(&before, &second_id).abs().to_string();
    let (status, settlement) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expense-groups/{group_id}/settlements"),
            Some(&second),
            Some(&owner_org),
            Some(serde_json::json!({
                "to_user_id": first_id,
                "amount": debt,
                "method": "bank_transfer",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {settlement}");
    let settlement_id = settlement["id"].as_str().expect("id").to_owned();
    let (status, confirmed) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/settlements/{settlement_id}/confirm"),
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {confirmed}");

    let settled = balances(&first).await;
    let creditor_before = balance_of(&settled, &first_id);
    assert!(creditor_before > rust_decimal::Decimal::ZERO);
    assert_eq!(
        balance_of(&settled, &second_id),
        rust_decimal::Decimal::ZERO
    );

    // **Il terzo non può**: non l'ha registrata né anticipata, e non è staff. Un
    // `403` con il codice che dice chi può, non un `404`: la spesa lui la vede.
    let (status, refused) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expenses/{expense_id}/cancel"),
            Some(&third),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {refused}");
    assert_eq!(refused["code"], "expense.not_yours");

    // Chi l'ha registrata annulla.
    let (status, cancelled) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expenses/{expense_id}/cancel"),
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {cancelled}");
    assert_eq!(cancelled["id"], expense_id);

    // **I saldi scendono.** La spesa è uscita e il regolamento è rimasto: il
    // primo, che ha ricevuto 33,33 per una spesa che non c'è più, è ora in
    // **debito** di quella cifra verso il secondo, e la somma fa ancora zero.
    let after = balances(&first).await;
    let creditor_after = balance_of(&after, &first_id);
    assert!(
        creditor_after < creditor_before,
        "il saldo di chi aveva anticipato deve scendere: prima {creditor_before}, dopo {creditor_after}"
    );
    assert!(creditor_after < rust_decimal::Decimal::ZERO);
    assert_eq!(balance_of(&after, &second_id), -creditor_after);
    assert_eq!(balance_of(&after, &third_id), rust_decimal::Decimal::ZERO);
    let sum: rust_decimal::Decimal = after["items"]
        .as_array()
        .expect("saldi")
        .iter()
        .map(|item| {
            item["net_balance"]
                .as_str()
                .expect("saldo")
                .parse::<rust_decimal::Decimal>()
                .expect("decimale")
        })
        .sum();
    assert_eq!(
        sum,
        rust_decimal::Decimal::ZERO,
        "la somma dei saldi fa ancora zero"
    );

    // La spesa non si legge più: `404` sul dettaglio, sparita dall'elenco, e un
    // secondo annullamento trova lo stesso `404` — non un `409`. È la forma del
    // soft delete, e il client la classifica come «era già così».
    let (status, gone) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/expenses/{expense_id}"),
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {gone}");
    assert_eq!(gone["code"], "expense.not_found");
    let (status, again) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expenses/{expense_id}/cancel"),
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {again}");
    let (status, list) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/expense-groups/{group_id}/expenses"),
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {list}");
    assert!(
        list["items"].as_array().expect("elenco").is_empty(),
        "una spesa annullata non è in nessun elenco"
    );

    // Lo staff può, su una spesa che non ha né registrato né anticipato: il
    // proprietario annulla quella registrata dal terzo.
    let (status, other) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expense-groups/{group_id}/expenses"),
            Some(&third),
            Some(&owner_org),
            Some(serde_json::json!({ "description": "Bombola", "amount": "30.00" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {other}");
    let other_id = other["id"].as_str().expect("id").to_owned();
    let (status, by_staff) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expenses/{other_id}/cancel"),
            Some(&owner),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {by_staff}");

    // E su un conto chiuso non si annulla: è una scrittura sul conto.
    let (status, third_expense) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expense-groups/{group_id}/expenses"),
            Some(&first),
            Some(&owner_org),
            Some(serde_json::json!({ "description": "Ultima", "amount": "3.00" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {third_expense}");
    let third_expense_id = third_expense["id"].as_str().expect("id").to_owned();
    let (status, archived) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expense-groups/{group_id}/archive"),
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {archived}");
    let (status, closed) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expenses/{third_expense_id}/cancel"),
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {closed}");
    assert_eq!(closed["code"], "expense_group.archived");
}

/// La ripartizione pesata dal bordo HTTP, e il peso che viaggia con le quote.
#[tokio::test]
async fn a_weighted_split_carries_the_weight_onto_the_wire() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "proprietario-pesata").await;
    let property = create_property(&app, &owner, "Casa con la doppia").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    let (first, _) = coliving_tenant(&app, &owner, &lease, "pesata-uno").await;
    let (_, second_id) = coliving_tenant(&app, &owner, &lease, "pesata-due").await;

    let (status, group) = app
        .send_in_org(
            "POST",
            "/api/v1/expense-groups",
            Some(&first),
            Some(&owner_org),
            Some(serde_json::json!({
                "name": "Con la doppia",
                "default_split_method": "weighted",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {group}");
    let group_id = group["id"].as_str().expect("id").to_owned();

    // Il secondo occupa la doppia: peso 2, e paga il doppio.
    let (status, added) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expense-groups/{group_id}/members"),
            Some(&first),
            Some(&owner_org),
            Some(serde_json::json!({ "user_id": second_id, "weight": "2" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {added}");
    assert_eq!(
        added["weight"], "2.000",
        "il peso esce con tre decimali, come `NUMERIC(6,3)`"
    );

    let (status, expense) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expense-groups/{group_id}/expenses"),
            Some(&first),
            Some(&owner_org),
            Some(serde_json::json!({
                "description": "Bolletta della luce",
                "amount": "30.00",
                "category": "utilities",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {expense}");
    assert_eq!(expense["split_method"], "weighted");

    let shares = expense["shares"].as_array().expect("quote");
    let of = |user: &str| -> &str {
        shares
            .iter()
            .find(|share| share["user_id"] == user)
            .map(|share| share["share_amount"].as_str().expect("importo"))
            .expect("quota presente")
    };
    assert_eq!(of(&second_id), "20.00", "la doppia paga il doppio");
    assert!(
        shares.iter().all(|share| !share["share_weight"].is_null()),
        "su una ripartizione pesata il peso ha partecipato al calcolo e viaggia"
    );
}

/// I tre metodi fuori perimetro danno `422` invece di una ripartizione
/// approssimata, e un peso malformato pure.
#[tokio::test]
async fn the_unsupported_split_methods_are_refused_with_a_field_error() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "proprietario-metodi").await;
    let property = create_property(&app, &owner, "Casa dei metodi").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;
    let (tenant, _) = coliving_tenant(&app, &owner, &lease, "metodi-uno").await;

    let (status, group) = app
        .send_in_org(
            "POST",
            "/api/v1/expense-groups",
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({ "name": "Metodi" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {group}");
    let group_id = group["id"].as_str().expect("id").to_owned();

    for method in ["percentage", "exact", "by_usage"] {
        let (status, refused) = app
            .send_in_org(
                "POST",
                &format!("/api/v1/expense-groups/{group_id}/expenses"),
                Some(&tenant),
                Some(&owner_org),
                Some(serde_json::json!({
                    "description": "Spesa",
                    "amount": "10.00",
                    "split_method": method,
                })),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "`{method}` doveva essere rifiutato, corpo: {refused}"
        );
        assert_eq!(refused["code"], "validation.failed");
        assert_eq!(refused["errors"][0]["field"], "split_method");
    }

    // Un peso con quattro decimali non entra in `NUMERIC(6,3)`, e il rifiuto
    // arriva dal bordo invece che dal database.
    let (status, refused) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expense-groups/{group_id}/members"),
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({
                "user_id": uuid::Uuid::now_v7().to_string(),
                "weight": "1.2345",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {refused}");
    assert_eq!(refused["errors"][0]["field"], "weight");
}

/// Un gruppo di un'altra organizzazione dà `404` da ogni porta.
#[tokio::test]
async fn an_expense_group_of_another_organization_is_invisible() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "proprietario-iso-a").await;
    let property = create_property(&app, &owner, "Casa mia").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;
    let (tenant, _) = coliving_tenant(&app, &owner, &lease, "iso-uno").await;

    let (_, group) = app
        .send_in_org(
            "POST",
            "/api/v1/expense-groups",
            Some(&tenant),
            Some(&owner_org),
            Some(serde_json::json!({ "name": "Casa mia" })),
        )
        .await;
    let group_id = group["id"].as_str().expect("id").to_owned();

    // Un estraneo, con la propria organizzazione.
    let (stranger, _) = common::register_and_login(&app, "estraneo-coliving").await;

    for (method, path) in [
        ("GET", format!("/api/v1/expense-groups/{group_id}")),
        ("GET", format!("/api/v1/expense-groups/{group_id}/balances")),
        ("GET", format!("/api/v1/expense-groups/{group_id}/expenses")),
    ] {
        let (status, body) = app.send(method, &path, Some(&stranger), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}, corpo: {body}");
        assert_eq!(body["code"], "expense_group.not_found");
    }

    let (status, body) = app
        .post_auth(
            &format!("/api/v1/expense-groups/{group_id}/expenses"),
            &stranger,
            serde_json::json!({ "description": "Spesa in casa d'altri", "amount": "10.00" }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {body}");
    assert_eq!(body["code"], "expense_group.not_found");
}

// ---------------------------------------------------------------------------
// I coinquilini si vedono
// ---------------------------------------------------------------------------

/// Un residente **per stanza**: stanza, contratto proprio, invito, accettazione,
/// attivazione.
///
/// Restituisce il solo token, e l'omissione è il punto: `user_id` non lo si
/// restituisce perché il test deve dimostrare che si trova **dall'elenco dei
/// residenti**. Il [`coliving_tenant`] di questo stesso file lo legge invece dal
/// contratto passando dal proprietario, ed è la via che oggi un client dovrebbe
/// percorrere in un co-living a contratto unico.
async fn resident_of_room(
    app: &TestApp,
    owner: &str,
    property: &str,
    room_name: &str,
    label: &str,
) -> String {
    let (status, room) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/rooms"),
            owner,
            serde_json::json!({ "name": room_name }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {room}");
    let room_id = room["id"].as_str().expect("id").to_owned();

    // Nessuna parte aggiunta a mano: la crea l'invito, con la quota residua — cioè
    // 100 su un contratto vuoto — e `is_primary`.
    let lease = draft_lease(app, owner, property, Some(&room_id), -30, Some(335)).await;

    let email = common::unique_email(label);
    let (_, accept_token) = invite_tenant(app, owner, &lease, &email).await;
    let (token, _tenant_id) = common::register_tenant_from_invitation(
        app,
        &accept_token,
        label,
        "cavallo batteria graffetta",
    )
    .await;

    let (status, activated) = app
        .send(
            "POST",
            &format!("/api/v1/leases/{lease}/activate"),
            Some(owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {activated}");

    token
}

/// **Il giro che ieri non si poteva fare**, contro il router vero.
///
/// Il proprietario crea due stanze e due contratti **per stanza** e invita X e Y;
/// entrambi accettano. Poi X, **da solo**: elenca i residenti dell'immobile, ci
/// trova Y con il suo identificatore, crea un gruppo di spesa e ce lo aggiunge —
/// senza che il proprietario gli passi niente a mano.
///
/// È il buco che la fetta co-living aveva lasciato: in un co-living per stanza
/// ognuno ha il proprio contratto, quindi `can_access_lease` non mostra a X la
/// parte contrattuale di Y, e `user_id` non compariva da nessuna parte.
#[tokio::test]
async fn a_housemate_finds_the_other_and_adds_them_to_the_group_alone() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "proprietario-coinquilini").await;
    let property = create_property(&app, &owner, "Casa per stanze").await;

    let first = resident_of_room(&app, &owner, &property, "Camera uno", "coinquilino-x").await;
    let second = resident_of_room(&app, &owner, &property, "Camera due", "coinquilino-y").await;

    // X elenca i residenti di casa propria. Nessun passaggio dal proprietario.
    let (status, page) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/properties/{property}/residents"),
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {page}");
    let items = page["items"].as_array().expect("items");
    assert_eq!(items.len(), 2, "X vede sé stesso e Y: {page}");

    // **L'email non c'è, e non costa niente non metterla**: la risposta viene da
    // una funzione che proietta, non da una riga resa leggibile.
    for item in items {
        assert!(
            item.get("email").is_none(),
            "l'elenco dei residenti non porta l'email: {item}"
        );
        assert!(item["room_name"].is_string(), "la stanza c'è: {item}");
    }

    // X non è il proprietario, e il proprietario non è un residente.
    let names: Vec<&str> = items
        .iter()
        .map(|item| item["full_name"].as_str().expect("nome"))
        .collect();
    assert!(
        !names.iter().any(|name| name.contains("Proprietario")),
        "chi possiede la casa non ci abita: {names:?}"
    );

    // Il secondo residente, con il suo identificatore: è ciò che serviva.
    let (status, mine) = app
        .send_in_org(
            "GET",
            "/api/v1/leases",
            Some(&first),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {mine}");
    assert_eq!(
        mine["items"].as_array().expect("items").len(),
        1,
        "X vede **un solo** contratto, il proprio: è l'assert che dice che nessun \
         canone altrui è diventato leggibile. Allargare `leases_select` avrebbe \
         fatto funzionare l'elenco a quel prezzo: {mine}"
    );
    let own_lease_party = mine["items"][0]["id"].as_str().expect("id").to_owned();

    let housemate = items
        .iter()
        .find(|item| {
            item["user_id"].is_string() && item["room_name"] == serde_json::json!("Camera due")
        })
        .and_then(|item| item["user_id"].as_str())
        .expect("il coinquilino ha un identificatore: ha accettato l'invito")
        .to_owned();

    // X crea il gruppo e ci mette Y, da solo.
    let (status, group) = app
        .send_in_org(
            "POST",
            "/api/v1/expense-groups",
            Some(&first),
            Some(&owner_org),
            Some(serde_json::json!({
                "name": "Bollette di casa",
                "property_id": property,
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {group}");
    let group_id = group["id"].as_str().expect("id").to_owned();

    let (status, added) = app
        .send_in_org(
            "POST",
            &format!("/api/v1/expense-groups/{group_id}/members"),
            Some(&first),
            Some(&owner_org),
            Some(serde_json::json!({ "user_id": housemate })),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "il giro si chiude qui, e ieri si fermava un passo prima: {added}"
    );

    let (status, detail) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/expense-groups/{group_id}"),
            Some(&second),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {detail}");
    assert_eq!(
        detail["members"].as_array().expect("membri").len(),
        2,
        "e Y si ritrova nel gruppo: {detail}"
    );
    assert!(
        !own_lease_party.is_empty(),
        "il contratto di X esiste ed è uno solo"
    );
}

/// L'immobile di un'altra organizzazione risponde `404`, non un elenco vuoto.
///
/// Un elenco vuoto direbbe «questa casa esiste e non ci abita nessuno», che è uno
/// stato legittimo di un immobile sfitto: qui sarebbe una conferma gratuita che
/// l'identificatore indovinato esiste.
#[tokio::test]
async fn the_residents_of_someone_elses_property_are_not_found() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "residenti-iso").await;
    let property = create_property(&app, &owner, "Casa altrui").await;
    let (stranger, _) = common::register_and_login(&app, "estraneo-residenti").await;

    let (status, problem) = app
        .send(
            "GET",
            &format!("/api/v1/properties/{property}/residents"),
            Some(&stranger),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {problem}");
    assert_eq!(problem["code"], "property.not_found");
}

// ---------------------------------------------------------------------------
// Lettura automatica delle bollette
// ---------------------------------------------------------------------------

/// Una bolletta in chiaro che l'adattatore deterministico legge per intero.
const BILL_SCAN: &[u8] = b"Fornitore: Enel Energia\nScadenza: 31/08/2026\nTotale: 184,20\n";

/// Fa girare il ciclo del worker finché il job della bolletta non è concluso.
///
/// Il worker **non è montato nel router** — non è un bordo HTTP — quindi il
/// contratto lo deve costruire da sé. Senza, questi test verificherebbero solo
/// l'accodamento, cioè metà del percorso: è la lezione della fetta dei pagamenti,
/// dove un test che passava da uno strato non provava niente sullo strato sotto.
async fn run_ocr_worker(app: &TestApp) {
    use propflow_application::use_cases::ai::{ProcessAiJob, Processed};
    use propflow_domain::ai::AiTask;

    let ocr = ProcessAiJob::new(
        std::sync::Arc::new(propflow_infrastructure::db::PgWorkerUnitOfWorkFactory::new(
            app.pools.privileged().clone(),
        )),
        std::sync::Arc::clone(&app.storage)
            as std::sync::Arc<dyn propflow_application::ports::ObjectStorage>,
        std::sync::Arc::new(propflow_infrastructure::DeterministicOcrProvider),
        std::sync::Arc::new(propflow_infrastructure::SystemClock),
    );
    // Più giri: la coda è condivisa fra i test che girano in parallelo, e il primo
    // giro può prendere il job di un altro. È il comportamento corretto del worker
    // — serve tutte le organizzazioni — non un difetto da aggirare.
    for _ in 0..64 {
        if ocr.execute(AiTask::BillOcr).await.expect("ciclo") == Processed::Nothing {
            return;
        }
    }
}

/// Il giro completo visto **da fuori**: si carica una scansione, il worker legge,
/// e l'API racconta cosa è successo.
#[tokio::test]
async fn a_scan_makes_the_bill_reviewable_and_the_api_says_so() {
    let app = require_app!();
    let _gate = common::lock_ocr_queue(&app.pools).await;
    let (owner, _) = common::register_and_login(&app, "ocr-contratto").await;
    let property = create_property(&app, &owner, "Immobile con OCR").await;
    let bill = register_bill(&app, &owner, &property, None, "2026/HTTP/0001").await;

    // Prima di caricare qualunque cosa: la rotta risponde, e dice `null`.
    let (status, before) = app
        .send(
            "GET",
            &format!("/api/v1/utility-bills/{bill}/ocr"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {before}");
    assert_eq!(before["utility_bill_id"], bill, "corpo: {before}");
    assert_eq!(before["status"], "draft", "corpo: {before}");
    assert!(before["job"].is_null(), "corpo: {before}");
    assert!(before["extracted"].is_null(), "corpo: {before}");
    assert!(before["confidence"].is_null(), "corpo: {before}");

    let (status, uploaded) = app
        .upload(common::Upload {
            path: &format!("/api/v1/utility-bills/{bill}/documents"),
            token: &owner,
            org: None,
            filename: "bolletta-agosto.txt",
            content_type: "text/plain",
            content: BILL_SCAN,
            kind: Some("utility_bill"),
        })
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {uploaded}");

    // Il caricamento ha accodato, e **non ha toccato la bolletta**: l'esito di un
    // caricamento non dipende dalla disponibilità del fornitore.
    let (status, queued) = app
        .send(
            "GET",
            &format!("/api/v1/utility-bills/{bill}/ocr"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {queued}");
    assert_eq!(queued["job"]["status"], "queued", "corpo: {queued}");
    assert_eq!(queued["job"]["task"], "bill_ocr", "corpo: {queued}");
    assert_eq!(queued["job"]["attempts"], 0, "corpo: {queued}");
    assert_eq!(queued["status"], "draft", "corpo: {queued}");

    run_ocr_worker(&app).await;

    let (status, done) = app
        .send(
            "GET",
            &format!("/api/v1/utility-bills/{bill}/ocr"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {done}");
    assert_eq!(done["job"]["status"], "succeeded", "corpo: {done}");
    assert_eq!(done["job"]["attempts"], 1, "corpo: {done}");
    assert_eq!(done["status"], "pending_review", "corpo: {done}");
    assert!(done["job"]["error_code"].is_null(), "corpo: {done}");

    let extracted = done["extracted"].as_array().expect("i campi proposti");
    assert_eq!(extracted.len(), 3, "corpo: {done}");
    let total = extracted
        .iter()
        .find(|field| field["field"] == "total_amount")
        .expect("il totale è fra i campi proposti");
    assert_eq!(total["type"], "amount", "corpo: {done}");
    // **Stringa e non numero**: un decimale che passa da un float binario torna
    // con cifre che nessuno ha scritto.
    assert_eq!(total["value"], "184.20", "corpo: {done}");
    assert!(total["confidence"].is_string(), "corpo: {done}");

    // **La bolletta non è cambiata**: la proposta si legge accanto ai numeri
    // digitati, non al posto loro.
    let (status, detail) = app
        .send(
            "GET",
            &format!("/api/v1/utility-bills/{bill}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {detail}");
    assert_eq!(detail["status"], "pending_review", "corpo: {detail}");
    assert_eq!(detail["total_amount"], "184.20", "corpo: {detail}");
    assert_eq!(
        detail["invoice_number"], "2026/HTTP/0001",
        "corpo: {detail}"
    );
    assert!(detail["supplier_name"].is_null(), "corpo: {detail}");

    // Da `pending_review` si corregge — e si **resta** lì — e poi si valida.
    // Prima del passo 3 dell'OCR entrambe rispondevano `409`.
    let (status, corrected) = app
        .send(
            "PATCH",
            &format!("/api/v1/utility-bills/{bill}"),
            Some(&owner),
            Some(serde_json::json!({"supplier_name": "Enel Energia"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {corrected}");
    assert_eq!(corrected["status"], "pending_review", "corpo: {corrected}");

    let (status, validated) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/validate"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {validated}");
    assert_eq!(validated["status"], "validated", "corpo: {validated}");

    // …e la proposta è ancora lì, che è ciò che rende misurabile l'accuratezza.
    let (status, after) = app
        .send(
            "GET",
            &format!("/api/v1/utility-bills/{bill}/ocr"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {after}");
    assert_eq!(after["status"], "validated", "corpo: {after}");
    assert!(after["extracted"].is_array(), "corpo: {after}");
}

/// Il fallimento, visto da fuori: il job dice perché, la bolletta resta in bozza,
/// e **la richiesta che ha caricato era riuscita**.
#[tokio::test]
async fn a_failed_reading_is_reported_and_the_bill_stays_a_draft() {
    let app = require_app!();
    let _gate = common::lock_ocr_queue(&app.pools).await;
    let (owner, _) = common::register_and_login(&app, "ocr-contratto-ko").await;
    let property = create_property(&app, &owner, "Immobile con scansione illeggibile").await;
    let bill = register_bill(&app, &owner, &property, None, "2026/HTTP/0002").await;

    let (status, uploaded) = app
        .upload(common::Upload {
            path: &format!("/api/v1/utility-bills/{bill}/documents"),
            token: &owner,
            org: None,
            filename: "lettera.txt",
            content_type: "text/plain",
            content: b"Questa non e' una bolletta.\n",
            kind: Some("other"),
        })
        .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "il caricamento riesce anche se l'OCR poi fallirà: corpo: {uploaded}"
    );

    run_ocr_worker(&app).await;

    let (status, failed) = app
        .send(
            "GET",
            &format!("/api/v1/utility-bills/{bill}/ocr"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {failed}");
    assert_eq!(failed["job"]["status"], "failed", "corpo: {failed}");
    assert_eq!(
        failed["job"]["error_code"], "unsupported_document",
        "corpo: {failed}"
    );
    assert_eq!(failed["status"], "draft", "corpo: {failed}");
    assert!(failed["extracted"].is_null(), "corpo: {failed}");

    // La bolletta si corregge e si valida a mano, come se l'OCR non esistesse.
    let (status, validated) = app
        .post_auth(
            &format!("/api/v1/utility-bills/{bill}/validate"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {validated}");
    assert_eq!(validated["status"], "validated", "corpo: {validated}");
}

/// La rotta è **dello staff**, come `ai_jobs_select`. Un inquilino che vede la
/// bolletta non vede il job: sarebbe un `403`, non un `200` con `null`, perché
/// «non ti è permesso» e «non c'è» sono due risposte diverse.
#[tokio::test]
async fn the_ocr_route_is_staff_only_and_unknown_bills_are_not_found() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "ocr-permessi").await;
    let property = create_property(&app, &owner, "Immobile con inquilino").await;
    let lease = draft_lease(&app, &owner, &property, None, -10, Some(355)).await;

    let email = common::unique_email("ocr-inquilino");
    add_tenant(&app, &owner, &lease, "Inquilino OCR", Some(&email)).await;
    let (status, activated) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {activated}");
    let (_, accept_token) = invite_tenant(&app, &owner, &lease, &email).await;
    let (tenant, _tenant_id) = common::register_tenant_from_invitation(
        &app,
        &accept_token,
        "Inquilino OCR",
        "cavallo batteria graffetta",
    )
    .await;

    let bill = register_bill(&app, &owner, &property, Some(&lease), "2026/HTTP/0003").await;

    // L'inquilino la bolletta la vede…
    let (status, visible) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/utility-bills/{bill}"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {visible}");

    // …e il job no.
    let (status, denied) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/utility-bills/{bill}/ocr"),
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {denied}");
    assert_eq!(denied["code"], "org.staff_role_required", "corpo: {denied}");

    // Una bolletta che non esiste è `404`, non un `200` con tutto a `null`.
    let (status, missing) = app
        .send(
            "GET",
            &format!("/api/v1/utility-bills/{}/ocr", uuid::Uuid::now_v7()),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {missing}");
    assert_eq!(
        missing["code"], "utility_bill.not_found",
        "corpo: {missing}"
    );
}

// ---------------------------------------------------------------------------
// Dashboard
// ---------------------------------------------------------------------------

/// I nove numeri si muovono insieme a ciò che il proprietario fa.
///
/// **È la prima superficie che attraversa tutti i moduli insieme**, quindi il
/// primo posto dove si vedrebbe un errore di integrazione fra le fette: un
/// addebito che non entra nell'insoluto, una bolletta che non risulta da
/// rivedere, un canone che non si somma. Per questo il test verifica il
/// *movimento* dei numeri e non i loro valori assoluti — che dipenderebbero da
/// quante altre righe esistono nell'organizzazione.
#[tokio::test]
async fn the_dashboard_counts_what_the_owner_has_just_done() {
    let app = require_app!();
    let (owner, _org) = common::register_and_login(&app, "dashboard").await;

    // Un'organizzazione appena creata: tutto a zero, e gli importi sono stringhe
    // con due decimali come ogni altro importo dell'API.
    let (status, empty) = app
        .send("GET", "/api/v1/dashboard", Some(&owner), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {empty}");
    assert_eq!(empty["properties_count"], 0, "corpo: {empty}");
    assert_eq!(empty["active_leases"], 0);
    assert_eq!(empty["monthly_rent_expected"], "0.00");
    assert_eq!(empty["total_outstanding"], "0.00");
    assert_eq!(empty["overdue_charges"], 0);
    assert_eq!(empty["open_tickets"], 0);
    assert_eq!(empty["leases_expiring_90d"], 0);
    assert_eq!(empty["bills_pending_review"], 0);
    assert_eq!(empty["written_off_total"], "0.00");

    let property = create_property(&app, &owner, "Immobile della dashboard").await;
    let lease = draft_lease(&app, &owner, &property, None, -10, Some(30)).await;
    add_tenant(&app, &owner, &lease, "Inquilino Contato", None).await;
    let (status, activated) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/activate"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {activated}");

    // Un addebito già scaduto e mai pagato: entra nell'insoluto e in `overdue`.
    let (status, charge) = app
        .post_auth(
            &format!("/api/v1/leases/{lease}/charges"),
            &owner,
            serde_json::json!({
                "charge_type": "rent",
                "amount": "800.00",
                "due_date": day(-1),
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {charge}");

    // Una bolletta in bozza: è una delle due popolazioni di
    // `bills_pending_review`, e la conta senza che nessuno abbia caricato nulla.
    let (status, bill) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/utility-bills"),
            &owner,
            serde_json::json!({
                "utility_type": "gas",
                "period_start": day(-40),
                "period_end": day(-10),
                "total_amount": "120.00",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {bill}");

    // Una segnalazione aperta.
    let (status, ticket) = app
        .post_auth(
            &format!("/api/v1/properties/{property}/tickets"),
            &owner,
            serde_json::json!({ "title": "Il boiler perde", "description": "Da ieri sera" }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {ticket}");

    let (status, full) = app
        .send("GET", "/api/v1/dashboard", Some(&owner), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {full}");
    assert_eq!(full["properties_count"], 1, "corpo: {full}");
    assert_eq!(full["active_leases"], 1, "corpo: {full}");
    assert_eq!(full["monthly_rent_expected"], "800.00", "corpo: {full}");
    assert_eq!(full["total_outstanding"], "800.00", "corpo: {full}");
    assert_eq!(full["overdue_charges"], 1, "corpo: {full}");
    assert_eq!(full["open_tickets"], 1, "corpo: {full}");
    // Il contratto scade fra trenta giorni: rientra nella finestra dei novanta.
    assert_eq!(full["leases_expiring_90d"], 1, "corpo: {full}");
    assert_eq!(full["bills_pending_review"], 1, "corpo: {full}");

    // Annullare l'addebito lo toglie **sia** dall'insoluto sia dagli scaduti: la
    // pretesa è caduta, e non è dovuto niente. È la migration `0024` vista dalla
    // dashboard.
    let charge_id = charge["id"].as_str().expect("id");
    let (status, voided) = app
        .post_auth(
            &format!("/api/v1/charges/{charge_id}/void"),
            &owner,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {voided}");

    let (status, after) = app
        .send("GET", "/api/v1/dashboard", Some(&owner), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {after}");
    assert_eq!(after["total_outstanding"], "0.00", "corpo: {after}");
    assert_eq!(after["overdue_charges"], 0, "corpo: {after}");
}

/// La dashboard è dello staff, e a un inquilino risponde `403` e non una riga di
/// zeri.
///
/// La Row Level Security da sola non lo fermerebbe in modo utile: la vista passa
/// da `organizations_select`, che è `is_org_member` e non `is_org_staff`, quindi
/// un inquilino otterrebbe i numeri **dell'intera organizzazione**. È uno dei casi
/// in cui il controllo applicativo non duplica la policy ma la stringe.
#[tokio::test]
async fn the_dashboard_is_staff_only() {
    let app = require_app!();
    let (owner, owner_org, tenant, _charge) =
        charge_with_a_real_tenant(&app, "dashboard-inquilino").await;

    let (status, body) = app
        .send_in_org(
            "GET",
            "/api/v1/dashboard",
            Some(&tenant),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "corpo: {body}");
    assert_eq!(body["code"], "org.staff_role_required", "corpo: {body}");

    // Allo stesso proprietario risponde, così il `403` di sopra è del ruolo e non
    // della rotta.
    let (status, body) = app
        .send("GET", "/api/v1/dashboard", Some(&owner), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {body}");
}

/// Senza organizzazione attiva la dashboard non indovina: `403 org.not_selected`.
///
/// È il caso dell'inquilino registrato da sé, che ha la propria organizzazione
/// personale in testa — un debito noto — ma qui vale per chiunque presenti un
/// token senza organizzazione.
#[tokio::test]
async fn the_dashboard_needs_authentication() {
    let app = require_app!();
    let (status, body) = app.send("GET", "/api/v1/dashboard", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "corpo: {body}");
    assert_eq!(body["code"], "auth.token_missing", "corpo: {body}");
}

// ---------------------------------------------------------------------------
// ADR-053: si entra per invito, non da una rotta aperta a chiunque.
//
// `POST /auth/register` è sparita (F6): il test più sotto prova il `404`. Le
// due rotte qui sotto sono quelle su cui il prodotto atterra: un token da
// proprietario apre un account e un'organizzazione, un token da inquilino
// apre un account **dentro** l'organizzazione di chi ha invitato.
// ---------------------------------------------------------------------------

/// `POST /auth/register` non «richiede un token»: **non esiste più**. Un `404`
/// e non un `405`, per qualunque corpo: la rotta è sparita per ogni verbo, non
/// solo per `POST` — vedi `unknown_routes_return_problem_details_too` per la
/// stessa forma su una rotta che non è mai esistita.
#[tokio::test]
async fn registering_freely_is_gone() {
    let app = require_app!();
    let (status, body) = app
        .post(
            "/api/v1/auth/register",
            serde_json::json!({
                "email": common::unique_email("porta-vecchia"),
                "full_name": "Tizio",
                "password": "cavallo batteria graffetta",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "corpo: {body}");
    assert_eq!(body["code"], "endpoint.not_found", "corpo: {body}");
}

/// Scrive un invito da proprietario **già scaduto**, retrodatando sia
/// `created_at` sia `expires_at`: il vincolo `owner_invitations_expiry_ck`
/// impone `expires_at > created_at`, quindi non basta spostare indietro la
/// sola scadenza di una riga già inserita da `common::issue_owner_invitation`.
async fn issue_owner_invitation_expired(app: &TestApp, email: &str) -> String {
    use propflow_application::ports::OpaqueTokenFactory as _;
    use propflow_infrastructure::security::OpaqueTokenGenerator;

    let token = OpaqueTokenGenerator.generate().expect("token generato");
    let created_at = chrono::Utc::now() - chrono::Duration::days(9);
    let expires_at = chrono::Utc::now() - chrono::Duration::days(2);
    sqlx::query(
        "INSERT INTO app.owner_invitations (email, token_hash, expires_at, created_at)
         VALUES ($1::app.email_address, $2, $3, $4)",
    )
    .bind(email)
    .bind(token.hash.as_bytes().as_slice())
    .bind(expires_at)
    .bind(created_at)
    .execute(app.pools.privileged())
    .await
    .expect("invito scaduto inserito");
    token.plaintext
}

/// Scrive un invito da proprietario **già revocato**, direttamente in
/// `INSERT` e non con una `UPDATE` successiva: il runbook revoca con `UPDATE
/// … SET revoked_at`, ma quella `UPDATE` gira con il ruolo del **migratore**
/// (`docs/07-operazioni.md`), che i test non hanno — la migration `0040`
/// concede a `propflow_worker`, il ruolo privilegiato dei test, `INSERT` senza
/// restrizioni di colonna e `UPDATE` sulle sole `accepted_at`/`accepted_by`.
/// Una riga che nasce già revocata produce lo stesso stato che il runbook
/// produrrebbe con una scrittura in più.
async fn issue_owner_invitation_revoked(app: &TestApp, email: &str) -> String {
    use propflow_application::ports::OpaqueTokenFactory as _;
    use propflow_infrastructure::security::OpaqueTokenGenerator;

    let token = OpaqueTokenGenerator.generate().expect("token generato");
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO app.owner_invitations (email, token_hash, expires_at, created_at, revoked_at)
         VALUES ($1::app.email_address, $2, $3, $4, $5)",
    )
    .bind(email)
    .bind(token.hash.as_bytes().as_slice())
    .bind(now + chrono::Duration::days(7))
    .bind(now)
    .bind(now)
    .execute(app.pools.privileged())
    .await
    .expect("invito revocato inserito");
    token.plaintext
}

#[tokio::test]
async fn accepting_an_owner_invitation_creates_an_account_and_an_organization() {
    let app = require_app!();
    let email = common::unique_email("proprietario-invitato");
    let token = common::issue_owner_invitation(&app, &email).await;

    let (status, body, cookie) = app
        .post_returning_cookie(
            "/api/v1/auth/accept-owner-invitation",
            serde_json::json!({
                "token": token,
                "full_name": "Proprietario Invitato",
                "password": "cavallo batteria graffetta",
                "organization_name": "L'organizzazione di Proprietario Invitato",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {body}");
    // Sessione e cookie come il login: nessun secondo giro per entrare.
    assert!(body["access_token"].as_str().is_some_and(|t| !t.is_empty()));
    assert_eq!(body["role"], "owner", "corpo: {body}");
    let org_id = body["organization_id"]
        .as_str()
        .expect("organizzazione")
        .to_owned();
    assert!(
        cookie.is_some(),
        "atteso il cookie di sessione, come il login"
    );
    let token = body["access_token"].as_str().expect("access token");

    // L'organizzazione esiste davvero ed è l'unica di questo utente, con il
    // ruolo `owner` — esattamente ciò che la vecchia registrazione libera dava
    // oggi a chiunque.
    let (status, orgs) = app
        .send("GET", "/api/v1/organizations", Some(token), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {orgs}");
    let items = orgs["items"].as_array().expect("elenco organizzazioni");
    assert_eq!(items.len(), 1, "corpo: {orgs}");
    assert_eq!(items[0]["id"], org_id, "corpo: {orgs}");
    assert_eq!(items[0]["role"], "owner", "corpo: {orgs}");
}

/// I quattro stati non validi di un invito da proprietario rispondono **la
/// stessa cosa**: token inesistente, scaduto, revocato o già accettato. È il
/// criterio di fine di F4 e la decisione 6 dell'ADR-053 — distinguerli sarebbe
/// un oracolo su quali token sono stati emessi.
#[tokio::test]
async fn owner_invitation_token_states_respond_identically() {
    let app = require_app!();

    let nonexistent = "token-mai-emesso-1234567890123456789012345";

    let expired_email = common::unique_email("owner-scaduto");
    let expired = issue_owner_invitation_expired(&app, &expired_email).await;

    let revoked =
        issue_owner_invitation_revoked(&app, &common::unique_email("owner-revocato")).await;

    let accepted_email = common::unique_email("owner-accettato");
    let accepted = common::issue_owner_invitation(&app, &accepted_email).await;
    let (status, body) = app
        .post(
            "/api/v1/auth/accept-owner-invitation",
            serde_json::json!({
                "token": accepted,
                "full_name": "Proprietario Accettato",
                "password": "cavallo batteria graffetta",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {body}");

    for (label, token) in [
        ("inesistente", nonexistent.to_owned()),
        ("scaduto", expired),
        ("revocato", revoked),
        ("già accettato", accepted),
    ] {
        let (status, body) = app
            .post(
                "/api/v1/auth/accept-owner-invitation",
                serde_json::json!({
                    "token": token,
                    "full_name": "Non deve importare",
                    "password": "cavallo batteria graffetta",
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "caso «{label}», corpo: {body}"
        );
        assert_eq!(
            body["code"], "auth.invalid_credentials",
            "caso «{label}», corpo: {body}"
        );
    }
}

#[tokio::test]
async fn registering_from_a_tenant_invitation_lands_inside_the_inviters_organization() {
    let app = require_app!();
    let (owner, owner_org) = common::register_and_login(&app, "invita-da-zero").await;
    let property = create_property(&app, &owner, "Immobile per invito diretto").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;
    let tenant_email = common::unique_email("registrato-da-invito");
    let (_, token) = invite_tenant(&app, &owner, &lease, &tenant_email).await;

    let (status, body, cookie) = app
        .post_returning_cookie(
            "/api/v1/auth/register-from-invitation",
            serde_json::json!({
                "token": token,
                "full_name": "Inquilino Diretto",
                "password": "cavallo batteria graffetta",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {body}");
    assert_eq!(body["role"], "tenant", "corpo: {body}");
    assert_eq!(body["organization_id"], owner_org, "corpo: {body}");
    assert!(
        cookie.is_some(),
        "atteso il cookie di sessione, come il login"
    );
    let tenant_token = body["access_token"].as_str().expect("access token");

    // Zero organizzazioni proprie: non è passato dalla vecchia registrazione
    // libera, quindi non esiste un'organizzazione personale da nascondere o da
    // ripulire.
    let (status, orgs) = app
        .send("GET", "/api/v1/organizations", Some(tenant_token), None)
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {orgs}");
    let items = orgs["items"].as_array().expect("elenco organizzazioni");
    assert_eq!(
        items.len(),
        1,
        "un solo elemento, quello di chi ha invitato: {orgs}"
    );
    assert_eq!(items[0]["id"], owner_org, "corpo: {orgs}");
    assert_eq!(items[0]["role"], "tenant", "corpo: {orgs}");

    // La parte contrattuale è collegata: è ciò che rende visibile il contratto.
    let (status, detail) = app
        .send(
            "GET",
            &format!("/api/v1/leases/{lease}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {detail}");
    let party = detail["parties"]
        .as_array()
        .expect("parti del contratto")
        .iter()
        .find(|party| party["email"] == tenant_email)
        .expect("la parte esiste");
    assert!(
        party.get("user_id").is_some(),
        "la parte doveva collegarsi all'utente appena registrato: {detail}"
    );

    // E l'inquilino stesso vede il proprio contratto, con l'header di
    // organizzazione: la regola 24 non ha eccezioni per chi è appena entrato.
    let (status, mine) = app
        .send_in_org(
            "GET",
            &format!("/api/v1/leases/{lease}"),
            Some(tenant_token),
            Some(&owner_org),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "corpo: {mine}");
}

/// I quattro stati non validi di un invito da inquilino rispondono **la
/// stessa cosa** su questa rotta nuova — identico a `owner_invitation_token_states_respond_identically`,
/// ma sull'altra tabella: inesistente, scaduto, usato o revocato.
#[tokio::test]
async fn register_from_invitation_token_states_respond_identically() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "stati-invito-inquilino").await;
    let property = create_property(&app, &owner, "Immobile per gli stati").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;

    let nonexistent = "token-mai-emesso-su-org-invitations-0000000";

    let (expired_id, expired_token) = invite_tenant(
        &app,
        &owner,
        &lease,
        &common::unique_email("tenant-scaduto"),
    )
    .await;
    sqlx::query(
        "UPDATE app.org_invitations
            SET created_at = now() - interval '9 days',
                expires_at = now() - interval '2 days'
          WHERE id = $1",
    )
    .bind(uuid::Uuid::parse_str(&expired_id).expect("uuid"))
    .execute(app.pools.privileged())
    .await
    .expect("invito retrodatato");

    let (revoked_id, revoked_token) = invite_tenant(
        &app,
        &owner,
        &lease,
        &common::unique_email("tenant-revocato"),
    )
    .await;
    let (status, _) = app
        .send(
            "DELETE",
            &format!("/api/v1/invitations/{revoked_id}"),
            Some(&owner),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let used_email = common::unique_email("tenant-usato");
    let (_, used_token) = invite_tenant(&app, &owner, &lease, &used_email).await;
    let (status, body) = app
        .post(
            "/api/v1/auth/register-from-invitation",
            serde_json::json!({
                "token": used_token,
                "full_name": "Inquilino Usato",
                "password": "cavallo batteria graffetta",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {body}");

    for (label, token) in [
        ("inesistente", nonexistent.to_owned()),
        ("scaduto", expired_token),
        ("revocato", revoked_token),
        ("già usato", used_token),
    ] {
        let (status, body) = app
            .post(
                "/api/v1/auth/register-from-invitation",
                serde_json::json!({
                    "token": token,
                    "full_name": "Non deve importare",
                    "password": "cavallo batteria graffetta",
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "caso «{label}», corpo: {body}"
        );
        assert_eq!(
            body["code"], "auth.invalid_credentials",
            "caso «{label}», corpo: {body}"
        );
    }
}

/// Un'email nel corpo non è un campo: si riconosce e si rifiuta con `422`,
/// come `email` in `PATCH /me` — non si ignora silenziosamente, perché
/// ignorarla darebbe un account collegato a un indirizzo diverso da quello
/// scritto nel corpo, senza alcun indizio.
#[tokio::test]
async fn register_from_invitation_rejects_an_email_field_in_the_body() {
    let app = require_app!();
    let (owner, _) = common::register_and_login(&app, "email-nel-corpo").await;
    let property = create_property(&app, &owner, "Immobile per il campo email").await;
    let lease = draft_lease(&app, &owner, &property, None, 0, Some(365)).await;
    let tenant_email = common::unique_email("email-invito");
    let (_, token) = invite_tenant(&app, &owner, &lease, &tenant_email).await;

    let (status, body) = app
        .post(
            "/api/v1/auth/register-from-invitation",
            serde_json::json!({
                "token": token,
                "full_name": "Prova Campo Email",
                "password": "cavallo batteria graffetta",
                "email": "un-indirizzo-diverso@example.test",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "corpo: {body}");
    assert_eq!(body["code"], "validation.failed", "corpo: {body}");
    assert_eq!(body["errors"][0]["field"], "email", "corpo: {body}");
}

/// Il `409` riservato a un token da proprietario: solo un token **valido**
/// nella tabella dei proprietari lo produce, per non aprire un secondo oracolo
/// accanto ad `auth.email_taken` (ADR-053, precisazione della sezione 8).
#[tokio::test]
async fn a_valid_owner_token_on_the_tenant_route_asks_for_the_organization_name() {
    let app = require_app!();
    let email = common::unique_email("proprietario-alla-porta-sbagliata");
    let owner_token = common::issue_owner_invitation(&app, &email).await;

    let (status, body) = app
        .post(
            "/api/v1/auth/register-from-invitation",
            serde_json::json!({
                "token": owner_token,
                "full_name": "Proprietario Confuso",
                "password": "cavallo batteria graffetta",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "corpo: {body}");
    assert_eq!(body["code"], "invitation.owner_token", "corpo: {body}");

    // Lo stesso token, alla porta giusta, funziona davvero: il `409` sopra non
    // era un rifiuto del token, ma della rotta.
    let (status, body) = app
        .post(
            "/api/v1/auth/accept-owner-invitation",
            serde_json::json!({
                "token": owner_token,
                "full_name": "Proprietario Confuso",
                "password": "cavallo batteria graffetta",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "corpo: {body}");
}

/// **Solo** un token da proprietario valido dà il `409`. Un token da
/// proprietario scaduto o revocato, e un token che non esiste in nessuna delle
/// due tabelle, restano `401` — altrimenti il `409` diventerebbe un secondo
/// modo di sapere quali token da proprietario esistono.
#[tokio::test]
async fn only_a_valid_owner_token_yields_the_409_every_other_case_is_401() {
    let app = require_app!();

    let expired_email = common::unique_email("owner-scaduto-sulla-rotta-inquilino");
    let expired_owner_token = issue_owner_invitation_expired(&app, &expired_email).await;

    let revoked_owner_token = issue_owner_invitation_revoked(
        &app,
        &common::unique_email("owner-revocato-sulla-rotta-inquilino"),
    )
    .await;

    let nonexistent = "nessuna-tabella-conosce-questo-token-1234567";

    for (label, token) in [
        ("proprietario scaduto", expired_owner_token),
        ("proprietario revocato", revoked_owner_token),
        ("inesistente ovunque", nonexistent.to_owned()),
    ] {
        let (status, body) = app
            .post(
                "/api/v1/auth/register-from-invitation",
                serde_json::json!({
                    "token": token,
                    "full_name": "Non deve importare",
                    "password": "cavallo batteria graffetta",
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "caso «{label}», corpo: {body}"
        );
        assert_eq!(
            body["code"], "auth.invalid_credentials",
            "caso «{label}», corpo: {body}"
        );
    }
}
