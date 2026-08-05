-- The initial schema indexes the join and filter columns already. What it misses is the two
-- places a rule is looked up from.

-- the highest-priority match wins, so this read is ordered
DROP INDEX automation_rules_user_id_idx;
CREATE INDEX automation_rules_user_priority_idx ON automation_rules (user_id, priority DESC);

-- both rule kinds load their template expense, and postgres indexes only the referenced side
-- of a foreign key
CREATE INDEX automation_rules_template_idx ON automation_rules (template_id);
CREATE INDEX periodic_rules_template_idx ON periodic_rules (template_id);
