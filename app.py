"""
Sunday Football Manager - Team Balancer & Elo Tracker
A lightweight Streamlit app for managing 7v7 Sunday League games.
"""

from __future__ import annotations

import json
import random
from dataclasses import dataclass, field
from datetime import date
from itertools import combinations
from typing import TYPE_CHECKING, Any

import gspread
import pandas as pd
import streamlit as st
from google.oauth2.service_account import Credentials

if TYPE_CHECKING:
    from gspread import Spreadsheet, Worksheet

# =============================================================================
# CONFIGURATION - Tune these weights based on your league's meta
# =============================================================================

TAG_WEIGHTS: dict[str, int] = {
    "PLAYMAKER": 100,  # Ball handlers define the game
    "RUNNER": 80,  # Stamina > positioning at this level
    "DEF": 40,  # Solid defense prevents chaos
    "ATK": 20,  # Finishers matter but less than creators
}

ELO_K_FACTOR = 32  # Standard Elo K-factor
ELO_DEFAULT = 1200.0  # Starting Elo for new players
GD_MULTIPLIER_CAP = 2.5  # Max goal difference multiplier (at 5+ goals)
MAX_PLAYERS = 14  # Maximum players for 7v7


# =============================================================================
# DATA MODELS
# =============================================================================


@dataclass
class Player:
    name: str
    elo: float = ELO_DEFAULT
    tags: list[str] = field(default_factory=list)
    matches_played: int = 0
    is_guest: bool = False

    @classmethod
    def from_sheet_row(cls, row: dict[str, Any]) -> Player:
        tags_str = str(row.get("Tags", ""))
        tags = [t.strip().upper() for t in tags_str.split(",") if t.strip()]
        return cls(
            name=str(row.get("Name", "Unknown")),
            elo=float(row.get("Elo", ELO_DEFAULT)),
            tags=tags,
            matches_played=int(row.get("Matches_Played", 0)),
        )

    def has_tag(self, tag: str) -> bool:
        return tag.upper() in self.tags


@dataclass
class Match:
    match_date: str
    team_a: list[str]
    team_b: list[str]
    score_a: int
    score_b: int
    elo_snapshot_a: dict[str, dict[str, float]] = field(default_factory=dict)
    elo_snapshot_b: dict[str, dict[str, float]] = field(default_factory=dict)

    @classmethod
    def from_sheet_row(cls, row: dict[str, Any]) -> Match:
        team_a_str = str(row.get("Team_A", ""))
        team_b_str = str(row.get("Team_B", ""))
        # Parse JSON snapshots, default to empty dict if missing/invalid
        try:
            snapshot_a = json.loads(str(row.get("Elo_Snapshot_A", "{}"))) or {}
        except json.JSONDecodeError:
            snapshot_a = {}
        try:
            snapshot_b = json.loads(str(row.get("Elo_Snapshot_B", "{}"))) or {}
        except json.JSONDecodeError:
            snapshot_b = {}
        return cls(
            match_date=str(row.get("Date", "")),
            team_a=[n.strip() for n in team_a_str.split(",") if n.strip()],
            team_b=[n.strip() for n in team_b_str.split(",") if n.strip()],
            score_a=int(row.get("Score_A", 0)),
            score_b=int(row.get("Score_B", 0)),
            elo_snapshot_a=snapshot_a,
            elo_snapshot_b=snapshot_b,
        )


@dataclass
class TeamSplit:
    team_a: list[Player]
    team_b: list[Player]
    cost: float
    elo_diff: float
    tag_costs: dict[str, int]


# =============================================================================
# GOOGLE SHEETS CONNECTION
# =============================================================================

SCOPES = [
    "https://www.googleapis.com/auth/spreadsheets",
    "https://www.googleapis.com/auth/drive",
]


@st.cache_resource
def get_google_client() -> gspread.Client:
    """Create authenticated Google Sheets client from Streamlit secrets."""
    creds_dict = st.secrets["gcp_service_account"]
    creds = Credentials.from_service_account_info(dict(creds_dict), scopes=SCOPES)
    return gspread.authorize(creds)


