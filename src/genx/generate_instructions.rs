use super::abilities::{
    ability_after_damage_hit, ability_before_move, ability_change_type, ability_end_of_turn,
    ability_modify_attack_against, ability_modify_attack_being_used, ability_on_switch_in,
    ability_on_switch_out, commander_activating, Abilities,
};
use super::choice_effects::{
    charge_choice_to_volatile, choice_after_damage_hit, choice_before_move, choice_change_type,
    choice_hazard_clear, choice_special_effect, modify_choice,
};
use crate::choices::{
    Boost, Choices, Effect, Heal, MoveChoiceTarget, MoveTarget, MultiHitMove, Secondary,
    SideCondition, StatBoosts, Status, VolatileStatus, MOVES,
};
use crate::engine::items::{item_before_move, item_change_type, item_modify_attack_against};
use crate::engine::items::{item_end_of_turn, item_modify_attack_being_used};
use crate::instruction::{
    ApplyVolatileStatusInstruction, BoostInstruction, ChangeAbilityInstruction,
    ChangeDamageDealtDamageInstruction, ChangeDamageDealtMoveCategoryInstruction,
    ChangeItemInstruction, ChangeSideConditionInstruction, ChangeTerrain,
    ChangeVolatileStatusDurationInstruction, ChangeWeather, DecrementRestTurnsInstruction,
    HealInstruction, RemoveVolatileStatusInstruction, SetSleepTurnsInstruction,
    ToggleBatonPassingInstruction, ToggleDamageDealtHitSubstituteInstruction,
    ToggleShedTailingInstruction, ToggleTrickRoomInstruction,
};
use crate::instruction::{DamageSubstituteInstruction, ToggleTerastallizedInstruction};
use crate::instruction::{FormeChangeInstruction, SetLastUsedMoveInstruction};
use crate::instruction::{SetSecondMoveSwitchOutMoveInstruction, ToggleForceSwitchInstruction};

use super::damage_calc::{calculate_damage, type_effectiveness_modifier, DamageRolls};
use super::items::{item_on_switch_in, Items};
use super::state::{MoveChoice, PokemonVolatileStatus, Terrain, Weather};
use crate::choices::{Choice, MoveCategory};
use crate::instruction::{
    ChangeStatusInstruction, DamageInstruction, Instruction, StateInstructions, SwitchInstruction,
};
use crate::state::{
    LastUsedMove, PokemonBoostableStat, PokemonIndex, PokemonMoveIndex, PokemonSideCondition,
    PokemonStatus, PokemonType, Side, SideReference, SideSlot, SlotReference, State,
};
use std::cmp;

#[cfg(feature = "terastallization")]
use crate::choices::MultiAccuracyMove;
use crate::pokemon::PokemonName;

pub const BASE_CRIT_CHANCE: f32 = 1.0 / 24.0;
pub const MAX_SLEEP_TURNS: i8 = 3;
pub const HIT_SELF_IN_CONFUSION_CHANCE: f32 = 1.0 / 3.0;
pub const CONSECUTIVE_PROTECT_CHANCE: f32 = 1.0 / 3.0;
pub const SIDE_CONDITION_DURATION: i8 = 5;
pub const TAILWIND_DURATION: i8 = 4;

const PROTECT_VOLATILES: [PokemonVolatileStatus; 6] = [
    PokemonVolatileStatus::PROTECT,
    PokemonVolatileStatus::BANEFULBUNKER,
    PokemonVolatileStatus::BURNINGBULWARK,
    PokemonVolatileStatus::SPIKYSHIELD,
    PokemonVolatileStatus::SILKTRAP,
    PokemonVolatileStatus::ENDURE,
];

fn chance_to_wake_up(turns_asleep: i8) -> f32 {
    if turns_asleep == 0 {
        0.0
    } else {
        1.0 / (1 + MAX_SLEEP_TURNS - turns_asleep) as f32
    }
}

fn set_last_used_move_as_switch(
    slot: &mut SideSlot,
    slot_ref: &SlotReference,
    new_pokemon_index: PokemonIndex,
    switching_side_ref: SideReference,
    incoming_instructions: &mut StateInstructions,
) {
    incoming_instructions
        .instruction_list
        .push(Instruction::SetLastUsedMove(SetLastUsedMoveInstruction {
            side_ref: switching_side_ref,
            slot_ref: *slot_ref,
            last_used_move: LastUsedMove::Switch(new_pokemon_index),
            previous_last_used_move: slot.last_used_move,
        }));
    slot.last_used_move = LastUsedMove::Switch(new_pokemon_index);
}

fn set_last_used_move_as_move(
    side: &mut Side,
    slot_ref: SlotReference,
    used_move: PokemonMoveIndex,
    switching_side_ref: SideReference,
    incoming_instructions: &mut StateInstructions,
) {
    let slot = side.get_slot(&slot_ref);
    if slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::FLINCH)
    {
        // if we were flinched after just switching in we don't want our last used move to be switch
        // this makes sure fakeout/firstimpression can't be used on the following turn
        if matches!(slot.last_used_move, LastUsedMove::Switch(_)) {
            incoming_instructions
                .instruction_list
                .push(Instruction::SetLastUsedMove(SetLastUsedMoveInstruction {
                    side_ref: switching_side_ref,
                    slot_ref,
                    last_used_move: LastUsedMove::None,
                    previous_last_used_move: slot.last_used_move,
                }));
            slot.last_used_move = LastUsedMove::None;
        }
        return;
    }
    match slot.last_used_move {
        LastUsedMove::Move(last_used_move) => {
            if last_used_move == used_move {
                return;
            }
        }
        _ => {}
    }
    incoming_instructions
        .instruction_list
        .push(Instruction::SetLastUsedMove(SetLastUsedMoveInstruction {
            side_ref: switching_side_ref,
            slot_ref,
            last_used_move: LastUsedMove::Move(used_move),
            previous_last_used_move: slot.last_used_move,
        }));
    slot.last_used_move = LastUsedMove::Move(used_move);
}

fn generate_instructions_from_tera(
    state: &State,
    side_ref: &SideReference,
    slot_ref: &SlotReference,
    pokemon_index: PokemonIndex,
    incoming_instructions: &mut StateInstructions,
) {
    // Note: The state is immutable here on purpose. This function is called before instructions
    // are applied. This only generates instructions, they will be applied later.
    // As a consequence, do not branch here
    incoming_instructions
        .instruction_list
        .push(Instruction::ToggleTerastallized(
            ToggleTerastallizedInstruction {
                side_ref: *side_ref,
                pokemon_index,
            },
        ));

    let active_pkmn = state
        .get_side_immutable(side_ref)
        .get_active_immutable(slot_ref);
    if active_pkmn.id == PokemonName::TERAPAGOSTERASTAL
        && active_pkmn.ability == Abilities::TERASHELL
    {
        incoming_instructions
            .instruction_list
            .push(Instruction::FormeChange(FormeChangeInstruction {
                side_ref: *side_ref,
                pokemon_index,
                name_change: PokemonName::TERAPAGOSSTELLAR as i16
                    - PokemonName::TERAPAGOSTERASTAL as i16,
            }));
        active_pkmn.recalculate_stats_without_updating_stats(
            side_ref,
            pokemon_index,
            incoming_instructions,
        );
        incoming_instructions
            .instruction_list
            .push(Instruction::ChangeAbility(ChangeAbilityInstruction {
                side_ref: *side_ref,
                pokemon_index,
                ability_change: Abilities::TERAFORMZERO as i16 - Abilities::TERASHELL as i16,
            }));
        if state.weather.turns_remaining > 0 && state.weather.weather_type != Weather::NONE {
            incoming_instructions
                .instruction_list
                .push(Instruction::ChangeWeather(ChangeWeather {
                    new_weather: Weather::NONE,
                    new_weather_turns_remaining: 0,
                    previous_weather: state.weather.weather_type,
                    previous_weather_turns_remaining: state.weather.turns_remaining,
                }));
        }
        if state.terrain.turns_remaining > 0 && state.terrain.terrain_type != Terrain::NONE {
            incoming_instructions
                .instruction_list
                .push(Instruction::ChangeTerrain(ChangeTerrain {
                    new_terrain: Terrain::NONE,
                    new_terrain_turns_remaining: 0,
                    previous_terrain: state.terrain.terrain_type,
                    previous_terrain_turns_remaining: state.terrain.turns_remaining,
                }));
        }
    }
}

fn generate_instructions_from_switch(
    state: &mut State,
    slot_ref: &SlotReference,
    new_pokemon_index: PokemonIndex,
    switching_side_ref: SideReference,
    incoming_instructions: &mut StateInstructions,
) {
    let should_last_used_move = state.use_last_used_move;
    state.apply_instructions(&incoming_instructions.instruction_list);

    let side = state.get_side(&switching_side_ref);
    let slot = side.get_slot(&slot_ref);
    if slot.force_switch {
        slot.force_switch = false;
        incoming_instructions
            .instruction_list
            .push(Instruction::ToggleForceSwitch(
                ToggleForceSwitchInstruction {
                    side_ref: switching_side_ref,
                    slot_ref: *slot_ref,
                },
            ));
    }

    let mut baton_passing = false;
    if slot.baton_passing {
        baton_passing = true;
        slot.baton_passing = false;
        match switching_side_ref {
            SideReference::SideOne => {
                incoming_instructions
                    .instruction_list
                    .push(Instruction::ToggleBatonPassing(
                        ToggleBatonPassingInstruction {
                            side_ref: SideReference::SideOne,
                            slot_ref: *slot_ref,
                        },
                    ));
            }
            SideReference::SideTwo => {
                incoming_instructions
                    .instruction_list
                    .push(Instruction::ToggleBatonPassing(
                        ToggleBatonPassingInstruction {
                            side_ref: SideReference::SideTwo,
                            slot_ref: *slot_ref,
                        },
                    ));
            }
        }
    }

    let mut shed_tailing = false;
    if slot.shed_tailing {
        shed_tailing = true;
        slot.shed_tailing = false;
        match switching_side_ref {
            SideReference::SideOne => {
                incoming_instructions
                    .instruction_list
                    .push(Instruction::ToggleShedTailing(
                        ToggleShedTailingInstruction {
                            side_ref: SideReference::SideOne,
                            slot_ref: *slot_ref,
                        },
                    ));
            }
            SideReference::SideTwo => {
                incoming_instructions
                    .instruction_list
                    .push(Instruction::ToggleShedTailing(
                        ToggleShedTailingInstruction {
                            side_ref: SideReference::SideTwo,
                            slot_ref: *slot_ref,
                        },
                    ));
            }
        }
    }

    // TODO: How can this be implemented? You need to know the trapper? eww
    // if opposite_side
    //     .volatile_statuses
    //     .contains(&PokemonVolatileStatus::PARTIALLYTRAPPED)
    // {
    //     incoming_instructions
    //         .instruction_list
    //         .push(Instruction::RemoveVolatileStatus(
    //             RemoveVolatileStatusInstruction {
    //                 side_ref: switching_side_ref.get_other_side(),
    //                 volatile_status: PokemonVolatileStatus::PARTIALLYTRAPPED,
    //             },
    //         ));
    //     opposite_side
    //         .volatile_statuses
    //         .remove(&PokemonVolatileStatus::PARTIALLYTRAPPED);
    // }

    state.re_enable_disabled_moves(
        &switching_side_ref,
        slot_ref,
        &mut incoming_instructions.instruction_list,
    );
    state.remove_volatile_statuses_on_switch(
        &switching_side_ref,
        slot_ref,
        &mut incoming_instructions.instruction_list,
        baton_passing,
        shed_tailing,
    );
    state.reset_toxic_count(
        &switching_side_ref,
        &mut incoming_instructions.instruction_list,
    );
    if !baton_passing {
        state.reset_boosts(
            &switching_side_ref,
            slot_ref,
            &mut incoming_instructions.instruction_list,
        );
    }

    ability_on_switch_out(state, &switching_side_ref, slot_ref, incoming_instructions);

    let switch_instruction = Instruction::Switch(SwitchInstruction {
        side_ref: switching_side_ref,
        slot_ref: *slot_ref,
        previous_index: state
            .get_side(&switching_side_ref)
            .get_slot(slot_ref)
            .active_index,
        next_index: new_pokemon_index,
    });

    let side = state.get_side(&switching_side_ref);
    let slot = side.get_slot(&slot_ref);
    slot.active_index = new_pokemon_index;
    incoming_instructions
        .instruction_list
        .push(switch_instruction);

    if should_last_used_move {
        set_last_used_move_as_switch(
            slot,
            slot_ref,
            new_pokemon_index,
            switching_side_ref,
            incoming_instructions,
        );
    }

    if side.side_conditions.healing_wish > 0 {
        let mut healing_wish_consumed = false;
        let pkmn_index = side.get_slot_immutable(slot_ref).active_index;
        let switched_in_pkmn = side.get_active(slot_ref);
        if switched_in_pkmn.hp < switched_in_pkmn.maxhp {
            let heal_amount = switched_in_pkmn.maxhp - switched_in_pkmn.hp;
            let heal_instruction = Instruction::Heal(HealInstruction {
                side_ref: switching_side_ref,
                pokemon_index: pkmn_index,
                heal_amount,
            });
            incoming_instructions
                .instruction_list
                .push(heal_instruction);
            switched_in_pkmn.hp += heal_amount;
            healing_wish_consumed = true;
        }
        if switched_in_pkmn.status != PokemonStatus::NONE {
            add_remove_status_instructions(
                incoming_instructions,
                new_pokemon_index,
                switching_side_ref,
                side,
            );
            healing_wish_consumed = true;
        }

        if healing_wish_consumed {
            incoming_instructions
                .instruction_list
                .push(Instruction::ChangeSideCondition(
                    ChangeSideConditionInstruction {
                        side_ref: switching_side_ref,
                        side_condition: PokemonSideCondition::HealingWish,
                        amount: -1 * side.side_conditions.healing_wish,
                    },
                ));
            side.side_conditions.healing_wish = 0;
        }
    }

    let active = side.get_active_immutable(slot_ref);
    if active.item != Items::HEAVYDUTYBOOTS {
        let switched_in_pkmn = side.get_active_immutable(slot_ref);
        if side.side_conditions.sticky_web == 1 && switched_in_pkmn.is_grounded() {
            // a pkmn switching in doesn't have any other speed drops,
            // so no need to check for going below -6
            if let Some(sticky_web_instruction) = get_boost_instruction(
                &side,
                &PokemonBoostableStat::Speed,
                &-1,
                &switching_side_ref,
                &switching_side_ref,
                slot_ref,
            ) {
                state.apply_one_instruction(&sticky_web_instruction);
                incoming_instructions
                    .instruction_list
                    .push(sticky_web_instruction);
            }
        }

        let side = state.get_side_immutable(&switching_side_ref);
        let switched_in_pkmn = side.get_active_immutable(slot_ref);
        let mut toxic_spike_instruction: Option<Instruction> = None;
        if side.side_conditions.toxic_spikes > 0 && switched_in_pkmn.is_grounded() {
            if !immune_to_status(
                &state,
                &MoveTarget::User,
                &switching_side_ref.get_other_side(),
                &switching_side_ref,
                slot_ref,
                &PokemonStatus::POISON,
            ) {
                if side.side_conditions.toxic_spikes == 1 {
                    toxic_spike_instruction =
                        Some(Instruction::ChangeStatus(ChangeStatusInstruction {
                            side_ref: switching_side_ref,
                            pokemon_index: side.get_slot_immutable(slot_ref).active_index,
                            old_status: switched_in_pkmn.status,
                            new_status: PokemonStatus::POISON,
                        }))
                } else if side.side_conditions.toxic_spikes == 2 {
                    toxic_spike_instruction =
                        Some(Instruction::ChangeStatus(ChangeStatusInstruction {
                            side_ref: switching_side_ref,
                            pokemon_index: side.get_slot_immutable(slot_ref).active_index,
                            old_status: switched_in_pkmn.status,
                            new_status: PokemonStatus::TOXIC,
                        }))
                }
            } else if switched_in_pkmn.has_type(&PokemonType::POISON) {
                toxic_spike_instruction = Some(Instruction::ChangeSideCondition(
                    ChangeSideConditionInstruction {
                        side_ref: switching_side_ref,
                        side_condition: PokemonSideCondition::ToxicSpikes,
                        amount: -1 * side.side_conditions.toxic_spikes,
                    },
                ))
            }

            if let Some(i) = toxic_spike_instruction {
                state.apply_one_instruction(&i);
                incoming_instructions.instruction_list.push(i);
            }
        }

        let side = state.get_side(&switching_side_ref);
        let active_index = side.get_slot_immutable(slot_ref).active_index;
        let active = side.get_active_immutable(slot_ref);
        if active.ability != Abilities::MAGICGUARD {
            if side.side_conditions.stealth_rock == 1 {
                let switched_in_pkmn = side.get_active(slot_ref);
                let multiplier = type_effectiveness_modifier(&PokemonType::ROCK, &switched_in_pkmn);

                let dmg_amount = cmp::min(
                    (switched_in_pkmn.maxhp as f32 * multiplier / 8.0) as i16,
                    switched_in_pkmn.hp,
                );
                let stealth_rock_dmg_instruction = Instruction::Damage(DamageInstruction {
                    side_ref: switching_side_ref,
                    pokemon_index: active_index,
                    damage_amount: dmg_amount,
                });
                switched_in_pkmn.hp -= dmg_amount;
                incoming_instructions
                    .instruction_list
                    .push(stealth_rock_dmg_instruction);
            }

            let switched_in_pkmn = side.get_active_immutable(slot_ref);
            if side.side_conditions.spikes > 0 && switched_in_pkmn.is_grounded() {
                let dmg_amount = cmp::min(
                    switched_in_pkmn.maxhp * side.side_conditions.spikes as i16 / 8,
                    switched_in_pkmn.hp,
                );
                let spikes_dmg_instruction = Instruction::Damage(DamageInstruction {
                    side_ref: switching_side_ref,
                    pokemon_index: active_index,
                    damage_amount: dmg_amount,
                });
                side.get_active(slot_ref).hp -= dmg_amount;
                incoming_instructions
                    .instruction_list
                    .push(spikes_dmg_instruction);
            }
        }
    }

    ability_on_switch_in(state, &switching_side_ref, &slot_ref, incoming_instructions);
    item_on_switch_in(state, &switching_side_ref, &slot_ref, incoming_instructions);

    // switching in as dondozo when ally has commander
    let neutralizing_gas_active = state.neutralizing_gas_is_active();
    let side = state.get_side_immutable(&switching_side_ref);
    let active = side.get_active_immutable(slot_ref);
    if active.id == PokemonName::DONDOZO {
        let active_ally = side.get_active_immutable(&slot_ref.get_other_slot());
        if active_ally.ability == Abilities::COMMANDER
            && (!neutralizing_gas_active || active_ally.item == Items::ABILITYSHIELD)
        {
            commander_activating(
                state,
                &switching_side_ref,
                &slot_ref.get_other_slot(),
                incoming_instructions,
            )
        }
    }

    state.reverse_instructions(&incoming_instructions.instruction_list);
}

fn generate_instructions_from_increment_side_condition(
    state: &mut State,
    side_condition: &SideCondition,
    attacking_side_reference: &SideReference,
    incoming_instructions: &mut StateInstructions,
) {
    let affected_side_ref;
    match side_condition.target {
        MoveTarget::Target => affected_side_ref = attacking_side_reference.get_other_side(),
        MoveTarget::User => affected_side_ref = *attacking_side_reference,
    }

    let max_layers = match side_condition.condition {
        PokemonSideCondition::Spikes => 3,
        PokemonSideCondition::ToxicSpikes => 2,
        _ => 1,
    };

    let affected_side = state.get_side(&affected_side_ref);
    if affected_side.get_side_condition(side_condition.condition) < max_layers {
        let ins = Instruction::ChangeSideCondition(ChangeSideConditionInstruction {
            side_ref: affected_side_ref,
            side_condition: side_condition.condition,
            amount: 1,
        });
        affected_side.update_side_condition(side_condition.condition, 1);
        incoming_instructions.instruction_list.push(ins);
    }
}

