use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::ai::{
    AiCost, AiJob, AiJobErrorCode, AiJobFailure, AiJobParts, AiTask, BillField, Confidence,
    ProviderUsage,
};
use propflow_domain::document::{AttachmentOwner, AttachmentOwnerType};
use propflow_domain::ports::{AiJobQueue, AiJobRepository, RepositoryError, RepositoryResult};
use propflow_domain::shared::{AiJobId, Currency, DocumentId, OrgId};

use super::user::try_get;
use crate::db::db_enums;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

/// Colonne lette di un job.
///
/// **Non ci sono `request_params` né `trace_id`.** Il commento della `0010` chiede
/// che in `ai_jobs` vadano «SOLO parametri e riferimenti, mai il prompt con PII,
/// mai l'immagine»: finché la porta non ha parametri da passare, la prima colonna
/// non ha niente di onesto da contenere e resta al suo `{}`. `trace_id` è la
/// correlazione con OpenTelemetry, e scriverci il nostro `request_id`
/// significherebbe chiamare traccia una cosa che non lo è.
///
/// **Non c'è `deleted_at`**, e non è una svista: `app.ai_jobs` non ha quella
/// colonna. È la terza tabella senza, dopo i pagamenti e quelle del co-living, e
/// filtrarla per abitudine darebbe un errore di colonna inesistente invece di
/// zero righe.
macro_rules! select_job {
    () => {
        "
    SELECT id,
           org_id,
           capability::text   AS capability,
           task,
           status::text       AS status,
           provider_name,
           model,
           provider_version,
           subject_type::text AS subject_type,
           subject_id,
           input_document_id,
           output,
           confidence::numeric  AS confidence,
           tokens_input,
           tokens_output,
           cost_amount::numeric AS cost_amount,
           currency::text       AS currency,
           latency_ms,
           attempts,
           error_code,
           error_message,
           started_at,
           finished_at,
           created_at,
           updated_at
      FROM app.ai_jobs
"
    };
}

