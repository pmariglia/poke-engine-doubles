use super::abilities::Abilities;
use super::damage_calc::type_effectiveness_modifier;
use super::generate_instructions::{
    add_remove_status_instructions, apply_boost_instructions, get_instructions_from_heal,
};
use super::state::{PokemonVolatileStatus, Terrain, Weather};
use crate::choices::{
    Boost, Choice, Choices, Effect, Heal, MoveCategory, MoveTarget, Secondary, StatBoosts,
};
use crate::engine::items::{get_choice_move_disable_instructions, Items};
use crate::instruction::{
    ApplyVolatileStatusInstruction, BoostInstruction, ChangeItemInstruction,
    ChangeSideConditionInstruction, ChangeStatusInstruction, ChangeSubsituteHealthInstruction,
    ChangeTerrain, ChangeType, ChangeWeather, ChangeWishInstruction, DamageInstruction,
    HealInstruction, IncrementTimesAttackedInstruction, Instruction,
    RemoveVolatileStatusInstruction, SetSleepTurnsInstruction, StateInstructions,
    ToggleTrickRoomInstruction,
};
use crate::pokemon::PokemonName;
use crate::state::{
    pokemon_index_iter, LastUsedMove, PokemonBoostableStat, PokemonSideCondition, PokemonStatus,
    PokemonType, Side, SideReference, SlotReference, State,
};
use std::cmp;

const CHOICE_THAWS_USER: [Choices; 10] = [
    Choices::FLAMEWHEEL,
    Choices::SACREDFIRE,
    Choices::FLAREBLITZ,
    Choices::FUSIONFLARE,
    Choices::SCALD,
    Choices::STEAMERUPTION,
    Choices::BURNUP,
    Choices::PYROBALL,
    Choices::SCORCHINGSANDS,
    Choices::MATCHAGOTCHA,
];

pub fn choice_change_type(
    state: &mut State,
    attacker_choice: &mut Choice,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
) {
    /*
    Similar to `modify_choice`, but this needs to be called before certain checks.
    Whether something goes here or in `modify_choice` doesn't really matter, but changing a move's
    type **needs** to be done here
    */
    let attacking_side = state.get_side_immutable(attacking_side_ref);
    match attacker_choice.move_id {
        Choices::AURAWHEEL => {
            if attacking_side.get_active_immutable(attacking_slot_ref).id
                == PokemonName::MORPEKOHANGRY
            {
                attacker_choice.move_type = PokemonType::DARK;
            }
        }
        Choices::IVYCUDGEL => {
            let attacker = attacking_side.get_active_immutable(attacking_slot_ref);
            match attacker.item {
                Items::WELLSPRINGMASK => {
                    attacker_choice.move_type = PokemonType::WATER;
                }
                Items::HEARTHFLAMEMASK => {
                    attacker_choice.move_type = PokemonType::FIRE;
                }
                Items::CORNERSTONEMASK => {
                    attacker_choice.move_type = PokemonType::ROCK;
                }
                _ => {}
            }
        }
        Choices::RAGINGBULL => {
            // this gives the correct result even though it's not the "correct" way to implement it
            // reflect is only removed if the move hits, but I don't have a way to check that
            // doubling the power ensures the same damage calculation
            match attacking_side.get_active_immutable(attacking_slot_ref).id {
                PokemonName::TAUROSPALDEACOMBAT => {
                    attacker_choice.move_type = PokemonType::FIGHTING;
                }
                PokemonName::TAUROSPALDEABLAZE => {
                    attacker_choice.move_type = PokemonType::FIRE;
                }
                PokemonName::TAUROSPALDEAAQUA => {
                    attacker_choice.move_type = PokemonType::WATER;
                }
                _ => {}
            }
        }
        Choices::JUDGMENT => {
            attacker_choice.move_type = attacking_side
                .get_active_immutable(attacking_slot_ref)
                .types
                .0;
        }
        Choices::MULTIATTACK => {
            attacker_choice.move_type = attacking_side
                .get_active_immutable(attacking_slot_ref)
                .types
                .0;
        }
        Choices::REVELATIONDANCE => {
            if attacking_side
                .get_active_immutable(attacking_slot_ref)
                .terastallized
            {
                attacker_choice.move_type = attacking_side
                    .get_active_immutable(attacking_slot_ref)
                    .tera_type;
            } else {
                attacker_choice.move_type = attacking_side
                    .get_active_immutable(attacking_slot_ref)
                    .types
                    .0;
            }
        }
        Choices::TERABLAST => {
            let active = attacking_side.get_active_immutable(attacking_slot_ref);
            if active.terastallized {
                attacker_choice.move_type = active.tera_type;
            }
        }
        Choices::TERRAINPULSE => match state.terrain.terrain_type {
            Terrain::ELECTRICTERRAIN => {
                attacker_choice.move_type = PokemonType::ELECTRIC;
            }
            Terrain::GRASSYTERRAIN => {
                attacker_choice.move_type = PokemonType::GRASS;
            }
            Terrain::MISTYTERRAIN => {
                attacker_choice.move_type = PokemonType::FAIRY;
            }
            Terrain::PSYCHICTERRAIN => {
                attacker_choice.move_type = PokemonType::PSYCHIC;
            }
            Terrain::NONE => {}
        },
        Choices::WEATHERBALL => match state.weather.weather_type {
            Weather::SUN | Weather::HARSHSUN => {
                attacker_choice.move_type = PokemonType::FIRE;
            }
            Weather::RAIN | Weather::HEAVYRAIN => {
                attacker_choice.move_type = PokemonType::WATER;
            }
            Weather::SAND => {
                attacker_choice.move_type = PokemonType::ROCK;
            }
            Weather::HAIL | Weather::SNOW => {
                attacker_choice.move_type = PokemonType::ICE;
            }
            Weather::NONE => {}
        },
        _ => {}
    }
}

