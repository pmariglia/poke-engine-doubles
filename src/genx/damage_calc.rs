use super::abilities::Abilities;
use super::state::{PokemonVolatileStatus, Terrain, Weather};
use crate::choices::Choices;
use crate::choices::{Choice, MoveCategory};
use crate::state::{
    Pokemon, PokemonBoostableStat, PokemonIndex, PokemonStatus, PokemonType, Side, SideReference,
    SideSlot, SlotReference, State,
};

#[rustfmt::skip]
const TYPE_MATCHUP_DAMAGE_MULTIPICATION: [[f32; 19]; 19] = [
/*         0    1    2    3    4    5    6    7    8    9   10   11   12   13   14   15   16   17   18  */
/*  0 */ [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5, 0.0, 1.0, 1.0, 0.5, 1.0, 1.0],
/*  1 */ [1.0, 0.5, 0.5, 1.0, 2.0, 2.0, 1.0, 1.0, 1.0, 1.0, 1.0, 2.0, 0.5, 1.0, 0.5, 1.0, 2.0, 1.0, 1.0],
/*  2 */ [1.0, 2.0, 0.5, 1.0, 0.5, 1.0, 1.0, 1.0, 2.0, 1.0, 1.0, 1.0, 2.0, 1.0, 0.5, 1.0, 1.0, 1.0, 1.0],
/*  3 */ [1.0, 1.0, 2.0, 0.5, 0.5, 1.0, 1.0, 1.0, 0.0, 2.0, 1.0, 1.0, 1.0, 1.0, 0.5, 1.0, 1.0, 1.0, 1.0],
/*  4 */ [1.0, 0.5, 2.0, 1.0, 0.5, 1.0, 1.0, 0.5, 2.0, 0.5, 1.0, 0.5, 2.0, 1.0, 0.5, 1.0, 0.5, 1.0, 1.0],
/*  5 */ [1.0, 0.5, 0.5, 1.0, 2.0, 0.5, 1.0, 1.0, 2.0, 2.0, 1.0, 1.0, 1.0, 1.0, 2.0, 1.0, 0.5, 1.0, 1.0],
/*  6 */ [2.0, 1.0, 1.0, 1.0, 1.0, 2.0, 1.0, 0.5, 1.0, 0.5, 0.5, 0.5, 2.0, 0.0, 1.0, 2.0, 2.0, 0.5, 1.0],
/*  7 */ [1.0, 1.0, 1.0, 1.0, 2.0, 1.0, 1.0, 0.5, 0.5, 1.0, 1.0, 1.0, 0.5, 0.5, 1.0, 1.0, 0.0, 2.0, 1.0],
/*  8 */ [1.0, 2.0, 1.0, 2.0, 0.5, 1.0, 1.0, 2.0, 1.0, 0.0, 1.0, 0.5, 2.0, 1.0, 1.0, 1.0, 2.0, 1.0, 1.0],
/*  9 */ [1.0, 1.0, 1.0, 0.5, 2.0, 1.0, 2.0, 1.0, 1.0, 1.0, 1.0, 2.0, 0.5, 1.0, 1.0, 1.0, 0.5, 1.0, 1.0],
/* 10 */ [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 2.0, 2.0, 1.0, 1.0, 0.5, 1.0, 1.0, 1.0, 1.0, 0.0, 0.5, 1.0, 1.0],
/* 11 */ [1.0, 0.5, 1.0, 1.0, 2.0, 1.0, 0.5, 0.5, 1.0, 0.5, 2.0, 1.0, 1.0, 0.5, 1.0, 2.0, 0.5, 0.5, 1.0],
/* 12 */ [1.0, 2.0, 1.0, 1.0, 1.0, 2.0, 0.5, 1.0, 0.5, 2.0, 1.0, 2.0, 1.0, 1.0, 1.0, 1.0, 0.5, 1.0, 1.0],
/* 13 */ [0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 2.0, 1.0, 1.0, 2.0, 1.0, 0.5, 1.0, 1.0, 1.0],
/* 14 */ [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 2.0, 1.0, 0.5, 0.0, 1.0],
/* 15 */ [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5, 1.0, 1.0, 1.0, 2.0, 1.0, 1.0, 2.0, 1.0, 0.5, 1.0, 0.5, 1.0],
/* 16 */ [1.0, 0.5, 0.5, 0.5, 1.0, 2.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 2.0, 1.0, 1.0, 1.0, 0.5, 2.0, 1.0],
/* 17 */ [1.0, 0.5, 1.0, 1.0, 1.0, 1.0, 2.0, 0.5, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 2.0, 2.0, 0.5, 1.0, 1.0],
/* 18 */ [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]
];