fn generate_instructions_from_duration_side_conditions(
    state: &mut State,
    side_condition: &SideCondition,
    attacking_side_reference: &SideReference,
    incoming_instructions: &mut StateInstructions,
    duration: i8,
) {
    let affected_side_ref = match side_condition.target {
        MoveTarget::Target => attacking_side_reference.get_other_side(),
        MoveTarget::User => *attacking_side_reference,
    };
    if side_condition.condition == PokemonSideCondition::AuroraVeil
        && !state.weather_is_active(&Weather::HAIL)
        && !state.weather_is_active(&Weather::SNOW)
    {
        return;
    }
    let affected_side = state.get_side(&affected_side_ref);
    if affected_side.get_side_condition(side_condition.condition) == 0 {
        let ins = Instruction::ChangeSideCondition(ChangeSideConditionInstruction {
            side_ref: affected_side_ref,
            side_condition: side_condition.condition,
            amount: duration,
        });
        affected_side.update_side_condition(side_condition.condition, duration);
        incoming_instructions.instruction_list.push(ins);
    }
}

fn generate_instructions_from_side_conditions(
    state: &mut State,
    side_condition: &SideCondition,
    attacking_side_reference: &SideReference,
    incoming_instructions: &mut StateInstructions,
) {
    match side_condition.condition {
        PokemonSideCondition::AuroraVeil
        | PokemonSideCondition::LightScreen
        | PokemonSideCondition::Reflect
        | PokemonSideCondition::Safeguard
        | PokemonSideCondition::Mist => {
            generate_instructions_from_duration_side_conditions(
                state,
                side_condition,
                attacking_side_reference,
                incoming_instructions,
                SIDE_CONDITION_DURATION,
            );
        }
        PokemonSideCondition::Tailwind => {
            generate_instructions_from_duration_side_conditions(
                state,
                side_condition,
                attacking_side_reference,
                incoming_instructions,
                TAILWIND_DURATION,
            );
        }
        _ => generate_instructions_from_increment_side_condition(
            state,
            side_condition,
            attacking_side_reference,
            incoming_instructions,
        ),
    }
}

fn get_instructions_from_volatile_statuses(
    state: &mut State,
    attacker_choice: &Choice,
    volatile_status: &VolatileStatus,
    attacking_side_reference: &SideReference,
    attacking_slot_reference: &SlotReference,
    target_side_reference: &SideReference,
    target_slot: &SlotReference,
    incoming_instructions: &mut StateInstructions,
) {
    let target_side_ref: SideReference;
    let target_slot_ref: SlotReference;
    match volatile_status.target {
        MoveTarget::Target => {
            target_side_ref = *target_side_reference;
            target_slot_ref = *target_slot;
        }
        MoveTarget::User => {
            target_side_ref = *attacking_side_reference;
            target_slot_ref = *attacking_slot_reference;
        }
    }

    if volatile_status.volatile_status == PokemonVolatileStatus::YAWN
        && immune_to_status(
            state,
            &MoveTarget::Target,
            &attacking_side_reference,
            &target_side_ref,
            &target_slot_ref,
            &PokemonStatus::SLEEP,
        )
    {
        return;
    }
    let side = state.get_side(&target_side_ref);
    if side
        .get_active_immutable(&target_slot_ref)
        .volatile_status_can_be_applied(
            &volatile_status.volatile_status,
            &side.get_slot_immutable(&target_slot_ref).volatile_statuses,
            attacker_choice.first_move,
        )
    {
        let slot = side.get_slot(&target_slot_ref);
        let ins = Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
            side_ref: target_side_ref,
            slot_ref: target_slot_ref,
            volatile_status: volatile_status.volatile_status,
        });

        slot.volatile_statuses
            .insert(volatile_status.volatile_status);
        incoming_instructions.instruction_list.push(ins);
    }
}

pub fn add_remove_status_instructions(
    incoming_instructions: &mut StateInstructions,
    pokemon_index: PokemonIndex,
    side_reference: SideReference,
    side: &mut Side,
) {
    /*
    Single place to check for status removals, add the necessary instructions, and update the pokemon's status

    This is necessary because of some side effects to removing statuses
    i.e. a pre-mature wake-up from rest must set rest_turns to 0
    */
    let pkmn = &mut side.pokemon[pokemon_index];
    incoming_instructions
        .instruction_list
        .push(Instruction::ChangeStatus(ChangeStatusInstruction {
            side_ref: side_reference,
            pokemon_index: pokemon_index,
            old_status: pkmn.status,
            new_status: PokemonStatus::NONE,
        }));
    match pkmn.status {
        PokemonStatus::SLEEP => {
            if pkmn.rest_turns > 0 {
                incoming_instructions
                    .instruction_list
                    .push(Instruction::SetRestTurns(SetSleepTurnsInstruction {
                        side_ref: side_reference,
                        pokemon_index,
                        new_turns: 0,
                        previous_turns: pkmn.rest_turns,
                    }));
                pkmn.rest_turns = 0;
            } else if pkmn.sleep_turns > 0 {
                incoming_instructions
                    .instruction_list
                    .push(Instruction::SetSleepTurns(SetSleepTurnsInstruction {
                        side_ref: side_reference,
                        pokemon_index,
                        new_turns: 0,
                        previous_turns: pkmn.sleep_turns,
                    }));
                pkmn.sleep_turns = 0;
            }
        }
        PokemonStatus::TOXIC => {
            if side.side_conditions.toxic_count != 0 {
                incoming_instructions
                    .instruction_list
                    .push(Instruction::ChangeSideCondition(
                        ChangeSideConditionInstruction {
                            side_ref: side_reference,
                            side_condition: PokemonSideCondition::ToxicCount,
                            amount: -1 * side.side_conditions.toxic_count,
                        },
                    ));
                side.side_conditions.toxic_count = 0;
            }
        }
        _ => {}
    }
    pkmn.status = PokemonStatus::NONE;
}

pub fn immune_to_status(
    state: &State,
    status_target: &MoveTarget,
    attacking_side_ref: &SideReference,
    target_side_ref: &SideReference,
    slot_ref: &SlotReference,
    status: &PokemonStatus,
) -> bool {
    let attacking_side = state.get_side_immutable(attacking_side_ref);
    let target_side = state.get_side_immutable(target_side_ref);
    let target_slot = target_side.get_slot_immutable(slot_ref);
    let target_pkmn = target_side.get_active_immutable(slot_ref);
    let attacking_pkmn = attacking_side.get_active_immutable(slot_ref);

    // General Status Immunity
    match target_pkmn.ability {
        Abilities::SHIELDSDOWN => return target_pkmn.hp > target_pkmn.maxhp / 2,
        Abilities::PURIFYINGSALT => return true,
        Abilities::COMATOSE => return true,
        Abilities::LEAFGUARD => return state.weather_is_active(&Weather::SUN),
        _ => {}
    }

    if target_pkmn.status != PokemonStatus::NONE || target_pkmn.hp <= 0 {
        true
    } else if state.terrain.terrain_type == Terrain::MISTYTERRAIN && target_pkmn.is_grounded() {
        true
    } else if (target_slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::SUBSTITUTE)
        || target_side.side_conditions.safeguard > 0)
        && status_target == &MoveTarget::Target
    // substitute/safeguard don't block if the target is yourself (eg. rest)
    {
        true
    } else {
        // Specific status immunity
        match status {
            PokemonStatus::BURN => {
                target_pkmn.has_type(&PokemonType::FIRE)
                    || [
                        Abilities::WATERVEIL,
                        Abilities::WATERBUBBLE,
                        Abilities::THERMALEXCHANGE,
                    ]
                    .contains(&target_pkmn.ability)
            }
            PokemonStatus::FREEZE => {
                target_pkmn.has_type(&PokemonType::ICE)
                    || target_pkmn.ability == Abilities::MAGMAARMOR
                    || state.weather_is_active(&Weather::SUN)
                    || state.weather_is_active(&Weather::HARSHSUN)
            }
            PokemonStatus::SLEEP => {
                (state.terrain.terrain_type == Terrain::ELECTRICTERRAIN
                    && target_pkmn.is_grounded())
                    || [
                        Abilities::INSOMNIA,
                        Abilities::SWEETVEIL,
                        Abilities::VITALSPIRIT,
                    ]
                    .contains(&target_pkmn.ability)
            }

            PokemonStatus::PARALYZE => {
                target_pkmn.has_type(&PokemonType::ELECTRIC)
                    || target_pkmn.ability == Abilities::LIMBER
            }

            PokemonStatus::POISON | PokemonStatus::TOXIC => {
                ((target_pkmn.has_type(&PokemonType::POISON)
                    || target_pkmn.has_type(&PokemonType::STEEL))
                    && attacking_pkmn.ability != Abilities::CORROSION)
                    || [Abilities::IMMUNITY, Abilities::PASTELVEIL].contains(&target_pkmn.ability)
            }
            _ => false,
        }
    }
}

fn get_instructions_from_status_effects(
    state: &mut State,
    status: &Status,
    attacking_side_reference: &SideReference,
    attacking_slot_reference: &SlotReference,
    target_side_reference: &SideReference,
    target_slot: &SlotReference,
    incoming_instructions: &mut StateInstructions,
    hit_sub: bool,
) {
    let target_side_ref: SideReference;
    let target_slot_ref: SlotReference;
    match status.target {
        MoveTarget::Target => {
            target_side_ref = *target_side_reference;
            target_slot_ref = *target_slot;
        }
        MoveTarget::User => {
            target_side_ref = *attacking_side_reference;
            target_slot_ref = *attacking_slot_reference;
        }
    }

    if hit_sub
        || immune_to_status(
            state,
            &status.target,
            attacking_side_reference,
            &target_side_ref,
            &target_slot_ref,
            &status.status,
        )
    {
        return;
    }

    let target_side = state.get_side(&target_side_ref);
    let target_slot = target_side.get_slot(&target_slot_ref);
    let target_side_active = target_slot.active_index;
    let target_pkmn = target_side.get_active(&target_slot_ref);

    let status_hit_instruction = Instruction::ChangeStatus(ChangeStatusInstruction {
        side_ref: target_side_ref,
        pokemon_index: target_side_active,
        old_status: target_pkmn.status,
        new_status: status.status,
    });
    target_pkmn.status = status.status;
    incoming_instructions
        .instruction_list
        .push(status_hit_instruction);
}

pub fn get_boost_amount(
    side: &Side,
    slot_ref: &SlotReference,
    boost: &PokemonBoostableStat,
    amount: i8,
) -> i8 {
    /*
    returns that amount that can actually be applied from the attempted boost amount
        e.g. using swordsdance at +5 attack would result in a +1 boost instead of +2
    */
    let current_boost = side.get_boost_from_boost_enum(slot_ref, boost);

    if amount > 0 {
        return cmp::min(6 - current_boost, amount);
    } else if amount < 0 {
        return cmp::max(-6 - current_boost, amount);
    }
    0
}

pub fn get_boost_instruction(
    target_side: &Side,
    stat: &PokemonBoostableStat,
    boost: &i8,
    attacking_side_ref: &SideReference,
    target_side_ref: &SideReference,
    target_slot_ref: &SlotReference,
) -> Option<Instruction> {
    /*
    Single point for checking whether a boost can be applied to a pokemon
    Returns that boost instruction, if applicable
    */
    let target_slot = target_side.get_slot_immutable(target_slot_ref);
    let target_pkmn = target_side.get_active_immutable(target_slot_ref);

    if boost != &0
        && !(target_side_ref != attacking_side_ref
            && target_pkmn
                .immune_to_stats_lowered_by_opponent(&stat, &target_slot.volatile_statuses))
        && target_pkmn.hp != 0
    {
        let mut boost_amount = *boost;
        if target_pkmn.ability == Abilities::CONTRARY {
            boost_amount *= -1;
        }
        boost_amount = get_boost_amount(target_side, target_slot_ref, &stat, boost_amount);
        if boost_amount != 0 {
            return Some(Instruction::Boost(BoostInstruction {
                side_ref: *target_side_ref,
                slot_ref: *target_slot_ref,
                stat: *stat,
                amount: boost_amount,
            }));
        }
    }
    None
}

fn get_instructions_from_boosts(
    state: &mut State,
    boosts: &Boost,
    attacking_side_reference: &SideReference,
    attacking_slot_reference: &SlotReference,
    target_side_reference: &SideReference,
    target_slot: &SlotReference,
    incoming_instructions: &mut StateInstructions,
) {
    let target_side_ref: SideReference;
    let target_slot_ref: SlotReference;
    match boosts.target {
        MoveTarget::Target => {
            target_side_ref = *target_side_reference;
            target_slot_ref = *target_slot;
        }
        MoveTarget::User => {
            target_side_ref = *attacking_side_reference;
            target_slot_ref = *attacking_slot_reference;
        }
    }
    let boostable_stats = boosts.boosts.get_as_pokemon_boostable();
    for (pkmn_boostable_stat, boost) in boostable_stats.iter().filter(|(_, b)| b != &0) {
        let side = state.get_side_immutable(&target_side_ref);
        if let Some(boost_instruction) = get_boost_instruction(
            &side,
            pkmn_boostable_stat,
            boost,
            attacking_side_reference,
            &target_side_ref,
            &target_slot_ref,
        ) {
            state.apply_one_instruction(&boost_instruction);
            incoming_instructions
                .instruction_list
                .push(boost_instruction);
        }
    }
}

fn compare_health_with_damage_multiples(max_damage: i16, health: i16) -> (i16, i16) {
    let max_damage_f32 = max_damage as f32;
    let health_f32 = health as f32;

    let mut total_less_than = 0;
    let mut num_less_than = 0;
    let mut num_greater_than = 0;
    let increment = max_damage as f32 * 0.01;
    let mut damage = max_damage_f32 * 0.85;
    for _ in 0..16 {
        if damage < health_f32 {
            total_less_than += damage as i16;
            num_less_than += 1;
        } else if damage > health_f32 {
            num_greater_than += 1;
        }
        damage += increment;
    }

    (total_less_than / num_less_than, num_greater_than)
}

fn get_instructions_from_secondaries(
    state: &mut State,
    attacker_choice: &Choice,
    secondaries: &Vec<Secondary>,
    side_reference: &SideReference,
    slot_reference: &SlotReference,
    target_side_ref: &SideReference,
    target_slot_ref: &SlotReference,
    incoming_instructions: StateInstructions,
    hit_sub: bool,
) -> Vec<StateInstructions> {
    let mut return_instruction_list = Vec::with_capacity(4);
    return_instruction_list.push(incoming_instructions);

    for secondary in secondaries {
        if secondary.target == MoveTarget::Target && hit_sub {
            continue;
        }
        let secondary_percent_hit = (secondary.chance / 100.0).min(1.0);

        let mut i = 0;
        while i < return_instruction_list.len() {
            let mut secondary_hit_instructions = return_instruction_list.remove(i);

            if secondary_percent_hit < 1.0 {
                let mut secondary_miss_instructions = secondary_hit_instructions.clone();
                secondary_miss_instructions.update_percentage(1.0 - secondary_percent_hit);
                return_instruction_list.insert(i, secondary_miss_instructions);
                i += 1;
            }

            if secondary_percent_hit > 0.0 {
                secondary_hit_instructions.update_percentage(secondary_percent_hit);

                state.apply_instructions(&secondary_hit_instructions.instruction_list);
                match &secondary.effect {
                    Effect::VolatileStatus(volatile_status) => {
                        get_instructions_from_volatile_statuses(
                            state,
                            attacker_choice,
                            &VolatileStatus {
                                target: secondary.target.clone(),
                                volatile_status: volatile_status.clone(),
                            },
                            side_reference,
                            slot_reference,
                            target_side_ref,
                            target_slot_ref,
                            &mut secondary_hit_instructions,
                        );
                    }
                    Effect::Boost(boost) => {
                        get_instructions_from_boosts(
                            state,
                            &Boost {
                                target: secondary.target.clone(),
                                boosts: boost.clone(),
                            },
                            side_reference,
                            slot_reference,
                            target_side_ref,
                            target_slot_ref,
                            &mut secondary_hit_instructions,
                        );
                    }
                    Effect::Status(status) => {
                        get_instructions_from_status_effects(
                            state,
                            &Status {
                                target: secondary.target.clone(),
                                status: status.clone(),
                            },
                            side_reference,
                            slot_reference,
                            target_side_ref,
                            target_slot_ref,
                            &mut secondary_hit_instructions,
                            hit_sub,
                        );
                    }
                    Effect::Heal(heal_amount) => {
                        get_instructions_from_heal(
                            state,
                            &Heal {
                                target: secondary.target.clone(),
                                amount: *heal_amount,
                            },
                            side_reference,
                            slot_reference,
                            target_side_ref,
                            target_slot_ref,
                            &mut secondary_hit_instructions,
                        );
                    }
                    Effect::RemoveItem => {
                        let secondary_target_side_ref: SideReference;
                        let secondary_target_slot_ref: SlotReference;
                        match secondary.target {
                            MoveTarget::Target => {
                                secondary_target_side_ref = *target_side_ref;
                                secondary_target_slot_ref = *target_slot_ref;
                            }
                            MoveTarget::User => {
                                secondary_target_side_ref = *side_reference;
                                secondary_target_slot_ref = *slot_reference;
                            }
                        }
                        let side = state.get_side(&secondary_target_side_ref);
                        let target_pokemon_index =
                            side.get_slot(&secondary_target_slot_ref).active_index;
                        let target_pkmn = state
                            .get_side(&secondary_target_side_ref)
                            .get_active(&secondary_target_slot_ref);
                        secondary_hit_instructions
                            .instruction_list
                            .push(Instruction::ChangeItem(ChangeItemInstruction {
                                side_ref: secondary_target_side_ref,
                                pokemon_index: target_pokemon_index,
                                current_item: target_pkmn.item.clone(),
                                new_item: Items::NONE,
                            }));
                        target_pkmn.item = Items::NONE;
                    }
                }
                state.reverse_instructions(&secondary_hit_instructions.instruction_list);
                return_instruction_list.insert(i, secondary_hit_instructions);
                i += 1; // Increment i only if we didn't remove an element
            }
        }
    }

    return_instruction_list
}

pub fn get_instructions_from_heal(
    state: &mut State,
    heal: &Heal,
    attacking_side_reference: &SideReference,
    attacking_slot_reference: &SlotReference,
    target_side_reference: &SideReference,
    target_slot: &SlotReference,
    incoming_instructions: &mut StateInstructions,
) {
    let target_side_ref: SideReference;
    let target_slot_ref: SlotReference;
    match heal.target {
        MoveTarget::Target => {
            target_side_ref = *target_side_reference;
            target_slot_ref = *target_slot;
        }
        MoveTarget::User => {
            target_side_ref = *attacking_side_reference;
            target_slot_ref = *attacking_slot_reference;
        }
    }

    let target_side = state.get_side(&target_side_ref);
    let target_pokemon_index = target_side.get_slot(&target_slot_ref).active_index;
    let target_pkmn = state
        .get_side(&target_side_ref)
        .get_active(&target_slot_ref);

    let mut health_recovered = (heal.amount * target_pkmn.maxhp as f32) as i16;
    let final_health = target_pkmn.hp + health_recovered;
    if final_health > target_pkmn.maxhp {
        health_recovered -= final_health - target_pkmn.maxhp;
    } else if final_health < 0 {
        health_recovered -= final_health;
    }

    if health_recovered != 0 {
        let ins = Instruction::Heal(HealInstruction {
            side_ref: target_side_ref,
            pokemon_index: target_pokemon_index,
            heal_amount: health_recovered,
        });
        target_pkmn.hp += health_recovered;
        incoming_instructions.instruction_list.push(ins);
    }
}

