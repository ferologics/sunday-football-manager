# Changelog

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
