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

/// Chi può vedere il **costo stimato** di una segnalazione.
///
/// La regola vive qui e non al bordo HTTP perché è autorizzazione, non
/// presentazione: il bordo la applica alla proiezione, ma non la decide.
///
/// `estimated_cost` è la cifra di una trattativa con un fornitore e non riguarda
/// chi abita la casa; `actual_cost` sì, ed è visibile a chiunque veda la
/// segnalazione. La distinzione **non è un confine di sicurezza**: la Row Level
/// Security lascia comunque leggere la riga a chi ha segnalato, quindi questa è
/// una regola del contratto dell'API. Renderla un confine vero richiederebbe
/// privilegi di colonna in `SELECT` o una policy separata.
///
/// La direzione è deliberata: aggiungere un campo a una risposta è additivo,
/// toglierlo è una rottura.
#[must_use]
pub fn discloses_estimated_cost(context: &RequestContext) -> bool {
    context.is_staff()
}

/// Chi può vedere **quanto di una bolletta è già stato riaddebitato**.
///
/// Come [`discloses_estimated_cost`], la regola sta qui e non al bordo HTTP: è
/// autorizzazione, non presentazione.
///
/// A differenza della stima, però, qui la ragione non è solo di contratto: la somma
/// si legge **sotto Row Level Security**, e `charges_select` mostra all'inquilino le
/// sole quote sue. Il numero che riceverebbe sarebbe **parziale e indistinguibile
/// da un totale** — riaddebitato 90 su una bolletta da 184,20 che è stata
/// riaddebitata per intero a tre coinquilini. Un numero sbagliato è peggio di un
/// numero assente, e a chi abita quel totale non serve: è il conto di chi
/// distribuisce.
#[must_use]
pub fn discloses_recharged_total(context: &RequestContext) -> bool {
    context.is_staff()
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

    /// La stima non è per chi abita la casa. Il test fissa la regola dove è
    /// scritta, così un cambiamento al bordo HTTP non può spostarla di nascosto.
    #[test]
    fn only_staff_sees_the_estimated_cost() {
        let tenant = RequestContext::authenticated(UserId::generate())
            .with_org(OrgId::generate(), OrgRole::Tenant);
        assert!(!discloses_estimated_cost(&tenant));

        for role in [OrgRole::Owner, OrgRole::Manager] {
            let staff =
                RequestContext::authenticated(UserId::generate()).with_org(OrgId::generate(), role);
            assert!(discloses_estimated_cost(&staff), "{role:?}");
        }
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
