use super::abilities::Abilities;
use super::items::Items;
use super::state::PokemonVolatileStatus;
use crate::choices::MoveCategory;
use crate::engine::generate_instructions::get_effective_speed;
use crate::state::{Pokemon, PokemonStatus, Side, SideReference, SideSlot, SlotReference, State};

const POKEMON_ALIVE: f32 = 30.0;
const POKEMON_HP: f32 = 100.0;
const USED_TERA: f32 = -50.0;

const POKEMON_ATTACK_BOOST: f32 = 30.0;
const POKEMON_DEFENSE_BOOST: f32 = 15.0;
const POKEMON_SPECIAL_ATTACK_BOOST: f32 = 30.0;
const POKEMON_SPECIAL_DEFENSE_BOOST: f32 = 15.0;
const POKEMON_SPEED_BOOST: f32 = 30.0;

const POKEMON_BOOST_MULTIPLIER_6: f32 = 3.3;
const POKEMON_BOOST_MULTIPLIER_5: f32 = 3.15;
const POKEMON_BOOST_MULTIPLIER_4: f32 = 3.0;
const POKEMON_BOOST_MULTIPLIER_3: f32 = 2.5;
const POKEMON_BOOST_MULTIPLIER_2: f32 = 2.0;
const POKEMON_BOOST_MULTIPLIER_1: f32 = 1.0;
const POKEMON_BOOST_MULTIPLIER_0: f32 = 0.0;
const POKEMON_BOOST_MULTIPLIER_NEG_1: f32 = -1.0;
const POKEMON_BOOST_MULTIPLIER_NEG_2: f32 = -2.0;
const POKEMON_BOOST_MULTIPLIER_NEG_3: f32 = -2.5;
const POKEMON_BOOST_MULTIPLIER_NEG_4: f32 = -3.0;
const POKEMON_BOOST_MULTIPLIER_NEG_5: f32 = -3.15;
const POKEMON_BOOST_MULTIPLIER_NEG_6: f32 = -3.3;

const POKEMON_FROZEN: f32 = -40.0;
const POKEMON_ASLEEP: f32 = -45.0;
const POKEMON_PARALYZED: f32 = -25.0;
const POKEMON_TOXIC: f32 = -30.0;
const POKEMON_POISONED: f32 = -10.0;
const POKEMON_BURNED: f32 = -25.0;

const LEECH_SEED: f32 = -30.0;
const SUBSTITUTE: f32 = 40.0;
const CONFUSION: f32 = -20.0;
const PERISH3: f32 = -15.0;
const PERISH2: f32 = -30.0;
const PERISH1: f32 = -45.0;

const REFLECT: f32 = 20.0;
const LIGHT_SCREEN: f32 = 20.0;
const AURORA_VEIL: f32 = 40.0;
const SAFE_GUARD: f32 = 5.0;
const TAILWIND: f32 = 5.0;
const HEALING_WISH: f32 = 30.0;

const STEALTH_ROCK: f32 = -10.0;
const SPIKES: f32 = -7.0;
const TOXIC_SPIKES: f32 = -7.0;
const STICKY_WEB: f32 = -25.0;

const FASTER_THAN_OPPONENT: f32 = 5.0;

fn evaluate_poison(pokemon: &Pokemon, base_score: f32) -> f32 {
    match pokemon.ability {
        Abilities::POISONHEAL => 15.0,
        Abilities::GUTS
        | Abilities::MARVELSCALE
        | Abilities::QUICKFEET
        | Abilities::TOXICBOOST
        | Abilities::MAGICGUARD => 10.0,
        _ => base_score,
    }
}

fn evaluate_burned(pokemon: &Pokemon) -> f32 {
    // burn is not as punishing in certain situations

    // guts, marvel scale, quick feet will result in a positive evaluation
    match pokemon.ability {
        Abilities::GUTS | Abilities::MARVELSCALE | Abilities::QUICKFEET => {
            return -2.0 * POKEMON_BURNED
        }
        _ => {}
    }

    let mut multiplier = 0.0;
    for mv in pokemon.moves.into_iter() {
        if mv.choice.category == MoveCategory::Physical {
            multiplier += 1.0;
        }
    }

    // don't make burn as punishing for special attackers
    if pokemon.special_attack > pokemon.attack {
        multiplier /= 2.0;
    }

    multiplier * POKEMON_BURNED
}

