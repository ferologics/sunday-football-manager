# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
just run      # Run locally with Shuttle (alias: just r)
just check    # cargo check + clippy (alias: just c)
just deploy   # Deploy to Shuttle (alias: just d)
just test     # Run tests
```

## Architecture

Rust web app using Axum + Shuttle.rs for hosting, with PostgreSQL database.

**Tech Stack:**
- Axum 0.7 - Web framework
- Shuttle - Hosting and DB provisioning
- sqlx - Database queries (compile-time checked)
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

**Database:**
- PostgreSQL via `shuttle-shared-db`
- Migrations in `migrations/`
- Two tables: `players` and `matches`

**Team Balancing Algorithm:**
- Brute force all C(n, n/2) combinations
- Cost = |avg_elo_A - avg_elo_B| + Σ(tag_penalty × weight)
- Tag weights: PLAYMAKER(100) > RUNNER(80) > DEF(40) > ATK(20)
- GK handling: force split if 2 GKs, random if 1

**Elo System:**
- K-factor: 32
- Goal diff multiplier: min(1 + (GD-1)*0.5, 2.5)
- Standard expected score formula
