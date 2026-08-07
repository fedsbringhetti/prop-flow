use chrono::{DateTime, Utc};

use crate::shared::{OrgId, UserId};

/// Ruolo di un utente dentro un'organizzazione.
///
/// Non è un permesso: è un'appartenenza. I permessi derivano da questo ruolo
/// nelle policy RLS (`app.is_org_staff`) e nel service layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrgRole {
    Owner,
    Manager,
    Tenant,
}

impl OrgRole {
    /// Owner e manager amministrano l'organizzazione; l'inquilino vede solo
    /// ciò che è legato ai propri contratti.
    #[must_use]
    pub const fn is_staff(self) -> bool {
        matches!(self, Self::Owner | Self::Manager)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Manager => "manager",
            Self::Tenant => "tenant",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberStatus {
    Invited,
    Active,
    Suspended,
    Left,
}

#[derive(Debug, Clone)]
pub struct OrgMember {
    org_id: OrgId,
    user_id: UserId,
    role: OrgRole,
    status: MemberStatus,
    joined_at: Option<DateTime<Utc>>,
}

impl OrgMember {
    /// Membro attivo: usata al bootstrap dell'organizzazione personale e
    /// all'accettazione di un invito.
    #[must_use]
    pub const fn active(
        org_id: OrgId,
        user_id: UserId,
        role: OrgRole,
        joined_at: DateTime<Utc>,
    ) -> Self {
        Self {
            org_id,
            user_id,
            role,
            status: MemberStatus::Active,
            joined_at: Some(joined_at),
        }
    }

    /// Ricostruzione da persistenza: non riesegue le invarianti, perché i dati
    /// sono già stati validati quando sono stati scritti.
    #[must_use]
    pub const fn from_persistence(
        org_id: OrgId,
        user_id: UserId,
        role: OrgRole,
        status: MemberStatus,
        joined_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            org_id,
            user_id,
            role,
            status,
            joined_at,
        }
    }

    #[must_use]
    pub const fn org_id(&self) -> OrgId {
        self.org_id
    }
    #[must_use]
    pub const fn user_id(&self) -> UserId {
        self.user_id
    }
    #[must_use]
    pub const fn role(&self) -> OrgRole {
        self.role
    }
    #[must_use]
    pub const fn status(&self) -> MemberStatus {
        self.status
    }
    #[must_use]
    pub const fn joined_at(&self) -> Option<DateTime<Utc>> {
        self.joined_at
    }
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self.status, MemberStatus::Active)
    }
}