pub fn modify_choice(
    state: &State,
    attacker_choice: &mut Choice,
    target_choice: &Choice,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
    target_side_ref: &SideReference,
    target_slot_ref: &SlotReference,
    target_has_moved: bool,
) {
    let (attacking_side, target_side) =
        state.get_sides_immutable(attacking_side_ref, target_side_ref);
    match attacker_choice.move_id {
        Choices::BEATUP => {
            // an approximation to get beatup to do something
            attacker_choice.base_power = 10.0;
        }
        Choices::RAGEFIST => {
            let multiplier = 1.0
                + attacking_side
                    .get_active_immutable(attacking_slot_ref)
                    .times_attacked
                    .min(6) as f32;
            attacker_choice.base_power *= multiplier
        }
        Choices::DIRECLAW => {
            attacker_choice.add_or_create_secondaries(Secondary {
                chance: 16.67,
                target: MoveTarget::Target,
                effect: Effect::Status(PokemonStatus::POISON),
            });
            attacker_choice.add_or_create_secondaries(Secondary {
                chance: 20.00,
                target: MoveTarget::Target,
                effect: Effect::Status(PokemonStatus::PARALYZE),
            });
            attacker_choice.add_or_create_secondaries(Secondary {
                chance: 25.0,
                target: MoveTarget::Target,
                effect: Effect::Status(PokemonStatus::SLEEP),
            });
        }
        Choices::ELECTROSHOT => {
            if state.weather_is_active(&Weather::RAIN) {
                attacker_choice.flags.charge = false;
            }
        }
        Choices::ORDERUP => {
            match attacking_side
                .get_active_immutable(&attacking_slot_ref.get_other_slot())
                .id
            {
                PokemonName::TATSUGIRI => attacker_choice.add_or_create_secondaries(Secondary {
                    chance: 100.0,
                    target: MoveTarget::User,
                    effect: Effect::Boost(StatBoosts {
                        attack: 1,
                        defense: 0,
                        special_attack: 0,
                        special_defense: 0,
                        speed: 0,
                        accuracy: 0,
                    }),
                }),
                PokemonName::TATSUGIRIDROOPY => {
                    attacker_choice.add_or_create_secondaries(Secondary {
                        chance: 100.0,
                        target: MoveTarget::User,
                        effect: Effect::Boost(StatBoosts {
                            attack: 0,
                            defense: 1,
                            special_attack: 0,
                            special_defense: 0,
                            speed: 0,
                            accuracy: 0,
                        }),
                    })
                }
                PokemonName::TATSUGIRISTRETCHY => {
                    attacker_choice.add_or_create_secondaries(Secondary {
                        chance: 100.0,
                        target: MoveTarget::User,
                        effect: Effect::Boost(StatBoosts {
                            attack: 0,
                            defense: 0,
                            special_attack: 0,
                            special_defense: 0,
                            speed: 1,
                            accuracy: 0,
                        }),
                    })
                }
                _ => {}
            }
        }
        Choices::DOUBLESHOCK => {
            if !attacking_side
                .get_active_immutable(attacking_slot_ref)
                .has_type(&PokemonType::ELECTRIC)
            {
                attacker_choice.remove_all_effects();
            }
        }
        Choices::BURNUP => {
            if !attacking_side
                .get_active_immutable(attacking_slot_ref)
                .has_type(&PokemonType::FIRE)
            {
                attacker_choice.remove_all_effects();
            }
        }
        Choices::REVERSAL => {
            let attacker = attacking_side.get_active_immutable(attacking_slot_ref);
            let hp_ratio = attacker.hp as f32 / attacker.maxhp as f32;
            if hp_ratio >= 0.688 {
                attacker_choice.base_power = 20.0;
            } else if hp_ratio >= 0.354 {
                attacker_choice.base_power = 40.0;
            } else if hp_ratio >= 0.208 {
                attacker_choice.base_power = 80.0;
            } else if hp_ratio >= 0.104 {
                attacker_choice.base_power = 100.0;
            } else if hp_ratio >= 0.042 {
                attacker_choice.base_power = 150.0;
            } else {
                attacker_choice.base_power = 200.0;
            }
        }
        Choices::RAGINGBULL => {
            // this gives the correct result even though it's not the "correct" way to implement it
            // reflect is only removed if the move hits, but I don't have a way to check that
            // doubling the power ensures the same damage calculation
            if target_side.side_conditions.reflect > 0
                || target_side.side_conditions.aurora_veil > 0
            {
                attacker_choice.base_power *= 2.0;
            }
        }
        Choices::BOLTBEAK | Choices::FISHIOUSREND => {
            if attacker_choice.first_move {
                attacker_choice.base_power *= 2.0;
            }
        }
        Choices::HARDPRESS => {
            let target = target_side.get_active_immutable(target_slot_ref);
            attacker_choice.base_power = 100.0 * (target.hp as f32 / target.maxhp as f32);
        }
        Choices::LASTRESPECTS => {
            // Technically not correct because of reviving moves but good enough
            let mut bp_boost = 1.0;
            bp_boost += 1.0 * attacking_side.num_fainted_pkmn() as f32;
            attacker_choice.base_power *= bp_boost
        }
        Choices::CLANGOROUSSOUL => {
            let attacker = attacking_side.get_active_immutable(attacking_slot_ref);
            if attacker.hp > attacker.maxhp / 3 {
                attacker_choice.heal = Some(Heal {
                    target: MoveTarget::User,
                    amount: -0.33,
                });
                attacker_choice.boost = Some(Boost {
                    target: MoveTarget::User,
                    boosts: StatBoosts {
                        attack: 1,
                        defense: 1,
                        special_attack: 1,
                        special_defense: 1,
                        speed: 1,
                        accuracy: 0,
                    },
                });
            }
        }
        Choices::EXPANDINGFORCE => {
            if state.terrain.terrain_type == Terrain::PSYCHICTERRAIN {
                attacker_choice.base_power *= 1.5;
            }
        }
        Choices::FILLETAWAY => {
            let attacker = attacking_side.get_active_immutable(attacking_slot_ref);
            if attacker.hp > attacker.maxhp / 2 {
                attacker_choice.heal = Some(Heal {
                    target: MoveTarget::User,
                    amount: -0.5,
                });
                attacker_choice.boost = Some(Boost {
                    target: MoveTarget::User,
                    boosts: StatBoosts {
                        attack: 2,
                        defense: 0,
                        special_attack: 2,
                        special_defense: 0,
                        speed: 2,
                        accuracy: 0,
                    },
                });
            }
        }
        Choices::FAKEOUT | Choices::FIRSTIMPRESSION => match attacking_side
            .get_slot_immutable(attacking_slot_ref)
            .last_used_move
        {
            LastUsedMove::Switch(_) => {}
            _ => attacker_choice.remove_all_effects(),
        },
        Choices::GROWTH => {
            if state.weather_is_active(&Weather::SUN) {
                attacker_choice.boost = Some(Boost {
                    target: MoveTarget::User,
                    boosts: StatBoosts {
                        attack: 2,
                        defense: 0,
                        special_attack: 2,
                        special_defense: 0,
                        speed: 0,
                        accuracy: 0,
                    },
                });
            }
        }
        Choices::HEX => {
            if target_side.get_active_immutable(target_slot_ref).status != PokemonStatus::NONE {
                attacker_choice.base_power *= 2.0;
            }
        }
        Choices::HYDROSTEAM => {
            if state.weather_is_active(&Weather::SUN) {
                attacker_choice.base_power *= 3.0; // 1.5x for being in sun, 2x for cancelling out rain debuff
            }
        }
        Choices::MISTYEXPLOSION => {
            if state.terrain.terrain_type == Terrain::MISTYTERRAIN {
                attacker_choice.base_power *= 1.5;
            }
        }
        Choices::MORNINGSUN | Choices::MOONLIGHT | Choices::SYNTHESIS => {
            match state.weather.weather_type {
                Weather::SUN => {
                    attacker_choice.heal = Some(Heal {
                        target: MoveTarget::User,
                        amount: 0.667,
                    })
                }
                Weather::NONE => {}
                _ => {
                    attacker_choice.heal = Some(Heal {
                        target: MoveTarget::User,
                        amount: 0.25,
                    })
                }
            }
        }
        Choices::NORETREAT => {
            if attacking_side
                .get_slot_immutable(attacking_slot_ref)
                .volatile_statuses
                .contains(&PokemonVolatileStatus::NORETREAT)
            {
                attacker_choice.boost = None;
            }
        }
        Choices::POLTERGEIST => {
            if target_side.get_active_immutable(target_slot_ref).item == Items::NONE {
                attacker_choice.base_power = 0.0;
            }
        }
        Choices::PSYBLADE => {
            if state.terrain.terrain_type == Terrain::ELECTRICTERRAIN {
                attacker_choice.base_power *= 1.5;
            }
        }
        Choices::PURSUIT => {
            if target_choice.category == MoveCategory::Switch {
                attacker_choice.base_power *= 2.0;
            }
        }
        Choices::RISINGVOLTAGE => {
            if state.terrain.terrain_type == Terrain::ELECTRICTERRAIN {
                attacker_choice.base_power *= 1.5;
            }
        }
        Choices::SHOREUP => {
            if state.weather_is_active(&Weather::SAND) {
                attacker_choice.heal = Some(Heal {
                    target: MoveTarget::User,
                    amount: 0.667,
                });
            }
        }
        Choices::STEELROLLER => {
            if state.terrain.terrain_type == Terrain::NONE {
                attacker_choice.base_power = 0.0;
            }
        }
        Choices::STRENGTHSAP => {
            attacker_choice.boost = Some(Boost {
                target: MoveTarget::Target,
                boosts: StatBoosts {
                    attack: -1,
                    defense: 0,
                    special_attack: 0,
                    special_defense: 0,
                    speed: 0,
                    accuracy: 0,
                },
            });
            let target_slot = target_side.get_slot_immutable(target_slot_ref);
            if target_slot.attack_boost != -6 {
                let target_attack = target_side
                    .calculate_boosted_stat(target_slot_ref, PokemonBoostableStat::Attack);
                let attacker_maxhp = attacking_side
                    .get_active_immutable(attacking_slot_ref)
                    .maxhp;

                if target_side.get_active_immutable(target_slot_ref).ability
                    == Abilities::LIQUIDOOZE
                {
                    attacker_choice.heal = Some(Heal {
                        target: MoveTarget::User,
                        amount: -1.0 * target_attack as f32 / attacker_maxhp as f32,
                    });
                } else {
                    attacker_choice.heal = Some(Heal {
                        target: MoveTarget::User,
                        amount: target_attack as f32 / attacker_maxhp as f32,
                    });
                }
            }
        }
        Choices::TERABLAST => {
            let active = attacking_side.get_active_immutable(attacking_slot_ref);
            if active.terastallized {
                if attacking_side
                    .calculate_boosted_stat(attacking_slot_ref, PokemonBoostableStat::Attack)
                    > attacking_side.calculate_boosted_stat(
                        attacking_slot_ref,
                        PokemonBoostableStat::SpecialAttack,
                    )
                {
                    attacker_choice.category = MoveCategory::Physical;
                }
                if active.tera_type == PokemonType::STELLAR {
                    attacker_choice.base_power *= 100.0 / 80.0;
                    attacker_choice.add_or_create_secondaries(Secondary {
                        chance: 100.0,
                        target: MoveTarget::User,
                        effect: Effect::Boost(StatBoosts {
                            attack: -1,
                            defense: 0,
                            special_attack: -1,
                            special_defense: 0,
                            speed: 0,
                            accuracy: 0,
                        }),
                    })
                }
            }
        }
        Choices::TERASTARSTORM => {
            // changing to a spread move is done elsewhere
            let active = attacking_side.get_active_immutable(attacking_slot_ref);
            if active.terastallized {
                if attacking_side
                    .calculate_boosted_stat(attacking_slot_ref, PokemonBoostableStat::Attack)
                    > attacking_side.calculate_boosted_stat(
                        attacking_slot_ref,
                        PokemonBoostableStat::SpecialAttack,
                    )
                {
                    attacker_choice.category = MoveCategory::Physical;
                }
            }
        }
        Choices::PHOTONGEYSER => {
            if attacking_side
                .calculate_boosted_stat(attacking_slot_ref, PokemonBoostableStat::Attack)
                > attacking_side
                    .calculate_boosted_stat(attacking_slot_ref, PokemonBoostableStat::SpecialAttack)
            {
                attacker_choice.category = MoveCategory::Physical;
            }
        }
        Choices::TERRAINPULSE => match state.terrain.terrain_type {
            Terrain::ELECTRICTERRAIN => {
                attacker_choice.base_power *= 2.0;
            }
            Terrain::GRASSYTERRAIN => {
                attacker_choice.base_power *= 2.0;
            }
            Terrain::MISTYTERRAIN => {
                attacker_choice.base_power *= 2.0;
            }
            Terrain::PSYCHICTERRAIN => {
                attacker_choice.base_power *= 2.0;
            }
            Terrain::NONE => {}
        },
        Choices::TOXIC => {
            if attacking_side
                .get_active_immutable(attacking_slot_ref)
                .has_type(&PokemonType::POISON)
            {
                attacker_choice.accuracy = 100.0;
            }
        }
        Choices::WEATHERBALL => match state.weather.weather_type {
            Weather::SUN | Weather::HARSHSUN => {
                attacker_choice.base_power = 100.0;
            }
            Weather::RAIN | Weather::HEAVYRAIN => {
                attacker_choice.base_power = 100.0;
            }
            Weather::SAND => {
                attacker_choice.base_power = 100.0;
            }
            Weather::HAIL | Weather::SNOW => {
                attacker_choice.base_power = 100.0;
            }
            Weather::NONE => {}
        },
        Choices::SOLARBEAM | Choices::SOLARBLADE => {
            if state.weather_is_active(&Weather::SUN) || state.weather_is_active(&Weather::HARSHSUN)
            {
                attacker_choice.flags.charge = false;
            } else if !state.weather_is_active(&Weather::SUN)
                && state.weather.weather_type != Weather::NONE
            {
                attacker_choice.base_power /= 2.0;
            }
        }
        Choices::BLIZZARD => {
            if state.weather_is_active(&Weather::HAIL) {
                attacker_choice.accuracy = 100.0;
            }
        }
        Choices::HURRICANE
        | Choices::THUNDER
        | Choices::BLEAKWINDSTORM
        | Choices::SANDSEARSTORM
        | Choices::WILDBOLTSTORM => {
            if state.weather_is_active(&Weather::RAIN)
                || state.weather_is_active(&Weather::HEAVYRAIN)
            {
                attacker_choice.accuracy = 100.0;
            } else if state.weather_is_active(&Weather::SUN)
                || state.weather_is_active(&Weather::HARSHSUN)
            {
                attacker_choice.accuracy = 50.0;
            }
        }

        Choices::KNOCKOFF => {
            // Bonus damage still applies if substitute is hit
            let target = target_side.get_active_immutable(target_slot_ref);
            if !target.item_is_permanent() && target.item != Items::NONE {
                attacker_choice.base_power *= 1.5;
            }
        }

        Choices::ACROBATICS => {
            if attacking_side.get_active_immutable(attacking_slot_ref).item == Items::NONE {
                attacker_choice.base_power *= 2.0;
            }
        }
        Choices::FOCUSPUNCH => {
            let target_slot = target_side.get_slot_immutable(target_slot_ref);
            if (target_slot.damage_dealt.move_category == MoveCategory::Physical
                || target_slot.damage_dealt.move_category == MoveCategory::Special)
                && !target_slot.damage_dealt.hit_substitute
                && target_slot.damage_dealt.damage > 0
            {
                attacker_choice.remove_all_effects();
            }
        }
        Choices::ELECTROBALL => {
            let attacker_speed = attacking_side
                .calculate_boosted_stat(attacking_slot_ref, PokemonBoostableStat::Speed);
            let target_speed =
                target_side.calculate_boosted_stat(target_slot_ref, PokemonBoostableStat::Speed);
            let speed_ratio = attacker_speed as f32 / target_speed as f32;
            if speed_ratio >= 4.0 {
                attacker_choice.base_power = 150.0;
            } else if speed_ratio >= 3.0 {
                attacker_choice.base_power = 120.0;
            } else if speed_ratio >= 2.0 {
                attacker_choice.base_power = 80.0;
            } else if speed_ratio >= 1.0 {
                attacker_choice.base_power = 60.0;
            } else {
                attacker_choice.base_power = 40.0;
            }
        }
        Choices::GYROBALL => {
            let attacker_speed = attacking_side
                .calculate_boosted_stat(attacking_slot_ref, PokemonBoostableStat::Speed);
            let target_speed =
                target_side.calculate_boosted_stat(target_slot_ref, PokemonBoostableStat::Speed);

            attacker_choice.base_power =
                ((25.0 * target_speed as f32 / attacker_speed as f32) + 1.0).min(150.0);
        }
        Choices::AVALANCHE => {
            if !attacker_choice.first_move
                && (target_choice.category == MoveCategory::Physical
                    || target_choice.category == MoveCategory::Special)
            {
                attacker_choice.base_power *= 2.0;
            }
        }
        Choices::PAYBACK => {
            if !attacker_choice.first_move && target_choice.category != MoveCategory::Switch {
                attacker_choice.base_power *= 2.0;
            }
        }
        Choices::FACADE => {
            if attacking_side
                .get_active_immutable(attacking_slot_ref)
                .status
                != PokemonStatus::NONE
            {
                attacker_choice.base_power *= 2.0;
            }
        }
        Choices::STOREDPOWER | Choices::POWERTRIP => {
            let attacking_slot = attacking_side.get_slot_immutable(attacking_slot_ref);
            let total_boosts = attacking_slot.attack_boost.max(0)
                + attacking_slot.defense_boost.max(0)
                + attacking_slot.special_attack_boost.max(0)
                + attacking_slot.special_defense_boost.max(0)
                + attacking_slot.speed_boost.max(0)
                + attacking_slot.accuracy_boost.max(0)
                + attacking_slot.evasion_boost.max(0);
            if total_boosts > 0 {
                attacker_choice.base_power += 20.0 * total_boosts as f32;
            }
        }
        Choices::BARBBARRAGE => {
            let target_pkmn_status = target_side.get_active_immutable(target_slot_ref).status;
            if target_pkmn_status == PokemonStatus::POISON
                || target_pkmn_status == PokemonStatus::TOXIC
            {
                attacker_choice.base_power *= 2.0;
            }
        }
        Choices::FREEZEDRY => {
            if target_side
                .get_active_immutable(target_slot_ref)
                .has_type(&PokemonType::WATER)
            {
                attacker_choice.base_power *= 4.0; // 2x for being super effective, 2x for nullifying water resistance
            }
        }
        Choices::ERUPTION | Choices::WATERSPOUT | Choices::DRAGONENERGY => {
            let attacker = attacking_side.get_active_immutable(attacking_slot_ref);
            let hp_ratio = attacker.hp as f32 / attacker.maxhp as f32;
            attacker_choice.base_power *= hp_ratio;
        }
        Choices::SUCKERPUNCH | Choices::THUNDERCLAP => {
            if target_has_moved || target_choice.category == MoveCategory::Status {
                attacker_choice.base_power = 0.0;
            }
        }
        Choices::UPPERHAND => {
            if target_has_moved || target_choice.priority <= 0 {
                attacker_choice.remove_all_effects()
            }
        }
        Choices::COLLISIONCOURSE | Choices::ELECTRODRIFT => {
            let target_active = target_side.get_active_immutable(target_slot_ref);
            if type_effectiveness_modifier(&attacker_choice.move_type, &target_active) > 1.0 {
                attacker_choice.base_power *= 1.3;
            }
        }
        Choices::GRASSKNOT | Choices::LOWKICK => {
            let target_active = target_side.get_active_immutable(target_slot_ref);
            if target_active.weight_kg < 10.0 {
                attacker_choice.base_power = 20.0;
            } else if target_active.weight_kg < 25.0 {
                attacker_choice.base_power = 40.0;
            } else if target_active.weight_kg < 50.0 {
                attacker_choice.base_power = 60.0;
            } else if target_active.weight_kg < 100.0 {
                attacker_choice.base_power = 80.0;
            } else if target_active.weight_kg < 200.0 {
                attacker_choice.base_power = 100.0;
            } else {
                attacker_choice.base_power = 120.0;
            }
        }
        Choices::HEATCRASH | Choices::HEAVYSLAM => {
            let attacker = attacking_side.get_active_immutable(attacking_slot_ref);
            let target = target_side.get_active_immutable(target_slot_ref);
            let weight_ratio = target.weight_kg / attacker.weight_kg;
            if weight_ratio > 0.5 {
                attacker_choice.base_power = 40.0;
            } else if weight_ratio > 0.3335 {
                attacker_choice.base_power = 60.0;
            } else if weight_ratio >= 0.2501 {
                attacker_choice.base_power = 80.0;
            } else if weight_ratio >= 0.2001 {
                attacker_choice.base_power = 100.0;
            } else {
                attacker_choice.base_power = 120.0;
            }
        }
        _ => {}
    }
}

