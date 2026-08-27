use std::fmt;

use crate::error::{DomainError, DomainResult};
use crate::shared::text;

/// Tipo MIME dichiarato per il contenuto.
///
/// Validato e non preso alla lettera: finisce nell'header `Content-Type` di una
/// risposta pre-firmata, quindi un valore con caratteri di controllo o con una
/// newline sarebbe un'iniezione di header a spese di chi scarica.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MimeType(String);

impl MimeType {
    /// Limite dello schema (`TEXT`, nessun `CHECK`): il tetto è nostro e serve a
    /// non accettare un header arbitrariamente lungo.
    pub const MAX_LENGTH: usize = 255;

    pub fn parse(value: &str) -> DomainResult<Self> {
        let trimmed = value.trim();
        let invalid = |message: &'static str| DomainError::validation("mime_type", message);

        if trimmed.is_empty() {
            return Err(invalid("is required"));
        }
        if trimmed.len() > Self::MAX_LENGTH {
            return Err(invalid("exceeds 255 characters"));
        }
        // `type/subtype`, eventualmente seguito da parametri. Un solo `/`, niente
        // spazi e solo ASCII stampabile.
        let Some((kind, rest)) = trimmed.split_once('/') else {
            return Err(invalid("must have the form `type/subtype`"));
        };
        if kind.is_empty() || rest.is_empty() || rest.contains('/') {
            return Err(invalid("must have the form `type/subtype`"));
        }
        if !trimmed
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
        {
            return Err(invalid("contains characters that are not allowed"));
        }
        Ok(Self(trimmed.to_ascii_lowercase()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Ricostruzione da persistenza: non rivalida, come ogni `from_persistence`.
    #[must_use]
    pub fn from_persistence(value: String) -> Self {
        Self(value)
    }
}

impl fmt::Display for MimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Dimensione dell'oggetto in byte.
///
/// Positiva: lo schema ha `CHECK (size_bytes > 0)`, e un documento di zero byte è
/// un upload fallito che non merita una riga.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectSize(u64);

impl ObjectSize {
    pub fn new(bytes: u64) -> DomainResult<Self> {
        if bytes == 0 {
            return Err(DomainError::validation("size_bytes", "must be positive"));
        }
        // `size_bytes` è un BIGINT con segno: oltre questo il valore non è
        // rappresentabile e la scrittura fallirebbe con un errore del driver.
        if bytes > i64::MAX as u64 {
            return Err(DomainError::validation("size_bytes", "is too large"));
        }
        Ok(Self(bytes))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn as_i64(self) -> i64 {
        // La costruzione garantisce il rientro: `new` rifiuta oltre `i64::MAX`.
        self.0 as i64
    }

    /// Ricostruzione da persistenza. Un valore negativo in database
    /// contraddirebbe il `CHECK`, quindi non è un caso da modellare: si satura a
    /// zero, che la rilettura rende visibile invece di far esplodere il cast.
    #[must_use]
    pub const fn from_persistence(value: i64) -> Self {
        Self(if value < 0 { 0 } else { value as u64 })
    }
}

/// Digest SHA-256 del contenuto.
///
/// Calcolato da noi e mai dichiarato dal client: un checksum che arriva dal filo
/// certifica solo che il client sa calcolare uno SHA-256. È la ragione per cui
/// l'upload passa dall'API invece di usare un `PUT` pre-firmato.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Checksum([u8; 32]);

impl Checksum {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Rappresentazione esadecimale, per il filo e per i confronti a occhio.
    #[must_use]
    pub fn to_hex(self) -> String {
        let mut out = String::with_capacity(64);
        for byte in self.0 {
            // `write!` su String non fallisce, ma `unwrap` è vietato: la
            // formattazione a mano evita il ramo di errore invece di ignorarlo.
            out.push(nibble(byte >> 4));
            out.push(nibble(byte & 0x0f));
        }
        out
    }
}

const fn nibble(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        _ => (b'a' + value - 10) as char,
    }
}

/// Un checksum non è un segreto, ma nemmeno un dato da spargere nei log: `Debug`
/// mostra i primi otto caratteri, che bastano a correlare due righe.
impl fmt::Debug for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Checksum({}…)", &self.to_hex()[..8])
    }
}

/// Nome originale del file, come lo ha chiamato chi lo ha caricato.
///
/// **È un dato personale.** `contratto-mario-rossi.pdf` nomina una persona, e
/// spesso il nome del file dice più del documento stesso. Da qui discendono tre
/// vincoli, tutti applicati altrove ma che iniziano da questo tipo:
///
/// - `Debug` non lo mostra (regola 18: niente PII nei log e nelle tracce);
/// - non entra in nessun evento di dominio;
/// - **non entra nella chiave dell'oggetto**, che si deriva da
///   `{org_id}/{document_id}` — vedi [`super::Document::object_key`].
#[derive(Clone, PartialEq, Eq)]
pub struct FileName(String);

impl FileName {
    pub const MAX_LENGTH: usize = 255;

