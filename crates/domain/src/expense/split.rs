use rust_decimal::Decimal;

use super::value_objects::Weight;
use crate::error::{DomainError, DomainResult};
use crate::shared::{Money, UserId};

/// Come una spesa si divide fra i membri.
///
/// **Due valori su cinque**, e i tre che restano fuori hanno ragioni diverse fra
/// loro, non una sola:
///
/// - `Percentage` ed `Exact` chiedono **un input per membro**, cioè una forma di
///   richiesta diversa dalle altre due, con la propria validazione («le
///   percentuali sommano a cento», «gli importi sommano al totale») e i propri
///   test, per un caso che nessuno ha chiesto;
/// - `ByUsage` chiede letture di **contatori**, che in questo schema non esistono
///   da nessuna parte.
///
/// Restano fra le varianti perché una riga scritta a mano potrebbe contenerle, e
/// un valore sconosciuto in lettura deve dare errore invece di ricadere su un
/// default plausibile — è la stessa scelta di
/// [`ChargeStatus`](crate::billing::ChargeStatus) e di
/// [`Payable`](crate::payment::Payable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitMethod {
    /// In parti uguali fra i membri attivi.
    Equal,
    /// In proporzione al [`Weight`] di ciascun membro. È il caso co-living vero:
    /// chi occupa la stanza doppia paga di più.
    Weighted,
    /// Mai scritto: vedi la nota sul tipo.
    Percentage,
    /// Mai scritto: vedi la nota sul tipo.
    Exact,
    /// Mai scritto: vedi la nota sul tipo.
    ByUsage,
}

impl SplitMethod {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Equal => "equal",
            Self::Weighted => "weighted",
            Self::Percentage => "percentage",
            Self::Exact => "exact",
            Self::ByUsage => "by_usage",
        }
    }

    /// I due metodi che questa fetta sa calcolare.
    pub const SUPPORTED: [Self; 2] = [Self::Equal, Self::Weighted];

    #[must_use]
    pub const fn is_supported(self) -> bool {
        matches!(self, Self::Equal | Self::Weighted)
    }
}

/// Un partecipante alla ripartizione: chi è, e quanto pesa.
///
/// Il peso c'è anche per [`SplitMethod::Equal`], dove viene ignorato: il
/// chiamante passa i membri del gruppo come sono, e a decidere se il peso conti è
/// il metodo. L'alternativa — due tipi di partecipante — costringerebbe chi chiama
/// a convertire una lista in un'altra prima di sapere quale metodo userà.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Participant {
    pub user_id: UserId,
    pub weight: Weight,
}

/// La quota di un partecipante.
///
/// `weight` è `Some` solo per le ripartizioni pesate, e finisce in
/// `app.expense_shares.share_weight`, che è annullabile: su una divisione in parti
/// uguali il peso non ha partecipato al calcolo, e scriverne uno direbbe il
/// contrario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Share {
    pub user_id: UserId,
    pub amount: Money,
    pub weight: Option<Weight>,
}