pub fn choice_after_damage_hit(
    state: &mut State,
    choice: &Choice,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
    target_side_ref: &SideReference,
    target_slot_ref: &SlotReference,
    instructions: &mut StateInstructions,
    hit_sub: bool,
) {
    let target_side = state.get_side(target_side_ref);
    // only increment times_attacked if they have ragefist, otherwise it's a waste of an instruction
    let (target_pkmn, target_active_index) = target_side.get_active_with_index(target_slot_ref);
    if target_pkmn.has_move(&Choices::RAGEFIST) && target_pkmn.times_attacked < 6 && !hit_sub {
        instructions
            .instruction_list
            .push(Instruction::IncrementTimesAttacked(
                IncrementTimesAttackedInstruction {
                    side_ref: *target_side_ref,
                    pokemon_index: target_active_index,
                },
            ));
        target_pkmn.times_attacked += 1;
    }

    let attacking_side = state.get_side(attacking_side_ref);
    if choice.flags.recharge {
        let instruction = Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
            side_ref: *attacking_side_ref,
            slot_ref: *attacking_slot_ref,
            volatile_status: PokemonVolatileStatus::MUSTRECHARGE,
        });
        instructions.instruction_list.push(instruction);
        attacking_side
            .get_slot(attacking_slot_ref)
            .volatile_statuses
            .insert(PokemonVolatileStatus::MUSTRECHARGE);

    // Recharging and truant are mutually exclusive, with recharge taking priority
    } else if attacking_side.get_active(attacking_slot_ref).ability == Abilities::TRUANT {
        let instruction = Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
            side_ref: attacking_side_ref.clone(),
            slot_ref: *attacking_slot_ref,
            volatile_status: PokemonVolatileStatus::TRUANT,
        });
        instructions.instruction_list.push(instruction);
        attacking_side
            .get_slot(attacking_slot_ref)
            .volatile_statuses
            .insert(PokemonVolatileStatus::TRUANT);
    }
    match choice.move_id {
        Choices::DOUBLESHOCK => {
            let (attacker_active, attacker_index) =
                attacking_side.get_active_with_index(attacking_slot_ref);
            let instruction = if attacker_active.types.0 == PokemonType::ELECTRIC {
                Some(Instruction::ChangeType(ChangeType {
                    side_ref: *attacking_side_ref,
                    pokemon_index: attacker_index,
                    new_types: (PokemonType::TYPELESS, attacker_active.types.1),
                    old_types: attacker_active.types,
                }))
            } else if attacker_active.types.1 == PokemonType::ELECTRIC {
                Some(Instruction::ChangeType(ChangeType {
                    side_ref: *attacking_side_ref,
                    pokemon_index: attacker_index,
                    new_types: (attacker_active.types.0, PokemonType::TYPELESS),
                    old_types: attacker_active.types,
                }))
            } else {
                None
            };
            if let Some(typechange_instruction) = instruction {
                if !attacking_side
                    .get_slot(attacking_slot_ref)
                    .volatile_statuses
                    .contains(&PokemonVolatileStatus::TYPECHANGE)
                {
                    instructions
                        .instruction_list
                        .push(Instruction::ApplyVolatileStatus(
                            ApplyVolatileStatusInstruction {
                                side_ref: *attacking_side_ref,
                                slot_ref: *attacking_slot_ref,
                                volatile_status: PokemonVolatileStatus::TYPECHANGE,
                            },
                        ));
                    attacking_side
                        .get_slot(attacking_slot_ref)
                        .volatile_statuses
                        .insert(PokemonVolatileStatus::TYPECHANGE);
                }
                state.apply_one_instruction(&typechange_instruction);
                instructions.instruction_list.push(typechange_instruction);
            }
        }
        Choices::BURNUP => {
            let (attacker_active, attacker_index) =
                attacking_side.get_active_with_index(attacking_slot_ref);
            let instruction = if attacker_active.types.0 == PokemonType::FIRE {
                Some(Instruction::ChangeType(ChangeType {
                    side_ref: *attacking_side_ref,
                    pokemon_index: attacker_index,
                    new_types: (PokemonType::TYPELESS, attacker_active.types.1),
                    old_types: attacker_active.types,
                }))
            } else if attacker_active.types.1 == PokemonType::FIRE {
                Some(Instruction::ChangeType(ChangeType {
                    side_ref: *attacking_side_ref,
                    pokemon_index: attacker_index,
                    new_types: (attacker_active.types.0, PokemonType::TYPELESS),
                    old_types: attacker_active.types,
                }))
            } else {
                None
            };
            if let Some(typechange_instruction) = instruction {
                let attacking_slot = attacking_side.get_slot(attacking_slot_ref);
                if !attacking_slot
                    .volatile_statuses
                    .contains(&PokemonVolatileStatus::TYPECHANGE)
                {
                    instructions
                        .instruction_list
                        .push(Instruction::ApplyVolatileStatus(
                            ApplyVolatileStatusInstruction {
                                side_ref: attacking_side_ref.clone(),
                                slot_ref: *attacking_slot_ref,
                                volatile_status: PokemonVolatileStatus::TYPECHANGE,
                            },
                        ));
                    attacking_slot
                        .volatile_statuses
                        .insert(PokemonVolatileStatus::TYPECHANGE);
                }
                state.apply_one_instruction(&typechange_instruction);
                instructions.instruction_list.push(typechange_instruction);
            }
        }
        Choices::RAGINGBULL => {
            let target_side = state.get_side(target_side_ref);
            if target_side.side_conditions.reflect > 0 {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeSideCondition(
                        ChangeSideConditionInstruction {
                            side_ref: *target_side_ref,
                            side_condition: PokemonSideCondition::Reflect,
                            amount: -1 * target_side.side_conditions.reflect,
                        },
                    ));
                target_side.side_conditions.reflect = 0;
            }
            if target_side.side_conditions.light_screen > 0 {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeSideCondition(
                        ChangeSideConditionInstruction {
                            side_ref: *target_side_ref,
                            side_condition: PokemonSideCondition::LightScreen,
                            amount: -1 * target_side.side_conditions.light_screen,
                        },
                    ));
                target_side.side_conditions.light_screen = 0;
            }
            if target_side.side_conditions.aurora_veil > 0 {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeSideCondition(
                        ChangeSideConditionInstruction {
                            side_ref: attacking_side_ref.get_other_side(),
                            side_condition: PokemonSideCondition::AuroraVeil,
                            amount: -1 * target_side.side_conditions.aurora_veil,
                        },
                    ));
                target_side.side_conditions.aurora_veil = 0;
            }
        }
        Choices::KNOCKOFF => {
            let target_side = state.get_side(target_side_ref);
            let (target_active, target_active_index) =
                target_side.get_active_with_index(target_slot_ref);
            if target_active.item_can_be_removed() && target_active.item != Items::NONE && !hit_sub
            {
                let instruction = Instruction::ChangeItem(ChangeItemInstruction {
                    side_ref: *target_side_ref,
                    pokemon_index: target_active_index,
                    current_item: target_active.item,
                    new_item: Items::NONE,
                });
                instructions.instruction_list.push(instruction);
                target_active.item = Items::NONE;
            }
        }
        Choices::THIEF
            if attacking_side.get_active_immutable(attacking_slot_ref).item == Items::NONE =>
        {
            let (target_active, target_active_index) = state
                .get_side(target_side_ref)
                .get_active_with_index(target_slot_ref);
            if target_active.item_can_be_removed() && target_active.item != Items::NONE && !hit_sub
            {
                let target_item = target_active.item;

                let instruction = Instruction::ChangeItem(ChangeItemInstruction {
                    side_ref: *target_side_ref,
                    pokemon_index: target_active_index,
                    current_item: target_item,
                    new_item: Items::NONE,
                });
                instructions.instruction_list.push(instruction);
                target_active.item = Items::NONE;

                let (attacker_active, attacker_active_index) = state
                    .get_side(attacking_side_ref)
                    .get_active_with_index(attacking_slot_ref);
                let instruction = Instruction::ChangeItem(ChangeItemInstruction {
                    side_ref: *attacking_side_ref,
                    pokemon_index: attacker_active_index,
                    current_item: Items::NONE,
                    new_item: target_item,
                });
                instructions.instruction_list.push(instruction);
                attacker_active.item = target_item;
            }
        }
        Choices::CLEARSMOG => {
            state.reset_boosts(
                target_side_ref,
                target_slot_ref,
                &mut instructions.instruction_list,
            );
        }
        Choices::ICESPINNER => {
            if state.terrain.terrain_type != Terrain::NONE && state.terrain.turns_remaining > 0 {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeTerrain(ChangeTerrain {
                        new_terrain: Terrain::NONE,
                        new_terrain_turns_remaining: 0,
                        previous_terrain: state.terrain.terrain_type,
                        previous_terrain_turns_remaining: state.terrain.turns_remaining,
                    }));
                state.terrain.terrain_type = Terrain::NONE;
                state.terrain.turns_remaining = 0;
            }
        }
        _ => {}
    }
}