pub const CRIT_MULTIPLIER: f32 = 1.5;

#[allow(dead_code)]
pub enum DamageRolls {
    Average,
    Min,
    Max,
}

fn type_enum_to_type_matchup_int(type_enum: &PokemonType) -> usize {
    match type_enum {
        PokemonType::NORMAL => 0,
        PokemonType::FIRE => 1,
        PokemonType::WATER => 2,
        PokemonType::ELECTRIC => 3,
        PokemonType::GRASS => 4,
        PokemonType::ICE => 5,
        PokemonType::FIGHTING => 6,
        PokemonType::POISON => 7,
        PokemonType::GROUND => 8,
        PokemonType::FLYING => 9,
        PokemonType::PSYCHIC => 10,
        PokemonType::BUG => 11,
        PokemonType::ROCK => 12,
        PokemonType::GHOST => 13,
        PokemonType::DRAGON => 14,
        PokemonType::DARK => 15,
        PokemonType::STEEL => 16,
        PokemonType::FAIRY => 17,
        PokemonType::TYPELESS => 18,
        PokemonType::STELLAR => 18, // Stellar is typeless for type effectiveness
    }
}

pub fn type_effectiveness_modifier(attacking_type: &PokemonType, defender: &Pokemon) -> f32 {
    #[cfg(not(feature = "terastallization"))]
    let defending_types = defender.types;
    #[cfg(feature = "terastallization")]
    let defending_types = if defender.terastallized {
        (defender.tera_type, PokemonType::TYPELESS)
    } else {
        defender.types
    };
    _type_effectiveness_modifier(attacking_type, &defending_types)
}

fn _type_effectiveness_modifier(
    attacking_type: &PokemonType,
    defending_types: &(PokemonType, PokemonType),
) -> f32 {
    let mut modifier = 1.0;
    let attacking_type_index = type_enum_to_type_matchup_int(attacking_type);
    modifier = modifier
        * TYPE_MATCHUP_DAMAGE_MULTIPICATION[attacking_type_index]
            [type_enum_to_type_matchup_int(&defending_types.0)];
    modifier = modifier
        * TYPE_MATCHUP_DAMAGE_MULTIPICATION[attacking_type_index]
            [type_enum_to_type_matchup_int(&defending_types.1)];
    modifier
}

fn weather_modifier(attacking_move_type: &PokemonType, weather: &Weather) -> f32 {
    match weather {
        Weather::SUN => match attacking_move_type {
            PokemonType::FIRE => 1.5,
            PokemonType::WATER => 0.5,
            _ => 1.0,
        },
        Weather::RAIN => match attacking_move_type {
            PokemonType::WATER => 1.5,
            PokemonType::FIRE => 0.5,
            _ => 1.0,
        },
        Weather::HARSHSUN => match attacking_move_type {
            PokemonType::FIRE => 1.5,
            PokemonType::WATER => 0.0,
            _ => 1.0,
        },
        Weather::HEAVYRAIN => match attacking_move_type {
            PokemonType::WATER => 1.5,
            PokemonType::FIRE => 0.0,
            _ => 1.0,
        },
        _ => 1.0,
    }
}