    pub fn parse(value: &str) -> DomainResult<Self> {
        let name = text::required("original_filename", value, Self::MAX_LENGTH)?;
        // Separatori e byte nulli: un nome del genere non arriva da un browser
        // onesto, e la sola cosa che può fare è confondere chi lo rilegge.
        if name.contains('/') || name.contains('\\') || name.contains('\0') {
            return Err(DomainError::validation(
                "original_filename",
                "must not contain path separators",
            ));
        }
        Ok(Self(name))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn from_persistence(value: String) -> Self {
        Self(value)
    }
}

impl fmt::Debug for FileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FileName(<redatto>)")
    }
}

/// Dove l'oggetto è finito: fornitore, bucket e chiave.
///
/// Tre campi e non una URL: la URL di un oggetto S3 dipende dallo stile del
/// percorso e dall'endpoint, che sono configurazione e cambiano con l'ambiente.
/// La tripla invece è stabile e coincide con l'indice unico
/// `documents_object_uk`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageLocation {
    provider: String,
    bucket: String,
    object_key: String,
}

impl StorageLocation {
    pub fn new(provider: &str, bucket: &str, object_key: &str) -> DomainResult<Self> {
        Ok(Self {
            provider: text::required("storage_provider", provider, 32)?,
            bucket: text::required("storage_bucket", bucket, 255)?,
            object_key: text::required("object_key", object_key, 1024)?,
        })
    }

    #[must_use]
    pub fn provider(&self) -> &str {
        &self.provider
    }
    #[must_use]
    pub fn bucket(&self) -> &str {
        &self.bucket
    }
    #[must_use]
    pub fn object_key(&self) -> &str {
        &self.object_key
    }

    #[must_use]
    pub const fn from_persistence(provider: String, bucket: String, object_key: String) -> Self {
        Self {
            provider,
            bucket,
            object_key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_type_is_normalised_and_shaped() {
        assert_eq!(
            MimeType::parse("  Application/PDF ")
                .expect("valido")
                .as_str(),
            "application/pdf"
        );
        for bad in ["", "   ", "application", "a/b/c", "/pdf", "application/"] {
            assert!(
                MimeType::parse(bad).is_err(),
                "`{bad}` doveva essere rifiutato"
            );
        }
    }

    /// Una newline nel MIME diventerebbe un header in più nella risposta di
    /// download: è il motivo per cui questo tipo esiste invece di una `String`.
    #[test]
    fn mime_type_refuses_control_characters() {
        assert!(MimeType::parse("application/pdf\r\nX-Evil: 1").is_err());
    }

    #[test]
    fn size_must_be_positive_and_representable() {
        assert!(ObjectSize::new(0).is_err());
        assert!(ObjectSize::new(u64::MAX).is_err());
        assert_eq!(ObjectSize::new(1).expect("valido").as_i64(), 1);
    }

    #[test]
    fn checksum_renders_as_lowercase_hex() {
        let checksum = Checksum::from_bytes([0xab; 32]);
        assert_eq!(checksum.to_hex(), "ab".repeat(32));
        assert_eq!(format!("{checksum:?}"), "Checksum(abababab…)");
    }

    /// Il nome del file è PII: se comparisse in un `Debug` finirebbe in un log al
    /// primo `tracing::debug!` su una struttura che lo contiene.
    #[test]
    fn file_name_is_not_leaked_by_debug() {
        let name = FileName::parse("contratto-mario-rossi.pdf").expect("valido");
        assert_eq!(format!("{name:?}"), "FileName(<redatto>)");
        assert_eq!(name.as_str(), "contratto-mario-rossi.pdf");
    }

    #[test]
    fn file_name_refuses_path_separators() {
        for bad in ["../etc/passwd", "a\\b.pdf", "   ", ""] {
            assert!(
                FileName::parse(bad).is_err(),
                "`{bad}` doveva essere rifiutato"
            );
        }
    }
}
