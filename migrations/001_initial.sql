-- Players table
CREATE TABLE IF NOT EXISTS players (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    elo REAL NOT NULL DEFAULT 1200.0,
    tags VARCHAR(255) NOT NULL DEFAULT '',
    matches_played INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Matches table
CREATE TABLE IF NOT EXISTS matches (
    id SERIAL PRIMARY KEY,
    played_at DATE NOT NULL DEFAULT CURRENT_DATE,
    team_a TEXT[] NOT NULL,
    team_b TEXT[] NOT NULL,
    score_a INTEGER NOT NULL,
    score_b INTEGER NOT NULL,
    elo_snapshot JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for faster player lookups
CREATE INDEX IF NOT EXISTS idx_players_name ON players(name);

-- Index for match history queries
CREATE INDEX IF NOT EXISTS idx_matches_played_at ON matches(played_at DESC);
