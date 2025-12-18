# Sunday Football Manager

A simple app for organizing pickup football games. It balances teams fairly and tracks player skill over time.

## How It Works

1. **Before the game**: Check in who's playing today
2. **Team generation**: The app splits players into two balanced teams
3. **After the game**: Enter the score, and everyone's rating updates

That's it. No accounts, no complex setup—just fair teams every week.

---

## The Balancing Logic

### Player Roles

Each player can have tags describing their style:

| Tag           | What it means                              |
| ------------- | ------------------------------------------ |
| **PLAYMAKER** | Good at passing, vision, controls the ball |
| **RUNNER**    | High stamina, covers a lot of ground       |
| **DEF**       | Solid defender                             |
| **ATK**       | Finisher, good at scoring                  |
| **GK**        | Goalkeeper                                 |

Players can have multiple tags (e.g., a RUNNER who's also ATK).

### Why Roles Matter

In amateur football, some skills matter more than others:

- **Playmakers** are critical—if one team has all the ball-handlers, it's unfair
- **Runners** dominate because stamina beats positioning when everyone's tired
- **Defenders/Attackers** matter less at this level

The app weighs these when splitting teams, so you don't end up with all the playmakers on one side.

### Skill Ratings (Elo)

Everyone starts at 1200. Win and your rating goes up; lose and it goes down. The amount depends on:

- **Who you beat**: Beating a stronger team = bigger gain
- **Score margin**: A 5-0 win moves ratings more than 2-1

Over time, ratings reflect actual skill. Better players rise, weaker players fall.

---

## Technical Details

### The Algorithm

The app tries every possible way to split players into two teams and picks the "fairest" split.

**Cost function:**

```
cost = |avg_elo_A - avg_elo_B| + sum(tag_penalties)
```

Where `tag_penalty = |count_A - count_B| × weight`

**Default weights:**
| Tag | Weight |
|-----|--------|
| PLAYMAKER | 100 |
| RUNNER | 80 |
| DEF | 40 |
| ATK | 20 |

These are tunable constants at the top of `app.py`.

**GK handling:**

- 2 GKs playing → force one to each team
- 1 GK playing → random assignment (advantage is situational)

### Elo Formula

```python
expected = 1 / (1 + 10^((opponent_elo - my_elo) / 400))
actual = 1 (win), 0.5 (draw), 0 (loss)

gd_multiplier = min(1 + (goal_diff - 1) × 0.5, 2.5)
K = 32

elo_change = K × gd_multiplier × (actual - expected)
```

---

## Setup (Self-Hosting)

### 1. Google Cloud Service Account

1. Go to [Google Cloud Console](https://console.cloud.google.com)
2. Create a project (or use existing)
3. Enable **Google Sheets API**
4. Create credentials → Service Account
5. Download the JSON key

### 2. Google Sheet

Create a sheet with two tabs:

**Players** tab:
| Name | Elo | Tags | Matches_Played |
|------|-----|------|----------------|
| John | 1200 | PLAYMAKER, RUNNER | 0 |

**Matches** tab:
| Date | Team_A | Team_B | Score_A | Score_B |
|------|--------|--------|---------|---------|
| 2024-01-15 | John, Jane | Bob, Alice | 3 | 2 |

Share the sheet with your service account email (Editor access).

### 3. Streamlit Secrets

Create `.streamlit/secrets.toml`:

```toml
sheet_url = "https://docs.google.com/spreadsheets/d/YOUR_SHEET_ID"

[gcp_service_account]
type = "service_account"
project_id = "your-project"
private_key_id = "..."
private_key = "-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----\n"
client_email = "your-bot@your-project.iam.gserviceaccount.com"
client_id = "..."
auth_uri = "https://accounts.google.com/o/oauth2/auth"
token_uri = "https://oauth2.googleapis.com/token"
auth_provider_x509_cert_url = "https://www.googleapis.com/oauth2/v1/certs"
client_x509_cert_url = "..."
```

### 4. Run

```bash
uv sync
uv run streamlit run app.py
```

Or deploy to [Streamlit Community Cloud](https://streamlit.io/cloud) for free hosting.
