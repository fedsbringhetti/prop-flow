use std::sync::Arc;

use propflow_application::ports::{
    Notifier, ObjectStorage, OpaqueTokenFactory, PasswordHasher, RefreshTokenFactory, TokenIssuer,
};
use propflow_application::use_cases::account::{ChangeMyPassword, GetMyAccount, UpdateMyProfile};
use propflow_application::use_cases::ai::GetBillOcrStatus;
use propflow_application::use_cases::auth::{
    ConfirmPasswordReset, Login, Logout, RefreshSessionUseCase, RegisterOwner, RequestPasswordReset,
};
use propflow_application::use_cases::billing::{
    GetCharge, GetUtilityBill, IssueCharge, ListCharges, ListUtilityBills, MarkUtilityBillPaid,
    RechargeUtilityBill, RegisterUtilityBill, UpdateUtilityBill, ValidateUtilityBill, VoidCharge,
    VoidUtilityBill, WriteOffCharge,
};
use propflow_application::use_cases::document::{
    ArchiveDocument, DownloadDocument, ListDocuments, UploadDocument,
};
use propflow_application::use_cases::expense::{
    AddGroupMember, ArchiveExpenseGroup, CancelSettlement, ConfirmSettlement, CreateExpenseGroup,
    DeclareSettlement, GetExpense, GetExpenseGroup, GetGroupBalances, LeaveGroup,
    ListExpenseGroups, ListExpenses, ListSettlements, RecordExpense, RejectSettlement,
};
use propflow_application::use_cases::invitation::{
    AcceptInvitation, InviteTenant, RevokeInvitation,
};
use propflow_application::use_cases::lease::{
    ActivateLease, AddLeaseParty, CancelLease, CorrectLeaseParty, DraftLease, GetLease, ListLeases,
    RemoveLeaseParty, TerminateLease,
};
use propflow_application::use_cases::organization::{LeaveOrganization, ListOrganizations};
use propflow_application::use_cases::payment::{
    ConfirmPayment, DeclarePayment, GetPayment, ListPayments, RecordPayment, RejectPayment,
};
use propflow_application::use_cases::property::{
    ArchiveProperty, CreateProperty, GetProperty, ListProperties, ListPropertyResidents,
    UpdateProperty,
};
use propflow_application::use_cases::reporting::GetOwnerDashboard;
use propflow_application::use_cases::room::{ArchiveRoom, CreateRoom, ListRooms, UpdateRoom};
use propflow_application::use_cases::ticket::{
    AssignTicket, CancelTicket, CloseTicket, CorrectTicketReport, GetTicket, ListTicketMessages,
    ListTickets, OpenTicket, PostTicketMessage, ReopenTicket, StartTicketWork, SuspendTicket,
    TriageTicket, UpdateTicketCosts,
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
    /// Le due rotte del recupero della password: girano sul pool privilegiato,
    /// perché chi ha dimenticato la password non è autenticato (ADR-052).
    pub request_password_reset: Arc<RequestPasswordReset>,
    pub confirm_password_reset: Arc<ConfirmPasswordReset>,

    /// Le tre rotte che rispondono **senza un'organizzazione attiva**: dire quali
    /// si hanno, e leggere o correggere i propri dati, non appartiene a nessuna di
    /// esse. Pretendere `X-Org-Id` romperebbe proprio il caso in cui servono — chi
    /// è appena uscito dall'unica in cui abitava.
    pub list_organizations: Arc<ListOrganizations>,
    /// **Nessuna delle tre prende un identificatore di utente**, ed è la forma che
    /// tiene chiusa la domanda di ADR-036: chi si legge viene dal token.
    pub get_my_account: Arc<GetMyAccount>,
    pub update_my_profile: Arc<UpdateMyProfile>,
    /// Pretende la password corrente e revoca **tutte** le sessioni, per
    /// uniformità con il recupero (ADR-052): due regole diverse per due percorsi
    /// che una persona non distingue divergono alla prima modifica.
    pub change_my_password: Arc<ChangeMyPassword>,
    /// Uscire da un'organizzazione. `status = 'left'` e non un soft delete: la
    /// riga resta, ed è ciò che permette di rientrare accettando un invito nuovo.
    pub leave_organization: Arc<LeaveOrganization>,

    /// La sola lettura che attraversa tutti i moduli insieme, ed è per questo la
    /// prima superficie in cui si vedrebbe un errore di integrazione fra le fette.
    pub owner_dashboard: Arc<GetOwnerDashboard>,

    pub create_property: Arc<CreateProperty>,
    pub get_property: Arc<GetProperty>,
    pub list_properties: Arc<ListProperties>,
    pub update_property: Arc<UpdateProperty>,
    pub archive_property: Arc<ArchiveProperty>,

    pub create_room: Arc<CreateRoom>,
    pub list_property_residents: Arc<ListPropertyResidents>,
    pub list_rooms: Arc<ListRooms>,
    pub update_room: Arc<UpdateRoom>,
    pub archive_room: Arc<ArchiveRoom>,

    pub draft_lease: Arc<DraftLease>,
    pub get_lease: Arc<GetLease>,
    pub list_leases: Arc<ListLeases>,
    pub add_lease_party: Arc<AddLeaseParty>,
    pub activate_lease: Arc<ActivateLease>,
    pub terminate_lease: Arc<TerminateLease>,
    pub cancel_lease: Arc<CancelLease>,
    pub correct_lease_party: Arc<CorrectLeaseParty>,
    pub remove_lease_party: Arc<RemoveLeaseParty>,

    pub upload_document: Arc<UploadDocument>,
    pub list_documents: Arc<ListDocuments>,
    pub download_document: Arc<DownloadDocument>,
    pub archive_document: Arc<ArchiveDocument>,

    pub open_ticket: Arc<OpenTicket>,
    pub get_ticket: Arc<GetTicket>,
    pub list_tickets: Arc<ListTickets>,
    pub update_ticket_costs: Arc<UpdateTicketCosts>,
    /// La correzione del testo è **un caso d'uso a sé** e non un ramo di
    /// `UpdateTicketCosts`: le due hanno autorizzazioni diverse — i costi allo
    /// staff, il testo anche a chi ha segnalato finché è aperta — e un'unica rotta
    /// che scegliesse in base ai campi presenti nel corpo sbaglierebbe in silenzio.
    pub correct_ticket_report: Arc<CorrectTicketReport>,
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
    pub mark_utility_bill_paid: Arc<MarkUtilityBillPaid>,
    /// Il riaddebito è l'**unica** via per cui una bolletta e un addebito si
    /// legano: `POST /leases/{id}/charges` non accetta `utility_bill_id`.
    pub recharge_utility_bill: Arc<RechargeUtilityBill>,
    /// **Sola lettura**, e non c'è un gemello che scrive: il job lo accoda il
    /// caricamento di una scansione e lo esegue il worker. Una rotta che
    /// lanciasse l'OCR a comando legherebbe l'esito di una richiesta HTTP alla
    /// disponibilità di un fornitore esterno, che è la decisione 6 del piano.
    pub get_bill_ocr_status: Arc<GetBillOcrStatus>,

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

    /// Creare un gruppo e **iscriverci chi lo crea** è una transazione sola: lo
    /// schema, senza la migration `0026`, produceva un gruppo inutilizzabile da
    /// chi lo aveva creato.
    pub create_expense_group: Arc<CreateExpenseGroup>,
    pub get_expense_group: Arc<GetExpenseGroup>,
    pub list_expense_groups: Arc<ListExpenseGroups>,
    pub archive_expense_group: Arc<ArchiveExpenseGroup>,
    pub add_group_member: Arc<AddGroupMember>,
    /// Uscire valorizza `left_at` e **non cancella la riga**: dalla migration
    /// `0027` chi esce con un conto aperto resta nei saldi.
    pub leave_group: Arc<LeaveGroup>,
    pub record_expense: Arc<RecordExpense>,
    pub get_expense: Arc<GetExpense>,
    pub list_expenses: Arc<ListExpenses>,
    /// I saldi si **leggono** dalla vista e non si ricalcolano: due definizioni
    /// dello stesso numero si contraddicono al primo caso limite.
    pub get_group_balances: Arc<GetGroupBalances>,
    /// **A confermare è chi riceve**, non lo staff: è l'unico che sa se il
    /// bonifico è arrivato.
    pub declare_settlement: Arc<DeclareSettlement>,
    pub confirm_settlement: Arc<ConfirmSettlement>,
    pub reject_settlement: Arc<RejectSettlement>,
    /// Il ritiro di chi ha dichiarato, che **fra i pagamenti non esiste**:
    /// `settlements_update` ammette `from_user_id`, `payments_update` no.
    pub cancel_settlement: Arc<CancelSettlement>,
    pub list_settlements: Arc<ListSettlements>,

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
    /// Usati dall'endpoint di prontezza.
    ///
    /// Il canale di posta ci sta accanto al database e non per completezza: l'invio
    /// è best-effort dopo il commit, quindi un relay caduto è **muto** — e la posta
    /// è l'unica via verso chi non riesce più a entrare. Vedi `Notifier::probe`.
    pub pools: DatabasePools,
    pub notifier: Arc<dyn Notifier>,
    /// Se il cookie del refresh token debba dichiarare `Secure`.
    ///
    /// Vive nello stato e non nella firma di `router()` perché a leggerlo sono gli
    /// handler dell'autenticazione, non i layer: passarlo come quinto parametro
    /// posizionale accanto a due limiti e un timeout sarebbe un `bool` anonimo
    /// fra numeri, cioè un argomento che si scambia senza che nulla protesti.
    pub secure_cookies: bool,
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
    /// Il canale verso l'esterno. **Obbligatorio**, non facoltativo: da quando
    /// esiste il recupero della password è l'unica via verso chi è chiuso fuori dal
    /// proprio account, e un `Notifier` assente produrrebbe una richiesta che
    /// risponde `202` senza mandare niente.
    pub notifier: Arc<dyn Notifier>,
    /// Vita dell'URL di download pre-firmato. Vive qui e non dentro l'adattatore
    /// perché è una decisione applicativa — quanto a lungo vale un permesso — e non
    /// un dettaglio del fornitore.
    pub presign_ttl_seconds: u32,
    /// Il nome del fornitore di OCR a cui i job accodati sono destinati.
    ///
    /// **Una stringa e non una porta**: l'API non deve poter chiamare l'OCR. Se
    /// `Arc<dyn OcrProvider>` fosse qui, un handler potrebbe leggere una bolletta
    /// dentro una richiesta — che è ciò che la decisione 6 del piano esclude,
    /// perché legherebbe l'esito di un caricamento alla disponibilità di un
    /// fornitore esterno e ne moltiplicherebbe il costo per ogni ritentativo
    /// dell'utente. Il valore lo passa `main.rs`, che è anche il posto dove il
    /// worker costruisce l'adattatore vero: se i due divergessero, `provider_name`
    /// sui job direbbe una cosa e `model` un'altra.
    pub ocr_provider_name: String,
    pub pools: DatabasePools,
    /// Vedi [`AppState::secure_cookies`]. `false` solo in sviluppo su HTTP.
    pub secure_cookies: bool,
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
            notifier,
            presign_ttl_seconds,
            ocr_provider_name,
            pools,
            secure_cookies,
        } = dependencies;

        Self {
            register_owner: Arc::new(RegisterOwner::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&hasher),
                Arc::clone(&clock),
            )),
            login: Arc::new(Login::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&hasher),
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
            list_organizations: Arc::new(ListOrganizations::new(Arc::clone(&unit_of_work))),
            get_my_account: Arc::new(GetMyAccount::new(Arc::clone(&unit_of_work))),
            update_my_profile: Arc::new(UpdateMyProfile::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            change_my_password: Arc::new(ChangeMyPassword::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&hasher),
                Arc::clone(&clock),
            )),
            leave_organization: Arc::new(LeaveOrganization::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            owner_dashboard: Arc::new(GetOwnerDashboard::new(Arc::clone(&unit_of_work))),
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
            list_property_residents: Arc::new(ListPropertyResidents::new(Arc::clone(
                &unit_of_work,
            ))),
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
            cancel_lease: Arc::new(CancelLease::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            correct_lease_party: Arc::new(CorrectLeaseParty::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            remove_lease_party: Arc::new(RemoveLeaseParty::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            upload_document: Arc::new(UploadDocument::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&object_storage),
                Arc::clone(&clock),
                ocr_provider_name,
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
            correct_ticket_report: Arc::new(CorrectTicketReport::new(
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
            mark_utility_bill_paid: Arc::new(MarkUtilityBillPaid::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            recharge_utility_bill: Arc::new(RechargeUtilityBill::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            get_bill_ocr_status: Arc::new(GetBillOcrStatus::new(Arc::clone(&unit_of_work))),
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
            create_expense_group: Arc::new(CreateExpenseGroup::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            get_expense_group: Arc::new(GetExpenseGroup::new(Arc::clone(&unit_of_work))),
            list_expense_groups: Arc::new(ListExpenseGroups::new(Arc::clone(&unit_of_work))),
            archive_expense_group: Arc::new(ArchiveExpenseGroup::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            add_group_member: Arc::new(AddGroupMember::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            leave_group: Arc::new(LeaveGroup::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            record_expense: Arc::new(RecordExpense::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            get_expense: Arc::new(GetExpense::new(Arc::clone(&unit_of_work))),
            list_expenses: Arc::new(ListExpenses::new(Arc::clone(&unit_of_work))),
            get_group_balances: Arc::new(GetGroupBalances::new(Arc::clone(&unit_of_work))),
            declare_settlement: Arc::new(DeclareSettlement::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            confirm_settlement: Arc::new(ConfirmSettlement::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            reject_settlement: Arc::new(RejectSettlement::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            cancel_settlement: Arc::new(CancelSettlement::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            list_settlements: Arc::new(ListSettlements::new(Arc::clone(&unit_of_work))),
            request_password_reset: Arc::new(RequestPasswordReset::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&invitation_tokens),
                Arc::clone(&notifier),
                Arc::clone(&clock),
            )),
            confirm_password_reset: Arc::new(ConfirmPasswordReset::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&invitation_tokens),
                Arc::clone(&hasher),
                Arc::clone(&clock),
            )),
            invite_tenant: Arc::new(InviteTenant::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&invitation_tokens),
                Arc::clone(&clock),
                Arc::clone(&notifier),
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
            notifier,
            pools,
            secure_cookies,
        }
    }
}
