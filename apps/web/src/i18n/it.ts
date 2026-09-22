/**
 * Il catalogo delle stringhe italiane.
 *
 * **Nessuna stringa visibile va scritta dentro un componente**, ed è il vincolo
 * che vale dal primo giorno per una ragione che non è di stile: la traduzione in
 * inglese è già decisa e soltanto rimandata, e una fetta costruita con i testi nel
 * JSX la rende una riscrittura invece di un'aggiunta. Il livello di indirezione è
 * la parte impossibile da recuperare dopo; la seconda lingua è un secondo file.
 *
 * Non serve una libreria di internazionalizzazione per questo — servirà per
 * plurali e genere quando le lingue saranno due. Serve che i componenti non
 * conoscano il testo.
 */
/**
 * Le voci della navigazione, fuori dal catalogo perché la guida le **usa** come
 * titoli delle proprie sezioni: una sezione della guida si chiama come la voce
 * che spiega, e con una copia le due divergerebbero senza che nessuno se ne accorga.
 */
const nav = {
  dashboard: 'Riepilogo',
  properties: 'Immobili',
  leases: 'Contratti',
  /** Al singolare, e solo per l'inquilino: per chi abita il contratto è uno. */
  lease: 'Contratto',
  tickets: 'Segnalazioni',
  bills: 'Bollette',
  charges: 'Addebiti',
  payments: 'Pagamenti',
  /** Solo per chi abita: il denaro fra coinquilini non riguarda chi amministra. */
  expenses: 'Spese',
  main: 'Navigazione principale',
  account: 'Il mio account',
  /** Nel menù dell'account, con le cose che si fanno su di sé. */
  guide: 'Come si usa',
} as const

