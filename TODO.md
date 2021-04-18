# TODO

## Frontend+Backend

- importer: heuristic for automatic conversion of prepayments to booking periods?

- Vorauszahlungen sind Ausgaben mit Buchungszeit in der Zukunft
- Was ist mit Rückzahlungen?
- Miete sind zwei Transaktionen, die automatisch kommen + müssen auf den nächsten Monat gebucht werden ...
  -> Der User muss irgendwie die Möglichkeit haben, Ausgaben zu kombinieren oder zu gruppieren, dies muss auch durch Regeln möglich sein => Gruppierungs-Key?
- Gruppierung über "unbenannte" Gruppen (manuelles Verknüpfen von Ausgaben in der UI), und/oder über "benannte" Gruppen (zB. "miete-05-2021"), letztere Hauptsächlich gedacht für automatische Ausgaben
- paar Workflows überlegen und dokumentieren!

- edit/create/delete expenses, balances, categories, accounts
- automation using templates and expense deliveries
- analysis
- push notifications
- front page: stream of recent events

## Frontend

- on big screens: expandable rows for expense table?

## Backend

- express sql queries using diesel; common functionality -> postgres functions
