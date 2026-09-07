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
export const it = {
  app: {
    name: 'PropFlow',
    skipToContent: 'Vai al contenuto',
  },

  common: {
    save: 'Salva',
    cancel: 'Annulla',
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
  },

  state: {
    loadingTitle: 'Caricamento',
    errorTitle: 'Qualcosa non ha funzionato',
    errorRequestId: 'Riferimento della richiesta',
    emptyTitle: 'Non c’è ancora niente',
  },

  auth: {
    loginTitle: 'Entra in PropFlow',
    loginSubtitle: 'Amministra i tuoi immobili, i contratti e la contabilità.',
    registerTitle: 'Crea il tuo account',
    registerSubtitle: 'Bastano un’email e una password per cominciare.',
    email: 'Email',
    password: 'Password',
    passwordHint: 'Almeno 12 caratteri. Una frase lunga vale più di un simbolo.',
    fullName: 'Nome e cognome',
    organizationName: 'Nome dell’organizzazione',
    organizationNameHint: 'Se lo lasci vuoto usiamo il tuo nome.',
    submitLogin: 'Entra',
    submitRegister: 'Crea l’account',
    toRegister: 'Non hai un account? Registrati',
    toLogin: 'Hai già un account? Entra',
    logout: 'Esci',
    sessionExpired: 'La sessione è scaduta. Entra di nuovo.',
    registeredNowSignIn: 'Account creato. Entra con le tue credenziali.',
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
  },

  nav: {
    dashboard: 'Riepilogo',
    properties: 'Immobili',
    leases: 'Contratti',
    /** Al singolare, e solo per l'inquilino: per chi abita il contratto è uno. */
    lease: 'Contratto',
    tickets: 'Segnalazioni',
    bills: 'Bollette',
    charges: 'Addebiti',
    payments: 'Pagamenti',
    main: 'Navigazione principale',
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
    empty: 'Comincia registrando il tuo primo immobile.',
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
    archiveExplain:
      'Non viene cancellato: sparisce dagli elenchi e non è più modificabile. La traccia resta.',
    archived: 'Immobile archiviato.',
    created: 'Immobile registrato.',
    updated: 'Immobile aggiornato.',
    empty: 'Nessun immobile registrato.',
    emptyHint: 'Un immobile è il punto di partenza: da lì nascono stanze e contratti.',
    residents: 'Chi ci abita',
    residentsEmpty: 'Nessuno ci abita in questo momento.',
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
  },

  room: {
    title: 'Stanze',
    create: 'Aggiungi una stanza',
    newTitle: 'Nuova stanza',
    name: 'Nome',
    nameHint: 'Unico nell’immobile: «Camera doppia», «Singola sul cortile».',
    monthlyRent: 'Canone mensile',
    surface: 'Superficie',
    privateBathroom: 'Bagno privato',
    furnished: 'Arredata',
    status: 'Stato',
    archive: 'Archivia la stanza',
    archiveConfirm: 'Archiviare questa stanza?',
    archiveExplain: 'Il nome torna libero per una stanza nuova. La traccia resta.',
    created: 'Stanza aggiunta.',
    updated: 'Stanza aggiornata.',
    archived: 'Stanza archiviata.',
    empty: 'Nessuna stanza: il contratto riguarderà l’immobile intero.',
    nameTaken: 'Questo nome è già usato da un’altra stanza dell’immobile.',
    statuses: {
      available: 'Disponibile',
      occupied: 'Occupata',
      unavailable: 'Non disponibile',
    },
  },

  lease: {
    title: 'Contratti',
    newTitle: 'Nuovo contratto',
    create: 'Redigi un contratto',
    property: 'Immobile',
    room: 'Stanza',
    wholeProperty: 'Immobile intero',
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
    activate: 'Attiva',
    activateConfirm: 'Attivare questo contratto?',
    activateExplain:
      'Il periodo viene occupato e il contratto diventa in essere. Serve almeno un inquilino, e le quote devono sommare a 100.',
    activated: 'Contratto attivato.',
    terminate: 'Disdici',
    terminateConfirm: 'Disdire questo contratto?',
    terminateExplain: 'Indica la data di fine rapporto. Il periodo si libera.',
    terminationDate: 'Data di fine rapporto',
    terminated: 'Contratto disdetto.',
    created: 'Contratto redatto.',
    empty: 'Nessun contratto.',
    emptyHint: 'Un contratto nasce in bozza: si aggiungono le parti e poi si attiva.',
    /** Il vuoto di chi abita non è lo stesso di chi amministra: lui non ha un
     *  gesto per riempirlo, e dirgli «creane uno» sarebbe indicargli una porta
     *  che l'API gli chiude. */
    emptyForTenant: 'Non risulta nessun contratto a tuo nome in questa organizzazione.',
    noTenantParty: 'Il contratto non ha inquilini fra le parti.',
    sharesNotHundred: 'Le quote degli inquilini non sommano a 100.',
    periodOverlap: 'Il periodo si sovrappone a una locazione già attiva.',
    /** `expiring` ed `expired` sono derivati: si mostrano e non si filtrano. */
    derivedStatusNote:
      'In scadenza e scaduto sono stati derivati dalla data: si vedono, non si filtrano.',
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

  invitation: {
    title: 'Inviti',
    invite: 'Invita un inquilino',
    inviteTitle: 'Invita un inquilino',
    email: 'Email dell’invitato',
    fullName: 'Nome e cognome',
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
    alreadyPending: 'C’è già un invito in attesa per questa email.',
    pending: 'In attesa',
    accepted: 'Accettato',
    expired: 'Scaduto',
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
    /** La prima riga la scrive il trigger all'inserimento, fuori da un contesto
     *  di richiesta: `changed_by` può quindi mancare, e non è un difetto. */
    historyOpened: 'Aperta',
    thread: 'Conversazione',
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
    cancel: 'Annulla la segnalazione',
    cancelTitle: 'Annullare questa segnalazione?',
    cancelExplain:
      'Vuol dire «non serve più». Resta visibile con la sua storia, e non si modifica più.',
    /** L'API risponde `ticket.not_yours` e non `org.staff_role_required`: non
     *  manca un ruolo, manca l'essere stati noi a segnalare. Il messaggio del
     *  primo caso sta fra gli `errors`, dove si instrada sul codice; questo
     *  spiega **prima** il caso in cui il bottone non compare affatto. */
    onlyOpenCancelledByReporter:
      'Puoi annullarla finché nessuno ci ha messo mano. Da qui in poi la annulla chi amministra.',
    onlyOpen: 'Solo quelle aperte',
    /** `?open=true` non è `status <> 'closed'`: l'indice parziale
     *  `tickets_open_ix` è definito su esattamente sei valori. */
    onlyOpenHint: 'Le sei fasi di lavorazione, chiuse e annullate escluse.',
    empty: 'Nessuna segnalazione.',
    emptyOpen: 'Nessuna segnalazione aperta.',
    emptyHintTenant: 'Se qualcosa non funziona in casa, segnalalo: resta scritto, con la sua data.',
    emptyHintStaff: 'Quando un inquilino segnala un guasto, lo trovi qui.',
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
    void: 'Annulla',
    voidConfirm: 'Annullare questa bolletta?',
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
    recharged: 'Riaddebito effettuato.',
    created: 'Bolletta registrata.',
    updated: 'Bolletta aggiornata.',
    duplicate: 'Questa fattura di questo fornitore è già registrata.',
    empty: 'Nessuna bolletta registrata.',
    emptyHint: 'Registra una bolletta e allega la scansione: la lettura automatica parte da lì.',
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
    paidAtHint: 'Vuoto significa adesso.',
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
    empty: 'Nessun movimento su questo addebito.',
    queueEmpty: 'Nessuna dichiarazione in attesa.',
    queueEmptyHint: 'Quando un inquilino dichiara di aver pagato, lo trovi qui.',
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

  errors: {
    /** Si instrada sul `code`, che è il contratto: `title` e `detail` sono per
     *  chi legge i log e possono essere riscritti in qualunque momento. */
    generic: 'Non è stato possibile completare l’operazione.',
    network: 'Non riesco a raggiungere il server. Controlla la connessione.',
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
    'org.staff_role_required': 'Serve il ruolo di proprietario o gestore.',
    'validation.failed': 'Controlla i campi segnalati.',
    'property.not_found': 'Immobile non trovato.',
    'property.archived': 'L’immobile è archiviato e non è modificabile.',
    'room.not_found': 'Stanza non trovata.',
    'room.name_taken': 'Questo nome è già usato nell’immobile.',
    'room.archived': 'La stanza è archiviata e non è modificabile.',
    'lease.not_found': 'Contratto non trovato.',
    'lease.period_overlap': 'Il periodo si sovrappone a una locazione già attiva.',
    'lease.invalid_transition': 'Non si può fare da questo stato.',
    'lease.no_tenant_party': 'Il contratto non ha inquilini fra le parti.',
    'lease.shares_not_hundred': 'Le quote degli inquilini non sommano a 100.',
    'lease.primary_party_exists': 'C’è già un intestatario principale.',
    'lease.derived_status_filter': 'Questo stato è derivato e non si può filtrare.',
    'invitation.not_found': 'Invito non trovato.',
    'invitation.already_pending': 'C’è già un invito in attesa per questa email.',
    'invitation.already_accepted': 'Questo invito è già stato accettato.',
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