fn destinybond_before_move(
    attacking_side: &mut Side,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
    choice: &mut Choice,
    instructions: &mut StateInstructions,
) {
    // gens 7+ destinybond cannot be used if destinybond is active
    let attacking_slot = attacking_side.get_slot(attacking_slot_ref);
    if attacking_slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::DESTINYBOND)
    {
        instructions
            .instruction_list
            .push(Instruction::RemoveVolatileStatus(
                RemoveVolatileStatusInstruction {
                    side_ref: *attacking_side_ref,
                    slot_ref: *attacking_slot_ref,
                    volatile_status: PokemonVolatileStatus::DESTINYBOND,
                },
            ));
        attacking_slot
            .volatile_statuses
            .remove(&PokemonVolatileStatus::DESTINYBOND);
        if choice.move_id == Choices::DESTINYBOND {
            choice.remove_all_effects();
        }
    }
}

pub fn choice_before_move(
    state: &mut State,
    choice: &mut Choice,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
    target_side_ref: &SideReference,
    instructions: &mut StateInstructions,
) {
    let defender_ability = state
        .get_side_immutable(target_side_ref)
        .get_active_immutable(attacking_slot_ref)
        .ability;
    let attacking_side = state.get_side(attacking_side_ref);
    destinybond_before_move(
        attacking_side,
        attacking_side_ref,
        attacking_slot_ref,
        choice,
        instructions,
    );

    if attacking_side
        .get_active_immutable(attacking_slot_ref)
        .status
        == PokemonStatus::FREEZE
        && CHOICE_THAWS_USER.contains(&choice.move_id)
    {
        add_remove_status_instructions(
            instructions,
            attacking_side
                .get_slot_immutable(attacking_slot_ref)
                .active_index,
            *attacking_side_ref,
            attacking_side,
        );
    }

    let (attacker, attacker_index) = attacking_side.get_active_with_index(attacking_slot_ref);
    match choice.move_id {
        // TODO: Futuresight needs to be re-implemented for doubles
        // Choices::FUTURESIGHT => {
        //     choice.remove_all_effects();
        //     if attacking_side.future_sight.0 == 0 {
        //         instructions
        //             .instruction_list
        //             .push(Instruction::SetFutureSight(SetFutureSightInstruction {
        //                 side_ref: *attacking_side_ref,
        //                 pokemon_index: attacking_side.active_index,
        //                 previous_pokemon_index: attacking_side.future_sight.1,
        //             }));
        //         attacking_side.future_sight = (3, attacking_side.active_index);
        //     }
        // }
        Choices::PARTINGSHOT => {
            let target_pkmn = state
                .get_side_immutable(target_side_ref)
                .get_active_immutable(attacking_slot_ref);
            if [
                Abilities::CLEARBODY,
                Abilities::WHITESMOKE,
                Abilities::FULLMETALBODY,
            ]
            .contains(&target_pkmn.ability)
                || ([Items::CLEARAMULET].contains(&target_pkmn.item))
            {
                choice.remove_all_effects();
            }
        }
        Choices::EXPLOSION | Choices::SELFDESTRUCT | Choices::MISTYEXPLOSION
            if defender_ability != Abilities::DAMP =>
        {
            let damage_amount = attacker.hp;
            instructions
                .instruction_list
                .push(Instruction::Damage(DamageInstruction {
                    side_ref: *attacking_side_ref,
                    pokemon_index: attacker_index,
                    damage_amount,
                }));
            attacker.hp = 0;
        }
        Choices::MINDBLOWN if defender_ability != Abilities::DAMP => {
            let damage_amount = cmp::min(attacker.maxhp / 2, attacker.hp);
            instructions
                .instruction_list
                .push(Instruction::Damage(DamageInstruction {
                    side_ref: *attacking_side_ref,
                    pokemon_index: attacker_index,
                    damage_amount,
                }));
            attacker.hp -= damage_amount;
        }
        Choices::METEORBEAM | Choices::ELECTROSHOT if choice.flags.charge => {
            apply_boost_instructions(
                attacking_side,
                &PokemonBoostableStat::SpecialAttack,
                &1,
                attacking_side_ref,
                attacking_side_ref,
                attacking_slot_ref,
                instructions,
            );
        }
        _ => {}
    }
    let attacking_side = state.get_side(attacking_side_ref);
    let attacker = attacking_side.get_active(attacking_slot_ref);
    if choice.flags.charge
        && attacker.item == Items::POWERHERB
        && choice.move_id != Choices::SKYDROP
    {
        let instruction = Instruction::ChangeItem(ChangeItemInstruction {
            side_ref: *attacking_side_ref,
            pokemon_index: attacker_index,
            current_item: Items::POWERHERB,
            new_item: Items::NONE,
        });
        attacker.item = Items::NONE;
        choice.flags.charge = false;
        instructions.instruction_list.push(instruction);
    }
    if let Some(choice_volatile_status) = &choice.volatile_status {
        if choice_volatile_status.volatile_status == PokemonVolatileStatus::LOCKEDMOVE
            && choice_volatile_status.target == MoveTarget::User
        {
            let ins = get_choice_move_disable_instructions(
                attacker,
                attacker_index,
                attacking_side_ref,
                &choice.move_id,
            );
            for i in ins {
                state.apply_one_instruction(&i);
                instructions.instruction_list.push(i);
            }
        }
    }
}

