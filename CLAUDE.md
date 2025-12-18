# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
just run      # Run locally (alias: just r)
just watch    # Run with auto-reload (alias: just w)
just check    # cargo check + clippy (alias: just c)
just test     # Run tests
just clean    # Clean build artifacts
```

## Architecture

Rust web app using Axum, hosted on Render.com with Neon PostgreSQL.

**Tech Stack:**
- Axum - Web framework
- Render - Hosting (Docker)
- Neon - Serverless PostgreSQL
- sqlx - Database queries (with TLS via rustls)
- Maud - Compile-time HTML templates
- htmx - Client-side interactivity
- PicoCSS - Styling

**Project Structure:**
- `src/main.rs` - Entry point, router setup
- `src/models.rs` - Data structures, constants
- `src/db.rs` - Database queries
- `src/balance.rs` - Team balancing algorithm
- `src/elo.rs` - Elo calculations
- `src/views/` - Maud HTML templates for each page

**Environment:**
- `.env` - Production config (DATABASE_URL, AUTH_PASSWORD)
- `.env.local` - Local overrides (optional, loaded first)

**Authentication:**
- Set `AUTH_PASSWORD` env var to enable login
- If not set, site runs unprotected (for dev)
- Protects: add/delete players, record results
- Read-only pages always accessible

**Database:**
- PostgreSQL via Neon (requires `?sslmode=require`)
- Migrations in `migrations/` (run automatically on startup)
- Two tables: `players` and `matches`

**Team Balancing Algorithm:**
- Brute force all C(n, n/2) combinations
- Cost = |avg_elo_A - avg_elo_B| + |tag_value_A - tag_value_B|
- Player tag value = sum of their tag weights
- Tag weights: PLAYMAKER(50) > RUNNER(40) > DEF(20) > ATK(10)
- This naturally splits "star" multi-tag players between teams
- GK handling: force split if 2 GKs, random if 1

**Elo System:**
- K-factor: 32
- Goal diff multiplier: min(1 + (GD-1)*0.5, 2.5)
- Standard expected score formula
- Handicap: 100 Elo per missing player-equivalent (for uneven teams/injuries)
- Participation: injured players get proportional Elo credit (e.g., 50% participation = 50% delta)
