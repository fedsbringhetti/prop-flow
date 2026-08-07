use std::net::IpAddr;

use crate::error::{DomainError, DomainResult};
use crate::identity::OrgRole;
use crate::shared::{OrgId, UserId};

/// Contesto della richiesta in corso: chi agisce, per conto di quale
/// organizzazione, con quale ruolo.
///
/// È l'unico canale attraverso cui l'infrastruttura imposta la Row Level Security
/// (`app.set_request_context`). Il ruolo qui presente serve a rispondere `403`
/// prima di toccare il database: **l'autorizzazione autorevole resta la RLS**, che
/// rilegge la membership dal database e non si fida di questo valore.
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    user_id: Option<UserId>,
    org_id: Option<OrgId>,
    org_role: Option<OrgRole>,
    platform_admin: bool,
    request_id: Option<String>,
    client_ip: Option<IpAddr>,
    user_agent: Option<String>,
}

impl RequestContext {
    /// Contesto senza utente: registrazione e login, che avvengono prima
    /// dell'autenticazione. Non concede visibilità su alcuna riga.
    #[must_use]
    pub fn anonymous() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn authenticated(user_id: UserId) -> Self {
        Self {
            user_id: Some(user_id),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_org(mut self, org_id: OrgId, role: OrgRole) -> Self {
        self.org_id = Some(org_id);
        self.org_role = Some(role);
        self
    }

    #[must_use]
    pub fn with_platform_admin(mut self, platform_admin: bool) -> Self {
        self.platform_admin = platform_admin;
        self
    }

    #[must_use]
    pub fn with_transport(
        mut self,
        request_id: Option<String>,
        client_ip: Option<IpAddr>,
        user_agent: Option<String>,
    ) -> Self {
        self.request_id = request_id;
        self.client_ip = client_ip;
        self.user_agent = user_agent.map(|value| value.chars().take(512).collect());
        self
    }

    #[must_use]
    pub const fn user_id(&self) -> Option<UserId> {
        self.user_id
    }
    #[must_use]
    pub const fn org_id(&self) -> Option<OrgId> {
        self.org_id
    }
    #[must_use]
    pub const fn org_role(&self) -> Option<OrgRole> {
        self.org_role
    }
    #[must_use]
    pub const fn platform_admin(&self) -> bool {
        self.platform_admin
    }
    #[must_use]
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }
    #[must_use]
    pub const fn client_ip(&self) -> Option<IpAddr> {
        self.client_ip
    }
    #[must_use]
    pub fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }

    pub fn require_user(&self) -> DomainResult<UserId> {
        self.user_id
            .ok_or(DomainError::Forbidden("richiede autenticazione"))
    }

    pub fn require_org(&self) -> DomainResult<OrgId> {
        self.org_id.ok_or(DomainError::Forbidden(
            "richiede un'organizzazione selezionata",
        ))
    }

    /// `true` per owner e manager dell'organizzazione, o per un amministratore
    /// di piattaforma. Gli inquilini non sono staff.
    #[must_use]
    pub fn is_staff(&self) -> bool {
        self.platform_admin || self.org_role.is_some_and(OrgRole::is_staff)
    }
}
