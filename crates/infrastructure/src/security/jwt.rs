use chrono::{DateTime, Duration, TimeZone, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use uuid::Uuid;

use propflow_application::ports::{AccessTokenClaims, CryptoError, TokenIssuer};
use propflow_domain::identity::{OrgRole, PlatformRole};
use propflow_domain::shared::{OrgId, UserId};

/// Configurazione dell'emissione dei token.
#[derive(Debug, Clone)]
pub struct JwtSettings {
    /// Chiave privata Ed25519 in PEM PKCS#8.
    pub private_key_pem: String,
    /// Chiave pubblica Ed25519 in PEM SPKI.
    pub public_key_pem: String,
    pub issuer: String,
    pub audience: String,
    pub access_token_lifetime: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum JwtSetupError {
    #[error("chiave non valida: {0}")]
    InvalidKey(String),
}

/// Access token firmati con EdDSA (Ed25519).
///
/// Asimmetrico e non HMAC: quando i moduli si separeranno in servizi, ognuno
/// potrà verificare i token con la sola chiave pubblica, senza che il segreto di
/// firma debba essere distribuito a tutti. Con HS256 ogni verificatore è anche un
/// potenziale emittente.
pub struct EdDsaTokenIssuer {
    encoding: EncodingKey,
    decoding: DecodingKey,
    validation: Validation,
    header: Header,
    issuer: String,
    audience: String,
    access_token_lifetime: Duration,
}

impl EdDsaTokenIssuer {
    pub fn new(settings: &JwtSettings) -> Result<Self, JwtSetupError> {
        let encoding = EncodingKey::from_ed_pem(settings.private_key_pem.as_bytes())
            .map_err(|error| JwtSetupError::InvalidKey(error.to_string()))?;
        let decoding = DecodingKey::from_ed_pem(settings.public_key_pem.as_bytes())
            .map_err(|error| JwtSetupError::InvalidKey(error.to_string()))?;

        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.set_issuer(std::slice::from_ref(&settings.issuer));
        validation.set_audience(std::slice::from_ref(&settings.audience));
        // `exp` obbligatorio: un token senza scadenza che passasse la validazione
        // sarebbe eterno.
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        // Tolleranza minima per lo sfasamento fra gli orologi dei nodi.
        validation.leeway = 5;

        Ok(Self {
            encoding,
            decoding,
            validation,
            header: Header::new(Algorithm::EdDSA),
            issuer: settings.issuer.clone(),
            audience: settings.audience.clone(),
            access_token_lifetime: settings.access_token_lifetime,
        })
    }
}

impl TokenIssuer for EdDsaTokenIssuer {
    fn issue(&self, claims: &AccessTokenClaims) -> Result<String, CryptoError> {
        let wire = WireClaims {
            sub: claims.user_id.as_uuid(),
            org: claims.org_id.map(|id| id.as_uuid()),
            role: claims.org_role.map(role_to_str).map(str::to_owned),
            prole: platform_role_to_str(claims.platform_role).to_owned(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            exp: claims.expires_at.timestamp(),
            iat: Utc::now().timestamp(),
            jti: Uuid::now_v7(),
        };
        encode(&self.header, &wire, &self.encoding).map_err(|error| CryptoError::Backend {
            details: error.to_string(),
        })
    }

    fn verify(&self, token: &str) -> Result<AccessTokenClaims, CryptoError> {
        let decoded =
            decode::<WireClaims>(token, &self.decoding, &self.validation).map_err(|error| {
                match error.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => CryptoError::ExpiredToken,
                    _ => CryptoError::InvalidToken,
                }
            })?;
        let wire = decoded.claims;

        let expires_at: DateTime<Utc> = Utc
            .timestamp_opt(wire.exp, 0)
            .single()
            .ok_or(CryptoError::InvalidToken)?;

        let org_role = match wire.role.as_deref() {
            None => None,
            Some(value) => Some(role_from_str(value).ok_or(CryptoError::InvalidToken)?),
        };

        Ok(AccessTokenClaims {
            user_id: UserId::from_uuid(wire.sub),
            org_id: wire.org.map(OrgId::from_uuid),
            org_role,
            platform_role: platform_role_from_str(&wire.prole).ok_or(CryptoError::InvalidToken)?,
            expires_at,
        })
    }

    fn access_token_lifetime(&self) -> Duration {
        self.access_token_lifetime
    }
}

/// Claim sul filo. Nomi brevi perché ogni richiesta li trasporta in un header.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct WireClaims {
    sub: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    org: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    prole: String,
    iss: String,
    aud: String,
    exp: i64,
    iat: i64,
    jti: Uuid,
}

const fn role_to_str(role: OrgRole) -> &'static str {
    match role {
        OrgRole::Owner => "owner",
        OrgRole::Manager => "manager",
        OrgRole::Tenant => "tenant",
    }
}

fn role_from_str(value: &str) -> Option<OrgRole> {
    match value {
        "owner" => Some(OrgRole::Owner),
        "manager" => Some(OrgRole::Manager),
        "tenant" => Some(OrgRole::Tenant),
        _ => None,
    }
}

const fn platform_role_to_str(role: PlatformRole) -> &'static str {
    match role {
        PlatformRole::User => "user",
        PlatformRole::Support => "support",
        PlatformRole::Admin => "admin",
    }
}

fn platform_role_from_str(value: &str) -> Option<PlatformRole> {
    match value {
        "user" => Some(PlatformRole::User),
        "support" => Some(PlatformRole::Support),
        "admin" => Some(PlatformRole::Admin),
        _ => None,
    }
}