fn get_boost_multiplier(boost: i8) -> f32 {
    match boost {
        6 => POKEMON_BOOST_MULTIPLIER_6,
        5 => POKEMON_BOOST_MULTIPLIER_5,
        4 => POKEMON_BOOST_MULTIPLIER_4,
        3 => POKEMON_BOOST_MULTIPLIER_3,
        2 => POKEMON_BOOST_MULTIPLIER_2,
        1 => POKEMON_BOOST_MULTIPLIER_1,
        0 => POKEMON_BOOST_MULTIPLIER_0,
        -1 => POKEMON_BOOST_MULTIPLIER_NEG_1,
        -2 => POKEMON_BOOST_MULTIPLIER_NEG_2,
        -3 => POKEMON_BOOST_MULTIPLIER_NEG_3,
        -4 => POKEMON_BOOST_MULTIPLIER_NEG_4,
        -5 => POKEMON_BOOST_MULTIPLIER_NEG_5,
        -6 => POKEMON_BOOST_MULTIPLIER_NEG_6,
        _ => panic!("Invalid boost value: {}", boost),
    }
}

fn evaluate_hazards(pokemon: &Pokemon, side: &Side) -> f32 {
    let mut score = 0.0;
    let pkmn_is_grounded = pokemon.is_grounded();
    if pokemon.item != Items::HEAVYDUTYBOOTS {
        if pokemon.ability != Abilities::MAGICGUARD {
            score += side.side_conditions.stealth_rock as f32 * STEALTH_ROCK;
            if pkmn_is_grounded {
                score += side.side_conditions.spikes as f32 * SPIKES;
                score += side.side_conditions.toxic_spikes as f32 * TOXIC_SPIKES;
            }
        }
        if pkmn_is_grounded {
            score += side.side_conditions.sticky_web as f32 * STICKY_WEB;
        }
    }

    score
}

fn evaluate_pokemon(pokemon: &Pokemon) -> f32 {
    let mut score = 0.0;
    score += POKEMON_HP * pokemon.hp as f32 / pokemon.maxhp as f32;

    match pokemon.status {
        PokemonStatus::BURN => score += evaluate_burned(pokemon),
        PokemonStatus::FREEZE => score += POKEMON_FROZEN,
        PokemonStatus::SLEEP => score += POKEMON_ASLEEP,
        PokemonStatus::PARALYZE => score += POKEMON_PARALYZED,
        PokemonStatus::TOXIC => score += evaluate_poison(pokemon, POKEMON_TOXIC),
        PokemonStatus::POISON => score += evaluate_poison(pokemon, POKEMON_POISONED),
        PokemonStatus::NONE => {}
    }

    if pokemon.item != Items::NONE {
        score += 10.0;
    }

    // without this a low hp pokemon could get a negative score and incentivize the other side
    // to keep it alive
    if score < 0.0 {
        score = 0.0;
    }

    score += POKEMON_ALIVE;

    score
}

fn evaluate_slot(
    side: &Side,
    slot: &SideSlot,
    has_alive_reserve: bool,
    other_side_a: &Pokemon,
    other_side_b: &Pokemon,
) -> f32 {
    let mut score = 0.0;
    for vs in slot.volatile_statuses.iter() {
        match vs {
            PokemonVolatileStatus::LEECHSEED => score += LEECH_SEED,
            PokemonVolatileStatus::SUBSTITUTE => score += SUBSTITUTE,
            PokemonVolatileStatus::CONFUSION => score += CONFUSION,
            PokemonVolatileStatus::PERISH3
                if !has_alive_reserve || side.trapped(slot, other_side_a, other_side_b) =>
            {
                score += PERISH3
            }
            PokemonVolatileStatus::PERISH1
                if !has_alive_reserve || side.trapped(slot, other_side_a, other_side_b) =>
            {
                score += PERISH2
            }
            PokemonVolatileStatus::PERISH1
                if !has_alive_reserve || side.trapped(slot, other_side_a, other_side_b) =>
            {
                score += PERISH1
            }
            _ => {}
        }
    }

    score += get_boost_multiplier(slot.attack_boost) * POKEMON_ATTACK_BOOST;
    score += get_boost_multiplier(slot.defense_boost) * POKEMON_DEFENSE_BOOST;
    score += get_boost_multiplier(slot.special_attack_boost) * POKEMON_SPECIAL_ATTACK_BOOST;
    score += get_boost_multiplier(slot.special_defense_boost) * POKEMON_SPECIAL_DEFENSE_BOOST;
    score += get_boost_multiplier(slot.speed_boost) * POKEMON_SPEED_BOOST;
    score
}