def get_spreadsheet() -> Spreadsheet:
    """Get the configured spreadsheet."""
    client = get_google_client()
    sheet_url = st.secrets["sheet_url"]
    return client.open_by_url(sheet_url)


def get_players_worksheet() -> Worksheet:
    return get_spreadsheet().worksheet("Players")


def get_matches_worksheet() -> Worksheet:
    return get_spreadsheet().worksheet("Matches")


def load_players() -> list[Player]:
    """Load all players from the Players sheet."""
    ws = get_players_worksheet()
    records = ws.get_all_records()
    # Filter out rows with empty names
    return [Player.from_sheet_row(row) for row in records if row.get("Name")]


def load_matches() -> list[Match]:
    """Load all matches from the Matches sheet."""
    ws = get_matches_worksheet()
    records = ws.get_all_records()
    return [Match.from_sheet_row(row) for row in records]


def save_player(player: Player) -> None:
    """Add a new player to the sheet and cache."""
    ws = get_players_worksheet()
    ws.append_row([player.name, player.elo, ",".join(player.tags), player.matches_played])
    # Update cache
    if st.session_state.get("players_cache") is not None:
        st.session_state.players_cache.append(player)


def update_player_elo(name: str, new_elo: float, new_matches: int) -> None:
    """Update a player's Elo and match count in the sheet and cache."""
    ws = get_players_worksheet()
    cell = ws.find(name, in_column=1)
    if cell:
        ws.update_cell(cell.row, 2, round(new_elo, 1))
        ws.update_cell(cell.row, 4, new_matches)
    # Update cache
    if st.session_state.get("players_cache") is not None:
        for p in st.session_state.players_cache:
            if p.name == name:
                p.elo = new_elo
                p.matches_played = new_matches
                break


def update_player_full(name: str, new_elo: float, new_tags: str, new_matches: int) -> None:
    """Update all player fields in sheet and cache."""
    ws = get_players_worksheet()
    cell = ws.find(name, in_column=1)
    if cell:
        ws.update_cell(cell.row, 2, round(new_elo, 1))
        ws.update_cell(cell.row, 3, new_tags)
        ws.update_cell(cell.row, 4, new_matches)
    # Update cache
    if st.session_state.get("players_cache") is not None:
        for p in st.session_state.players_cache:
            if p.name == name:
                p.elo = new_elo
                p.tags = [t.strip().upper() for t in new_tags.split(",") if t.strip()]
                p.matches_played = new_matches
                break


def delete_player(name: str) -> None:
    """Delete a player from the sheet and cache."""
    ws = get_players_worksheet()
    cell = ws.find(name, in_column=1)
    if cell:
        ws.delete_rows(cell.row)
    # Update cache
    if st.session_state.get("players_cache") is not None:
        st.session_state.players_cache = [
            p for p in st.session_state.players_cache if p.name != name
        ]


def save_match(match: Match) -> None:
    """Save a match result to the sheet and cache."""
    ws = get_matches_worksheet()
    ws.append_row(
        [
            match.match_date,
            ",".join(match.team_a),
            ",".join(match.team_b),
            match.score_a,
            match.score_b,
            json.dumps(match.elo_snapshot_a),
            json.dumps(match.elo_snapshot_b),
        ]
    )
    # Update cache
    if st.session_state.get("matches_cache") is not None:
        st.session_state.matches_cache.append(match)


# =============================================================================
# TEAM BALANCING ALGORITHM
# =============================================================================


def count_tags(players: list[Player], tag: str) -> int:
    """Count how many players have a specific tag."""
    return sum(1 for p in players if p.has_tag(tag))


def average_elo(players: list[Player]) -> float:
    """Calculate average Elo of a team."""
    if not players:
        return 0.0
    return sum(p.elo for p in players) / len(players)


