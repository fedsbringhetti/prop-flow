//! Il subentro: una parte esce oggi e un'altra prende il suo posto (ADR-056).
//!
//! Vive nel dominio, e non nel caso d'uso, perché è una regola su più righe che si
//! verifica **tutta prima di scrivere**: chi esce, chi entra, le quote di chi resta
//! e la somma finale. Il caso d'uso riceve le righe già calcolate e le scrive
//! nell'ordine di [`HandoverOutcome`].

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;

use crate::error::{DomainError, DomainResult, Entity, Invariant};
use crate::shared::{Email, LeasePartyId};

use super::{Lease, LeaseParty, LeasePartyChanges, NewLeaseParty};

/// Chi entra. **Ruolo e intestazione non ci sono**: si ereditano da chi esce, che
/// è ciò che «prendere il suo posto» significa.
#[derive(Debug, Clone)]
pub struct JoiningParty {
    pub id: LeasePartyId,
    pub full_name: String,
    pub email: Email,
    pub share_percent: Decimal,
}

/// La transizione richiesta. La data non c'è: è oggi, per la ragione scritta in
/// [`LeaseParty::leave`].
#[derive(Debug, Clone)]
pub struct Handover {
    pub leaving_party_id: LeasePartyId,
    pub joining: JoiningParty,
    /// Le quote di chi resta. Una parte che resta e non compare qui **non cambia**.
    pub shares: Vec<(LeasePartyId, Decimal)>,
}

/// Le righe che il subentro cambia, **nell'ordine in cui vanno scritte**: chi esce
/// per primo, perché ha appena perso l'intestazione che chi entra riceve, e
/// `lease_parties_primary_uk` non ammette due intestatari nemmeno per un istante.
#[derive(Debug, Clone)]
pub struct HandoverOutcome {
    pub left: LeaseParty,
    pub reshared: Vec<LeaseParty>,
    pub joined: LeaseParty,
}

