# Football Manager

A web app for organizing pickup football games. Balances teams fairly and tracks player skill over time.

## Tech Stack

- **Backend**: Rust + Axum
- **Hosting**: Render.com
- **Database**: Neon (serverless PostgreSQL)
- **Templating**: Maud (compile-time HTML)
- **Interactivity**: htmx
- **Styling**: PicoCSS

## How It Works

1. **Before the game**: Check in who's playing today
2. **Team generation**: The app splits players into two balanced teams
3. **After the game**: Enter the score, and everyone's rating updates

## Features

### Player Roles

Each player can have tags describing their style:

| Tag           | What it means                              | Weight |
| ------------- | ------------------------------------------ | ------ |
| **PLAYMAKER** | Good at passing, vision, controls the ball | 100    |
| **RUNNER**    | High stamina, covers a lot of ground       | 80     |
| **DEF**       | Solid defender                             | 40     |
| **ATK**       | Finisher, good at scoring                  | 20     |
| **GK**        | Goalkeeper (special handling)              | -      |

### Team Balancing

The app tries every possible way to split players into two teams and picks the "fairest" split.

**Cost function:**
```
cost = |avg_elo_A - avg_elo_B| + sum(tag_penalties)
```

Where `tag_penalty = |count_A - count_B| × weight`

**GK handling:**
- 2 GKs → force one to each team
- 1 GK → random assignment

### Elo Ratings

Everyone starts at 1200. Win and your rating goes up; lose and it goes down.

```
expected = 1 / (1 + 10^((opponent_elo - my_elo) / 400))
actual = 1 (win), 0.5 (draw), 0 (loss)
gd_multiplier = min(1 + (goal_diff - 1) × 0.5, 2.5)
K = 32

elo_change = K × gd_multiplier × (actual - expected)
```

## Development

### Prerequisites

- Rust (stable)
- A PostgreSQL database (Neon free tier works)

### Setup

```bash
# Install cargo-watch for auto-reload
just setup

# Copy env file and add your DATABASE_URL
cp .env.example .env
```

### Run Locally

```bash
just run    # or: just r
```

### Commands

```bash
just run     # Run locally
just watch   # Run with auto-reload
just check   # Check + clippy
just test    # Run tests
just clean   # Clean build artifacts
```

## Deployment

Deployed on Render with Docker. Set `DATABASE_URL` env var to your Neon connection string.

## Project Structure

```
src/
├── main.rs       # Entry point, router
├── db.rs         # Database queries
├── models.rs     # Data structures
├── balance.rs    # Team balancing algorithm
├── elo.rs        # Elo calculations
└── views/
    ├── layout.rs     # Base HTML template
    ├── match_day.rs  # Check-in, team generation
    ├── roster.rs     # Player management
    ├── record.rs     # Record match results
    └── history.rs    # Match history
```
