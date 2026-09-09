use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::identity::{PasswordHash, User};
use propflow_domain::ports::{PersonName, RepositoryError, RepositoryResult, UserRepository};
use propflow_domain::shared::{Email, UserId};

use crate::db::db_enums;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

/// Colonne dell'utente. Macro e non costante: `concat!` compone solo letterali,
/// e comporre a compile time evita le stringhe SQL dinamiche che sqlx 0.9
/// richiede di dichiarare esplicitamente come sicure.
macro_rules! select_user {
    () => {
        "
    SELECT id,
           email::text            AS email,
           email_verified_at,
           password_hash,
           full_name,
           phone,
           locale,
           platform_role::text    AS platform_role,
           status::text           AS status,
           last_login_at,
           failed_login_count,
           locked_until,
           created_at
      FROM app.users
"
    };
}

#[async_trait]
impl UserRepository for PgUnitOfWork {
    async fn find_by_id(&mut self, id: UserId) -> RepositoryResult<Option<User>> {
        let row = sqlx::query(concat!(
            select_user!(),
            " WHERE id = $1 AND deleted_at IS NULL"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(map_user).transpose()
    }

    async fn find_by_email(&mut self, email: &Email) -> RepositoryResult<Option<User>> {
        // `citext` rende il confronto già insensibile alla capitalizzazione, ma
        // l'email arriva qui normalizzata dal value object: due difese, un
        // risultato prevedibile.
        let row = sqlx::query(concat!(
            select_user!(),
            " WHERE email = $1::app.email_address AND deleted_at IS NULL"
        ))
        .bind(email.as_str())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(map_user).transpose()
    }

    async fn insert(&mut self, user: &User) -> RepositoryResult<()> {
        sqlx::query(
            "INSERT INTO app.users (
                 id, email, email_verified_at, password_hash, full_name, phone, locale,
                 platform_role, status, last_login_at, failed_login_count, locked_until, created_at
             ) VALUES (
                 $1, $2::app.email_address, $3, $4, $5, $6, $7,
                 $8::app.platform_role, $9::app.user_status, $10, $11, $12, $13
             )",
        )
        .bind(user.id().as_uuid())
        .bind(user.email().as_str())
        .bind(user.email_verified_at())
        .bind(user.password_hash().map(PasswordHash::as_str))
        .bind(user.full_name())
        .bind(user.phone())
        .bind(user.locale())
        .bind(db_enums::platform_role_to_db(user.platform_role()))
        .bind(db_enums::user_status_to_db(user.status()))
        .bind(user.last_login_at())
        .bind(i16::from(user.failed_login_count()))
        .bind(user.locked_until())
        .bind(user.created_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn update(&mut self, user: &User) -> RepositoryResult<()> {
        // `updated_at` non è nella SET list: lo scrive il trigger
        // `set_updated_at`, così nessuna scrittura può dimenticarselo.
        let result = sqlx::query(
            "UPDATE app.users
                SET email_verified_at  = $2,
                    password_hash      = $3,
                    full_name          = $4,
                    phone              = $5,
                    locale             = $6,
                    status             = $7::app.user_status,
                    last_login_at      = $8,
                    failed_login_count = $9,
                    locked_until       = $10
              WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(user.id().as_uuid())
        .bind(user.email_verified_at())
        .bind(user.password_hash().map(PasswordHash::as_str))
        .bind(user.full_name())
        .bind(user.phone())
        .bind(user.locale())
        .bind(db_enums::user_status_to_db(user.status()))
        .bind(user.last_login_at())
        .bind(i16::from(user.failed_login_count()))
        .bind(user.locked_until())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::Backend {
                details: "aggiornamento utente senza effetto: riga assente o non visibile"
                    .to_owned(),
            });
        }
        Ok(())
    }

    async fn names(&mut self, ids: &[UserId]) -> RepositoryResult<Vec<PersonName>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        // **Non passa da `select_user!`, ed è il punto.** Quella macro legge
        // l'aggregato — email, telefono, hash della password, stato dell'account —
        // e qui serve una colonna sola. Proiettare invece di selezionare è la
        // forma di `app.property_residents` (ADR-036): ciò che non si nomina non
        // può uscire per sbaglio.
        //
        // **Il confine è `users_select` e non questo `WHERE`.** La policy ammette
        // sé stessi, chi abita la stessa casa (`shares_home_with`), le persone
        // delle proprie organizzazioni (`is_staff_of_user`) e lo staff della
        // propria (`is_my_staff`): un identificatore che nessuno di quei rami
        // copre semplicemente non torna, e chi chiama mostra un'etichetta neutra.
        // Questa query è il primo lettore di quella policy — la `0030` aveva
        // scritto che la finestra per restringerla si chiudeva col primo client
        // che leggesse un nome da lì, ed è questo (ADR-049).
        let uuids: Vec<Uuid> = ids.iter().map(|id| id.as_uuid()).collect();
        let rows = sqlx::query(
            "SELECT id, full_name
               FROM app.users
              WHERE id = ANY($1) AND deleted_at IS NULL",
        )
        .bind(&uuids)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        rows.into_iter()
            .map(|row| {
                let id: Uuid = try_get(&row, "id")?;
                Ok(PersonName {
                    user_id: UserId::from_uuid(id),
                    full_name: try_get(&row, "full_name")?,
                })
            })
            .collect()
    }
}

fn map_user(row: PgRow) -> RepositoryResult<User> {
    let email: String = try_get(&row, "email")?;
    let email = Email::parse(email).map_err(|error| RepositoryError::Backend {
        details: format!("email non valida in database: {error}"),
    })?;

    let password_hash: Option<String> = try_get(&row, "password_hash")?;
    let password_hash = password_hash
        .map(PasswordHash::new)
        .transpose()
        .map_err(|error| RepositoryError::Backend {
            details: format!("hash password non valido in database: {error}"),
        })?;

    let failed_login_count: i16 = try_get(&row, "failed_login_count")?;
    let status: String = try_get(&row, "status")?;
    let platform_role: String = try_get(&row, "platform_role")?;
    let id: Uuid = try_get(&row, "id")?;
    let email_verified_at: Option<DateTime<Utc>> = try_get(&row, "email_verified_at")?;
    let last_login_at: Option<DateTime<Utc>> = try_get(&row, "last_login_at")?;
    let locked_until: Option<DateTime<Utc>> = try_get(&row, "locked_until")?;
    let created_at: DateTime<Utc> = try_get(&row, "created_at")?;

    Ok(User::from_persistence(
        UserId::from_uuid(id),
        email,
        email_verified_at,
        password_hash,
        try_get(&row, "full_name")?,
        try_get(&row, "phone")?,
        try_get(&row, "locale")?,
        db_enums::platform_role_from_db(&platform_role)?,
        db_enums::user_status_from_db(&status)?,
        last_login_at,
        // Il contatore è SMALLINT in database: valori negativi non sono
        // rappresentabili nel dominio e vengono riportati a zero.
        u8::try_from(failed_login_count.max(0)).unwrap_or(u8::MAX),
        locked_until,
        created_at,
    ))
}

/// Estrae una colonna riportando il nome nel messaggio: senza il nome, un errore
/// di decodifica su venti colonne non dice quale.
pub(crate) fn try_get<'r, T>(row: &'r PgRow, column: &'static str) -> RepositoryResult<T>
where
    T: sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>,
{
    row.try_get(column)
        .map_err(|error| RepositoryError::Backend {
            details: format!("colonna `{column}` non decodificabile: {error}"),
        })
}