fn stab_modifier(attacking_move_type: &PokemonType, active_pkmn: &Pokemon) -> f32 {
    if attacking_move_type == &PokemonType::TYPELESS {
        return 1.0;
    }

    let active_types = active_pkmn.types;
    let move_has_basic_stab =
        attacking_move_type == &active_types.0 || attacking_move_type == &active_types.1;
    if active_pkmn.terastallized {
        if &active_pkmn.tera_type == attacking_move_type && move_has_basic_stab {
            return 2.0;
        } else if &active_pkmn.tera_type == attacking_move_type || move_has_basic_stab {
            return 1.5;
        }
    } else if move_has_basic_stab {
        return 1.5;
    }
    1.0
}

fn burn_modifier(
    attacking_move_category: &MoveCategory,
    attacking_pokemon_status: &PokemonStatus,
) -> f32 {
    if attacking_pokemon_status == &PokemonStatus::BURN
        && attacking_move_category == &MoveCategory::Physical
    {
        return 0.5;
    }

    1.0
}

fn terrain_modifier(
    terrain: &Terrain,
    attacker: &Pokemon,
    defender: &Pokemon,
    choice: &Choice,
) -> f32 {
    let terrain_boost = 1.3;

    match terrain {
        Terrain::ELECTRICTERRAIN => {
            if choice.move_type == PokemonType::ELECTRIC && attacker.is_grounded() {
                terrain_boost
            } else {
                1.0
            }
        }
        Terrain::GRASSYTERRAIN => {
            if choice.move_type == PokemonType::GRASS && attacker.is_grounded() {
                terrain_boost
            } else if choice.move_id == Choices::EARTHQUAKE {
                0.5
            } else {
                1.0
            }
        }
        Terrain::MISTYTERRAIN => {
            if choice.move_type == PokemonType::DRAGON && defender.is_grounded() {
                0.5
            } else {
                1.0
            }
        }
        Terrain::PSYCHICTERRAIN => {
            if choice.move_type == PokemonType::PSYCHIC && attacker.is_grounded() {
                terrain_boost
            } else {
                1.0
            }
        }
        Terrain::NONE => 1.0,
    }
}

fn volatile_status_modifier(
    choice: &Choice,
    attacking_slot: &SideSlot,
    defending_slot: &SideSlot,
) -> f32 {
    let mut modifier = 1.0;
    for vs in attacking_slot.volatile_statuses.iter() {
        match vs {
            PokemonVolatileStatus::HELPINGHAND => {
                modifier *= 1.5;
            }
            PokemonVolatileStatus::FLASHFIRE if choice.move_type == PokemonType::FIRE => {
                modifier *= 1.5;
            }
            PokemonVolatileStatus::SLOWSTART if choice.category == MoveCategory::Physical => {
                modifier *= 0.5;
            }
            PokemonVolatileStatus::CHARGE if choice.move_type == PokemonType::ELECTRIC => {
                modifier *= 2.0;
            }
            PokemonVolatileStatus::PROTOSYNTHESISATK | PokemonVolatileStatus::QUARKDRIVEATK
                if choice.category == MoveCategory::Physical =>
            {
                modifier *= 1.3;
            }
            PokemonVolatileStatus::PROTOSYNTHESISSPA | PokemonVolatileStatus::QUARKDRIVESPA
                if choice.category == MoveCategory::Special =>
            {
                modifier *= 1.3;
            }
            _ => {}
        }
    }

    for vs in defending_slot.volatile_statuses.iter() {
        match vs {
            PokemonVolatileStatus::MAGNETRISE
                if choice.move_type == PokemonType::GROUND
                    && choice.move_id != Choices::THOUSANDARROWS =>
            {
                return 0.0;
            }
            PokemonVolatileStatus::TARSHOT if choice.move_type == PokemonType::FIRE => {
                modifier *= 2.0;
            }
            PokemonVolatileStatus::PHANTOMFORCE
            | PokemonVolatileStatus::SHADOWFORCE
            | PokemonVolatileStatus::BOUNCE
            | PokemonVolatileStatus::DIG
            | PokemonVolatileStatus::DIVE
            | PokemonVolatileStatus::FLY => {
                return 0.0;
            }
            PokemonVolatileStatus::GLAIVERUSH => {
                modifier *= 2.0;
            }
            PokemonVolatileStatus::PROTOSYNTHESISDEF | PokemonVolatileStatus::QUARKDRIVEDEF
                if choice.category == MoveCategory::Physical =>
            {
                modifier /= 1.3;
            }
            PokemonVolatileStatus::PROTOSYNTHESISSPD | PokemonVolatileStatus::QUARKDRIVESPD
                if choice.category == MoveCategory::Special =>
            {
                modifier /= 1.3;
            }
            _ => {}
        }
    }

    modifier
}