def calculate_split_cost(team_a: list[Player], team_b: list[Player]) -> TeamSplit:
    """Calculate the cost of a team split."""
    elo_a = average_elo(team_a)
    elo_b = average_elo(team_b)
    elo_diff = abs(elo_a - elo_b)

    tag_costs: dict[str, int] = {}
    total_tag_cost = 0.0

    for tag, weight in TAG_WEIGHTS.items():
        count_a = count_tags(team_a, tag)
        count_b = count_tags(team_b, tag)
        diff = abs(count_a - count_b)
        tag_costs[tag] = diff
        total_tag_cost += diff * weight

    total_cost = elo_diff + total_tag_cost

    return TeamSplit(
        team_a=team_a,
        team_b=team_b,
        cost=total_cost,
        elo_diff=elo_diff,
        tag_costs=tag_costs,
    )


def balance_teams(players: list[Player]) -> TeamSplit | None:
    """
    Find the optimal team split using brute force.
    Handles GK special logic:
    - 2 GKs: Force split (one per team)
    - 1 GK: Random assignment
    - 0 GKs: Normal balancing
    """
    if len(players) < 2:
        return None

    if len(players) % 2 != 0:
        st.warning("Odd number of players - one team will have an extra player")

    team_size = len(players) // 2

    # Identify goalkeepers
    gks = [p for p in players if p.has_tag("GK")]
    non_gks = [p for p in players if not p.has_tag("GK")]

    best_split: TeamSplit | None = None

    if len(gks) == 2:
        # Force split: one GK per team
        gk_a, gk_b = gks[0], gks[1]
        remaining_size = team_size - 1

        if remaining_size < 0 or len(non_gks) < remaining_size * 2:
            # Not enough non-GK players, fall back to normal balancing
            gks = []
            non_gks = players
        else:
            for combo in combinations(non_gks, remaining_size):
                team_a = [gk_a, *list(combo)]
                team_b = [gk_b, *[p for p in non_gks if p not in combo]]
                split = calculate_split_cost(team_a, team_b)
                if best_split is None or split.cost < best_split.cost:
                    best_split = split
            return best_split

    elif len(gks) == 1:
        # Random assignment for single GK
        gk = gks[0]
        gk_team = random.choice(["A", "B"])

        for combo in combinations(non_gks, team_size - 1):
            if gk_team == "A":
                team_a = [gk, *list(combo)]
                team_b = [p for p in non_gks if p not in combo]
            else:
                team_a = list(combo)
                team_b = [gk, *[p for p in non_gks if p not in combo]]

            # Ensure teams are balanced in size
            if abs(len(team_a) - len(team_b)) > 1:
                continue

            split = calculate_split_cost(team_a, team_b)
            if best_split is None or split.cost < best_split.cost:
                best_split = split

        return best_split

    # No GK special logic - standard brute force
    for combo in combinations(players, team_size):
        team_a = list(combo)
        team_b = [p for p in players if p not in combo]
        split = calculate_split_cost(team_a, team_b)
        if best_split is None or split.cost < best_split.cost:
            best_split = split

    return best_split


# =============================================================================
# ELO CALCULATIONS
# =============================================================================


def expected_score(elo_a: float, elo_b: float) -> float:
    """Calculate expected score for team A."""
    return 1 / (1 + 10 ** ((elo_b - elo_a) / 400))


def goal_diff_multiplier(goal_diff: int) -> float:
    """Calculate multiplier based on goal difference (capped at 5)."""
    if goal_diff <= 1:
        return 1.0
    return min(1 + (goal_diff - 1) * 0.5, GD_MULTIPLIER_CAP)


def calculate_elo_changes(
    team_a: list[Player],
    team_b: list[Player],
    score_a: int,
    score_b: int,
) -> dict[str, float]:
    """
    Calculate Elo changes for all players.
    Returns dict mapping player name to Elo change.
    """
    elo_a = average_elo(team_a)
    elo_b = average_elo(team_b)

    expected_a = expected_score(elo_a, elo_b)

    if score_a > score_b:
        actual_a = 1.0
    elif score_a < score_b:
        actual_a = 0.0
    else:
        actual_a = 0.5

    gd = abs(score_a - score_b)
    multiplier = goal_diff_multiplier(gd)

    delta_a = ELO_K_FACTOR * multiplier * (actual_a - expected_a)
    delta_b = -delta_a  # Zero-sum

    changes: dict[str, float] = {}
    for p in team_a:
        if not p.is_guest:
            changes[p.name] = delta_a
    for p in team_b:
        if not p.is_guest:
            changes[p.name] = delta_b

    return changes


