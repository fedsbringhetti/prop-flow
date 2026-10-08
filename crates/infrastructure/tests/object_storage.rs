// Crate di test: qui `expect` è lo strumento corretto. Un panic segnala una
// precondizione di test non soddisfatta e fa fallire il test, che è il
// comportamento desiderato.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

//! Test dell'adattatore S3 contro uno storage **vero**.
//!
//! Girano solo se l'ambiente è configurato: `ops/ci/test-integration.sh` e la CI
//! avviano uno storage effimero (`ops/ci/prepare-object-storage.sh`, SeaweedFS
//! dopo ADR-057), e la CI fallisce se trova un test `SALTATO`. Senza le variabili
//! si dichiarano saltati, come i test che richiedono il database.
//!
//! Per eseguirli in locale:
//!
//! ```bash
//! docker compose up -d storage storage-setup
//! export PROPFLOW_TEST_S3_ENDPOINT=http://127.0.0.1:9000
//! export PROPFLOW_TEST_S3_BUCKET=propflow-documents
//! export PROPFLOW_TEST_S3_ACCESS_KEY_ID=propflow
//! export PROPFLOW_TEST_S3_SECRET_ACCESS_KEY=propflow-dev-secret
//! cargo test -p propflow-infrastructure --test object_storage
//! ```

use std::time::Duration;

use propflow_application::ports::ObjectStorage;
use propflow_infrastructure::storage::{S3ObjectStorage, S3Settings};

fn storage() -> Option<S3ObjectStorage> {
    let (Ok(endpoint), Ok(bucket), Ok(access_key_id), Ok(secret_access_key)) = (
        std::env::var("PROPFLOW_TEST_S3_ENDPOINT"),
        std::env::var("PROPFLOW_TEST_S3_BUCKET"),
        std::env::var("PROPFLOW_TEST_S3_ACCESS_KEY_ID"),
        std::env::var("PROPFLOW_TEST_S3_SECRET_ACCESS_KEY"),
    ) else {
        eprintln!(
            "SALTATO: storage oggetti non configurato \
             (PROPFLOW_TEST_S3_ENDPOINT e affini, vedi docker compose up -d storage storage-setup)"
        );
        return None;
    };

    Some(
        S3ObjectStorage::new(&S3Settings {
            endpoint,
            region: std::env::var("PROPFLOW_TEST_S3_REGION")
                .unwrap_or_else(|_| "us-east-1".to_owned()),
            bucket,
            access_key_id,
            secret_access_key,
            // In locale non c'è un DNS `<bucket>.<host>`: senza path style ogni
            // richiesta fallirebbe nella risoluzione del nome.
            path_style: true,
            request_timeout: Duration::from_secs(10),
        })
        .expect("configurazione valida"),
    )
}

macro_rules! require_storage {
    () => {
        match storage() {
            Some(storage) => storage,
            None => return,
        }
    };
}

/// Scrittura, rilettura e digest: le tre cose su cui il resto del codice fa
/// affidamento. Il checksum lo calcola **chi scrive**, e deve coprire i byte che
/// tornano indietro.
#[tokio::test]
async fn an_object_survives_a_write_and_a_read() {
    let storage = require_storage!();
    let key = format!("test/{}", uuid::Uuid::now_v7());
    let content = b"%PDF-1.7 contenuto di prova".to_vec();

    let stored = storage
        .put(&key, "application/pdf", content.clone())
        .await
        .expect("scrittura riuscita");
    assert_eq!(stored.key, key);
    assert_eq!(stored.size_bytes, content.len() as u64);

    let read = storage.get(&key).await.expect("lettura riuscita");
    assert_eq!(read, content);

    use sha2::{Digest as _, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&read);
    let expected: [u8; 32] = hasher.finalize().into();
    assert_eq!(stored.checksum_sha256, expected);
}

/// Un oggetto che non c'è è `NotFound` e non un errore generico: il caso d'uso
/// deve poter distinguere "il bucket non risponde" da "quella chiave non esiste".
#[tokio::test]
async fn a_missing_object_is_reported_as_not_found() {
    let storage = require_storage!();
    let error = storage
        .get(&format!("test/mai-scritto-{}", uuid::Uuid::now_v7()))
        .await
        .expect_err("deve fallire");
    assert!(
        matches!(error, propflow_application::ports::StorageError::NotFound),
        "errore inatteso: {error:?}"
    );
}

/// L'URL pre-firmato deve funzionare **senza credenziali**: è il punto di tutta
/// l'asimmetria fra caricamento e scaricamento. Se non funzionasse, il `307`
/// dell'API manderebbe il client su un `403`.
#[tokio::test]
async fn a_presigned_url_can_be_fetched_without_credentials() {
    let storage = require_storage!();
    let key = format!("test/{}", uuid::Uuid::now_v7());
    let content = b"contenuto pre-firmato".to_vec();
    storage
        .put(&key, "text/plain", content.clone())
        .await
        .expect("scrittura riuscita");

    let url = storage
        .presigned_get_url(&key, 120)
        .await
        .expect("firma prodotta");

    let response = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .expect("richiesta eseguita");
    assert!(
        response.status().is_success(),
        "l'URL pre-firmato è stato rifiutato: {}",
        response.status()
    );
    assert_eq!(
        response.bytes().await.expect("corpo").to_vec(),
        content,
        "l'URL pre-firmato ha restituito un contenuto diverso"
    );
}
