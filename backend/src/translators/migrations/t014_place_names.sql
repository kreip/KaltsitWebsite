-- The objects' names: the search's block of objects and the archives'
-- object list read only the `name` places (a few thousand of 180,000).
CREATE INDEX IF NOT EXISTS text_places_names ON translators.text_places (server, section) WHERE field = 'name';
