#!/usr/bin/env python3
"""Writes docs/demo/rudolph.json: two invented people sharing a household, in the format
`moneta export` writes and `moneta import` reads.

Rudolph and Krösus share a flat and a child in Kita. Rudolph pays the rent, Krösus the Kita
and the electricity, and whoever is at the till pays for the groceries; each split goes half
onto their debt accounts, which mirror each other, and Krösus settles up at the end of every
month. Around that: salaries, a Commerzbank Girokonto with its Kreditkarte, a Tagesgeld, an ETF
depot read once a year, cash, insurances, subscriptions, and the odd purchase that is not
part of ordinary spending.

Generated rather than written by hand: two years of a household are several hundred expenses,
and every recorded balance has to agree with them to the cent. Stdlib only and seeded, so the
same arguments give the same file byte for byte. Rerun it when the export format changes, or
with a later --until to bring the demo up to date before taking screenshots:

    python3 docs/demo/generate.py --until 2027-03-01

Everything here is invented. Nothing is taken from anybody's real books.
"""

import argparse
import calendar
import json
import random
from datetime import date, datetime, timedelta, timezone
from pathlib import Path

START = date(2024, 12, 1)
SEED = 2030


def stamp(day, hour=12, minute=0):
    """Midday UTC unless said otherwise: the same calendar day in Berlin, summer or winter."""
    return datetime(day.year, day.month, day.day, hour, minute, tzinfo=timezone.utc)


def iso(moment):
    return moment.strftime("%Y-%m-%dT%H:%M:%SZ")


def last_day(year, month):
    return date(year, month, calendar.monthrange(year, month)[1])


def months(first, last):
    year, month = first.year, first.month
    while (year, month) <= (last.year, last.month):
        yield year, month
        year, month = (year + 1, 1) if month == 12 else (year, month + 1)


def workday_before(day):
    """Salaries and direct debits move to the Friday before a weekend."""
    while day.weekday() >= 5:
        day -= timedelta(days=1)
    return day


def jsonb_order(value):
    """Key order as postgres keeps it in jsonb (shorter keys first), so that the changelog's
    snapshots come back from an import exactly as they are written here."""
    if isinstance(value, dict):
        return {k: jsonb_order(value[k]) for k in sorted(value, key=lambda k: (len(k), k))}
    if isinstance(value, list):
        return [jsonb_order(v) for v in value]
    return value