fn get_defending_types(
    slot: &SideSlot,
    defending_pkmn: &Pokemon,
    attacking_pkmn: &Pokemon,
    attacking_choice: &Choice,
) -> (PokemonType, PokemonType) {
    if defending_pkmn.terastallized && !(defending_pkmn.tera_type == PokemonType::STELLAR) {
        return (defending_pkmn.tera_type, PokemonType::TYPELESS);
    }
    let mut defender_types = defending_pkmn.types;
    if slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::ROOST)
    {
        if defender_types.0 == PokemonType::FLYING {
            defender_types = (PokemonType::TYPELESS, defender_types.1);
        }
        if defender_types.1 == PokemonType::FLYING {
            defender_types = (defender_types.0, PokemonType::TYPELESS);
        }
    }
    if (attacking_pkmn.ability == Abilities::SCRAPPY
        || attacking_pkmn.ability == Abilities::MINDSEYE)
        && (attacking_choice.move_type == PokemonType::NORMAL
            || attacking_choice.move_type == PokemonType::FIGHTING)
    {
        if defender_types.0 == PokemonType::GHOST {
            defender_types = (PokemonType::TYPELESS, defender_types.1);
        }
        if defender_types.1 == PokemonType::GHOST {
            defender_types = (defender_types.0, PokemonType::TYPELESS);
        }
    }
    defender_types
}

