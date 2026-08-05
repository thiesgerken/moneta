DROP INDEX periodic_rules_template_idx;
DROP INDEX automation_rules_template_idx;
DROP INDEX automation_rules_user_priority_idx;
CREATE INDEX automation_rules_user_id_idx ON automation_rules (user_id);
