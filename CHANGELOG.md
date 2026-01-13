# Changelog

## 0.4.0

### Added

- **Shareable team links**: Generated teams now update the URL hash (`#a=1,2,3&b=4,5,6`). Copy and share the link - anyone opening it sees the same teams.
- **Copy link button**: One-click copy of the shareable URL after generating teams
- **Record page pre-fill**: Teams are saved to localStorage, so clicking "Record this match →" auto-fills both teams on the Record page

## 0.3.8

### Added

- **Goal rotation order**: Teams without a dedicated GK now display players as a numbered list sorted by Elo. Higher Elo = later in rotation = less time in goal 🧤

## 0.3.7

### Fixed

- **Player selection cap**: Enforce 14-player max on Teams page (unchecked boxes disable when limit reached)

## 0.3.6

### Changed

- **Code cleanup**: Removed dead code from `db.rs` (unused `get_player` and `get_player_by_name` functions)
- **DRY refactor**: Extracted `render_participation` helper to eliminate duplicated participation display code in `record.rs` and `history.rs`

### Added

- **Unit tests**: Added 10 tests for `models.rs` covering Tag parsing, Player methods, and EloSnapshot serialization

## 0.3.5

### Changed

- **Code cleanup**: Replaced 12 inline styles with 8 reusable CSS classes

## 0.3.4

### Fixed

- **Secure cookies**: Auth cookies now use `secure=true` by default (HTTPS only). Set `SECURE_COOKIES=false` for local dev.

## 0.3.3

### Added

- **Loading spinners**: Visual feedback on Generate Teams, Add Player, and Submit Result buttons

### Fixed

- **Server-side score validation**: Scores now clamped to 0-50 range

## 0.3.2

### Changed

- **Record page UX**: Participation moved to collapsible section; player chips now show just name (no more awkward wrapping)

## 0.3.1

### Fixed

- **Mobile nav alignment**: Navigation button text now vertically centers when buttons wrap to multiple lines
- **Mobile team grid**: Teams stack vertically on mobile screens, side-by-side on desktop (768px+)
- **Performance**: `tag_value()` now parses tag string once instead of 4 times per call
- **Roster table scroll**: Table now scrolls horizontally on mobile instead of causing full-page overflow
- **Compact nav labels**: "Teams", "Roster", "Record", "History" now fit in one row on mobile
- **Header layout**: Login/logout right-aligned below title on mobile, inline on desktop

## 0.3.0

### Changed

- **Team balancing algorithm**: Now uses "player tag value" (sum of tag weights) instead of counting tags individually. This naturally splits multi-tag "star" players between teams.

  - Old: `cost = elo_diff + Σ(|tag_count_A - tag_count_B| × weight)`
  - New: `cost = elo_diff + |team_tag_value_A - team_tag_value_B|`

- **Tag weights halved**: PLAYMAKER 100→50, RUNNER 80→40, DEF 40→20, ATK 20→10. This keeps tag balancing meaningful while ensuring Elo remains the primary factor.

- **Elo snapshot format**: Match history now stores player IDs instead of names in `elo_snapshot`. Migration included for existing data.

## 0.2.0

### Added

- **Injury/participation tracking**: When recording matches, set participation per player (100%, 75%, 50%, 25%). Partial participants receive proportional Elo changes.

- **Handicap system**: Short-handed teams get Elo credit. Each missing player-equivalent = 100 Elo handicap adjustment to expected score.

## 0.1.0

Initial release.

### Added

- **Player management**: Add, edit, and delete players with Elo ratings and role tags
- **Role tags**: PLAYMAKER, RUNNER, DEF, ATK, GK - describe player strengths
- **Team balancing**: Automatically generate fair teams based on Elo and role distribution
- **Elo rating system**: Track player skill over time (K=32, goal difference multiplier)
- **Match recording**: Log match results and update player ratings
- **Match history**: View past matches with Elo changes per player
- **Elo evolution chart**: Visualize rating progression over time
- **Optional authentication**: Password-protect admin actions
