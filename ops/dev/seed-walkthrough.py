#!/usr/bin/env python3
"""Semina il mondo del collaudo, passando dall'API vera.

**Perché dall'API e non con delle INSERT.** Le righe scritte a mano scavalcano
la Row Level Security, i trigger e gli invarianti del dominio: si otterrebbe un
database che *sembra* pieno e in cui il primo clic risponde `403` o `409` per
una ragione che non c'entra con ciò che si sta collaudando. Passando dalle
rotte, i dati sono validi per costruzione — e se il seeder fallisce, ha appena
trovato un difetto vero.

**Che cosa semina e che cosa no.** Semina ciò che nel collaudo è impalcatura:
un immobile con le stanze, due contratti attivi, due inquilini già dentro
l'organizzazione giusta, gli stati delle segnalazioni che servono a vedere la
sezione 5, e i numeri che servono a vedere che il residuo scende. **Non** semina
i punti 1-5 del giro, che sono precisamente ciò che quei punti provano: entrare
da un invito da proprietario e atterrare sul riepilogo vuoto, correggere una
stanza, revocare un invito e rifarlo, e — da ADR-053 e F7, in un solo passo
invece di due — registrarsi da un invito da inquilino ritrovandosi già dentro
l'organizzazione di chi ha invitato.

Uso:

    ./ops/dev/seed-walkthrough.py                 # http://localhost:8080
    PROPFLOW_API_URL=http://localhost:8081 ./ops/dev/seed-walkthrough.py

Vuole un database **appena migrato**: gira due volte, e la seconda si ferma
subito — prima di emettere niente — perché `OWNER` sa già entrare con
`PASSWORD`. È il modo giusto di dire «questo mondo esiste già»: un `POST
/auth/login`, non un invito emesso e mai marcato accettato, che al terzo giro
farebbe fallire lo script con un errore psql invece di un messaggio chiaro
(`owner_invitations_pending_uk`, migration `0040` — un solo invito pendente
per indirizzo).

**Vuole anche `PROPFLOW__DATABASE__MIGRATOR_URL` nell'ambiente**: da ADR-053 il
proprietario non si registra più da una rotta aperta, ma da un invito che
`ops/dev/issue-owner-invitation.sh` emette con il ruolo del migratore — la
stessa variabile che legge `propflow-migrator`. Non esiste una ricetta `just`
per questo script: prima di lanciarlo a mano serve caricare `.env` nella
shell, `set -a; source .env; set +a` (CLAUDE.md §4).
"""

import base64
import json
import os
import subprocess
import sys
import urllib.error
import urllib.request
import uuid
from datetime import date, timedelta
from pathlib import Path

BASE = os.environ.get("PROPFLOW_API_URL", "http://localhost:8080").rstrip("/")
API = f"{BASE}/api/v1"
ROOT = Path(__file__).resolve().parent.parent.parent
ISSUE_OWNER_INVITATION = ROOT / "ops" / "dev" / "issue-owner-invitation.sh"

# Una sola password per tutti: il collaudo non prova la robustezza delle
# credenziali, e ricordarne tre non aiuta nessuno. Dodici caratteri sono il
# minimo che `password_policy::validate` accetta, sulle due rotte da invito
# come ovunque nel prodotto.
PASSWORD = "collaudo-propflow-2026"

OWNER = "proprietario@collaudo.test"
ANNA = "anna@collaudo.test"
BRUNO = "bruno@collaudo.test"

# La finta bolletta che `DeterministicOcrProvider` sa leggere. **Non è un PDF e
# non è un JPEG**: quell'adattatore tratta il documento come testo e cerca righe
# `Etichetta: valore`. Una scansione vera verrebbe rifiutata con
# `UnsupportedDocument`, che è la verità — non sa leggerla — e chi non lo sa
# conclude che il prodotto è rotto.
FINTA_BOLLETTA = """Fornitore: Enel Energia
Partita IVA: 06655971007
Numero fattura: 2026-000123
Data emissione: 01/09/2026
Scadenza: 30/09/2026
Periodo dal: 01/07/2026
Periodo al: 31/08/2026
Imponibile: 96,00
IVA: 21,12
Totale: 117,12
Consumo: 412,5 kWh
"""

# Un PNG 1x1 valido: serve a provare che la foto di un guasto segue la
# segnalazione condivisa, non a mostrare un guasto.
PNG_1X1 = base64.b64decode(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=="
)