# =============================================================================
# STREAMLIT UI
# =============================================================================


def init_session_state() -> None:
    """Initialize session state variables."""
    if "checked_players" not in st.session_state:
        st.session_state.checked_players = set()
    if "guests" not in st.session_state:
        st.session_state.guests = []
    if "current_split" not in st.session_state:
        st.session_state.current_split = None
    if "match_recorded" not in st.session_state:
        st.session_state.match_recorded = False
    # Data cache to avoid API rate limits
    if "players_cache" not in st.session_state:
        st.session_state.players_cache = None
    if "matches_cache" not in st.session_state:
        st.session_state.matches_cache = None


def get_players() -> list[Player]:
    """Get players from cache, loading from sheet if needed."""
    if st.session_state.players_cache is None:
        st.session_state.players_cache = load_players()
    return st.session_state.players_cache


def get_matches() -> list[Match]:
    """Get matches from cache, loading from sheet if needed."""
    if st.session_state.matches_cache is None:
        st.session_state.matches_cache = load_matches()
    return st.session_state.matches_cache


def refresh_data() -> None:
    """Force reload data from Google Sheets."""
    st.session_state.players_cache = load_players()
    st.session_state.matches_cache = load_matches()


def page_match_day() -> None:
    """Match Day page - check-in and team balancing."""
    st.header("Match Day")

    try:
        players = get_players()
    except Exception as e:
        st.error(f"Failed to load players: {e}")
        st.info("Make sure your Google Sheet is set up correctly with a 'Players' tab.")
        return

    # Player check-in
    st.subheader("Player Check-In")

    if not players:
        st.warning("No players in database. Add players in the Roster page.")
        return

    # Sort players by name
    players.sort(key=lambda p: p.name.lower())

    # Calculate current selection count
    num_checked = len(st.session_state.checked_players)
    num_guests = len(st.session_state.guests)
    total_selected = num_checked + num_guests
    at_capacity = total_selected >= MAX_PLAYERS

    if at_capacity:
        st.warning(f"Maximum {MAX_PLAYERS} players reached. Remove someone to add more.")

    # Create checkbox grid
    cols = st.columns(3)
    for i, player in enumerate(players):
        with cols[i % 3]:
            is_checked = player.name in st.session_state.checked_players
            # Disable unchecked boxes when at capacity
            disabled = at_capacity and not is_checked
            checked = st.checkbox(
                f"{player.name} ({player.elo:.0f})",
                key=f"check_{player.name}",
                value=is_checked,
                disabled=disabled,
            )
            if checked and not is_checked:
                st.session_state.checked_players.add(player.name)
            elif not checked and is_checked:
                st.session_state.checked_players.discard(player.name)

    # Guest players section
    with st.expander("Add Guest Player"), st.form("guest_form", clear_on_submit=True):
        col1, col2 = st.columns([2, 1])
        with col1:
            guest_name = st.text_input("Guest Name")
        with col2:
            guest_skill = st.slider("Skill Level", 1000, 1400, 1200)

        guest_tags = st.multiselect(
            "Tags (optional)",
            options=list(TAG_WEIGHTS.keys()) + ["GK"],
        )

        if st.form_submit_button("Add Guest") and guest_name:
            if at_capacity:
                st.error(f"Cannot add guest: maximum {MAX_PLAYERS} players reached.")
            else:
                guest = Player(
                    name=f"[G] {guest_name}",
                    elo=float(guest_skill),
                    tags=guest_tags,
                    is_guest=True,
                )
                st.session_state.guests.append(guest)
                st.success(f"Added guest: {guest.name}")
                st.rerun()

    # Show current guests
    if st.session_state.guests:
        st.write("**Current Guests:**")
        for i, guest in enumerate(st.session_state.guests):
            col1, col2 = st.columns([3, 1])
            with col1:
                st.write(f"- {guest.name} (Elo: {guest.elo:.0f}, Tags: {', '.join(guest.tags)})")
            with col2:
                if st.button("Remove", key=f"remove_guest_{i}"):
                    st.session_state.guests.pop(i)
                    st.rerun()

    st.divider()

    # Team generation
    checked_names = st.session_state.checked_players
    checked_players = [p for p in players if p.name in checked_names]
    all_players = checked_players + st.session_state.guests

    st.write(f"**Players selected: {len(all_players)}/{MAX_PLAYERS}**")

    if len(all_players) >= 2:
        col1, col2 = st.columns(2)
        with col1:
            if st.button("Generate Teams", type="primary"):
                split = balance_teams(all_players)
                if split:
                    st.session_state.current_split = split
                    st.session_state.match_recorded = False
                else:
                    st.error("Could not generate teams")

        with col2:
            if st.button("Shuffle (Re-roll)"):
                # Re-run the balancer (randomness in GK assignment will vary)
                split = balance_teams(all_players)
                if split:
                    st.session_state.current_split = split
                    st.session_state.match_recorded = False

    # Display generated teams
    split = st.session_state.current_split
    if split:
        st.subheader("Generated Teams")

        col1, col2 = st.columns(2)

        with col1:
            st.markdown("### Team A")
            team_a_elo = average_elo(split.team_a)
            st.write(f"**Avg Elo: {team_a_elo:.0f}**")
            for p in sorted(split.team_a, key=lambda x: -x.elo):
                tags_str = f" [{', '.join(p.tags)}]" if p.tags else ""
                st.write(f"- {p.name} ({p.elo:.0f}){tags_str}")

        with col2:
            st.markdown("### Team B")
            team_b_elo = average_elo(split.team_b)
            st.write(f"**Avg Elo: {team_b_elo:.0f}**")
            for p in sorted(split.team_b, key=lambda x: -x.elo):
                tags_str = f" [{', '.join(p.tags)}]" if p.tags else ""
                st.write(f"- {p.name} ({p.elo:.0f}){tags_str}")

        # Cost breakdown
        with st.expander("Balance Details"):
            st.write(f"**Total Cost:** {split.cost:.1f}")
            st.write(f"**Elo Difference:** {split.elo_diff:.1f}")
            st.write("**Tag Imbalances:**")
            for tag, diff in split.tag_costs.items():
                if diff > 0:
                    st.write(f"- {tag}: {diff} (penalty: {diff * TAG_WEIGHTS[tag]})")
    else:
        st.info("Select players and click 'Generate Teams' to balance")


