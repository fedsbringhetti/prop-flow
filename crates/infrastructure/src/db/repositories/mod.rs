//! Implementazioni dei repository su PostgreSQL.
//!
//! Tutte sono `impl … for PgUnitOfWork`: condividono la stessa transazione, che è
//! l'unico modo di garantire che comando ed evento di dominio siano atomici.
//!
//! Le query leggono i tipi dominio di PostgreSQL (`app.money_amount`,
//! `app.email_address`, gli enum) con un cast esplicito a `text`/`numeric` e li
//! scrivono con un cast esplicito al tipo di destinazione. Il cast rende la
//! decodifica indipendente dalla risoluzione degli OID dei tipi definiti
//! dall'utente, che varia con `search_path` e cache di connessione.

mod billing;
mod document;
mod expense;
mod expense_group;
mod invitation;
mod lease;
mod lease_party;
mod member;
mod organization;
mod payment;
mod property;
mod room;
mod session;
mod settlement;
mod ticket;
mod user;
