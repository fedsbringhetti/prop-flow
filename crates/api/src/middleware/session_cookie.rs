//! Il cookie che porta il refresh token.
//!
//! # Perché un cookie e non il corpo della risposta
//!
//! L'access token vive **solo in memoria** JavaScript e muore al ricaricamento
//! della pagina; il refresh token vale trenta giorni e va protetto da uno script
//! che arrivi dopo il caricamento. `httpOnly` è ciò che glielo impedisce, e vale
//! soltanto se il valore **non compare anche altrove**: metterlo nel cookie *e*
//! nel JSON del login sarebbe metà misura a costo pieno, perché un XSS lo
//! leggerebbe dalla risposta. Per questo `SessionResponse` non lo porta più.
//!
//! # Perché non serve un token CSRF
//!
//! Il CSRF è possibile quando il browser autentica una richiesta **da solo**, cioè
//! con un cookie. Qui non accade, per due ragioni che si sommano:
//!
//! - ogni rotta di business è autenticata dall'header `Authorization`, che un sito
//!   attaccante non può impostare su una richiesta cross-origin senza il permesso
//!   di CORS — e CORS non è montato affatto, perché client e API stanno sulla
//!   stessa origine dietro un reverse proxy;
//! - le sole rotte autenticate dal cookie sono `/api/v1/auth/refresh` e
//!   `/api/v1/auth/logout`, e su quelle il cookie è `SameSite=Strict`: il browser
//!   non lo manda su una richiesta che parte da un altro sito.
//!
//! **L'argomento regge finché nessuna rotta di business accetta il cookie**, e il
//! giorno in cui una lo accettasse cadrebbe *in silenzio* — nessun test
//! fallirebbe da sé. È la ragione per cui esiste
//! `the_session_cookie_is_refused_by_business_routes` in
//! `crates/api/tests/http_contract.rs`: quel test *è* questa nota, in forma
//! eseguibile.
//!
//! `Path=/api/v1/auth` non è cosmetico: limita l'invio alle sole rotte che il
//! cookie usa, quindi una richiesta a `/api/v1/properties` non se lo porta dietro
//! nemmeno per sbaglio. È difesa in profondità — se un giorno l'argomento sopra si
//! incrinasse, il cookie comunque non arriverebbe dove non serve.

use axum::http::{HeaderMap, HeaderValue, header};

use crate::error::ApiError;

/// Nome del cookie.
///
/// **Senza i prefissi `__Host-` e `__Secure-`**, e non per distrazione: il primo
/// esige `Path=/`, che è precisamente ciò che qui non si vuole, e il secondo esige
/// `Secure`, che in sviluppo su `http://localhost` va tolto. Un prefisso che il
/// browser rifiuta silenziosamente sarebbe peggio di nessun prefisso.
pub const REFRESH_COOKIE: &str = "propflow_refresh";

/// Percorso su cui il browser manda il cookie: le sole rotte che lo usano.
pub const REFRESH_COOKIE_PATH: &str = "/api/v1/auth";

/// Legge il refresh token dal cookie della richiesta.
///
/// `None` copre tutti i modi di non averlo — header assente, non testo, cookie
/// assente fra gli altri, valore vuoto — perché per chi chiama sono la stessa
/// condizione e il rimedio è uno solo: rifare il login.
#[must_use]
pub fn refresh_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(name, value)| (name.trim() == REFRESH_COOKIE).then(|| value.trim().to_owned()))
        .filter(|value| !value.is_empty())
}

/// Il `Set-Cookie` che consegna un refresh token appena emesso.
///
/// `max_age_seconds` viene dalla sessione e non dalla configurazione: è la vita
/// **residua** del token che si sta consegnando, e derivarla di nuovo dal TTL
/// configurato la farebbe divergere dal valore che il database ha memorizzato.
pub fn set_refresh_cookie(
    token: &str,
    max_age_seconds: i64,
    secure: bool,
) -> Result<HeaderValue, ApiError> {
    header_value(&format!(
        "{REFRESH_COOKIE}={token}; Max-Age={}; {}",
        max_age_seconds.max(0),
        attributes(secure)
    ))
}