def page_record_result() -> None:
    """Record Result page - input scores and update Elo."""
    st.header("Record Match Result")

    # Load players
    try:
        players = get_players()
    except Exception as e:
        st.error(f"Failed to load players: {e}")
        return

    # Include guests from session state
    if "guests" not in st.session_state:
        st.session_state.guests = []
    guests: list[Player] = st.session_state.guests
    all_players = players + guests

    # Add guest form
    with st.expander("Add Guest Player"):
        col1, col2 = st.columns([2, 1])
        with col1:
            guest_name = st.text_input("Guest Name", key="record_guest_name")
        with col2:
            guest_skill = st.slider("Skill", 1000, 1400, 1200, key="record_guest_skill")
        if st.button("Add Guest", key="record_add_guest") and guest_name:
            guest = Player(
                name=f"[G] {guest_name}",
                elo=float(guest_skill),
                tags=[],
                is_guest=True,
            )
            st.session_state.guests.append(guest)
            st.rerun()

    if not all_players:
        st.warning("No players available. Add players in Roster or add a guest above.")
        return

    player_names = sorted([p.name for p in all_players])
    player_map = {p.name: p for p in all_players}

    # Check for generated teams
    split = st.session_state.get("current_split")
    has_generated = split is not None and not st.session_state.get("match_recorded")

    if has_generated and split is not None:
        st.info("Using generated teams. You can modify the selections below if needed.")
        default_a = [p.name for p in split.team_a if p.name in player_map]
        default_b = [p.name for p in split.team_b if p.name in player_map]
    else:
        default_a = []
        default_b = []

    # Team selection
    st.subheader("Select Teams")

    col1, col2 = st.columns(2)
    with col1:
        team_a_names = st.multiselect(
            "Team A",
            options=player_names,
            default=default_a,
            key="record_team_a",
        )
    with col2:
        team_b_names = st.multiselect(
            "Team B",
            options=player_names,
            default=default_b,
            key="record_team_b",
        )

    # Validation
    overlap = set(team_a_names) & set(team_b_names)
    if overlap:
        st.error(f"Players cannot be on both teams: {', '.join(overlap)}")
        return

    if not team_a_names or not team_b_names:
        st.warning("Select players for both teams to record a result.")
        return

    # Convert to Player objects
    team_a = [player_map[name] for name in team_a_names]
    team_b = [player_map[name] for name in team_b_names]

    # Show team summaries
    col1, col2 = st.columns(2)
    with col1:
        st.write(f"**Team A** ({len(team_a)} players, avg Elo: {average_elo(team_a):.0f})")
    with col2:
        st.write(f"**Team B** ({len(team_b)} players, avg Elo: {average_elo(team_b):.0f})")

    st.divider()

    # Score input
    st.subheader("Enter Score")

    col1, col2, col3 = st.columns([2, 1, 2])
    with col1:
        score_a = st.number_input("Team A", min_value=0, max_value=50, value=0)
    with col2:
        st.markdown("<h2 style='text-align: center'>-</h2>", unsafe_allow_html=True)
    with col3:
        score_b = st.number_input("Team B", min_value=0, max_value=50, value=0)

    if st.button("Submit Result", type="primary"):
        # Calculate Elo changes
        changes = calculate_elo_changes(team_a, team_b, score_a, score_b)

        # Display changes
        st.subheader("Elo Changes")

        col1, col2 = st.columns(2)
        with col1:
            st.write("**Team A:**")
            for p in team_a:
                if p.name in changes:
                    delta = changes[p.name]
                    sign = "+" if delta >= 0 else ""
                    st.write(f"- {p.name}: {sign}{delta:.1f}")

        with col2:
            st.write("**Team B:**")
            for p in team_b:
                if p.name in changes:
                    delta = changes[p.name]
                    sign = "+" if delta >= 0 else ""
                    st.write(f"- {p.name}: {sign}{delta:.1f}")

        # Save to database
        try:
            # Build Elo snapshots before updating
            snapshot_a: dict[str, dict[str, float]] = {}
            snapshot_b: dict[str, dict[str, float]] = {}
            for p in team_a:
                if p.name in changes:
                    snapshot_a[p.name] = {"before": p.elo, "delta": changes[p.name]}
            for p in team_b:
                if p.name in changes:
                    snapshot_b[p.name] = {"before": p.elo, "delta": changes[p.name]}

            # Update player Elos
            for name, delta in changes.items():
                if name in player_map:
                    p = player_map[name]
                    new_elo = p.elo + delta
                    new_matches = p.matches_played + 1
                    update_player_elo(name, new_elo, new_matches)

            # Save match record with snapshots
            match = Match(
                match_date=date.today().isoformat(),
                team_a=[p.name for p in team_a],
                team_b=[p.name for p in team_b],
                score_a=score_a,
                score_b=score_b,
                elo_snapshot_a=snapshot_a,
                elo_snapshot_b=snapshot_b,
            )
            save_match(match)

            st.session_state.match_recorded = True
            st.success("Match recorded and Elos updated!")
            st.rerun()

        except Exception as e:
            st.error(f"Failed to save: {e}")


