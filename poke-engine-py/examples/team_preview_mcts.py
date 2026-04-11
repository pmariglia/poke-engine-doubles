from poke_engine import (
    monte_carlo_tree_search_team_preview,
    TeamPreviewFilters,
    TeamPreviewFilterSide,
)

from example_state import state

state.team_preview = True

team_preview_filter_s1 = TeamPreviewFilterSide(
    valid_pokemon=[
        state.side_one.pokemon[0].id,
        state.side_one.pokemon[1].id,
        state.side_one.pokemon[2].id,
        state.side_one.pokemon[3].id,
    ],
    forced_leads=[
        (
            "charmander",
            "squirtle",
        ),
        (
            "charmander",
            "pikachu",
        ),
    ],
)
team_preview_filter_s2 = TeamPreviewFilterSide(
    valid_pokemon=[
        state.side_two.pokemon[0].id,
        state.side_two.pokemon[1].id,
        state.side_two.pokemon[2].id,
        state.side_two.pokemon[3].id,
    ],
    forced_leads=[
        (
            "charmander",
            "squirtle",
        ),
        (
            "charmander",
            "bulbasaur",
        ),
    ],
)
team_preview_filters = TeamPreviewFilters(
    side_one=team_preview_filter_s1,
    side_two=team_preview_filter_s2,
)


result = monte_carlo_tree_search_team_preview(
    state, team_preview_filters, duration_ms=1000
)
print(f"Total Iterations: {result.total_visits}")
print([(i.move_choice, i.total_score, i.visits) for i in result.side_one])
print([(i.move_choice, i.total_score, i.visits) for i in result.side_two])
