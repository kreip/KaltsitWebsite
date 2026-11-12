-- The markup tags of the game's texts, rebuilt by the import: the style
-- spans (`@ba.vup`, coloured from the game's RichTextStyles) and the terms
-- (`$ba.stun`, with the name and the description the game shows for them,
-- in English and in the pack's Russian), each with how many texts use it.
CREATE TABLE IF NOT EXISTS translators.tags (
    server TEXT NOT NULL,
    -- With its sigil: `@ba.vup`, `$ba.stun`.
    tag TEXT NOT NULL,
    -- `style` or `term`.
    kind TEXT NOT NULL,
    -- A style's colour in the game (`#0098DC`).
    color TEXT,
    name_en TEXT,
    name_ru TEXT,
    description_en TEXT,
    description_ru TEXT,
    -- Texts of the list (`translators.texts`) whose English holds it.
    uses INTEGER NOT NULL,
    PRIMARY KEY (server, tag)
);
