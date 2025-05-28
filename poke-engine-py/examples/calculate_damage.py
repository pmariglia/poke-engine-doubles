from poke_engine import calculate_damage

from example_state import state

rolls = calculate_damage(
    state,
    "1",
    "a",
    "2",
    "a",
    "breakingswipe",
    "tackle",
)

print(rolls)
