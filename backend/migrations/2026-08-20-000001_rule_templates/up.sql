-- The amount a periodic rule generates belongs on its template.
--
-- Two places held it and they disagreed in every rule that set the override, because moneydb
-- only ever wrote `amountReplacement` when the price had moved and nothing then maintained the
-- template: Rundfunk 10,00 against a template of 9,00, Grundsteuer 20,00 against 25,00, the
-- Zeitungs-Abo 5,00 against 4,00. The override is the current one, so it wins.
--
-- It also only ever reached a template with exactly one fixed transaction. With more there was
-- no way to divide it and `instantiate` applied none of it, silently. On the template each line
-- says what it is.
--
-- Only onto a template. The whole premise here is that a rule may be pointing at a real
-- booking -- 38 of 50 did -- and folding the amount onto one of those would rewrite money that
-- is already in the balances and the Auswertung. Establishing the templates is the importer's
-- job; this migration only moves what is safe to move.
UPDATE expense_transactions t
SET amount = r.amount_replacement
FROM periodic_rules r, expenses e
WHERE r.template_id = t.expense_id
  AND e.id = r.template_id
  AND e.is_template
  AND r.amount_replacement IS NOT NULL
  AND t.amount IS NOT NULL
  AND (
      SELECT count(*) FROM expense_transactions x
      WHERE x.expense_id = r.template_id AND x.amount IS NOT NULL
  ) = 1;

-- A fraction is a fraction of the booking, so the reference amount follows it -- but only where
-- there is a fraction at all, which is the invariant the rest of the schema is written to.
UPDATE expenses e
SET reference_amount = r.amount_replacement
FROM periodic_rules r
WHERE r.template_id = e.id
  AND e.is_template
  AND r.amount_replacement IS NOT NULL
  AND EXISTS (
      SELECT 1 FROM expense_transactions x
      WHERE x.expense_id = e.id AND x.fraction IS NOT NULL
  );

ALTER TABLE periodic_rules DROP COLUMN amount_replacement;