fn check_move_hit_or_miss(
    state: &mut State,
    choice: &Choice,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
    damage: Option<(i16, i16)>,
    incoming_instructions: &mut StateInstructions,
    frozen_instructions: &mut Vec<(StateInstructions, Vec<RemainingToMove>)>,
    remaining_to_move: &Vec<RemainingToMove>,
) {
    /*
    Checks whether a move can miss

    If the move can miss - adds it to `frozen_instructions`, signifying that the rest of the
    half-turn will not run.

    Otherwise, update the incoming instructions' percent_hit to reflect the chance of the move hitting
    */
    let attacking_side = state.get_side_immutable(attacking_side_ref);
    let attacking_pokemon_index = attacking_side
        .get_slot_immutable(attacking_slot_ref)
        .active_index;
    let attacking_pokemon = attacking_side.get_active_immutable(attacking_slot_ref);

    let mut percent_hit = (choice.accuracy / 100.0).min(1.0);
    if Some((0, 0)) == damage {
        percent_hit = 0.0;
    }

    if percent_hit < 1.0 {
        let mut move_missed_instruction = incoming_instructions.clone();
        move_missed_instruction.update_percentage(1.0 - percent_hit);
        if let Some(crash_fraction) = choice.crash {
            let crash_amount = (attacking_pokemon.maxhp as f32 * crash_fraction) as i16;
            let crash_instruction = Instruction::Damage(DamageInstruction {
                side_ref: *attacking_side_ref,
                pokemon_index: attacking_pokemon_index,
                damage_amount: cmp::min(crash_amount, attacking_pokemon.hp),
            });

            move_missed_instruction
                .instruction_list
                .push(crash_instruction);
        }

        if Items::BLUNDERPOLICY == attacking_pokemon.item {
            if let Some(boost_instruction) = get_boost_instruction(
                &attacking_side,
                &PokemonBoostableStat::Speed,
                &2,
                attacking_side_ref,
                attacking_side_ref,
                attacking_slot_ref,
            ) {
                move_missed_instruction
                    .instruction_list
                    .push(Instruction::ChangeItem(ChangeItemInstruction {
                        side_ref: *attacking_side_ref,
                        pokemon_index: attacking_pokemon_index,
                        current_item: attacking_pokemon.item,
                        new_item: Items::NONE,
                    }));
                move_missed_instruction
                    .instruction_list
                    .push(boost_instruction);
            }
        }

        frozen_instructions.push((move_missed_instruction, remaining_to_move.clone()));
    }
    incoming_instructions.update_percentage(percent_hit);
}

fn get_instructions_from_pivot(
    state: &mut State,
    attacking_choice: &Choice,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
    incoming_instructions: &mut StateInstructions,
    remaining_to_move: &Vec<RemainingToMove>,
) {
    let attacking_side = state.get_side(attacking_side_ref);
    if attacking_side.num_alive_reserves() > 0 {
        let slot = attacking_side.get_slot(attacking_slot_ref);
        if attacking_choice.move_id == Choices::BATONPASS {
            slot.baton_passing = !slot.baton_passing;
            incoming_instructions
                .instruction_list
                .push(Instruction::ToggleBatonPassing(
                    ToggleBatonPassingInstruction {
                        side_ref: *attacking_side_ref,
                        slot_ref: *attacking_slot_ref,
                    },
                ));
        } else if attacking_choice.move_id == Choices::SHEDTAIL {
            slot.shed_tailing = !slot.shed_tailing;
            incoming_instructions
                .instruction_list
                .push(Instruction::ToggleShedTailing(
                    ToggleShedTailingInstruction {
                        side_ref: *attacking_side_ref,
                        slot_ref: *attacking_slot_ref,
                    },
                ));
        }
        slot.force_switch = !slot.force_switch;
        incoming_instructions
            .instruction_list
            .push(Instruction::ToggleForceSwitch(
                ToggleForceSwitchInstruction {
                    side_ref: *attacking_side_ref,
                    slot_ref: *attacking_slot_ref,
                },
            ));

        for rtm in remaining_to_move {
            if (&rtm.side_ref == attacking_side_ref && &rtm.slot_ref == attacking_slot_ref)
                || rtm.move_choice == MoveChoice::None
            {
                continue;
            }
            let slot = state.get_side(&rtm.side_ref).get_slot(&rtm.slot_ref);
            incoming_instructions
                .instruction_list
                .push(Instruction::SetSwitchOutMove(
                    SetSecondMoveSwitchOutMoveInstruction {
                        side_ref: rtm.side_ref,
                        slot_ref: rtm.slot_ref,
                        new_choice: rtm.move_choice,
                        previous_choice: slot.switch_out_move_second_saved_move,
                    },
                ));
            slot.switch_out_move_second_saved_move = rtm.move_choice;
        }
    }
}

fn get_instructions_from_drag(
    state: &mut State,
    target_side_reference: &SideReference,
    target_slot_reference: &SlotReference,
    incoming_instructions: StateInstructions,
    frozen_instructions: &mut Vec<(StateInstructions, Vec<RemainingToMove>)>,
    remaining_to_move: &Vec<RemainingToMove>,
) {
    let defending_side = state.get_side(target_side_reference);
    if defending_side
        .get_active_immutable(target_slot_reference)
        .hp
        == 0
    {
        state.reverse_instructions(&incoming_instructions.instruction_list);
        frozen_instructions.push((incoming_instructions, remaining_to_move.clone()));
        return;
    }

    let defending_side_alive_reserve_indices = defending_side.get_alive_pkmn_indices();

    state.reverse_instructions(&incoming_instructions.instruction_list);

    let num_alive_reserve = defending_side_alive_reserve_indices.len();
    if num_alive_reserve == 0 {
        frozen_instructions.push((incoming_instructions, remaining_to_move.clone()));
        return;
    }

    for pkmn_id in defending_side_alive_reserve_indices {
        let mut cloned_instructions = incoming_instructions.clone();
        generate_instructions_from_switch(
            state,
            target_slot_reference,
            pkmn_id,
            *target_side_reference,
            &mut cloned_instructions,
        );
        cloned_instructions.update_percentage(1.0 / num_alive_reserve as f32);
        frozen_instructions.push((cloned_instructions, remaining_to_move.clone()));
    }
}

fn reset_damage_dealt(
    slot: &SideSlot,
    side_reference: &SideReference,
    slot_reference: &SlotReference,
    incoming_instructions: &mut StateInstructions,
) {
    // This creates instructions but does not modify the side
    // because this function is called before the state applies the instructions
    if slot.damage_dealt.damage != 0 {
        incoming_instructions
            .instruction_list
            .push(Instruction::ChangeDamageDealtDamage(
                ChangeDamageDealtDamageInstruction {
                    side_ref: *side_reference,
                    slot_ref: *slot_reference,
                    damage_change: 0 - slot.damage_dealt.damage,
                },
            ));
    }
    if slot.damage_dealt.move_category != MoveCategory::Physical {
        incoming_instructions
            .instruction_list
            .push(Instruction::ChangeDamageDealtMoveCatagory(
                ChangeDamageDealtMoveCategoryInstruction {
                    side_ref: *side_reference,
                    slot_ref: *slot_reference,
                    move_category: MoveCategory::Physical,
                    previous_move_category: slot.damage_dealt.move_category,
                },
            ));
    }
    if slot.damage_dealt.hit_substitute {
        incoming_instructions
            .instruction_list
            .push(Instruction::ToggleDamageDealtHitSubstitute(
                ToggleDamageDealtHitSubstituteInstruction {
                    side_ref: *side_reference,
                    slot_ref: *slot_reference,
                },
            ));
    }
}

fn set_damage_dealt(
    attacking_slot: &mut SideSlot,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
    damage_dealt: i16,
    choice: &Choice,
    hit_substitute: bool,
    incoming_instructions: &mut StateInstructions,
) {
    if attacking_slot.damage_dealt.damage != damage_dealt {
        incoming_instructions
            .instruction_list
            .push(Instruction::ChangeDamageDealtDamage(
                ChangeDamageDealtDamageInstruction {
                    side_ref: *attacking_side_ref,
                    slot_ref: *attacking_slot_ref,
                    damage_change: damage_dealt - attacking_slot.damage_dealt.damage,
                },
            ));
        attacking_slot.damage_dealt.damage = damage_dealt;
    }

    if attacking_slot.damage_dealt.move_category != choice.category {
        incoming_instructions
            .instruction_list
            .push(Instruction::ChangeDamageDealtMoveCatagory(
                ChangeDamageDealtMoveCategoryInstruction {
                    side_ref: *attacking_side_ref,
                    slot_ref: *attacking_slot_ref,
                    move_category: choice.category,
                    previous_move_category: attacking_slot.damage_dealt.move_category,
                },
            ));
        attacking_slot.damage_dealt.move_category = choice.category;
    }

    if attacking_slot.damage_dealt.hit_substitute != hit_substitute {
        incoming_instructions
            .instruction_list
            .push(Instruction::ToggleDamageDealtHitSubstitute(
                ToggleDamageDealtHitSubstituteInstruction {
                    side_ref: *attacking_side_ref,
                    slot_ref: *attacking_slot_ref,
                },
            ));
        attacking_slot.damage_dealt.hit_substitute = hit_substitute;
    }
}

fn generate_instructions_from_damage(
    mut state: &mut State,
    choice: &mut Choice,
    calculated_damage: i16,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
    target_side_ref: &SideReference,
    target_slot_ref: &SlotReference,
    mut incoming_instructions: &mut StateInstructions,
) -> bool {
    /*
    TODO:
        - arbitrary other after_move as well from the old engine (triggers on hit OR miss)
            - dig/dive/bounce/fly volatilestatus
    */
    let mut hit_sub = false;
    let attacking_side = state.get_side(attacking_side_ref);

    if calculated_damage <= 0 {
        if let Some(crash_fraction) = choice.crash {
            let attacking_pkmn_index = attacking_side
                .get_slot_immutable(target_slot_ref)
                .active_index;
            let attacking_pokemon = attacking_side.get_active(attacking_slot_ref);
            let crash_amount = (attacking_pokemon.maxhp as f32 * crash_fraction) as i16;
            let damage_taken = cmp::min(crash_amount, attacking_pokemon.hp);
            let crash_instruction = Instruction::Damage(DamageInstruction {
                side_ref: *attacking_side_ref,
                pokemon_index: attacking_pkmn_index,
                damage_amount: damage_taken,
            });
            attacking_pokemon.hp -= damage_taken;
            incoming_instructions
                .instruction_list
                .push(crash_instruction);
        }
        return hit_sub;
    }

    let percent_hit = (choice.accuracy / 100.0).min(1.0);

    if percent_hit > 0.0 {
        let should_use_damage_dealt = state.use_damage_dealt;
        let attacker_has_infiltrator = state
            .get_side_immutable(attacking_side_ref)
            .get_active_immutable(attacking_slot_ref)
            .ability
            == Abilities::INFILTRATOR;
        let defending_slot = state.get_side(target_side_ref).get_slot(target_slot_ref);
        let defending_active_index = defending_slot.active_index;
        let defender_has_substitute = defending_slot
            .volatile_statuses
            .contains(&PokemonVolatileStatus::SUBSTITUTE);
        let mut damage_dealt;
        if defender_has_substitute && !choice.flags.sound && attacker_has_infiltrator {
            damage_dealt = cmp::min(calculated_damage, defending_slot.substitute_health);
            let substitute_damage_dealt = cmp::min(calculated_damage, damage_dealt);
            let substitute_instruction =
                Instruction::DamageSubstitute(DamageSubstituteInstruction {
                    side_ref: *target_side_ref,
                    slot_ref: *target_slot_ref,
                    damage_amount: substitute_damage_dealt,
                });
            defending_slot.substitute_health -= substitute_damage_dealt;
            incoming_instructions
                .instruction_list
                .push(substitute_instruction);

            if should_use_damage_dealt {
                set_damage_dealt(
                    state
                        .get_side(attacking_side_ref)
                        .get_slot(attacking_slot_ref),
                    attacking_side_ref,
                    attacking_slot_ref,
                    damage_dealt,
                    choice,
                    true,
                    &mut incoming_instructions,
                );
            }

            let defending_slot = state.get_side(target_side_ref).get_slot(target_slot_ref);
            if defending_slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::SUBSTITUTE)
                && defending_slot.substitute_health == 0
            {
                incoming_instructions
                    .instruction_list
                    .push(Instruction::RemoveVolatileStatus(
                        RemoveVolatileStatusInstruction {
                            side_ref: *target_side_ref,
                            slot_ref: *target_slot_ref,
                            volatile_status: PokemonVolatileStatus::SUBSTITUTE,
                        },
                    ));
                defending_slot
                    .volatile_statuses
                    .remove(&PokemonVolatileStatus::SUBSTITUTE);
            }

            hit_sub = true;
        } else {
            let defending_side = state.get_side(target_side_ref);
            let has_endure = defending_side
                .get_slot(target_slot_ref)
                .volatile_statuses
                .contains(&PokemonVolatileStatus::ENDURE);
            let defending_pokemon = defending_side.get_active(target_slot_ref);
            let mut knocked_out = false;
            damage_dealt = cmp::min(calculated_damage, defending_pokemon.hp);
            if damage_dealt != 0 {
                if has_endure
                    || ((defending_pokemon.ability == Abilities::STURDY
                        || defending_pokemon.item == Items::FOCUSSASH)
                        && defending_pokemon.maxhp == defending_pokemon.hp)
                {
                    damage_dealt -= 1;
                }

                if damage_dealt >= defending_pokemon.hp {
                    knocked_out = true;
                }

                let damage_instruction = Instruction::Damage(DamageInstruction {
                    side_ref: *target_side_ref,
                    pokemon_index: defending_active_index,
                    damage_amount: damage_dealt,
                });
                defending_pokemon.hp -= damage_dealt;
                incoming_instructions
                    .instruction_list
                    .push(damage_instruction);

                if knocked_out
                    && defending_side
                        .get_slot_immutable(target_slot_ref)
                        .volatile_statuses
                        .contains(&PokemonVolatileStatus::DESTINYBOND)
                {
                    let attacking_side = state.get_side(attacking_side_ref);
                    let attacking_pokemon = attacking_side.get_active(attacking_slot_ref);
                    let damage_instruction = Instruction::Damage(DamageInstruction {
                        side_ref: *attacking_side_ref,
                        pokemon_index: defending_active_index,
                        damage_amount: attacking_pokemon.hp,
                    });
                    attacking_pokemon.hp = 0;
                    incoming_instructions
                        .instruction_list
                        .push(damage_instruction);
                }

                if should_use_damage_dealt {
                    let attacking_side = state.get_side(attacking_side_ref);
                    set_damage_dealt(
                        attacking_side.get_slot(attacking_slot_ref),
                        attacking_side_ref,
                        attacking_slot_ref,
                        damage_dealt,
                        choice,
                        false,
                        &mut incoming_instructions,
                    );
                }

                ability_after_damage_hit(
                    &mut state,
                    choice,
                    attacking_side_ref,
                    attacking_slot_ref,
                    target_side_ref,
                    target_slot_ref,
                    damage_dealt,
                    &mut incoming_instructions,
                );
            }
        }

        let attacking_side = state.get_side(attacking_side_ref);
        let active_index = attacking_side
            .get_slot_immutable(attacking_slot_ref)
            .active_index;
        let attacking_pokemon = attacking_side.get_active(attacking_slot_ref);
        if let Some(drain_fraction) = choice.drain {
            let drain_amount = (damage_dealt as f32 * drain_fraction) as i16;
            let heal_amount =
                cmp::min(drain_amount, attacking_pokemon.maxhp - attacking_pokemon.hp);
            if heal_amount != 0 {
                let drain_instruction = Instruction::Heal(HealInstruction {
                    side_ref: *attacking_side_ref,
                    pokemon_index: active_index,
                    heal_amount,
                });
                attacking_pokemon.hp += heal_amount;
                incoming_instructions
                    .instruction_list
                    .push(drain_instruction);
            }
        }

        let attacking_side = state.get_side(attacking_side_ref);
        let active_index = attacking_side
            .get_slot_immutable(attacking_slot_ref)
            .active_index;
        let attacking_pokemon = attacking_side.get_active(attacking_slot_ref);
        if let Some(recoil_fraction) = choice.recoil {
            let recoil_amount = (damage_dealt as f32 * recoil_fraction) as i16;
            let damage_amount = cmp::min(recoil_amount, attacking_pokemon.hp);
            let recoil_instruction = Instruction::Damage(DamageInstruction {
                side_ref: *attacking_side_ref,
                pokemon_index: active_index,
                damage_amount,
            });
            attacking_pokemon.hp -= damage_amount;
            incoming_instructions
                .instruction_list
                .push(recoil_instruction);
        }
        choice_after_damage_hit(
            &mut state,
            &choice,
            attacking_side_ref,
            attacking_slot_ref,
            target_side_ref,
            target_slot_ref,
            &mut incoming_instructions,
            hit_sub,
        );
    }
    hit_sub
}

fn move_has_no_effect(
    state: &State,
    choice: &Choice,
    target_side_ref: &SideReference,
    target_slot_ref: &SlotReference,
) -> bool {
    let target_side = state.get_side_immutable(target_side_ref);
    let target = target_side.get_active_immutable(target_slot_ref);

    if target_side.side_conditions.wide_guard > 0
        && (choice.move_choice_target == MoveChoiceTarget::AllFoes
            || choice.move_choice_target == MoveChoiceTarget::AllOther)
    {
        return true;
    }

    if choice.flags.powder
        && choice.target == MoveTarget::Target
        && (target.has_type(&PokemonType::GRASS) || target.item == Items::SAFETYGOGGLES)
    {
        return true;
    }

    if choice.move_type == PokemonType::ELECTRIC
        && choice.target == MoveTarget::Target
        && target.has_type(&PokemonType::GROUND)
    {
        return true;
    } else if choice.move_id == Choices::ENCORE {
        return match state
            .get_side_immutable(target_side_ref)
            .get_slot_immutable(target_slot_ref)
            .last_used_move
        {
            LastUsedMove::None => true,
            LastUsedMove::Move(_) => false,
            LastUsedMove::Switch(_) => true,
        };
    } else if state.terrain_is_active(&Terrain::PSYCHICTERRAIN)
        && choice.target == MoveTarget::Target
        && target.is_grounded()
        && choice.priority > 0
    {
        return true;
    }

    if state
        .get_side_immutable(target_side_ref)
        .get_slot_immutable(target_slot_ref)
        .volatile_statuses
        .contains(&PokemonVolatileStatus::COMMANDING)
    {
        return true;
    }

    false
}

fn cannot_use_move(
    state: &State,
    choice: &Choice,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
    target_side_ref: &SideReference,
    target_slot_ref: &SlotReference,
) -> bool {
    // If the opponent has 0 hp, you can't use a non-status move
    if state
        .get_side_immutable(target_side_ref)
        .get_active_immutable(target_slot_ref)
        .hp
        == 0
        && choice.category != MoveCategory::Status
    {
        return true;
    }

    // If you were taunted, you can't use a Physical/Special move
    let attacking_slot = state
        .get_side_immutable(attacking_side_ref)
        .get_slot_immutable(attacking_slot_ref);

    if attacking_slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::DISABLE)
    {
        match attacking_slot.last_used_move {
            LastUsedMove::Move(mv_index) => {
                if mv_index == choice.move_index {
                    return true;
                }
            }
            _ => {}
        }
    }
    if attacking_slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::COMMANDING)
    {
        return true;
    }
    if attacking_slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::TAUNT)
        && matches!(choice.category, MoveCategory::Status)
    {
        return true;
    } else if attacking_slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::FLINCH)
    {
        return true;
    } else if choice.flags.heal
        && attacking_slot
            .volatile_statuses
            .contains(&PokemonVolatileStatus::HEALBLOCK)
    {
        return true;
    }
    false
}

#[cfg(feature = "terastallization")]
fn terastallized_base_power_floor(
    state: &mut State,
    choice: &mut Choice,
    attacking_side: &SideReference,
    attacking_slot: &SlotReference,
) {
    let attacker = state
        .get_side_immutable(attacking_side)
        .get_active_immutable(attacking_slot);

    if attacker.terastallized
        && choice.move_type == attacker.tera_type
        && choice.base_power < 60.0
        && choice.priority <= 0
        && choice.multi_hit() == MultiHitMove::None
        && choice.multi_accuracy() == MultiAccuracyMove::None
    {
        choice.base_power = 60.0;
    }
}

