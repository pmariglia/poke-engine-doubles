from poke_engine import (
    monte_carlo_tree_search_team_preview,
    TeamPreviewFilters,
)

from example_state import state

state.team_preview = True

team_preview_filters = TeamPreviewFilters(
    side_one=[(0, 1, 2, 3), (0, 3, 2, 1)],
    side_two=[(0, 1, 2, 3), (0, 2, 3, 1)],
)


result = monte_carlo_tree_search_team_preview(
    state, team_preview_filters, duration_ms=1000
)
print(f"Total Iterations: {result.total_visits}")
print([(i.move_choice, i.total_score, i.visits) for i in result.side_one])
print([(i.move_choice, i.total_score, i.visits) for i in result.side_two])
