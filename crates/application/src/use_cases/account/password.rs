use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::events::{
    DomainEvent, DomainEventPayload, EventMetadata, UserPasswordChanged,
};
use propflow_domain::identity::RevocationReason;
use propflow_domain::ports::{Clock, UnitOfWorkFactory};

use crate::authorization;
use crate::error::{AppError, AppResult};
use crate::password_policy;
use crate::ports::PasswordHasher;

#[derive(Debug, Clone)]
pub struct ChangeMyPasswordCommand {
    /// Quella corrente. Non finisce nei log, come ogni altra credenziale.
    pub current_password: String,
    pub new_password: String,
}

/// Cambiare la password da dentro: **pretende quella vecchia**, e chiude tutto.
///
/// # Perché la vecchia serve, visto che il chiamante è già autenticato
///
/// Perché il contesto è diverso da quello del recupero. Là la prova d'identità è
/// il controllo della casella di posta; qui è **una sessione**, e una sessione può
/// essere una scheda lasciata aperta. Senza la vecchia, chiunque si sieda davanti
/// a un browser non bloccato si prende l'account per sempre — cambia la password e
/// da quel momento il proprietario è quello che resta fuori.
///
/// # Revoca **tutte** le sessioni, compresa la propria
///
/// Uniformità con [ADR-052](../../../../../docs/08-decisioni.md), che fa lo stesso
/// per il recupero, ed è la decisione meno ovvia di questa fetta. L'alternativa che
/// sembra più gentile — tenere viva la sessione da cui si sta cambiando,
/// riconoscendola dal cookie di refresh che la richiesta porta già — è stata
/// scartata perché *«cambiare la password chiude tutto»* è **una frase sola**, e
/// quella frase va scritta in almeno quattro posti: la schermata del recupero,
/// quella del proprio account, `docs/04-sicurezza.md` e il corpo dell'API. Due
/// regole diverse per due percorsi che una persona non distingue — *«ho cambiato la
/// password»* è la stessa cosa dai due lati — sono due frasi da tenere allineate, e
/// la seconda diverge alla prima modifica.
///
/// L'attrito che si paga è piccolo e visibile: chi cambia rientra subito con la
/// password che ha appena scelto.
///
/// **E «tutte le sessioni» sono i refresh token, non gli access token.** L'access
/// token è un JWT firmato e non è revocabile ([ADR-012](../../../../../docs/08-decisioni.md)):
/// chi ne ha uno in mano resta dentro fino alla scadenza, quindici minuti. Ciò che
/// muore all'istante è il **rinnovo**. È la nota di ADR-052 che si dimentica, ed è
/// la stessa su cui un test della 5-quater aveva asserito la cosa sbagliata.
///
/// # Pool **applicativo**, e qui è la regola 27 a dirlo
///
/// Chi cambia la password è autenticato, quindi un contesto di sicurezza esiste e
/// `begin_privileged()` non è ammesso. È la differenza con il recupero, che gira
/// privilegiato perché là il contesto non c'è affatto.
///
/// Ne segue che tre policy devono reggere, e reggono: `users_select` ammette
/// `id = app.current_user_id()`, `users_update_self` ammette la propria riga, e
/// `user_sessions_own` è `user_id = app.current_user_id()`. La scrittura passa da
/// `update_profile`, che nomina le sole colonne che la `0038` concede.
pub struct ChangeMyPassword {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    hasher: Arc<dyn PasswordHasher>,
    clock: Arc<dyn Clock>,
}

impl ChangeMyPassword {
    #[must_use]
    pub const fn new(
        unit_of_work: Arc<dyn UnitOfWorkFactory>,
        hasher: Arc<dyn PasswordHasher>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            unit_of_work,
            hasher,
            clock,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        command: ChangeMyPasswordCommand,
    ) -> AppResult<()> {
        authorization::ensure_authenticated(context)?;
        let user_id = context.require_user()?;
        // La politica **prima** di toccare il database, come nel recupero: una
        // password debole è un errore del chiamante e merita un `422` che nomina il
        // campo, mentre la vecchia sbagliata non lo è mai — quella è un `401`.
        password_policy::validate(&command.new_password)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut user = uow
            .users()
            .find_by_id(user_id)
            .await?
            .ok_or(AppError::Unauthenticated)?;

        // Un account senza credenziali locali — invitato e mai registrato con una
        // password — non ha una vecchia da confrontare: la risposta è la stessa di
        // una vecchia sbagliata, perché il rimedio è il recupero in entrambi i casi.
        let Some(stored) = user.password_hash().cloned() else {
            return Err(AppError::Unauthenticated);
        };
        let matches = self
            .hasher
            .verify(&command.current_password, &stored)
            .await
            .map_err(AppError::internal)?;
        if !matches {
            // **Nessuna scrittura, e nessun contatore.** Il blocco dopo dieci
            // tentativi difende il *login*, dove chi prova non è autenticato; qui
            // chi prova ha già una sessione, e incrementarlo darebbe a chiunque
            // rubi una scheda il modo di bloccare fuori il legittimo proprietario.
            // La `0038` non concede comunque quella colonna a questo pool.
            return Err(AppError::Unauthenticated);
        }

        let password_hash = self
            .hasher
            .hash(&command.new_password)
            .await
            .map_err(AppError::internal)?;
        // `change_password` e non `set_password`: il secondo azzera il contatore dei
        // tentativi e il blocco, che questo pool non ha il privilegio di scrivere.
        user.change_password(password_hash);
        uow.users().update_profile(&user).await?;

        // **Nella stessa transazione della scrittura.** Se le due cose si
        // separassero ci sarebbe una finestra in cui la password è nuova e la
        // sessione di chi era entrato con quella vecchia rinnova ancora.
        uow.sessions()
            .revoke_all_for_user(user_id, RevocationReason::PasswordChanged)
            .await?;

        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::UserPasswordChanged(UserPasswordChanged {
                    user_id,
                    changed_at: now,
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(())
    }
}