class Household:
    def __init__(self, until, rng):
        # nothing is created later than this on the last day, so the demo has no future in it
        # whenever on that day it is looked at
        self.now = stamp(until, 7)
        self.rng = rng
        self.users, self.categories, self.accounts = [], [], []
        self.expenses, self.balances, self.periodic, self.automation = [], [], [], []
        self.deliveries, self.changes = [], []
        # account id -> [(moment, cents)], what the recorded balances are computed from
        self.ledger = {}
        self.opening = {}

    # --- ids -------------------------------------------------------------------------------

    def uuid(self, moment):
        """A UUIDv7 for that instant, as the database would have handed out."""
        millis = int(moment.timestamp() * 1000)
        value = (millis << 80) | (0x7 << 76) | (self.rng.getrandbits(12) << 64)
        value |= (0b10 << 62) | self.rng.getrandbits(62)
        text = f"{value:032x}"
        return f"{text[:8]}-{text[8:12]}-{text[12:16]}-{text[16:20]}-{text[20:]}"

    # --- the fixed parts -------------------------------------------------------------------

    def user(self, name, full_name, seen_at, seen_before):
        user = {
            "id": self.uuid(stamp(date(2024, 11, 20), 9)),
            "name": name,
            "fullName": full_name,
            "seenAt": iso(seen_at),
            "seenBefore": iso(seen_before),
        }
        self.users.append(user)
        return user["id"]

    def category(self, name, color, parent=None, description="", limit=None):
        category = {
            "id": self.uuid(stamp(date(2024, 11, 20), 10)),
            "name": name,
            "description": description,
            "color": color,
            "parentId": parent,
            "monthlyLimit": limit,
        }
        self.categories.append(category)
        return category["id"]

    def account(self, user, name, kind, availability, risk, opened, color, opening=0, **extra):
        account = {
            "id": self.uuid(stamp(date(2024, 11, 20), 11)),
            "userId": user,
            "name": name,
            "description": extra.get("description", ""),
            "color": color,
            "iban": extra.get("iban"),
            "kind": kind,
            "availability": availability,
            "risk": risk,
            "opened": iso(stamp(opened, 0)),
            "closed": None,
            "ownershipFraction": 1.0,
            "syncAccountId": None,
            "syncInvert": None,
            "renditierAccountId": None,
        }
        self.accounts.append(account)
        self.ledger[account["id"]] = []
        self.opening[account["id"]] = opening
        return account["id"]

    def synchronize(self, first, second):
        """A debt between two people: each sees the other's entry, sign flipped."""
        by_id = {a["id"]: a for a in self.accounts}
        by_id[first]["syncAccountId"], by_id[first]["syncInvert"] = second, True
        by_id[second]["syncAccountId"], by_id[second]["syncInvert"] = first, True

    # --- expenses ----------------------------------------------------------------------------

    def expense(
        self,
        title,
        day,
        legs,
        categories=(),
        store="",
        accrual=None,
        reference=None,
        created=None,
        template=False,
        unchecked=False,
        tax=False,
        incomplete=False,
        extraordinary=False,
        positions=(),
        description="",
        comments="",
    ):
        """`legs` are dicts with `account` and either `amount` or `fraction`, optionally
        `statement`, `booked` (the bank's posting date), `comments` and `day`."""
        start, end = accrual or (day, day)
        created = min(
            created
            or stamp(
                day + timedelta(days=self.rng.choice((0, 0, 1, 2))),
                self.rng.randint(7, 22),
                self.rng.randint(0, 59),
            ),
            self.now,
        )
        expense = {
            "id": self.uuid(created),
            "title": title,
            "store": store,
            "description": description,
            "comments": comments,
            "referenceAmount": reference,
            "accrualStart": iso(stamp(start)),
            "accrualEnd": iso(stamp(end)),
            "isTemplate": template,
            "isTaxRelevant": tax,
            "isUnchecked": unchecked,
            "isIncomplete": incomplete,
            "isExtraordinary": extraordinary,
            "receipts": [],
            "categories": [{"categoryId": c, "weight": w} for c, w in categories],
            "transactions": [
                {
                    "accountId": leg["account"],
                    "date": iso(stamp(leg.get("day", day))),
                    "bookingDate": iso(stamp(leg["booked"])) if leg.get("booked") else None,
                    "amount": leg.get("amount"),
                    "fraction": leg.get("fraction"),
                    "comments": leg.get("comments", ""),
                    "statement": leg.get("statement", ""),
                }
                for leg in legs
            ],
            "positions": [
                {"amount": a, "quantity": q, "unit": u, "title": t, "comments": ""}
                for t, q, u, a in positions
            ],
            "createdAt": iso(created),
            "updatedAt": iso(created),
        }
        expense["categories"].sort(key=lambda c: c["categoryId"])
        self.expenses.append(expense)

        if not template:
            for leg in legs:
                cents = (
                    leg["amount"] if leg.get("amount") is not None else leg["fraction"] * reference
                )
                self.ledger[leg["account"]].append((stamp(leg.get("day", day)), cents))
        return expense

    def held(self, account, before):
        """What an account held just before an instant, opening reading included."""
        return self.opening[account] + sum(c for m, c in self.ledger[account] if m < before)

    def reading(self, account, day, comments="Kontoauszug", amount=None):
        moment = stamp(day, 0)
        amount = round(self.held(account, moment)) if amount is None else amount
        created = stamp(day, 18, self.rng.randint(0, 59))
        self.balances.append(
            {
                "id": self.uuid(created),
                "accountId": account,
                "date": iso(moment),
                "amount": amount,
                "comments": comments,
                "createdAt": iso(created),
                "updatedAt": iso(created),
            }
        )

    # --- rules -------------------------------------------------------------------------------

    def periodic_rule(self, user, title, template, period, last_creation, replacement=None):
        created = stamp(date(2024, 11, 28), 20)
        self.periodic.append(
            {
                "id": self.uuid(created),
                "userId": user,
                "title": title,
                "templateId": template["id"],
                "period": period,
                "titleReplacement": replacement,
                "lastCreation": iso(last_creation),
                "createdAt": iso(created),
                "updatedAt": iso(created),
            }
        )

    def automation_rule(self, user, title, template, priority, statement, account=None):
        created = stamp(date(2024, 11, 28), 21)
        rule = {
            "id": self.uuid(created),
            "userId": user,
            "title": title,
            "templateId": template["id"],
            "priority": priority,
            "regexTime": None,
            "regexStatement": statement,
            "filterAmount": None,
            "filterAccountId": account,
            "lastDelivery": None,
            "createdAt": iso(created),
            "updatedAt": iso(created),
        }
        self.automation.append(rule)
        return rule

    def delivered(self, rule, user, account, day, cents, statement, expense):
        """The bank line an automation rule turned into `expense`."""
        created = (
            datetime.fromisoformat(expense["createdAt"])
            if expense
            else min(stamp(day + timedelta(days=1), 6, self.rng.randint(0, 59)), self.now)
        )
        self.deliveries.append(
            {
                "id": self.uuid(created),
                "userId": user,
                "accountId": account,
                "date": iso(stamp(day)),
                "amount": cents,
                "statement": statement,
                "expenseId": expense["id"] if expense else None,
                "createdAt": iso(created),
            }
        )
        if rule:
            rule["lastDelivery"] = iso(created)

    def change(self, user, moment, old, new, tool=None):
        self.changes.append(
            {
                "id": self.uuid(moment),
                "targetId": (new or old)["id"],
                "userId": user,
                "date": iso(moment),
                "tool": tool,
                "automatic": tool is not None,
                "confidence": 1.0,
                "oldState": jsonb_order(old) if old else None,
                "newState": jsonb_order(new) if new else None,
            }
        )

    def document(self):
        def by_id(rows):
            return sorted(rows, key=lambda r: r["id"])

        return {
            "users": by_id(self.users),
            "categories": by_id(self.categories),
            "accounts": by_id(self.accounts),
            "expenses": by_id(self.expenses),
            "balances": by_id(self.balances),
            "periodicRules": by_id(self.periodic),
            "automationRules": by_id(self.automation),
            "seenDeliveries": by_id(self.deliveries),
            "expenseChanges": by_id(self.changes),
        }