fn get_attacking_and_defending_stats(
    attacker: &Pokemon,
    defender: &Pokemon,
    attacking_side: &Side,
    attacking_slot_ref: &SlotReference,
    defending_side: &Side,
    defending_slot_ref: &SlotReference,
    state: &State,
    choice: &Choice,
) -> (i16, i16, i16, i16) {
    let mut should_calc_attacker_boost = true;
    let mut should_calc_defender_boost = true;
    let (attacking_stat, defending_stat);
    let (
        mut attacking_final_stat,
        mut defending_final_stat,
        mut crit_attacking_stat,
        mut crit_defending_stat,
    );

    match choice.category {
        MoveCategory::Physical => {
            if attacking_side
                .get_slot_immutable(attacking_slot_ref)
                .attack_boost
                > 0
            {
                crit_attacking_stat = attacking_side
                    .calculate_boosted_stat(attacking_slot_ref, PokemonBoostableStat::Attack);
            } else {
                crit_attacking_stat = attacker.attack;
            }
            if defending_side
                .get_slot_immutable(defending_slot_ref)
                .defense_boost
                <= 0
            {
                crit_defending_stat = defending_side
                    .calculate_boosted_stat(defending_slot_ref, PokemonBoostableStat::Defense);
            } else {
                crit_defending_stat = defender.defense;
            }

            // Unaware checks
            if defender.ability == Abilities::UNAWARE && attacker.ability != Abilities::MOLDBREAKER
            {
                should_calc_attacker_boost = false;
            }
            if attacker.ability == Abilities::UNAWARE && defender.ability != Abilities::MOLDBREAKER
            {
                should_calc_defender_boost = false;
            }

            // Get the attacking stat

            // checks for moves that change which stat is used for the attacking_stat
            if choice.move_id == Choices::FOULPLAY {
                if should_calc_attacker_boost {
                    attacking_final_stat = defending_side
                        .calculate_boosted_stat(defending_slot_ref, PokemonBoostableStat::Attack);
                } else {
                    attacking_final_stat = defender.attack;
                }
                attacking_stat = PokemonBoostableStat::Attack;
                crit_attacking_stat = defending_side
                    .get_active_immutable(defending_slot_ref)
                    .attack;
            } else if choice.move_id == Choices::BODYPRESS {
                if should_calc_attacker_boost {
                    attacking_final_stat = attacking_side
                        .calculate_boosted_stat(attacking_slot_ref, PokemonBoostableStat::Defense);
                } else {
                    attacking_final_stat = attacker.defense;
                }
                attacking_stat = PokemonBoostableStat::Defense;
                crit_attacking_stat = attacking_side
                    .get_active_immutable(attacking_slot_ref)
                    .defense;
            } else if should_calc_attacker_boost {
                attacking_final_stat = attacking_side
                    .calculate_boosted_stat(attacking_slot_ref, PokemonBoostableStat::Attack);
                attacking_stat = PokemonBoostableStat::Attack;
            } else {
                attacking_final_stat = attacker.attack;
                attacking_stat = PokemonBoostableStat::Attack;
            }

            // Get the defending stat
            defending_stat = PokemonBoostableStat::Defense;
            if should_calc_defender_boost {
                defending_final_stat = defending_side
                    .calculate_boosted_stat(defending_slot_ref, PokemonBoostableStat::Defense);
            } else {
                defending_final_stat = defender.defense;
            }
        }
        MoveCategory::Special => {
            attacking_stat = PokemonBoostableStat::SpecialAttack;

            if attacking_side
                .get_slot_immutable(attacking_slot_ref)
                .special_attack_boost
                > 0
            {
                crit_attacking_stat = attacking_side.calculate_boosted_stat(
                    attacking_slot_ref,
                    PokemonBoostableStat::SpecialAttack,
                );
            } else {
                crit_attacking_stat = attacker.special_attack;
            }
            if defending_side
                .get_slot_immutable(defending_slot_ref)
                .special_defense_boost
                <= 0
            {
                crit_defending_stat = defending_side.calculate_boosted_stat(
                    defending_slot_ref,
                    PokemonBoostableStat::SpecialDefense,
                );
            } else {
                crit_defending_stat = defender.special_defense;
            }

            // Unaware checks
            if defender.ability == Abilities::UNAWARE && attacker.ability != Abilities::MOLDBREAKER
            {
                should_calc_attacker_boost = false;
            }
            if attacker.ability == Abilities::UNAWARE && defender.ability != Abilities::MOLDBREAKER
            {
                should_calc_defender_boost = false;
            }

            // Get the attacking stat
            if should_calc_attacker_boost {
                attacking_final_stat = attacking_side.calculate_boosted_stat(
                    attacking_slot_ref,
                    PokemonBoostableStat::SpecialAttack,
                );
            } else {
                attacking_final_stat = attacker.special_attack;
            }

            // Get the defending stat
            // check for moves that change which stat is used for the defending_stat
            if choice.move_id == Choices::PSYSHOCK
                || choice.move_id == Choices::SECRETSWORD
                || choice.move_id == Choices::PSYSTRIKE
            {
                if defending_side
                    .get_slot_immutable(defending_slot_ref)
                    .defense_boost
                    <= 0
                {
                    crit_defending_stat = defending_side
                        .calculate_boosted_stat(defending_slot_ref, PokemonBoostableStat::Defense);
                } else {
                    crit_defending_stat = defender.defense;
                }

                defending_stat = PokemonBoostableStat::Defense;
                if should_calc_defender_boost {
                    defending_final_stat = defending_side
                        .calculate_boosted_stat(defending_slot_ref, PokemonBoostableStat::Defense);
                } else {
                    defending_final_stat = defender.defense;
                }
            } else {
                defending_stat = PokemonBoostableStat::SpecialDefense;
                if should_calc_defender_boost {
                    defending_final_stat = defending_side.calculate_boosted_stat(
                        defending_slot_ref,
                        PokemonBoostableStat::SpecialDefense,
                    );
                } else {
                    defending_final_stat = defender.special_defense;
                }
            }
        }
        _ => panic!("Can only calculate damage for physical or special moves"),
    }

    if state.weather_is_active(&Weather::SNOW)
        && defender.has_type(&PokemonType::ICE)
        && defending_stat == PokemonBoostableStat::Defense
    {
        defending_final_stat = (defending_final_stat as f32 * 1.5) as i16;
    } else if state.weather_is_active(&Weather::SAND)
        && defender.has_type(&PokemonType::ROCK)
        && defending_stat == PokemonBoostableStat::SpecialDefense
    {
        defending_final_stat = (defending_final_stat as f32 * 1.5) as i16;
    }

    let neutralizing_gas_is_active = state.neutralizing_gas_is_active();

    if attacking_stat == PokemonBoostableStat::Attack
        && attacker.ability != Abilities::TABLETSOFRUIN
        && attacker.ability != Abilities::MOLDBREAKER
        && state.ability_is_active(Abilities::TABLETSOFRUIN)
        && !neutralizing_gas_is_active
    {
        attacking_final_stat = (attacking_final_stat as f32 * 0.75) as i16;
        crit_attacking_stat = (crit_attacking_stat as f32 * 0.75) as i16;
    } else if attacking_stat == PokemonBoostableStat::SpecialAttack
        && attacker.ability != Abilities::VESSELOFRUIN
        && attacker.ability != Abilities::MOLDBREAKER
        && state.ability_is_active(Abilities::VESSELOFRUIN)
        && !neutralizing_gas_is_active
    {
        attacking_final_stat = (attacking_final_stat as f32 * 0.75) as i16;
        crit_attacking_stat = (crit_attacking_stat as f32 * 0.75) as i16;
    }

    if defending_stat == PokemonBoostableStat::Defense
        && defender.ability != Abilities::SWORDOFRUIN
        && state.ability_is_active(Abilities::SWORDOFRUIN)
        && !neutralizing_gas_is_active
    {
        defending_final_stat = (defending_final_stat as f32 * 0.75) as i16;
        crit_defending_stat = (crit_defending_stat as f32 * 0.75) as i16;
    } else if defending_stat == PokemonBoostableStat::SpecialDefense
        && defender.ability != Abilities::BEADSOFRUIN
        && state.ability_is_active(Abilities::BEADSOFRUIN)
        && !neutralizing_gas_is_active
    {
        defending_final_stat = (defending_final_stat as f32 * 0.75) as i16;
        crit_defending_stat = (crit_defending_stat as f32 * 0.75) as i16;
    }

    (
        attacking_final_stat,
        defending_final_stat,
        crit_attacking_stat,
        crit_defending_stat,
    )
}