fn before_move(
    state: &mut State,
    choice: &mut Choice,
    target_choice: &Choice,
    attacking_side: &SideReference,
    attacking_slot: &SlotReference,
    target_side: &SideReference,
    target_slot: &SlotReference,
    final_run_move: bool,
    target_has_moved: bool,
    incoming_instructions: &mut StateInstructions,
) {
    #[cfg(feature = "terastallization")]
    terastallized_base_power_floor(state, choice, attacking_side, attacking_slot);

    ability_before_move(
        state,
        choice,
        attacking_side,
        attacking_slot,
        target_side,
        target_slot,
        incoming_instructions,
    );
    item_before_move(
        state,
        choice,
        attacking_side,
        attacking_slot,
        target_side,
        target_slot,
        incoming_instructions,
    );
    choice_before_move(
        state,
        choice,
        attacking_side,
        attacking_slot,
        target_side,
        incoming_instructions,
    );

    modify_choice(
        state,
        choice,
        target_choice,
        attacking_side,
        attacking_slot,
        target_side,
        target_slot,
        target_has_moved,
    );

    ability_modify_attack_being_used(
        state,
        choice,
        target_choice,
        attacking_side,
        attacking_slot,
        target_side,
        target_slot,
    );
    ability_modify_attack_against(
        state,
        choice,
        attacking_side,
        attacking_slot,
        target_side,
        target_slot,
    );

    item_modify_attack_being_used(
        state,
        choice,
        attacking_side,
        attacking_slot,
        target_side,
        target_slot,
        final_run_move,
    );
    item_modify_attack_against(state, choice, target_side, target_slot);

    /*
        TODO: this needs to be here because from_drag is called after the substitute volatilestatus
            has already been removed
    */
    let attacking_side = state.get_side_immutable(attacking_side);
    let attacking_slot = attacking_side.get_slot_immutable(attacking_slot);

    // Update Choice for `charge` moves
    if choice.flags.charge {
        let charge_volatile_status = charge_choice_to_volatile(&choice.move_id);
        if !attacking_slot
            .volatile_statuses
            .contains(&charge_volatile_status)
        {
            choice.remove_all_effects();
            choice.volatile_status = Some(VolatileStatus {
                target: MoveTarget::User,
                volatile_status: charge_volatile_status,
            });
        }
    }

    let defending_slot = state
        .get_side_immutable(target_side)
        .get_slot_immutable(target_slot);
    if defending_slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::SUBSTITUTE)
        && choice.category != MoveCategory::Status
    {
        choice.flags.drag = false;
    }

    // modify choice if defender has protect active
    if (defending_slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::PROTECT)
        || defending_slot
            .volatile_statuses
            .contains(&PokemonVolatileStatus::SPIKYSHIELD)
        || defending_slot
            .volatile_statuses
            .contains(&PokemonVolatileStatus::BANEFULBUNKER)
        || defending_slot
            .volatile_statuses
            .contains(&PokemonVolatileStatus::BURNINGBULWARK)
        || defending_slot
            .volatile_statuses
            .contains(&PokemonVolatileStatus::SILKTRAP))
        && choice.flags.protect
    {
        choice.remove_effects_for_protect();
        if choice.crash.is_some() {
            choice.accuracy = 0.0;
        }

        if defending_slot
            .volatile_statuses
            .contains(&PokemonVolatileStatus::SPIKYSHIELD)
            && choice.flags.contact
        {
            choice.heal = Some(Heal {
                target: MoveTarget::User,
                amount: -0.125,
            })
        } else if defending_slot
            .volatile_statuses
            .contains(&PokemonVolatileStatus::BANEFULBUNKER)
            && choice.flags.contact
        {
            choice.status = Some(Status {
                target: MoveTarget::User,
                status: PokemonStatus::POISON,
            })
        } else if defending_slot
            .volatile_statuses
            .contains(&PokemonVolatileStatus::BURNINGBULWARK)
            && choice.flags.contact
        {
            choice.status = Some(Status {
                target: MoveTarget::User,
                status: PokemonStatus::BURN,
            })
        } else if defending_slot
            .volatile_statuses
            .contains(&PokemonVolatileStatus::SILKTRAP)
            && choice.flags.contact
        {
            choice.boost = Some(Boost {
                target: MoveTarget::User,
                boosts: StatBoosts {
                    attack: 0,
                    defense: 0,
                    special_attack: 0,
                    special_defense: 0,
                    speed: -1,
                    accuracy: 0,
                },
            })
        }
    }
}

fn generate_instructions_from_existing_status_conditions(
    state: &mut State,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
    attacker_choice: &Choice,
    incoming_instructions: &mut StateInstructions,
    final_instructions: &mut Vec<(StateInstructions, Vec<RemainingToMove>)>,
    remaining_to_move: &Vec<RemainingToMove>,
) {
    let attacking_side = state.get_side(attacking_side_ref);
    let attacking_slot = attacking_side.get_slot(attacking_slot_ref);
    let current_active_index = attacking_slot.active_index;
    let attacker_active = attacking_side.get_active(attacking_slot_ref);
    match attacker_active.status {
        PokemonStatus::PARALYZE => {
            // Fully-Paralyzed Branch
            let mut fully_paralyzed_instruction = incoming_instructions.clone();
            fully_paralyzed_instruction.update_percentage(0.25);
            final_instructions.push((fully_paralyzed_instruction, remaining_to_move.clone()));

            // Non-Paralyzed Branch
            incoming_instructions.update_percentage(0.75);
        }
        PokemonStatus::FREEZE => {
            let mut still_frozen_instruction = incoming_instructions.clone();
            still_frozen_instruction.update_percentage(0.80);
            final_instructions.push((still_frozen_instruction, remaining_to_move.clone()));

            incoming_instructions.update_percentage(0.20);
            attacker_active.status = PokemonStatus::NONE;
            incoming_instructions
                .instruction_list
                .push(Instruction::ChangeStatus(ChangeStatusInstruction {
                    side_ref: attacking_side_ref.clone(),
                    pokemon_index: current_active_index,
                    old_status: PokemonStatus::FREEZE,
                    new_status: PokemonStatus::NONE,
                }));
        }
        PokemonStatus::SLEEP => {
            match attacker_active.rest_turns {
                // Pokemon is not asleep because of Rest.
                0 => {
                    let current_sleep_turns = attacker_active.sleep_turns;
                    let chance_to_wake = chance_to_wake_up(current_sleep_turns);
                    if chance_to_wake == 1.0 {
                        attacker_active.status = PokemonStatus::NONE;
                        attacker_active.sleep_turns = 0;
                        incoming_instructions
                            .instruction_list
                            .push(Instruction::ChangeStatus(ChangeStatusInstruction {
                                side_ref: *attacking_side_ref,
                                pokemon_index: current_active_index,
                                old_status: PokemonStatus::SLEEP,
                                new_status: PokemonStatus::NONE,
                            }));
                        incoming_instructions
                            .instruction_list
                            .push(Instruction::SetSleepTurns(SetSleepTurnsInstruction {
                                side_ref: *attacking_side_ref,
                                pokemon_index: current_active_index,
                                new_turns: 0,
                                previous_turns: current_sleep_turns,
                            }));
                    } else if chance_to_wake == 0.0 {
                        if attacker_choice.move_id == Choices::SLEEPTALK {
                            // if we are using sleeptalk we want to continue using this move
                            incoming_instructions.instruction_list.push(
                                Instruction::SetSleepTurns(SetSleepTurnsInstruction {
                                    side_ref: *attacking_side_ref,
                                    pokemon_index: current_active_index,
                                    new_turns: current_sleep_turns + 1,
                                    previous_turns: current_sleep_turns,
                                }),
                            );
                        } else {
                            let mut still_asleep_instruction = incoming_instructions.clone();
                            still_asleep_instruction.update_percentage(1.0);
                            still_asleep_instruction.instruction_list.push(
                                Instruction::SetSleepTurns(SetSleepTurnsInstruction {
                                    side_ref: *attacking_side_ref,
                                    pokemon_index: current_active_index,
                                    new_turns: current_sleep_turns + 1,
                                    previous_turns: current_sleep_turns,
                                }),
                            );
                            final_instructions
                                .push((still_asleep_instruction, remaining_to_move.clone()));
                            incoming_instructions.update_percentage(0.0);
                        }
                    } else {
                        // This code deals with the situation where there is a chance to wake up
                        // as well as a chance to stay asleep.
                        // This logic will branch the state and one branch will represent where
                        // nothing happens and the other will represent where something happens
                        // Normally "nothing happens" means you stay asleep and "something happens"
                        // means you wake up. If the move is sleeptalk these are reversed.
                        let do_nothing_percentage;
                        let mut do_nothing_instructions = incoming_instructions.clone();
                        if attacker_choice.move_id == Choices::SLEEPTALK {
                            do_nothing_percentage = chance_to_wake;
                            do_nothing_instructions.instruction_list.push(
                                Instruction::ChangeStatus(ChangeStatusInstruction {
                                    side_ref: *attacking_side_ref,
                                    pokemon_index: current_active_index,
                                    old_status: PokemonStatus::SLEEP,
                                    new_status: PokemonStatus::NONE,
                                }),
                            );
                            do_nothing_instructions.instruction_list.push(
                                Instruction::SetSleepTurns(SetSleepTurnsInstruction {
                                    side_ref: *attacking_side_ref,
                                    pokemon_index: current_active_index,
                                    new_turns: 0,
                                    previous_turns: current_sleep_turns,
                                }),
                            );
                            incoming_instructions.instruction_list.push(
                                Instruction::SetSleepTurns(SetSleepTurnsInstruction {
                                    side_ref: *attacking_side_ref,
                                    pokemon_index: current_active_index,
                                    new_turns: current_sleep_turns + 1,
                                    previous_turns: current_sleep_turns,
                                }),
                            );
                            attacker_active.sleep_turns += 1;
                        } else {
                            do_nothing_percentage = 1.0 - chance_to_wake;
                            do_nothing_instructions.instruction_list.push(
                                Instruction::SetSleepTurns(SetSleepTurnsInstruction {
                                    side_ref: *attacking_side_ref,
                                    pokemon_index: current_active_index,
                                    new_turns: current_sleep_turns + 1,
                                    previous_turns: current_sleep_turns,
                                }),
                            );
                            incoming_instructions
                                .instruction_list
                                .push(Instruction::ChangeStatus(ChangeStatusInstruction {
                                    side_ref: *attacking_side_ref,
                                    pokemon_index: current_active_index,
                                    old_status: PokemonStatus::SLEEP,
                                    new_status: PokemonStatus::NONE,
                                }));
                            incoming_instructions.instruction_list.push(
                                Instruction::SetSleepTurns(SetSleepTurnsInstruction {
                                    side_ref: *attacking_side_ref,
                                    pokemon_index: current_active_index,
                                    new_turns: 0,
                                    previous_turns: current_sleep_turns,
                                }),
                            );
                            attacker_active.status = PokemonStatus::NONE;
                            attacker_active.sleep_turns = 0;
                        }
                        do_nothing_instructions.update_percentage(do_nothing_percentage);
                        incoming_instructions.update_percentage(1.0 - do_nothing_percentage);
                        final_instructions
                            .push((do_nothing_instructions, remaining_to_move.clone()));
                    }
                }
                // Pokemon is asleep because of Rest, and will wake up this turn
                1 => {
                    attacker_active.status = PokemonStatus::NONE;
                    attacker_active.rest_turns -= 1;
                    incoming_instructions
                        .instruction_list
                        .push(Instruction::ChangeStatus(ChangeStatusInstruction {
                            side_ref: *attacking_side_ref,
                            pokemon_index: current_active_index,
                            old_status: PokemonStatus::SLEEP,
                            new_status: PokemonStatus::NONE,
                        }));
                    incoming_instructions
                        .instruction_list
                        .push(Instruction::DecrementRestTurns(
                            DecrementRestTurnsInstruction {
                                side_ref: *attacking_side_ref,
                                pokemon_index: current_active_index,
                            },
                        ));
                }
                // Pokemon is asleep because of Rest, and will stay asleep this turn
                2 | 3 => {
                    attacker_active.rest_turns -= 1;
                    incoming_instructions
                        .instruction_list
                        .push(Instruction::DecrementRestTurns(
                            DecrementRestTurnsInstruction {
                                side_ref: *attacking_side_ref,
                                pokemon_index: current_active_index,
                            },
                        ));
                }
                _ => panic!("Invalid rest_turns value: {}", attacker_active.rest_turns),
            }
        }
        _ => {}
    }

    let attacking_slot = attacking_side.get_slot(attacking_slot_ref);

    if attacking_slot.volatile_status_durations.protect > 0 {
        if let Some(vs) = &attacker_choice.volatile_status {
            if PROTECT_VOLATILES.contains(&vs.volatile_status) {
                let protect_success_chance = CONSECUTIVE_PROTECT_CHANCE
                    .powi(attacking_slot.volatile_status_durations.protect as i32);
                let mut protect_fail_instruction = incoming_instructions.clone();
                protect_fail_instruction.update_percentage(1.0 - protect_success_chance);
                final_instructions.push((protect_fail_instruction, remaining_to_move.clone()));
                incoming_instructions.update_percentage(protect_success_chance);
            }
        }
    }

    if attacking_slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::CONFUSION)
    {
        let mut hit_yourself_instruction = incoming_instructions.clone();
        hit_yourself_instruction.update_percentage(HIT_SELF_IN_CONFUSION_CHANCE);

        let attacking_stat =
            attacking_side.calculate_boosted_stat(attacking_slot_ref, PokemonBoostableStat::Attack);
        let defending_stat = attacking_side
            .calculate_boosted_stat(attacking_slot_ref, PokemonBoostableStat::Defense);

        let attacker_active = attacking_side.get_active(attacking_slot_ref);
        let mut damage_dealt = 2.0 * attacker_active.level as f32;
        damage_dealt = damage_dealt.floor() / 5.0;
        damage_dealt = damage_dealt.floor() + 2.0;
        damage_dealt = damage_dealt.floor() * 40.0; // 40 is the base power of confusion damage
        damage_dealt = damage_dealt * attacking_stat as f32 / defending_stat as f32;
        damage_dealt = damage_dealt.floor() / 50.0;
        damage_dealt = damage_dealt.floor() + 2.0;
        if attacker_active.status == PokemonStatus::BURN {
            damage_dealt /= 2.0;
        }

        let damage_dealt = cmp::min(damage_dealt as i16, attacker_active.hp);
        let damage_instruction = Instruction::Damage(DamageInstruction {
            side_ref: *attacking_side_ref,
            pokemon_index: current_active_index,
            damage_amount: damage_dealt,
        });
        hit_yourself_instruction
            .instruction_list
            .push(damage_instruction);

        final_instructions.push((hit_yourself_instruction, remaining_to_move.clone()));

        incoming_instructions.update_percentage(1.0 - HIT_SELF_IN_CONFUSION_CHANCE);
    }
}

pub fn generate_instructions_from_move(
    state: &mut State,
    choice: &mut Choice,
    defender_choice: &Choice,
    attacking_side: SideReference,
    attacking_slot: SlotReference,
    target_side: SideReference,
    target_slot: SlotReference,
    incoming_instructions: StateInstructions,
    final_instructions: &mut Vec<(StateInstructions, Vec<RemainingToMove>)>,
    remaining_to_move: Vec<RemainingToMove>,
    branch_on_damage: bool,
) {
    // Split up spread moves into individual targets, if necessary
    let choices: Vec<(Choice, SideReference, SlotReference)> = match choice.move_choice_target {
        // Spread Move
        MoveChoiceTarget::AllFoes => {
            // find all targets with more than 1 hp
            // if num targets is 2, reduce damage by 25%
            let mut choices = Vec::with_capacity(3);
            let target_side = state.get_side_immutable(&attacking_side.get_other_side());
            let slot_a_alive = target_side.get_active_immutable(&SlotReference::SlotA).hp > 0;
            let slot_b_alive = target_side.get_active_immutable(&SlotReference::SlotB).hp > 0;
            if slot_a_alive && slot_b_alive {
                choice.base_power *= 0.75;
                choices.push((
                    choice.clone(),
                    attacking_side.get_other_side(),
                    SlotReference::SlotA,
                ));
                choices.push((
                    choice.clone(),
                    attacking_side.get_other_side(),
                    SlotReference::SlotB,
                ));
            } else if slot_a_alive {
                choices.push((
                    choice.clone(),
                    attacking_side.get_other_side(),
                    SlotReference::SlotA,
                ));
            } else if slot_b_alive {
                choices.push((
                    choice.clone(),
                    attacking_side.get_other_side(),
                    SlotReference::SlotB,
                ));
            }

            choices
        }
        // Targets Everyone except the user
        MoveChoiceTarget::AllOther => {
            let mut choices = Vec::with_capacity(3);
            if state
                .get_side_immutable(&target_side)
                .get_active_immutable(&SlotReference::SlotA)
                .hp
                > 0
            {
                choices.push((choice.clone(), target_side, SlotReference::SlotA))
            }
            if state
                .get_side_immutable(&target_side)
                .get_active_immutable(&SlotReference::SlotB)
                .hp
                > 0
            {
                choices.push((choice.clone(), target_side, SlotReference::SlotB))
            }
            if state
                .get_side_immutable(&attacking_side)
                .get_active_immutable(&attacking_slot.get_other_slot())
                .hp
                > 0
            {
                choices.push((
                    choice.clone(),
                    attacking_side,
                    attacking_slot.get_other_slot(),
                ))
            }
            choices
        }
        // Single Target Move (target already chosen)
        MoveChoiceTarget::Normal => {
            vec![(choice.clone(), target_side, target_slot)]
        }
        _ => vec![(choice.clone(), target_side, target_slot)],
    };

    state.reverse_instructions(&incoming_instructions.instruction_list);
    let mut state_instructions_vec: Vec<(StateInstructions, Vec<RemainingToMove>)> =
        Vec::with_capacity(4);
    state_instructions_vec.push((incoming_instructions, remaining_to_move));

    let len = choices.len();
    for (i, (current_choice, current_target_side, current_target_slot)) in
        choices.into_iter().enumerate()
    {
        let mut next_state_instructions_vec = Vec::with_capacity(4);
        let final_run_move = i == len - 1; // true only on the final iteration

        for (state_instruction, remaining_moves) in state_instructions_vec {
            run_move(
                state,
                &mut current_choice.clone(),
                defender_choice,
                attacking_side,
                attacking_slot,
                current_target_side,
                current_target_slot,
                state_instruction,
                &mut next_state_instructions_vec,
                remaining_moves,
                final_run_move,
                branch_on_damage,
            );
        }

        state_instructions_vec = next_state_instructions_vec;
    }

    // Add all the final states to the final_instructions vector
    final_instructions.extend(state_instructions_vec);
}