pub fn evaluate(state: &State) -> f32 {
    let mut score = 0.0;
    let side_one_a = &state.side_one.pokemon[state.side_one.slot_a.active_index];
    let side_one_b = &state.side_one.pokemon[state.side_one.slot_b.active_index];
    let side_two_a = &state.side_one.pokemon[state.side_two.slot_a.active_index];
    let side_two_b = &state.side_one.pokemon[state.side_two.slot_b.active_index];
    let mut iter = state.side_one.pokemon.into_iter();
    let mut s1_used_tera = false;
    let mut side_one_has_alive_reserve = false;
    while let Some(pkmn) = iter.next() {
        if pkmn.hp > 0 {
            score += evaluate_pokemon(pkmn);
            score += evaluate_hazards(pkmn, &state.side_one);
            if iter.pokemon_index != state.side_one.slot_a.active_index
                && iter.pokemon_index != state.side_one.slot_b.active_index
            {
                side_one_has_alive_reserve = true;
            }
        }
        if pkmn.terastallized {
            s1_used_tera = true;
        }
    }
    if state.side_one.pokemon[state.side_one.slot_a.active_index].hp > 0 {
        score += evaluate_slot(
            &state.side_one,
            &state.side_one.slot_a,
            side_one_has_alive_reserve,
            side_two_a,
            side_two_b,
        );
    }
    if state.side_one.pokemon[state.side_one.slot_b.active_index].hp > 0 {
        score += evaluate_slot(
            &state.side_one,
            &state.side_one.slot_b,
            side_one_has_alive_reserve,
            side_two_a,
            side_two_b,
        );
    }
    if s1_used_tera {
        score += USED_TERA;
    }
    let mut iter = state.side_two.pokemon.into_iter();
    let mut s2_used_tera = false;
    let mut side_two_has_alive_reserve = false;
    while let Some(pkmn) = iter.next() {
        if pkmn.hp > 0 {
            score -= evaluate_pokemon(pkmn);
            score -= evaluate_hazards(pkmn, &state.side_two);
            if iter.pokemon_index != state.side_two.slot_a.active_index
                && iter.pokemon_index != state.side_two.slot_b.active_index
            {
                side_two_has_alive_reserve = true;
            }
        }
        if pkmn.terastallized {
            s2_used_tera = true;
        }
    }
    if state.side_two.pokemon[state.side_two.slot_a.active_index].hp > 0 {
        score -= evaluate_slot(
            &state.side_two,
            &state.side_two.slot_a,
            side_two_has_alive_reserve,
            side_one_a,
            side_one_b,
        );
    }
    if state.side_two.pokemon[state.side_two.slot_b.active_index].hp > 0 {
        score -= evaluate_slot(
            &state.side_two,
            &state.side_two.slot_b,
            side_two_has_alive_reserve,
            side_one_a,
            side_one_b,
        );
    }
    if s2_used_tera {
        score -= USED_TERA;
    }

    let s1a_speed = get_effective_speed(state, &SideReference::SideOne, &SlotReference::SlotA);
    let s1b_speed = get_effective_speed(state, &SideReference::SideOne, &SlotReference::SlotB);
    let s2a_speed = get_effective_speed(state, &SideReference::SideTwo, &SlotReference::SlotA);
    let s2b_speed = get_effective_speed(state, &SideReference::SideTwo, &SlotReference::SlotB);

    let faster_than_multiplier = if state.trick_room.active {
        // in trick room, slower pokemon are favored
        // but the value is diminished by the number of turns remaining
        4.0 / state.trick_room.turns_remaining as f32
    } else {
        1.0
    };

    let comparisons = [
        (s1a_speed, s2a_speed),
        (s1a_speed, s2b_speed),
        (s1b_speed, s2a_speed),
        (s1b_speed, s2b_speed),
    ];

    for (s1, s2) in comparisons {
        let s1_is_faster = if state.trick_room.active {
            s1 < s2 // slower is better in trick room
        } else {
            s1 > s2 // faster is better otherwise
        };

        if s1_is_faster {
            score += FASTER_THAN_OPPONENT * faster_than_multiplier;
        } else {
            score -= FASTER_THAN_OPPONENT * faster_than_multiplier;
        }
    }

    score += state.side_one.side_conditions.reflect as f32 * REFLECT;
    score += state.side_one.side_conditions.light_screen as f32 * LIGHT_SCREEN;
    score += state.side_one.side_conditions.aurora_veil as f32 * AURORA_VEIL;
    score += state.side_one.side_conditions.safeguard as f32 * SAFE_GUARD;
    score += state.side_one.side_conditions.tailwind as f32 * TAILWIND;
    score += state.side_one.side_conditions.healing_wish as f32 * HEALING_WISH;

    score -= state.side_two.side_conditions.reflect as f32 * REFLECT;
    score -= state.side_two.side_conditions.light_screen as f32 * LIGHT_SCREEN;
    score -= state.side_two.side_conditions.aurora_veil as f32 * AURORA_VEIL;
    score -= state.side_two.side_conditions.safeguard as f32 * SAFE_GUARD;
    score -= state.side_two.side_conditions.tailwind as f32 * TAILWIND;
    score -= state.side_two.side_conditions.healing_wish as f32 * HEALING_WISH;

    score
}
