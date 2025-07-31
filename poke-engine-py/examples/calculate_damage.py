from poke_engine import calculate_damage

from example_state import state

max_damage, max_crit_damage = calculate_damage(
    state,
    "1",
    "a",
    "2",
    "a",
    "breakingswipe",
    "tackle",
)

print(f"Max damage: {max_damage}")
print(f"Max crit damage: {max_crit_damage}")