fn common_pkmn_damage_calc(
    attacking_slot: &SideSlot,
    attacker: &Pokemon,
    attacking_stat: i16,
    target_slot: &SideSlot,
    defender: &Pokemon,
    defending_stat: i16,
    weather: &Weather,
    terrain: &Terrain,
    choice: &Choice,
) -> f32 {
    let mut damage: f32;
    damage = 2.0 * attacker.level as f32;
    damage = damage.floor() / 5.0;
    damage = damage.floor() + 2.0;
    damage = damage.floor() * choice.base_power;
    damage = damage * attacking_stat as f32 / defending_stat as f32;
    damage = damage.floor() / 50.0;
    damage = damage.floor() + 2.0;

    let defender_types = get_defending_types(&target_slot, defender, attacker, choice);

    let mut damage_modifier = 1.0;

    if defender.terastallized && choice.move_type == PokemonType::STELLAR {
        damage_modifier *= 2.0;
    } else if defender.ability == Abilities::TERASHELL && defender.hp == defender.maxhp {
        damage_modifier *= 0.5;
    } else {
        damage_modifier *= _type_effectiveness_modifier(&choice.move_type, &defender_types);
    }

    if attacker.ability != Abilities::CLOUDNINE
        && attacker.ability != Abilities::AIRLOCK
        && defender.ability != Abilities::CLOUDNINE
        && defender.ability != Abilities::AIRLOCK
    {
        damage_modifier *= weather_modifier(&choice.move_type, weather);
    }

    damage_modifier *= stab_modifier(&choice.move_type, &attacker);
    damage_modifier *= burn_modifier(&choice.category, &attacker.status);
    damage_modifier *= volatile_status_modifier(&choice, attacking_slot, target_slot);
    damage_modifier *= terrain_modifier(terrain, attacker, defender, &choice);

    damage * damage_modifier
}

