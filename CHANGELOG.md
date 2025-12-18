# Changelog

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