impl Lease {
    /// Calcola il subentro su `parties`, le parti del contratto come sono ora.
    ///
    /// Non scrive niente e non tocca `parties`: se una verifica fallisce non c'è
    /// nessuno stato a metà da disfare. **La somma si riverifica sullo stato
    /// finale**, ed è la differenza con la correzione di una parte, dove lo stato
    /// intermedio è legittimo e nessuno controlla: qui lo stato intermedio non
    /// esiste.
    pub fn hand_over(
        &self,
        parties: &[LeaseParty],
        handover: Handover,
        today: NaiveDate,
        now: DateTime<Utc>,
    ) -> DomainResult<HandoverOutcome> {
        self.ensure_open_to_handover(today)?;
        let Handover {
            leaving_party_id,
            joining,
            shares,
        } = handover;

        // **Tutte** le righe, uscite comprese: è la ragione dell'invariante.
        if parties
            .iter()
            .any(|party| party.email() == Some(&joining.email))
        {
            return Err(DomainError::Invariant(Invariant::LeasePartyEmailTaken));
        }

        let mut left = parties
            .iter()
            .find(|party| party.id() == leaving_party_id)
            .cloned()
            .ok_or(DomainError::NotFound(Entity::LeaseParty))?;
        let was_primary = left.leave(today, now)?;

        let mut reshared: Vec<LeaseParty> = Vec::with_capacity(shares.len());
        for (party_id, share) in shares {
            if reshared.iter().any(|party| party.id() == party_id) {
                return Err(DomainError::validation(
                    "shares",
                    "each party can appear at most once",
                ));
            }
            let mut party = parties
                .iter()
                .find(|party| {
                    party.id() == party_id
                        && party.id() != leaving_party_id
                        && party.left_at().is_none()
                })
                .cloned()
                .ok_or_else(|| {
                    DomainError::validation("shares", "must name a party that stays on the lease")
                })?;
            party.correct(
                LeasePartyChanges {
                    share_percent: Some(share),
                    ..LeasePartyChanges::default()
                },
                now,
            )?;
            reshared.push(party);
        }

        let joined = LeaseParty::new(
            NewLeaseParty {
                id: joining.id,
                org_id: self.org_id(),
                lease_id: self.id(),
                role: left.role(),
                full_name: joining.full_name,
                email: Some(joining.email),
                share_percent: joining.share_percent,
                is_primary: was_primary,
            },
            now,
        )?;

        let after: Vec<LeaseParty> = parties
            .iter()
            .map(|party| {
                if party.id() == left.id() {
                    return left.clone();
                }
                reshared
                    .iter()
                    .find(|changed| changed.id() == party.id())
                    .unwrap_or(party)
                    .clone()
            })
            .chain(std::iter::once(joined.clone()))
            .collect();
        LeaseParty::ensure_tenancy_complete(&after)?;

        Ok(HandoverOutcome {
            left,
            reshared,
            joined,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lease::{LeaseDraft, LeasePartyRole, RentFrequency};
    use crate::shared::{Currency, LeaseId, Money, OrgId, PropertyId, UserId};
    use std::str::FromStr;

    fn date(raw: &str) -> NaiveDate {
        NaiveDate::from_str(raw).expect("data di test valida")
    }

    fn dec(raw: &str) -> Decimal {
        Decimal::from_str(raw).expect("decimale di test valido")
    }

    fn email(raw: &str) -> Email {
        Email::parse(raw).expect("email di test valida")
    }

    /// Un contratto in bozza dal 1° gennaio 2026 al 1° gennaio 2027.
    fn draft() -> Lease {
        Lease::draft(
            LeaseDraft {
                id: LeaseId::generate(),
                org_id: OrgId::generate(),
                property_id: PropertyId::generate(),
                room_id: None,
                start_date: date("2026-01-01"),
                end_date: Some(date("2027-01-01")),
                rent: Money::new(dec("800.00"), Currency::EUR).expect("importo valido"),
                deposit: None,
                rent_frequency: RentFrequency::Monthly,
                rent_due_day: 5,
                notice_months: None,
                auto_renew: false,
                renewal_months: None,
                utilities_included: false,
                istat_indexation: false,
                notes: None,
            },
            Utc::now(),
        )
        .expect("contratto valido")
    }

    fn party(lease: &Lease, name: &str, address: &str, share: &str, primary: bool) -> LeaseParty {
        LeaseParty::new(
            NewLeaseParty {
                id: LeasePartyId::generate(),
                org_id: lease.org_id(),
                lease_id: lease.id(),
                role: LeasePartyRole::Cotenant,
                full_name: name.to_owned(),
                email: Some(email(address)),
                share_percent: dec(share),
                is_primary: primary,
            },
            Utc::now(),
        )
        .expect("parte valida")
    }

    /// Anna (intestataria) 50, Marco 50, su un contratto attivo.
    fn active_with_two() -> (Lease, Vec<LeaseParty>) {
        let mut lease = draft();
        let parties = vec![
            party(&lease, "Anna", "anna@example.com", "50", true),
            party(&lease, "Marco", "marco@example.com", "50", false),
        ];
        lease.activate(&parties, Utc::now()).expect("attivato");
        (lease, parties)
    }

    fn giulia(share: &str) -> JoiningParty {
        JoiningParty {
            id: LeasePartyId::generate(),
            full_name: "Giulia".to_owned(),
            email: email("giulia@example.com"),
            share_percent: dec(share),
        }
    }

    const IN_FORCE: &str = "2026-06-01";

    fn invariant(result: DomainResult<HandoverOutcome>) -> Invariant {
        match result {
            Err(DomainError::Invariant(invariant)) => invariant,
            other => panic!("attesa un'invariante, ottenuto {other:?}"),
        }
    }

    #[test]
    fn only_a_lease_in_force_today_accepts_a_handover() {
        let mut lease = draft();
        let parties = vec![party(&lease, "Anna", "anna@example.com", "100", true)];
        let handover = || Handover {
            leaving_party_id: parties[0].id(),
            joining: giulia("100"),
            shares: vec![],
        };

        // In bozza le parti si aggiungono: il subentro non serve.
        assert!(matches!(
            invariant(lease.hand_over(&parties, handover(), date(IN_FORCE), Utc::now())),
            Invariant::LeaseNotHandoverable
        ));

        lease.activate(&parties, Utc::now()).expect("attivato");
        // Attivo ma non ancora cominciato, e attivo ma già finito: `active` in
        // memoria in entrambi i casi, e nessuno dei due è in essere.
        for day in ["2025-12-31", "2027-01-01"] {
            assert!(
                matches!(
                    invariant(lease.hand_over(&parties, handover(), date(day), Utc::now())),
                    Invariant::LeaseNotHandoverable
                ),
                "il {day} il contratto non è in essere"
            );
        }
        assert!(
            lease
                .hand_over(&parties, handover(), date(IN_FORCE), Utc::now())
                .is_ok()
        );

        lease.terminate(None, Utc::now()).expect("disdetto");
        assert!(matches!(
            invariant(lease.hand_over(&parties, handover(), date(IN_FORCE), Utc::now())),
            Invariant::LeaseNotHandoverable
        ));
    }

    #[test]
    fn the_joining_party_takes_the_place_the_share_and_the_title_of_the_leaving_one() {
        let (lease, parties) = active_with_two();
        let anna = parties[0].id();

        let outcome = lease
            .hand_over(
                &parties,
                Handover {
                    leaving_party_id: anna,
                    joining: giulia("50"),
                    shares: vec![],
                },
                date(IN_FORCE),
                Utc::now(),
            )
            .expect("subentro");

        assert_eq!(outcome.left.id(), anna);
        assert_eq!(outcome.left.left_at(), Some(date(IN_FORCE)));
        assert!(
            !outcome.left.is_primary(),
            "chi esce non intesta più niente"
        );
        assert!(
            outcome.joined.is_primary(),
            "l'intestazione passa a chi entra"
        );
        assert_eq!(outcome.joined.role(), LeasePartyRole::Cotenant);
        assert_eq!(outcome.joined.share_percent(), dec("50"));
        assert_eq!(outcome.joined.user_id(), None, "un segnaposto in attesa");
        assert!(
            outcome.reshared.is_empty(),
            "Marco non cambia se non lo si chiede"
        );
    }

    #[test]
    fn the_shares_of_who_stays_can_change_and_the_final_sum_is_checked() {
        let (lease, parties) = active_with_two();
        let (anna, marco) = (parties[0].id(), parties[1].id());
        let handover = |joining: &str, marco_share: &str| Handover {
            leaving_party_id: anna,
            joining: giulia(joining),
            shares: vec![(marco, dec(marco_share))],
        };

        let outcome = lease
            .hand_over(&parties, handover("60", "40"), date(IN_FORCE), Utc::now())
            .expect("60 + 40");
        assert_eq!(outcome.reshared.len(), 1);
        assert_eq!(outcome.reshared[0].share_percent(), dec("40"));

        // Anna esce con il suo 50, e non conta più: 50 + 40 non fa 100.
        assert!(matches!(
            invariant(lease.hand_over(&parties, handover("50", "40"), date(IN_FORCE), Utc::now())),
            Invariant::LeaseSharesNotHundred { .. }
        ));
        assert!(matches!(
            invariant(lease.hand_over(&parties, handover("0", "100"), date(IN_FORCE), Utc::now())),
            Invariant::LeaseTenantWithoutShare
        ));
    }

    #[test]
    fn a_share_can_only_be_set_on_a_party_that_stays() {
        let (lease, parties) = active_with_two();
        let anna = parties[0].id();
        for party_id in [anna, LeasePartyId::generate()] {
            let result = lease.hand_over(
                &parties,
                Handover {
                    leaving_party_id: anna,
                    joining: giulia("50"),
                    shares: vec![(party_id, dec("50"))],
                },
                date(IN_FORCE),
                Utc::now(),
            );
            assert!(
                matches!(result, Err(DomainError::Validation { .. })),
                "{result:?}"
            );
        }
    }

    #[test]
    fn a_party_leaves_once_and_its_email_cannot_come_back() {
        let (lease, parties) = active_with_two();
        let (anna, marco) = (parties[0].id(), parties[1].id());
        let first = lease
            .hand_over(
                &parties,
                Handover {
                    leaving_party_id: anna,
                    joining: giulia("50"),
                    shares: vec![],
                },
                date(IN_FORCE),
                Utc::now(),
            )
            .expect("primo subentro");
        let after = vec![first.left, parties[1].clone(), first.joined];

        // Anna è già uscita.
        let again = Handover {
            leaving_party_id: anna,
            joining: JoiningParty {
                email: email("luca@example.com"),
                ..giulia("50")
            },
            shares: vec![],
        };
        assert!(matches!(
            invariant(lease.hand_over(&after, again, date(IN_FORCE), Utc::now())),
            Invariant::LeasePartyAlreadyLeft
        ));

        // Anna non rientra al posto di Marco: la sua riga, uscita, ha quell'email.
        let back = Handover {
            leaving_party_id: marco,
            joining: JoiningParty {
                email: email("anna@example.com"),
                ..giulia("50")
            },
            shares: vec![],
        };
        assert!(matches!(
            invariant(lease.hand_over(&after, back, date(IN_FORCE), Utc::now())),
            Invariant::LeasePartyEmailTaken
        ));
    }

    #[test]
    fn a_party_that_is_not_on_the_lease_cannot_leave_it() {
        let (lease, parties) = active_with_two();
        let result = lease.hand_over(
            &parties,
            Handover {
                leaving_party_id: LeasePartyId::generate(),
                joining: giulia("50"),
                shares: vec![],
            },
            date(IN_FORCE),
            Utc::now(),
        );
        assert!(matches!(
            result,
            Err(DomainError::NotFound(Entity::LeaseParty))
        ));
    }

    #[test]
    fn leaving_writes_the_date_drops_the_title_and_happens_once() {
        let (_, mut parties) = active_with_two();
        let anna = &mut parties[0];
        anna.link_user(UserId::generate(), Utc::now())
            .expect("collegata");

        assert!(
            anna.leave(date(IN_FORCE), Utc::now()).expect("esce"),
            "era intestataria"
        );
        assert_eq!(anna.left_at(), Some(date(IN_FORCE)));
        assert!(!anna.is_primary());
        assert!(matches!(
            anna.leave(date(IN_FORCE), Utc::now()),
            Err(DomainError::Invariant(Invariant::LeasePartyAlreadyLeft))
        ));
    }
}