/// La ripartizione di un importo fra i partecipanti.
///
/// # L'aritmetica, che è il cuore di questo modulo
///
/// **La somma delle quote deve essere esattamente l'importo.** Non è
/// un'aspirazione: il trigger `expense_shares_balanced` rifiuta la transazione se
/// non lo è, e lo fa al `COMMIT` perché è `DEFERRABLE INITIALLY DEFERRED`. Cento
/// euro fra tre fanno 33,33 tre volte e avanza un centesimo, e quel centesimo non
/// si può buttare: va assegnato a qualcuno.
///
/// La regola è quella di Splitwise: **i centesimi di resto si distribuiscono uno
/// per uno ai primi partecipanti**, in ordine di `user_id` crescente.
///
/// L'ordine è per `user_id` e non per data di iscrizione né per nome perché deve
/// essere **riproducibile**: la stessa spesa ricalcolata deve dare le stesse
/// quote, e `joined_at` cambia se qualcuno esce e rientra. Vale per **entrambi** i
/// metodi, `weighted` compreso, dove la regola concorrente sarebbe «il resto a chi
/// ha la parte frazionaria più grande»: si sceglie la riproducibilità, così c'è
/// un'unica regola, un unico test e nessuna sorpresa nel passare da un metodo
/// all'altro.
///
/// Scartate, e vale la pena sapere perché:
///
/// - **il resto a chi ha anticipato**: più semplice da spiegare, e concentra su
///   una persona sola tutti i centesimi di tutte le spese che ha pagato lei — che
///   su un anno di spesa quotidiana non è più un arrotondamento;
/// - **a rotazione fra le spese**: la più equa e la peggiore da implementare.
///   Richiede uno stato che ricordi a chi è toccato l'ultima volta — una colonna e
///   una migration — e rende il calcolo di una quota non riproducibile guardando
///   la sola spesa.
///
/// # Perché in interi e non in `Decimal`
///
/// **`rust_decimal` non arrotonda da sé, e la sua divisione non è esatta.** Una
/// divisione produce molte cifre e viene troncata alla precisione massima del
/// tipo, quindi un valore matematicamente intero può arrivare come `4.999…9`; una
/// `trunc` a due decimali su quel valore perderebbe un centesimo in più, e la
/// quota risultante sarebbe diversa a seconda di come il chiamante ha costruito i
/// numeri. In contabilità «quasi sempre uguale» è il difetto peggiore, perché non
/// si manifesta sui casi che si provano.
///
/// Quindi non si divide mai: importi in **centesimi** e pesi in **millesimi**,
/// entrambi interi esatti perché le scale di `NUMERIC(14,2)` e `NUMERIC(6,3)` lo
/// garantiscono, e tutta l'aritmetica in `i128` — che regge il prodotto fra il
/// massimo importo (10^14 centesimi) e il massimo peso (10^6 millesimi) senza
/// avvicinarsi al limite.
pub struct Split;

impl Split {
    /// In parti uguali fra i partecipanti.
    pub fn equal(amount: Money, participants: &[Participant]) -> DomainResult<Vec<Share>> {
        // Peso uguale per tutti: la divisione in parti uguali *è* la pesata con
        // pesi identici, e scrivere due volte la distribuzione del resto
        // significherebbe avere due regole da tenere allineate.
        Self::allocate(amount, participants, |_| 1, None)
    }

    /// In proporzione al peso di ciascun partecipante.
    pub fn weighted(amount: Money, participants: &[Participant]) -> DomainResult<Vec<Share>> {
        Self::allocate(
            amount,
            participants,
            |participant| participant.weight.milli(),
            Some(()),
        )
    }

    /// Riparte secondo il metodo richiesto.
    ///
    /// Rifiuta i tre metodi che questa fetta non calcola invece di ricadere su
    /// `equal`: una spesa ripartita con un metodo diverso da quello che dichiara è
    /// una riga che mente, e il rifiuto è l'unica risposta onesta.
    pub fn by_method(
        method: SplitMethod,
        amount: Money,
        participants: &[Participant],
    ) -> DomainResult<Vec<Share>> {
        match method {
            SplitMethod::Equal => Self::equal(amount, participants),
            SplitMethod::Weighted => Self::weighted(amount, participants),
            SplitMethod::Percentage | SplitMethod::Exact | SplitMethod::ByUsage => {
                Err(DomainError::validation(
                    "split_method",
                    "only `equal` and `weighted` are supported",
                ))
            }
        }
    }

