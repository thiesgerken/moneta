-- The amounts folded into the templates are deliberately not folded back out. The template is
-- where they belong now, and a rule that read its own amount again would apply the change twice
-- rather than undo it. This restores the shape, not the data.
ALTER TABLE periodic_rules ADD COLUMN amount_replacement integer;