/// Il `Set-Cookie` che lo cancella, all'uscita.
///
/// `Max-Age=0` e non una data nel passato: la data richiederebbe di formattare un
/// timestamp HTTP e di sperare che l'orologio del browser sia d'accordo.
pub fn clear_refresh_cookie(secure: bool) -> Result<HeaderValue, ApiError> {
    header_value(&format!(
        "{REFRESH_COOKIE}=; Max-Age=0; {}",
        attributes(secure)
    ))
}

/// Gli attributi comuni alle due direttive.
///
/// **Devono coincidere**: un browser cancella un cookie solo se `Path` e `Domain`
/// del `Set-Cookie` che lo azzera sono gli stessi con cui era stato posato.
/// Comporli in un punto solo è ciò che impedisce a un logout di sembrare riuscito
/// lasciando il cookie dov'era.
fn attributes(secure: bool) -> String {
    // `Secure` è condizionale perché su `http://localhost` il browser non manda
    // un cookie che lo dichiara: la sessione durerebbe i quindici minuti
    // dell'access token e il refresh fallirebbe sempre, con un sintomo che non
    // nomina la causa. È l'unica configurazione che questa fetta introduce.
    let secure = if secure { "Secure; " } else { "" };
    format!("Path={REFRESH_COOKIE_PATH}; HttpOnly; {secure}SameSite=Strict")
}

/// Il valore è ASCII per costruzione — il token è base64url e il resto è testo
/// fisso — ma i lint vietano `expect`, e un cookie perso in silenzio sarebbe una
/// sessione che non si rinnova mai senza che nulla lo dica.
fn header_value(raw: &str) -> Result<HeaderValue, ApiError> {
    HeaderValue::from_str(raw).map_err(|_| {
        ApiError::from(propflow_application::AppError::Internal {
            details: "valore di Set-Cookie non rappresentabile".to_owned(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(raw: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, HeaderValue::from_str(raw).expect("cookie"));
        headers
    }

    #[test]
    fn the_cookie_is_found_among_others() {
        let headers = headers("theme=dark; propflow_refresh=abc123; consent=1");
        assert_eq!(refresh_cookie(&headers).as_deref(), Some("abc123"));
    }

    /// Un cookie il cui nome *finisce* come il nostro non è il nostro: senza il
    /// confronto sul nome intero, `altro_propflow_refresh` verrebbe accettato.
    #[test]
    fn a_similar_name_is_not_the_cookie() {
        let headers = headers("altro_propflow_refresh=abc123");
        assert_eq!(refresh_cookie(&headers), None);
    }

    #[test]
    fn an_empty_value_counts_as_absent() {
        assert_eq!(refresh_cookie(&headers("propflow_refresh=")), None);
        assert_eq!(refresh_cookie(&HeaderMap::new()), None);
    }

    /// In sviluppo `Secure` sparisce e nient'altro cambia: se cambiasse anche
    /// `Path`, il logout non cancellerebbe il cookie che il login ha posato.
    #[test]
    fn only_secure_depends_on_the_environment() {
        let secure = attributes(true);
        let insecure = attributes(false);
        assert_eq!(secure.replace("Secure; ", ""), insecure);
        assert!(insecure.contains("Path=/api/v1/auth"));
        assert!(insecure.contains("HttpOnly"));
        assert!(insecure.contains("SameSite=Strict"));
    }

    #[test]
    fn clearing_uses_the_same_attributes_as_setting() {
        let set = set_refresh_cookie("token", 100, true).expect("cookie");
        let cleared = clear_refresh_cookie(true).expect("cookie");
        let set = set.to_str().expect("ascii");
        let cleared = cleared.to_str().expect("ascii");
        let tail = |value: &str| {
            value
                .split_once("; ")
                .map(|(_, rest)| rest.to_owned())
                .unwrap_or_default()
        };
        assert_eq!(
            tail(&tail(set)),
            tail(&tail(cleared)),
            "gli attributi divergono: il logout non cancellerebbe il cookie"
        );
    }
}
