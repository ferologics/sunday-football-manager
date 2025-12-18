-- Migrate elo_snapshot from name-keyed to ID-keyed format
-- Example: {"Alice": {"before": 1200, ...}} -> {"1": {"before": 1200, ...}}

UPDATE matches m
SET elo_snapshot = (
    SELECT jsonb_object_agg(
        p.id::text,
        m.elo_snapshot->p.name
    )
    FROM players p
    WHERE m.elo_snapshot ? p.name
)
WHERE EXISTS (
    SELECT 1 FROM players p WHERE m.elo_snapshot ? p.name
);