def giorno(offset: int) -> str:
    return (date.today() + timedelta(days=offset)).isoformat()


class ApiError(RuntimeError):
    pass


def chiama(method, path, token=None, org=None, body=None, idem=None, atteso=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(f"{API}{path}", data=data, method=method)
    if data is not None:
        req.add_header("content-type", "application/json")
    if token:
        req.add_header("authorization", f"Bearer {token}")
    # `X-Org-Id` **sempre**, anche quando coincide con quella del token: senza,
    # gli elenchi rispondono 200 con zero elementi, che è una schermata vuota
    # senza nessun indizio (CLAUDE.md, regola 24).
    if org:
        req.add_header("x-org-id", org)
    if idem:
        req.add_header("idempotency-key", idem)
    try:
        with urllib.request.urlopen(req) as risposta:
            grezzo = risposta.read()
            return risposta.status, json.loads(grezzo) if grezzo else None
    except urllib.error.HTTPError as errore:
        grezzo = errore.read()
        corpo = json.loads(grezzo) if grezzo else None
        if atteso and errore.code in atteso:
            return errore.code, corpo
        raise ApiError(f"{method} {path} → {errore.code}: {corpo}") from errore
    except urllib.error.URLError as errore:
        raise ApiError(
            f"API irraggiungibile su {BASE}: {errore.reason}.\n"
            "Serve `just api` in un terminale (e `just worker` per l'OCR)."
        ) from errore


def carica(path, token, org, filename, content_type, contenuto, kind):
    """Caricamento multipart, scritto a mano perché la stdlib non lo compone."""
    confine = "----propflowseedboundary"
    corpo = bytearray()
    corpo += f'--{confine}\r\nContent-Disposition: form-data; name="kind"\r\n\r\n{kind}\r\n'.encode()
    corpo += (
        f'--{confine}\r\nContent-Disposition: form-data; name="file"; '
        f'filename="{filename}"\r\nContent-Type: {content_type}\r\n\r\n'
    ).encode()
    corpo += contenuto
    corpo += f"\r\n--{confine}--\r\n".encode()

    req = urllib.request.Request(f"{API}{path}", data=bytes(corpo), method="POST")
    req.add_header("content-type", f"multipart/form-data; boundary={confine}")
    req.add_header("authorization", f"Bearer {token}")
    req.add_header("x-org-id", org)
    try:
        with urllib.request.urlopen(req) as risposta:
            return json.loads(risposta.read())
    except urllib.error.HTTPError as errore:
        raise ApiError(f"POST {path} → {errore.code}: {errore.read()}") from errore


def gia_seminato(email, corpo=None):
    corpo_riga = f"\ncorpo: {corpo}" if corpo is not None else ""
    raise ApiError(
        f"{email} esiste già: questo mondo è stato seminato una volta.\n"
        "Per rifarlo da zero serve un database pulito — `just down-hard`, "
        f"che porta via anche i volumi, MinIO compreso.{corpo_riga}"
    )


def mondo_esiste_gia():
    """Vero se `OWNER` sa già entrare con `PASSWORD`.

    **Va provato prima di emettere qualunque cosa**, non dopo. Il vecchio
    rilevamento accettava l'invito e leggeva il `409 auth.email_taken` che ne
    seguiva — ma per arrivarci doveva prima emetterlo, e su un mondo già
    seminato quell'invito resta pendente (l'accettazione fallisce prima di
    marcarlo). Al giro successivo — il terzo, sull'intero seeder —
    `owner_invitations_pending_uk` (migration `0040`) rifiuta un secondo
    invito pendente per lo stesso indirizzo: lo script fallisce con un errore
    psql invece del messaggio chiaro qui sotto. Il login non scrive niente,
    quindi non lascia questa traccia: o sa entrare, ed è la stessa prova che
    l'indirizzo esiste già, o non ci riesce, ed è la stessa prova del
    contrario.
    """
    status, _ = chiama(
        "POST",
        "/auth/login",
        body={"email": OWNER, "password": PASSWORD},
        atteso=(401,),
    )
    return status == 200


def emetti_invito_proprietario(email):
    """Chiama `ops/dev/issue-owner-invitation.sh`, come farebbe chi amministra
    la piattaforma (ADR-053): il seeder non scrive l'invito con un `INSERT`
    proprio perché il punto della decisione è che quella scrittura passa da
    uno script con il ruolo del migratore, non dal seeder stesso — replicarla
    qui a mano proverebbe un percorso diverso da quello vero."""
    if not os.environ.get("PROPFLOW__DATABASE__MIGRATOR_URL"):
        raise ApiError(
            "PROPFLOW__DATABASE__MIGRATOR_URL non impostata: serve a "
            "ops/dev/issue-owner-invitation.sh per scrivere l'invito con il "
            "ruolo del migratore (docs/07-operazioni.md).\n"
            "Non esiste una ricetta `just` per questo script: caricare "
            "`.env` nella shell prima di lanciarlo, `set -a; source .env; "
            "set +a` (CLAUDE.md §4)."
        )
    try:
        esito = subprocess.run(
            [str(ISSUE_OWNER_INVITATION), email],
            check=True,
            capture_output=True,
            text=True,
        )
    except subprocess.CalledProcessError as errore:
        raise ApiError(
            f"issue-owner-invitation.sh fallito per {email}:\n{errore.stderr.strip()}"
        ) from errore
    # Il token in chiaro è la sola riga su stdout: lo script manda ogni log a
    # stderr apposta, perché chi lo consegna possa incollare stdout senza
    # ripulirlo (vedi il commento dello script).
    return esito.stdout.strip()


def registra_proprietario(email, nome, nome_organizzazione):
    """Accetta un invito da proprietario (ADR-053): un solo passo, che crea
    utente e organizzazione e restituisce già una sessione come il login —
    non serve un `entra()` separato dopo, a differenza della vecchia
    `POST /auth/register`."""
    token = emetti_invito_proprietario(email)
    _, corpo = chiama(
        "POST",
        "/auth/accept-owner-invitation",
        body={
            "token": token,
            "full_name": nome,
            "password": PASSWORD,
            "organization_name": nome_organizzazione,
        },
        atteso=(409,),
    )
    if corpo and corpo.get("code") == "auth.email_taken":
        gia_seminato(email, corpo)
    return corpo["access_token"], corpo["organization_id"], corpo["user_id"]


def registra_inquilino_da_invito(token, nome, email):
    """Accetta un invito da inquilino (ADR-053): un solo passo — non più
    `POST /auth/register` seguito da `POST /auth/accept-invitation` — che
    registra e collega alla parte contrattuale nella stessa transazione,
    restituendo già una sessione **dentro l'organizzazione di chi ha
    invitato**."""
    _, corpo = chiama(
        "POST",
        "/auth/register-from-invitation",
        body={"token": token, "full_name": nome, "password": PASSWORD},
        atteso=(409,),
    )
    if corpo and corpo.get("code") == "auth.email_taken":
        gia_seminato(email, corpo)
    return corpo["access_token"], corpo["user_id"]


def passo(testo):
    print(f"  · {testo}", flush=True)


def main():
    print(f"Semino il mondo del collaudo su {BASE}\n")
    # `/health` sta fuori da `/api/v1`: si chiede prima di tutto, perché il modo
    # peggiore di scoprire che l'API non è su è a metà semina.
    try:
        with urllib.request.urlopen(f"{BASE}/health", timeout=5) as risposta:
            risposta.read()
    except OSError as errore:
        raise ApiError(
            f"API irraggiungibile su {BASE}: {errore}.\n"
            "Serve `just api` in un terminale (e `just worker` per l'OCR)."
        ) from errore

    # Prima di emettere qualunque cosa: se il mondo esiste già, il messaggio
    # chiaro va dato **prima** di scrivere un invito che poi resterebbe
    # pendente (vedi `mondo_esiste_gia`).
    if mondo_esiste_gia():
        gia_seminato(OWNER)

    # ---------------------------------------------------------------- proprietario
    print("Proprietario e casa")
    owner, org, owner_id = registra_proprietario(
        OWNER, "Giulia Ferrari", "Immobili di Giulia Ferrari"
    )
    passo(f"{OWNER} — organizzazione {org}")

    _, casa = chiama(
        "POST",
        "/properties",
        owner,
        org,
        {
            "label": "Via Verdi 12",
            "property_type": "apartment",
            "address": {"street": "Via Verdi 12", "city": "Milano", "province": "MI"},
        },
    )
    casa_id = casa["id"]
    # Fuori da `draft`: un immobile in bozza non è ciò che si guarda collaudando,
    # e lo stato memorizzato resta un giudizio di chi amministra (ADR-019).
    chiama("PATCH", f"/properties/{casa_id}", owner, org, {"status": "available"})
    passo(f"Via Verdi 12 — {casa_id}")

    stanze = {}
    for nome, canone in [("Camera Nord", "450.00"), ("Camera Sud", "420.00"), ("Sgombero", "0.00")]:
        _, stanza = chiama(
            "POST",
            f"/properties/{casa_id}/rooms",
            owner,
            org,
            {"name": nome, "monthly_rent": canone, "has_private_bathroom": nome == "Camera Nord"},
        )
        stanze[nome] = stanza["id"]
    # Una stanza dichiarata non disponibile **e libera**: è il giudizio che la
    # sezione 4 ha lasciato memorizzato, accanto ai due stati che invece deriva.
    chiama("PATCH", f"/rooms/{stanze['Sgombero']}", owner, org, {"status": "unavailable"})
    passo("tre stanze, di cui una in ristrutturazione")

    # ------------------------------------------------------------------ contratti
    print("\nContratti e inquilini")
    contratti = {}
    for nome_stanza, inquilino, email in [
        ("Camera Nord", "Anna Bianchi", ANNA),
        ("Camera Sud", "Bruno Conti", BRUNO),
    ]:
        _, contratto = chiama(
            "POST",
            "/leases",
            owner,
            org,
            {
                "property_id": casa_id,
                "room_id": stanze[nome_stanza],
                "start_date": giorno(-60),
                "end_date": giorno(305),
                "rent_amount": "450.00" if nome_stanza == "Camera Nord" else "420.00",
            },
        )
        contratto_id = contratto["id"]
        chiama(
            "POST",
            f"/leases/{contratto_id}/parties",
            owner,
            org,
            {"full_name": inquilino, "email": email, "is_primary": True, "share_percent": "100"},
        )
        chiama("POST", f"/leases/{contratto_id}/activate", owner, org, {})
        contratti[nome_stanza] = contratto_id
        passo(f"{nome_stanza}: contratto attivo per {inquilino}")

    # La registrazione dall'invito è il passo che porta l'inquilino direttamente
    # **nell'organizzazione del proprietario** (ADR-053): un solo passo, non più
    # «registrati, poi accetta» — `register-from-invitation` fa le due cose in
    # una transazione. Nel giro a mano è il punto 5 e va percorso lì; qui serve
    # solo ad avere due inquilini veri.
    sessioni = {}
    for nome, email, nome_stanza in [("Anna Bianchi", ANNA, "Camera Nord"), ("Bruno Conti", BRUNO, "Camera Sud")]:
        _, invito = chiama(
            "POST",
            f"/leases/{contratti[nome_stanza]}/invitations",
            owner,
            org,
            {"email": email, "full_name": nome},
        )
        token, utente = registra_inquilino_da_invito(invito["accept_token"], nome, email)
        sessioni[email] = (token, utente)
        passo(f"{email} dentro come tenant in {org}")
        # Nessuna organizzazione personale da cui ripartire al login successivo:
        # chi si registra da un invito da inquilino non ne ha mai avuta una
        # (ADR-053, decisione 1). Da qui in poi il seeder passa esplicitamente
        # l'organizzazione del proprietario, che è ciò che il client fa dopo
        # `selectOrganization`.

    anna, _anna_id = sessioni[ANNA]
    bruno, bruno_id = sessioni[BRUNO]

    # ------------------------------------------- il vicolo cieco che la sezione 3 apre
    print("\nUn contratto sbagliato, da correggere a mano")
    _, altra_casa = chiama(
        "POST",
        "/properties",
        owner,
        org,
        {
            "label": "Corso Italia 4",
            "property_type": "apartment",
            "address": {"street": "Corso Italia 4", "city": "Milano", "province": "MI"},
        },
    )
    _, bozza = chiama(
        "POST",
        "/leases",
        owner,
        org,
        {"property_id": altra_casa["id"], "start_date": giorno(15), "end_date": giorno(380), "rent_amount": "900.00"},
    )
    # Due parti al 100 e allo 0: è **esattamente** lo stato in cui il prodotto
    # sapeva mettersi e da cui non sapeva uscire, prima della sezione 3.
    # L'attivazione lo rifiuta, e ora esiste il modo di correggerlo.
    chiama(
        "POST",
        f"/leases/{bozza['id']}/parties",
        owner,
        org,
        {"full_name": "Carla Neri", "is_primary": True, "share_percent": "100"},
    )
    chiama(
        "POST",
        f"/leases/{bozza['id']}/parties",
        owner,
        org,
        {"full_name": "Dario Villa", "share_percent": "0"},
    )
    passo(f"bozza su Corso Italia 4 con le quote 100/0 — {bozza['id']}")

    # ------------------------------------------------------------------- bollette
    print("\nBollette")
    _, da_finire = chiama(
        "POST",
        f"/properties/{casa_id}/utility-bills",
        owner,
        org,
        {
            "utility_type": "electricity",
            "supplier_name": "Enel Energia",
            "supplier_vat": "IT06655971007",
            "invoice_number": "2026-000123",
            "period_start": giorno(-70),
            "period_end": giorno(-10),
            "total_amount": "117.12",
            "lease_id": contratti["Camera Nord"],
        },
    )
    # La scansione su una **bozza** accoda la lettura automatica: senza il worker
    # non arriva mai in revisione, e il sintomo è muto — non va in errore niente,
    # semplicemente non succede nulla.
    carica(
        f"/utility-bills/{da_finire['id']}/documents",
        owner,
        org,
        "bolletta-luglio-agosto.txt",
        "text/plain",
        FINTA_BOLLETTA.encode(),
        "utility_bill",
    )
    passo(f"bozza con la scansione già caricata, da rivedere e validare — {da_finire['id']}")

    _, riaddebitata = chiama(
        "POST",
        f"/properties/{casa_id}/utility-bills",
        owner,
        org,
        {
            "utility_type": "gas",
            "supplier_name": "Italgas",
            "supplier_vat": "IT00489490011",
            "invoice_number": "2026-000042",
            "period_start": giorno(-130),
            "period_end": giorno(-71),
            "total_amount": "184.20",
            "lease_id": contratti["Camera Sud"],
        },
    )
    chiama("POST", f"/utility-bills/{riaddebitata['id']}/validate", owner, org, {})
    chiama(
        "POST",
        f"/utility-bills/{riaddebitata['id']}/recharge",
        owner,
        org,
        {
            "lease_id": contratti["Camera Sud"],
            "amount": "92.10",
            "due_date": giorno(8),
            "description": "Gas, bimestre",
        },
    )
    passo(f"una già validata e riaddebitata, da dichiarare pagata — {riaddebitata['id']}")

    # -------------------------------------------------------------------- addebiti
    print("\nAddebiti")
    _, canone_anna = chiama(
        "POST",
        f"/leases/{contratti['Camera Nord']}/charges",
        owner,
        org,
        {"charge_type": "rent", "amount": "450.00", "due_date": giorno(-5), "description": "Canone del mese"},
    )
    passo(f"450,00 a carico di Anna, scaduto — {canone_anna['id']}")

    _, canone_bruno = chiama(
        "POST",
        f"/leases/{contratti['Camera Sud']}/charges",
        owner,
        org,
        {"charge_type": "rent", "amount": "420.00", "due_date": giorno(-35), "description": "Canone del mese scorso"},
    )
    # **`payer_user_id` va valorizzato**, e non è anagrafica: `payments_select`
    # mostra la riga a chi ha pagato, e senza questo campo l'incasso resta
    # visibile al solo staff. Peggio, `v_charge_balances` è `security_invoker`,
    # quindi somma i pagamenti sotto la Row Level Security di chi legge: a Bruno
    # il residuo arriverebbe **420,00 su un addebito `partially_paid`** — una riga
    # che si contraddice, e il numero sbagliato è quello che costa denaro. È il
    # difetto `R29` in `docs/12-da-fare.md`, misurato seminando questo mondo.
    chiama(
        "POST",
        f"/charges/{canone_bruno['id']}/payments",
        owner,
        org,
        {
            "amount": "150.00",
            "method": "bank_transfer",
            "reference": "Acconto",
            "payer_user_id": bruno_id,
        },
        idem=str(uuid.uuid4()),
    )
    passo("420,00 a carico di Bruno, di cui 150,00 incassati: il residuo è 270,00")

    # ------------------------------------------------------------- spese co-living
    print("\nSpese condivise")
    _, gruppo = chiama(
        "POST",
        "/expense-groups",
        anna,
        org,
        {"name": "Via Verdi 12", "property_id": casa_id},
    )
    # Il nome si scrive **all'ingresso**, perché dopo non esiste più: la riga
    # della membership non sparisce mai, e quel nome resta leggibile anche a chi
    # se ne va (ADR-043).
    chiama(
        "POST",
        f"/expense-groups/{gruppo['id']}/members",
        anna,
        org,
        {"user_id": bruno_id, "display_name": "Bruno Conti"},
    )
    passo(f"gruppo con Anna e Bruno dentro, senza spese — {gruppo['id']}")

    # ------------------------------------------------------------- le segnalazioni
    print("\nSegnalazioni, negli stati che servono a vedere la sezione 5")

    def apre(titolo, descrizione, condivisa, con_contratto=True):
        corpo = {
            "title": titolo,
            "description": descrizione,
            "visible_to_residents": condivisa,
        }
        if con_contratto:
            corpo["lease_id"] = contratti["Camera Nord"]
        _, segnalazione = chiama("POST", f"/properties/{casa_id}/tickets", anna, org, corpo)
        return segnalazione

    privata = apre(
        "Il rubinetto del bagno gocciola",
        "Gocciola dalla notte, il pavimento è bagnato la mattina.",
        condivisa=False,
    )
    passo(f"privata, aperta da Anna — Bruno non deve vederla ({privata['reference']})")

    # Senza contratto e **condivisa**: è la forma che prova il quinto ramo di
    # `app.can_access_ticket` e nessun altro, perché il ramo del contratto è
    # fuori gioco per costruzione.
    condivisa = apre(
        "La caldaia condominiale è ferma",
        "Da stamattina niente acqua calda in tutto l'appartamento.",
        condivisa=True,
        con_contratto=False,
    )
    carica(
        f"/tickets/{condivisa['id']}/documents",
        anna,
        org,
        "caldaia.png",
        "image/png",
        PNG_1X1,
        "photo",
    )
    chiama(
        "POST",
        f"/tickets/{condivisa['id']}/messages",
        anna,
        org,
        {"body": "Ho provato a riavviarla dal pannello, non riparte."},
    )
    passo(f"condivisa, con foto e un messaggio — Bruno deve vederla tutta ({condivisa['reference']})")

    in_carico = apre(
        "La serratura della porta d'ingresso è dura",
        "Bisogna forzare la chiave per chiudere.",
        condivisa=False,
    )
    chiama("POST", f"/tickets/{in_carico['id']}/assign", owner, org, {"assignee_id": owner_id})
    passo(f"presa in carico — «In carico a» deve dire *Giulia Ferrari* ({in_carico['reference']})")

    chiusa = apre(
        "Il termosifone della camera non scalda",
        "Resta freddo anche con la valvola aperta.",
        condivisa=False,
    )
    chiama("POST", f"/tickets/{chiusa['id']}/close", owner, org, {"resolution_notes": "Spurgato l'impianto."})
    passo(f"chiusa — il thread si legge e non si scrive ({chiusa['reference']})")

    annullata = apre(
        "Mi sembra di sentire un odore di gas",
        "Poi ho capito che veniva dalle scale.",
        condivisa=False,
    )
    chiama("POST", f"/tickets/{annullata['id']}/cancel", anna, org, {})
    passo(f"annullata — non si offre più niente ({annullata['reference']})")

    # ------------------------------------------------------------------ riepilogo
    print("\n" + "=" * 68)
    print("Mondo seminato. Le credenziali, una password per tutti:\n")
    print(f"  password   {PASSWORD}\n")
    print(f"  proprietario  {OWNER}   Giulia Ferrari")
    print(f"  inquilina     {ANNA}          Anna Bianchi   — Camera Nord")
    print(f"  inquilino     {BRUNO}         Bruno Conti    — Camera Sud")
    print("\nSe l'OCR resta «in coda», manca `just worker`: senza, la bolletta")
    print("non arriva mai in revisione e non va in errore niente.")


if __name__ == "__main__":
    try:
        main()
    except ApiError as errore:
        print(f"\nInterrotto: {errore}", file=sys.stderr)
        sys.exit(1)
