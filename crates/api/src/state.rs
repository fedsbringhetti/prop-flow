use std::sync::Arc;

use propflow_application::ports::{
    ObjectStorage, OpaqueTokenFactory, PasswordHasher, RefreshTokenFactory, TokenIssuer,
};
use propflow_application::use_cases::auth::{Login, Logout, RefreshSessionUseCase, RegisterOwner};
use propflow_application::use_cases::billing::{
    GetCharge, GetUtilityBill, IssueCharge, ListCharges, ListUtilityBills, RechargeUtilityBill,
    RegisterUtilityBill, UpdateUtilityBill, ValidateUtilityBill, VoidCharge, VoidUtilityBill,
    WriteOffCharge,
};
use propflow_application::use_cases::document::{
    ArchiveDocument, DownloadDocument, ListDocuments, UploadDocument,
};
use propflow_application::use_cases::invitation::{
    AcceptInvitation, InviteTenant, RevokeInvitation,
};
use propflow_application::use_cases::lease::{
    ActivateLease, AddLeaseParty, DraftLease, GetLease, ListLeases, TerminateLease,
};
use propflow_application::use_cases::payment::{
    ConfirmPayment, DeclarePayment, GetPayment, ListPayments, RecordPayment, RejectPayment,
};
use propflow_application::use_cases::property::{
    ArchiveProperty, CreateProperty, GetProperty, ListProperties, UpdateProperty,
};
use propflow_application::use_cases::room::{ArchiveRoom, CreateRoom, ListRooms, UpdateRoom};
use propflow_application::use_cases::ticket::{
    AssignTicket, CancelTicket, CloseTicket, GetTicket, ListTicketMessages, ListTickets,
    OpenTicket, PostTicketMessage, ReopenTicket, StartTicketWork, SuspendTicket, TriageTicket,
    UpdateTicketCosts,
};
use propflow_domain::ports::UnitOfWorkFactory;
use propflow_infrastructure::DatabasePools;

/// Contenitore delle dipendenze passato agli handler.
///
/// Gli handler vedono solo use case e porte: nessun `PgPool`, nessun tipo di
/// SQLx. È il confine che impedisce a un handler di scrivere una query, e quindi
/// di aggirare il service layer e la Row Level Security.
#[derive(Clone)]
pub struct AppState {
    pub register_owner: Arc<RegisterOwner>,
    pub login: Arc<Login>,
    pub refresh_session: Arc<RefreshSessionUseCase>,
    pub logout: Arc<Logout>,

    pub create_property: Arc<CreateProperty>,
    pub get_property: Arc<GetProperty>,
    pub list_properties: Arc<ListProperties>,
    pub update_property: Arc<UpdateProperty>,
    pub archive_property: Arc<ArchiveProperty>,

    pub create_room: Arc<CreateRoom>,
    pub list_rooms: Arc<ListRooms>,
    pub update_room: Arc<UpdateRoom>,
    pub archive_room: Arc<ArchiveRoom>,

    pub draft_lease: Arc<DraftLease>,
    pub get_lease: Arc<GetLease>,
    pub list_leases: Arc<ListLeases>,
    pub add_lease_party: Arc<AddLeaseParty>,
    pub activate_lease: Arc<ActivateLease>,
    pub terminate_lease: Arc<TerminateLease>,

    pub upload_document: Arc<UploadDocument>,
    pub list_documents: Arc<ListDocuments>,
    pub download_document: Arc<DownloadDocument>,
    pub archive_document: Arc<ArchiveDocument>,

    pub open_ticket: Arc<OpenTicket>,
    pub get_ticket: Arc<GetTicket>,
    pub list_tickets: Arc<ListTickets>,
    pub update_ticket_costs: Arc<UpdateTicketCosts>,
    pub post_ticket_message: Arc<PostTicketMessage>,
    pub list_ticket_messages: Arc<ListTicketMessages>,
    /// Una transizione per caso d'uso: ognuna autorizza in modo proprio — la
    /// riapertura spetta a chi ha segnalato, l'annullamento a lui solo finché
    /// nessuno ci ha messo mano — e ognuna ha effetti che un `PATCH` sullo stato
    /// produrrebbe di soppiatto.
    pub triage_ticket: Arc<TriageTicket>,
    pub assign_ticket: Arc<AssignTicket>,
    pub start_ticket_work: Arc<StartTicketWork>,
    pub suspend_ticket: Arc<SuspendTicket>,
    pub close_ticket: Arc<CloseTicket>,
    pub reopen_ticket: Arc<ReopenTicket>,
    pub cancel_ticket: Arc<CancelTicket>,