    /// La distribuzione, unica per entrambi i metodi.
    ///
    /// `keep_weight` distingue soltanto che cosa finisce in
    /// `app.expense_shares.share_weight`: il calcolo è lo stesso, e su `equal` i
    /// pesi valgono tutti uno.
    fn allocate(
        amount: Money,
        participants: &[Participant],
        weight_of: impl Fn(&Participant) -> i128,
        keep_weight: Option<()>,
    ) -> DomainResult<Vec<Share>> {
        if participants.is_empty() {
            return Err(DomainError::validation(
                "participants",
                "an expense needs at least one participant",
            ));
        }

        let currency = amount.currency();
        let total_cents = to_cents(amount.amount());
        if total_cents <= 0 {
            return Err(DomainError::validation(
                "amount",
                "must be greater than zero",
            ));
        }

        // L'ordine di uscita è quello di `user_id` crescente, e non quello in cui
        // il chiamante ha passato i membri: è ciò che rende la ripartizione
        // riproducibile. Ordinare qui invece di pretenderlo dal chiamante evita
        // che una `list_members` con un `ORDER BY` diverso cambi le quote.
        let mut ordered: Vec<&Participant> = participants.iter().collect();
        ordered.sort_by_key(|participant| participant.user_id.as_uuid());

        let weights: Vec<i128> = ordered.iter().map(|p| weight_of(p)).collect();
        let total_weight: i128 = weights.iter().sum();
        if total_weight <= 0 {
            // Irraggiungibile finché `Weight` resta strettamente positivo, ma la
            // divisione per zero che seguirebbe è cara abbastanza da meritare un
            // ramo esplicito invece di una fiducia.
            return Err(DomainError::validation(
                "weight",
                "the weights must add up to more than zero",
            ));
        }

        // Divisione intera: ciascuno prende la propria parte troncata, e il resto
        // resta da distribuire. `total_cents * weight` sta comodamente in `i128`.
        let mut cents: Vec<i128> = weights
            .iter()
            .map(|weight| total_cents * weight / total_weight)
            .collect();

        // Il resto è per costruzione minore del numero di partecipanti: ogni
        // troncamento perde meno di un centesimo. Distribuirne uno per testa in
        // ordine di `user_id` lo esaurisce sempre.
        let assigned: i128 = cents.iter().sum();
        let mut remainder = total_cents - assigned;
        for slot in &mut cents {
            if remainder == 0 {
                break;
            }
            *slot += 1;
            remainder -= 1;
        }

        ordered
            .iter()
            .zip(cents)
            .map(|(participant, cents)| {
                Ok(Share {
                    user_id: participant.user_id,
                    amount: Money::non_negative(from_cents(cents), currency)
                        .map_err(|error| DomainError::validation("share_amount", error))?,
                    weight: keep_weight.map(|()| participant.weight),
                })
            })
            .collect()
    }
}

/// Un importo in centesimi, esatto.
///
/// [`Money`] ammette al massimo due decimali, quindi la moltiplicazione per cento
/// non può perdere cifre e la `trunc` non tronca niente: serve solo a togliere la
/// scala prima della conversione.
fn to_cents(amount: Decimal) -> i128 {
    (amount * Decimal::ONE_HUNDRED)
        .trunc()
        .try_into()
        .unwrap_or(0)
}