def page_roster() -> None:
    """Roster Management page."""
    st.header("Roster Management")

    try:
        players = get_players()
    except Exception as e:
        st.error(f"Failed to load players: {e}")
        return

    # Add new player
    st.subheader("Add New Player")
    with st.form("add_player_form", clear_on_submit=True):
        col1, col2 = st.columns([2, 1])
        with col1:
            new_name = st.text_input("Name")
        with col2:
            new_elo = st.number_input(
                "Starting Elo", value=int(ELO_DEFAULT), min_value=800, max_value=2000
            )

        new_tags = st.multiselect("Tags", options=list(TAG_WEIGHTS.keys()) + ["GK"])

        if st.form_submit_button("Add Player") and new_name:
            existing_names = {p.name.lower() for p in players}
            if new_name.lower() in existing_names:
                st.error("Player already exists!")
            else:
                player = Player(
                    name=new_name,
                    elo=float(new_elo),
                    tags=new_tags,
                    matches_played=0,
                )
                save_player(player)
                st.success(f"Added {new_name}!")
                st.rerun()

    st.divider()

    # View/Edit players
    st.subheader("Current Roster")

    if not players:
        st.info("No players yet. Add your first player above!")
        return

    # Sort by Elo descending
    players.sort(key=lambda p: -p.elo)

    # Create DataFrame for display
    df = pd.DataFrame(
        [
            {
                "Name": p.name,
                "Elo": f"{p.elo:.0f}",
                "Tags": ", ".join(p.tags),
                "Matches": p.matches_played,
            }
            for p in players
        ]
    )

    st.dataframe(df, width="stretch", hide_index=True)

    # Edit/Delete section
    st.subheader("Edit Player")
    player_names = [p.name for p in players]
    selected_name = st.selectbox("Select Player", options=player_names)

    if selected_name:
        selected_player = next(p for p in players if p.name == selected_name)

        with st.form("edit_player_form"):
            edit_elo = st.number_input(
                "Elo",
                value=int(selected_player.elo),
                min_value=800,
                max_value=2000,
            )
            edit_tags = st.multiselect(
                "Tags",
                options=list(TAG_WEIGHTS.keys()) + ["GK"],
                default=selected_player.tags,
            )

            col1, col2 = st.columns(2)
            with col1:
                if st.form_submit_button("Update"):
                    update_player_full(
                        selected_name,
                        float(edit_elo),
                        ",".join(edit_tags),
                        selected_player.matches_played,
                    )
                    st.success(f"Updated {selected_name}!")
                    st.rerun()

            with col2:
                if st.form_submit_button("Delete", type="secondary"):
                    delete_player(selected_name)
                    st.success(f"Deleted {selected_name}!")
                    st.rerun()