pub fn choice_hazard_clear(
    state: &mut State,
    choice: &Choice,
    attacking_side_ref: &SideReference,
    target_side_ref: &SideReference,
    target_slot_ref: &SlotReference,
    instructions: &mut StateInstructions,
) {
    let target_ability = state
        .get_side_immutable(target_side_ref)
        .get_active_immutable(target_slot_ref)
        .ability;
    let attacking_side = state.get_side(attacking_side_ref);
    match choice.move_id {
        Choices::COURTCHANGE => {
            let mut instruction_list = vec![];
            let courtchange_swaps = [
                PokemonSideCondition::Stealthrock,
                PokemonSideCondition::Spikes,
                PokemonSideCondition::ToxicSpikes,
                PokemonSideCondition::StickyWeb,
                PokemonSideCondition::Reflect,
                PokemonSideCondition::LightScreen,
                PokemonSideCondition::AuroraVeil,
                PokemonSideCondition::Tailwind,
            ];

            for side in [SideReference::SideOne, SideReference::SideTwo] {
                for side_condition in courtchange_swaps {
                    let side_condition_num = state
                        .get_side_immutable(&side)
                        .get_side_condition(side_condition);
                    if side_condition_num > 0 {
                        instruction_list.push(Instruction::ChangeSideCondition(
                            ChangeSideConditionInstruction {
                                side_ref: side,
                                side_condition,
                                amount: -1 * side_condition_num,
                            },
                        ));
                        instruction_list.push(Instruction::ChangeSideCondition(
                            ChangeSideConditionInstruction {
                                side_ref: side.get_other_side(),
                                side_condition,
                                amount: side_condition_num,
                            },
                        ));
                    }
                }
            }
            state.apply_instructions(&instruction_list);
            for i in instruction_list {
                instructions.instruction_list.push(i)
            }
        }
        Choices::DEFOG if target_ability != Abilities::GOODASGOLD => {
            if state.terrain.terrain_type != Terrain::NONE {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeTerrain(ChangeTerrain {
                        new_terrain: Terrain::NONE,
                        new_terrain_turns_remaining: 0,
                        previous_terrain: state.terrain.terrain_type,
                        previous_terrain_turns_remaining: state.terrain.turns_remaining,
                    }));
                state.terrain.terrain_type = Terrain::NONE;
                state.terrain.turns_remaining = 0;
            }
            let side_condition_clears = [
                PokemonSideCondition::Stealthrock,
                PokemonSideCondition::Spikes,
                PokemonSideCondition::ToxicSpikes,
                PokemonSideCondition::StickyWeb,
                PokemonSideCondition::Reflect,
                PokemonSideCondition::LightScreen,
                PokemonSideCondition::AuroraVeil,
            ];

            for side in [SideReference::SideOne, SideReference::SideTwo] {
                for side_condition in side_condition_clears {
                    let side_condition_num = state
                        .get_side_immutable(&side)
                        .get_side_condition(side_condition);
                    if side_condition_num > 0 {
                        let i = Instruction::ChangeSideCondition(ChangeSideConditionInstruction {
                            side_ref: side,
                            side_condition,
                            amount: -1 * side_condition_num,
                        });
                        state.apply_one_instruction(&i);
                        instructions.instruction_list.push(i)
                    }
                }
            }
        }
        Choices::TIDYUP => {
            let side_condition_clears = [
                PokemonSideCondition::Stealthrock,
                PokemonSideCondition::Spikes,
                PokemonSideCondition::ToxicSpikes,
                PokemonSideCondition::StickyWeb,
            ];
            for side in [SideReference::SideOne, SideReference::SideTwo] {
                for side_condition in side_condition_clears {
                    let side_condition_num = state
                        .get_side_immutable(&side)
                        .get_side_condition(side_condition);
                    if side_condition_num > 0 {
                        let i = Instruction::ChangeSideCondition(ChangeSideConditionInstruction {
                            side_ref: side,
                            side_condition,
                            amount: -1 * side_condition_num,
                        });
                        state.apply_one_instruction(&i);
                        instructions.instruction_list.push(i)
                    }
                }
            }
            for (side_r, slot_r) in State::get_all_sides_and_slots() {
                let slot = state.get_side(&side_r).get_slot(&slot_r);
                if slot
                    .volatile_statuses
                    .contains(&PokemonVolatileStatus::SUBSTITUTE)
                {
                    instructions
                        .instruction_list
                        .push(Instruction::ChangeSubstituteHealth(
                            ChangeSubsituteHealthInstruction {
                                side_ref: side_r,
                                slot_ref: slot_r,
                                health_change: -1 * slot.substitute_health,
                            },
                        ));
                    instructions
                        .instruction_list
                        .push(Instruction::RemoveVolatileStatus(
                            RemoveVolatileStatusInstruction {
                                side_ref: side_r,
                                slot_ref: slot_r,
                                volatile_status: PokemonVolatileStatus::SUBSTITUTE,
                            },
                        ));
                    slot.substitute_health = 0;
                    slot.volatile_statuses
                        .remove(&PokemonVolatileStatus::SUBSTITUTE);
                }
            }
        }
        Choices::RAPIDSPIN | Choices::MORTALSPIN => {
            if attacking_side.side_conditions.stealth_rock > 0 {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeSideCondition(
                        ChangeSideConditionInstruction {
                            side_ref: *attacking_side_ref,
                            side_condition: PokemonSideCondition::Stealthrock,
                            amount: -1 * attacking_side.side_conditions.stealth_rock,
                        },
                    ));
                attacking_side.side_conditions.stealth_rock = 0;
            }
            if attacking_side.side_conditions.spikes > 0 {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeSideCondition(
                        ChangeSideConditionInstruction {
                            side_ref: *attacking_side_ref,
                            side_condition: PokemonSideCondition::Spikes,
                            amount: -1 * attacking_side.side_conditions.spikes,
                        },
                    ));
                attacking_side.side_conditions.spikes = 0;
            }
            if attacking_side.side_conditions.toxic_spikes > 0 {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeSideCondition(
                        ChangeSideConditionInstruction {
                            side_ref: *attacking_side_ref,
                            side_condition: PokemonSideCondition::ToxicSpikes,
                            amount: -1 * attacking_side.side_conditions.toxic_spikes,
                        },
                    ));
                attacking_side.side_conditions.toxic_spikes = 0;
            }
            if attacking_side.side_conditions.sticky_web > 0 {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeSideCondition(
                        ChangeSideConditionInstruction {
                            side_ref: *attacking_side_ref,
                            side_condition: PokemonSideCondition::StickyWeb,
                            amount: -1 * attacking_side.side_conditions.sticky_web,
                        },
                    ));
                attacking_side.side_conditions.sticky_web = 0;
            }
        }
        _ => {}
    }
}