export const it = {
  app: {
    name: 'PropFlow',
    skipToContent: 'Vai al contenuto',
  },

  common: {
    save: 'Salva',
    cancel: 'Annulla',
    /**
     * Il bottone che **non fa niente** in una conferma. Non «Annulla»: tre
     * entità si *annullano*, e una modale con «Annulla» a sinistra e «Annulla» a
     * destra — una lascia stare, l'altra annulla l'addebito — è una domanda a cui
     * si risponde a caso. Trovata montando il verbo in blocco, esisteva da prima.
     */
    keep: 'Lascia com’è',
    close: 'Chiudi',
    create: 'Crea',
    edit: 'Modifica',
    back: 'Indietro',
    retry: 'Riprova',
    loadMore: 'Carica altri',
    copy: 'Copia',
    copied: 'Copiato',
    optional: 'facoltativo',
    yes: 'Sì',
    no: 'No',
    none: '—',
    of: 'di',
    actions: 'Azioni',
    confirm: 'Conferma',
    search: 'Cerca',
    all: 'Tutti',
    saving: 'Salvataggio…',
    /**
     * L'etichetta neutra di un nome che l'API non ha potuto risolvere.
     *
     * Sta in `common` e non sotto un modulo perché la stessa frase serve a
     * contratti, segnalazioni, addebiti e bollette: le query dei nomi filtrano
     * `deleted_at IS NULL` come impone la regola 10, quindi un immobile archiviato
     * lascia il campo assente ovunque. **Mai l'identificatore al suo posto**
     * (ADR-043): un UUID a schermo non è un nome, è un guasto che si vede.
     */
    archivedProperty: 'Immobile archiviato',
  },

  state: {
    loadingTitle: 'Caricamento',
    errorTitle: 'Qualcosa non ha funzionato',
    errorRequestId: 'Riferimento della richiesta',
    emptyTitle: 'Non c’è ancora niente',
    /** Il vuoto di un elenco **con un filtro attivo** non è il vuoto della
     *  sezione: dice che è del filtro, e non spiega la sezione a chi la conosce
     *  già. Vale per ogni elenco filtrabile, quindi sta qui e non per modulo. */
    filteredTitle: 'Niente che corrisponda ai filtri.',
    filteredHint: 'I filtri sono attivi: toglili per vedere tutto.',
  },

  /**
   * La selezione in tabella e l'azione in blocco, che è la sezione 7.
   *
   * Nessuna di queste frasi dice «Elimina»: il verbo è dell'entità e sta nel
   * suo modulo — *Archivia*, *Annulla* — perché il prodotto non cancella niente
   * e ogni entità esce di scena a modo suo. Qui c'è solo ciò che è uguale per
   * tutte: contare, scegliere, dire come è andata.
   */
  selection: {
    selectPage: 'Seleziona tutte le righe caricate',
    selectRow: 'Seleziona',
    region: 'Righe selezionate',
    /** «Caricate» e mai «tutte»: gli elenchi sono a cursore, e la pagina è ciò
     *  che si vede, non l'elenco intero. */
    count: '{count} selezionate su {total} caricate',
    progress: 'In corso: {done} di {total}',
    clear: 'Deseleziona',
    /**
     * L'esito di un giro, quando qualcosa non è passato. Le righe non fatte
     * restano selezionate con il motivo accanto, e questa frase dice solo che
     * cosa guardare. `{failed}` è il numero di righe rimaste.
     */
    partial: '{done} fatte. {failed} non fatte: restano selezionate, con il motivo accanto. Correggi e riprova.',
  },

  auth: {
    loginTitle: 'Entra in PropFlow',
    loginSubtitle: 'Amministra i tuoi immobili, i contratti e la contabilità.',
    /** `/registrazione` da fuori sessione, da `F6`: non più un form, un rimando
     *  che dice perché — non c'è una porta aperta, si entra da un invito. */
    registrationMovedTitle: 'Ci si registra da un invito',
    registrationMovedBody:
      'PropFlow non ha una registrazione aperta: chi ti ospita, o chi amministra la piattaforma, ti manda un codice da incollare.',
    registrationMovedAction: 'Ho un codice d’invito',
    email: 'Email',
    password: 'Password',
    passwordHint: 'Almeno 12 caratteri. Una frase lunga vale più di un simbolo.',
    /** L'etichetta cambia con lo stato: l'icona da sola è ambigua fra «adesso è
     *  visibile» e «premi per renderla visibile». */
    showPassword: 'Mostra la password',
    hidePassword: 'Nascondi la password',
    fullName: 'Nome e cognome',
    organizationName: 'Nome dell’organizzazione',
    organizationNameHint: 'Se lo lasci vuoto usiamo il tuo nome.',
    submitLogin: 'Entra',
    toRegister: 'Non hai un account? Registrati',
    toLogin: 'Hai già un account? Entra',
    logout: 'Esci',
    sessionExpired: 'La sessione è scaduta. Entra di nuovo.',

    /** Il recupero della password. Le stringhe dicono **la verità** su ciò che
     *  succede: la risposta è la stessa per un indirizzo noto e per uno
     *  sconosciuto, e dirlo è più onesto che lasciarlo intuire (ADR-052). */
    forgotten: 'Password dimenticata?',
    resetTitle: 'Recupera la password',
    resetSubtitle: 'Ti mandiamo un codice da incollare qui.',
    resetSubmit: 'Mandami il codice',
    /** Non «ti abbiamo mandato un'email»: sarebbe una promessa che il prodotto non
     *  può mantenere per un indirizzo che non ha un account, e dirla renderebbe la
     *  sua assenza un modo per sapere chi è registrato. */
    resetSent:
      'Se quell’indirizzo ha un account, riceverà un codice. Vale 30 minuti e si usa una volta sola.',
    resetCode: 'Codice ricevuto per email',
    /** Come per l'invito: un codice si incolla, non si clicca. Un token dentro un
     *  URL finisce nella cronologia, nel «Referer» e nei log di ogni proxy. */
    resetCodeHint: 'Incollalo come l’hai ricevuto.',
    newPassword: 'Password nuova',
    resetConfirmTitle: 'Scegli la password nuova',
    resetConfirmSubtitle: 'Il codice vale una volta sola.',
    resetConfirmSubmit: 'Cambia la password',
    /** La revoca è l'unica parte con una conseguenza che si sente: va detta prima,
     *  non scoperta dopo trovandosi disconnessi ovunque. */
    resetRevokesSessions:
      'Cambiare la password chiude le sessioni aperte su tutti i dispositivi.',
    resetDone: 'Password cambiata. Entra con quella nuova.',
    backToLogin: 'Torna all’accesso',
  },

  /**
   * L'informativa sulla privacy.
   *
   * **Il testo sta qui e non nel componente**, come ogni altra stringa visibile
   * (regola 18): è prosa lunga, e la tentazione di scriverla nel JSX «perché è un
   * caso a parte» è precisamente il modo in cui l'indirezione si perde. Le sezioni
   * sono un elenco, così aggiungerne una non tocca il componente.
   *
   * **Dice quello che il prodotto fa davvero**, e non quello che si vorrebbe: la
   * cancellazione è una richiesta da scrivere e non un bottone, perché il ruolo
   * applicativo non ha il privilegio `DELETE` e tutto passa da `deleted_at`
   * (ADR-003). Promettere un bottone sarebbe una promessa che il prodotto non può
   * mantenere — vedi `R37`.
   */
  privacy: {
    title: 'Privacy e dati personali',
    subtitle:
      'Che cosa raccogliamo, perché, dove sta e come si chiede la cancellazione.',
    updated: 'Ultimo aggiornamento',
    contactLabel: 'Per qualunque richiesta',
    sections: [
      {
        title: 'Chi tratta i dati',
        body: 'Il titolare del trattamento è la persona o la società che gestisce questa installazione di PropFlow, ed è la stessa a cui hai chiesto di usarlo. I contatti sono in fondo a questa pagina.',
      },
      {
        title: 'Che cosa raccogliamo',
        body: 'Di chi usa il prodotto: nome, email, e la password in una forma da cui non si torna indietro. Di chi è parte di un contratto: nome, email e telefono, se chi amministra li inserisce. Poi ciò che serve al servizio: immobili, contratti, importi, segnalazioni, bollette e i documenti che vengono caricati. Non raccogliamo dati di navigazione a fini pubblicitari e non usiamo strumenti di profilazione.',
      },
      {
        title: 'Perché',
        body: 'Per far funzionare il servizio che hai chiesto: gestire i tuoi immobili e i rapporti con chi ci abita. La base giuridica è il contratto con te, e per i dati di terzi che inserisci — i tuoi inquilini — sei tu a doverli informare, perché sei tu ad averli raccolti.',
      },
      {
        title: 'Dove stanno e per quanto',
        body: 'Su un server nell’Unione Europea, in un database e in uno storage di documenti che stanno sulla stessa macchina. Le copie di sicurezza sono cifrate e conservate fuori da quella macchina. I dati restano finché usi il servizio, e le copie di sicurezza per il periodo dichiarato nel contratto.',
      },
      {
        title: 'Chi altro li vede',
        body: 'Nessuno oltre a chi amministra la tua organizzazione e alle persone a cui quei dati si riferiscono. Non li vendiamo e non li cediamo. Il fornitore del server e quello della posta li trattano per conto nostro, per il solo fatto di ospitarli e di recapitare le email del servizio.',
      },
      {
        title: 'Che cosa puoi chiedere',
        body: 'Di sapere quali dati abbiamo, di correggerli, di riceverne una copia, di limitarne l’uso e di farli cancellare. Scrivi all’indirizzo qui sotto: rispondiamo entro trenta giorni. La cancellazione oggi è una procedura che eseguiamo a mano, non un bottone nel prodotto, e la differenza è che richiede la richiesta scritta.',
      },
      {
        title: 'Che cosa la cancellazione non porta via',
        body: 'Restano le righe che servono a un obbligo di legge — la contabilità, per il tempo che la legge impone — e la traccia tecnica delle operazioni, che non contiene i tuoi dati ma il fatto che qualcosa sia successo. Restano anche i dati che riguardano un’altra persona: se il tuo nome è in un contratto, quel contratto è anche di qualcun altro.',
      },
    ],
  },

  org: {
    label: 'Organizzazione attiva',
    switch: 'Cambia organizzazione',
    role: { owner: 'Proprietario', manager: 'Gestore', tenant: 'Inquilino' },
    /** L'organizzazione attiva è stato esplicito, non un default implicito:
     *  senza `X-Org-Id` gli elenchi rispondono `200` con zero elementi. */
    missing: 'Nessuna organizzazione selezionata.',
    /** Un elenco che sta caricando non deve sembrare vuoto: «non ce ne sono» e
     *  «non le ho ancora lette» sono due cose diverse, e mostrarle uguali fa
     *  cercare un problema che non c'è. */
    loading: 'Cerco le tue organizzazioni…',
    empty: 'Non appartieni a nessuna organizzazione.',
    loadFailed: 'Non riesco a leggere le tue organizzazioni.',
    /** Il nome non è ancora arrivato: si mostra questo e non l'UUID, che è
     *  esattamente ciò da cui questo passo libera. */
    unknownName: 'Organizzazione',

    /**
     * Il terzo stato del guscio (`F7`, ADR-053): nessuna organizzazione, non un
     * errore e non il vuoto di un elenco. Ci si arriva uscendo dall'unica che
     * si aveva — da inquilino, da gestore, o da proprietario non ultimo — non
     * dalla revoca di un invito: un invito non accettato non ha mai fatto
     * entrare nessuno.
     */
    noneTitle: 'Nessuna organizzazione',
    noneBody: (name: string) =>
      `${name}, non fai parte di nessuna organizzazione: su PropFlow si entra da un invito.`,
    /** Finché `/me` non è arrivata: dice la stessa cosa senza il nome. */
    noneBodyFallback: 'Non fai parte di nessuna organizzazione: su PropFlow si entra da un invito.',
    noneHint: 'Se hai un codice, incollalo qui: ti mette dentro l’organizzazione di chi te lo ha mandato.',
    /** `R41`: da ADR-053 a questa schermata si arriva **uscendo**, e chi ci
     *  ripensa la guarda senza sapere che il codice di allora è speso e che il
     *  rientro passa da chi amministra. Condizionale perché la stessa riga
     *  vale per chi verrà tolto (`R39`), che qui atterrerà allo stesso modo. */
    noneAfterLeaving:
      'Se sei uscito da un’organizzazione, il codice di allora non vale più: per rientrare serve un invito nuovo da chi la amministra.',
  },

  /**
   * Il proprio account: i dati anagrafici, il cambio password, l'uscita.
   *
   * **Le conseguenze si dicono prima del bottone**, non si scoprono dopo: cambiare
   * la password disconnette ovunque, e uscire dall'unica organizzazione in cui si
   * abita lascia con la propria, vuota. Un avviso dopo il fatto si legge come un
   * guasto.
   */
  account: {
    menuEntry: 'Il mio account',
    title: 'Il mio account',
    subtitle: 'I tuoi dati, la password e le organizzazioni a cui appartieni.',

    profileTitle: 'I tuoi dati',
    fullName: 'Nome e cognome',
    phone: 'Telefono',
    phoneHint: 'Facoltativo. Lascialo vuoto per toglierlo.',
    /** L'email si mostra e **non si modifica**, con la riga che dice perché: un
     *  campo disabilitato senza spiegazione si legge come un difetto. */
    email: 'Email',
    emailLocked:
      'L’email è la tua credenziale d’accesso e per ora non si cambia da qui.',
    saveProfile: 'Salva i dati',
    profileSaved: 'Dati aggiornati.',

    passwordTitle: 'Cambia la password',
    currentPassword: 'Password attuale',
    /** La stessa frase del recupero (`auth.resetRevokesSessions`): la conseguenza
     *  è identica dai due lati — «ho cambiato la password» — e due frasi diverse
     *  divergerebbero alla prima modifica. */
    passwordRevokesSessions:
      'Cambiando la password chiudi le sessioni aperte su tutti i dispositivi, questo compreso: dovrai rientrare con quella nuova.',
    changePassword: 'Cambia la password',
    /** Non «fatto e basta»: il cambio disconnette, quindi si dice che si sta per
     *  tornare all’accesso. */
    passwordChanged: 'Password cambiata. Entra di nuovo con quella nuova.',
    /** La vecchia sbagliata è un `401` che non dice altro: qui il messaggio è
     *  esplicito perché è l’unico errore su cui chi scrive può agire. */
    currentPasswordWrong: 'La password attuale non è corretta.',

    leaveTitle: 'Esci da un’organizzazione',
    leaveHint: 'Smetti di far parte di un’organizzazione a cui qualcuno ti ha aggiunto.',
    leave: 'Esci',
    /** L’avviso che dice che cosa resta **prima** di uscire: chi esce dall’unica
     *  organizzazione in cui abita resta con la propria, personale e vuota. */
    leaveConfirmTitle: 'Vuoi uscire da {org}?',
    leaveConfirmBody:
      'Non vedrai più i contratti, gli addebiti e le segnalazioni di questa organizzazione. Se qualcuno ti reinvita potrai rientrare.',
    leaveConfirm: 'Sì, esci',
    left: 'Sei uscito da {org}.',
    /** Accanto alla propria organizzazione, dove il bottone «esci» non c'è: ne sei
     *  l'unico proprietario, e dirlo è meglio di un bottone che darebbe errore. */
    leaveLastOwnerShort: 'La tua organizzazione',
  },

  nav,

  /**
   * La guida: una pagina, due versioni scelte dal ruolo. Racconta un ciclo — un
   * immobile, un contratto, un invito, gli addebiti, gli incassi — e per chi abita
   * l'altro lato dello stesso ciclo; non è un elenco di funzioni. Le sezioni si
   * chiamano come le voci di `nav`, e i paragrafi stanno qui e non nel JSX, come
   * `privacy.sections`.
   */
  guide: {
    title: 'Come si usa PropFlow',
    entry: 'Come si entra',
    back: 'Torna all’applicazione',
    /** Il rimando dagli stati vuoti che sono un primo schermo. */
    read: 'Leggi come si usa',
    /** Dalla schermata senza organizzazione: chi aspetta un invito legge la
     *  versione di chi abita. */
    fromNoOrganization: 'Come funziona PropFlow',
    staff: {
      subtitle:
        'Il giro completo di chi amministra: un immobile, un contratto, un invito, gli addebiti, gli incassi.',
      sections: [
        {
          title: 'Come si entra',
          body: 'Si entra da un invito: chi gestisce questa installazione ti ha mandato un codice. Dalla pagina di accesso, «Registrati» lo chiede: lo incolli, scegli nome e password, e la tua organizzazione nasce con te. L’email è quella a cui l’invito è arrivato, e non si cambia. Da lì in poi si entra con email e password; se la password non la ricordi, «Recupera la password» ti manda un codice.',
        },
        {
          title: nav.dashboard,
          body: 'È la prima schermata, e dice che cosa richiede attenzione adesso: quanti immobili e contratti attivi, i canoni attesi al mese, l’insoluto, gli addebiti scaduti, le segnalazioni aperte, i contratti che finiscono entro novanta giorni, le bollette da rivedere. Resta vuota finché non c’è un immobile: ogni numero parte da lì.',
        },
        {
          title: nav.properties,
          body: 'Un immobile è il punto di partenza: si registra con un’etichetta, l’indirizzo e il tipo. Le stanze servono solo se affitti per stanza — ogni contratto potrà poi riguardarne una — e se non ne aggiungi il contratto riguarderà l’immobile intero. Nella scheda stanno gli allegati (planimetrie, certificazioni, verbali) e chi ci abita. Un immobile non si cancella: si archivia, e la traccia resta.',
        },
        {
          title: nav.leases,
          body: 'Un contratto lega un immobile, o una stanza, a chi ci abita per un periodo e un canone. Nasce in bozza: aggiungi le parti — le persone che abiteranno lì, con le quote se il canone si divide — e per ciascuna generi un invito, che è un codice da consegnare; quando lo usa, la persona entra nella tua organizzazione come inquilino e vede il proprio contratto. Poi lo attivi, e da quel momento il periodo è occupato. Si disdice ciò che è in corso e si annulla ciò che non è mai entrato in vigore: sono due cose diverse, e i due bottoni lo dicono.',
        },
        {
          title: nav.charges,
          body: 'Un addebito è una somma chiesta a chi abita: il canone, una spesa, una bolletta riaddebitata. Si emette dalla scheda del contratto, con una scadenza; da quella lo stato si deriva da sé — scaduto quando la data passa, pagato quando entra tutto. Un addebito emesso non si modifica: si annulla e si riemette. Se non incasserai mai, la rinuncia lo chiude e la cifra finisce fra le perdite del riepilogo.',
        },
        {
          title: nav.payments,
          body: 'Quando un inquilino dichiara di aver pagato, la dichiarazione arriva qui, in attesa: finché non la confermi non muove il saldo, e puoi rifiutarla. Gli incassi che ricevi direttamente li registri dalla scheda dell’addebito, e il residuo si aggiorna da solo. Sotto «Tutti» c’è ogni movimento, decisi compresi.',
        },
        {
          title: nav.bills,
          body: 'Una bolletta è una spesa dell’immobile — luce, gas, acqua, condominio — da riaddebitare a chi abita. La registri con fornitore, periodo e importo e alleghi la scansione: la lettura automatica propone i valori che ha letto accanto a quelli che hai scritto, e li rivedi tu. Con «Riaddebita» nasce un addebito sul contratto che scegli, per la quota che indichi.',
        },
        {
          title: nav.tickets,
          body: 'Una segnalazione è un guasto o un problema in casa, aperto da chi ci abita — o da te. La prendi in carico, la lavori, la chiudi, e ogni passaggio resta scritto con la data. Dentro ci sono una conversazione, gli allegati (una foto dice più di una descrizione) e i costi sostenuti. Con «Solo quelle aperte» spuntato vedi ciò che è in lavorazione; le chiuse restano sotto «Tutte».',
        },
        {
          title: nav.account,
          body: 'Dal menù dell’organizzazione — in fondo alla barra laterale, o in alto dal telefono: i tuoi dati, la password, le organizzazioni a cui appartieni e l’uscita da ciascuna, l’informativa sulla privacy. Se hai ricevuto un altro codice d’invito, «Ho un codice d’invito» sta nello stesso menù.',
        },
      ],
    },
    tenant: {
      subtitle:
        'L’altro lato dello stesso giro: chi amministra registra il contratto e gli addebiti, tu vedi quanto devi e dichiari quando paghi.',
      sections: [
        {
          title: 'Come si entra',
          body: 'Chi amministra la casa in cui abiti ti ha invitato con un codice. Dalla pagina di accesso, «Registrati» lo chiede: lo incolli, scegli nome e password, e sei dentro la sua organizzazione come inquilino: non ne hai una tua, e vedi solo ciò che riguarda te. L’email è quella a cui l’invito è arrivato. Da lì in poi si entra con email e password; se la password non la ricordi, «Recupera la password» ti manda un codice.',
        },
        {
          title: nav.charges,
          body: 'È la prima schermata: quanto devi, con la scadenza e quanto resta da pagare di ciascun addebito. Gli addebiti li emette chi amministra — il canone, le spese, le bollette riaddebitate — e quando ne arriva uno lo vedi qui. Quando paghi, apri l’addebito e dichiaralo con «Ho pagato»: resta in attesa finché chi amministra lo conferma, e da quel momento il residuo scende. Non si può ritirare una dichiarazione: se hai sbagliato, dillo a chi amministra.',
        },
        {
          title: nav.lease,
          body: 'Il tuo contratto: l’immobile o la stanza, il periodo, il canone e la tua quota se lo dividi con altri. Finché chi amministra non lo attiva è in bozza, e lo vedi lo stesso. Nella scheda stanno gli allegati — il contratto firmato, le ricevute — e gli addebiti che ne derivano.',
        },
        {
          title: nav.bills,
          body: 'Le bollette dell’immobile che chi amministra ti ha riaddebitato, con la scansione da aprire. Non le modifichi: sono qui perché un addebito che nasce da una bolletta si possa risalire al documento, invece di accettare una cifra sulla fiducia.',
        },
        {
          title: nav.tickets,
          body: 'Se qualcosa non funziona in casa, segnalalo qui: una riga per dire che cosa succede, una foto se aiuta. Resta scritto con la data, chi amministra lo vede subito e lo prende in carico, e ogni passaggio fino alla chiusura lo vedi nella scheda, dove c’è anche una conversazione. Puoi annullare una segnalazione finché è ancora aperta.',
        },
        {
          title: nav.expenses,
          body: 'Il conto fra coinquilini: un gruppo è la cassa di una casa. Chi anticipa una spesa la registra, dicendo con chi la divide; i saldi dicono chi deve a chi, e quando qualcuno rimborsa lo dichiara. Chi amministra non lo vede: è denaro fra chi abita, non fra voi e la proprietà.',
        },
        {
          title: nav.account,
          body: 'Dal menù dell’organizzazione — in fondo alla barra laterale, o in alto dal telefono: i tuoi dati, la password, l’informativa sulla privacy, e l’uscita dall’organizzazione — che non è possibile finché il tuo contratto è in vigore. Uscire spende il codice con cui eri entrato: se ci ripensi, per rientrare serve un invito nuovo da chi amministra. Se ricevi un altro codice d’invito, «Ho un codice d’invito» sta nello stesso menù.',
        },
      ],
    },
  },

  dashboard: {
    title: 'Riepilogo',
    subtitle: 'Che cosa richiede attenzione, adesso.',
    propertiesCount: 'Immobili',
    activeLeases: 'Contratti attivi',
    monthlyRentExpected: 'Canoni attesi al mese',
    monthlyRentNote: 'Somma dei canoni dei contratti attivi.',
    totalOutstanding: 'Insoluto',
    totalOutstandingNote: 'Residuo dovuto, esclusi annullati e stornati.',
    overdueCharges: 'Addebiti scaduti',
    openTickets: 'Segnalazioni aperte',
    leasesExpiring: 'Contratti in scadenza',
    leasesExpiringNote: 'Entro novanta giorni.',
    billsPendingReview: 'Bollette da rivedere',
    billsPendingReviewNote: 'In bozza o lette dalla macchina.',
    writtenOffTotal: 'Perdite',
    writtenOffTotalNote: 'Residuo degli addebiti stornati. Gli annullati non contano.',
    goToWrittenOff: 'Vedi gli addebiti stornati',
    goToOverdue: 'Vedi lo scadenzario',
    goToBills: 'Vedi le bollette',
    goToLeases: 'Vedi i contratti',
    goToTickets: 'Vedi le segnalazioni',
    goToProperties: 'Vedi gli immobili',
    /** Il vuoto del riepilogo è il **primo schermo** di ogni proprietario nuovo:
     *  racconta il giro in tre passi con i tre gesti, e rimanda alla guida. Non è
     *  un primo accesso guidato con un overlay: è lo stesso `Empty`, con un
     *  `action` che è un gruppo di tre. */
    empty: 'Si comincia da un immobile.',
    emptyHint:
      'Il giro è in tre passi: registri un immobile, redigi un contratto su di esso, inviti chi ci abita. Da lì arrivano gli addebiti e gli incassi, e questa pagina si riempie.',
    /** L'invito si genera dalla scheda del contratto: il terzo passo porta ai
     *  contratti, che è dove sta. */
    inviteStep: 'Invita chi abita',
  },

  property: {
    title: 'Immobili',
    newTitle: 'Nuovo immobile',
    create: 'Registra un immobile',
    label: 'Etichetta',
    labelHint: 'Come lo chiami tu: «Bilocale centro», «Via Roma 1».',
    type: 'Tipologia',
    status: 'Stato',
    address: 'Indirizzo',
    street: 'Via e numero',
    city: 'Comune',
    province: 'Provincia',
    postalCode: 'CAP',
    surface: 'Superficie',
    rooms: 'Stanze',
    roomsCount: 'Locali',
    floor: 'Piano',
    energyClass: 'Classe energetica',
    notes: 'Note',
    archive: 'Archivia',
    archiveConfirm: 'Archiviare questo immobile?',
    /** In blocco cambia il numero, non la frase: la conseguenza è la stessa. */
    archiveConfirmMany: 'Archiviare {count} immobili?',
    archiveExplain:
      'Non viene cancellato: sparisce dagli elenchi e non è più modificabile. La traccia resta.',
    archived: 'Immobile archiviato.',
    created: 'Immobile registrato.',
    updated: 'Immobile aggiornato.',
    empty: 'Nessun immobile registrato.',
    emptyHint:
      'Un immobile è il punto di partenza: da lì nascono le stanze, i contratti e gli addebiti.',
    /** Chi abita non registra immobili, e l'elenco gli mostra solo quelli dei
     *  suoi contratti: il vuoto dice chi lo riempie e quando. */
    emptyForTenant:
      'Gli immobili li registra chi amministra: il tuo compare qui quando un contratto a tuo nome lo riguarda.',
    residents: 'Chi ci abita',
    residentsEmpty: 'Nessuno ci abita in questo momento.',
    residentsEmptyHint:
      'Chi abita compare qui quando un contratto su questo immobile è attivo e la persona ha accettato l’invito.',
    residentPending: 'Invitato, non ha ancora accettato',
    notFound: 'Questo immobile non esiste, o non è tuo.',
    types: {
      apartment: 'Appartamento',
      house: 'Casa indipendente',
      studio: 'Monolocale',
      villa: 'Villa',
      office: 'Ufficio',
      retail: 'Locale commerciale',
      garage: 'Box o garage',
      land: 'Terreno',
      other: 'Altro',
    },
    statuses: {
      draft: 'Bozza',
      available: 'Disponibile',
      partially_rented: 'Parzialmente affittato',
      rented: 'Affittato',
      maintenance: 'In manutenzione',
      archived: 'Archiviato',
    },
    /** Lo stato che chi amministra decide: gli altri due discendono dai contratti
     *  e non si scelgono (ADR-019). */
    statusLabel: 'Stato dichiarato',
    statusHint:
      'Affittato e parzialmente affittato non si scelgono: li calcolano i contratti in essere.',
    /** Alla creazione la frase è un'altra: lì non serve dire quali stati mancano,
     *  serve dire a che cosa servono i due che ci sono. «Bozza» è una scelta
     *  legittima — chi sta ancora compilando — ma è anche quella che rifiuta
     *  l'attivazione di un contratto, e chi la sceglie deve saperlo adesso e non
     *  dal `409`. */
    statusCreateHint:
      'Bozza è per chi sta ancora compilando: un immobile in bozza non si può affittare.',
    /** Il badge mostra il fatto; il dichiarato compare accanto solo quando è un
     *  giudizio che il fatto smentisce — un immobile in manutenzione e insieme
     *  affittato. Nella scheda c'è comunque, per chi lo cerca. */
    filteredByDeclaredStatus:
      'Il filtro usa lo stato dichiarato; il badge mostra quello che dicono i contratti in essere.',
    occupancy: 'Occupazione',
    filterOccupied: 'Con contratti in essere',
    filterFree: 'Senza contratti in essere',
  },

  room: {
    title: 'Stanze',
    create: 'Aggiungi una stanza',
    newTitle: 'Nuova stanza',
    name: 'Nome',
    nameHint: 'Unico nell’immobile: «Camera doppia», «Singola sul cortile».',
    monthlyRent: 'Canone mensile',
    surface: 'Superficie',
    floor: 'Piano',
    privateBathroom: 'Bagno privato',
    furnished: 'Arredata',
    status: 'Stato',
    utilityShare: 'Quota delle utenze',
    /** Un campo numerico vuoto, in una schermata di co-living, è ambiguo fra
     *  «zero» e «non specificato»: qui il vuoto ha un significato e va scritto. */
    utilityShareHint: 'Percentuale delle utenze a carico di questa stanza. Vuota vale «in parti uguali».',
    /** Il nome è unico nell'immobile, quindi «tanto lo sistemo dopo» è proprio
     *  la cosa da dire a chi crea una stanza — e non era detta da nessuna parte. */
    newExplain: 'Non serve avere tutto adesso: ogni dato si corregge dopo, aprendo la stanza dall’elenco.',
    /** L'unica superficie che una stanza abbia: `GET /rooms/{id}` non esiste. */
    dialogExplain: 'Correggi i dati della stanza. Per liberarne il nome, archiviala dall’elenco.',
    archive: 'Archivia',
    archiveConfirm: 'Archiviare questa stanza?',
    archiveConfirmMany: 'Archiviare {count} stanze?',
    archiveExplain: 'Il nome torna libero per una stanza nuova. La traccia resta.',
    created: 'Stanza aggiunta.',
    updated: 'Stanza aggiornata.',
    archived: 'Stanza archiviata.',
    empty: 'Nessuna stanza: il contratto riguarderà l’immobile intero.',
    emptyHint:
      'Le stanze servono solo se affitti per stanza: ogni contratto potrà poi riguardarne una.',
    nameTaken: 'Questo nome è già usato da un’altra stanza dell’immobile.',
    statuses: {
      available: 'Disponibile',
      occupied: 'Occupata',
      unavailable: 'Non disponibile',
    },
    /** «Occupata» non è una scelta: discende dai contratti. Quello che si dichiara
     *  è se la stanza sia fuori uso. */
    statusLabel: 'Stato dichiarato',
    statusHint: 'Occupata non si sceglie: lo dice il contratto in essere.',
  },

  lease: {
    title: 'Contratti',
    newTitle: 'Nuovo contratto',
    create: 'Redigi un contratto',
    property: 'Immobile',
    room: 'Stanza',
    wholeProperty: 'Immobile intero',
    /** L'avviso arriva **prima**, ma non vieta: due bozze sullo stesso periodo
     *  sono ipotesi alternative, e i vincoli del database le ammettono apposta. */
    overlapTitle: 'Su questo periodo c’è già un contratto',
    overlapExplain:
      'La bozza si crea lo stesso — due ipotesi sullo stesso periodo sono ammesse — ma l’attivazione verrà rifiutata finché l’altro contratto è in essere.',
    overlapAcknowledge: 'L’ho visto: crea la bozza lo stesso.',
    overlapPeriod: (start: string, end: string) => `In essere dal ${start} al ${end}.`,
    overlapNoEnd: (start: string) => `In essere dal ${start}, senza scadenza.`,
    startDate: 'Inizio',
    endDate: 'Fine',
    endDateHint: 'Esclusiva: per un contratto valido fino al 31 dicembre indica il 1° gennaio.',
    rentAmount: 'Canone mensile',
    depositAmount: 'Deposito',
    paymentDay: 'Giorno di pagamento',
    status: 'Stato',
    effectiveStatus: 'Stato alla data odierna',
    inForce: 'In essere',
    type: 'Tipo',
    parties: 'Parti contrattuali',
    partyName: 'Nome e cognome',
    partyEmail: 'Email',
    partyRole: 'Ruolo',
    partyShare: 'Quota',
    partyPrimary: 'Intestatario principale',
    addParty: 'Aggiungi una parte',
    partyAdded: 'Parte aggiunta.',
    partyPhone: 'Telefono',
    /** Il residuo detto **prima** di digitare: proponendo 100 a tutti, il primo si
     *  prendeva tutto e all'invito successivo restava zero (ADR-046). */
    partyShareRemaining: (remaining: string) => `Resta da assegnare: ${remaining} %.`,
    partyPrimaryTaken:
      'L’intestatario principale c’è già: è uno solo per contratto. Per cambiarlo, correggi la parte che lo è adesso.',
    editParty: 'Correggi',
    editPartyTitle: 'Correggi la parte',
    /** La correzione esiste perché una quota sbagliata bloccava tutto: il
     *  contratto non si attivava e non si poteva nemmeno rifare. */
    editPartyExplain:
      'Si corregge anche a contratto attivo: è il modo di sistemare le quote sbagliate che sono già in essere.',
    partyCorrected: 'Parte corretta.',
    /** L'email è ciò a cui l'invito è intestato: cambiarla dopo sposterebbe il
     *  rapporto sotto i piedi di chi l'ha accettato. */
    partyEmailLocked: 'Non si cambia: questa persona ha già accettato l’invito.',
    removeParty: 'Togli dalla lista',
    removePartyConfirm: 'Togliere questa parte?',
    removePartyExplain:
      'Si toglie ciò che è stato inserito per sbaglio. Il posto di intestatario principale torna libero.',
    partyRemoved: 'Parte tolta.',
    activate: 'Attiva',
    activateConfirm: 'Attivare questo contratto?',
    activateExplain:
      'Il periodo viene occupato e il contratto diventa in essere. Serve almeno un inquilino, e le quote devono sommare a 100.',
    activated: 'Contratto attivato.',
    /** Annullare e disdire non sono la stessa cosa, e le due frasi devono
     *  dirlo: si annulla ciò che non è mai stato, si disdice ciò che è in corso. */
    cancel: 'Annulla il contratto',
    cancelConfirm: 'Annullare questo contratto?',
    cancelExplain:
      'Resta negli elenchi, con lo stato «Annullato», e il suo periodo torna libero per un altro contratto. Si annulla solo ciò che non è mai entrato in vigore.',
    cancelled: 'Contratto annullato.',
    terminate: 'Disdici',
    terminateConfirm: 'Disdire questo contratto?',
    terminateExplain: 'Indica la data di fine rapporto. Il periodo si libera.',
    terminationDate: 'Data di fine rapporto',
    terminated: 'Contratto disdetto.',
    created: 'Contratto redatto.',
    empty: 'Nessun contratto.',
    emptyHint:
      'Un contratto lega un immobile a chi ci abita. Nasce in bozza: si aggiungono le parti, si attiva, e da lì partono gli addebiti.',
    /** Il vuoto di chi abita non è lo stesso di chi amministra: lui non ha un
     *  gesto per riempirlo, e dirgli «creane uno» sarebbe indicargli una porta
     *  che l'API gli chiude. Gli si dice chi lo riempie. */
    emptyForTenant:
      'Non risulta nessun contratto a tuo nome in questa organizzazione. I contratti li redige chi amministra: se te ne aspetti uno, chiedi a chi ti ha invitato.',
    /** Il vuoto degli addebiti dentro la scheda: il gesto per chi lo ha, chi lo
     *  farà per chi non lo ha. */
    chargesEmptyHint:
      'Il primo addebito si emette da qui: il canone, una spesa, una bolletta riaddebitata.',
    chargesEmptyForTenant:
      'Gli addebiti di questo contratto li emette chi amministra: quando ne arriva uno lo vedi qui e negli addebiti.',
    noTenantParty: 'Il contratto non ha inquilini fra le parti.',
    sharesNotHundred: 'Le quote degli inquilini non sommano a 100.',
    periodOverlap: 'Il periodo si sovrappone a una locazione già attiva.',
    /** `expiring` ed `expired` sono derivati: si mostrano e non si filtrano. */
    derivedStatusNote:
      'In scadenza e scaduto sono stati derivati dalla data: si vedono, non si filtrano.',
    /** La risposta alla domanda che il filtro fa nascere: «ho chiesto gli attivi e
     *  me ne torna uno marcato Scaduto». Sta accanto al filtro perché è lì che la
     *  domanda nasce, e non ripetuta su ogni riga — che era `R30`. */
    filteredByDeclaredStatus:
      'Il filtro usa lo stato dichiarato; il badge mostra quello derivato dalle date.',
    /** Lo stato che qualcuno ha scelto, distinto da quello che le date dicono. */
    statusLabel: 'Stato dichiarato',
    charges: 'Addebiti del contratto',
    issueCharge: 'Emetti un addebito',
    types: { whole_property: 'Immobile intero', room: 'Stanza' },
    statuses: {
      draft: 'Bozza',
      pending_signature: 'In attesa di firma',
      active: 'Attivo',
      expiring: 'In scadenza',
      expired: 'Scaduto',
      terminated: 'Disdetto',
      cancelled: 'Annullato',
    },
    partyRoles: {
      tenant: 'Inquilino',
      cotenant: 'Co-inquilino',
      guarantor: 'Garante',
    },
    frequencies: {
      monthly: 'Mensile',
      quarterly: 'Trimestrale',
      semiannual: 'Semestrale',
      annual: 'Annuale',
    },
  },

  /**
   * Gli allegati, con un catalogo solo per tutti e sei gli owner.
   *
   * Le stringhe sono deliberatamente generiche — «Allegati», «Carica un file» —
   * perché la stessa scheda compare su un immobile, una stanza, un contratto, una
   * segnalazione e un addebito. Dove il nome proprio conta, come la *scansione* di
   * una bolletta, la schermata passa il proprio titolo.
   */
  document: {
    title: 'Allegati',
    upload: 'Carica un file',
    uploadHint: 'PDF o immagine, fino a 25 MB.',
    download: 'Scarica',
    empty: 'Nessun allegato.',
    emptyHint: 'Carica un PDF o un’immagine: resta qui, con la data e chi lo ha caricato.',
    /** Chi abita e non può caricare — le scansioni di una bolletta — legge chi
     *  lo farà, non un titolo e basta. */
    emptyForTenant: 'Gli allegati qui li carica chi amministra: quando ci sono, li apri da questa scheda.',
    archive: 'Archivia',
    archiveConfirm: 'Archiviare questo allegato?',
    archiveConfirmMany: 'Archiviare {count} allegati?',
    /** La policy ammette lo staff **oppure** chi ha caricato: il messaggio non
     *  promette un ruolo, perché all'uploader quel ruolo non serve. */
    archiveExplain:
      'Sparisce dall’elenco e non si scarica più. La traccia resta, e il file non si ricarica da qui: bisogna caricarne un altro.',
    /* I titoli propri di chi ha già un nome per i suoi allegati. */
    propertyTitle: 'Documenti dell’immobile',
    propertyHint: 'Planimetrie, certificazioni, verbali.',
    roomTitle: 'Allegati della stanza',
    leaseTitle: 'Documenti del contratto',
    leaseHint: 'Il contratto firmato, i documenti d’identità, le ricevute.',
    ticketTitle: 'Foto e allegati',
    ticketHint: 'Una foto dice più di una descrizione: allegala qui.',
    chargeTitle: 'Allegati dell’addebito',
    chargeHint: 'La fattura, la ricevuta, il conteggio.',
  },

  invitation: {
    title: 'Inviti',
    invite: 'Invita un inquilino',
    inviteTitle: 'Invita un inquilino',
    email: 'Email dell’invitato',
    fullName: 'Nome e cognome',
    /** Che quota riceverà, detto prima di generare l'invito: è il passo in cui una
     *  quota a zero passava inosservata. */
    shareOnAccept: (share: string) =>
      `Alla parte creata da questo invito andrà il ${share} % del canone. Si corregge dopo, dall’elenco delle parti.`,
    send: 'Genera l’invito',
    revoke: 'Revoca',
    revoked: 'Invito revocato.',
    created: 'Invito creato.',
    /** Niente bugie nell'interfaccia: `Notifier` non ha un adattatore, quindi
     *  nessuna email parte e il token lo consegna il proprietario. */
    tokenTitle: 'Consegna tu questo invito',
    tokenExplain:
      'L’invio automatico delle email non è ancora attivo: copia questo codice e mandalo tu, come preferisci. Vale sette giorni e si vede una volta sola.',
    tokenLabel: 'Codice d’invito',
    /** Dove il codice si usa. Senza questa riga il proprietario ha in mano una
     *  credenziale e nessun posto a cui mandare chi la riceve. */
    tokenWhereToUse:
      'Chi lo riceve deve prima creare il proprio account, poi incollare il codice in «Ho un codice d’invito».',
    alreadyPending: 'C’è già un invito in attesa per questa email.',
    pending: 'In attesa',
    accepted: 'Accettato',
    expired: 'Scaduto',

    /* La revoca, che vive **solo** qui.
       Nessuna rotta elenca gli inviti: `revokeInvitation` vuole l'id, e l'unico
       momento in cui il client ce l'ha è subito dopo averlo creato. Costruire un
       elenco vorrebbe dire aggiungere una rotta. */
    revokeTitle: 'Revocare questo invito?',
    revokeExplain:
      'Il codice smette di funzionare subito e potrai rifarne uno. È l’unico momento in cui puoi revocarlo: chiusa questa finestra, resta solo aspettare che scada.',
    revokeDone: 'Invito revocato. Puoi rifarlo qui, con l’indirizzo giusto.',

    /* L'accettazione, che è la metà del giro che mancava. */
    acceptEntry: 'Ho un codice d’invito',
    acceptTitle: 'Accetta un invito',
    acceptSubtitle:
      'Incolla il codice che ti ha mandato chi affitta la casa: ti mette dentro la sua organizzazione, dove trovi il contratto e quello che devi.',
    acceptSubmit: 'Entra',
    acceptCodeHint: 'Vale sette giorni dall’emissione. Non è la password del tuo account.',
    /** `auth.invalid_credentials` è ciò che l'API risponde a un codice sbagliato,
     *  scaduto o intestato a un altro indirizzo: distinguerli direbbe a chi prova
     *  quali inviti esistono. Il messaggio del catalogo — «email o password non
     *  corrette» — qui sarebbe fuori posto, quindi la schermata lo riscrive. */
    acceptCodeRejected:
      'Questo codice non è valido: può essere sbagliato, scaduto, o intestato a un’altra email. Chiedine uno nuovo a chi te lo ha mandato.',
    /** «Già accettato» non vuol dire «ancora dentro»: dalla sezione 6 si può
 *  uscire, e chi ripresenta il codice dopo l'uscita è esattamente chi legge
 *  questa riga. Un invito si spende una volta; la via per rientrare è un
 *  altro invito (`R41`). */
    acceptAlreadyDone:
      'Questo invito è già stato usato e non vale una seconda volta. Se sei uscito dall’organizzazione, chiedi un invito nuovo a chi te lo aveva mandato.',

    /**
     * `/invito` da fuori sessione (`F7`, ADR-053): il codice **prima**, il form
     * di registrazione **dopo**, con il codice già dentro. Due passi e non uno
     * solo perché il primo non fa nessuna chiamata — non esiste una rotta di
     * sola lettura che dica di che genere è un invito, per non regalare un
     * oracolo su quali codici esistono — e il secondo si adatta a quel che il
     * server risponde davvero.
     */
    entryTitle: 'Entra con un invito',
    entrySubtitle:
      'Incolla il codice che hai ricevuto: da chi ti affitta casa, o da chi amministra la piattaforma se diventi proprietario.',
    entryContinue: 'Continua',
    /** Step due: nessun campo email, è quella dell'invito (ADR-053, decisione 6). */
    registerTitle: 'Crea il tuo account',
    registerSubtitle: 'Il codice è quello che hai appena incollato: non serve un’altra email.',
    registerSubmit: 'Crea l’account ed entra',
    /** Compare **solo** dopo `409 invitation.owner_token`: il codice incollato
     *  era da proprietario, non da inquilino, e crea un'organizzazione. */
    ownerTokenPrompt:
      'Questo codice è da proprietario: crea anche un’organizzazione. Dalle un nome.',
    backToCode: 'Ho sbagliato codice',
  },

  ticket: {
    title: 'Segnalazioni',
    /** L'inquilino apre un guasto; il proprietario lo lavora. La stessa schermata
     *  vista da due ruoli, e non due schermate da tenere allineate. */
    tenantSubtitle: 'I guasti che hai segnalato, e a che punto sono.',
    staffSubtitle: 'I guasti segnalati, e chi li sta lavorando.',
    create: 'Segnala un guasto',
    newTitle: 'Segnala un guasto',
    detailTitle: 'Segnalazione',
    backToList: 'Torna alle segnalazioni',
    reference: 'Riferimento',
    /** Il numero è **per organizzazione**: dice quante segnalazioni ha aperto
     *  quel proprietario, non quante ne esistono sulla piattaforma. */
    referenceHint: 'Il numero è progressivo dentro l’organizzazione.',
    ticketTitle: 'Che cosa succede',
    ticketTitleHint: 'Una riga: «Il boiler perde», «La serratura non chiude».',
    description: 'Raccontalo per esteso',
    descriptionHint: 'Da quando, dove, che cosa hai già provato. Quello che scrivi ora resta.',
    property: 'Immobile',
    room: 'Stanza',
    lease: 'Contratto',
    /** Senza `lease_id` la segnalazione non la vede nessun inquilino:
     *  `app.can_access_ticket` passa dal contratto. Stessa forma di
     *  `payer_user_id` nella fetta A — il difetto non è che il campo sia
     *  facoltativo, è che ometterlo sia il default. */
    leaseHint: 'È ciò che rende la segnalazione visibile a chi abita.',
    leaseNone: 'Nessun contratto',
    leaseNoneHint:
      'La vedrai tu e chi amministra, e nessun altro: senza contratto indicato nessun inquilino la trova, nemmeno chi abita quella casa.',
    wholeProperty: 'Tutto l’immobile',
    category: 'Categoria',
    priority: 'Priorità',
    status: 'Stato',
    openedAt: 'Aperta il',
    openedBy: 'Aperta da',
    closedAt: 'Chiusa il',
    assignedTo: 'In carico a',
    assignedToNobody: 'Nessuno l’ha ancora presa in carico',
    /** `estimated_cost` è `null` per chi non è staff **anche quando un valore
     *  c'è**: mostrare `0,00 €` sarebbe una bugia, il trattino è la verità. */
    estimatedCost: 'Costo stimato',
    estimatedCostHint: 'È la trattativa con il fornitore: la vede solo chi amministra.',
    actualCost: 'Costo sostenuto',
    actualCostHint: 'Lo vede chiunque veda la segnalazione: chi ha subìto il guasto sa quanto è costato ripararlo.',
    costs: 'Costi',
    editCosts: 'Modifica i costi',
    /** L'unico `PATCH` del modulo, ed è solo questo: titolo e descrizione sono il
     *  testo con cui una persona ha descritto un guasto in un momento preciso. */
    onlyCostsArePatchable:
      'Di una segnalazione si modificano solo i costi. Il testo resta com’è stato scritto: per correggere il tiro c’è il thread.',
    resolutionNotes: 'Com’è andata a finire',
    resolutionNotesHint: 'Facoltativo. Una riapertura la cancella, perché fa parte della chiusura.',
    history: 'Storico degli stati',
    /** Irraggiungibile in pratica — la prima riga la scrive il trigger — ma
     *  «Non c'è ancora niente» non direbbe che cosa ci finirà. */
    historyEmpty: 'Nessun passaggio di stato.',
    historyEmptyHint:
      'Ogni passaggio — presa in carico, lavorazione, chiusura — resta scritto qui, con la data e chi lo ha fatto.',
    /** La prima riga la scrive il trigger all'inserimento, fuori da un contesto
     *  di richiesta: `changed_by` può quindi mancare, e non è un difetto. */
    historyOpened: 'Aperta',
    thread: 'Conversazione',
    /** Il thread non è live per scelta: i messaggi arrivano quando li si va a
     *  prendere, e il bottone lo dice invece di far credere il contrario. */
    threadRefresh: 'Aggiorna',
    threadEmpty: 'Nessun messaggio.',
    threadEmptyHint: 'Scrivi qui per aggiungere qualcosa: il thread non si riscrive mai.',
    message: 'Messaggio',
    messagePlaceholder: 'Scrivi un aggiornamento…',
    send: 'Invia',
    sent: 'Messaggio inviato.',
    /** Non si modifica e non si cancella, nemmeno in modo soft: due sole policy,
     *  `_select` e `_insert`. Va detto prima di inviare, non dopo. */
    messageIsFinal: 'Un messaggio inviato non si modifica e non si cancella.',
    internalNote: 'Nota interna',
    internalNoteHint: 'La vede solo chi amministra: all’inquilino non arriva affatto.',
    triage: 'Classifica',
    triageTitle: 'Classifica la segnalazione',
    triageExplain:
      'Categoria e priorità si impostano qui, ed è ciò che «classificata» significa: dopo non si cambiano più altrove.',
    take: 'Prendi in carico',
    takeTitle: 'Prendere in carico questa segnalazione?',
    /** L'assegnazione è solo a sé stessi, e non è una semplificazione:
     *  un'organizzazione ha esattamente uno staff, perché gli inviti scrivono
     *  `role: tenant` fisso e `manager` non lo scrive nessuno. */
    takeExplain: 'Risulterai tu come responsabile della lavorazione.',
    start: 'Avvia la lavorazione',
    resume: 'Riprendi la lavorazione',
    wait: 'Metti in attesa',
    waitTitle: 'Su chi stai aspettando?',
    /** Non è una fase a sé ma una sospensione: serve a dire perché una
     *  segnalazione è ferma. Senza un nome resta una che sembra abbandonata. */
    waitExplain:
      'La lavorazione resta ferma e si vede perché. Si riprende con «riprendi la lavorazione».',
    waitOn: 'Aspetto',
    waitOnTenant: 'L’inquilino',
    waitOnOwner: 'Il proprietario',
    close: 'Chiudi',
    closeTitle: 'Chiudere questa segnalazione?',
    closeExplain:
      'Puoi scrivere com’è andata a finire: la legge anche chi ha segnalato. Se il guasto torna, la segnalazione si riapre.',
    reopen: 'Riapri',
    reopenTitle: 'Riaprire questa segnalazione?',
    /** Riaprire disfa la chiusura, e la nota di risoluzione fa parte della
     *  chiusura: una `open` che dichiara come è stata risolta direbbe il falso
     *  sul proprio stato. */
    reopenExplain:
      'Torna aperta, e la nota su com’era andata a finire viene cancellata: fa parte della chiusura. Quello che deve restare va scritto nel thread.',
    cancel: 'Annulla',
    cancelTitle: 'Annullare questa segnalazione?',
    cancelTitleMany: 'Annullare {count} segnalazioni?',
    cancelExplain:
      'Vuol dire «non serve più». Resta visibile con la sua storia, e non si modifica più.',
    /** L'API risponde `ticket.not_yours` e non `org.staff_role_required`: non
     *  manca un ruolo, manca l'essere stati noi a segnalare. Il messaggio del
     *  primo caso sta fra gli `errors`, dove si instrada sul codice; questo
     *  spiega **prima** il caso in cui il bottone non compare affatto. */
    onlyOpenCancelledByReporter:
      'Puoi annullarla finché nessuno ci ha messo mano. Da qui in poi la annulla chi amministra.',
    /** La visibilità è una scelta di chi apre, e il default resta «privata».
     *  Il confine è nel database — `app.can_access_ticket`, migration `0035` — e
     *  non una preferenza di interfaccia. */
    visibleToResidents: 'Riguarda tutta la casa',
    visibleToResidentsHint:
      'La leggono anche gli altri che abitano in questo immobile, con la conversazione e le foto. Senza la casella la vedi solo tu e chi amministra.',
    visibility: 'Visibilità',
    visibilityShared: 'Visibile a chi abita qui',
    visibilityPrivate: 'Solo tu e chi amministra',
    /** La correzione è una rotta a sé — `PATCH /tickets/{id}/report` — perché ha
     *  un'autorizzazione sola: lo staff sempre, chi ha segnalato finché nessuno
     *  ci ha messo mano. */
    correct: 'Correggi',
    correctTitle: 'Correggi la segnalazione',
    correctExplain:
      'Puoi sistemare il testo e cambiare idea sulla visibilità. Categoria e priorità no: le decide chi amministra classificandola.',
    onlyOpenCorrectedByReporter:
      'Puoi correggerla finché nessuno ci ha messo mano. Da qui in poi la corregge chi amministra.',
    /** Un thread terminato si **legge** e non si scrive: è una regola di dominio,
     *  e il client dice perché invece di far sparire la casella in silenzio. */
    threadClosed: 'La conversazione è chiusa.',
    threadClosedHintReporter:
      'Se il guasto non è risolto, riapri la segnalazione: il motivo che scrivi diventa il primo messaggio.',
    threadClosedHint: 'Riaprendo la segnalazione la conversazione riprende.',
    /** Il motivo è obbligatorio perché è l'unica cosa che spiega perché la
     *  segnalazione è tornata viva: dopo la chiusura non si scrive più. */
    reopenReason: 'Perché la riapri',
    reopenReasonHint: 'Finisce nella conversazione, e resta anche dopo la prossima chiusura.',
    reopenReasonPlaceholder: 'Ha ricominciato a perdere due giorni dopo…',
    onlyOpen: 'Solo quelle aperte',
    /** `?open=true` non è `status <> 'closed'`: l'indice parziale
     *  `tickets_open_ix` è definito su esattamente sei valori. */
    onlyOpenHint: 'Le sei fasi di lavorazione, chiuse e annullate escluse.',
    empty: 'Nessuna segnalazione.',
    emptyOpen: 'Nessuna segnalazione aperta.',
    /** Il vuoto con «Solo quelle aperte» spuntato è il vuoto di un **filtro**,
     *  non della sezione: dice dove stanno le altre, e non spiega la sezione a
     *  chi la conosce già. Il gesto resta, perché è anche la prima schermata. */
    emptyOpenHint:
      'Niente in lavorazione. Le segnalazioni chiuse e annullate compaiono togliendo «Solo quelle aperte».',
    emptyHintTenant:
      'Se qualcosa non funziona in casa, segnalalo qui: resta scritto, con la sua data, e chi amministra lo vede subito.',
    emptyHintStaff:
      'Quando un inquilino segnala un guasto, lo trovi qui e lo prendi in carico. Puoi aprirne una anche tu.',
    /** Nessun immobile su cui aprire: senza un contratto in essere non c'è una
     *  casa di cui segnalare i guasti. */
    noProperty: 'Non risulta nessun immobile a te collegato: non c’è nulla su cui aprire una segnalazione.',
    categories: {
      plumbing: 'Idraulica',
      electrical: 'Elettrico',
      heating_cooling: 'Riscaldamento e clima',
      appliance: 'Elettrodomestici',
      structural: 'Struttura',
      pest: 'Infestazioni',
      cleaning: 'Pulizie',
      connectivity: 'Rete e connettività',
      security: 'Sicurezza',
      other: 'Altro',
    },
    priorities: {
      low: 'Bassa',
      medium: 'Media',
      high: 'Alta',
      urgent: 'Urgente',
    },
    statuses: {
      open: 'Aperta',
      triaged: 'Classificata',
      assigned: 'Assegnata',
      in_progress: 'In lavorazione',
      waiting_tenant: 'In attesa dell’inquilino',
      waiting_owner: 'In attesa del proprietario',
      /** Nell'enum e mai scritto dall'API: compare qui perché una riga letta
       *  potrebbe portarlo, non perché qualcuno debba poterlo scegliere. */
      resolved: 'Risolta',
      closed: 'Chiusa',
      cancelled: 'Annullata',
    },
    actors: {
      tenant: 'Inquilino',
      owner: 'Proprietario',
      manager: 'Gestore',
      system: 'Sistema',
      ai: 'Assistente automatico',
    },
    /** Quando l'API non manda il nome perché `users_select` non concede quella
     *  riga a chi guarda. **Mai l'UUID**: un identificatore in faccia a chi legge
     *  non è informazione, è una resa — stessa regola di `expense.memberUnknown`. */
    personUnknown: 'Persona non più visibile',
  },

  bill: {
    title: 'Bollette',
    newTitle: 'Nuova bolletta',
    create: 'Registra una bolletta',
    utilityType: 'Utenza',
    supplierName: 'Fornitore',
    supplierVat: 'Partita IVA del fornitore',
    invoiceNumber: 'Numero fattura',
    issueDate: 'Data fattura',
    dueDate: 'Scadenza',
    periodStart: 'Periodo dal',
    periodEnd: 'Periodo al',
    totalAmount: 'Importo totale',
    taxableAmount: 'Imponibile',
    vatAmount: 'IVA',
    consumption: 'Consumo',
    consumptionValue: 'Valore',
    consumptionUnit: 'Unità',
    lease: 'Contratto a cui è imputata',
    leaseHint:
      'Senza contratto la bolletta resta visibile solo allo staff: è il campo che la rende leggibile a chi la paga.',
    property: 'Immobile',
    notes: 'Note',
    status: 'Stato',
    validate: 'Valida',
    validateConfirm: 'Validare questa bolletta?',
    validateExplain: 'Stai affermando che questi numeri sono quelli giusti.',
    validated: 'Bolletta validata.',
    validatedAt: 'Validata il',
    /** Dichiarare, non registrare un pagamento: il denaro in uscita non è
     *  modellato, e questa è la differenza che la frase deve far capire. */
    pay: 'Dichiara pagata',
    payConfirm: 'Dichiarare pagata questa bolletta?',
    payExplain:
      'Registra che l’hai pagata al fornitore, con la data di oggi. Non è un pagamento tracciato: è una dichiarazione, e non si disfa.',
    paid: 'Bolletta dichiarata pagata.',
    paidAt: 'Pagata il',
    void: 'Annulla',
    voidConfirm: 'Annullare questa bolletta?',
    voidConfirmMany: 'Annullare {count} bollette?',
    voidExplain: 'Sparisce dagli elenchi e il numero fattura torna reinseribile. La traccia resta.',
    voided: 'Bolletta annullata.',
    correctionUndoesValidation:
      'Correggere una bolletta validata ne toglie la validazione: chi la valida di nuovo lo fa apposta.',
    recharge: 'Riaddebita',
    rechargeTitle: 'Riaddebita a un contratto',
    rechargeExplain:
      'Crea un addebito verso l’inquilino. Si può ripetere su più contratti; nessuno verifica che le quote sommino al totale.',
    rechargeAmount: 'Importo da riaddebitare',
    rechargeDueDate: 'Scadenza dell’addebito',
    /** Il conto che si faceva a mente, e a mente si sbagliava: riaddebitare una
     *  bolletta d'immobile a tre contratti sono tre sottrazioni. */
    rechargeRemaining: 'Da riaddebitare',
    rechargeAlready: 'Già riaddebitato',
    /** Superare il totale **è ammesso** e resta ammesso: qui si avvisa, non si
     *  vieta. Il vincolo non esiste in database, e uno applicativo cadrebbe fra
     *  due riaddebiti concorrenti. */
    rechargeOverBudget: 'Questo importo supera quanto resta da riaddebitare.',
    /** Solo i contratti in corso ricevono un addebito: una bozza non è ancora un
     *  rapporto, un contratto annullato o disdetto non lo è più. */
    rechargeNoLeases: 'Nessun contratto in corso su questo immobile.',
    recharged: 'Riaddebito effettuato.',
    created: 'Bolletta registrata.',
    updated: 'Bolletta aggiornata.',
    duplicate: 'Questa fattura di questo fornitore è già registrata.',
    empty: 'Nessuna bolletta registrata.',
    emptyHint:
      'Una bolletta è una spesa dell’immobile da riaddebitare a chi abita. Registrala e allega la scansione: la lettura automatica parte da lì.',
    /** Chi abita non carica scansioni: `ocr.none` gli direbbe «quando ne carichi
     *  una». */
    scanEmpty: 'Nessuna scansione.',
    emptyForTenant:
      'Nessuna bolletta imputata al tuo contratto. Le registra chi amministra, e le vedi qui quando ti riguardano.',
    scan: 'Scansione',
    uploadScan: 'Carica la scansione',
    uploadHint: 'PDF o immagine, fino a 25 MB.',
    tooLarge: 'Il file supera i 25 MB.',
    uploaded: 'Scansione caricata.',
    attachments: 'Allegati',
    download: 'Scarica',
    types: {
      electricity: 'Elettricità',
      gas: 'Gas',
      water: 'Acqua',
      heating: 'Riscaldamento',
      waste: 'Rifiuti',
      internet: 'Internet',
      condo_fees: 'Spese condominiali',
      other: 'Altro',
    },
    statuses: {
      draft: 'Bozza',
      pending_review: 'Da rivedere',
      validated: 'Validata',
      allocated: 'Riaddebitata',
      paid: 'Pagata',
      disputed: 'Contestata',
      void: 'Annullata',
    },
  },

  ocr: {
    title: 'Lettura automatica',
    subtitle: 'Che cosa ha letto la macchina, accanto a ciò che hai scritto tu.',
    none: 'Nessuna scansione caricata: la lettura parte quando ne carichi una.',
    queued: 'In coda.',
    running: 'Lettura in corso…',
    succeeded: 'Lettura completata.',
    failed: 'La lettura non è riuscita.',
    cancelled: 'Lettura annullata.',
    confidence: 'Confidenza',
    /** La peggiore fra i campi, non la media: dieci campi a 0.99 non devono
     *  nascondere il totale letto a 0.40. */
    confidenceNote: 'È la peggiore fra quelle dei campi, non la media.',
    lowConfidence: 'Confidenza bassa: controlla i valori uno per uno.',
    proposed: 'Proposta della macchina',
    current: 'Valore attuale',
    accept: 'Usa questo valore',
    accepted: 'Valore accettato',
    identical: 'Uguale',
    apply: 'Applica i valori accettati',
    applied: 'Valori applicati alla bolletta.',
    nothingAccepted: 'Non hai ancora accettato nessun valore.',
    /** L'accettazione è campo per campo, e non per prudenza: è ciò che permette
     *  di misurare quanto la macchina è accurata. */
    whyFieldByField:
      'Si accetta un campo alla volta: ciò che ha proposto la macchina resta distinto da ciò che hai confermato tu, ed è quello che permetterà di misurare quanto legge bene.',
    refresh: 'Aggiorna',
    provider: 'Fornitore',
    model: 'Modello',
    error: {
      unsupported_document: 'Il documento non è in un formato leggibile.',
      provider_unavailable: 'Il fornitore di lettura non era disponibile.',
      document_archived: 'Il documento è stato archiviato.',
      document_unavailable: 'Il documento non è raggiungibile.',
      subject_not_processable: 'Questa bolletta non è in uno stato leggibile.',
    },
    /**
     * I nomi che l'API manda, che sono quelli delle **colonne della bolletta**:
     * chi rivede una proposta guarda quella colonna, ed è la ragione per cui
     * `BillField::as_str` li fa coincidere. Un nome nuovo qui manca e si vede —
     * compare grezzo — che è il comportamento voluto: un campo che il catalogo
     * non conosce deve essere rumoroso, non sparire.
     */
    fields: {
      supplier_name: 'Fornitore',
      supplier_vat: 'Partita IVA',
      invoice_number: 'Numero fattura',
      issue_date: 'Data fattura',
      due_date: 'Scadenza',
      period_start: 'Periodo dal',
      period_end: 'Periodo al',
      total_amount: 'Importo totale',
      taxable_amount: 'Imponibile',
      vat_amount: 'IVA',
      consumption: 'Consumo',
    },
  },

  charge: {
    title: 'Addebiti',
    newTitle: 'Nuovo addebito',
    create: 'Emetti un addebito',
    outstandingOnly: 'Solo insoluti',
    type: 'Tipo',
    description: 'Descrizione',
    amount: 'Importo',
    paidAmount: 'Incassato',
    outstandingAmount: 'Residuo',
    dueDate: 'Scadenza',
    periodStart: 'Periodo dal',
    periodEnd: 'Periodo al',
    status: 'Stato',
    lease: 'Contratto',
    void: 'Annulla',
    voidConfirm: 'Annullare questo addebito?',
    voidConfirmMany: 'Annullare {count} addebiti?',
    voidExplain:
      'Vuol dire «non doveva essere chiesto». Resta visibile a chi doveva pagarlo, e il residuo va a zero.',
    voided: 'Addebito annullato.',
    writeOff: 'Rinuncia all’incasso',
    writeOffConfirm: 'Rinunciare a incassare questo addebito?',
    writeOffExplain:
      'Vuol dire «era dovuto e ci rinunciamo»: è una perdita, ed è diverso dall’annullamento. Si può anche su un addebito già pagato in parte: quello che è stato incassato resta incassato, e si rinuncia al residuo.',
    /** `void` resta ammesso dal solo `issued`: su un addebito con del denaro
     *  dentro dire «non doveva essere chiesto» sarebbe falso. */
    voidNotWhenPartlyPaid:
      'Su questo addebito è già entrato del denaro: annullarlo direbbe che non doveva essere chiesto. La via è la rinuncia all’incasso.',
    writtenOff: 'Rinuncia registrata.',
    created: 'Addebito emesso.',
    empty: 'Nessun addebito.',
    /** È la prima schermata di ogni inquilino: il vuoto dice perché è qui e
     *  che cosa arriverà, e a chi amministra dice da dove si emette. */
    emptyHintStaff:
      'Un addebito è una somma chiesta a chi abita: il canone, una spesa, una bolletta riaddebitata. Si emette dalla scheda di un contratto.',
    emptyHintTenant:
      'Gli addebiti li emette chi amministra: il canone, le spese, le bollette riaddebitate. Quando ne arriva uno lo vedi qui, con la scadenza e quanto resta da pagare.',
    emptyOutstanding: 'Nessun insoluto: sono tutti pagati.',
    noPatch: 'Un addebito emesso non si modifica: si annulla e si riemette.',
    /** `?status=overdue` risponde 422: lo stato è derivato dalla scadenza. */
    derivedStatusNote: 'Scaduto è derivato dalla scadenza: si vede, e si filtra con «solo insoluti».',
    /** La pagina iniziale di chi abita: la domanda che si fa davvero. */
    tenantSubtitle: 'Quanto devi, e a che punto sta ogni richiesta.',
    fromBill: 'Da bolletta',
    detailTitle: 'Addebito',
    /** Un residuo negativo è un credito verso l'inquilino: non compensa gli altri
     *  insoluti, non compare nel riepilogo, e almeno qui va detto. */
    credit: 'Credito',
    creditExplain:
      'È stato incassato più di quanto era dovuto. Il credito resta su questo addebito: non abbassa l’insoluto degli altri e non compare nel riepilogo.',
    issuedAt: 'Emesso il',
    notes: 'Note',
    backToList: 'Torna agli addebiti',
    types: {
      rent: 'Canone',
      deposit: 'Deposito',
      utility_recharge: 'Riaddebito utenze',
      late_fee: 'Mora',
      maintenance: 'Manutenzione',
      adjustment: 'Conguaglio',
      other: 'Altro',
    },
    statuses: {
      draft: 'Bozza',
      issued: 'Emesso',
      partially_paid: 'Parzialmente pagato',
      paid: 'Pagato',
      overdue: 'Scaduto',
      void: 'Annullato',
      written_off: 'Stornato',
    },
  },

  payment: {
    title: 'Pagamenti',
    queueTitle: 'Dichiarazioni da verificare',
    queueSubtitle:
      'Chi dice di aver pagato, in attesa di una conferma. Finché sono qui non muovono nessun saldo.',
    chargePayments: 'Movimenti di questo addebito',
    record: 'Registra un incasso',
    recordTitle: 'Registra un incasso',
    recordExplain:
      'Un incasso che hai già visto sull’estratto conto. Nasce confermato e abbassa subito il residuo.',
    amount: 'Importo incassato',
    method: 'Strumento',
    paidAt: 'Data del movimento',
    /** Il campo mostra oggi e la richiesta tace finché non lo si cambia: così un
     *  pagamento delle 18 resta delle 18 invece di diventare delle 00:00. Il
     *  suggerimento dice a chi guarda che cosa succede se non tocca niente. */
    paidAtHint: 'Lasciata a oggi vale l’istante dell’invio. Cambiala se il movimento è di un altro giorno.',
    reference: 'Causale o riferimento',
    referenceHint: 'Quello che leggi sull’estratto conto: serve a ritrovarlo.',
    payer: 'A chi attribuirlo',
    /** `payer_user_id` omesso rende l'incasso invisibile a chi ha pagato: è
     *  lecito, ma va scelto e non subìto. */
    payerHint:
      'Chi non lo vede non lo trova nel proprio elenco, pur vedendone l’effetto sul residuo.',
    payerUnattributed: 'Non attribuito',
    payerUnattributedHint:
      'Scegli questo per un bonifico che non sai a chi ricondurre. Nessuno degli inquilini lo vedrà.',
    payerNoCandidates:
      'Nessun inquilino ha ancora accettato l’invito: finché non lo fa, l’incasso può solo restare non attribuito.',
    recorded: 'Incasso registrato.',
    declare: 'Ho pagato',
    declareTitle: 'Dichiara di aver pagato',
    /** Nasce `pending` e **non muove il saldo**: renderlo visibile senza renderlo
     *  vero è tutto il punto della rotta. */
    declareExplain:
      'Dichiari un pagamento che hai già fatto. Il residuo non cambia finché chi amministra non lo conferma.',
    declaredAmount: 'Quanto hai pagato',
    declaredAmountHint: 'Puoi dichiarare anche un acconto: il residuo scenderà di quello.',
    declaredReferenceHint: 'La causale del bonifico, o come si ritrova il movimento.',
    /** `payments_update` è staff soltanto: un ritiro otterrebbe zero righe
     *  aggiornate, cioè `auth.denied_by_policy`. Va detto prima, non scoperto
     *  cercando un bottone che non c'è. */
    declareNoWithdraw:
      'Una volta inviata non la puoi ritirare: se ti accorgi di un errore, scrivilo a chi amministra — può rifiutarla.',
    /** Il residuo non è ancora sceso, e dirlo dove si guarda il numero evita due
     *  persone convinte di due cose diverse sullo stesso importo. */
    declaredWaiting: 'Hai una dichiarazione in attesa di conferma su questo addebito.',
    status: 'Stato',
    /** Quando la riga è arrivata: `created_at`, che c'è **sempre**. È la data che
     *  ordina una coda di lavoro. */
    declaredAt: 'Ricevuto il',
    /** Quando il denaro si è mosso: `paid_at`, che su una dichiarazione **può
     *  mancare** — chi dichiara non è obbligato a dire quando — e che il rifiuto
     *  azzera. Sta accanto alla precedente e non al suo posto: una colonna sola
     *  sarebbe vuota proprio nel caso comune. */
    movedAt: 'Pagato il',
    charge: 'Addebito',
    confirm: 'Conferma',
    confirmTitle: 'Confermare questa dichiarazione?',
    confirmExplain:
      'Da qui il denaro conta: il residuo dell’addebito scende e il suo stato si ricalcola.',
    confirmed: 'Dichiarazione confermata.',
    reject: 'Rifiuta',
    rejectTitle: 'Rifiutare questa dichiarazione?',
    /** Un `pending` non aveva mosso il saldo: dire «saldo aggiornato» dopo un
     *  rifiuto sarebbe falso. */
    rejectExplain:
      'La dichiarazione passa ad annullata. Il saldo resta dov’è: una dichiarazione in attesa non lo aveva mosso.',
    rejected: 'Dichiarazione rifiutata.',
    /** Il vuoto della scheda di un addebito, e quello dell'elenco intero: la
     *  stessa stringa su `/pagamenti` diceva «su questo addebito» a chi ne
     *  guardava tutti. */
    empty: 'Nessun movimento su questo addebito.',
    emptyHintStaff: 'Quando incassi, registralo con «Registra un incasso»: il residuo si aggiorna da solo.',
    emptyHintTenant:
      'Se hai pagato, dichiaralo con «Ho pagato»: resta in attesa finché chi amministra lo conferma.',
    emptySettled: 'Questo addebito è chiuso e non accetta più movimenti.',
    listEmpty: 'Nessun movimento.',
    listEmptyHintStaff:
      'Un movimento è un incasso che registri tu, o un pagamento che un inquilino dichiara e tu confermi. Si registra dalla scheda di un addebito.',
    listEmptyHintTenant:
      'Quando paghi, dichiaralo dalla scheda dell’addebito: il movimento compare qui, in attesa finché chi amministra lo conferma.',
    goToCharges: 'Vai agli addebiti',
    queueEmpty: 'Nessuna dichiarazione in attesa.',
    /** È il vuoto di un filtro — la coda — e lo dice: gli altri movimenti stanno
     *  sotto «Tutti». */
    queueEmptyHint:
      'Quando un inquilino dichiara di aver pagato, lo trovi qui. Gli incassi e le dichiarazioni già decise stanno sotto «Tutti».',
    pendingDoesNotMove: 'In attesa: non muove il saldo finché non la confermi.',
    onlyStaffDecides:
      'A confermare o rifiutare è chi amministra: chi ha dichiarato non può ritirare.',
    /** La stessa rotta, letta da chi abita: le righe le sceglie `payments_select`,
     *  non un ramo sul ruolo. */
    mineSubtitle: 'I tuoi movimenti su questa organizzazione.',
    /** `payments_update` è staff soltanto: un ritiro otterrebbe zero righe
     *  aggiornate, cioè `auth.denied_by_policy`. Meglio dirlo che offrirlo. */
    mineCannotWithdraw:
      'Una dichiarazione in attesa la conferma o la rifiuta chi amministra: non si ritira.',
    goToCharge: 'Vai all’addebito',
    methods: {
      bank_transfer: 'Bonifico',
      sepa_direct_debit: 'Addebito SEPA',
      card: 'Carta',
      cash: 'Contanti',
      check: 'Assegno',
      other: 'Altro',
    },
    statuses: {
      pending: 'In attesa',
      authorized: 'Autorizzato',
      succeeded: 'Incassato',
      failed: 'Fallito',
      cancelled: 'Rifiutato',
      refunded: 'Rimborsato',
      partially_refunded: 'Rimborsato in parte',
    },
  },

  expense: {
    title: 'Spese condivise',
    subtitle: 'Il conto fra coinquilini: chi ha anticipato, chi deve, e a che punto sono i rimborsi.',

    /** Il netto complessivo in cima, che è **come si scopre di dovere qualcosa**.
     *  Non un contatore in navigazione: quello costerebbe una richiesta per gruppo
     *  a ogni caricamento del guscio, e non esiste una rotta che dia questo numero
     *  in una volta sola. */
    netTitle: 'Il tuo saldo complessivo',
    netCredit: 'Ti devono',
    netDebt: 'Devi',
    netEven: 'Sei in pari.',
    /** Sono due tasche diverse, con due rimedi diversi: uno si paga al
     *  proprietario e lo conferma lui, l'altro a un coinquilino e lo conferma lui.
     *  Affiancarli inviterebbe a sommarli. */
    netHint: 'Somma dei tuoi saldi in tutti i gruppi. Non c’entra con quello che devi al proprietario: sono due conti separati.',
    netLoading: 'Sto leggendo i saldi dei tuoi gruppi…',
    /** Un totale calcolato su una lettura incompleta è peggio di nessun totale:
     *  sembra vero. */
    netFailed: 'Non riesco a leggere i saldi di tutti i gruppi: un totale parziale direbbe il falso.',

    groupsEmpty: 'Non sei in nessun gruppo di spesa.',
    groupsEmptyHint: 'Un gruppo è il conto di una casa: chi anticipa una spesa, e come si divide fra chi ci abita.',
    create: 'Crea un gruppo',
    createTitle: 'Nuovo gruppo di spesa',
    createExplain: 'Ci sei già dentro tu. Gli altri si aggiungono dopo, scegliendoli fra i residenti della casa.',
    name: 'Nome del gruppo',
    nameHint: 'Come lo chiamate fra voi: «Casa di via Roma».',
    kind: 'Tipo',
    kinds: {
      coliving: 'Co-living',
      household: 'Casa',
      property: 'Immobile',
    },
    property: 'Immobile',
    propertyHint: 'Da qui arrivano i residenti che potrai aggiungere al gruppo.',
    /** `property_id` è facoltativo per l'API, e un gruppo creato senza immobile
     *  non ha un elenco di residenti da cui pescare. Il form lo chiede sempre; il
     *  ramo senza serve a non rompersi sui gruppi creati altrimenti. */
    propertyMissing: 'Questo gruppo non è legato a un immobile, quindi non c’è un elenco di residenti da cui scegliere.',
    propertiesEmpty: 'Non risulti in nessun immobile: senza una casa non c’è un conto da tenere.',
    defaultSplit: 'Come dividete di solito',
    defaultSplitHint: 'È la consuetudine del gruppo, non una regola: ogni spesa può dividersi in un altro modo.',
    /** Il menù ha **due** voci scritte a mano. `percentage`, `exact` e `by_usage`
     *  sono nell'enum e l'API li rifiuta con `422`. */
    splitMethods: {
      equal: 'In parti uguali',
      weighted: 'In proporzione al peso',
    },
    displayName: 'Come compari nel gruppo',
    /** È l'unico nome che questo modulo conservi: saldi, quote e rimborsi portano
     *  solo l'identificatore. Chi lascia la casa non è più fra i residenti, quindi
     *  dopo non c'è da nessuna parte da recuperarlo. */
    displayNameHint: 'Gli altri vedranno questo nome accanto ai tuoi saldi. È l’unico nome che il gruppo conserva.',
    weight: 'Il tuo peso',
    memberWeight: 'Il suo peso',
    /** `weight` sta in `NUMERIC(6,3)`: un quarto decimale dà `422` sul campo. */
    weightHint: 'Conta solo per la divisione in proporzione: chi sta in doppia scrive 2. Fino a tre decimali.',

    archived: 'Archiviato',
    archive: 'Archivia',
    archiveConfirm: 'Chiudere il conto di questo gruppo?',
    /** «Archivia» in questo progetto ha già due significati, e questo è il terzo:
     *  qui non nasconde niente, chiude le scritture. */
    archiveExplain:
      'Non accetterà più spese né rimborsi. Non è una cancellazione: saldi e spese restano leggibili a tutti. Si chiude anche con dei conti aperti, e quelli restano aperti.',
    archivedNote: 'Il conto è chiuso: niente più spese né rimborsi. I saldi restano leggibili.',
    /**
     * Il verbo della spesa: **Annulla**, che qui è il soft delete — la spesa non
     * ha uno stato. La conseguenza da dire è quella che chi annulla non pensa:
     * i regolamenti già confermati restano, e i saldi si spostano.
     */
    cancel: 'Annulla',
    cancelConfirm: 'Annullare questa spesa?',
    cancelConfirmMany: 'Annullare {count} spese?',
    cancelExplain:
      'Esce dai saldi del gruppo. I rimborsi già confermati restano: se questa spesa era già stata regolata, i saldi cambiano di conseguenza.',
    cancelled: 'Spesa annullata.',

    members: 'Chi c’è dentro',
    /** L'intestazione della colonna dei nomi, dove «Chi c'è dentro» sarebbe falsa:
     *  nei saldi compare anche chi è uscito. */
    person: 'Persona',
    membersCount: 'Membri attivi',
    memberLeft: 'Uscito',
    memberSince: 'Dentro dal',
    memberLeftOn: 'Uscito il',
    /** Quando `display_name` manca — un gruppo creato da `curl`, o da una versione
     *  che dimentica la regola — e la persona non è più fra i residenti. Mai
     *  l'UUID: un identificatore in faccia a chi legge non è informazione. */
    memberUnknown: 'Membro del gruppo',
    you: 'tu',
    membersEmpty: 'Nessuno nel gruppo.',
    membersEmptyHint: 'Chi crea il gruppo ne fa parte; gli altri residenti si aggiungono con «Aggiungi qualcuno».',
    addMember: 'Aggiungi qualcuno',
    addMemberTitle: 'Aggiungi al gruppo',
    addMemberExplain:
      'Scegli chi, fra i residenti della casa. Il nome che ha lì è quello che tutti vedranno accanto ai suoi saldi, anche fra un anno.',
    addMemberWho: 'Chi',
    residentNoAccount: 'non ha ancora accettato l’invito',
    residentAlreadyIn: 'è già nel gruppo',
    residentRejoins: 'era uscito: rientra',
    /** Un residente senza `user_id` si mostra **non selezionabile con la
     *  ragione**, non si nasconde: nasconderlo farebbe cercare una persona che
     *  sullo schermo c'è. */
    addMemberNoCandidates: 'Nessuno da aggiungere.',
    addMemberNoCandidatesHint:
      'Gli altri residenti sono già nel gruppo, o non hanno ancora accettato l’invito.',
    addMemberOutsider:
      'Si può aggiungere anche chi non abita più qui: chi se n’è andato lasciando un conto aperto va tenuto nel gruppo, non tolto.',

    leave: 'Esci dal gruppo',
    leaveConfirm: 'Uscire da questo gruppo?',
    /** `can_access_expense_group` esige `left_at IS NULL`: chi esce perde di vista
     *  anche i propri saldi, e rientrare richiede che lo riaggiunga qualcun altro.
     *  Va detto **prima**, non scoperto dopo. */
    leaveExplain:
      'Da quel momento non vedrai più il gruppo: né le spese, né i tuoi saldi. Quello che devi resta e continua a comparire agli altri. Per rientrare dovrà riaggiungerti uno di loro: da solo non puoi.',
    /** `403 expense_group.membership_not_yours`, e dalla `0029` lo dice anche la
     *  RLS. Il client offre l'uscita solo per sé. */
    leaveOnlyYourself: 'Si esce per sé stessi: togliere qualcun altro spetta a chi amministra.',

    balances: 'Saldi',
    balancesHint: 'Positivo vuol dire che ti devono, negativo che devi.',
    /** L'API dà i netti per persona e non una minimizzazione dei bonifici. Il
     *  client non la calcola nemmeno: due client con euristiche diverse
     *  proporrebbero giri diversi sugli stessi saldi. */
    whoPaysWhom: 'Chi paga chi lo decidete voi: qui ci sono i saldi, non un giro di bonifici da seguire.',
    balancesEmpty: 'Nessun saldo: non c’è ancora nessuna spesa.',
    netBalance: 'Saldo',
    totalPaid: 'Anticipato',
    totalOwed: 'Dovuto',
    settledOut: 'Versato',
    settledIn: 'Ricevuto',
    inCredit: 'in credito',
    inDebt: 'in debito',
    even: 'in pari',
    /** Chi è uscito con un conto aperto resta nei saldi (migration `0027`): senza
     *  di lui la somma smetterebbe di fare zero. Un nome che non è fra i membri
     *  attivi è la prima domanda che chi guarda si fa. */
    balanceLeft: 'uscito, conto ancora aperto',
    /** Verificata con `@/lib/decimal`: l'aritmetica nativa produrrebbe uno zero
     *  che a volte non è zero. */
    balancesSumZero: 'La somma dei saldi fa zero, come deve.',
    balancesSumBroken: 'La somma dei saldi non fa zero: questa lettura è incompleta.',

    expenses: 'Spese',
    expensesEmpty: 'Nessuna spesa in questo gruppo.',
    expensesEmptyHint:
      'Chi anticipa qualcosa per la casa la registra qui, con chi la divide: la prima spesa fa comparire i saldi.',
    record: 'Registra una spesa',
    recordTitle: 'Nuova spesa',
    recordExplain: 'Quello che qualcuno ha anticipato per la casa. Si divide subito fra i membri attivi.',
    description: 'Che cos’era',
    amount: 'Importo',
    category: 'Categoria',
    categories: {
      utilities: 'Utenze',
      groceries: 'Spesa',
      cleaning: 'Pulizie',
      connectivity: 'Internet e telefono',
      maintenance: 'Manutenzione',
      rent: 'Affitto',
      furniture: 'Arredamento',
      other: 'Altro',
    },
    paidBy: 'Chi ha anticipato',
    /** `409 expense.payer_not_in_group` a chi non è membro attivo: il menù si
     *  costruisce da chi ha `left_at` nullo. */
    paidByHint: 'Solo i membri attivi: chi è uscito dal gruppo non può risultare come chi ha anticipato.',
    paidByMe: 'Io',
    expenseDate: 'Quando',
    expenseDateHint: 'Vuoto significa oggi.',
    splitMethod: 'Come si divide',
    notes: 'Note',
    expenseDetail: 'Dettaglio della spesa',
    shares: 'Come si è divisa',
    share: 'Quota',
    shareWeight: 'Peso',
    /** Le quote le calcola il server e il client non le ricalcola: il trigger
     *  `expense_shares_balanced` non ammette che sommino a un altro numero, e il
     *  centesimo di resto va ai primi in ordine di identificatore. */
    sharesSum: 'Le quote sommano esattamente all’importo: le calcola il server, e il centesimo di resto va ai primi.',

    settlements: 'Rimborsi',
    settlementsEmpty: 'Nessun rimborso dichiarato.',
    settlementsEmptyHint: 'Quando qualcuno dice di aver pagato un altro, lo trovi qui.',
    declare: 'Ho rimborsato qualcuno',
    declareTitle: 'Dichiara un rimborso',
    /** Nasce `pending` e **non muove i saldi**: a muoverli è la conferma di chi
     *  riceve, che è l'unico a sapere se il denaro è arrivato. */
    declareExplain:
      'Un pagamento che hai già fatto a un coinquilino. I saldi non si muovono finché non lo conferma lui.',
    declareTo: 'A chi hai pagato',
    /** L'importo proposto è il minimo fra il proprio debito e il credito di quella
     *  persona: un suggerimento visibile nel campo, non un algoritmo nascosto. */
    declareToHint: 'Chi è in credito. L’importo proposto è il minore fra quello che devi e quello che gli devono.',
    declareAmount: 'Quanto hai pagato',
    declareNoCreditors: 'Nessuno è in credito in questo gruppo: non c’è nessuno da rimborsare.',
    /** L'asimmetria con le dichiarazioni di pagamento va detta al contrario: là il
     *  ritiro non c'è perché `payments_update` è staff soltanto, qui c'è perché
     *  `settlements_update` nomina anche chi ha dichiarato. */
    declareCanWithdraw:
      'Finché nessuno l’ha confermata puoi ritirarla — qui il ritiro c’è, a differenza delle dichiarazioni di pagamento al proprietario.',
    settledAt: 'Data del pagamento',
    /** `settled_at` resta **nullo** finché non si conferma, e allora ricade sul
     *  momento della conferma: dire «vuoto significa adesso» sarebbe falso, perché
     *  «adesso» è quando l'altro guarderà. Una data dichiarata invece resta. */
    settledAtHint:
      'Vuoto: varrà il momento in cui l’altro conferma. Se scrivi una data, la conferma non la riscrive.',
    /** Due colonne e non una, per la ragione già trovata sui pagamenti: finché è
     *  in attesa la data del movimento non c'è, e una colonna sola sarebbe vuota
     *  proprio nel caso comune. */
    declaredAt: 'Dichiarato il',
    settledAtColumn: 'Pagato il',
    note: 'Nota',
    noteHint: 'La causale del bonifico, o come si ritrova il movimento.',
    from: 'Da',
    to: 'A',
    settlementStatuses: {
      pending: 'In attesa',
      confirmed: 'Confermato',
      rejected: 'Rifiutato',
      cancelled: 'Ritirato',
    },
    pendingDoesNotMove: 'In attesa: non muove i saldi finché chi riceve non conferma.',
    waitingYou: 'Aspetta la tua conferma.',
    onlyReceiverDecides: 'A confermare o rifiutare è chi riceve; a ritirare chi ha dichiarato.',
    confirm: 'Conferma',
    confirmTitle: 'Confermare di aver ricevuto?',
    /** Confermare **non ricalcola niente**, a differenza della conferma di un
     *  pagamento: il saldo non è memorizzato da nessuna parte, si legge da una
     *  vista. */
    confirmExplain:
      'Da qui i saldi si muovono. Confermi tu perché sei l’unico a sapere se il denaro è arrivato davvero.',
    reject: 'Rifiuta',
    rejectTitle: 'Dire che non è arrivato?',
    rejectExplain:
      'La dichiarazione passa a rifiutata. I saldi restano dov’erano: una dichiarazione in attesa non li aveva mossi.',
    withdraw: 'Ritira',
    withdrawTitle: 'Ritirare la tua dichiarazione?',
    withdrawExplain:
      'Sparisce dalle cose da confermare. I saldi non cambiano, perché non li aveva mai mossi.',

    backToGroups: 'Torna alle spese',
  },

  errors: {
    /** Si instrada sul `code`, che è il contratto: `title` e `detail` sono per
     *  chi legge i log e possono essere riscritti in qualunque momento. */
    generic: 'Non è stato possibile completare l’operazione.',
    network: 'Non riesco a raggiungere il server. Controlla la connessione.',
    /** Risponde il proxy, non l'API: un corpo che non è JSON prende `http.<stato>`
     *  come codice (`apiErrorFrom`). È ciò che un ricaricamento durante un
     *  deploy incontra, e «non è stato possibile» non direbbe che è passeggero. */
    'http.502': 'Il server non risponde in questo momento. Riprova fra poco.',
    'http.503': 'Il server non è disponibile in questo momento. Riprova fra poco.',
    'http.504': 'Il server non ha risposto in tempo. Riprova fra poco.',
    'auth.token_missing': 'Devi entrare per vedere questa pagina.',
    'auth.token_expired': 'La sessione è scaduta.',
    'auth.token_invalid': 'La sessione non è più valida.',
    'auth.invalid_credentials': 'Email o password non corrette.',
    'auth.email_taken': 'Esiste già un account con questa email.',
    'auth.required': 'Devi entrare per fare questa operazione.',
    'auth.denied_by_policy': 'Operazione non consentita.',
    'org.not_selected': 'Seleziona un’organizzazione.',
    'org.not_a_member': 'Non fai parte di questa organizzazione.',
    'ticket.not_yours': 'Questa operazione spetta a chi ha aperto la segnalazione.',
    'ticket.not_found': 'Segnalazione non trovata.',
    'ticket.invalid_transition': 'Non si può fare da questo stato.',
    'ticket.cancelled': 'La segnalazione è annullata e non si modifica più.',
    /** Codice proprio e non `ticket.invalid_transition`: non è una transizione
     *  rifiutata — nessuno stava chiedendo di muovere la segnalazione — e il
     *  rimedio è diverso, cioè riaprire. */
    'ticket.thread_closed':
      'Questa segnalazione è chiusa: per riprendere la conversazione va riaperta.',
    'org.staff_role_required': 'Serve il ruolo di proprietario o gestore.',
    'validation.failed': 'Controlla i campi segnalati.',
    'property.not_found': 'Immobile non trovato.',
    'property.archived': 'L’immobile è archiviato e non è modificabile.',
    /** Dice **che cosa fare**, non solo che cosa non si può: il rifiuto arriva
     *  sull'attivazione di un contratto, e chi lo legge è a un campo di distanza
     *  dal rimedio. */
    'property.not_lettable':
      'L’immobile è in bozza: dichiaralo disponibile prima di attivare il contratto.',
    'room.not_found': 'Stanza non trovata.',
    'room.name_taken': 'Questo nome è già usato nell’immobile.',
    'room.archived': 'La stanza è archiviata e non è modificabile.',
    'lease.not_found': 'Contratto non trovato.',
    'lease.period_overlap': 'Il periodo si sovrappone a una locazione già attiva.',
    'property.derived_status_filter':
      'Affittato e parzialmente affittato si calcolano dai contratti: filtra per occupazione.',
    'lease.party_not_found': 'Questa parte non esiste più.',
    'lease.party_email_locked':
      'L’email non si cambia: questa persona ha già accettato l’invito con quell’indirizzo.',
    'lease.party_has_user':
      'Questa parte è collegata a un inquilino registrato e non si toglie: è una persona che abita, non una riga sbagliata.',
    'lease.invalid_transition': 'Non si può fare da questo stato.',
    'lease.no_tenant_party': 'Il contratto non ha inquilini fra le parti.',
    'lease.shares_not_hundred': 'Le quote degli inquilini non sommano a 100.',
    'lease.tenant_without_share':
      'Un inquilino ha la quota a zero: correggila prima di attivare, o toglilo dalle parti.',
    'lease.parties_locked':
      'Le parti si aggiungono finché il contratto è in bozza. Su un contratto in essere si correggono quelle che ci sono.',
    'lease.primary_party_exists': 'C’è già un intestatario principale.',
    'lease.derived_status_filter': 'Questo stato è derivato e non si può filtrare.',
    'invitation.not_found': 'Invito non trovato.',
    'invitation.already_pending': 'C’è già un invito in attesa per questa email.',
    'invitation.already_accepted': 'Questo invito è già stato accettato.',
    /** Non un errore da mostrare così com'è: il form di registrazione
     *  dall'invito lo intercetta e mostra il campo dell'organizzazione invece
     *  di questo messaggio. Il catalogo lo porta comunque, per non lasciare un
     *  codice muto se compare in un punto che non lo gestisce. */
    'invitation.owner_token': 'Questo codice è da proprietario: crea anche un’organizzazione.',
    'utility_bill.not_found': 'Bolletta non trovata.',
    'utility_bill.duplicate': 'Questa fattura di questo fornitore è già registrata.',
    'utility_bill.invalid_transition': 'Non si può fare da questo stato.',
    'charge.not_found': 'Addebito non trovato.',
    'charge.invalid_transition': 'Non si può fare da questo stato.',
    'charge.derived_status_filter': 'Scaduto è derivato: filtra con «solo insoluti».',
    'payment.not_found': 'Pagamento non trovato.',
    'payment.invalid_transition': 'Questa dichiarazione non è più in attesa.',
    'payment.idempotency_key_reused':
      'Questa operazione era già stata inviata per un altro addebito. Ricarica e riprova.',
    'request.idempotency_key_missing': 'Manca la chiave dell’operazione. Ricarica la pagina.',
    'expense_group.not_found': 'Gruppo di spesa non trovato.',
    'expense_group.archived': 'Il gruppo è archiviato: non accetta più spese né rimborsi.',
    /** Si esce per sé stessi, e dalla `0029` lo dice anche la Row Level
     *  Security: il codice è la versione parlante di un rifiuto che la policy
     *  darebbe comunque. */
    'expense_group.membership_not_yours': 'Da un gruppo si esce per sé stessi.',
    'expense_group.membership_exists': 'Questa persona è già nel gruppo.',
    'expense_group.membership_unchanged': 'Non c’è niente da cambiare: è già così.',
    'expense.not_found': 'Spesa non trovata.',
    'expense.not_yours': 'A annullare una spesa è chi l’ha registrata o chi l’ha anticipata.',
    'expense.payer_not_in_group': 'Chi ha anticipato non è un membro attivo del gruppo.',
    'settlement.not_found': 'Rimborso non trovato.',
    'settlement.invalid_transition': 'Questo rimborso non è più in attesa.',
    /** Confermare e rifiutare spettano a chi riceve, ritirare a chi ha
     *  dichiarato: due rifiuti diversi, due rimedi diversi. */
    'settlement.not_addressed_to_you': 'A confermare o rifiutare è chi doveva ricevere il denaro.',
    'settlement.not_declared_by_you': 'A ritirare una dichiarazione è chi l’ha fatta.',
    'membership.not_found': 'Non fai parte di questa organizzazione.',
    /** I tre rifiuti dell'uscita, ognuno con il proprio rimedio. Il primo arriva
     *  dalla policy e non da un `if`; il secondo è l'invariante «almeno un owner
     *  per org»; il terzo è osservabile perché la riga resta. */
    'membership.lease_in_force':
      'Hai un contratto in essere in questa organizzazione: non puoi uscirne finché è attivo.',
    'membership.last_owner':
      'Sei l’unico proprietario di questa organizzazione e non puoi uscirne: è la tua organizzazione personale.',
    'membership.already_left':
      'Eri già uscito da questa organizzazione. Per rientrare serve un nuovo invito da chi la amministra.',
    'document.not_found': 'Documento non trovato.',
    'request.payload_too_large': 'Il file è troppo grande.',
    'endpoint.not_found': 'Questa pagina non esiste.',
    'internal.error': 'Errore interno. Riprova, e se persiste segnala il riferimento.',
  },

  validation: {
    required: 'Questo campo è obbligatorio.',
    email: 'Indirizzo email non valido.',
    passwordTooShort: 'Almeno 12 caratteri.',
    amount: 'Indica un importo, per esempio 184,20.',
    positiveAmount: 'L’importo deve essere maggiore di zero.',
    date: 'Indica una data valida.',
    endBeforeStart: 'La fine deve venire dopo l’inizio.',
    percentage: 'Indica una percentuale fra 0 e 100.',
    integer: 'Indica un numero intero.',
  },
} as const

export type Catalog = typeof it