def page_history() -> None:
    """Match History page."""
    st.header("Match History")

    try:
        matches = get_matches()
    except Exception as e:
        st.error(f"Failed to load matches: {e}")
        return

    if not matches:
        st.info("No matches recorded yet.")
        return

    # Build Elo history from snapshots for chart
    elo_history: list[dict[str, Any]] = []
    for match in matches:
        match_date = match.match_date
        # Process team A snapshots
        for name, snapshot in match.elo_snapshot_a.items():
            if isinstance(snapshot, dict) and "before" in snapshot and "delta" in snapshot:
                elo_history.append({
                    "date": match_date,
                    "player": name,
                    "elo": snapshot["before"] + snapshot["delta"],
                })
        # Process team B snapshots
        for name, snapshot in match.elo_snapshot_b.items():
            if isinstance(snapshot, dict) and "before" in snapshot and "delta" in snapshot:
                elo_history.append({
                    "date": match_date,
                    "player": name,
                    "elo": snapshot["before"] + snapshot["delta"],
                })

    # Show Elo progression chart if we have data
    if elo_history:
        st.subheader("Elo Progression")
        df = pd.DataFrame(elo_history)

        # Player filter
        all_players = sorted(df["player"].unique())
        selected_players = st.multiselect(
            "Filter players",
            options=all_players,
            default=all_players,
            key="history_player_filter",
        )

        if selected_players:
            filtered_df = df[df["player"].isin(selected_players)]
            # Pivot for line chart: dates as index, players as columns
            pivot_df = filtered_df.pivot_table(
                index="date", columns="player", values="elo", aggfunc="last"
            )
            st.line_chart(pivot_df)

    st.divider()
    st.subheader("Match Log")

    # Reverse to show most recent first
    matches_display = list(reversed(matches))

    for match in matches_display:
        result_emoji = ""
        if match.score_a > match.score_b:
            result_emoji = "Team A wins"
        elif match.score_b > match.score_a:
            result_emoji = "Team B wins"
        else:
            result_emoji = "Draw"

        with st.expander(
            f"{match.match_date} - {match.score_a} : {match.score_b} ({result_emoji})"
        ):
            col1, col2 = st.columns(2)
            with col1:
                st.write("**Team A:**")
                for name in match.team_a:
                    snapshot = match.elo_snapshot_a.get(name, {})
                    if isinstance(snapshot, dict) and "before" in snapshot and "delta" in snapshot:
                        delta = snapshot["delta"]
                        sign = "+" if delta >= 0 else ""
                        st.write(f"- {name} ({snapshot['before']:.0f} {sign}{delta:.0f})")
                    else:
                        st.write(f"- {name}")
            with col2:
                st.write("**Team B:**")
                for name in match.team_b:
                    snapshot = match.elo_snapshot_b.get(name, {})
                    if isinstance(snapshot, dict) and "before" in snapshot and "delta" in snapshot:
                        delta = snapshot["delta"]
                        sign = "+" if delta >= 0 else ""
                        st.write(f"- {name} ({snapshot['before']:.0f} {sign}{delta:.0f})")
                    else:
                        st.write(f"- {name}")