def generate(until):
    rng = random.Random(SEED)
    h = Household(until, rng)

    # the user name is what the app shows (the greeting, the changelog), so it is the name
    rudolph = h.user(
        "Rudolph",
        "Rudolph",
        stamp(until - timedelta(days=3), 19, 12),
        stamp(until - timedelta(days=8), 7, 45),
    )
    kroesus = h.user(
        "Krösus",
        "Krösus",
        stamp(until - timedelta(days=2), 21, 3),
        stamp(until - timedelta(days=6), 20, 30),
    )

    # --- categories ----------------------------------------------------------------------

    wohnen = h.category("Wohnen", "#8b5cf6")
    miete = h.category("Miete", "#7c3aed", wohnen, "Warmmiete mit Nebenkosten")
    energie = h.category("Energie", "#a78bfa", wohnen, "Strom")
    internet = h.category("Internet & Telefon", "#c4b5fd", wohnen)
    lebensmittel = h.category("Lebensmittel", "#22c55e", limit=70000)
    supermarkt = h.category("Supermarkt", "#16a34a", lebensmittel)
    markt = h.category("Wochenmarkt & Bäcker", "#4ade80", lebensmittel)
    kinder = h.category("Kinder", "#f59e0b")
    kita = h.category("Kita", "#d97706", kinder)
    kinderkram = h.category("Spielzeug & Kleidung", "#fbbf24", kinder)
    versicherungen = h.category("Versicherungen", "#0ea5e9")
    mobilitaet = h.category("Mobilität", "#64748b", description="Deutschlandticket, Fahrrad")
    freizeit = h.category("Freizeit", "#ec4899", limit=25000)
    abos = h.category("Abos", "#db2777", freizeit)
    restaurant = h.category("Restaurant & Café", "#f472b6", freizeit)
    urlaub = h.category("Urlaub", "#be185d", freizeit)
    haushalt = h.category("Haushalt", "#78716c", description="Drogerie, Geräte, Reparaturen")
    gesundheit = h.category("Gesundheit", "#ef4444")
    einkommen = h.category("Einkommen", "#10b981")
    gehalt = h.category("Gehalt", "#059669", einkommen)
    zinsen = h.category("Zinsen", "#34d399", einkommen)

    # --- accounts --------------------------------------------------------------------------

    giro = h.account(
        rudolph,
        "Girokonto",
        "debit",
        "immediately",
        "none",
        date(2016, 4, 1),
        "#facc15",
        284012,
        description="Commerzbank",
        iban="DE00000000000000000000",
    )
    karte = h.account(
        rudolph,
        "Kreditkarte",
        "credit",
        "immediately",
        "none",
        date(2016, 4, 1),
        "#f59e0b",
        -21450,
        description="Commerzbank",
    )
    tagesgeld = h.account(
        rudolph,
        "Tagesgeld",
        "debit",
        "days",
        "none",
        date(2018, 9, 1),
        "#0ea5e9",
        612000,
        description="Commerzbank",
    )
    depot = h.account(
        rudolph,
        "Depot",
        "stocks",
        "years",
        "medium",
        date(2019, 1, 15),
        "#6366f1",
        1482000,
        description="ETF-Sparplan",
    )
    bargeld = h.account(
        rudolph, "Bargeld", "cash", "immediately", "slight", date(2016, 4, 1), "#84cc16", 12000
    )
    schulden_r = h.account(rudolph, "Schulden bei Krösus", "debt", "days", "none", START, "#ef4444")
    k_giro = h.account(
        kroesus,
        "Girokonto",
        "debit",
        "immediately",
        "none",
        date(2017, 2, 1),
        "#14b8a6",
        248030,
        description="Commerzbank",
    )
    k_bargeld = h.account(
        kroesus, "Bargeld", "cash", "immediately", "slight", date(2017, 2, 1), "#a3e635", 6000
    )
    schulden_k = h.account(
        kroesus, "Schulden bei Rudolph", "debt", "days", "none", START, "#f97316"
    )
    h.synchronize(schulden_r, schulden_k)

    # which debt account takes the other half when this person pays
    share_of = {giro: schulden_r, karte: schulden_r, k_giro: schulden_k}

    def split(payer, cents, statement="", booked=None):
        """A purchase paid by one and shared half and half: the booking is the whole of the
        reference amount, the other half goes onto the payer's debt account."""
        return dict(
            reference=cents,
            legs=[
                {"account": payer, "fraction": 1.0, "statement": statement, "booked": booked},
                {"account": share_of[payer], "fraction": -0.5},
            ],
        )

    def month_of(day):
        return day.replace(day=1), last_day(day.year, day.month)

    # --- templates and rules -----------------------------------------------------------------

    setup = stamp(date(2024, 11, 28), 20, 15)

    miete_tpl = h.expense(
        "Miete",
        START,
        accrual=month_of(START),
        store="Hausverwaltung",
        categories=[(miete, 1.0)],
        template=True,
        created=setup,
        **split(giro, -138000, "DAUERAUFTRAG MIETE WHG 3 OG TESTSTADT"),
    )
    spar_tpl = h.expense(
        "Sparplan",
        date(2024, 12, 2),
        template=True,
        created=setup,
        legs=[
            {"account": giro, "amount": -25000, "statement": "SPARPLAN ETF DEPOT"},
            {"account": depot, "amount": 25000},
        ],
    )
    haftpflicht_tpl = h.expense(
        "Haftpflichtversicherung",
        date(2025, 1, 15),
        store="Beispielversicherung",
        accrual=(date(2025, 1, 1), date(2025, 12, 31)),
        categories=[(versicherungen, 1.0)],
        template=True,
        created=setup,
        **split(giro, -7850, "BEISPIELVERSICHERUNG 000000000000 HAFTPFLICHT"),
    )

    gehalt_tpl = h.expense(
        "Gehalt",
        START,
        accrual=month_of(START),
        store="Beispiel GmbH",
        categories=[(gehalt, 1.0)],
        template=True,
        created=setup,
        legs=[{"account": giro, "amount": 0}],
    )
    k_gehalt_tpl = h.expense(
        "Gehalt",
        START,
        accrual=month_of(START),
        store="Stadt Teststadt",
        categories=[(gehalt, 1.0)],
        template=True,
        created=setup,
        legs=[{"account": k_giro, "amount": 0}],
    )
    rewe_tpl = h.expense(
        "Einkauf",
        START,
        store="REWE",
        categories=[(supermarkt, 1.0)],
        template=True,
        unchecked=True,
        created=setup,
        reference=0,
        legs=[
            {"account": giro, "amount": 0},
            {"account": schulden_r, "fraction": -0.5},
        ],
    )
    # paid on the last workday of the month before, for the month it pays for
    kita_tpl = h.expense(
        "Kita",
        date(2024, 11, 29),
        accrual=month_of(START),
        store="Kita Sonnenschein",
        categories=[(kita, 1.0)],
        tax=True,
        template=True,
        created=setup,
        reference=0,
        legs=[
            {"account": k_giro, "amount": 0},
            {"account": schulden_k, "fraction": -0.5},
        ],
    )
    spotify_tpl = h.expense(
        "Spotify Family",
        START,
        accrual=month_of(START),
        store="Spotify",
        categories=[(abos, 1.0)],
        template=True,
        created=setup,
        legs=[{"account": karte, "amount": 0}],
    )

    rewe_rule = h.automation_rule(rudolph, "REWE", rewe_tpl, 10, "REWE", giro)
    k_rewe_tpl = h.expense(
        "Einkauf",
        START,
        store="REWE",
        categories=[(supermarkt, 1.0)],
        template=True,
        unchecked=True,
        created=setup,
        reference=0,
        legs=[
            {"account": k_giro, "amount": 0},
            {"account": schulden_k, "fraction": -0.5},
        ],
    )
    k_rewe_rule = h.automation_rule(kroesus, "REWE", k_rewe_tpl, 10, "REWE", k_giro)
    gehalt_rule = h.automation_rule(rudolph, "Gehalt", gehalt_tpl, 20, r"LOHN/GEHALT", giro)
    k_gehalt_rule = h.automation_rule(kroesus, "Gehalt", k_gehalt_tpl, 20, r"LOHN/GEHALT", k_giro)
    kita_rule = h.automation_rule(kroesus, "Kita", kita_tpl, 15, r"KITA SONNENSCHEIN", k_giro)
    spotify_rule = h.automation_rule(rudolph, "Spotify", spotify_tpl, 5, r"Spotify", karte)

    # --- two years of it ---------------------------------------------------------------------

    def happened(day):
        return START <= day <= until

    def auto(rule, user, template, account, day, cents, statement, unchecked=None, **fields):
        """What an automation rule makes of a delivered line: the template around it, the
        delivered booking in place of the blank one."""
        start, end = fields.pop("accrual", (day, day))
        shared = any(t["fraction"] is not None for t in template["transactions"])
        legs = [
            {
                "account": account,
                "fraction": 1.0 if shared else None,
                "amount": None if shared else cents,
                "statement": statement,
                "booked": day + timedelta(days=rng.choice((0, 1, 1, 3))),
            }
        ]
        legs += [
            {"account": t["accountId"], "fraction": t["fraction"], "amount": t["amount"]}
            for t in template["transactions"]
            if t["amount"] != 0
        ]
        created = min(stamp(day + timedelta(days=1), 6, rng.randint(0, 59)), h.now)
        expense = h.expense(
            template["title"],
            day,
            legs,
            store=template["store"],
            accrual=(start, end),
            reference=cents if shared else None,
            created=created,
            categories=[(c["categoryId"], c["weight"]) for c in template["categories"]],
            tax=template["isTaxRelevant"],
            unchecked=template["isUnchecked"] if unchecked is None else unchecked,
            **fields,
        )
        h.delivered(rule, user, account, day, cents, statement, expense)
        return expense

    # --- the things that are not every month, before the monthly settlements that cover them -

    def once(title, day, **fields):
        if happened(day):
            return h.expense(title, day, **fields)

    once(
        "Waschmaschine",
        date(2025, 5, 17),
        store="Elektro Beispiel",
        extraordinary=True,
        categories=[(haushalt, 1.0)],
        description="Die alte hat nach zwölf Jahren aufgegeben",
        positions=[
            ("Waschmaschine 8 kg", 1.0, "Stück", -59900),
            ("Lieferung und Anschluss", 1.0, "Stück", -5000),
        ],
        **split(giro, -64900, "ELEKTRO BEISPIEL TESTSTADT"),
    )
    once(
        "Ferienwohnung Ostsee",
        date(2025, 7, 20),
        store="Ferienwohnungen Beispiel",
        extraordinary=True,
        accrual=(date(2025, 8, 2), date(2025, 8, 16)),
        categories=[(urlaub, 1.0)],
        **split(karte, -184000, "FEWO BEISPIEL"),
    )
    once(
        "Steuererstattung 2024",
        date(2025, 9, 18),
        store="Finanzamt Teststadt",
        extraordinary=True,
        tax=True,
        categories=[(einkommen, 1.0)],
        legs=[{"account": giro, "amount": 41230, "statement": "FINANZAMT TESTSTADT ESt 2024"}],
    )
    once(
        "Tagesgeld",
        date(2025, 8, 25),
        comments="für den Urlaub",
        legs=[
            {"account": tagesgeld, "amount": -100000, "statement": "UEBERTRAG GIRO"},
            {"account": giro, "amount": 100000, "statement": "UEBERTRAG TAGESGELD"},
        ],
    )
    once(
        "Ferienhaus Schwarzwald",
        date(2026, 6, 2),
        store="Ferienwohnungen Beispiel",
        extraordinary=True,
        accrual=(date(2026, 7, 25), date(2026, 8, 8)),
        categories=[(urlaub, 1.0)],
        **split(karte, -212000, "FEWO BEISPIEL"),
    )
    once(
        "Weihnachtsgeschenke",
        date(2024, 12, 14),
        store="BEISPIELMARKT/TESTSTADT",
        extraordinary=True,
        categories=[(kinderkram, 0.6), (freizeit, 0.4)],
        **split(karte, -23400, "BEISPIELMARKT/TESTSTADT"),
    )
    once(
        "Laptop",
        date(2026, 2, 21),
        store="Elektro Beispiel",
        extraordinary=True,
        categories=[(haushalt, 1.0)],
        legs=[{"account": karte, "amount": -104900, "statement": "ELEKTRO BEISPIEL TESTSTADT"}],
    )
    once(
        "Fahrradreparatur",
        date(2026, 4, 11),
        store="Radladen Teststadt",
        categories=[(mobilitaet, 1.0)],
        legs=[{"account": giro, "amount": -18450}],
    )
    once(
        "Weihnachtsgeschenke",
        date(2025, 12, 13),
        store="BEISPIELMARKT/TESTSTADT",
        extraordinary=True,
        categories=[(kinderkram, 0.6), (freizeit, 0.4)],
        **split(karte, -26800, "BEISPIELMARKT/TESTSTADT"),
    )
    once(
        "Zahnarzt",
        date(2026, 6, 23),
        store="Praxis Dr. Beispiel",
        tax=True,
        categories=[(gesundheit, 1.0)],
        legs=[{"account": k_giro, "amount": -14000, "statement": "PRAXIS DR BEISPIEL"}],
    )
    once(
        "Online-Bestellung Regenjacke",
        until - timedelta(days=12),
        store="Versand Beispiel",
        incomplete=True,
        comments="Eine Jacke geht zurück, Erstattung steht aus",
        categories=[(kinderkram, 1.0)],
        legs=[{"account": karte, "amount": -8998, "statement": "VERSAND BEISPIEL"}],
    )

    periodic = set()

    def by_rule(title, day, at, **fields):
        """What `moneta periodic --run` wrote: created at the occurrence itself."""
        expense = h.expense(title, day, created=stamp(at), **fields)
        periodic.add(expense["id"])

    recent = until - timedelta(days=21)
    for year, month in months(START, until):
        first = date(year, month, 1)
        end = last_day(year, month)
        period = (first, end)
        raise_2026 = year >= 2026

        # rent, savings plan, insurance: written by the periodic rules
        if happened(first):
            by_rule(
                f"Miete {month}/{year}",
                first,
                first,
                accrual=period,
                store="Hausverwaltung",
                categories=[(miete, 1.0)],
                **split(giro, -138000, "DAUERAUFTRAG MIETE WHG 3 OG TESTSTADT"),
            )
        if happened(date(year, month, 2)):
            by_rule(
                "Sparplan",
                date(year, month, 2),
                date(year, month, 2),
                legs=[
                    {"account": giro, "amount": -25000, "statement": "SPARPLAN ETF DEPOT"},
                    {"account": depot, "amount": 25000},
                ],
            )
        # anchored on the first of the year it covers, debited on the 15th
        if month == 1 and happened(date(year, 1, 1)):
            by_rule(
                "Haftpflichtversicherung",
                date(year, 1, 15),
                date(year, 1, 1),
                store="Beispielversicherung",
                accrual=(date(year, 1, 1), date(year, 12, 31)),
                categories=[(versicherungen, 1.0)],
                **split(giro, -7850, "BEISPIELVERSICHERUNG 000000000000 HAFTPFLICHT"),
            )

        # salaries, delivered
        payday = workday_before(date(year, month, 27))
        if happened(payday):
            cents = 272840 if raise_2026 else 265120
            auto(
                gehalt_rule,
                rudolph,
                gehalt_tpl,
                giro,
                payday,
                cents,
                f"LOHN/GEHALT {month:02d}/{year} BEISPIEL GMBH",
                accrual=period,
            )
        k_payday = workday_before(end)
        if happened(k_payday):
            cents = 191260 if raise_2026 else 186400
            auto(
                k_gehalt_rule,
                kroesus,
                k_gehalt_tpl,
                k_giro,
                k_payday,
                cents,
                f"LOHN/GEHALT {month:02d}/{year} STADT TESTSTADT",
                accrual=period,
            )

        # Kita for next month, debited on the last workday of this one
        kita_day = workday_before(end)
        nxt = end + timedelta(days=1)
        if happened(kita_day):
            auto(
                kita_rule,
                kroesus,
                kita_tpl,
                k_giro,
                kita_day,
                -38000,
                f"KITA SONNENSCHEIN BEITRAG {nxt.month:02d}/{nxt.year}",
                accrual=(nxt, last_day(nxt.year, nxt.month)),
            )

        # direct debits entered by hand
        if happened(date(year, month, 15)):
            h.expense(
                "Strom Abschlag",
                date(year, month, 15),
                store="Stadtwerke Teststadt",
                accrual=period,
                categories=[(energie, 1.0)],
                **split(
                    k_giro,
                    -9400 if year == 2024 or (year == 2025 and month < 4) else -10200,
                    "STADTWERKE TESTSTADT ABSCHLAG STROM",
                ),
            )
        if happened(date(year, month, 4)):
            h.expense(
                "Internet",
                date(year, month, 4),
                store="Beispiel Telekom",
                accrual=period,
                categories=[(internet, 1.0)],
                **split(giro, -3999, "BEISPIEL TELEKOM RECHNUNG"),
            )
            h.expense(
                "Zahnzusatzversicherung",
                date(year, month, 4),
                store="Beispielversicherung",
                accrual=period,
                categories=[(versicherungen, 1.0)],
                legs=[
                    {
                        "account": giro,
                        "amount": -2390,
                        "statement": "BEISPIELVERSICHERUNG 000000000000 ZAHN",
                    }
                ],
            )
            h.expense(
                "Netflix",
                date(year, month, 4),
                store="Netflix",
                accrual=period,
                categories=[(abos, 1.0)],
                legs=[{"account": k_giro, "amount": -1399, "statement": "NETFLIX.COM"}],
            )
        if happened(date(year, month, 1)):
            for who, payer in ((rudolph, karte), (kroesus, k_giro)):
                h.expense(
                    "Deutschlandticket",
                    date(year, month, 1),
                    store="Teststadt Verkehr",
                    accrual=period,
                    categories=[(mobilitaet, 1.0)],
                    legs=[
                        {
                            "account": payer,
                            "amount": -5800 if year < 2026 else -6300,
                            "statement": "TESTSTADT VERKEHR D-TICKET",
                        }
                    ],
                )
        if month == 3 and happened(date(year, 3, 3)):
            h.expense(
                "Hausratversicherung",
                date(year, 3, 3),
                store="Beispielversicherung",
                accrual=(date(year, 3, 1), date(year + 1, 2, last_day(year + 1, 2).day)),
                categories=[(versicherungen, 1.0)],
                **split(k_giro, -11240, "BEISPIELVERSICHERUNG 000000000000 HAUSRAT"),
            )
        if month == 2 and happened(date(year, 2, 10)):
            h.expense(
                "Zeitung Jahresabo",
                date(year, 2, 10),
                store="Teststädter Rundschau",
                accrual=(date(year, 2, 10), date(year + 1, 2, 9)),
                categories=[(abos, 1.0)],
                legs=[{"account": karte, "amount": -16800, "statement": "RUNDSCHAU ABO"}],
            )

        spotify_day = date(year, month, 8)
        if happened(spotify_day):
            auto(
                spotify_rule,
                rudolph,
                spotify_tpl,
                karte,
                spotify_day,
                -1799,
                "Spotify P3A1B2C3D4 Stockholm",
                accrual=period,
            )

        # groceries: whoever is at the till pays, half of it is the other's
        shopping = sorted(rng.sample(range(1, end.day + 1), 6))
        for i, d in enumerate(shopping):
            day = date(year, month, d)
            if not happened(day):
                continue
            cents = -rng.randint(2500, 16800)
            if i % 2 == 0:
                auto(
                    rewe_rule,
                    rudolph,
                    rewe_tpl,
                    giro,
                    day,
                    cents,
                    f"REWE SAGT DANKE {rng.randint(10000, 99999)}//TESTSTADT",
                    unchecked=day >= recent,
                )
            else:
                auto(
                    k_rewe_rule,
                    kroesus,
                    k_rewe_tpl,
                    k_giro,
                    day,
                    cents,
                    f"REWE MARKT GMBH {rng.randint(10000, 99999)}//TESTSTADT",
                    unchecked=day >= recent,
                )
        for d in sorted(rng.sample(range(1, end.day + 1), 2)):
            day = date(year, month, d)
            if happened(day):
                payer = rng.choice((giro, k_giro))
                h.expense(
                    "Drogerie",
                    day,
                    store="BEISPIELMARKT/TESTSTADT",
                    categories=[(haushalt, 0.7), (gesundheit, 0.3)],
                    **split(payer, -rng.randint(900, 4800), "BEISPIELMARKT/TESTSTADT"),
                )
        for i, d in enumerate(sorted(rng.sample(range(1, end.day + 1), 6))):
            day = date(year, month, d)
            if happened(day):
                cash = (bargeld, k_bargeld)[i % 2]
                what = rng.choice(
                    (("Wochenmarkt", "Wochenmarkt Teststadt"), ("Brötchen", "Bäckerei Beispiel"))
                )
                h.expense(
                    what[0],
                    day,
                    store=what[1],
                    categories=[(markt, 1.0)],
                    legs=[{"account": cash, "amount": -rng.randint(350, 2800)}],
                )
        for d in sorted(rng.sample(range(1, end.day + 1), 2)):
            day = date(year, month, d)
            if happened(day):
                h.expense(
                    rng.choice(("Pizza", "Café", "Essen beim Italiener", "Eis")),
                    day,
                    store=rng.choice(("Trattoria Beispiel", "Café am Markt", "Eiscafé Teststadt")),
                    categories=[(restaurant, 1.0)],
                    **split(karte, -rng.randint(1200, 7800), "KARTENZAHLUNG GASTRONOMIE"),
                )
        if happened(date(year, month, 5)):
            h.expense(
                "Schwimmbad",
                date(year, month, 5),
                store="Hallenbad Teststadt",
                accrual=period,
                categories=[(freizeit, 1.0)],
                legs=[{"account": karte, "amount": -4900, "statement": "HALLENBAD"}],
            )
        for d in sorted(rng.sample(range(1, end.day + 1), 2)):
            day = date(year, month, d)
            if happened(day):
                h.expense(
                    "Carsharing",
                    day,
                    store="Teilauto Teststadt",
                    categories=[(mobilitaet, 1.0)],
                    **split(karte, -rng.randint(1900, 6400), "TEILAUTO TESTSTADT"),
                )
        if month in (4, 10) and happened(date(year, month, 19)):
            for payer in (karte, k_giro):
                h.expense(
                    "Kleidung",
                    date(year, month, 19),
                    store="BEISPIELMARKT/TESTSTADT",
                    categories=[(haushalt, 1.0)],
                    legs=[
                        {
                            "account": payer,
                            "amount": -rng.randint(6000, 17000),
                            "statement": "BEISPIELMARKT/TESTSTADT",
                        }
                    ],
                )
        if month in (3, 6, 9, 11) and happened(date(year, month, 12)):
            h.expense(
                "Kinderkleidung",
                date(year, month, 12),
                store="BEISPIELMARKT/TESTSTADT",
                categories=[(kinderkram, 1.0)],
                **split(k_giro, -rng.randint(3500, 9800), "BEISPIELMARKT/TESTSTADT"),
            )

        # cash from the machine
        for who, account, cash in ((rudolph, giro, bargeld), (kroesus, k_giro, k_bargeld)):
            day = date(year, month, 10)
            if happened(day):
                h.expense(
                    "Bargeld",
                    day,
                    legs=[
                        {
                            "account": account,
                            "amount": -5000,
                            "statement": "GA COMMERZBANK TESTSTADT",
                        },
                        {"account": cash, "amount": 5000},
                    ],
                )

        # to the Tagesgeld
        if happened(date(year, month, 3)):
            h.expense(
                "Tagesgeld",
                date(year, month, 3),
                legs=[
                    {"account": giro, "amount": -20000, "statement": "UEBERTRAG TAGESGELD"},
                    {"account": tagesgeld, "amount": 20000},
                ],
            )
        # interest once a year, on roughly what sat there (a month of it in the first December)
        if month == 12 and happened(date(year, 12, 31)):
            held = h.held(tagesgeld, stamp(date(year, 12, 31)))
            interest = round(held * 0.0225 / (12 if year == START.year else 1.6))
            h.expense(
                "Zinsen",
                date(year, 12, 31),
                store="Commerzbank",
                categories=[(zinsen, 1.0)],
                legs=[{"account": tagesgeld, "amount": interest, "statement": "ZINSEN"}],
            )

        # the credit card is settled from the Girokonto on the 6th, for the month before
        settle = date(year, month, 6)
        if happened(settle):
            owed = -round(h.held(karte, stamp(first, 0)))
            if owed > 0:
                h.expense(
                    "Kreditkartenabrechnung",
                    settle,
                    legs=[
                        {"account": giro, "amount": -owed, "statement": "KREDITKARTENABRECHNUNG"},
                        {"account": karte, "amount": owed},
                    ],
                )

        # Krösus settles what he owes, rounded to ten euros, on the 28th
        ausgleich = date(year, month, 28)
        if happened(ausgleich):
            owed = round(
                h.held(schulden_r, stamp(ausgleich)) - h.held(schulden_k, stamp(ausgleich))
            )
            owed = owed // 1000 * 1000
            if owed > 0:
                h.expense(
                    "Ausgleich",
                    ausgleich,
                    comments="Monatsausgleich",
                    legs=[
                        {"account": k_giro, "amount": -owed, "statement": "AUSGLEICH WG KASSE"},
                        {"account": schulden_k, "amount": owed},
                        {"account": giro, "amount": owed, "statement": "AUSGLEICH WG KASSE"},
                    ],
                )

    # --- a few things to look at on the start page -------------------------------------------

    # lines the bank delivered and no rule claimed yet
    for days, account, user, cents, statement in (
        (2, giro, rudolph, -2490, "PAYPAL *BEISPIELSHOP 1043"),
        (3, k_giro, kroesus, -6500, "SEPA LASTSCHRIFT SPORTVEREIN TESTSTADT"),
    ):
        h.delivered(None, user, account, until - timedelta(days=days), cents, statement, None)

    # Krösus went through a delivered REWE purchase and typed in what was in the bag
    groceries = [
        e
        for e in h.expenses
        if e["store"] == "REWE"
        and not e["isTemplate"]
        and e["isUnchecked"]
        and e["transactions"][0]["accountId"] == k_giro
        and e["createdAt"] < iso(stamp(until - timedelta(days=2)))
    ]
    if groceries:
        checked = groceries[-1]
        before = json.loads(json.dumps(checked))
        checked["isUnchecked"] = False
        total = checked["referenceAmount"]
        checked["positions"] = [
            {
                "amount": round(total * 0.4),
                "quantity": 1.0,
                "unit": None,
                "title": "Obst und Gemüse",
                "comments": "",
            },
            {
                "amount": round(total * 0.35),
                "quantity": 1.0,
                "unit": None,
                "title": "Milchprodukte",
                "comments": "",
            },
            {
                "amount": total - round(total * 0.4) - round(total * 0.35),
                "quantity": 2.0,
                "unit": "Packung",
                "title": "Windeln",
                "comments": "",
            },
        ]
        checked["categories"] = sorted(
            [
                {"categoryId": supermarkt, "weight": 0.75},
                {"categoryId": kinderkram, "weight": 0.25},
            ],
            key=lambda c: c["categoryId"],
        )
        moment = stamp(until - timedelta(days=1), 20, 14)
        checked["updatedAt"] = iso(moment)
        h.change(kroesus, moment, before, checked)

    # Rudolph corrected a restaurant bill he had typed in wrong
    meals = [
        e
        for e in h.expenses
        if e["categories"]
        and e["categories"][0]["categoryId"] == restaurant
        and e["transactions"][0]["accountId"] == karte
    ]
    if meals:
        meal = meals[-1]
        before = json.loads(json.dumps(meal))
        meal["comments"] = "mit Trinkgeld"
        moment = stamp(until - timedelta(days=2), 18, 41)
        meal["updatedAt"] = iso(moment)
        h.change(rudolph, moment, before, meal)

    # creations of the last weeks, by whoever or whatever made them
    owner = {a["id"]: a["userId"] for a in h.accounts}
    delivered = {d["expenseId"] for d in h.deliveries}
    for e in h.expenses:
        created = datetime.fromisoformat(e["createdAt"])
        if e["isTemplate"] or created.date() < until - timedelta(days=30):
            continue
        tool = (
            "moneta-automation"
            if e["id"] in delivered
            else "moneta-periodic"
            if e["id"] in periodic
            else None
        )
        # an expense edited since was created as it looked before the edit
        snapshot = next((c["oldState"] for c in h.changes if c["targetId"] == e["id"]), e)
        h.change(owner[e["transactions"][0]["accountId"]], created, None, snapshot, tool)

    # --- what the accounts held, read off now and then ---------------------------------------

    for account in (giro, karte, tagesgeld, bargeld, k_giro, k_bargeld):
        h.reading(account, START, "Startstand")
    for day in (
        date(2025, 1, 1),
        date(2025, 7, 1),
        date(2026, 1, 1),
        date(2026, 7, 1),
        date(2026, 10, 1),
    ):
        if happened(day):
            for account in (giro, tagesgeld, k_giro):
                h.reading(account, day)
    for day in (date(2025, 1, 1), date(2026, 1, 1)):
        if happened(day):
            for account in (bargeld, k_bargeld):
                h.reading(account, day, "gezählt")

    # the depot is read at the turn of the year, with a conservative eye on the gains
    h.reading(depot, START, "Depotauszug")
    value = 1482000
    for year in (2025, 2026):
        day = date(year, 1, 1)
        if happened(day):
            contributed = sum(
                c for m, c in h.ledger[depot] if stamp(date(year - 1, 1, 1), 0) <= m < stamp(day, 0)
            )
            value = round((value + contributed) * (1.012 if year == 2025 else 1.061) / 100) * 100
            h.reading(depot, day, "Depotauszug, Kurse zum Jahresende abgerundet", value)

    # rule bookkeeping: up to the last occurrence the expenses above already contain
    def last_occurrence(template):
        return max(
            datetime.fromisoformat(e["accrualStart"])
            for e in h.expenses
            if e["id"] in periodic
            and e["store"] == template["store"]
            and e["title"].startswith(template["title"])
        )

    h.periodic_rule(
        rudolph, "Miete", miete_tpl, "monthly", last_occurrence(miete_tpl), "Miete %m/%y"
    )
    h.periodic_rule(rudolph, "Sparplan", spar_tpl, "monthly", last_occurrence(spar_tpl))
    h.periodic_rule(
        rudolph,
        "Haftpflichtversicherung",
        haftpflicht_tpl,
        "yearly",
        last_occurrence(haftpflicht_tpl),
    )

    return h.document()


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "--until",
        type=date.fromisoformat,
        default=date(2026, 11, 8),
        help="the last day of the demo, 'today' as far as it is concerned",
    )
    parser.add_argument("--output", type=Path, default=Path(__file__).with_name("rudolph.json"))
    args = parser.parse_args()

    document = generate(args.until)
    args.output.write_text(
        json.dumps(document, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