#[async_trait]
impl AiJobRepository for PgUnitOfWork {
    async fn enqueue(&mut self, job: &AiJob) -> RepositoryResult<()> {
        // `status` compare esplicitamente anche se il default è `'queued'`: la
        // policy `ai_jobs_enqueue` della `0031` lo vincola nella `WITH CHECK`, e
        // affidarsi al default significherebbe che una modifica al default
        // cambierebbe in silenzio quale ramo della policy si sta soddisfacendo.
        //
        // `currency` non compare: resta al default `EUR`, per la stessa ragione
        // per cui non compare nell'inserimento di una bolletta.
        //
        // Nessun `RETURNING`: gli identificatori nascono nel dominio con
        // `generate()` (UUIDv7). E qui il `RETURNING` costerebbe caro per un
        // motivo in più — rileggerebbe la riga sotto `ai_jobs_select`, cioè sotto
        // una policy diversa da quella che l'ha ammessa.
        sqlx::query(
            "INSERT INTO app.ai_jobs (
                 id, org_id, capability, task, status, provider_name,
                 subject_type, subject_id, input_document_id,
                 attempts, created_at, updated_at
             ) VALUES (
                 $1, $2, $3::app.ai_capability, $4, $5::app.ai_job_status, $6,
                 $7::app.attachment_owner_type, $8, $9,
                 $10, $11, $12
             )",
        )
        .bind(job.id().as_uuid())
        .bind(job.org_id().as_uuid())
        .bind(db_enums::ai_capability_to_db(job.capability()))
        .bind(db_enums::ai_task_to_db(job.task()))
        .bind(db_enums::ai_job_status_to_db(job.status()))
        .bind(job.provider_name())
        .bind(db_enums::attachment_owner_type_to_db(
            job.subject().owner_type(),
        ))
        .bind(job.subject().owner_id())
        .bind(job.input_document_id().map(DocumentId::as_uuid))
        .bind(i16::try_from(job.attempts()).unwrap_or(i16::MAX))
        .bind(job.created_at())
        .bind(job.updated_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn find_latest_for_subject(
        &mut self,
        subject: AttachmentOwner,
    ) -> RepositoryResult<Option<AiJob>> {
        // `ORDER BY created_at DESC` e non per id: `ai_jobs_subject_ix` è
        // `(subject_type, subject_id, created_at DESC)`, e ordinare per id —
        // pur equivalente, con gli UUIDv7 — non userebbe quell'ordinamento.
        let row = sqlx::query(concat!(
            select_job!(),
            " WHERE subject_type = $1::app.attachment_owner_type
                AND subject_id = $2
              ORDER BY created_at DESC
              LIMIT 1"
        ))
        .bind(db_enums::attachment_owner_type_to_db(subject.owner_type()))
        .bind(subject.owner_id())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.as_ref().map(map_job).transpose()
    }
}

/// La coda vista dal worker: gira come `propflow_worker` con `BYPASSRLS`, quindi
/// nessuna di queste query passa da una policy.
///
/// È implementata su `PgUnitOfWork` come tutti gli altri repository — condividono
/// la transazione — ma **non è raggiungibile da `UnitOfWork`**: la si ottiene solo
/// da `WorkerUnitOfWork`, che si apre solo dal pool del worker. Il tipo dice da sé
/// con quale postura si sta parlando al database.
#[async_trait]
impl AiJobQueue for PgUnitOfWork {
    async fn claim_next(&mut self, task: AiTask) -> RepositoryResult<Option<AiJob>> {
        // `FOR UPDATE SKIP LOCKED`: chi arriva secondo salta le righe già
        // bloccate invece di aspettarle. È ciò che rende il worker replicabile
        // senza coordinamento esterno, e va scritto anche con una replica sola.
        //
        // **Senza `SKIP LOCKED` il difetto non è quello che ci si aspetta.** Tolto
        // e misurato: il secondo worker non prende lo stesso job — la riga è
        // bloccata — ma si **ferma ad aspettare**, e resta fermo finché lo
        // `statement_timeout` non lo uccide con un `57014`. Sotto carico questo è
        // peggio del doppio lavoro: le connessioni del pool si esauriscono aspettando
        // una riga che qualcun altro sta lavorando.
        //
        // `status = 'queued'` e non l'elenco dell'indice: `ai_jobs_queue_ix` è
        // parziale su `('queued','running')`, e un'uguaglianza su uno dei due valori
        // implica il predicato dell'indice. Verificato con `EXPLAIN`, non dedotto:
        // il piano è un `Bitmap Index Scan on ai_jobs_queue_ix` con
        // `Recheck Cond: status = ANY ('{queued,running}')`. Prendere anche i
        // `running` sarebbe sbagliato: sono i job che un altro worker ha già in mano.
        let row = sqlx::query(concat!(
            select_job!(),
            " WHERE status = 'queued'::app.ai_job_status
                AND task = $1
              ORDER BY created_at
              LIMIT 1
              FOR UPDATE SKIP LOCKED"
        ))
        .bind(db_enums::ai_task_to_db(task))
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.as_ref().map(map_job).transpose()
    }

