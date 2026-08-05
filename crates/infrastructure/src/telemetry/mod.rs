//! Osservabilità: log strutturati e tracing distribuito.

use opentelemetry_otlp::WithExportConfig as _;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::{EnvFilter, Layer as _};

/// Configurazione della telemetria.
#[derive(Debug, Clone)]
pub struct TelemetrySettings {
    pub service_name: String,
    /// Endpoint OTLP (gRPC). Se assente si esporta solo su stdout.
    pub otlp_endpoint: Option<String>,
    /// Direttiva per `EnvFilter`, es. `info,propflow_api=debug`.
    pub filter: String,
    /// Log in JSON: adatto a un aggregatore. In sviluppo conviene il testo.
    pub json: bool,
}

impl Default for TelemetrySettings {
    fn default() -> Self {
        Self {
            service_name: "propflow-api".to_owned(),
            otlp_endpoint: None,
            filter: "info".to_owned(),
            json: false,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    #[error("configurazione della telemetria non valida: {0}")]
    Invalid(String),
}

/// Handle da tenere vivo per la durata del processo: al `Drop` svuota le tracce
/// ancora in coda. Perderle allo shutdown significa non avere proprio le tracce
/// degli errori che hanno causato lo shutdown.
pub struct TelemetryGuard {
    provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.provider.take()
            && let Err(error) = provider.shutdown()
        {
            eprintln!("chiusura dell'esportatore di tracce non riuscita: {error}");
        }
    }
}

/// Inizializza log e tracce.
///
/// Nessun campo con dati personali viene registrato dai layer applicativi: email,
/// token e corpi delle richieste restano fuori dalle tracce per scelta, non per
/// dimenticanza — il requisito è "niente dati sensibili nei log".
pub fn init(settings: &TelemetrySettings) -> Result<TelemetryGuard, TelemetryError> {
    let filter = EnvFilter::try_new(&settings.filter)
        .map_err(|error| TelemetryError::Invalid(error.to_string()))?;

    let fmt_layer = if settings.json {
        tracing_subscriber::fmt::layer()
            .json()
            .with_current_span(true)
            .boxed()
    } else {
        tracing_subscriber::fmt::layer().with_target(true).boxed()
    };

    let Some(endpoint) = settings.otlp_endpoint.clone() else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .init();
        return Ok(TelemetryGuard { provider: None });
    };

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .map_err(|error| TelemetryError::Invalid(error.to_string()))?;

    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            opentelemetry_sdk::Resource::builder()
                .with_service_name(settings.service_name.clone())
                .build(),
        )
        .build();

    let tracer = opentelemetry::trace::TracerProvider::tracer(&provider, "propflow");
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();

    Ok(TelemetryGuard {
        provider: Some(provider),
    })
}