fn run_move(
    state: &mut State,
    choice: &mut Choice,
    defender_choice: &Choice,
    attacking_side: SideReference,
    attacking_slot: SlotReference,
    mut target_side: SideReference,
    mut target_slot: SlotReference,
    mut incoming_instructions: StateInstructions,
    mut final_instructions: &mut Vec<(StateInstructions, Vec<RemainingToMove>)>,
    remaining_to_move: Vec<RemainingToMove>,
    final_run_move: bool,
    branch_on_damage: bool,
) {
    if state.use_damage_dealt {
        reset_damage_dealt(
            state
                .get_side_immutable(&attacking_side)
                .get_slot_immutable(&attacking_slot),
            &attacking_side,
            &attacking_slot,
            &mut incoming_instructions,
        );
    }

    if choice.category == MoveCategory::Switch {
        generate_instructions_from_switch(
            state,
            &attacking_slot,
            choice.switch_id,
            attacking_side,
            &mut incoming_instructions,
        );
        final_instructions.push((incoming_instructions, remaining_to_move));
        return;
    }

    let attacker_side = state.get_side(&attacking_side);
    let attacker_slot = attacker_side.get_slot(&attacking_slot);
    if choice.move_id == Choices::NONE {
        if attacker_slot
            .volatile_statuses
            .contains(&PokemonVolatileStatus::MUSTRECHARGE)
        {
            incoming_instructions
                .instruction_list
                .push(Instruction::RemoveVolatileStatus(
                    RemoveVolatileStatusInstruction {
                        side_ref: attacking_side,
                        slot_ref: attacking_slot,
                        volatile_status: PokemonVolatileStatus::MUSTRECHARGE,
                    },
                ));
        }
        final_instructions.push((incoming_instructions, remaining_to_move));
        return;
    }

    if attacker_slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::TRUANT)
    {
        incoming_instructions
            .instruction_list
            .push(Instruction::RemoveVolatileStatus(
                RemoveVolatileStatusInstruction {
                    side_ref: attacking_side,
                    slot_ref: attacking_slot,
                    volatile_status: PokemonVolatileStatus::TRUANT,
                },
            ));
        final_instructions.push((incoming_instructions, remaining_to_move));
        return;
    }

    // TODO: test first-turn dragontail missing - it should not trigger this early return
    if !choice.first_move && defender_choice.flags.drag {
        final_instructions.push((incoming_instructions, remaining_to_move));
        return;
    }

    state.apply_instructions(&incoming_instructions.instruction_list);

    let side = state.get_side(&attacking_side);
    let slot = side.get_slot(&attacking_slot);
    if slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::ENCORE)
    {
        match slot.last_used_move {
            LastUsedMove::Move(last_used_move) => {
                if choice.move_index != last_used_move {
                    *choice = MOVES
                        .get(&side.get_active_immutable(&attacking_slot).moves[&last_used_move].id)
                        .unwrap()
                        .clone();
                    choice.move_index = last_used_move;
                }
            }
            _ => panic!("Encore should not be active when last used move is not a move"),
        }

        let slot = side.get_slot(&attacking_slot);
        if slot.volatile_status_durations.encore == 2 {
            incoming_instructions
                .instruction_list
                .push(Instruction::RemoveVolatileStatus(
                    RemoveVolatileStatusInstruction {
                        side_ref: attacking_side,
                        slot_ref: attacking_slot,
                        volatile_status: PokemonVolatileStatus::ENCORE,
                    },
                ));
            incoming_instructions
                .instruction_list
                .push(Instruction::ChangeVolatileStatusDuration(
                    ChangeVolatileStatusDurationInstruction {
                        side_ref: attacking_side,
                        slot_ref: attacking_slot,
                        volatile_status: PokemonVolatileStatus::ENCORE,
                        amount: -2,
                    },
                ));
            slot.volatile_status_durations.encore = 0;
            slot.volatile_statuses
                .remove(&PokemonVolatileStatus::ENCORE);
        } else {
            incoming_instructions
                .instruction_list
                .push(Instruction::ChangeVolatileStatusDuration(
                    ChangeVolatileStatusDurationInstruction {
                        side_ref: attacking_side,
                        slot_ref: attacking_slot,
                        volatile_status: PokemonVolatileStatus::ENCORE,
                        amount: 1,
                    },
                ));
            slot.volatile_status_durations.encore += 1;
        }
    }

    let slot = side.get_slot(&attacking_slot);
    if slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::TAUNT)
    {
        match slot.volatile_status_durations.taunt {
            0 | 1 => {
                incoming_instructions.instruction_list.push(
                    Instruction::ChangeVolatileStatusDuration(
                        ChangeVolatileStatusDurationInstruction {
                            side_ref: attacking_side,
                            slot_ref: attacking_slot,
                            volatile_status: PokemonVolatileStatus::TAUNT,
                            amount: 1,
                        },
                    ),
                );
                slot.volatile_status_durations.taunt += 1;
            }

            // Technically taunt is removed at the end of the turn but because we are already
            // dealing with taunt here we can save a check at the end of the turn
            // This shouldn't change anything because taunt only affects which move is selected
            // and by this point a move has been chosen
            2 => {
                slot.volatile_statuses.remove(&PokemonVolatileStatus::TAUNT);
                incoming_instructions
                    .instruction_list
                    .push(Instruction::RemoveVolatileStatus(
                        RemoveVolatileStatusInstruction {
                            side_ref: attacking_side,
                            slot_ref: attacking_slot,
                            volatile_status: PokemonVolatileStatus::TAUNT,
                        },
                    ));
                incoming_instructions.instruction_list.push(
                    Instruction::ChangeVolatileStatusDuration(
                        ChangeVolatileStatusDurationInstruction {
                            side_ref: attacking_side,
                            slot_ref: attacking_slot,
                            volatile_status: PokemonVolatileStatus::TAUNT,
                            amount: -2,
                        },
                    ),
                );
                slot.volatile_status_durations.taunt = 0;
                state.re_enable_disabled_moves(
                    &attacking_side,
                    &attacking_slot,
                    &mut incoming_instructions.instruction_list,
                );
            }
            _ => panic!(
                "Taunt duration cannot be {} when taunt volatile is active",
                slot.volatile_status_durations.taunt
            ),
        }
    }

    // if any pkmn is pivoting besides this pkmn and we still need to move, set our pending move and return
    let sides = [SideReference::SideOne, SideReference::SideTwo];
    let slots = [SlotReference::SlotA, SlotReference::SlotB];
    for side_ref in sides {
        for slot_ref in slots {
            if side_ref == attacking_side && slot_ref == attacking_slot {
                continue;
            }
            if state
                .get_side_immutable(&side_ref)
                .get_slot_immutable(&slot_ref)
                .force_switch
            {
                state
                    .get_side(&attacking_side)
                    .get_slot(&attacking_slot)
                    .switch_out_move_second_saved_move =
                    MoveChoice::Move(target_slot, target_side, choice.move_index);
                state.reverse_instructions(&incoming_instructions.instruction_list);
                final_instructions.push((incoming_instructions, remaining_to_move));
                return;
            }
        }
    }

    if state
        .get_side_immutable(&attacking_side)
        .get_active_immutable(&attacking_slot)
        .hp
        == 0
    {
        state.reverse_instructions(&incoming_instructions.instruction_list);
        final_instructions.push((incoming_instructions, remaining_to_move));
        return;
    }

    // If the move is a charge move, remove the volatile status if damage was done
    if choice.flags.charge {
        let side = state.get_side(&attacking_side);
        let slot = side.get_slot(&attacking_slot);
        let volatile_status = charge_choice_to_volatile(&choice.move_id);
        if slot.volatile_statuses.contains(&volatile_status) {
            choice.flags.charge = false;
            let instruction = Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                side_ref: attacking_side,
                slot_ref: attacking_slot,
                volatile_status,
            });
            incoming_instructions.instruction_list.push(instruction);
            slot.volatile_statuses.remove(&volatile_status);
        }
    }

    // move_change_type
    // need to pull out the things in abilities/items/choice_effects that change the type of a move
    // these type changes need to be done _before_ this redirection logic because the type of a
    // move may change the target of a move
    ability_change_type(state, choice, &attacking_side, &attacking_slot);
    item_change_type(state, choice, &attacking_side, &attacking_slot);
    choice_change_type(state, choice, &attacking_side, &attacking_slot);

    if choice.move_choice_target == MoveChoiceTarget::Normal {
        let attacker = state
            .get_side_immutable(&attacking_side)
            .get_active_immutable(&attacking_slot);
        let attacker_partner = state
            .get_side_immutable(&attacking_side)
            .get_active_immutable(&attacking_slot.get_other_slot());
        let target = state
            .get_side_immutable(&target_side)
            .get_active_immutable(&target_slot);
        let _target_partner = state
            .get_side_immutable(&target_side)
            .get_active_immutable(&target_slot.get_other_slot());
        let target_partner_slot = state
            .get_side_immutable(&target_side)
            .get_slot_immutable(&target_slot.get_other_slot());
        let other_side_a = state
            .get_side_immutable(&attacking_side.get_other_side())
            .get_active_immutable(&SlotReference::SlotA);
        let other_side_b = state
            .get_side_immutable(&attacking_side.get_other_side())
            .get_active_immutable(&SlotReference::SlotB);

        let target_partner_alive = _target_partner.hp > 0;

        // redirect move if target has fainted and you are targeting the other side
        if target_side != attacking_side
            && (target.hp == 0
                || (target_partner_slot
                    .volatile_statuses
                    .contains(&PokemonVolatileStatus::RAGEPOWDER)
                    && !attacker.immune_to_rage_powder_redirection()
                    && target_partner_alive)
                || (target_partner_alive
                    && target_partner_slot
                        .volatile_statuses
                        .contains(&PokemonVolatileStatus::FOLLOWME)))
        {
            target_slot = target_slot.get_other_slot();
        }

        // redirect if something is forcing the redirect
        if attacker_partner.redirects_move_to_self(choice) {
            target_side = attacking_side;
            target_slot = attacking_slot.get_other_slot();
        } else if other_side_a.redirects_move_to_self(choice) {
            target_side = attacking_side.get_other_side();
            target_slot = SlotReference::SlotA
        } else if other_side_b.redirects_move_to_self(choice) {
            target_side = attacking_side.get_other_side();
            target_slot = SlotReference::SlotB
        }
    }

    let mut target_has_moved = true;
    for remaining in &remaining_to_move {
        if remaining.side_ref == target_side && remaining.slot_ref == target_slot {
            target_has_moved = false;
            break;
        }
    }

    before_move(
        state,
        choice,
        defender_choice,
        &attacking_side,
        &attacking_slot,
        &target_side,
        &target_slot,
        final_run_move,
        target_has_moved,
        &mut incoming_instructions,
    );
    if incoming_instructions.percentage == 0.0 {
        state.reverse_instructions(&incoming_instructions.instruction_list);
        return;
    }

    if state.use_last_used_move {
        set_last_used_move_as_move(
            state.get_side(&attacking_side),
            attacking_slot,
            choice.move_index,
            attacking_side,
            &mut incoming_instructions,
        );
    }

    if cannot_use_move(
        state,
        &choice,
        &attacking_side,
        &attacking_slot,
        &target_side,
        &target_slot,
    ) {
        state.reverse_instructions(&incoming_instructions.instruction_list);
        final_instructions.push((incoming_instructions, remaining_to_move));
        return;
    }

    // TODO: this is not correct because PP should only decrement once per move, but this
    //       will decrement it multiple times if the move is a spread move or targets everthing
    // most of the time pp decrement doesn't matter and just adds another instruction
    // so we only decrement pp if the move is at 10 or less pp since that is when it starts
    // to matter
    // let target_has_pressure = state
    //     .get_side_immutable(&target_side)
    //     .get_active_immutable(&target_slot)
    //     .ability
    //     == Abilities::PRESSURE;
    // let attacker_side = state.get_side(&attacking_side);
    // let attacker_slot = attacker_side.get_slot(&attacking_slot);
    // let attacker_active_index = attacker_slot.active_index;
    // let active = attacker_side.get_active(&attacking_slot);
    // if active.moves[&choice.move_index].pp < 10 {
    //     let pp_decrement_amount = if choice.target == MoveTarget::Target && target_has_pressure {
    //         2
    //     } else {
    //         1
    //     };
    //     incoming_instructions
    //         .instruction_list
    //         .push(Instruction::DecrementPP(DecrementPPInstruction {
    //             side_ref: attacking_side,
    //             pokemon_index: attacker_active_index,
    //             move_index: choice.move_index,
    //             amount: pp_decrement_amount,
    //         }));
    //     active.moves[&choice.move_index].pp -= pp_decrement_amount;
    // }

    if final_run_move && !choice.sleep_talk_move {
        generate_instructions_from_existing_status_conditions(
            state,
            &attacking_side,
            &attacking_slot,
            &choice,
            &mut incoming_instructions,
            &mut final_instructions,
            &remaining_to_move,
        );
    }
    let attacker = state
        .get_side_immutable(&attacking_side)
        .get_active_immutable(&attacking_slot);
    if choice.move_id == Choices::SLEEPTALK && attacker.status == PokemonStatus::SLEEP {
        let new_choices = attacker.get_sleep_talk_choices();
        state.reverse_instructions(&incoming_instructions.instruction_list);
        let num_choices = new_choices.len() as f32;
        for mut new_choice in new_choices {
            new_choice.sleep_talk_move = true;
            let mut sleep_talk_instructions = incoming_instructions.clone();
            sleep_talk_instructions.update_percentage(1.0 / num_choices);
            run_move(
                state,
                &mut new_choice,
                defender_choice,
                attacking_side,
                attacking_slot,
                target_side,
                target_slot,
                sleep_talk_instructions,
                &mut final_instructions,
                remaining_to_move.clone(),
                final_run_move,
                false,
            );
        }
        return;
    } else if attacker.status == PokemonStatus::SLEEP && !choice.sleep_talk_move {
        state.reverse_instructions(&incoming_instructions.instruction_list);
        if incoming_instructions.percentage > 0.0 {
            final_instructions.push((incoming_instructions, remaining_to_move));
        }
        return;
    }

    if move_has_no_effect(state, &choice, &target_side, &target_slot) {
        state.reverse_instructions(&incoming_instructions.instruction_list);
        final_instructions.push((incoming_instructions, remaining_to_move));
        return;
    }
    choice_special_effect(
        state,
        choice,
        &attacking_side,
        &attacking_slot,
        &target_side,
        &target_slot,
        &mut incoming_instructions,
    );
    let damage = calculate_damage(
        state,
        &attacking_side,
        &attacking_slot,
        &target_side,
        &target_slot,
        &choice,
        DamageRolls::Max,
    );
    check_move_hit_or_miss(
        state,
        &choice,
        &attacking_side,
        &attacking_slot,
        damage,
        &mut incoming_instructions,
        &mut final_instructions,
        &remaining_to_move,
    );

    if incoming_instructions.percentage == 0.0 {
        state.reverse_instructions(&incoming_instructions.instruction_list);
        return;
    }

    // start multi-hit
    let hit_count;
    match choice.multi_hit() {
        MultiHitMove::None => {
            hit_count = 1;
        }
        MultiHitMove::DoubleHit => {
            hit_count = 2;
        }
        MultiHitMove::TripleHit => {
            hit_count = 3;
        }
        MultiHitMove::TwoToFiveHits => {
            hit_count = if state
                .get_side(&attacking_side)
                .get_active(&attacking_slot)
                .ability
                == Abilities::SKILLLINK
            {
                5
            } else if state
                .get_side(&attacking_side)
                .get_active(&attacking_slot)
                .item
                == Items::LOADEDDICE
            {
                4
            } else {
                3 // too lazy to implement branching here. Average is 3.2 so this is a fine approximation
            };
        }
        MultiHitMove::PopulationBomb => {
            // population bomb checks accuracy each time but lets approximate
            hit_count = if state
                .get_side(&attacking_side)
                .get_active(&attacking_slot)
                .item
                == Items::WIDELENS
            {
                9
            } else {
                6
            };
        }
        MultiHitMove::TripleAxel => {
            // triple axel checks accuracy each time but until multi-accuracy is implemented this
            // is the best we can do
            hit_count = 3
        }
    }

    let defender_side = state.get_side(&target_side);
    let defender_active = defender_side.get_active(&target_slot);
    let mut does_damage = false;
    let (mut branch_damage, mut regular_damage) = (0, 0);
    let mut branch_instructions: Option<StateInstructions> = None;
    if let Some((max_damage_dealt, max_crit_damage)) = damage {
        does_damage = true;
        let avg_damage_dealt = (max_damage_dealt as f32 * 0.925) as i16;
        let min_damage_dealt = (max_damage_dealt as f32 * 0.85) as i16;
        if branch_on_damage
            && max_damage_dealt >= defender_active.hp
            && min_damage_dealt < defender_active.hp
        {
            let (average_non_kill_damage, num_kill_rolls) =
                compare_health_with_damage_multiples(max_damage_dealt, defender_active.hp);

            let crit_rate = if defender_active.ability == Abilities::BATTLEARMOR
                || defender_active.ability == Abilities::SHELLARMOR
            {
                0.0
            } else if choice.move_id.guaranteed_crit() {
                1.0
            } else if choice.move_id.increased_crit_ratio() {
                1.0 / 8.0
            } else {
                BASE_CRIT_CHANCE
            };

            // the chance of a branch is the chance of the roll killing + the chance of a crit
            let branch_chance = ((1.0 - crit_rate) * (num_kill_rolls as f32 / 16.0)) + crit_rate;

            let mut branch_ins = incoming_instructions.clone();
            branch_ins.update_percentage(branch_chance);
            branch_instructions = Some(branch_ins);
            branch_damage = defender_active.hp;

            incoming_instructions.update_percentage(1.0 - branch_chance);
            regular_damage = average_non_kill_damage;
        } else if branch_on_damage && max_damage_dealt < defender_active.hp {
            let crit_rate = if defender_active.ability == Abilities::BATTLEARMOR
                || defender_active.ability == Abilities::SHELLARMOR
            {
                0.0
            } else if choice.move_id.guaranteed_crit() {
                1.0
            } else if choice.move_id.increased_crit_ratio() {
                1.0 / 8.0
            } else {
                BASE_CRIT_CHANCE
            };
            let mut branch_ins = incoming_instructions.clone();
            branch_ins.update_percentage(crit_rate);
            branch_instructions = Some(branch_ins);
            branch_damage = (max_crit_damage as f32 * 0.925) as i16;
            incoming_instructions.update_percentage(1.0 - crit_rate);
            regular_damage = (max_damage_dealt as f32 * 0.925) as i16;
        } else {
            regular_damage = avg_damage_dealt;
        }
    }

    if incoming_instructions.percentage != 0.0 {
        execute_move_effects(
            state,
            attacking_side,
            attacking_slot,
            target_side,
            target_slot,
            incoming_instructions,
            hit_count,
            does_damage,
            regular_damage,
            choice,
            defender_choice,
            &mut final_instructions,
            &remaining_to_move,
        );
    } else {
        state.reverse_instructions(&incoming_instructions.instruction_list);
    }

    // A branch representing either a roll that kills the opponent or a crit
    if let Some(branch_ins) = branch_instructions {
        if branch_ins.percentage != 0.0 {
            state.apply_instructions(&branch_ins.instruction_list);
            execute_move_effects(
                state,
                attacking_side,
                attacking_slot,
                target_side,
                target_slot,
                branch_ins,
                hit_count,
                does_damage,
                branch_damage,
                choice,
                defender_choice,
                &mut final_instructions,
                &remaining_to_move,
            );
        }
    }

    return;
}

fn remove_low_chance_instructions(
    instructions: &mut Vec<(StateInstructions, Vec<RemainingToMove>)>,
    threshold: f32,
) {
    let total_percentage: f32 = instructions.iter().map(|(ins, _)| ins.percentage).sum();
    let min_percentage = total_percentage * threshold / 100.0;

    let mut new_total = 0.0;
    instructions.retain(|(instruction, _)| {
        if instruction.percentage < min_percentage {
            false
        } else {
            new_total += instruction.percentage;
            true
        }
    });
    for instruction in instructions.iter_mut() {
        instruction.0.percentage = instruction.0.percentage * 100.0 / new_total;
    }
}

fn combine_duplicate_instructions(
    list_of_instructions: &mut Vec<(StateInstructions, Vec<RemainingToMove>)>,
) {
    for i in 0..list_of_instructions.len() {
        let mut j = i + 1;
        while j < list_of_instructions.len() {
            if list_of_instructions[i].0.instruction_list
                == list_of_instructions[j].0.instruction_list
            {
                list_of_instructions[i].0.percentage += list_of_instructions[j].0.percentage;
                list_of_instructions.remove(j);
            } else {
                j += 1;
            }
        }
    }
}

