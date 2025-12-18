# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
just run      # Run the Streamlit app (alias: just r)
just check    # Run ruff + ty checks (alias: just c)
uv sync       # Install dependencies
```

## Architecture

Single-file Streamlit app (`app.py`) for managing Sunday League 7v7 football. Uses Google Sheets as database via `gspread`.

**Data Flow:**
- Google Sheets stores `Players` (Name, Elo, Tags, Matches_Played) and `Matches` (Date, Team_A, Team_B, Score_A, Score_B)
- Credentials via `st.secrets["gcp_service_account"]` and `st.secrets["sheet_url"]`
- Guest players stored in `st.session_state.guests` (not persisted to sheet)

**Team Balancing Algorithm:**
- Brute force all C(n, n/2) combinations
- Cost function: `abs(avg_elo_A - avg_elo_B) + sum(tag_penalties)`
- Tag weights at top of file: PLAYMAKER > RUNNER > DEF > ATK
- GK special handling: force split if 2 GKs, random assignment if 1

**Elo System:**
- Standard Elo with K=32
- Goal difference multiplier: `min(1 + (GD-1)*0.5, 2.5)`
- Guests (`is_guest=True`) excluded from Elo updates

## Google Sheet Column Names

The code expects title case headers: `Name`, `Elo`, `Tags`, `Matches_Played`, `Date`, `Team_A`, `Team_B`, `Score_A`, `Score_B`