pub fn choice_special_effect(
    state: &mut State,
    choice: &mut Choice,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
    target_side_ref: &SideReference,
    target_slot_ref: &SlotReference,
    instructions: &mut StateInstructions,
) {
    let attacking_side = state.get_side(attacking_side_ref);
    match choice.move_id {
        Choices::TAILWIND => {
            let partner_slot_ref = attacking_slot_ref.get_other_slot();
            let ally_pkmn = attacking_side.get_active_immutable(&partner_slot_ref);
            if ally_pkmn.ability == Abilities::WINDRIDER {
                instructions
                    .instruction_list
                    .push(Instruction::Boost(BoostInstruction {
                        side_ref: *attacking_side_ref,
                        slot_ref: partner_slot_ref,
                        stat: PokemonBoostableStat::Attack,
                        amount: 1,
                    }));
                attacking_side.get_slot(&partner_slot_ref).attack_boost += 1;
            }
        }
        Choices::LIFEDEW => {
            get_instructions_from_heal(
                state,
                &Heal {
                    target: MoveTarget::Target,
                    amount: 0.25,
                },
                attacking_side_ref,
                attacking_slot_ref,
                attacking_side_ref,
                attacking_slot_ref,
                instructions,
            );
            get_instructions_from_heal(
                state,
                &Heal {
                    target: MoveTarget::Target,
                    amount: 0.25,
                },
                attacking_side_ref,
                attacking_slot_ref,
                attacking_side_ref,
                &attacking_slot_ref.get_other_slot(),
                instructions,
            );
        }
        Choices::JUNGLEHEALING | Choices::LUNARBLESSING => {
            if attacking_side.pokemon[attacking_side.slot_a.active_index].status
                != PokemonStatus::NONE
            {
                add_remove_status_instructions(
                    instructions,
                    attacking_side.slot_a.active_index,
                    *attacking_side_ref,
                    attacking_side,
                );
            }
            if attacking_side.pokemon[attacking_side.slot_b.active_index].status
                != PokemonStatus::NONE
            {
                add_remove_status_instructions(
                    instructions,
                    attacking_side.slot_b.active_index,
                    *attacking_side_ref,
                    attacking_side,
                );
            }
            get_instructions_from_heal(
                state,
                &Heal {
                    target: MoveTarget::Target,
                    amount: 0.25,
                },
                attacking_side_ref,
                attacking_slot_ref,
                attacking_side_ref,
                attacking_slot_ref,
                instructions,
            );
            get_instructions_from_heal(
                state,
                &Heal {
                    target: MoveTarget::Target,
                    amount: 0.25,
                },
                attacking_side_ref,
                attacking_slot_ref,
                attacking_side_ref,
                &attacking_slot_ref.get_other_slot(),
                instructions,
            );
        }
        Choices::POLLENPUFF => {
            if attacking_side_ref == target_side_ref {
                choice.category = MoveCategory::Status;
                choice.base_power = 0.0;
                choice.accuracy = 100.0;
                choice.heal = Some(Heal {
                    target: MoveTarget::Target,
                    amount: 0.5,
                });
            }
        }
        Choices::FLORALHEALING => {
            if state.terrain_is_active(&Terrain::GRASSYTERRAIN) {
                choice.heal = Some(Heal {
                    target: MoveTarget::Target,
                    amount: 2.0 / 3.0,
                });
            }
        }
        Choices::BELLYDRUM => {
            let (attacker, attacker_index) =
                attacking_side.get_active_with_index(attacking_slot_ref);
            if attacker.hp > attacker.maxhp / 2 {
                instructions
                    .instruction_list
                    .push(Instruction::Damage(DamageInstruction {
                        side_ref: *attacking_side_ref,
                        pokemon_index: attacker_index,
                        damage_amount: attacker.maxhp / 2,
                    }));
                attacker.hp -= attacker.maxhp / 2;

                let attacking_slot = attacking_side.get_slot(attacking_slot_ref);
                let boost_amount = 6 - attacking_slot.attack_boost;
                attacking_slot.attack_boost = 6;
                instructions
                    .instruction_list
                    .push(Instruction::Boost(BoostInstruction {
                        side_ref: *attacking_side_ref,
                        slot_ref: *attacking_slot_ref,
                        stat: PokemonBoostableStat::Attack,
                        amount: boost_amount,
                    }));
            }
        }
        Choices::COUNTER => {
            let target_side = state.get_side(target_side_ref);
            if target_side
                .get_active_immutable(target_slot_ref)
                .has_type(&PokemonType::GHOST)
            {
                return;
            }
            let target_slot = target_side.get_slot_immutable(target_slot_ref);
            if target_slot.damage_dealt.move_category != MoveCategory::Physical
                || target_slot.damage_dealt.damage == 0
            {
                return;
            }
            let damage_amount = cmp::min(
                target_slot.damage_dealt.damage * 2,
                target_side.get_active_immutable(target_slot_ref).hp,
            );
            let (target_active, target_active_index) =
                target_side.get_active_with_index(target_slot_ref);
            instructions
                .instruction_list
                .push(Instruction::Damage(DamageInstruction {
                    side_ref: *target_side_ref,
                    pokemon_index: target_active_index,
                    damage_amount,
                }));
            target_active.hp -= damage_amount;
        }
        Choices::MIRRORCOAT => {
            let target_side = state.get_side(target_side_ref);
            if target_side
                .get_active_immutable(target_slot_ref)
                .has_type(&PokemonType::DARK)
            {
                return;
            }
            let target_slot = target_side.get_slot_immutable(target_slot_ref);
            if target_slot.damage_dealt.move_category != MoveCategory::Special
                || target_slot.damage_dealt.damage == 0
            {
                return;
            }
            let damage_amount = cmp::min(
                target_slot.damage_dealt.damage * 2,
                target_side.get_active_immutable(target_slot_ref).hp,
            );
            let (target_active, target_active_index) =
                target_side.get_active_with_index(target_slot_ref);
            instructions
                .instruction_list
                .push(Instruction::Damage(DamageInstruction {
                    side_ref: *target_side_ref,
                    pokemon_index: target_active_index,
                    damage_amount,
                }));
            target_active.hp -= damage_amount;
        }
        Choices::METALBURST | Choices::COMEUPPANCE => {
            let target_side = state.get_side(target_side_ref);
            let target_slot = target_side.get_slot_immutable(target_slot_ref);
            let target_slot_damage_dealt = target_slot.damage_dealt.damage;
            if target_slot.damage_dealt.move_category == MoveCategory::Status
                || target_slot.damage_dealt.hit_substitute
                || choice.first_move
                || target_slot_damage_dealt == 0
            {
                return;
            }
            let (target_active, target_active_index) =
                target_side.get_active_with_index(target_slot_ref);
            let damage_amount = cmp::min((target_slot_damage_dealt * 3) / 2, target_active.hp);
            instructions
                .instruction_list
                .push(Instruction::Damage(DamageInstruction {
                    side_ref: *target_side_ref,
                    pokemon_index: target_active_index,
                    damage_amount,
                }));
            target_active.hp -= damage_amount;
        }
        Choices::WISH => {
            let attacker_maxhp = attacking_side
                .get_active_immutable(attacking_slot_ref)
                .maxhp;
            let attacking_slot = attacking_side.get_slot(attacking_slot_ref);
            if attacking_slot.wish.0 == 0 {
                instructions.instruction_list.push(Instruction::ChangeWish(
                    ChangeWishInstruction {
                        side_ref: *attacking_side_ref,
                        slot_ref: *attacking_slot_ref,
                        wish_amount_change: attacker_maxhp / 2 - attacking_slot.wish.1,
                    },
                ));
                attacking_slot.wish = (2, attacker_maxhp / 2);
            }
        }
        Choices::REFRESH => {
            let (active_pkmn, active_index) =
                attacking_side.get_active_with_index(attacking_slot_ref);
            if active_pkmn.status != PokemonStatus::NONE {
                add_remove_status_instructions(
                    instructions,
                    active_index,
                    *attacking_side_ref,
                    attacking_side,
                );
            }
        }
        Choices::HEALBELL | Choices::AROMATHERAPY => {
            for pkmn_index in pokemon_index_iter() {
                if attacking_side.pokemon[pkmn_index].status != PokemonStatus::NONE {
                    add_remove_status_instructions(
                        instructions,
                        pkmn_index,
                        *attacking_side_ref,
                        attacking_side,
                    );
                }
            }
        }
        Choices::HAZE => {
            state.reset_boosts(
                &SideReference::SideOne,
                &SlotReference::SlotA,
                &mut instructions.instruction_list,
            );
            state.reset_boosts(
                &SideReference::SideOne,
                &SlotReference::SlotB,
                &mut instructions.instruction_list,
            );
            state.reset_boosts(
                &SideReference::SideTwo,
                &SlotReference::SlotA,
                &mut instructions.instruction_list,
            );
            state.reset_boosts(
                &SideReference::SideTwo,
                &SlotReference::SlotB,
                &mut instructions.instruction_list,
            );
        }
        Choices::REST => {
            let (active_pkmn, active_index) =
                attacking_side.get_active_with_index(attacking_slot_ref);
            if active_pkmn.status != PokemonStatus::SLEEP {
                let heal_amount = active_pkmn.maxhp - active_pkmn.hp;
                instructions
                    .instruction_list
                    .push(Instruction::ChangeStatus(ChangeStatusInstruction {
                        side_ref: *attacking_side_ref,
                        pokemon_index: active_index,
                        old_status: active_pkmn.status,
                        new_status: PokemonStatus::SLEEP,
                    }));
                instructions
                    .instruction_list
                    .push(Instruction::SetRestTurns(SetSleepTurnsInstruction {
                        side_ref: *attacking_side_ref,
                        pokemon_index: active_index,
                        new_turns: 3,
                        previous_turns: active_pkmn.rest_turns,
                    }));
                instructions
                    .instruction_list
                    .push(Instruction::Heal(HealInstruction {
                        side_ref: *attacking_side_ref,
                        pokemon_index: active_index,
                        heal_amount,
                    }));
                active_pkmn.hp = active_pkmn.maxhp;
                active_pkmn.status = PokemonStatus::SLEEP;
                active_pkmn.rest_turns = 3;
            }
        }
        Choices::TRICKROOM => {
            let new_turns_remaining;
            if state.trick_room.active {
                new_turns_remaining = 0;
            } else {
                new_turns_remaining = 5;
            }
            instructions
                .instruction_list
                .push(Instruction::ToggleTrickRoom(ToggleTrickRoomInstruction {
                    currently_active: state.trick_room.active,
                    new_trickroom_turns_remaining: new_turns_remaining,
                    previous_trickroom_turns_remaining: state.trick_room.turns_remaining,
                }));
            state.trick_room.active = !state.trick_room.active;
        }
        Choices::SUPERFANG | Choices::NATURESMADNESS | Choices::RUINATION => {
            let target_side = state.get_side(target_side_ref);
            let (target_pkmn, target_active_index) =
                target_side.get_active_with_index(target_slot_ref);
            if target_pkmn.hp == 1 {
                return;
            }
            if choice.move_id == Choices::SUPERFANG
                && type_effectiveness_modifier(&PokemonType::NORMAL, &target_pkmn) == 0.0
            {
                return;
            }
            let target_hp = target_pkmn.hp / 2;
            instructions
                .instruction_list
                .push(Instruction::Damage(DamageInstruction {
                    side_ref: *target_side_ref,
                    pokemon_index: target_active_index,
                    damage_amount: target_pkmn.hp - target_hp,
                }));
            target_pkmn.hp = target_hp;
        }
        Choices::NIGHTSHADE => {
            let attacker_level = attacking_side
                .get_active_immutable(attacking_slot_ref)
                .level;
            let target_side = state.get_side(target_side_ref);
            let (target_pkmn, target_active_index) =
                target_side.get_active_with_index(target_slot_ref);
            if type_effectiveness_modifier(&PokemonType::GHOST, &target_pkmn) == 0.0 {
                return;
            }

            let damage_amount = cmp::min(attacker_level as i16, target_pkmn.hp);
            instructions
                .instruction_list
                .push(Instruction::Damage(DamageInstruction {
                    side_ref: *target_side_ref,
                    pokemon_index: target_active_index,
                    damage_amount,
                }));
            target_pkmn.hp -= damage_amount;
        }
        Choices::SEISMICTOSS => {
            let attacker_level = attacking_side
                .get_active_immutable(attacking_slot_ref)
                .level;
            let target_side = state.get_side(target_side_ref);
            let (target_pkmn, target_active_index) =
                target_side.get_active_with_index(target_slot_ref);
            if type_effectiveness_modifier(&PokemonType::NORMAL, &target_pkmn) == 0.0 {
                return;
            }

            let damage_amount = cmp::min(attacker_level as i16, target_pkmn.hp);
            instructions
                .instruction_list
                .push(Instruction::Damage(DamageInstruction {
                    side_ref: *target_side_ref,
                    pokemon_index: target_active_index,
                    damage_amount,
                }));
            target_pkmn.hp -= damage_amount;
        }
        Choices::ENDEAVOR => {
            let attacker_hp = attacking_side.get_active_immutable(attacking_slot_ref).hp;
            let target_side = state.get_side(target_side_ref);
            let (target_pkmn, target_active_index) =
                target_side.get_active_with_index(target_slot_ref);

            if type_effectiveness_modifier(&PokemonType::NORMAL, &target_pkmn) == 0.0
                || attacker_hp >= target_pkmn.hp
            {
                return;
            }

            let damage_amount = target_pkmn.hp - attacker_hp;
            instructions
                .instruction_list
                .push(Instruction::Damage(DamageInstruction {
                    side_ref: *target_side_ref,
                    pokemon_index: target_active_index,
                    damage_amount,
                }));
            target_pkmn.hp -= damage_amount;
        }
        Choices::FINALGAMBIT => {
            let attacker_hp = attacking_side.get_active_immutable(attacking_slot_ref).hp;
            let target_side = state.get_side(target_side_ref);
            let (target_pkmn, target_active_index) =
                target_side.get_active_with_index(target_slot_ref);

            if type_effectiveness_modifier(&PokemonType::NORMAL, &target_pkmn) == 0.0 {
                return;
            }

            let damage_amount = attacker_hp;
            instructions
                .instruction_list
                .push(Instruction::Damage(DamageInstruction {
                    side_ref: *target_side_ref,
                    pokemon_index: target_active_index,
                    damage_amount,
                }));
            target_pkmn.hp -= damage_amount;

            let (attacker, attacker_index) = state
                .get_side(attacking_side_ref)
                .get_active_with_index(attacking_slot_ref);
            instructions
                .instruction_list
                .push(Instruction::Damage(DamageInstruction {
                    side_ref: *attacking_side_ref,
                    pokemon_index: attacker_index,
                    damage_amount: attacker_hp,
                }));
            attacker.hp = 0;
        }
        Choices::PAINSPLIT => {
            let attacker_hp = attacking_side.get_active_immutable(attacking_slot_ref).hp;
            let target_hp = state
                .get_side(target_side_ref)
                .get_active_immutable(target_slot_ref)
                .hp;
            let average_hp = (attacker_hp + target_hp) / 2;
            let (attacker, attacker_index) = state
                .get_side(attacking_side_ref)
                .get_active_with_index(attacking_slot_ref);
            instructions
                .instruction_list
                .push(Instruction::Damage(DamageInstruction {
                    side_ref: *attacking_side_ref,
                    pokemon_index: attacker_index,
                    damage_amount: attacker.hp - average_hp,
                }));
            attacker.hp = average_hp;

            let (target, target_index) = state
                .get_side(target_side_ref)
                .get_active_with_index(target_slot_ref);
            instructions
                .instruction_list
                .push(Instruction::Damage(DamageInstruction {
                    side_ref: *target_side_ref,
                    pokemon_index: target_index,
                    damage_amount: target.hp - average_hp,
                }));
            target.hp = average_hp;
        }
        Choices::SUBSTITUTE | Choices::SHEDTAIL => {
            let attacking_slot = attacking_side.get_slot(attacking_slot_ref);
            if attacking_slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::SUBSTITUTE)
            {
                return;
            }
            let sub_current_health = attacking_slot.substitute_health;
            let (active_pkmn, active_index) =
                attacking_side.get_active_with_index(attacking_slot_ref);
            let sub_target_health = active_pkmn.maxhp / 4;
            let pkmn_health_reduction = if choice.move_id == Choices::SHEDTAIL {
                active_pkmn.maxhp / 2
            } else {
                sub_target_health
            };
            if active_pkmn.hp > pkmn_health_reduction {
                if choice.move_id == Choices::SHEDTAIL {
                    choice.flags.pivot = true;
                }

                let damage_instruction = Instruction::Damage(DamageInstruction {
                    side_ref: *attacking_side_ref,
                    pokemon_index: active_index,
                    damage_amount: pkmn_health_reduction,
                });
                let set_sub_health_instruction =
                    Instruction::ChangeSubstituteHealth(ChangeSubsituteHealthInstruction {
                        side_ref: *attacking_side_ref,
                        slot_ref: *attacking_slot_ref,
                        health_change: sub_target_health - sub_current_health,
                    });
                let apply_vs_instruction =
                    Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                        side_ref: *attacking_side_ref,
                        slot_ref: *attacking_slot_ref,
                        volatile_status: PokemonVolatileStatus::SUBSTITUTE,
                    });
                active_pkmn.hp -= pkmn_health_reduction;
                let attacking_slot = attacking_side.get_slot(attacking_slot_ref);
                attacking_slot.substitute_health = sub_target_health;
                attacking_slot
                    .volatile_statuses
                    .insert(PokemonVolatileStatus::SUBSTITUTE);
                instructions.instruction_list.push(damage_instruction);
                instructions
                    .instruction_list
                    .push(set_sub_health_instruction);
                instructions.instruction_list.push(apply_vs_instruction);
            }
        }
        Choices::PERISHSONG => {
            for (side_ref, slot_ref) in State::get_all_sides_and_slots() {
                let side = state.get_side(&side_ref);
                let slot = side.get_slot(&slot_ref);
                if slot
                    .volatile_statuses
                    .contains(&PokemonVolatileStatus::PERISH4)
                    || slot
                        .volatile_statuses
                        .contains(&PokemonVolatileStatus::PERISH3)
                    || slot
                        .volatile_statuses
                        .contains(&PokemonVolatileStatus::PERISH2)
                    || slot
                        .volatile_statuses
                        .contains(&PokemonVolatileStatus::PERISH1)
                {
                    continue;
                }
                let pkmn = side.get_active(&slot_ref);
                if pkmn.hp != 0 && pkmn.ability != Abilities::SOUNDPROOF {
                    instructions
                        .instruction_list
                        .push(Instruction::ApplyVolatileStatus(
                            ApplyVolatileStatusInstruction {
                                side_ref,
                                slot_ref,
                                volatile_status: PokemonVolatileStatus::PERISH4,
                            },
                        ));
                    side.get_slot(&slot_ref)
                        .volatile_statuses
                        .insert(PokemonVolatileStatus::PERISH4);
                }
            }
        }
        Choices::TRICK | Choices::SWITCHEROO => {
            let attacker_item = attacking_side.get_active(attacking_slot_ref).item;
            let target_side = state.get_side(target_side_ref);
            let target_has_sub = target_side
                .get_slot_immutable(target_slot_ref)
                .volatile_statuses
                .contains(&PokemonVolatileStatus::SUBSTITUTE);
            let target_item = target_side.get_active_immutable(target_slot_ref).item;
            let (target, target_index) = target_side.get_active_with_index(target_slot_ref);
            let target_item_can_be_removed = !target.item_can_be_removed();
            if attacker_item == target_item || target_item_can_be_removed || target_has_sub {
                return;
            }
            let change_target_item_instruction = Instruction::ChangeItem(ChangeItemInstruction {
                side_ref: *target_side_ref,
                pokemon_index: target_index,
                current_item: target_item,
                new_item: attacker_item,
            });
            instructions
                .instruction_list
                .push(change_target_item_instruction);
            target.item = attacker_item;

            let (attacker, attacker_index) = state
                .get_side(attacking_side_ref)
                .get_active_with_index(attacking_slot_ref);
            let change_attacker_item_instruction = Instruction::ChangeItem(ChangeItemInstruction {
                side_ref: *attacking_side_ref,
                pokemon_index: attacker_index,
                current_item: attacker_item,
                new_item: target_item,
            });
            attacker.item = target_item;
            instructions
                .instruction_list
                .push(change_attacker_item_instruction);
        }
        Choices::MISTYTERRAIN => {
            if state.terrain.terrain_type != Terrain::MISTYTERRAIN {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeTerrain(ChangeTerrain {
                        new_terrain: Terrain::MISTYTERRAIN,
                        new_terrain_turns_remaining: 5,
                        previous_terrain: state.terrain.terrain_type,
                        previous_terrain_turns_remaining: state.terrain.turns_remaining,
                    }));
                state.terrain.terrain_type = Terrain::MISTYTERRAIN;
                state.terrain.turns_remaining = 5;
            }
        }
        Choices::ELECTRICTERRAIN => {
            if state.terrain.terrain_type != Terrain::ELECTRICTERRAIN {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeTerrain(ChangeTerrain {
                        new_terrain: Terrain::ELECTRICTERRAIN,
                        new_terrain_turns_remaining: 5,
                        previous_terrain: state.terrain.terrain_type,
                        previous_terrain_turns_remaining: state.terrain.turns_remaining,
                    }));
                state.terrain.terrain_type = Terrain::ELECTRICTERRAIN;
                state.terrain.turns_remaining = 5;
            }
        }
        Choices::GRASSYTERRAIN => {
            if state.terrain.terrain_type != Terrain::GRASSYTERRAIN {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeTerrain(ChangeTerrain {
                        new_terrain: Terrain::GRASSYTERRAIN,
                        new_terrain_turns_remaining: 5,
                        previous_terrain: state.terrain.terrain_type,
                        previous_terrain_turns_remaining: state.terrain.turns_remaining,
                    }));
                state.terrain.terrain_type = Terrain::GRASSYTERRAIN;
                state.terrain.turns_remaining = 5;
            }
        }
        Choices::PSYCHICTERRAIN => {
            if state.terrain.terrain_type != Terrain::PSYCHICTERRAIN {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeTerrain(ChangeTerrain {
                        new_terrain: Terrain::PSYCHICTERRAIN,
                        new_terrain_turns_remaining: 5,
                        previous_terrain: state.terrain.terrain_type,
                        previous_terrain_turns_remaining: state.terrain.turns_remaining,
                    }));
                state.terrain.terrain_type = Terrain::PSYCHICTERRAIN;
                state.terrain.turns_remaining = 5;
            }
        }
        Choices::SUNNYDAY => {
            if state.weather.weather_type != Weather::SUN {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeWeather(ChangeWeather {
                        new_weather: Weather::SUN,
                        new_weather_turns_remaining: 5,
                        previous_weather: state.weather.weather_type,
                        previous_weather_turns_remaining: state.weather.turns_remaining,
                    }));
                state.weather.weather_type = Weather::SUN;
                state.weather.turns_remaining = 5;
            }
        }
        Choices::RAINDANCE => {
            if state.weather.weather_type != Weather::RAIN {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeWeather(ChangeWeather {
                        new_weather: Weather::RAIN,
                        new_weather_turns_remaining: 5,
                        previous_weather: state.weather.weather_type,
                        previous_weather_turns_remaining: state.weather.turns_remaining,
                    }));
                state.weather.weather_type = Weather::RAIN;
                state.weather.turns_remaining = 5;
            }
        }
        Choices::SANDSTORM => {
            if state.weather.weather_type != Weather::SAND {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeWeather(ChangeWeather {
                        new_weather: Weather::SAND,
                        new_weather_turns_remaining: 5,
                        previous_weather: state.weather.weather_type,
                        previous_weather_turns_remaining: state.weather.turns_remaining,
                    }));
                state.weather.weather_type = Weather::SAND;
                state.weather.turns_remaining = 5;
            }
        }
        Choices::HAIL => {
            if state.weather.weather_type != Weather::HAIL {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeWeather(ChangeWeather {
                        new_weather: Weather::HAIL,
                        new_weather_turns_remaining: 5,
                        previous_weather: state.weather.weather_type,
                        previous_weather_turns_remaining: state.weather.turns_remaining,
                    }));
                state.weather.weather_type = Weather::HAIL;
                state.weather.turns_remaining = 5;
            }
        }
        Choices::SNOWSCAPE | Choices::CHILLYRECEPTION => {
            if state.weather.weather_type != Weather::SNOW {
                instructions
                    .instruction_list
                    .push(Instruction::ChangeWeather(ChangeWeather {
                        new_weather: Weather::SNOW,
                        new_weather_turns_remaining: 5,
                        previous_weather: state.weather.weather_type,
                        previous_weather_turns_remaining: state.weather.turns_remaining,
                    }));
                state.weather.weather_type = Weather::SNOW;
                state.weather.turns_remaining = 5;
            }
        }
        _ => {}
    }
}

