use poke_engine::choices::{Choice, Choices, MOVES};
use poke_engine::engine::abilities::Abilities;
use poke_engine::engine::generate_instructions::calculate_damage_rolls;
use poke_engine::engine::items::Items;
use poke_engine::engine::state::Weather;
use poke_engine::pokemon::PokemonName;
use poke_engine::state::{PokemonType, SideReference, SlotReference, State};

#[test]
fn test_basic_damage_calculation() {
    let mut state = State::default();

    let choice = MOVES.get(&Choices::TACKLE).unwrap().clone();

    let damage_rolls = calculate_damage_rolls(
        &mut state,
        &SideReference::SideOne,
        &SlotReference::SlotA,
        &SideReference::SideTwo,
        &SlotReference::SlotA,
        choice,
        &Choice::default(),
    );

    assert_eq!(damage_rolls, Some(vec![52, 78]));
}

#[test]
fn test_gives_no_damage_when_target_is_immune() {
    let mut state = State::default();
    state.side_two.pokemon.p1.types.0 = PokemonType::GHOST;

    let choice = MOVES.get(&Choices::TACKLE).unwrap().clone();

    let damage_rolls = calculate_damage_rolls(
        &mut state,
        &SideReference::SideOne,
        &SlotReference::SlotA,
        &SideReference::SideTwo,
        &SlotReference::SlotB,
        choice,
        &Choice::default(),
    );

    assert_eq!(damage_rolls, Some(vec![0, 0]));
}

#[test]
fn test_seismictoss_does_static_damage() {
    let mut state = State::default();

    let choice = MOVES.get(&Choices::SEISMICTOSS).unwrap().clone();

    let damage_rolls = calculate_damage_rolls(
        &mut state,
        &SideReference::SideOne,
        &SlotReference::SlotA,
        &SideReference::SideTwo,
        &SlotReference::SlotA,
        choice,
        &Choice::default(),
    );

    assert_eq!(damage_rolls, Some(vec![100]));
}

#[test]
fn test_spread_move_damage_reduction() {
    let mut state = State::default();

    let choice = MOVES.get(&Choices::BREAKINGSWIPE).unwrap().clone();

    let damage_rolls = calculate_damage_rolls(
        &mut state,
        &SideReference::SideOne,
        &SlotReference::SlotA,
        &SideReference::SideTwo,
        &SlotReference::SlotA,
        choice,
        &Choice::default(),
    );

    assert_eq!(damage_rolls, Some(vec![39, 58]));
}

#[test]
fn test_spread_move_no_damage_reduction_if_single_target() {
    let mut state = State::default();
    state.side_two.pokemon.p1.hp = 0;

    let choice = MOVES.get(&Choices::BREAKINGSWIPE).unwrap().clone();

    let damage_rolls = calculate_damage_rolls(
        &mut state,
        &SideReference::SideOne,
        &SlotReference::SlotA,
        &SideReference::SideTwo,
        &SlotReference::SlotA,
        choice,
        &Choice::default(),
    );

    assert_eq!(damage_rolls, Some(vec![52, 78]));
}

#[test]
fn test_ivycudgel_changing_type() {
    // This was from a fuzz failure:
    // ogerponhearthflame using ivycudgel on a terastallized fire type means the damage is resisted
    let mut state = State::default();
    state.weather.weather_type = Weather::SUN;
    state.weather.turns_remaining = 3;
    state.side_one.pokemon.p1.level = 50;
    state.side_one.pokemon.p1.types = (PokemonType::GRASS, PokemonType::FIRE);
    state.side_one.pokemon.p1.ability = Abilities::MOLDBREAKER;
    state.side_one.pokemon.p1.item = Items::HEARTHFLAMEMASK;
    state.side_one.pokemon.p1.id = PokemonName::OGERPONHEARTHFLAME;
    state.side_one.pokemon.p1.attack = 181;

    state.side_one.pokemon.p0.level = 50;
    state.side_two.pokemon.p0.terastallized = true;
    state.side_two.pokemon.p0.tera_type = PokemonType::FIRE;
    state.side_two.pokemon.p0.hp = 207;
    state.side_two.pokemon.p0.defense = 160;

    let choice = MOVES.get(&Choices::IVYCUDGEL).unwrap().clone();

    let damage_rolls = calculate_damage_rolls(
        &mut state,
        &SideReference::SideOne,
        &SlotReference::SlotB,
        &SideReference::SideTwo,
        &SlotReference::SlotA,
        choice,
        &Choice::default(),
    );

    assert_eq!(damage_rolls, Some(vec![68, 102]));
}
