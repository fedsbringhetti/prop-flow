use crate::identity::{OrgRole, Slug};
use crate::shared::OrgId;

/// Un'organizzazione **vista da chi ne è membro**: il suo nome, e il ruolo che vi
/// si ha.
///
/// È una **proiezione** e non un aggregato, come `PropertyResident` e
/// `OwnerDashboard`: nasce da un join fra `app.org_members` e `app.organizations`,
/// nessuno la modifica, e non ha invarianti da difendere. Cercarle un costruttore
/// che applichi delle regole sarebbe cercare qualcosa che non esiste.
///
/// # Perché non è `(Organization, OrgMember)`
///
/// Perché quella coppia porterebbe fuso orario, valuta predefinita e natura
/// dell'organizzazione a chi ha chiesto un elenco per un menù, e perché
/// comporla richiederebbe una lettura delle organizzazioni per ogni
/// appartenenza — cioè un `N+1` su una domanda che una query sola sa risolvere.
#[derive(Debug, Clone)]
pub struct OrganizationMembership {
    org_id: OrgId,
    name: String,
    slug: Slug,
    role: OrgRole,
}

impl OrganizationMembership {
    #[must_use]
    pub const fn from_persistence(org_id: OrgId, name: String, slug: Slug, role: OrgRole) -> Self {
        Self {
            org_id,
            name,
            slug,
            role,
        }
    }

    #[must_use]
    pub const fn org_id(&self) -> OrgId {
        self.org_id
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub const fn slug(&self) -> &Slug {
        &self.slug
    }
    #[must_use]
    pub const fn role(&self) -> OrgRole {
        self.role
    }
}