pub fn get_effective_speed(
    state: &State,
    side_reference: &SideReference,
    slot_reference: &SlotReference,
) -> i16 {
    let side = state.get_side_immutable(side_reference);
    let active_pkmn = side.get_active_immutable(slot_reference);

    let mut boosted_speed =
        side.calculate_boosted_stat(slot_reference, PokemonBoostableStat::Speed) as f32;

    let slot = side.get_slot_immutable(slot_reference);
    match state.weather.weather_type {
        Weather::SUN | Weather::HARSHSUN if active_pkmn.ability == Abilities::CHLOROPHYLL => {
            boosted_speed *= 2.0
        }
        Weather::RAIN | Weather::HEAVYRAIN if active_pkmn.ability == Abilities::SWIFTSWIM => {
            boosted_speed *= 2.0
        }
        Weather::SAND if active_pkmn.ability == Abilities::SANDRUSH => boosted_speed *= 2.0,
        Weather::HAIL if active_pkmn.ability == Abilities::SLUSHRUSH => boosted_speed *= 2.0,
        _ => {}
    }

    match active_pkmn.ability {
        Abilities::SURGESURFER if state.terrain.terrain_type == Terrain::ELECTRICTERRAIN => {
            boosted_speed *= 2.0
        }
        Abilities::UNBURDEN
            if slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::UNBURDEN) =>
        {
            boosted_speed *= 2.0
        }
        Abilities::QUICKFEET if active_pkmn.status != PokemonStatus::NONE => boosted_speed *= 1.5,
        _ => {}
    }

    if slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::SLOWSTART)
    {
        boosted_speed *= 0.5;
    }

    if slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::PROTOSYNTHESISSPE)
        || slot
            .volatile_statuses
            .contains(&PokemonVolatileStatus::QUARKDRIVESPE)
    {
        boosted_speed *= 1.5;
    }

    if side.side_conditions.tailwind > 0 {
        boosted_speed *= 2.0
    }

    match active_pkmn.item {
        Items::IRONBALL => boosted_speed *= 0.5,
        Items::CHOICESCARF => boosted_speed *= 1.5,
        _ => {}
    }

    if active_pkmn.status == PokemonStatus::PARALYZE && active_pkmn.ability != Abilities::QUICKFEET
    {
        boosted_speed *= 0.50;
    }

    boosted_speed as i16
}

fn get_effective_priority(
    state: &State,
    side_reference: &SideReference,
    slot_reference: &SlotReference,
    choice: &Choice,
) -> i8 {
    let mut priority = choice.priority;
    let side = state.get_side_immutable(side_reference);
    let active_pkmn = side.get_active_immutable(slot_reference);

    if choice.move_id == Choices::GRASSYGLIDE && state.terrain_is_active(&Terrain::GRASSYTERRAIN) {
        priority += 1;
    }

    match active_pkmn.ability {
        Abilities::PRANKSTER if choice.category == MoveCategory::Status => priority += 1,
        Abilities::GALEWINGS
            if choice.move_type == PokemonType::FLYING && active_pkmn.hp == active_pkmn.maxhp =>
        {
            priority += 1
        }
        Abilities::TRIAGE if choice.flags.heal => priority += 3,
        _ => {}
    }

    priority
}

fn modify_choice_before_move(
    state: &State,
    side_reference: &SideReference,
    slot_reference: &SlotReference,
    choice: &mut Choice,
    pkmn_just_used_tera: bool,
) {
    let side = state.get_side_immutable(side_reference);
    let active_pkmn = side.get_active_immutable(slot_reference);
    match choice.move_id {
        Choices::TERASTARSTORM if pkmn_just_used_tera || active_pkmn.terastallized => {
            choice.move_choice_target = MoveChoiceTarget::AllFoes;
            choice.move_type = PokemonType::STELLAR;
        }
        Choices::EXPANDINGFORCE if state.terrain_is_active(&Terrain::PSYCHICTERRAIN) => {
            choice.move_choice_target = MoveChoiceTarget::AllFoes;
        }
        _ => {}
    }
}

fn speed_comparison(speed: i16, best_speed: i16, trick_room_active: bool) -> bool {
    if trick_room_active {
        speed <= best_speed
    } else {
        speed >= best_speed
    }
}

fn next_to_move(
    state: &State,
    need_to_move: &Vec<RemainingToMove>,
) -> (SideReference, SlotReference, usize, i8) {
    let mut best_index = 0;
    let mut best_speed = 0;
    let mut best_priority = -10;
    let mut found_switch = false;

    // Single pass to find the best move
    for (index, remaining_to_move) in need_to_move.iter().enumerate() {
        let is_switch = remaining_to_move.choice.category == MoveCategory::Switch;
        let speed = get_effective_speed(
            state,
            &remaining_to_move.side_ref,
            &remaining_to_move.slot_ref,
        );
        let priority = if is_switch {
            10
        } else {
            get_effective_priority(
                state,
                &remaining_to_move.side_ref,
                &remaining_to_move.slot_ref,
                &remaining_to_move.choice,
            )
        };

        let is_better = if found_switch && !is_switch {
            false // we already found a switch, non-switches can't be better
        } else if !found_switch && is_switch {
            true // first switch we found
        } else {
            // same category, compare priority then speed
            priority > best_priority
                || (priority == best_priority
                    && speed_comparison(speed, best_speed, state.trick_room.active))
        };

        if is_better {
            best_index = index;
            best_speed = speed;
            best_priority = priority;
            if is_switch {
                found_switch = true;
            }
        }
    }

    let best_move = &need_to_move[best_index];
    (
        best_move.side_ref,
        best_move.slot_ref,
        best_index,
        best_priority,
    )
}

fn get_active_protosynthesis(slot: &SideSlot) -> Option<PokemonVolatileStatus> {
    if slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::PROTOSYNTHESISATK)
    {
        Some(PokemonVolatileStatus::PROTOSYNTHESISATK)
    } else if slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::PROTOSYNTHESISDEF)
    {
        Some(PokemonVolatileStatus::PROTOSYNTHESISDEF)
    } else if slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::PROTOSYNTHESISSPA)
    {
        Some(PokemonVolatileStatus::PROTOSYNTHESISSPA)
    } else if slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::PROTOSYNTHESISSPD)
    {
        Some(PokemonVolatileStatus::PROTOSYNTHESISSPD)
    } else if slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::PROTOSYNTHESISSPE)
    {
        Some(PokemonVolatileStatus::PROTOSYNTHESISSPE)
    } else {
        None
    }
}

fn get_active_quarkdrive(slot: &SideSlot) -> Option<PokemonVolatileStatus> {
    if slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::QUARKDRIVEATK)
    {
        Some(PokemonVolatileStatus::QUARKDRIVEATK)
    } else if slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::QUARKDRIVEDEF)
    {
        Some(PokemonVolatileStatus::QUARKDRIVEDEF)
    } else if slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::QUARKDRIVESPA)
    {
        Some(PokemonVolatileStatus::QUARKDRIVESPA)
    } else if slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::QUARKDRIVESPD)
    {
        Some(PokemonVolatileStatus::QUARKDRIVESPD)
    } else if slot
        .volatile_statuses
        .contains(&PokemonVolatileStatus::QUARKDRIVESPE)
    {
        Some(PokemonVolatileStatus::QUARKDRIVESPE)
    } else {
        None
    }
}

