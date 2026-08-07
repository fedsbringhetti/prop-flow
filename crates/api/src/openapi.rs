use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};

/// Documento OpenAPI generato dai derive sugli handler.
///
/// Generato e non scritto a mano: una specifica mantenuta a parte divergerebbe dal
/// codice alla prima modifica, e una specifica sbagliata è peggio di nessuna
/// specifica.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "PropFlow API",
        version = "0.1.0",
        description = "Piattaforma PropTech bidirezionale: proprietari e inquilini."
    ),
    paths(
        crate::routes::health::liveness,
        crate::routes::health::readiness,
        crate::routes::auth::register,
        crate::routes::auth::login,
        crate::routes::auth::refresh,
        crate::routes::auth::logout,
        crate::routes::properties::create,
        crate::routes::properties::list,
        crate::routes::properties::detail,
        crate::routes::properties::update,
        crate::routes::properties::archive,
        crate::routes::rooms::create,
        crate::routes::rooms::list,
        crate::routes::rooms::update,
        crate::routes::rooms::archive,
        crate::routes::leases::create,
        crate::routes::leases::list,
        crate::routes::leases::detail,
        crate::routes::leases::add_party,
        crate::routes::leases::activate,
        crate::routes::leases::terminate,
        crate::routes::invitations::invite,
        crate::routes::invitations::revoke,
        crate::routes::invitations::accept,
    ),
    components(schemas(
        crate::error::ProblemDetails,
        crate::error::FieldError,
        crate::routes::health::HealthResponse,
        crate::routes::auth::RegisterRequest,
        crate::routes::auth::RegisterResponse,
        crate::routes::auth::LoginRequest,
        crate::routes::auth::RefreshRequest,
        crate::routes::auth::SessionResponse,
        crate::routes::properties::CreatePropertyRequest,
        crate::routes::properties::UpdatePropertyRequest,
        crate::routes::properties::PropertyResponse,
        crate::routes::properties::PropertyPageResponse,
        crate::routes::properties::AddressDto,
        crate::routes::properties::PropertyTypeDto,
        crate::routes::properties::PropertyStatusDto,
        crate::routes::properties::EnergyClassDto,
        crate::routes::rooms::CreateRoomRequest,
        crate::routes::rooms::UpdateRoomRequest,
        crate::routes::rooms::RoomResponse,
        crate::routes::rooms::RoomListResponse,
        crate::routes::rooms::RoomStatusDto,
        crate::routes::leases::CreateLeaseRequest,
        crate::routes::leases::AddLeasePartyRequest,
        crate::routes::leases::TerminateLeaseRequest,
        crate::routes::leases::LeaseResponse,
        crate::routes::leases::LeaseDetailResponse,
        crate::routes::leases::LeasePartyResponse,
        crate::routes::leases::LeasePageResponse,
        crate::routes::leases::LeaseTypeDto,
        crate::routes::leases::LeaseStatusDto,
        crate::routes::leases::RentFrequencyDto,
        crate::routes::leases::LeasePartyRoleDto,
        crate::routes::invitations::InviteTenantRequest,
        crate::routes::invitations::InvitationResponse,
        crate::routes::invitations::AcceptInvitationRequest,
        crate::routes::invitations::AcceptInvitationResponse,
    )),
    modifiers(&BearerAuth),
    tags(
        (name = "autenticazione", description = "Registrazione, accesso e rotazione dei token"),
        (name = "immobili", description = "Gestione del patrimonio"),
        (name = "stanze", description = "Stanze di un immobile in co-living"),
        (name = "contratti", description = "Contratti di locazione e parti contrattuali"),
        (name = "inviti", description = "Onboarding dell'inquilino"),
        (name = "sistema", description = "Liveness e readiness"),
    )
)]
pub struct ApiDoc;

struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}