def main() -> None:
    """Main app entry point."""
    st.set_page_config(
        page_title="Sunday Football Manager",
        page_icon="⚽",
        layout="wide",
    )

    init_session_state()

    st.title("Sunday Football Manager")

    # Sidebar navigation
    page = st.sidebar.radio(
        "Navigation",
        options=["Match Day", "Record Result", "Roster", "History"],
    )

    # Refresh button
    st.sidebar.divider()
    if st.sidebar.button("Refresh Data"):
        with st.spinner("Refreshing..."):
            refresh_data()
        st.sidebar.success("Data refreshed!")

    # Check for required secrets
    if "gcp_service_account" not in st.secrets or "sheet_url" not in st.secrets:
        st.error("Missing configuration! Please set up your Streamlit secrets.")
        st.markdown("""
        ### Setup Instructions

        1. Create a Google Cloud service account and download the JSON key
        2. Create a Google Sheet with two tabs: `Players` and `Matches`
        3. Share the sheet with your service account email
        4. Add secrets to `.streamlit/secrets.toml`:

        ```toml
        sheet_url = "https://docs.google.com/spreadsheets/d/YOUR_SHEET_ID"

        [gcp_service_account]
        type = "service_account"
        project_id = "your-project"
        private_key_id = "..."
        private_key = "-----BEGIN PRIVATE KEY-----\\n...\\n-----END PRIVATE KEY-----\\n"
        client_email = "your-service-account@your-project.iam.gserviceaccount.com"
        client_id = "..."
        auth_uri = "https://accounts.google.com/o/oauth2/auth"
        token_uri = "https://oauth2.googleapis.com/token"
        auth_provider_x509_cert_url = "https://www.googleapis.com/oauth2/v1/certs"
        client_x509_cert_url = "..."
        ```

        **Google Sheet Format:**

        **Players tab columns:** Name, Elo, Tags, Matches_Played

        **Matches tab columns:** Date, Team_A, Team_B, Score_A, Score_B
        """)
        return

    # Render selected page
    if page == "Match Day":
        page_match_day()
    elif page == "Record Result":
        page_record_result()
    elif page == "Roster":
        page_roster()
    elif page == "History":
        page_history()


if __name__ == "__main__":
    main()
