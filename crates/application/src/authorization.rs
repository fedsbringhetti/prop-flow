use propflow_domain::identity::OrgRole;
use propflow_domain::shared::OrgId;
use propflow_domain::{Denial, RequestContext};

use crate::error::{AppError, AppResult};

/// Controlli di autorizzazione del service layer.
///
/// Non sostituiscono la Row Level Security: la duplicano di proposito. La RLS è
/// l'autorità — non aggirabile nemmeno da una query scritta a mano — ma da sola
/// risponderebbe "zero righe" dove il chiamante merita un `403`, e un `404` al
/// posto di un `403` nasconde un errore di permessi facendolo sembrare un bug.
pub fn ensure_authenticated(context: &RequestContext) -> AppResult<()> {
    context
        .user_id()
        .map(|_| ())
        .ok_or(AppError::Forbidden(Denial::AuthenticationRequired))
}

/// Richiede un'organizzazione selezionata e l'appartenenza a essa.
pub fn ensure_member(context: &RequestContext) -> AppResult<OrgId> {
    ensure_authenticated(context)?;
    let org_id = context
        .org_id()
        .ok_or(AppError::Forbidden(Denial::OrgNotSelected))?;
    if context.org_role().is_none() && !context.platform_admin() {
        return Err(AppError::Forbidden(Denial::NotAMember));
    }
    Ok(org_id)
}

/// Richiede il ruolo di staff (owner o manager): scritture sul patrimonio,
/// contratti, contabilità. Gli inquilini sono membri ma non staff.
pub fn ensure_staff(context: &RequestContext) -> AppResult<OrgId> {
    let org_id = ensure_member(context)?;
    if !context.is_staff() {
        return Err(AppError::Forbidden(Denial::StaffRoleRequired));
    }
    Ok(org_id)
}

/// Ruolo effettivo da registrare negli eventi di dominio.
#[must_use]
pub fn effective_role(context: &RequestContext) -> OrgRole {
    context.org_role().unwrap_or(OrgRole::Tenant)
}

#[cfg(test)]
mod tests {
    use super::*;
    use propflow_domain::shared::UserId;

    #[test]
    fn anonymous_context_is_refused() {
        let context = RequestContext::anonymous();
        assert!(ensure_authenticated(&context).is_err());
        assert!(ensure_member(&context).is_err());
        assert!(ensure_staff(&context).is_err());
    }

    #[test]
    fn tenant_is_member_but_not_staff() {
        let context = RequestContext::authenticated(UserId::generate())
            .with_org(OrgId::generate(), OrgRole::Tenant);
        assert!(ensure_member(&context).is_ok());
        assert!(ensure_staff(&context).is_err());
    }

    #[test]
    fn owner_and_manager_are_staff() {
        for role in [OrgRole::Owner, OrgRole::Manager] {
            let context =
                RequestContext::authenticated(UserId::generate()).with_org(OrgId::generate(), role);
            assert!(
                ensure_staff(&context).is_ok(),
                "{role:?} doveva essere staff"
            );
        }
    }

    #[test]
    fn authenticated_without_org_cannot_pass_member_check() {
        let context = RequestContext::authenticated(UserId::generate());
        assert!(ensure_member(&context).is_err());
    }
}
