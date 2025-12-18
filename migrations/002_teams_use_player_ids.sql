-- Migration: Convert team_a and team_b from TEXT[] (player names) to INTEGER[] (player IDs)

-- Add new ID-based columns
ALTER TABLE matches ADD COLUMN team_a_ids INTEGER[];
ALTER TABLE matches ADD COLUMN team_b_ids INTEGER[];

-- Populate new columns by looking up player IDs from names
-- Note: Players deleted before this migration will result in NULL entries
UPDATE matches m SET
    team_a_ids = (
        SELECT ARRAY_AGG(p.id ORDER BY idx)
        FROM unnest(m.team_a) WITH ORDINALITY AS t(name, idx)
        LEFT JOIN players p ON p.name = t.name
    ),
    team_b_ids = (
        SELECT ARRAY_AGG(p.id ORDER BY idx)
        FROM unnest(m.team_b) WITH ORDINALITY AS t(name, idx)
        LEFT JOIN players p ON p.name = t.name
    );

-- Drop old columns
ALTER TABLE matches DROP COLUMN team_a;
ALTER TABLE matches DROP COLUMN team_b;

-- Rename new columns to original names
ALTER TABLE matches RENAME COLUMN team_a_ids TO team_a;
ALTER TABLE matches RENAME COLUMN team_b_ids TO team_b;

-- Add NOT NULL constraint (may fail if there are NULLs from deleted players)
-- If this fails, you'll need to clean up matches with deleted players first
ALTER TABLE matches ALTER COLUMN team_a SET NOT NULL;
ALTER TABLE matches ALTER COLUMN team_b SET NOT NULL;