pub fn charge_choice_to_volatile(choice: &Choices) -> PokemonVolatileStatus {
    match choice {
        Choices::BOUNCE => PokemonVolatileStatus::BOUNCE,
        Choices::DIG => PokemonVolatileStatus::DIG,
        Choices::DIVE => PokemonVolatileStatus::DIVE,
        Choices::FLY => PokemonVolatileStatus::FLY,
        Choices::FREEZESHOCK => PokemonVolatileStatus::FREEZESHOCK,
        Choices::GEOMANCY => PokemonVolatileStatus::GEOMANCY,
        Choices::ICEBURN => PokemonVolatileStatus::ICEBURN,
        Choices::METEORBEAM => PokemonVolatileStatus::METEORBEAM,
        Choices::ELECTROSHOT => PokemonVolatileStatus::ELECTROSHOT,
        Choices::PHANTOMFORCE => PokemonVolatileStatus::PHANTOMFORCE,
        Choices::RAZORWIND => PokemonVolatileStatus::RAZORWIND,
        Choices::SHADOWFORCE => PokemonVolatileStatus::SHADOWFORCE,
        Choices::SKULLBASH => PokemonVolatileStatus::SKULLBASH,
        Choices::SKYATTACK => PokemonVolatileStatus::SKYATTACK,
        Choices::SKYDROP => PokemonVolatileStatus::SKYDROP,
        Choices::SOLARBEAM => PokemonVolatileStatus::SOLARBEAM,
        Choices::SOLARBLADE => PokemonVolatileStatus::SOLARBLADE,
        _ => {
            panic!("Invalid choice for charge: {:?}", choice)
        }
    }
}
//
pub fn charge_volatile_to_choice(volatile: &PokemonVolatileStatus) -> Option<Choices> {
    match volatile {
        PokemonVolatileStatus::BOUNCE => Some(Choices::BOUNCE),
        PokemonVolatileStatus::DIG => Some(Choices::DIG),
        PokemonVolatileStatus::DIVE => Some(Choices::DIVE),
        PokemonVolatileStatus::FLY => Some(Choices::FLY),
        PokemonVolatileStatus::FREEZESHOCK => Some(Choices::FREEZESHOCK),
        PokemonVolatileStatus::GEOMANCY => Some(Choices::GEOMANCY),
        PokemonVolatileStatus::ICEBURN => Some(Choices::ICEBURN),
        PokemonVolatileStatus::METEORBEAM => Some(Choices::METEORBEAM),
        PokemonVolatileStatus::ELECTROSHOT => Some(Choices::ELECTROSHOT),
        PokemonVolatileStatus::PHANTOMFORCE => Some(Choices::PHANTOMFORCE),
        PokemonVolatileStatus::RAZORWIND => Some(Choices::RAZORWIND),
        PokemonVolatileStatus::SHADOWFORCE => Some(Choices::SHADOWFORCE),
        PokemonVolatileStatus::SKULLBASH => Some(Choices::SKULLBASH),
        PokemonVolatileStatus::SKYATTACK => Some(Choices::SKYATTACK),
        PokemonVolatileStatus::SKYDROP => Some(Choices::SKYDROP),
        PokemonVolatileStatus::SOLARBEAM => Some(Choices::SOLARBEAM),
        PokemonVolatileStatus::SOLARBLADE => Some(Choices::SOLARBLADE),
        _ => None,
    }
}
