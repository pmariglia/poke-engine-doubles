use poke_engine::pokemon::PokemonName;
use poke_engine::state::{Pokemon, PokemonNature};

#[test]
fn test_calculate_stats_neutral_nature() {
    let mut pokemon = Pokemon::default();
    pokemon.nature = PokemonNature::SERIOUS; // neutral
    pokemon.evs = (11, 11, 11, 11, 11, 11);
    pokemon.id = PokemonName::VENUSAURMEGA;
    let (hp, attack, defense, special_attack, special_defense, speed) =
        pokemon.calculate_stats_from_base_stats();

    assert_eq!(hp, 166);
    assert_eq!(attack, 131);
    assert_eq!(defense, 154);
    assert_eq!(special_attack, 153);
    assert_eq!(special_defense, 151);
    assert_eq!(speed, 111);
}

#[test]
fn test_calculate_stats_with_boosting_nature() {
    let mut pokemon = Pokemon::default();
    pokemon.nature = PokemonNature::MODEST; // boosts spa and lowers atk
    pokemon.evs = (11, 11, 11, 11, 11, 11);
    pokemon.id = PokemonName::VENUSAURMEGA;
    let (hp, attack, defense, special_attack, special_defense, speed) =
        pokemon.calculate_stats_from_base_stats();

    assert_eq!(hp, 166);
    assert_eq!(attack, 117); // lowered
    assert_eq!(defense, 154);
    assert_eq!(special_attack, 168); // boosted
    assert_eq!(special_defense, 151);
    assert_eq!(speed, 111);
}