    async fn update(&mut self, job: &AiJob) -> RepositoryResult<()> {
        // L'elenco è quello che il worker scrive, e nessuna `GRANT` di colonna lo
        // restringe: qui il ruolo è `propflow_worker`, che ha tutto. La difesa
        // sta dall'altra parte — il ruolo applicativo non ha **nessuna** policy di
        // `UPDATE` su questa tabella, quindi una richiesta HTTP non può riscrivere
        // ciò che una macchina ha registrato.
        //
        // `updated_at` non compare: lo scrive il trigger `set_updated_at`.
        let result = sqlx::query(
            "UPDATE app.ai_jobs
                SET status           = $2::app.ai_job_status,
                    model            = $3,
                    provider_version = $4,
                    output           = $5,
                    confidence       = $6::app.confidence,
                    tokens_input     = $7,
                    tokens_output    = $8,
                    cost_amount      = $9,
                    latency_ms       = $10,
                    attempts         = $11,
                    error_code       = $12,
                    error_message    = $13,
                    started_at       = $14,
                    finished_at      = $15
              WHERE id = $1",
        )
        .bind(job.id().as_uuid())
        .bind(db_enums::ai_job_status_to_db(job.status()))
        .bind(job.model())
        .bind(job.provider_version())
        .bind(encode_output(job.output()))
        .bind(job.confidence().map(Confidence::value))
        .bind(counter(job.usage().tokens_input))
        .bind(counter(job.usage().tokens_output))
        .bind(job.usage().cost.map(AiCost::amount))
        .bind(counter(job.usage().latency_ms))
        .bind(i16::try_from(job.attempts()).unwrap_or(i16::MAX))
        .bind(job.failure().map(|failure| failure.code.as_str()))
        .bind(job.failure().map(|failure| failure.message.as_str()))
        .bind(job.started_at())
        .bind(job.finished_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        // Zero righe qui non è la Row Level Security — il worker ha `BYPASSRLS` —
        // ma un job che non c'è più: la riga è stata cancellata a mano, o
        // l'identificatore è sbagliato. In entrambi i casi è un difetto nostro,
        // e tacerlo lascerebbe un job `running` per sempre.
        if result.rows_affected() == 0 {
            return Err(RepositoryError::Backend {
                details: format!("job `{}` non trovato in aggiornamento", job.id()),
            });
        }
        Ok(())
    }
}

/// I contatori del fornitore verso una colonna `INTEGER`.
///
/// Il dominio ha già rifiutato i valori che non ci stanno
/// (`ProviderUsage::ensure_storable`), quindi questa conversione non può fallire;
/// la si scrive comunque totale invece di con un `unwrap`, che è vietato in
/// produzione.
fn counter(value: Option<u32>) -> Option<i32> {
    value.and_then(|value| i32::try_from(value).ok())
}

/// La **forma** dell'esito verso `ai_jobs.output`.
///
/// ```json
/// {"fields": ["supplier_name", "total_amount"],
///  "confidence": {"supplier_name": "0.9900", "total_amount": "0.6200"}}
/// ```
///
/// L'elenco dei campi è **duplicato** rispetto alle chiavi di `confidence`, e la
/// duplicazione è il motivo per cui questa forma è stata scelta: `ai_jobs_output_ix`
/// è `gin (output jsonb_path_ops)`, e `jsonb_path_ops` indicizza il contenimento
/// (`@>`). Con il solo oggetto `confidence`, chiedere «quali job hanno letto il
/// totale» richiederebbe di conoscerne anche il valore; con l'array,
/// `output @> '{"fields":["total_amount"]}'` risponde usando l'indice. È la
/// domanda su cui si misura l'accuratezza per campo, quindi vale una chiave in
/// più.
///
/// **Nessun valore**: quelli stanno in `utility_bills.ocr_extracted`, che eredita
/// la RLS della bolletta. Vedi il commento della `0010`.
fn encode_output(shape: &[(BillField, Confidence)]) -> serde_json::Value {
    let fields: Vec<serde_json::Value> = shape
        .iter()
        .map(|(field, _)| serde_json::Value::String(field.as_str().to_owned()))
        .collect();
    let confidence: serde_json::Map<String, serde_json::Value> = shape
        .iter()
        .map(|(field, value)| {
            (
                field.as_str().to_owned(),
                // `Display` e non `value().to_string()`: il tipo rende sempre
                // quattro decimali, così la stessa confidenza si scrive allo
                // stesso modo che sia appena calcolata o riletta dal database.
                serde_json::Value::String(value.to_string()),
            )
        })
        .collect();
    serde_json::json!({ "fields": fields, "confidence": confidence })
}

/// La forma, riletta. Un campo che il codice non conosce **fa fallire la
/// lettura**, come ogni enum di questo progetto.
fn decode_output(value: &serde_json::Value) -> RepositoryResult<Vec<(BillField, Confidence)>> {
    let Some(fields) = value.get("fields").and_then(serde_json::Value::as_array) else {
        // `{}` è il default della colonna: un job che non è mai arrivato a
        // `succeeded` non ha una forma, e non è un errore.
        return Ok(Vec::new());
    };
    let confidences = value.get("confidence");
    fields
        .iter()
        .map(|entry| {
            let key = entry.as_str().ok_or_else(|| RepositoryError::Backend {
                details: "`output.fields` contiene un valore che non è una stringa".to_owned(),
            })?;
            let field = BillField::from_key(key).ok_or_else(|| RepositoryError::Backend {
                details: format!("campo `{key}` non riconosciuto in `ai_jobs.output`"),
            })?;
            let raw = confidences
                .and_then(|map| map.get(key))
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| RepositoryError::Backend {
                    details: format!("confidenza assente per il campo `{key}`"),
                })?;
            let confidence = raw
                .parse::<Decimal>()
                .map(Confidence::from_persistence)
                .map_err(|error| RepositoryError::Backend {
                    details: format!("confidenza `{raw}` non decodificabile: {error}"),
                })?;
            Ok((field, confidence))
        })
        .collect()
}

fn map_job(row: &PgRow) -> RepositoryResult<AiJob> {
    let subject_type: String = try_get(row, "subject_type")?;
    let subject_type: AttachmentOwnerType = db_enums::attachment_owner_type_from_db(&subject_type)?;
    let subject_id: Uuid = try_get(row, "subject_id")?;

    let capability: String = try_get(row, "capability")?;
    let task: String = try_get(row, "task")?;
    let status: String = try_get(row, "status")?;
    let currency: String = try_get(row, "currency")?;
    let currency = Currency::parse(&currency).map_err(|error| RepositoryError::Backend {
        details: format!("colonna `currency` non valida: {error}"),
    })?;

    let output: serde_json::Value = try_get(row, "output")?;
    let cost: Option<Decimal> = try_get(row, "cost_amount")?;
    let tokens_input: Option<i32> = try_get(row, "tokens_input")?;
    let tokens_output: Option<i32> = try_get(row, "tokens_output")?;
    let latency_ms: Option<i32> = try_get(row, "latency_ms")?;
    let attempts: i16 = try_get(row, "attempts")?;

    // Codice e messaggio viaggiano insieme: un `error_code` che il codice non
    // conosce fa fallire la lettura, come ogni altro valore d'enum.
    let error_code: Option<String> = try_get(row, "error_code")?;
    let failure = error_code
        .map(|code| {
            let code = decode_error_code(&code)?;
            let message: Option<String> = try_get(row, "error_message")?;
            Ok::<_, RepositoryError>(AiJobFailure::new(code, message.unwrap_or_default()))
        })
        .transpose()?;

    Ok(AiJob::from_persistence(AiJobParts {
        id: AiJobId::from_uuid(try_get(row, "id")?),
        org_id: OrgId::from_uuid(try_get(row, "org_id")?),
        capability: db_enums::ai_capability_from_db(&capability)?,
        task: db_enums::ai_task_from_db(&task)?,
        status: db_enums::ai_job_status_from_db(&status)?,
        provider_name: try_get(row, "provider_name")?,
        model: try_get(row, "model")?,
        provider_version: try_get(row, "provider_version")?,
        subject: AttachmentOwner::from_persistence(subject_type, subject_id),
        input_document_id: try_get::<Option<Uuid>>(row, "input_document_id")?
            .map(DocumentId::from_uuid),
        output: decode_output(&output)?,
        confidence: try_get::<Option<Decimal>>(row, "confidence")?
            .map(Confidence::from_persistence),
        usage: ProviderUsage {
            model: try_get(row, "model")?,
            provider_version: try_get(row, "provider_version")?,
            tokens_input: tokens_input.map(u32::try_from).transpose().ok().flatten(),
            tokens_output: tokens_output.map(u32::try_from).transpose().ok().flatten(),
            cost: cost.map(|amount| AiCost::from_persistence(amount, currency)),
            latency_ms: latency_ms.map(u32::try_from).transpose().ok().flatten(),
        },
        attempts: u16::try_from(attempts).unwrap_or_default(),
        failure,
        started_at: try_get::<Option<DateTime<Utc>>>(row, "started_at")?,
        finished_at: try_get::<Option<DateTime<Utc>>>(row, "finished_at")?,
        created_at: try_get(row, "created_at")?,
        updated_at: try_get(row, "updated_at")?,
    }))
}

fn decode_error_code(value: &str) -> RepositoryResult<AiJobErrorCode> {
    [
        AiJobErrorCode::DocumentArchived,
        AiJobErrorCode::UnsupportedDocument,
        AiJobErrorCode::ProviderUnavailable,
        AiJobErrorCode::SubjectNotProcessable,
        AiJobErrorCode::DocumentUnavailable,
    ]
    .into_iter()
    .find(|code| code.as_str() == value)
    .ok_or_else(|| RepositoryError::Backend {
        details: format!("`error_code` `{value}` non riconosciuto"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use propflow_domain::ai::{ExtractedField, ExtractedValue, OcrReading};
    use std::str::FromStr;

    fn confidence(raw: &str) -> Confidence {
        Confidence::new(Decimal::from_str(raw).expect("decimale")).expect("confidenza")
    }

    fn reading() -> OcrReading {
        OcrReading::new(vec![
            ExtractedField {
                field: BillField::SupplierName,
                value: ExtractedValue::Text("Enel Energia".to_owned()),
                confidence: confidence("0.9900"),
            },
            ExtractedField {
                field: BillField::DueDate,
                value: ExtractedValue::Date(
                    chrono::NaiveDate::from_ymd_opt(2026, 8, 31).expect("data"),
                ),
                confidence: confidence("1.0000"),
            },
            ExtractedField {
                field: BillField::TotalAmount,
                value: ExtractedValue::Amount(Decimal::from_str("184.20").expect("importo")),
                confidence: confidence("0.6200"),
            },
            ExtractedField {
                field: BillField::Consumption,
                value: ExtractedValue::Quantity {
                    value: Decimal::from_str("123.4567").expect("quantità"),
                    unit: "Smc".to_owned(),
                },
                confidence: confidence("1.0000"),
            },
        ])
    }

    /// La ragione per cui i numeri sono stringhe: `123.4567` deve tornare
    /// identico, e attraverso un numero JSON non lo farebbe.
    /// Il default della colonna non è un errore: è un job che non è mai arrivato
    /// a `succeeded`, quindi non ha una forma.
    #[test]
    fn the_column_default_reads_as_no_shape_at_all() {
        assert!(
            decode_output(&serde_json::json!({}))
                .expect("ok")
                .is_empty()
        );
    }

    /// La regola degli enum, applicata a una chiave JSON: ciò che non si sa
    /// interpretare deve essere rumoroso.
    #[test]
    fn an_unknown_field_makes_the_read_fail_instead_of_vanishing() {
        assert!(decode_output(&serde_json::json!({"fields": ["inventato"]})).is_err());
    }

    /// La forma che l'indice `gin (output jsonb_path_ops)` deve poter
    /// interrogare, e il confine che non deve essere attraversato.
    #[test]
    fn the_output_carries_the_fields_for_containment_and_no_values() {
        let encoded = encode_output(&reading().shape());
        assert_eq!(
            encoded["fields"],
            serde_json::json!(["supplier_name", "due_date", "total_amount", "consumption"])
        );
        assert_eq!(encoded["confidence"]["total_amount"], "0.6200");

        let text = encoded.to_string();
        assert!(
            !text.contains("Enel Energia"),
            "un valore è finito nel job: {text}"
        );
        assert!(
            !text.contains("184.20"),
            "un valore è finito nel job: {text}"
        );

        let round_tripped = decode_output(&encoded).expect("decodifica");
        assert_eq!(round_tripped, reading().shape());
    }

    #[test]
    fn every_error_code_survives_a_round_trip() {
        for code in [
            AiJobErrorCode::DocumentArchived,
            AiJobErrorCode::UnsupportedDocument,
            AiJobErrorCode::ProviderUnavailable,
            AiJobErrorCode::SubjectNotProcessable,
            AiJobErrorCode::DocumentUnavailable,
        ] {
            assert_eq!(decode_error_code(code.as_str()).expect("noto"), code);
        }
        assert!(decode_error_code("inventato").is_err());
    }
}
