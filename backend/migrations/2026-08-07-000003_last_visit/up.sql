-- When each reader last opened the start page, so it can say what has happened since.
--
-- Two columns rather than one: the page needs the visit *before* this one, and it has to keep
-- needing it for as long as this visit lasts. A single "last seen" moved on arrival would
-- answer "nothing has happened since you got here", which is true and useless.
--
-- Not the login time, which is the other obvious candidate and wrong twice over: a session
-- cookie outlives weeks of visits, so the marker would never move, and the day it finally did
-- it would jump to now and empty the page.
ALTER TABLE users
    ADD COLUMN seen_at timestamptz,
    ADD COLUMN seen_before timestamptz;