    pub register_utility_bill: Arc<RegisterUtilityBill>,
    pub get_utility_bill: Arc<GetUtilityBill>,
    pub list_utility_bills: Arc<ListUtilityBills>,
    pub update_utility_bill: Arc<UpdateUtilityBill>,
    pub validate_utility_bill: Arc<ValidateUtilityBill>,
    pub void_utility_bill: Arc<VoidUtilityBill>,
    /// Il riaddebito è l'**unica** via per cui una bolletta e un addebito si
    /// legano: `POST /leases/{id}/charges` non accetta `utility_bill_id`.
    pub recharge_utility_bill: Arc<RechargeUtilityBill>,

    pub issue_charge: Arc<IssueCharge>,
    pub get_charge: Arc<GetCharge>,
    pub list_charges: Arc<ListCharges>,
    /// Le due sole transizioni dell'addebito: non esiste un `PATCH`, perché una
    /// pretesa già comunicata non si riscrive.
    pub void_charge: Arc<VoidCharge>,
    pub write_off_charge: Arc<WriteOffCharge>,

    /// Due vie di ingresso e non una: registrare un incasso e dichiarare di aver
    /// pagato producono righe con stati diversi, e a distinguerle è chi chiama —
    /// che è autorizzazione, quindi service layer e non handler.
    pub record_payment: Arc<RecordPayment>,
    pub declare_payment: Arc<DeclarePayment>,
    /// Le due uscite di `pending`, nate insieme a lui: uno stato in cui si entra e
    /// da cui non si esce è peggio di nessuno stato.
    pub confirm_payment: Arc<ConfirmPayment>,
    pub reject_payment: Arc<RejectPayment>,
    pub get_payment: Arc<GetPayment>,
    pub list_payments: Arc<ListPayments>,

    pub invite_tenant: Arc<InviteTenant>,
    pub revoke_invitation: Arc<RevokeInvitation>,
    pub accept_invitation: Arc<AcceptInvitation>,

    /// Serve al middleware di autenticazione per verificare gli access token.
    pub tokens: Arc<dyn TokenIssuer>,
    /// Serve al bordo HTTP per derivare `effective_status` di un addebito quando
    /// la risposta non viene dalla vista dei saldi — emissione e transizioni, che
    /// non pagano il join. La data viene da qui e non da `Utc::now()` perché
    /// l'orologio è iniettato e i test lo sostituiscono.
    pub clock: Arc<dyn propflow_domain::ports::Clock>,
    /// Serve al middleware per risolvere il ruolo quando il client cambia
    /// organizzazione con l'header `X-Org-Id`.
    pub unit_of_work: Arc<dyn UnitOfWorkFactory>,
    /// Usato solo dall'endpoint di prontezza.
    pub pools: DatabasePools,
}

/// Componenti concrete costruite in `main`, prima di essere composte negli use case.
pub struct Dependencies {
    pub unit_of_work: Arc<dyn UnitOfWorkFactory>,
    pub hasher: Arc<dyn PasswordHasher>,
    pub tokens: Arc<dyn TokenIssuer>,
    pub refresh_tokens: Arc<dyn RefreshTokenFactory>,
    pub invitation_tokens: Arc<dyn OpaqueTokenFactory>,
    pub clock: Arc<dyn propflow_domain::ports::Clock>,
    pub object_storage: Arc<dyn ObjectStorage>,
    /// Vita dell'URL di download pre-firmato. Vive qui e non dentro l'adattatore
    /// perché è una decisione applicativa — quanto a lungo vale un permesso — e non
    /// un dettaglio del fornitore.
    pub presign_ttl_seconds: u32,
    pub pools: DatabasePools,
}