fn on_weather_end(
    state: &mut State,
    sides: [&SideReference; 2],
    incoming_instructions: &mut StateInstructions,
) {
    match state.weather.weather_type {
        Weather::SUN => {
            for side_ref in sides {
                let side = state.get_side(side_ref);
                for slot_ref in [&SlotReference::SlotA, &SlotReference::SlotB] {
                    let active_index = side.get_slot_immutable(slot_ref).active_index;
                    if side.get_active_immutable(slot_ref).ability == Abilities::PROTOSYNTHESIS {
                        let slot = side.get_slot(slot_ref);
                        if let Some(volatile_status) = get_active_protosynthesis(slot) {
                            let active = side.get_active(slot_ref);
                            if active.item == Items::BOOSTERENERGY {
                                incoming_instructions.instruction_list.push(
                                    Instruction::ChangeItem(ChangeItemInstruction {
                                        side_ref: *side_ref,
                                        pokemon_index: active_index,
                                        current_item: Items::BOOSTERENERGY,
                                        new_item: Items::NONE,
                                    }),
                                );
                                active.item = Items::NONE;
                            } else {
                                incoming_instructions.instruction_list.push(
                                    Instruction::RemoveVolatileStatus(
                                        RemoveVolatileStatusInstruction {
                                            side_ref: *side_ref,
                                            slot_ref: *slot_ref,
                                            volatile_status,
                                        },
                                    ),
                                );
                                side.get_slot(slot_ref)
                                    .volatile_statuses
                                    .remove(&volatile_status);
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn on_terrain_end(
    state: &mut State,
    sides: [&SideReference; 2],
    incoming_instructions: &mut StateInstructions,
) {
    match state.terrain.terrain_type {
        Terrain::ELECTRICTERRAIN => {
            for side_ref in sides {
                let side = state.get_side(side_ref);
                for slot_ref in [&SlotReference::SlotA, &SlotReference::SlotB] {
                    let active_index = side.get_slot_immutable(slot_ref).active_index;
                    if side.get_active_immutable(slot_ref).ability == Abilities::QUARKDRIVE {
                        let slot = side.get_slot(slot_ref);
                        if let Some(volatile_status) = get_active_quarkdrive(slot) {
                            let active = side.get_active(slot_ref);
                            if active.item == Items::BOOSTERENERGY {
                                incoming_instructions.instruction_list.push(
                                    Instruction::ChangeItem(ChangeItemInstruction {
                                        side_ref: *side_ref,
                                        pokemon_index: active_index,
                                        current_item: Items::BOOSTERENERGY,
                                        new_item: Items::NONE,
                                    }),
                                );
                                active.item = Items::NONE;
                            } else {
                                incoming_instructions.instruction_list.push(
                                    Instruction::RemoveVolatileStatus(
                                        RemoveVolatileStatusInstruction {
                                            side_ref: *side_ref,
                                            slot_ref: *slot_ref,
                                            volatile_status,
                                        },
                                    ),
                                );
                                side.get_slot(slot_ref)
                                    .volatile_statuses
                                    .remove(&volatile_status);
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn add_end_of_turn_instructions(
    state: &mut State,
    mut incoming_instructions: &mut StateInstructions,
    first_move_side: &SideReference,
) {
    if state.side_one.slot_a.force_switch
        || state.side_one.slot_b.force_switch
        || state.side_two.slot_a.force_switch
        || state.side_two.slot_b.force_switch
    {
        return;
    }

    let sides = [first_move_side, &first_move_side.get_other_side()];
    let slots = [&SlotReference::SlotA, &SlotReference::SlotB];

    // Weather decrement / dissipation
    if state.weather.turns_remaining > 0 && state.weather.weather_type != Weather::NONE {
        let weather_dissipate_instruction = Instruction::DecrementWeatherTurnsRemaining;
        incoming_instructions
            .instruction_list
            .push(weather_dissipate_instruction);
        state.weather.turns_remaining -= 1;
        if state.weather.turns_remaining == 0 {
            on_weather_end(state, sides, &mut incoming_instructions);
            let weather_end_instruction = Instruction::ChangeWeather(ChangeWeather {
                new_weather: Weather::NONE,
                new_weather_turns_remaining: 0,
                previous_weather: state.weather.weather_type,
                previous_weather_turns_remaining: 0,
            });
            incoming_instructions
                .instruction_list
                .push(weather_end_instruction);
            state.weather.weather_type = Weather::NONE;
        }
    }

    // Trick Room decrement / dissipation
    if state.trick_room.turns_remaining > 0 && state.trick_room.active {
        incoming_instructions
            .instruction_list
            .push(Instruction::DecrementTrickRoomTurnsRemaining);
        state.trick_room.turns_remaining -= 1;
        if state.trick_room.turns_remaining == 0 {
            incoming_instructions
                .instruction_list
                .push(Instruction::ToggleTrickRoom(ToggleTrickRoomInstruction {
                    currently_active: true,
                    new_trickroom_turns_remaining: 0,
                    previous_trickroom_turns_remaining: 0,
                }));
            state.trick_room.active = false;
        }
    }

    // Terrain decrement / dissipation
    if state.terrain.turns_remaining > 0 && state.terrain.terrain_type != Terrain::NONE {
        let terrain_dissipate_instruction = Instruction::DecrementTerrainTurnsRemaining;
        incoming_instructions
            .instruction_list
            .push(terrain_dissipate_instruction);
        state.terrain.turns_remaining -= 1;
        if state.terrain.turns_remaining == 0 {
            on_terrain_end(state, sides, &mut incoming_instructions);
            let terrain_end_instruction = Instruction::ChangeTerrain(ChangeTerrain {
                new_terrain: Terrain::NONE,
                new_terrain_turns_remaining: 0,
                previous_terrain: state.terrain.terrain_type,
                previous_terrain_turns_remaining: 0,
            });
            incoming_instructions
                .instruction_list
                .push(terrain_end_instruction);
            state.terrain.terrain_type = Terrain::NONE;
        }
    }

    // Side Condition decrement
    for side_ref in sides {
        let side = state.get_side(side_ref);
        if side.side_conditions.wide_guard > 0 {
            incoming_instructions
                .instruction_list
                .push(Instruction::ChangeSideCondition(
                    ChangeSideConditionInstruction {
                        side_ref: *side_ref,
                        side_condition: PokemonSideCondition::WideGuard,
                        amount: -1,
                    },
                ));
            side.side_conditions.wide_guard -= 1;
        }
        if side.side_conditions.reflect > 0 {
            incoming_instructions
                .instruction_list
                .push(Instruction::ChangeSideCondition(
                    ChangeSideConditionInstruction {
                        side_ref: *side_ref,
                        side_condition: PokemonSideCondition::Reflect,
                        amount: -1,
                    },
                ));
            side.side_conditions.reflect -= 1;
        }
        if side.side_conditions.light_screen > 0 {
            incoming_instructions
                .instruction_list
                .push(Instruction::ChangeSideCondition(
                    ChangeSideConditionInstruction {
                        side_ref: *side_ref,
                        side_condition: PokemonSideCondition::LightScreen,
                        amount: -1,
                    },
                ));
            side.side_conditions.light_screen -= 1;
        }
        if side.side_conditions.aurora_veil > 0 {
            incoming_instructions
                .instruction_list
                .push(Instruction::ChangeSideCondition(
                    ChangeSideConditionInstruction {
                        side_ref: *side_ref,
                        side_condition: PokemonSideCondition::AuroraVeil,
                        amount: -1,
                    },
                ));
            side.side_conditions.aurora_veil -= 1;
        }
        if side.side_conditions.tailwind > 0 {
            incoming_instructions
                .instruction_list
                .push(Instruction::ChangeSideCondition(
                    ChangeSideConditionInstruction {
                        side_ref: *side_ref,
                        side_condition: PokemonSideCondition::Tailwind,
                        amount: -1,
                    },
                ));
            side.side_conditions.tailwind -= 1;
        }
    }

    // Weather Damage
    if state.weather_is_active(&Weather::SAND) {
        for side_ref in sides {
            for slot_ref in slots {
                let side = state.get_side(side_ref);
                let active_index = side.get_slot_immutable(slot_ref).active_index;
                let active_pkmn = side.get_active(slot_ref);
                if active_pkmn.hp == 0
                    || active_pkmn.ability == Abilities::MAGICGUARD
                    || active_pkmn.ability == Abilities::OVERCOAT
                    || active_pkmn.has_type(&PokemonType::GROUND)
                    || active_pkmn.has_type(&PokemonType::STEEL)
                    || active_pkmn.has_type(&PokemonType::ROCK)
                {
                    continue;
                }
                let damage_amount =
                    cmp::min((active_pkmn.maxhp as f32 * 0.0625) as i16, active_pkmn.hp);
                let sand_damage_instruction = Instruction::Damage(DamageInstruction {
                    side_ref: *side_ref,
                    pokemon_index: active_index,
                    damage_amount,
                });
                active_pkmn.hp -= damage_amount;
                incoming_instructions
                    .instruction_list
                    .push(sand_damage_instruction);
            }
        }
    }

    // TODO: wish & future sight need a target now that there are 2 pkmn. Need to come back to this
    // for side_ref in sides {
    //     let (attacking_side, defending_side) = state.get_both_sides(side_ref);
    //     if attacking_side.future_sight.0 > 0 {
    //         let decrement_future_sight_instruction =
    //             Instruction::DecrementFutureSight(DecrementFutureSightInstruction {
    //                 side_ref: *side_ref,
    //             });
    //         if attacking_side.future_sight.0 == 1 {
    //             let mut damage = calculate_futuresight_damage(
    //                 &attacking_side,
    //                 &defending_side,
    //                 &attacking_side.future_sight.1,
    //             );
    //             let defender = defending_side.get_active();
    //             damage = cmp::min(damage, defender.hp);
    //             let future_sight_damage_instruction = Instruction::Damage(DamageInstruction {
    //                 side_ref: side_ref.get_other_side(),
    //                 damage_amount: damage,
    //             });
    //             incoming_instructions
    //                 .instruction_list
    //                 .push(future_sight_damage_instruction);
    //             defender.hp -= damage;
    //         }
    //         attacking_side.future_sight.0 -= 1;
    //         incoming_instructions
    //             .instruction_list
    //             .push(decrement_future_sight_instruction);
    //     }
    // }
    //
    // wish
    // for side_ref in sides {
    //     let side = state.get_side(side_ref);
    //     let side_wish = side.wish;
    //     let active_pkmn = side.get_active();
    //
    //     if side_wish.0 > 0 {
    //         let decrement_wish_instruction = Instruction::DecrementWish(DecrementWishInstruction {
    //             side_ref: *side_ref,
    //         });
    //         if side_wish.0 == 1 && 0 < active_pkmn.hp && active_pkmn.hp < active_pkmn.maxhp {
    //             #[cfg(not(feature = "gen4"))]
    //             let heal_amount = cmp::min(active_pkmn.maxhp - active_pkmn.hp, side_wish.1);
    //
    //             #[cfg(feature = "gen4")]
    //             let heal_amount =
    //                 cmp::min(active_pkmn.maxhp - active_pkmn.hp, active_pkmn.maxhp / 2);
    //
    //             let wish_heal_instruction = Instruction::Heal(HealInstruction {
    //                 side_ref: *side_ref,
    //                 heal_amount: heal_amount,
    //             });
    //             incoming_instructions
    //                 .instruction_list
    //                 .push(wish_heal_instruction);
    //             active_pkmn.hp += heal_amount;
    //         }
    //         side.wish.0 -= 1;
    //         incoming_instructions
    //             .instruction_list
    //             .push(decrement_wish_instruction);
    //     }
    // }

    // status damage
    let neutralizing_gas_active = state.neutralizing_gas_is_active();
    for side_ref in sides {
        for slot_ref in slots {
            let side = state.get_side(side_ref);
            let toxic_count = side.side_conditions.toxic_count as f32;
            let active_pkmn_index = side.get_slot(slot_ref).active_index;
            let active_pkmn = side.get_active(slot_ref);
            if active_pkmn.hp == 0 || active_pkmn.ability == Abilities::MAGICGUARD {
                continue;
            }

            match active_pkmn.status {
                PokemonStatus::BURN => {
                    let mut damage_factor = 0.0625;

                    if active_pkmn.ability == Abilities::HEATPROOF {
                        damage_factor /= 2.0;
                    }
                    let damage_amount = cmp::max(
                        cmp::min(
                            (active_pkmn.maxhp as f32 * damage_factor) as i16,
                            active_pkmn.hp,
                        ),
                        1,
                    );
                    let burn_damage_instruction = Instruction::Damage(DamageInstruction {
                        side_ref: *side_ref,
                        pokemon_index: active_pkmn_index,
                        damage_amount,
                    });
                    active_pkmn.hp -= damage_amount;
                    incoming_instructions
                        .instruction_list
                        .push(burn_damage_instruction);
                }
                PokemonStatus::POISON if active_pkmn.ability != Abilities::POISONHEAL => {
                    let damage_amount = cmp::max(
                        1,
                        cmp::min((active_pkmn.maxhp as f32 * 0.125) as i16, active_pkmn.hp),
                    );

                    let poison_damage_instruction = Instruction::Damage(DamageInstruction {
                        side_ref: *side_ref,
                        pokemon_index: active_pkmn_index,
                        damage_amount,
                    });
                    active_pkmn.hp -= damage_amount;
                    incoming_instructions
                        .instruction_list
                        .push(poison_damage_instruction);
                }
                PokemonStatus::TOXIC => {
                    if active_pkmn.ability != Abilities::POISONHEAL || neutralizing_gas_active {
                        let toxic_multiplier = (1.0 / 16.0) * toxic_count + (1.0 / 16.0);
                        let damage_amount = cmp::max(
                            cmp::min(
                                (active_pkmn.maxhp as f32 * toxic_multiplier) as i16,
                                active_pkmn.hp,
                            ),
                            1,
                        );
                        let toxic_damage_instruction = Instruction::Damage(DamageInstruction {
                            side_ref: *side_ref,
                            pokemon_index: active_pkmn_index,
                            damage_amount,
                        });

                        active_pkmn.hp -= damage_amount;
                        incoming_instructions
                            .instruction_list
                            .push(toxic_damage_instruction);
                    }

                    // toxic counter is always incremented, even if the pokemon has poison heal
                    let toxic_counter_increment_instruction =
                        Instruction::ChangeSideCondition(ChangeSideConditionInstruction {
                            side_ref: *side_ref,
                            side_condition: PokemonSideCondition::ToxicCount,
                            amount: 1,
                        });
                    side.side_conditions.toxic_count += 1;
                    incoming_instructions
                        .instruction_list
                        .push(toxic_counter_increment_instruction);
                }
                _ => {}
            }
        }
    }

    // ability/item end-of-turn effects
    for side_ref in sides {
        for slot_ref in slots {
            let side = state.get_side(side_ref);
            let active_pkmn = side.get_active(slot_ref);
            if active_pkmn.hp == 0 {
                continue;
            }
            item_end_of_turn(state, side_ref, slot_ref, &mut incoming_instructions);
            ability_end_of_turn(state, side_ref, slot_ref, &mut incoming_instructions);
        }
    }

    // TODO: Leechseed needs a fkn source now? Fuck that
    // leechseed sap
    // for side_ref in sides {
    //     let (leechseed_side, other_side) = state.get_both_sides(side_ref);
    //     if leechseed_side
    //         .volatile_statuses
    //         .contains(&PokemonVolatileStatus::LEECHSEED)
    //     {
    //         let active_pkmn = leechseed_side.get_active();
    //         let other_active_pkmn = other_side.get_active();
    //         if active_pkmn.hp == 0
    //             || other_active_pkmn.hp == 0
    //             || active_pkmn.ability == Abilities::MAGICGUARD
    //         {
    //             continue;
    //         }
    //
    //         let health_sapped = cmp::min((active_pkmn.maxhp as f32 * 0.125) as i16, active_pkmn.hp);
    //         let damage_ins = Instruction::Damage(DamageInstruction {
    //             side_ref: *side_ref,
    //             damage_amount: health_sapped,
    //         });
    //         active_pkmn.hp -= health_sapped;
    //         incoming_instructions.instruction_list.push(damage_ins);
    //
    //         let health_recovered = cmp::min(
    //             health_sapped,
    //             other_active_pkmn.maxhp - other_active_pkmn.hp,
    //         );
    //         if health_recovered > 0 {
    //             let heal_ins = Instruction::Heal(HealInstruction {
    //                 side_ref: side_ref.get_other_side(),
    //                 heal_amount: health_recovered,
    //             });
    //             other_active_pkmn.hp += health_recovered;
    //             incoming_instructions.instruction_list.push(heal_ins);
    //         }
    //     }
    // }

    // volatile statuses
    for side_ref in sides {
        for slot_ref in slots {
            let side = state.get_side(side_ref);
            if side.get_active(slot_ref).hp == 0 {
                continue;
            }
            let slot = side.get_slot(slot_ref);
            let active_index = slot.active_index;
            let has_rage_powder = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::RAGEPOWDER);
            let has_follow_me = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::FOLLOWME);
            let has_helping_hand = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::HELPINGHAND);
            let has_slowstart = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::SLOWSTART);
            let has_lockedmove = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::LOCKEDMOVE);
            let has_yawn = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::YAWN);
            let has_perish_1 = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::PERISH1);
            let has_perish_2 = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::PERISH2);
            let has_perish_3 = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::PERISH3);
            let has_perish_4 = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::PERISH4);
            let has_flinch = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::FLINCH);
            let has_roost = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::ROOST);
            let has_partiallytrapped = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::PARTIALLYTRAPPED);
            let has_saltcure = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::SALTCURE);

            if has_follow_me {
                incoming_instructions
                    .instruction_list
                    .push(Instruction::RemoveVolatileStatus(
                        RemoveVolatileStatusInstruction {
                            side_ref: *side_ref,
                            slot_ref: *slot_ref,
                            volatile_status: PokemonVolatileStatus::FOLLOWME,
                        },
                    ));
                slot.volatile_statuses
                    .remove(&PokemonVolatileStatus::FOLLOWME);
            }
            if has_rage_powder {
                incoming_instructions
                    .instruction_list
                    .push(Instruction::RemoveVolatileStatus(
                        RemoveVolatileStatusInstruction {
                            side_ref: *side_ref,
                            slot_ref: *slot_ref,
                            volatile_status: PokemonVolatileStatus::RAGEPOWDER,
                        },
                    ));
                slot.volatile_statuses
                    .remove(&PokemonVolatileStatus::RAGEPOWDER);
            }

            if has_helping_hand {
                incoming_instructions
                    .instruction_list
                    .push(Instruction::RemoveVolatileStatus(
                        RemoveVolatileStatusInstruction {
                            side_ref: *side_ref,
                            slot_ref: *slot_ref,
                            volatile_status: PokemonVolatileStatus::HELPINGHAND,
                        },
                    ));
                slot.volatile_statuses
                    .remove(&PokemonVolatileStatus::HELPINGHAND);
            }

            if has_slowstart {
                incoming_instructions.instruction_list.push(
                    Instruction::ChangeVolatileStatusDuration(
                        ChangeVolatileStatusDurationInstruction {
                            side_ref: *side_ref,
                            slot_ref: *slot_ref,
                            volatile_status: PokemonVolatileStatus::SLOWSTART,
                            amount: -1,
                        },
                    ),
                );
                slot.volatile_status_durations.slowstart -= 1;
                if slot.volatile_status_durations.slowstart == 0 {
                    incoming_instructions
                        .instruction_list
                        .push(Instruction::RemoveVolatileStatus(
                            RemoveVolatileStatusInstruction {
                                side_ref: *side_ref,
                                slot_ref: *slot_ref,
                                volatile_status: PokemonVolatileStatus::SLOWSTART,
                            },
                        ));
                    slot.volatile_statuses
                        .remove(&PokemonVolatileStatus::SLOWSTART);
                }
            }

            if has_lockedmove {
                // the number says 2 but this is 3 turns of using a locking move
                // because turn 0 is the first turn the move is used
                // branching is not implemented here so the engine assumes it always lasts 3 turns
                if slot.volatile_status_durations.lockedmove == 2 {
                    slot.volatile_status_durations.lockedmove = 0;
                    slot.volatile_statuses
                        .remove(&PokemonVolatileStatus::LOCKEDMOVE);
                    incoming_instructions.instruction_list.push(
                        Instruction::ChangeVolatileStatusDuration(
                            ChangeVolatileStatusDurationInstruction {
                                side_ref: *side_ref,
                                slot_ref: *slot_ref,
                                volatile_status: PokemonVolatileStatus::LOCKEDMOVE,
                                amount: -2,
                            },
                        ),
                    );
                    incoming_instructions
                        .instruction_list
                        .push(Instruction::RemoveVolatileStatus(
                            RemoveVolatileStatusInstruction {
                                side_ref: *side_ref,
                                slot_ref: *slot_ref,
                                volatile_status: PokemonVolatileStatus::LOCKEDMOVE,
                            },
                        ));
                    if !slot
                        .volatile_statuses
                        .contains(&PokemonVolatileStatus::CONFUSION)
                    {
                        incoming_instructions.instruction_list.push(
                            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                                side_ref: *side_ref,
                                slot_ref: *slot_ref,
                                volatile_status: PokemonVolatileStatus::CONFUSION,
                            }),
                        );
                        slot.volatile_statuses
                            .insert(PokemonVolatileStatus::CONFUSION);
                    }
                } else {
                    slot.volatile_status_durations.lockedmove += 1;
                    incoming_instructions.instruction_list.push(
                        Instruction::ChangeVolatileStatusDuration(
                            ChangeVolatileStatusDurationInstruction {
                                side_ref: *side_ref,
                                slot_ref: *slot_ref,
                                volatile_status: PokemonVolatileStatus::LOCKEDMOVE,
                                amount: 1,
                            },
                        ),
                    );
                }
            }

            if has_yawn {
                match slot.volatile_status_durations.yawn {
                    0 => {
                        incoming_instructions.instruction_list.push(
                            Instruction::ChangeVolatileStatusDuration(
                                ChangeVolatileStatusDurationInstruction {
                                    side_ref: *side_ref,
                                    slot_ref: *slot_ref,
                                    volatile_status: PokemonVolatileStatus::YAWN,
                                    amount: 1,
                                },
                            ),
                        );
                        slot.volatile_status_durations.yawn += 1;
                    }
                    1 => {
                        slot.volatile_statuses.remove(&PokemonVolatileStatus::YAWN);
                        incoming_instructions.instruction_list.push(
                            Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                                side_ref: *side_ref,
                                slot_ref: *slot_ref,
                                volatile_status: PokemonVolatileStatus::YAWN,
                            }),
                        );
                        incoming_instructions.instruction_list.push(
                            Instruction::ChangeVolatileStatusDuration(
                                ChangeVolatileStatusDurationInstruction {
                                    side_ref: *side_ref,
                                    slot_ref: *slot_ref,
                                    volatile_status: PokemonVolatileStatus::YAWN,
                                    amount: -1,
                                },
                            ),
                        );
                        slot.volatile_status_durations.yawn -= 1;

                        let active = side.get_active(slot_ref);
                        if active.status == PokemonStatus::NONE {
                            active.status = PokemonStatus::SLEEP;
                            incoming_instructions
                                .instruction_list
                                .push(Instruction::ChangeStatus(ChangeStatusInstruction {
                                    side_ref: *side_ref,
                                    pokemon_index: active_index,
                                    old_status: PokemonStatus::NONE,
                                    new_status: PokemonStatus::SLEEP,
                                }));
                        }
                    }
                    _ => panic!(
                        "Yawn duration cannot be {} when yawn volatile is active",
                        slot.volatile_status_durations.yawn
                    ),
                }
            }

            if has_perish_1 {
                let active_pkmn = side.get_active(slot_ref);
                incoming_instructions
                    .instruction_list
                    .push(Instruction::Damage(DamageInstruction {
                        side_ref: *side_ref,
                        pokemon_index: active_index,
                        damage_amount: active_pkmn.hp,
                    }));
                active_pkmn.hp = 0;
            }

            let slot = side.get_slot(slot_ref);
            if has_perish_2 {
                slot.volatile_statuses
                    .remove(&PokemonVolatileStatus::PERISH2);
                slot.volatile_statuses
                    .insert(PokemonVolatileStatus::PERISH1);
                incoming_instructions
                    .instruction_list
                    .push(Instruction::RemoveVolatileStatus(
                        RemoveVolatileStatusInstruction {
                            side_ref: *side_ref,
                            slot_ref: *slot_ref,
                            volatile_status: PokemonVolatileStatus::PERISH2,
                        },
                    ));
                incoming_instructions
                    .instruction_list
                    .push(Instruction::ApplyVolatileStatus(
                        ApplyVolatileStatusInstruction {
                            side_ref: *side_ref,
                            slot_ref: *slot_ref,
                            volatile_status: PokemonVolatileStatus::PERISH1,
                        },
                    ));
            }
            if has_perish_3 {
                slot.volatile_statuses
                    .remove(&PokemonVolatileStatus::PERISH3);
                slot.volatile_statuses
                    .insert(PokemonVolatileStatus::PERISH2);
                incoming_instructions
                    .instruction_list
                    .push(Instruction::RemoveVolatileStatus(
                        RemoveVolatileStatusInstruction {
                            side_ref: *side_ref,
                            slot_ref: *slot_ref,
                            volatile_status: PokemonVolatileStatus::PERISH3,
                        },
                    ));
                incoming_instructions
                    .instruction_list
                    .push(Instruction::ApplyVolatileStatus(
                        ApplyVolatileStatusInstruction {
                            side_ref: *side_ref,
                            slot_ref: *slot_ref,
                            volatile_status: PokemonVolatileStatus::PERISH2,
                        },
                    ));
            }
            if has_perish_4 {
                slot.volatile_statuses
                    .remove(&PokemonVolatileStatus::PERISH4);
                slot.volatile_statuses
                    .insert(PokemonVolatileStatus::PERISH3);
                incoming_instructions
                    .instruction_list
                    .push(Instruction::RemoveVolatileStatus(
                        RemoveVolatileStatusInstruction {
                            side_ref: *side_ref,
                            slot_ref: *slot_ref,
                            volatile_status: PokemonVolatileStatus::PERISH4,
                        },
                    ));
                incoming_instructions
                    .instruction_list
                    .push(Instruction::ApplyVolatileStatus(
                        ApplyVolatileStatusInstruction {
                            side_ref: *side_ref,
                            slot_ref: *slot_ref,
                            volatile_status: PokemonVolatileStatus::PERISH3,
                        },
                    ));
            }

            if has_flinch {
                slot.volatile_statuses
                    .remove(&PokemonVolatileStatus::FLINCH);
                incoming_instructions
                    .instruction_list
                    .push(Instruction::RemoveVolatileStatus(
                        RemoveVolatileStatusInstruction {
                            side_ref: *side_ref,
                            slot_ref: *slot_ref,
                            volatile_status: PokemonVolatileStatus::FLINCH,
                        },
                    ));
            }
            if has_roost {
                slot.volatile_statuses.remove(&PokemonVolatileStatus::ROOST);
                incoming_instructions
                    .instruction_list
                    .push(Instruction::RemoveVolatileStatus(
                        RemoveVolatileStatusInstruction {
                            side_ref: *side_ref,
                            slot_ref: *slot_ref,
                            volatile_status: PokemonVolatileStatus::ROOST,
                        },
                    ));
            }

            if has_partiallytrapped {
                let active_pkmn = side.get_active(slot_ref);

                let damage_amount =
                    cmp::min((active_pkmn.maxhp as f32 / 8.0) as i16, active_pkmn.hp);

                incoming_instructions
                    .instruction_list
                    .push(Instruction::Damage(DamageInstruction {
                        side_ref: *side_ref,
                        pokemon_index: active_index,
                        damage_amount,
                    }));
                active_pkmn.hp -= damage_amount;
            }
            if has_saltcure {
                let active_pkmn = side.get_active(slot_ref);
                let mut divisor = 8.0;
                if active_pkmn.has_type(&PokemonType::WATER)
                    || active_pkmn.has_type(&PokemonType::STEEL)
                {
                    divisor = 4.0;
                }
                let damage_amount =
                    cmp::min((active_pkmn.maxhp as f32 / divisor) as i16, active_pkmn.hp);
                incoming_instructions
                    .instruction_list
                    .push(Instruction::Damage(DamageInstruction {
                        side_ref: *side_ref,
                        pokemon_index: active_index,
                        damage_amount,
                    }));
                active_pkmn.hp -= damage_amount;
            }

            let possible_statuses = [
                PokemonVolatileStatus::PROTECT,
                PokemonVolatileStatus::BANEFULBUNKER,
                PokemonVolatileStatus::BURNINGBULWARK,
                PokemonVolatileStatus::SPIKYSHIELD,
                PokemonVolatileStatus::SILKTRAP,
                PokemonVolatileStatus::ENDURE,
            ];

            let slot = side.get_slot(slot_ref);
            let mut protect_vs = None;
            for status in &possible_statuses {
                if slot.volatile_statuses.contains(status) {
                    protect_vs = Some(*status);
                    break;
                }
            }

            if let Some(protect_vs) = protect_vs {
                incoming_instructions
                    .instruction_list
                    .push(Instruction::RemoveVolatileStatus(
                        RemoveVolatileStatusInstruction {
                            side_ref: *side_ref,
                            slot_ref: *slot_ref,
                            volatile_status: protect_vs,
                        },
                    ));
                slot.volatile_statuses.remove(&protect_vs);
                incoming_instructions.instruction_list.push(
                    Instruction::ChangeVolatileStatusDuration(
                        ChangeVolatileStatusDurationInstruction {
                            side_ref: *side_ref,
                            slot_ref: *slot_ref,
                            amount: 1,
                            volatile_status: PokemonVolatileStatus::PROTECT,
                        },
                    ),
                );
                slot.volatile_status_durations.protect += 1;
            } else if slot.volatile_status_durations.protect > 0 {
                incoming_instructions.instruction_list.push(
                    Instruction::ChangeVolatileStatusDuration(
                        ChangeVolatileStatusDurationInstruction {
                            side_ref: *side_ref,
                            slot_ref: *slot_ref,
                            amount: -1 * slot.volatile_status_durations.protect,
                            volatile_status: PokemonVolatileStatus::PROTECT,
                        },
                    ),
                );
                slot.volatile_status_durations.protect -= slot.volatile_status_durations.protect;
            }
        }
    } // end volatile statuses
}

fn execute_move_effects(
    state: &mut State,
    attacking_side: SideReference,
    attacking_slot: SlotReference,
    target_side: SideReference,
    target_slot: SlotReference,
    mut instructions: StateInstructions,
    hit_count: i8,
    does_damage: bool,
    damage_amount: i16,
    choice: &mut Choice,
    _defender_choice: &Choice,
    final_instructions: &mut Vec<(StateInstructions, Vec<RemainingToMove>)>,
    remaining_to_move: &Vec<RemainingToMove>,
) {
    let mut hit_sub = false;
    for _ in 0..hit_count {
        if does_damage {
            hit_sub = generate_instructions_from_damage(
                state,
                choice,
                damage_amount,
                &attacking_side,
                &attacking_slot,
                &target_side,
                &target_slot,
                &mut instructions,
            );
        }
        if let Some(side_condition) = &choice.side_condition {
            generate_instructions_from_side_conditions(
                state,
                side_condition,
                &attacking_side,
                &mut instructions,
            );
        }
        choice_hazard_clear(
            state,
            &choice,
            &attacking_side,
            &target_side,
            &target_slot,
            &mut instructions,
        );
        if let Some(volatile_status) = &choice.volatile_status {
            get_instructions_from_volatile_statuses(
                state,
                &choice,
                volatile_status,
                &attacking_side,
                &attacking_slot,
                &target_side,
                &target_slot,
                &mut instructions,
            );
        }
        if let Some(status) = &choice.status {
            get_instructions_from_status_effects(
                state,
                status,
                &attacking_side,
                &attacking_slot,
                &target_side,
                &target_slot,
                &mut instructions,
                hit_sub,
            );
        }
        if let Some(heal) = &choice.heal {
            get_instructions_from_heal(
                state,
                heal,
                &attacking_side,
                &attacking_slot,
                &target_side,
                &target_slot,
                &mut instructions,
            );
        }
    } // end multi-hit
      // this is wrong, but I am deciding it is good enough for this engine (for now)
      // each multi-hit move should trigger a chance for a secondary effect,
      // but the way this engine was structured makes it difficult to implement
      // without some performance hits.

    if let Some(boost) = &choice.boost {
        get_instructions_from_boosts(
            state,
            boost,
            &attacking_side,
            &attacking_slot,
            &target_side,
            &target_slot,
            &mut instructions,
        );
    }

    if choice.flags.drag
        && state
            .get_side_immutable(&target_side)
            .get_active_immutable(&target_slot)
            .ability
            != Abilities::GUARDDOG
    {
        get_instructions_from_drag(
            state,
            &target_side,
            &target_slot,
            instructions,
            final_instructions,
            remaining_to_move,
        );
        return;
    }

    if choice.flags.pivot {
        get_instructions_from_pivot(
            state,
            &choice,
            &attacking_side,
            &attacking_slot,
            &mut instructions,
            remaining_to_move,
        );
    }

    if state
        .get_side_immutable(&target_side)
        .get_active_immutable(&target_slot)
        .item
        == Items::COVERTCLOAK
    {
        state.reverse_instructions(&instructions.instruction_list);
        final_instructions.push((instructions, remaining_to_move.clone()));
    } else if let Some(secondaries_vec) = &choice.secondaries {
        state.reverse_instructions(&instructions.instruction_list);
        for i in get_instructions_from_secondaries(
            state,
            &choice,
            secondaries_vec,
            &attacking_side,
            &attacking_slot,
            &target_side,
            &target_slot,
            instructions,
            hit_sub,
        ) {
            final_instructions.push((i, remaining_to_move.clone()));
        }
    } else {
        state.reverse_instructions(&instructions.instruction_list);
        final_instructions.push((instructions, remaining_to_move.clone()));
    }
}