// This is a basic damage calculation function that assumes special effects/modifiers
// are reflected in the `Choice` struct
//
// i.e. if an ability would multiply a move's base-power by 1.3x, that should already
// be reflected in the `Choice`
pub fn calculate_damage(
    state: &State,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
    target_side_ref: &SideReference,
    target_slot_ref: &SlotReference,
    choice: &Choice,
    _damage_rolls: DamageRolls,
) -> Option<(i16, i16)> {
    if choice.category == MoveCategory::Status || choice.category == MoveCategory::Switch {
        return None;
    } else if choice.base_power == 0.0 {
        return Some((0, 0));
    }
    let attacking_side = state.get_side_immutable(attacking_side_ref);
    let attacking_slot = attacking_side.get_slot_immutable(attacking_slot_ref);
    let defending_side = state.get_side_immutable(target_side_ref);
    let defending_slot = defending_side.get_slot_immutable(target_slot_ref);
    let attacker = attacking_side.get_active_immutable(attacking_slot_ref);
    let defender = defending_side.get_active_immutable(target_slot_ref);
    let (attacking_stat, defending_stat, crit_attacking_stat, crit_defending_stat) =
        get_attacking_and_defending_stats(
            attacker,
            defender,
            attacking_side,
            attacking_slot_ref,
            defending_side,
            target_slot_ref,
            state,
            &choice,
        );

    let mut damage = common_pkmn_damage_calc(
        attacking_slot,
        attacker,
        attacking_stat,
        defending_slot,
        defender,
        defending_stat,
        &state.weather.weather_type,
        &state.terrain.terrain_type,
        choice,
    );
    if attacker.ability != Abilities::INFILTRATOR {
        if defending_side.side_conditions.aurora_veil > 0 {
            damage *= 0.667
        } else if defending_side.side_conditions.reflect > 0
            && choice.category == MoveCategory::Physical
        {
            damage *= 0.667
        } else if defending_side.side_conditions.light_screen > 0
            && choice.category == MoveCategory::Special
        {
            damage *= 0.667
        }
    }

    let mut crit_damage = common_pkmn_damage_calc(
        attacking_slot,
        attacker,
        crit_attacking_stat,
        defending_slot,
        defender,
        crit_defending_stat,
        &state.weather.weather_type,
        &state.terrain.terrain_type,
        choice,
    );
    crit_damage *= CRIT_MULTIPLIER;

    match _damage_rolls {
        DamageRolls::Average => {
            damage = damage.floor() * 0.925;
            crit_damage = crit_damage.floor() * 0.925;
        }
        DamageRolls::Min => {
            damage = damage.floor() * 0.85;
            crit_damage = crit_damage.floor() * 0.85;
        }
        DamageRolls::Max => {
            damage = damage.floor();
            crit_damage = crit_damage.floor();
        }
    }

    Some((damage as i16, crit_damage as i16))
}

pub fn calculate_futuresight_damage(
    _attacking_side: &Side,
    _defending_side: &Side,
    _attacking_side_pokemon_index: &PokemonIndex,
) -> i16 {
    0
}