impl AppState {
    #[must_use]
    pub fn new(dependencies: Dependencies) -> Self {
        let Dependencies {
            unit_of_work,
            hasher,
            tokens,
            refresh_tokens,
            invitation_tokens,
            clock,
            object_storage,
            presign_ttl_seconds,
            pools,
        } = dependencies;

        Self {
            register_owner: Arc::new(RegisterOwner::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&hasher),
                Arc::clone(&clock),
            )),
            login: Arc::new(Login::new(
                Arc::clone(&unit_of_work),
                hasher,
                Arc::clone(&tokens),
                Arc::clone(&refresh_tokens),
                Arc::clone(&clock),
            )),
            refresh_session: Arc::new(RefreshSessionUseCase::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&tokens),
                Arc::clone(&refresh_tokens),
                Arc::clone(&clock),
            )),
            logout: Arc::new(Logout::new(
                Arc::clone(&unit_of_work),
                refresh_tokens,
                Arc::clone(&clock),
            )),
            create_property: Arc::new(CreateProperty::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            get_property: Arc::new(GetProperty::new(Arc::clone(&unit_of_work))),
            list_properties: Arc::new(ListProperties::new(Arc::clone(&unit_of_work))),
            update_property: Arc::new(UpdateProperty::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            archive_property: Arc::new(ArchiveProperty::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            create_room: Arc::new(CreateRoom::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            list_rooms: Arc::new(ListRooms::new(Arc::clone(&unit_of_work))),
            update_room: Arc::new(UpdateRoom::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            archive_room: Arc::new(ArchiveRoom::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            draft_lease: Arc::new(DraftLease::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            get_lease: Arc::new(GetLease::new(Arc::clone(&unit_of_work), Arc::clone(&clock))),
            list_leases: Arc::new(ListLeases::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            add_lease_party: Arc::new(AddLeaseParty::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            activate_lease: Arc::new(ActivateLease::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            terminate_lease: Arc::new(TerminateLease::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            upload_document: Arc::new(UploadDocument::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&object_storage),
                Arc::clone(&clock),
            )),
            list_documents: Arc::new(ListDocuments::new(Arc::clone(&unit_of_work))),
            download_document: Arc::new(DownloadDocument::new(
                Arc::clone(&unit_of_work),
                object_storage,
                presign_ttl_seconds,
            )),
            archive_document: Arc::new(ArchiveDocument::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            open_ticket: Arc::new(OpenTicket::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            get_ticket: Arc::new(GetTicket::new(Arc::clone(&unit_of_work))),
            list_tickets: Arc::new(ListTickets::new(Arc::clone(&unit_of_work))),
            update_ticket_costs: Arc::new(UpdateTicketCosts::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            post_ticket_message: Arc::new(PostTicketMessage::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            list_ticket_messages: Arc::new(ListTicketMessages::new(Arc::clone(&unit_of_work))),
            triage_ticket: Arc::new(TriageTicket::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            assign_ticket: Arc::new(AssignTicket::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            start_ticket_work: Arc::new(StartTicketWork::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            suspend_ticket: Arc::new(SuspendTicket::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            close_ticket: Arc::new(CloseTicket::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            reopen_ticket: Arc::new(ReopenTicket::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            cancel_ticket: Arc::new(CancelTicket::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            register_utility_bill: Arc::new(RegisterUtilityBill::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            get_utility_bill: Arc::new(GetUtilityBill::new(Arc::clone(&unit_of_work))),
            list_utility_bills: Arc::new(ListUtilityBills::new(Arc::clone(&unit_of_work))),
            update_utility_bill: Arc::new(UpdateUtilityBill::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            validate_utility_bill: Arc::new(ValidateUtilityBill::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            void_utility_bill: Arc::new(VoidUtilityBill::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            recharge_utility_bill: Arc::new(RechargeUtilityBill::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            issue_charge: Arc::new(IssueCharge::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            get_charge: Arc::new(GetCharge::new(Arc::clone(&unit_of_work))),
            list_charges: Arc::new(ListCharges::new(Arc::clone(&unit_of_work))),
            void_charge: Arc::new(VoidCharge::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            write_off_charge: Arc::new(WriteOffCharge::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            record_payment: Arc::new(RecordPayment::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            declare_payment: Arc::new(DeclarePayment::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            confirm_payment: Arc::new(ConfirmPayment::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            reject_payment: Arc::new(RejectPayment::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            get_payment: Arc::new(GetPayment::new(Arc::clone(&unit_of_work))),
            list_payments: Arc::new(ListPayments::new(Arc::clone(&unit_of_work))),
            invite_tenant: Arc::new(InviteTenant::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&invitation_tokens),
                Arc::clone(&clock),
            )),
            revoke_invitation: Arc::new(RevokeInvitation::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            accept_invitation: Arc::new(AcceptInvitation::new(
                Arc::clone(&unit_of_work),
                invitation_tokens,
                Arc::clone(&clock),
            )),
            tokens,
            clock,
            unit_of_work,
            pools,
        }
    }
}