fn from_cents(cents: i128) -> Decimal {
    Decimal::from_i128_with_scale(cents, 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::Currency;
    use std::str::FromStr;
    use uuid::Uuid;

    fn money(raw: &str) -> Money {
        Money::new(
            Decimal::from_str(raw).expect("decimale di test valido"),
            Currency::EUR,
        )
        .expect("importo di test valido")
    }

    /// Identificatori in ordine noto: l'ordine del resto è per `user_id`, quindi
    /// un test che non lo controlla non sta verificando la regola.
    fn user(nth: u8) -> UserId {
        let mut bytes = [0_u8; 16];
        bytes[15] = nth;
        UserId::from_uuid(Uuid::from_bytes(bytes))
    }

    fn participants(weights: &[&str]) -> Vec<Participant> {
        weights
            .iter()
            .enumerate()
            .map(|(index, weight)| Participant {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "nessun test ha più di 255 partecipanti"
                )]
                user_id: user(index as u8 + 1),
                weight: Weight::parse(Decimal::from_str(weight).expect("decimale"))
                    .expect("peso valido"),
            })
            .collect()
    }

    fn total(shares: &[Share]) -> Decimal {
        shares.iter().map(|share| share.amount.amount()).sum()
    }

    /// Il criterio unico, e vale per ogni caso: **la somma delle quote è
    /// l'importo**. Se non lo è, il trigger rifiuta al commit.
    fn assert_balanced(amount: Money, shares: &[Share]) {
        assert_eq!(
            total(shares),
            amount.amount(),
            "le quote non sommano all'importo: il trigger rifiuterebbe al commit"
        );
    }

    /// Il caso della soglia: 100,00 fra tre. Il centesimo va al primo in ordine
    /// di `user_id`.
    #[test]
    fn a_hundred_between_three_gives_the_extra_cent_to_the_first() {
        let amount = money("100.00");
        let shares = Split::equal(amount, &participants(&["1", "1", "1"])).expect("ripartita");

        assert_eq!(shares.len(), 3);
        assert_eq!(
            shares[0].amount.amount(),
            Decimal::from_str("33.34").expect("d")
        );
        assert_eq!(
            shares[1].amount.amount(),
            Decimal::from_str("33.33").expect("d")
        );
        assert_eq!(
            shares[2].amount.amount(),
            Decimal::from_str("33.33").expect("d")
        );
        assert_balanced(amount, &shares);
    }

    /// Un centesimo fra tre: due quote sono **zero**, e devono esserlo.
    /// `app.expense_shares.share_amount` è `CHECK (share_amount >= 0)`, non `> 0`:
    /// lo schema questo caso lo prevede.
    #[test]
    fn one_cent_between_three_leaves_two_at_zero() {
        let amount = money("0.01");
        let shares = Split::equal(amount, &participants(&["1", "1", "1"])).expect("ripartita");

        assert_eq!(
            shares[0].amount.amount(),
            Decimal::from_str("0.01").expect("d")
        );
        assert_eq!(shares[1].amount.amount(), Decimal::ZERO);
        assert_eq!(shares[2].amount.amount(), Decimal::ZERO);
        assert_balanced(amount, &shares);
    }

    /// Pesi 1/1/2 su 10,00: la stanza doppia paga il doppio, e i conti tornano.
    #[test]
    fn weights_of_one_one_and_two_split_ten_euros() {
        let amount = money("10.00");
        let shares = Split::weighted(amount, &participants(&["1", "1", "2"])).expect("ripartita");

        assert_eq!(
            shares[0].amount.amount(),
            Decimal::from_str("2.50").expect("d")
        );
        assert_eq!(
            shares[1].amount.amount(),
            Decimal::from_str("2.50").expect("d")
        );
        assert_eq!(
            shares[2].amount.amount(),
            Decimal::from_str("5.00").expect("d")
        );
        assert_balanced(amount, &shares);
    }

    /// Pesi che non dividono l'importo: il resto si distribuisce comunque, e la
    /// regola resta quella dell'ordine di `user_id`, non quella della parte
    /// frazionaria più grande.
    #[test]
    fn a_weighted_split_that_does_not_divide_evenly_still_balances() {
        let amount = money("10.00");
        let shares = Split::weighted(amount, &participants(&["1", "1", "1"])).expect("ripartita");

        assert_eq!(
            shares[0].amount.amount(),
            Decimal::from_str("3.34").expect("d")
        );
        assert_eq!(
            shares[1].amount.amount(),
            Decimal::from_str("3.33").expect("d")
        );
        assert_eq!(
            shares[2].amount.amount(),
            Decimal::from_str("3.33").expect("d")
        );
        assert_balanced(amount, &shares);
    }

    /// Un peso abbastanza piccolo porta una quota a zero, e la spesa resta
    /// bilanciata: è il caso limite che il `CHECK (share_amount >= 0)` ammette.
    #[test]
    fn a_tiny_weight_can_earn_a_zero_share() {
        let amount = money("1.00");
        let shares = Split::weighted(amount, &participants(&["999", "0.001"])).expect("ripartita");

        assert_eq!(shares[1].amount.amount(), Decimal::ZERO);
        assert_balanced(amount, &shares);
    }

    /// Il peso finisce nella quota **solo** se ha partecipato al calcolo. Su una
    /// divisione in parti uguali `share_weight` resta nullo, perché scriverne uno
    /// direbbe che il calcolo lo ha usato.
    #[test]
    fn the_weight_travels_only_with_a_weighted_split() {
        let amount = money("9.00");
        let equal = Split::equal(amount, &participants(&["1", "2", "3"])).expect("ripartita");
        assert!(equal.iter().all(|share| share.weight.is_none()));

        let weighted = Split::weighted(amount, &participants(&["1", "2", "3"])).expect("ripartita");
        assert_eq!(
            weighted
                .iter()
                .map(|share| share.weight.map(Weight::value))
                .collect::<Vec<_>>(),
            vec![
                Some(Decimal::ONE),
                Some(Decimal::TWO),
                Some(Decimal::from(3))
            ]
        );
    }

    /// **L'ordine di uscita non dipende da quello di ingresso.** È la proprietà
    /// per cui la regola del resto è riproducibile: una `list_members` con un
    /// `ORDER BY` diverso non deve poter cambiare chi paga il centesimo.
    #[test]
    fn the_result_does_not_depend_on_the_order_of_the_input() {
        let amount = money("100.00");
        let mut forwards = participants(&["1", "1", "1"]);
        let straight = Split::equal(amount, &forwards).expect("ripartita");

        forwards.reverse();
        let reversed = Split::equal(amount, &forwards).expect("ripartita");

        assert_eq!(straight, reversed);
    }

    /// Un solo partecipante prende tutto: il caso banale, che è anche quello del
    /// gruppo appena creato in cui c'è solo chi lo ha creato.
    #[test]
    fn a_single_participant_takes_the_whole_amount() {
        let amount = money("77.77");
        let shares = Split::equal(amount, &participants(&["1"])).expect("ripartita");
        assert_eq!(shares.len(), 1);
        assert_balanced(amount, &shares);
    }

    /// Nessun partecipante non è una ripartizione a zero: è una richiesta senza
    /// senso, e il trigger la rifiuterebbe comunque al commit. Meglio dirlo prima.
    #[test]
    fn no_participants_is_refused() {
        assert!(Split::equal(money("10.00"), &[]).is_err());
        assert!(Split::weighted(money("10.00"), &[]).is_err());
    }

    /// I tre metodi fuori perimetro **danno errore** invece di ricadere su
    /// `equal`: una spesa ripartita con un metodo diverso da quello che dichiara è
    /// una riga che mente.
    #[test]
    fn the_three_unsupported_methods_are_refused_rather_than_approximated() {
        let people = participants(&["1", "1"]);
        for method in [
            SplitMethod::Percentage,
            SplitMethod::Exact,
            SplitMethod::ByUsage,
        ] {
            assert!(
                Split::by_method(method, money("10.00"), &people).is_err(),
                "`{}` non è calcolabile in questa fetta",
                method.as_str()
            );
            assert!(!method.is_supported());
        }
        for method in SplitMethod::SUPPORTED {
            assert!(Split::by_method(method, money("10.00"), &people).is_ok());
            assert!(method.is_supported());
        }
    }

    /// Il caso che l'aritmetica intera esiste per difendere: importi e numeri di
    /// partecipanti scelti per far avanzare ogni resto possibile. La somma torna
    /// **sempre**, che è l'unico criterio.
    #[test]
    fn every_remainder_lands_somewhere() {
        for cents in 1..=250_u32 {
            for people in 1..=7_usize {
                let amount = money(&format!("{}.{:02}", cents / 100, cents % 100));
                let weights = vec!["1"; people];
                let equal = Split::equal(amount, &participants(&weights)).expect("ripartita");
                assert_balanced(amount, &equal);

                let mixed: Vec<&str> = (0..people)
                    .map(|index| ["1", "2", "3.5", "0.25"][index % 4])
                    .collect();
                let weighted = Split::weighted(amount, &participants(&mixed)).expect("ripartita");
                assert_balanced(amount, &weighted);
            }
        }
    }
}
