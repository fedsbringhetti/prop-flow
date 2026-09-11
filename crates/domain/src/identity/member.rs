use chrono::{DateTime, Utc};

use crate::error::{DomainError, DomainResult, Invariant};
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

    /// Esce dall'organizzazione.
    ///
    /// **Uno stato, non un soft delete**, e le due cose non sono
    /// intercambiabili: `OrgMemberRepository::find` filtra `deleted_at IS NULL` e
    /// `org_members_org_user_uk` è unico e **non parziale**, quindi una membership
    /// cancellata non si vedrebbe più e chi è uscito non potrebbe più rientrare —
    /// né con un `INSERT`, che sbatte sul vincolo, né con un `UPDATE` su una riga
    /// che non si trova. È il difetto che la `0029` ha già pagato sulle spese
    /// co-living.
    ///
    /// **`joined_at` non si azzera.** Dice quando si è entrati, ed è un fatto:
    /// riscriverlo perderebbe l'unica traccia di quanto è durata l'appartenenza.
    /// Il rientro lo aggiorna, perché lì diventa falso.
    ///
    /// Il divieto «non si esce con un contratto in essere» **non è qui**: non è un
    /// invariante dell'aggregato — l'appartenenza non conosce i contratti — ed è
    /// garantito dalla `WITH CHECK` di `org_members_update_self`, con il caso
    /// d'uso a darne la versione parlante.
    pub const fn leave(&mut self) -> DomainResult<()> {
        if matches!(self.status, MemberStatus::Left) {
            return Err(DomainError::Invariant(Invariant::MembershipAlreadyLeft));
        }
        self.status = MemberStatus::Left;
        Ok(())
    }

    /// Riattiva un'appartenenza non attiva, accettando un invito nuovo.
    ///
    /// È il ramo che [ADR-051](../../../../docs/08-decisioni.md) aveva previsto
    /// per iscritto e non scritto, perché finché non si usciva non serviva a
    /// nessuno: senza, chi esce non rientra più — `AcceptInvitation` proverebbe un
    /// `INSERT` che sbatte su `org_members_org_user_uk` e diventa `409
    /// membership.already_exists`, cioè lo stesso `409` che quella voce aveva
    /// chiuso, ricomparso da un'altra porta.
    ///
    /// **Il ruolo non si tocca**, per la stessa ragione di ADR-051: chi era
    /// `owner` e viene reinvitato come `tenant` resta `owner`. L'invito serve a
    /// collegare la parte contrattuale, non a retrocedere qualcuno.
    ///
    /// `joined_at` invece sì: chi è tornato non è nell'organizzazione da prima di
    /// esserci tornato. È la stessa scelta della `0029`, ed è la ragione per cui
    /// quella colonna è fra quelle che la `0039` concede.
    pub const fn rejoin(&mut self, now: DateTime<Utc>) {
        self.status = MemberStatus::Active;
        self.joined_at = Some(now);
    }
}