#[derive(Clone)]
pub struct RemainingToMove {
    side_ref: SideReference,
    slot_ref: SlotReference,
    move_choice: MoveChoice,
    choice: Choice,
}

pub fn generate_instructions_from_move_pair(
    state: &mut State,
    side_one_a_move: &MoveChoice,
    side_one_b_move: &MoveChoice,
    side_two_a_move: &MoveChoice,
    side_two_b_move: &MoveChoice,
    branch_on_damage: bool,
) -> Vec<StateInstructions> {
    let mut side_one_a_target_side: SideReference = SideReference::SideOne;
    let mut side_one_b_target_side: SideReference = SideReference::SideOne;
    let mut side_two_a_target_side: SideReference = SideReference::SideOne;
    let mut side_two_b_target_side: SideReference = SideReference::SideOne;
    let mut side_one_a_target_slot: SlotReference = SlotReference::SlotA;
    let mut side_one_b_target_slot: SlotReference = SlotReference::SlotA;
    let mut side_two_a_target_slot: SlotReference = SlotReference::SlotA;
    let mut side_two_b_target_slot: SlotReference = SlotReference::SlotA;

    let mut side_one_a_choice;
    let mut side_one_b_choice;
    let mut s1_a_tera = false;
    let mut s1_a_replacing_fainted_pkmn = false;
    let mut s1_b_tera = false;
    let mut s1_b_replacing_fainted_pkmn = false;
    match side_one_a_move {
        MoveChoice::Switch(switch_id) => {
            if state.side_one.pokemon[state.side_one.slot_a.active_index].hp == 0 {
                s1_a_replacing_fainted_pkmn = true;
            }
            side_one_a_choice = Choice::default();
            side_one_a_choice.switch_id = *switch_id;
            side_one_a_choice.category = MoveCategory::Switch;
        }
        MoveChoice::Move(target_slot, target_side, move_index) => {
            side_one_a_choice = state.side_one.get_active(&SlotReference::SlotA).moves[move_index]
                .choice
                .clone();
            side_one_a_choice.move_index = *move_index;
            side_one_a_target_side = *target_side;
            side_one_a_target_slot = *target_slot;
        }
        MoveChoice::MoveTera(target_slot, target_side, move_index) => {
            side_one_a_choice = state.side_one.get_active(&SlotReference::SlotA).moves[move_index]
                .choice
                .clone();
            side_one_a_choice.move_index = *move_index;
            side_one_a_target_side = *target_side;
            side_one_a_target_slot = *target_slot;
            s1_a_tera = true;
        }
        MoveChoice::None => {
            side_one_a_choice = Choice::default();
        }
    }
    match side_one_b_move {
        MoveChoice::Switch(switch_id) => {
            if state.side_one.pokemon[state.side_one.slot_b.active_index].hp == 0 {
                s1_b_replacing_fainted_pkmn = true;
            }
            side_one_b_choice = Choice::default();
            side_one_b_choice.switch_id = *switch_id;
            side_one_b_choice.category = MoveCategory::Switch;
        }
        MoveChoice::Move(target_slot, target_side, move_index) => {
            side_one_b_choice = state.side_one.get_active(&SlotReference::SlotB).moves[move_index]
                .choice
                .clone();
            side_one_b_choice.move_index = *move_index;
            side_one_b_target_side = *target_side;
            side_one_b_target_slot = *target_slot;
        }
        MoveChoice::MoveTera(target_slot, target_side, move_index) => {
            side_one_b_choice = state.side_one.get_active(&SlotReference::SlotB).moves[move_index]
                .choice
                .clone();
            side_one_b_choice.move_index = *move_index;
            side_one_b_target_side = *target_side;
            side_one_b_target_slot = *target_slot;
            s1_b_tera = true;
        }
        MoveChoice::None => {
            side_one_b_choice = Choice::default();
        }
    }

    let mut side_two_a_choice;
    let mut side_two_b_choice;
    let mut s2_a_tera = false;
    let mut s2_a_replacing_fainted_pkmn = false;
    let mut s2_b_tera = false;
    let mut s2_b_replacing_fainted_pkmn = false;
    match side_two_a_move {
        MoveChoice::Switch(switch_id) => {
            if state.side_two.pokemon[state.side_two.slot_a.active_index].hp == 0 {
                s2_a_replacing_fainted_pkmn = true;
            }
            side_two_a_choice = Choice::default();
            side_two_a_choice.switch_id = *switch_id;
            side_two_a_choice.category = MoveCategory::Switch;
        }
        MoveChoice::Move(target_slot, target_side, move_index) => {
            side_two_a_choice = state.side_two.get_active(&SlotReference::SlotA).moves[move_index]
                .choice
                .clone();
            side_two_a_choice.move_index = *move_index;
            side_two_a_target_side = *target_side;
            side_two_a_target_slot = *target_slot;
        }
        MoveChoice::MoveTera(target_slot, target_side, move_index) => {
            side_two_a_choice = state.side_two.get_active(&SlotReference::SlotA).moves[move_index]
                .choice
                .clone();
            side_two_a_choice.move_index = *move_index;
            side_two_a_target_side = *target_side;
            side_two_a_target_slot = *target_slot;
            s2_a_tera = true;
        }
        MoveChoice::None => {
            side_two_a_choice = Choice::default();
        }
    }
    match side_two_b_move {
        MoveChoice::Switch(switch_id) => {
            if state.side_two.pokemon[state.side_two.slot_b.active_index].hp == 0 {
                s2_b_replacing_fainted_pkmn = true;
            }
            side_two_b_choice = Choice::default();
            side_two_b_choice.switch_id = *switch_id;
            side_two_b_choice.category = MoveCategory::Switch;
        }
        MoveChoice::Move(target_slot, target_side, move_index) => {
            side_two_b_choice = state.side_two.get_active(&SlotReference::SlotB).moves[move_index]
                .choice
                .clone();
            side_two_b_choice.move_index = *move_index;
            side_two_b_target_side = *target_side;
            side_two_b_target_slot = *target_slot;
        }
        MoveChoice::MoveTera(target_slot, target_side, move_index) => {
            side_two_b_choice = state.side_two.get_active(&SlotReference::SlotB).moves[move_index]
                .choice
                .clone();
            side_two_b_choice.move_index = *move_index;
            side_two_b_target_side = *target_side;
            side_two_b_target_slot = *target_slot;
            s2_b_tera = true;
        }
        MoveChoice::None => {
            side_two_b_choice = Choice::default();
        }
    }

    let mut state_instructions_vec: Vec<(StateInstructions, Vec<RemainingToMove>)> =
        Vec::with_capacity(8);
    let mut incoming_instructions: StateInstructions = StateInstructions::default();

    // Run terstallization type changes
    // Note: only create/apply instructions, don't apply changes
    // generate_instructions_from_move() assumes instructions have not been applied
    if s1_a_tera {
        generate_instructions_from_tera(
            state,
            &SideReference::SideOne,
            &SlotReference::SlotA,
            state.side_one.slot_a.active_index,
            &mut incoming_instructions,
        );
    }
    if s1_b_tera {
        generate_instructions_from_tera(
            state,
            &SideReference::SideOne,
            &SlotReference::SlotB,
            state.side_one.slot_b.active_index,
            &mut incoming_instructions,
        );
    }
    if s2_a_tera {
        generate_instructions_from_tera(
            state,
            &SideReference::SideTwo,
            &SlotReference::SlotA,
            state.side_two.slot_a.active_index,
            &mut incoming_instructions,
        );
    }
    if s2_b_tera {
        generate_instructions_from_tera(
            state,
            &SideReference::SideTwo,
            &SlotReference::SlotB,
            state.side_two.slot_b.active_index,
            &mut incoming_instructions,
        );
    }

    modify_choice_before_move(
        &state,
        &SideReference::SideOne,
        &SlotReference::SlotA,
        &mut side_one_a_choice,
        s1_a_tera,
    );
    modify_choice_before_move(
        &state,
        &SideReference::SideOne,
        &SlotReference::SlotB,
        &mut side_one_b_choice,
        s1_b_tera,
    );
    modify_choice_before_move(
        &state,
        &SideReference::SideTwo,
        &SlotReference::SlotA,
        &mut side_two_a_choice,
        s2_a_tera,
    );
    modify_choice_before_move(
        &state,
        &SideReference::SideTwo,
        &SlotReference::SlotB,
        &mut side_two_b_choice,
        s2_b_tera,
    );

    let need_to_move = vec![
        RemainingToMove {
            side_ref: SideReference::SideOne,
            slot_ref: SlotReference::SlotA,
            move_choice: side_one_a_move.clone(),
            choice: side_one_a_choice.clone(),
        },
        RemainingToMove {
            side_ref: SideReference::SideOne,
            slot_ref: SlotReference::SlotB,
            move_choice: side_one_b_move.clone(),
            choice: side_one_b_choice.clone(),
        },
        RemainingToMove {
            side_ref: SideReference::SideTwo,
            slot_ref: SlotReference::SlotA,
            move_choice: side_two_a_move.clone(),
            choice: side_two_a_choice.clone(),
        },
        RemainingToMove {
            side_ref: SideReference::SideTwo,
            slot_ref: SlotReference::SlotB,
            move_choice: side_two_b_move.clone(),
            choice: side_two_b_choice.clone(),
        },
    ];

    state_instructions_vec.push((incoming_instructions, need_to_move));

    let mut num_moves_done = 0;
    while num_moves_done < 4 {
        let mut i = 0;
        let vec_len = state_instructions_vec.len();
        while i < vec_len {
            let (state_instruction, mut remaining_to_move) = state_instructions_vec.remove(0);
            state.apply_instructions(&state_instruction.instruction_list);
            let (
                mut attacker_choice,
                attacker_side_ref,
                attacker_slot_ref,
                target_side_ref,
                target_slot_ref,
            ) = match next_to_move(&state, &remaining_to_move) {
                (
                    SideReference::SideOne,
                    SlotReference::SlotA,
                    chosen_index,
                    move_effective_priority,
                ) => {
                    remaining_to_move.remove(chosen_index);
                    let mut this_choice = side_one_a_choice.clone();
                    this_choice.priority = move_effective_priority;
                    (
                        this_choice,
                        SideReference::SideOne,
                        SlotReference::SlotA,
                        side_one_a_target_side,
                        side_one_a_target_slot,
                    )
                }
                (
                    SideReference::SideOne,
                    SlotReference::SlotB,
                    chosen_index,
                    move_effective_priority,
                ) => {
                    remaining_to_move.remove(chosen_index);
                    let mut this_choice = side_one_b_choice.clone();
                    this_choice.priority = move_effective_priority;
                    (
                        this_choice,
                        SideReference::SideOne,
                        SlotReference::SlotB,
                        side_one_b_target_side,
                        side_one_b_target_slot,
                    )
                }
                (
                    SideReference::SideTwo,
                    SlotReference::SlotA,
                    chosen_index,
                    move_effective_priority,
                ) => {
                    remaining_to_move.remove(chosen_index);
                    let mut this_choice = side_two_a_choice.clone();
                    this_choice.priority = move_effective_priority;
                    (
                        this_choice,
                        SideReference::SideTwo,
                        SlotReference::SlotA,
                        side_two_a_target_side,
                        side_two_a_target_slot,
                    )
                }
                (
                    SideReference::SideTwo,
                    SlotReference::SlotB,
                    chosen_index,
                    move_effective_priority,
                ) => {
                    remaining_to_move.remove(chosen_index);
                    let mut this_choice = side_two_b_choice.clone();
                    this_choice.priority = move_effective_priority;
                    (
                        this_choice,
                        SideReference::SideTwo,
                        SlotReference::SlotB,
                        side_two_b_target_side,
                        side_two_b_target_slot,
                    )
                }
            };
            let target_choice = match (target_side_ref, target_slot_ref) {
                (SideReference::SideOne, SlotReference::SlotA) => &side_one_a_choice,
                (SideReference::SideOne, SlotReference::SlotB) => &side_one_b_choice,
                (SideReference::SideTwo, SlotReference::SlotA) => &side_two_a_choice,
                (SideReference::SideTwo, SlotReference::SlotB) => &side_two_b_choice,
            };
            generate_instructions_from_move(
                state,
                &mut attacker_choice,
                target_choice,
                attacker_side_ref,
                attacker_slot_ref,
                target_side_ref,
                target_slot_ref,
                state_instruction,
                &mut state_instructions_vec,
                remaining_to_move,
                branch_on_damage,
            );
            i += 1;
        }
        combine_duplicate_instructions(&mut state_instructions_vec);
        remove_low_chance_instructions(&mut state_instructions_vec, 4.0);
        num_moves_done += 1;
    }

    for (state_instruction, _) in state_instructions_vec.iter_mut() {
        state.apply_instructions(&state_instruction.instruction_list);
        if !(s1_a_replacing_fainted_pkmn
            || s1_b_replacing_fainted_pkmn
            || s2_a_replacing_fainted_pkmn
            || s2_b_replacing_fainted_pkmn
            || state.side_one.slot_a.force_switch
            || state.side_one.slot_b.force_switch
            || state.side_two.slot_a.force_switch
            || state.side_two.slot_b.force_switch)
        {
            state_instruction.end_of_turn_triggered = true;
            add_end_of_turn_instructions(state, state_instruction, &SideReference::SideOne);
        }
        state.reverse_instructions(&state_instruction.instruction_list);
    }

    state_instructions_vec
        .into_iter()
        .map(|(state_instr, _)| state_instr)
        .collect()
}

/*
The return value of this function is the maximum for both
the regular damage roll and the critical hit damage roll.

If the move does static damage (Seismic Toss, Night Shade, etc.),
there will be only one element in the vector.
*/
pub fn calculate_damage_rolls(
    state: &mut State,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
    target_side_ref: &SideReference,
    target_slot_ref: &SlotReference,
    mut choice: Choice,
    mut defending_choice: &Choice,
) -> Option<Vec<i16>> {
    let mut incoming_instructions = StateInstructions::default();

    if choice.flags.charge {
        choice.flags.charge = false;
    }
    if choice.move_id == Choices::FAKEOUT || choice.move_id == Choices::FIRSTIMPRESSION {
        state
            .get_side(attacking_side_ref)
            .get_slot(attacking_slot_ref)
            .last_used_move = LastUsedMove::Switch(PokemonIndex::P0);
    }

    ability_change_type(state, &mut choice, &attacking_side_ref, &attacking_slot_ref);
    item_change_type(state, &mut choice, &attacking_side_ref, &attacking_slot_ref);
    choice_change_type(state, &mut choice, &attacking_side_ref, &attacking_slot_ref);

    let attacker_active = state
        .get_side_immutable(attacking_side_ref)
        .get_active_immutable(attacking_slot_ref);
    let defender_active = state
        .get_side_immutable(&target_side_ref)
        .get_active_immutable(target_slot_ref);
    match choice.move_id {
        Choices::SEISMICTOSS => {
            if type_effectiveness_modifier(&PokemonType::NORMAL, &defender_active) == 0.0 {
                return None;
            }
            return Some(vec![attacker_active.level as i16]);
        }
        Choices::NIGHTSHADE => {
            if type_effectiveness_modifier(&PokemonType::GHOST, &defender_active) == 0.0 {
                return None;
            }
            return Some(vec![attacker_active.level as i16]);
        }
        Choices::FINALGAMBIT => {
            if type_effectiveness_modifier(&PokemonType::GHOST, &defender_active) == 0.0 {
                return None;
            }
            return Some(vec![attacker_active.hp]);
        }
        Choices::ENDEAVOR => {
            if type_effectiveness_modifier(&PokemonType::GHOST, &defender_active) == 0.0
                || defender_active.hp <= attacker_active.hp
            {
                return None;
            }
            return Some(vec![defender_active.hp - attacker_active.hp]);
        }
        Choices::PAINSPLIT => {
            if type_effectiveness_modifier(&PokemonType::GHOST, &defender_active) == 0.0
                || defender_active.hp <= attacker_active.hp
            {
                return None;
            }
            return Some(vec![
                defender_active.hp - (attacker_active.hp + defender_active.hp) / 2,
            ]);
        }
        Choices::SUPERFANG
            if type_effectiveness_modifier(&PokemonType::NORMAL, &defender_active) == 0.0 =>
        {
            return None;
        }
        Choices::SUPERFANG | Choices::NATURESMADNESS | Choices::RUINATION => {
            return Some(vec![defender_active.hp / 2]);
        }
        Choices::SUCKERPUNCH | Choices::THUNDERCLAP => {
            defending_choice = MOVES.get(&Choices::TACKLE).unwrap();
        }

        _ => {}
    }

    before_move(
        state,
        &mut choice,
        defending_choice,
        attacking_side_ref,
        attacking_slot_ref,
        &target_side_ref,
        target_slot_ref,
        true,
        false,
        &mut incoming_instructions,
    );

    // Spread move 0.75x damage if there are two targets
    if choice.move_choice_target == MoveChoiceTarget::AllFoes {
        let target_side = if *attacking_side_ref == SideReference::SideOne {
            &state.side_two
        } else {
            &state.side_one
        };
        if target_side.get_active_immutable(&SlotReference::SlotA).hp > 0
            && target_side.get_active_immutable(&SlotReference::SlotB).hp > 0
        {
            choice.base_power *= 0.75;
        }
    }

    if choice.move_id == Choices::FUTURESIGHT {
        choice = MOVES.get(&Choices::FUTURESIGHT)?.clone();
    }

    let mut return_vec = Vec::with_capacity(4);
    if let Some((damage, crit_damage)) = calculate_damage(
        &state,
        attacking_side_ref,
        attacking_slot_ref,
        &target_side_ref,
        target_slot_ref,
        &choice,
        DamageRolls::Max,
    ) {
        return_vec.push(damage);
        return_vec.push(crit_damage);
        Some(return_vec)
    } else {
        None
    }
}

pub fn calculate_both_damage_rolls(
    _state: &State,
    _s1_a_choice: Choice,
    _s2_a_choice: Choice,
    _side_one_moves_first: bool,
) -> (Option<Vec<i16>>, Option<Vec<i16>>) {
    return (Some(vec![0]), Some(vec![0]));
    // if side_one_moves_first {
    //     s1_choice.first_move = true;
    //     s2_choice.first_move = false;
    // } else {
    //     s1_choice.first_move = false;
    //     s2_choice.first_move = true;
    // }

    // let damages_dealt_s1_a = calculate_damage_rolls(
    //     state.clone(),
    //     &SideReference::SideOne,
    //     s1_choice.clone(),
    //     &s2_choice,
    // );
    // let damages_dealt_s2 = calculate_damage_rolls(
    //     state.clone(),
    //     &SideReference::SideTwo,
    //     s2_choice,
    //     &s1_choice,
    // );

    // (damages_dealt_s1, damages_dealt_s2)
}
