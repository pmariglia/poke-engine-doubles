use super::abilities::Abilities;
use super::damage_calc::type_effectiveness_modifier;
use super::generate_instructions::{apply_boost_instructions, immune_to_status};
use super::state::Terrain;
use crate::choices::{Choice, Choices, Effect, MoveCategory, MoveTarget, Secondary, StatBoosts};
use crate::define_enum_with_from_str;
use crate::engine::generate_instructions::add_remove_status_instructions;
use crate::instruction::{
    ChangeItemInstruction, ChangeStatusInstruction, DamageInstruction, DisableMoveInstruction,
    HealInstruction, Instruction, StateInstructions,
};
use crate::pokemon::PokemonName;
use crate::state::{
    Pokemon, PokemonBoostableStat, PokemonIndex, PokemonStatus, PokemonType, Side, SideReference,
    SlotReference, State,
};
use std::cmp;

define_enum_with_from_str! {
    #[repr(u8)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    Items {
        NONE,
        UNKNOWNITEM,
        ABILITYSHIELD,
        ABSORBBULB,
        ADRENALINEORB,
        ADAMANTORB,
        ADAMANTCRYSTAL,
        AIRBALLOON,
        ASSAULTVEST,
        BABIRIBERRY,
        BLACKBELT,
        BLACKSLUDGE,
        BLACKGLASSES,
        BLANKPLATE,
        BOOSTERENERGY,
        CELLBATTERY,
        CHARCOAL,
        CHARTIBERRY,
        CHILANBERRY,
        CHOICEBAND,
        CHOICESPECS,
        CHOICESCARF,
        CHOPLEBERRY,
        COBABERRY,
        COLBURBERRY,
        COVERTCLOAK,
        CUSTAPBERRY,
        DRAGONFANG,
        DRAGONSCALE,
        DREADPLATE,
        EARTHPLATE,
        ELECTRICSEED,
        EXPERTBELT,
        EVIOLITE,
        FAIRYFEATHER,
        FISTPLATE,
        FLAMEORB,
        GRASSYSEED,
        HABANBERRY,
        KASIBBERRY,
        KEBIABERRY,
        LEFTOVERS,
        LIFEORB,
        LUSTROUSORB,
        LUSTROUSGLOBE,
        METALCOAT,
        MISTYSEED,
        MUSCLEBAND,
        MYSTICWATER,
        NEVERMELTICE,
        PINKBOW,
        POLKADOTBOW,
        OCCABERRY,
        ODDINCENSE,
        PASSHOBERRY,
        PAYAPABERRY,
        POISONBARB,
        POWERHERB,
        PSYCHICSEED,
        PUNCHINGGLOVE,
        RINDOBERRY,
        ROSELIBERRY,
        ROCKYHELMET,
        SEAINCENSE,
        SHARPBEAK,
        SPELLTAG,
        MIRACLESEED,
        SAFETYGOGGLES,
        SHELLBELL,
        SHUCABERRY,
        SILKSCARF,
        SILVERPOWDER,
        SKYPLATE,
        SOFTSAND,
        SOULDEW,
        GRISEOUSORB,
        GRISEOUSCORE,
        TANGABERRY,
        THROATSPRAY,
        THICKCLUB,
        TOXICORB,
        TOXICPLATE,
        TWISTEDSPOON,
        HARDSTONE,
        METALPOWDER,
        WACANBERRY,
        WAVEINCENSE,
        MAGNET,
        WEAKNESSPOLICY,
        WISEGLASSES,
        BLUNDERPOLICY,
        HEAVYDUTYBOOTS,
        CLEARAMULET,
        PROTECTIVEPADS,
        SHEDSHELL,
        YACHEBERRY,
        STONEPLATE,
        INSECTPLATE,
        SPOOKYPLATE,
        IRONBALL,
        IRONPLATE,
        FLAMEPLATE,
        SPLASHPLATE,
        MEADOWPLATE,
        ZAPPLATE,
        MINDPLATE,
        ICICLEPLATE,
        DRACOPLATE,
        PIXIEPLATE,
        LIGHTBALL,
        LIGHTCLAY,
        FOCUSSASH,
        CHESTOBERRY,
        LUMBERRY,
        SITRUSBERRY,
        PETAYABERRY,
        SALACBERRY,
        LIECHIBERRY,
        NORMALGEM,
        BUGGEM,
        ELECTRICGEM,
        FIGHTINGGEM,
        GHOSTGEM,
        PSYCHICGEM,
        FLYINGGEM,
        STEELGEM,
        ICEGEM,
        POISONGEM,
        FIREGEM,
        DRAGONGEM,
        GROUNDGEM,
        WATERGEM,
        DARKGEM,
        ROCKGEM,
        GRASSGEM,
        FAIRYGEM,
        BUGMEMORY,
        FIGHTINGMEMORY,
        GHOSTMEMORY,
        PSYCHICMEMORY,
        FLYINGMEMORY,
        STEELMEMORY,
        ICEMEMORY,
        POISONMEMORY,
        FIREMEMORY,
        DRAGONMEMORY,
        GROUNDMEMORY,
        WATERMEMORY,
        DARKMEMORY,
        ROCKMEMORY,
        GRASSMEMORY,
        FAIRYMEMORY,
        ELECTRICMEMORY,
        WELLSPRINGMASK,
        HEARTHFLAMEMASK,
        CORNERSTONEMASK,
        WIDELENS,
        LOADEDDICE,
        RUSTEDSWORD,
        RUSTEDSHIELD,
        SCOPELENS,
    },
    default = UNKNOWNITEM
}

pub fn get_choice_move_disable_instructions(
    pkmn: &Pokemon,
    pkmn_index: PokemonIndex,
    side_ref: &SideReference,
    move_name: &Choices,
) -> Vec<Instruction> {
    let mut moves_to_disable = vec![];
    let mut iter = pkmn.moves.into_iter();
    while let Some(p) = iter.next() {
        if &p.id != move_name && p.disabled == false {
            moves_to_disable.push(Instruction::DisableMove(DisableMoveInstruction {
                side_ref: *side_ref,
                pokemon_index: pkmn_index,
                move_index: iter.pokemon_move_index,
            }));
        }
    }
    moves_to_disable
}

fn damage_reduction_berry(
    defending_pkmn: &mut Pokemon,
    target_side_ref: &SideReference,
    target_active_index: PokemonIndex,
    choice: &mut Choice,
    berry: Items,
    pkmn_type: &PokemonType,
    instructions: &mut StateInstructions,
) {
    if &choice.move_type == pkmn_type
        && type_effectiveness_modifier(pkmn_type, &defending_pkmn) > 1.0
    {
        instructions
            .instruction_list
            .push(Instruction::ChangeItem(ChangeItemInstruction {
                side_ref: *target_side_ref,
                pokemon_index: target_active_index,
                current_item: berry,
                new_item: Items::NONE,
            }));
        defending_pkmn.item = Items::NONE;
        choice.base_power /= 2.0;
    }
}

/*
NormalGem, FlyingGem, etc.
*/
fn power_up_gem(
    attacking_side_ref: &SideReference,
    attacking_pkmn: &mut Pokemon,
    active_index: PokemonIndex,
    choice: &mut Choice,
    gem_type: PokemonType,
    instructions: &mut StateInstructions,
) {
    if &choice.move_type == &gem_type {
        choice.base_power *= 1.3;
        instructions
            .instruction_list
            .push(Instruction::ChangeItem(ChangeItemInstruction {
                side_ref: *attacking_side_ref,
                pokemon_index: active_index,
                current_item: attacking_pkmn.item,
                new_item: Items::NONE,
            }));
        attacking_pkmn.item = Items::NONE;
    }
}

/*
Regarding berries:
    most berries (lum, sitrus, etc) activate right away when applicable, but there isn't
    logic in this engine to implement that. Attempting to activate these berries before the user's
    move AND at the end-of-turn should be accurate enough for a simulation. The item is
    removed after this is triggered so only one will take effect
*/
fn lum_berry(
    active_pkmn: &mut Pokemon,
    active_index: PokemonIndex,
    side_ref: &SideReference,
    instructions: &mut StateInstructions,
) {
    instructions
        .instruction_list
        .push(Instruction::ChangeStatus(ChangeStatusInstruction {
            side_ref: *side_ref,
            pokemon_index: active_index,
            new_status: PokemonStatus::NONE,
            old_status: active_pkmn.status,
        }));
    active_pkmn.status = PokemonStatus::NONE;
    instructions
        .instruction_list
        .push(Instruction::ChangeItem(ChangeItemInstruction {
            side_ref: *side_ref,
            pokemon_index: active_index,
            current_item: Items::LUMBERRY,
            new_item: Items::NONE,
        }));
    active_pkmn.item = Items::NONE;
}

fn sitrus_berry(
    active_pkmn: &mut Pokemon,
    active_index: PokemonIndex,
    side_ref: &SideReference,
    instructions: &mut StateInstructions,
) {
    let heal_amount = cmp::min(active_pkmn.maxhp / 4, active_pkmn.maxhp - active_pkmn.hp);
    instructions
        .instruction_list
        .push(Instruction::Heal(HealInstruction {
            side_ref: *side_ref,
            pokemon_index: active_index,
            heal_amount: heal_amount,
        }));
    active_pkmn.hp += heal_amount;
    instructions
        .instruction_list
        .push(Instruction::ChangeItem(ChangeItemInstruction {
            side_ref: *side_ref,
            pokemon_index: active_index,
            current_item: Items::SITRUSBERRY,
            new_item: Items::NONE,
        }));
    active_pkmn.item = Items::NONE;
}

fn chesto_berry(
    attacking_side: &mut Side,
    active_index: PokemonIndex,
    side_ref: &SideReference,
    instructions: &mut StateInstructions,
) {
    instructions
        .instruction_list
        .push(Instruction::ChangeItem(ChangeItemInstruction {
            side_ref: *side_ref,
            pokemon_index: active_index,
            current_item: Items::CHESTOBERRY,
            new_item: Items::NONE,
        }));
    attacking_side.pokemon[&active_index].item = Items::NONE;
    add_remove_status_instructions(instructions, active_index, *side_ref, attacking_side);
}

fn boost_berry(
    state: &mut State,
    side_ref: &SideReference,
    slot_ref: &SlotReference,
    stat: PokemonBoostableStat,
    instructions: &mut StateInstructions,
) {
    apply_boost_instructions(
        state.get_side(side_ref),
        &stat,
        &1,
        side_ref,
        side_ref,
        slot_ref,
        instructions,
    );
    let attacking_side = state.get_side(side_ref);
    let attacking_index = attacking_side.get_slot_immutable(slot_ref).active_index;
    let attacker = attacking_side.get_active(slot_ref);
    instructions
        .instruction_list
        .push(Instruction::ChangeItem(ChangeItemInstruction {
            side_ref: *side_ref,
            pokemon_index: attacking_index,
            current_item: attacker.item,
            new_item: Items::NONE,
        }));
    attacker.item = Items::NONE;
}

pub fn item_change_type(
    state: &State,
    attacking_choice: &mut Choice,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
) {
    let attacking_side = state.get_side_immutable(attacking_side_ref);
    let attacking_pkmn = attacking_side.get_active_immutable(attacking_slot_ref);
    match attacking_pkmn.item {
        Items::FISTPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::FIGHTING;
            }
        }
        Items::SKYPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::FLYING;
            }
        }
        Items::TOXICPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::POISON;
            }
        }
        Items::EARTHPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::GROUND;
            }
        }
        Items::STONEPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::ROCK;
            }
        }
        Items::INSECTPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::BUG;
            }
        }
        Items::SPOOKYPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::GHOST;
            }
        }
        Items::IRONPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::STEEL;
            }
        }
        Items::FLAMEPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::FIRE;
            }
        }
        Items::SPLASHPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::WATER;
            }
        }
        Items::MEADOWPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::GRASS;
            }
        }
        Items::ZAPPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::ELECTRIC;
            }
        }
        Items::MINDPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::PSYCHIC;
            }
        }
        Items::ICICLEPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::ICE;
            }
        }
        Items::DRACOPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::DRAGON;
            }
        }
        Items::DREADPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::DARK;
            }
        }
        Items::PIXIEPLATE => {
            if attacking_choice.move_id == Choices::JUDGMENT {
                attacking_choice.move_type = PokemonType::FAIRY;
            }
        }
        _ => {}
    }
}

pub fn item_before_move(
    state: &mut State,
    choice: &mut Choice,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
    target_side_ref: &SideReference,
    target_slot_ref: &SlotReference,
    instructions: &mut StateInstructions,
) {
    let target_side = state.get_side(target_side_ref);
    let target_active_index = target_side.get_slot_immutable(target_slot_ref).active_index;
    let target_pkmn = target_side.get_active(target_slot_ref);
    match target_pkmn.item {
        Items::CHOPLEBERRY => damage_reduction_berry(
            target_pkmn,
            target_side_ref,
            target_active_index,
            choice,
            Items::CHOPLEBERRY,
            &PokemonType::FIGHTING,
            instructions,
        ),
        Items::BABIRIBERRY => damage_reduction_berry(
            target_pkmn,
            attacking_side_ref,
            target_active_index,
            choice,
            Items::BABIRIBERRY,
            &PokemonType::STEEL,
            instructions,
        ),
        Items::CHARTIBERRY => damage_reduction_berry(
            target_pkmn,
            attacking_side_ref,
            target_active_index,
            choice,
            Items::CHARTIBERRY,
            &PokemonType::ROCK,
            instructions,
        ),
        Items::CHILANBERRY => {
            // no type effectiveness check for chilan
            if &choice.move_type == &PokemonType::NORMAL {
                instructions.instruction_list.push(Instruction::ChangeItem(
                    ChangeItemInstruction {
                        side_ref: *target_side_ref,
                        pokemon_index: target_active_index,
                        current_item: Items::CHILANBERRY,
                        new_item: Items::NONE,
                    },
                ));
                target_pkmn.item = Items::NONE;
                choice.base_power /= 2.0;
            }
        }
        Items::COBABERRY => damage_reduction_berry(
            target_pkmn,
            attacking_side_ref,
            target_active_index,
            choice,
            Items::COBABERRY,
            &PokemonType::FLYING,
            instructions,
        ),
        Items::COLBURBERRY => damage_reduction_berry(
            target_pkmn,
            attacking_side_ref,
            target_active_index,
            choice,
            Items::COLBURBERRY,
            &PokemonType::DARK,
            instructions,
        ),
        Items::HABANBERRY => damage_reduction_berry(
            target_pkmn,
            attacking_side_ref,
            target_active_index,
            choice,
            Items::HABANBERRY,
            &PokemonType::DRAGON,
            instructions,
        ),
        Items::KASIBBERRY => damage_reduction_berry(
            target_pkmn,
            attacking_side_ref,
            target_active_index,
            choice,
            Items::KASIBBERRY,
            &PokemonType::GHOST,
            instructions,
        ),
        Items::KEBIABERRY => damage_reduction_berry(
            target_pkmn,
            attacking_side_ref,
            target_active_index,
            choice,
            Items::KEBIABERRY,
            &PokemonType::POISON,
            instructions,
        ),
        Items::OCCABERRY => damage_reduction_berry(
            target_pkmn,
            attacking_side_ref,
            target_active_index,
            choice,
            Items::OCCABERRY,
            &PokemonType::FIRE,
            instructions,
        ),
        Items::PASSHOBERRY => damage_reduction_berry(
            target_pkmn,
            attacking_side_ref,
            target_active_index,
            choice,
            Items::PASSHOBERRY,
            &PokemonType::WATER,
            instructions,
        ),
        Items::PAYAPABERRY => damage_reduction_berry(
            target_pkmn,
            attacking_side_ref,
            target_active_index,
            choice,
            Items::PAYAPABERRY,
            &PokemonType::PSYCHIC,
            instructions,
        ),
        Items::RINDOBERRY => damage_reduction_berry(
            target_pkmn,
            attacking_side_ref,
            target_active_index,
            choice,
            Items::RINDOBERRY,
            &PokemonType::GRASS,
            instructions,
        ),
        Items::ROSELIBERRY => damage_reduction_berry(
            target_pkmn,
            attacking_side_ref,
            target_active_index,
            choice,
            Items::ROSELIBERRY,
            &PokemonType::FAIRY,
            instructions,
        ),
        Items::SHUCABERRY => damage_reduction_berry(
            target_pkmn,
            attacking_side_ref,
            target_active_index,
            choice,
            Items::SHUCABERRY,
            &PokemonType::GROUND,
            instructions,
        ),
        Items::TANGABERRY => damage_reduction_berry(
            target_pkmn,
            attacking_side_ref,
            target_active_index,
            choice,
            Items::TANGABERRY,
            &PokemonType::BUG,
            instructions,
        ),
        Items::WACANBERRY => damage_reduction_berry(
            target_pkmn,
            attacking_side_ref,
            target_active_index,
            choice,
            Items::WACANBERRY,
            &PokemonType::ELECTRIC,
            instructions,
        ),
        Items::YACHEBERRY => damage_reduction_berry(
            target_pkmn,
            attacking_side_ref,
            target_active_index,
            choice,
            Items::YACHEBERRY,
            &PokemonType::ICE,
            instructions,
        ),
        _ => {}
    }

    let attacking_side = state.get_side(attacking_side_ref);
    let active_index = attacking_side
        .get_slot_immutable(attacking_slot_ref)
        .active_index;
    let active_pkmn = attacking_side.get_active(attacking_slot_ref);
    match active_pkmn.item {
        Items::NORMALGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::NORMAL,
            instructions,
        ),
        Items::BUGGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::BUG,
            instructions,
        ),
        Items::ELECTRICGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::ELECTRIC,
            instructions,
        ),
        Items::FIGHTINGGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::FIGHTING,
            instructions,
        ),
        Items::GHOSTGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::GHOST,
            instructions,
        ),
        Items::PSYCHICGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::PSYCHIC,
            instructions,
        ),
        Items::FLYINGGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::FLYING,
            instructions,
        ),
        Items::STEELGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::STEEL,
            instructions,
        ),
        Items::ICEGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::ICE,
            instructions,
        ),
        Items::POISONGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::POISON,
            instructions,
        ),
        Items::FIREGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::FIRE,
            instructions,
        ),
        Items::DRAGONGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::DRAGON,
            instructions,
        ),
        Items::GROUNDGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::GROUND,
            instructions,
        ),
        Items::WATERGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::WATER,
            instructions,
        ),
        Items::DARKGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::DARK,
            instructions,
        ),
        Items::ROCKGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::ROCK,
            instructions,
        ),
        Items::GRASSGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::GRASS,
            instructions,
        ),
        Items::FAIRYGEM => power_up_gem(
            attacking_side_ref,
            active_pkmn,
            active_index,
            choice,
            PokemonType::FAIRY,
            instructions,
        ),
        Items::LUMBERRY if active_pkmn.status != PokemonStatus::NONE => {
            lum_berry(active_pkmn, active_index, attacking_side_ref, instructions)
        }
        Items::SITRUSBERRY
            if active_pkmn.ability == Abilities::GLUTTONY
                && active_pkmn.hp <= active_pkmn.maxhp / 2 =>
        {
            sitrus_berry(active_pkmn, active_index, attacking_side_ref, instructions)
        }
        Items::SITRUSBERRY if active_pkmn.hp <= active_pkmn.maxhp / 4 => {
            sitrus_berry(active_pkmn, active_index, attacking_side_ref, instructions)
        }
        Items::CHESTOBERRY if active_pkmn.status == PokemonStatus::SLEEP => chesto_berry(
            attacking_side,
            active_index,
            attacking_side_ref,
            instructions,
        ),
        Items::PETAYABERRY if active_pkmn.hp <= active_pkmn.maxhp / 4 => boost_berry(
            state,
            attacking_side_ref,
            attacking_slot_ref,
            PokemonBoostableStat::SpecialAttack,
            instructions,
        ),
        Items::LIECHIBERRY if active_pkmn.hp <= active_pkmn.maxhp / 4 => boost_berry(
            state,
            attacking_side_ref,
            attacking_slot_ref,
            PokemonBoostableStat::Attack,
            instructions,
        ),
        Items::SALACBERRY if active_pkmn.hp <= active_pkmn.maxhp / 4 => boost_berry(
            state,
            attacking_side_ref,
            attacking_slot_ref,
            PokemonBoostableStat::Speed,
            instructions,
        ),
        Items::CHOICESPECS | Items::CHOICEBAND | Items::CHOICESCARF => {
            let ins = get_choice_move_disable_instructions(
                active_pkmn,
                active_index,
                attacking_side_ref,
                &choice.move_id,
            );
            for i in ins {
                state.apply_one_instruction(&i);
                instructions.instruction_list.push(i);
            }
        }
        Items::PROTECTIVEPADS => {
            choice.flags.contact = false;
        }
        _ => {}
    }
}

pub fn item_on_switch_in(
    state: &mut State,
    side_ref: &SideReference,
    slot_ref: &SlotReference,
    instructions: &mut StateInstructions,
) {
    let active_terrain = state.get_terrain();
    let switching_in_side = state.get_side(side_ref);
    let switching_in_index = switching_in_side.get_slot_immutable(slot_ref).active_index;
    let switching_in_pkmn = switching_in_side.get_active_immutable(&slot_ref);
    match switching_in_pkmn.item {
        Items::ELECTRICSEED => {
            if active_terrain == Terrain::ELECTRICTERRAIN {
                if apply_boost_instructions(
                    switching_in_side,
                    &PokemonBoostableStat::Defense,
                    &1,
                    side_ref,
                    side_ref,
                    slot_ref,
                    instructions,
                ) {
                    state.get_side(side_ref).get_active(slot_ref).item = Items::NONE;
                    instructions.instruction_list.push(Instruction::ChangeItem(
                        ChangeItemInstruction {
                            side_ref: side_ref.clone(),
                            pokemon_index: switching_in_index,
                            current_item: Items::ELECTRICSEED,
                            new_item: Items::NONE,
                        },
                    ));
                }
            }
        }
        Items::GRASSYSEED => {
            if active_terrain == Terrain::GRASSYTERRAIN {
                if apply_boost_instructions(
                    switching_in_side,
                    &PokemonBoostableStat::Defense,
                    &1,
                    side_ref,
                    side_ref,
                    slot_ref,
                    instructions,
                ) {
                    state.get_side(side_ref).get_active(slot_ref).item = Items::NONE;
                    instructions.instruction_list.push(Instruction::ChangeItem(
                        ChangeItemInstruction {
                            side_ref: side_ref.clone(),
                            pokemon_index: switching_in_index,
                            current_item: Items::GRASSYSEED,
                            new_item: Items::NONE,
                        },
                    ));
                }
            }
        }
        Items::MISTYSEED => {
            if active_terrain == Terrain::MISTYTERRAIN {
                if apply_boost_instructions(
                    switching_in_side,
                    &PokemonBoostableStat::SpecialDefense,
                    &1,
                    side_ref,
                    side_ref,
                    slot_ref,
                    instructions,
                ) {
                    state.get_side(side_ref).get_active(slot_ref).item = Items::NONE;
                    instructions.instruction_list.push(Instruction::ChangeItem(
                        ChangeItemInstruction {
                            side_ref: side_ref.clone(),
                            pokemon_index: switching_in_index,
                            current_item: Items::MISTYSEED,
                            new_item: Items::NONE,
                        },
                    ));
                }
            }
        }
        Items::PSYCHICSEED => {
            if active_terrain == Terrain::PSYCHICTERRAIN {
                if apply_boost_instructions(
                    switching_in_side,
                    &PokemonBoostableStat::SpecialDefense,
                    &1,
                    side_ref,
                    side_ref,
                    slot_ref,
                    instructions,
                ) {
                    state.get_side(side_ref).get_active(slot_ref).item = Items::NONE;
                    instructions.instruction_list.push(Instruction::ChangeItem(
                        ChangeItemInstruction {
                            side_ref: side_ref.clone(),
                            pokemon_index: switching_in_index,
                            current_item: Items::PSYCHICSEED,
                            new_item: Items::NONE,
                        },
                    ));
                }
            }
        }
        _ => {}
    }
}

pub fn item_end_of_turn(
    state: &mut State,
    side_ref: &SideReference,
    slot_ref: &SlotReference,
    instructions: &mut StateInstructions,
) {
    let attacking_side = state.get_side(side_ref);
    let active_pkmn_index = attacking_side.get_slot_immutable(slot_ref).active_index;
    let active_pkmn = attacking_side.get_active(slot_ref);
    match active_pkmn.item {
        Items::LUMBERRY if active_pkmn.status != PokemonStatus::NONE => {
            lum_berry(active_pkmn, active_pkmn_index, side_ref, instructions)
        }
        Items::SITRUSBERRY if active_pkmn.hp <= active_pkmn.maxhp / 2 => {
            sitrus_berry(active_pkmn, active_pkmn_index, side_ref, instructions)
        }
        Items::CHESTOBERRY if active_pkmn.status == PokemonStatus::SLEEP => {
            chesto_berry(attacking_side, active_pkmn_index, side_ref, instructions)
        }
        Items::BLACKSLUDGE => {
            if active_pkmn.has_type(&PokemonType::POISON) {
                if active_pkmn.hp < active_pkmn.maxhp {
                    let heal_amount =
                        cmp::min(active_pkmn.maxhp / 16, active_pkmn.maxhp - active_pkmn.hp);
                    let ins = Instruction::Heal(HealInstruction {
                        side_ref: side_ref.clone(),
                        pokemon_index: active_pkmn_index,
                        heal_amount,
                    });
                    active_pkmn.hp += heal_amount;
                    instructions.instruction_list.push(ins);
                }
            } else {
                let damage_amount =
                    cmp::min(active_pkmn.maxhp / 16, active_pkmn.maxhp - active_pkmn.hp);
                let ins = Instruction::Damage(DamageInstruction {
                    side_ref: side_ref.clone(),
                    pokemon_index: active_pkmn_index,
                    damage_amount,
                });
                active_pkmn.hp -= damage_amount;
                instructions.instruction_list.push(ins);
            }
        }
        Items::FLAMEORB => {
            if !immune_to_status(
                state,
                &MoveTarget::User,
                side_ref,
                side_ref,
                slot_ref,
                &PokemonStatus::BURN,
            ) {
                let side = state.get_side(side_ref);
                let ins = Instruction::ChangeStatus(ChangeStatusInstruction {
                    side_ref: side_ref.clone(),
                    pokemon_index: active_pkmn_index,
                    new_status: PokemonStatus::BURN,
                    old_status: PokemonStatus::NONE,
                });
                side.get_active(slot_ref).status = PokemonStatus::BURN;
                instructions.instruction_list.push(ins);
            }
        }
        Items::LEFTOVERS => {
            let attacker = state.get_side(side_ref).get_active(slot_ref);
            if attacker.hp < attacker.maxhp {
                let heal_amount = cmp::min(attacker.maxhp / 16, attacker.maxhp - attacker.hp);
                let ins = Instruction::Heal(HealInstruction {
                    side_ref: side_ref.clone(),
                    pokemon_index: active_pkmn_index,
                    heal_amount,
                });
                attacker.hp += heal_amount;
                instructions.instruction_list.push(ins);
            }
        }
        Items::TOXICORB => {
            if !immune_to_status(
                state,
                &MoveTarget::User,
                side_ref,
                side_ref,
                slot_ref,
                &PokemonStatus::TOXIC,
            ) {
                let side = state.get_side(side_ref);
                let ins = Instruction::ChangeStatus(ChangeStatusInstruction {
                    side_ref: side_ref.clone(),
                    pokemon_index: active_pkmn_index,
                    new_status: PokemonStatus::TOXIC,
                    old_status: PokemonStatus::NONE,
                });
                side.get_active(slot_ref).status = PokemonStatus::TOXIC;
                instructions.instruction_list.push(ins);
            }
        }
        _ => {}
    }
}

pub fn item_modify_attack_against(
    state: &State,
    attacking_choice: &mut Choice,
    target_side_ref: &SideReference,
    target_slot_ref: &SlotReference,
) {
    let target_side = state.get_side_immutable(target_side_ref);
    match target_side.get_active_immutable(target_slot_ref).item {
        Items::ABSORBBULB => {
            if attacking_choice.move_type == PokemonType::WATER {
                attacking_choice.add_or_create_secondaries(Secondary {
                    chance: 100.0,
                    effect: Effect::Boost(StatBoosts {
                        attack: 0,
                        defense: 0,
                        special_attack: 1,
                        special_defense: 0,
                        speed: 0,
                        accuracy: 0,
                    }),
                    target: MoveTarget::Target,
                });
                attacking_choice.add_or_create_secondaries(Secondary {
                    chance: 100.0,
                    effect: Effect::RemoveItem,
                    target: MoveTarget::Target,
                });
            }
        }
        Items::AIRBALLOON => {
            if attacking_choice.move_type == PokemonType::GROUND
                && attacking_choice.move_id != Choices::THOUSANDARROWS
            {
                attacking_choice.base_power = 0.0;
            } else if attacking_choice.target == MoveTarget::Target
                && attacking_choice.category != MoveCategory::Status
            {
                attacking_choice.add_or_create_secondaries(Secondary {
                    chance: 100.0,
                    effect: Effect::RemoveItem,
                    target: MoveTarget::Target,
                });
            }
        }
        Items::ASSAULTVEST => {
            if attacking_choice.targets_special_defense() {
                attacking_choice.base_power /= 1.5;
            }
        }
        Items::METALPOWDER
            if target_side.get_active_immutable(target_slot_ref).id == PokemonName::DITTO =>
        {
            attacking_choice.base_power /= 1.5;
        }
        Items::CELLBATTERY => {
            if attacking_choice.move_type == PokemonType::ELECTRIC {
                attacking_choice.add_or_create_secondaries(Secondary {
                    chance: 100.0,
                    effect: Effect::Boost(StatBoosts {
                        attack: 1,
                        defense: 0,
                        special_attack: 0,
                        special_defense: 0,
                        speed: 0,
                        accuracy: 0,
                    }),
                    target: MoveTarget::Target,
                });
                attacking_choice.add_or_create_secondaries(Secondary {
                    chance: 100.0,
                    effect: Effect::RemoveItem,
                    target: MoveTarget::Target,
                });
            }
        }
        Items::EVIOLITE => {
            attacking_choice.base_power /= 1.5;
        }
        Items::ROCKYHELMET => {
            if attacking_choice.flags.contact {
                attacking_choice.add_or_create_secondaries(Secondary {
                    chance: 100.0,
                    effect: Effect::Heal(-0.166),
                    target: MoveTarget::User,
                })
            }
        }
        Items::WEAKNESSPOLICY => {
            if attacking_choice.category != MoveCategory::Status
                && type_effectiveness_modifier(
                    &attacking_choice.move_type,
                    &target_side.get_active_immutable(target_slot_ref),
                ) > 1.0
            {
                attacking_choice.add_or_create_secondaries(Secondary {
                    chance: 100.0,
                    effect: Effect::Boost(StatBoosts {
                        attack: 2,
                        defense: 0,
                        special_attack: 2,
                        special_defense: 0,
                        speed: 0,
                        accuracy: 0,
                    }),
                    target: MoveTarget::Target,
                });
                attacking_choice.add_or_create_secondaries(Secondary {
                    chance: 100.0,
                    effect: Effect::RemoveItem,
                    target: MoveTarget::Target,
                });
            }
        }
        _ => {}
    }
}

pub fn item_modify_attack_being_used(
    state: &State,
    attacking_choice: &mut Choice,
    attacking_side_ref: &SideReference,
    attacking_slot_ref: &SlotReference,
    _target_side_ref: &SideReference,
    target_slot_ref: &SlotReference,
    final_run_move: bool,
) {
    let attacking_side = state.get_side_immutable(attacking_side_ref);
    let defending_side = state.get_side_immutable(attacking_side_ref);
    match attacking_side.get_active_immutable(attacking_slot_ref).item {
        Items::WELLSPRINGMASK => match attacking_side.get_active_immutable(attacking_slot_ref).id {
            PokemonName::OGERPONWELLSPRING | PokemonName::OGERPONWELLSPRINGTERA => {
                attacking_choice.base_power *= 1.2;
            }
            _ => {}
        },
        Items::HEARTHFLAMEMASK => {
            match attacking_side.get_active_immutable(attacking_slot_ref).id {
                PokemonName::OGERPONHEARTHFLAME | PokemonName::OGERPONHEARTHFLAMETERA => {
                    attacking_choice.base_power *= 1.2;
                }
                _ => {}
            }
        }
        Items::CORNERSTONEMASK => {
            match attacking_side.get_active_immutable(attacking_slot_ref).id {
                PokemonName::OGERPONCORNERSTONE | PokemonName::OGERPONCORNERSTONETERA => {
                    attacking_choice.base_power *= 1.2;
                }
                _ => {}
            }
        }
        Items::BLACKBELT => {
            if attacking_choice.move_type == PokemonType::FIGHTING {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::BLACKGLASSES => {
            if attacking_choice.move_type == PokemonType::DARK {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::CHARCOAL => {
            if attacking_choice.move_type == PokemonType::FIRE {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::CHOICEBAND => {
            if attacking_choice.category == MoveCategory::Physical {
                attacking_choice.base_power *= 1.5;
            }
        }
        Items::CHOICESPECS => {
            if attacking_choice.category == MoveCategory::Special {
                attacking_choice.base_power *= 1.5;
            }
        }
        Items::DRAGONFANG | Items::DRAGONSCALE => {
            if attacking_choice.move_type == PokemonType::DRAGON {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::EXPERTBELT => {
            if type_effectiveness_modifier(
                &attacking_choice.move_type,
                &defending_side.get_active_immutable(target_slot_ref),
            ) > 1.0
            {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::FAIRYFEATHER => {
            if attacking_choice.move_type == PokemonType::FAIRY {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::LIFEORB => {
            if attacking_choice.category != MoveCategory::Status {
                attacking_choice.base_power *= 1.3;

                if attacking_side
                    .get_active_immutable(attacking_slot_ref)
                    .ability
                    != Abilities::MAGICGUARD
                    && final_run_move
                {
                    attacking_choice.add_or_create_secondaries(Secondary {
                        chance: 100.0,
                        effect: Effect::Heal(-0.1),
                        target: MoveTarget::User,
                    });
                }
            }
        }
        Items::METALCOAT => {
            if attacking_choice.move_type == PokemonType::STEEL {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::MUSCLEBAND => {
            if attacking_choice.category == MoveCategory::Physical {
                attacking_choice.base_power *= 1.1;
            }
        }
        Items::MYSTICWATER => {
            if attacking_choice.move_type == PokemonType::WATER {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::NEVERMELTICE => {
            if attacking_choice.move_type == PokemonType::ICE {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::PINKBOW | Items::POLKADOTBOW => {
            if attacking_choice.move_type == PokemonType::NORMAL {
                attacking_choice.base_power *= 1.1;
            }
        }
        Items::ODDINCENSE => {
            if attacking_choice.move_type == PokemonType::PSYCHIC {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::POISONBARB => {
            if attacking_choice.move_type == PokemonType::POISON {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::PUNCHINGGLOVE => {
            if attacking_choice.flags.punch {
                attacking_choice.base_power *= 1.1;
                attacking_choice.flags.contact = false
            }
        }
        Items::SEAINCENSE => {
            if attacking_choice.move_type == PokemonType::WATER {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::SHARPBEAK => {
            if attacking_choice.move_type == PokemonType::FLYING {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::SHELLBELL => {
            attacking_choice.drain = Some(0.125);
        }
        Items::SILKSCARF => {
            if attacking_choice.move_type == PokemonType::NORMAL {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::SILVERPOWDER => {
            if attacking_choice.move_type == PokemonType::BUG {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::SOFTSAND => {
            if attacking_choice.move_type == PokemonType::GROUND {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::SPELLTAG => {
            if attacking_choice.move_type == PokemonType::GHOST {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::MIRACLESEED => {
            if attacking_choice.move_type == PokemonType::GRASS {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::SOULDEW => {
            if attacking_side.get_active_immutable(attacking_slot_ref).id == PokemonName::LATIOS
                || attacking_side.get_active_immutable(attacking_slot_ref).id == PokemonName::LATIAS
            {
                if attacking_choice.move_type == PokemonType::DRAGON
                    || attacking_choice.move_type == PokemonType::PSYCHIC
                {
                    attacking_choice.base_power *= 1.2;
                }
            }
        }
        Items::GRISEOUSORB | Items::GRISEOUSCORE => {
            if [PokemonName::GIRATINAORIGIN, PokemonName::GIRATINA]
                .contains(&attacking_side.get_active_immutable(attacking_slot_ref).id)
            {
                if attacking_choice.move_type == PokemonType::DRAGON
                    || attacking_choice.move_type == PokemonType::GHOST
                {
                    attacking_choice.base_power *= 1.2;
                }
            }
        }
        Items::LUSTROUSORB | Items::LUSTROUSGLOBE => {
            if [PokemonName::PALKIAORIGIN, PokemonName::PALKIA]
                .contains(&attacking_side.get_active_immutable(attacking_slot_ref).id)
            {
                if attacking_choice.move_type == PokemonType::DRAGON
                    || attacking_choice.move_type == PokemonType::WATER
                {
                    attacking_choice.base_power *= 1.2;
                }
            }
        }
        Items::ADAMANTORB | Items::ADAMANTCRYSTAL => {
            if [PokemonName::DIALGAORIGIN, PokemonName::DIALGA]
                .contains(&attacking_side.get_active_immutable(attacking_slot_ref).id)
            {
                if attacking_choice.move_type == PokemonType::DRAGON
                    || attacking_choice.move_type == PokemonType::STEEL
                {
                    attacking_choice.base_power *= 1.2;
                }
            }
        }
        Items::THROATSPRAY => {
            if attacking_choice.flags.sound {
                attacking_choice.add_or_create_secondaries(Secondary {
                    chance: 100.0,
                    effect: Effect::Boost(StatBoosts {
                        attack: 0,
                        defense: 0,
                        special_attack: 1,
                        special_defense: 0,
                        speed: 0,
                        accuracy: 0,
                    }),
                    target: MoveTarget::User,
                });
                attacking_choice.add_or_create_secondaries(Secondary {
                    chance: 100.0,
                    effect: Effect::RemoveItem,
                    target: MoveTarget::User,
                });
            }
        }
        Items::THICKCLUB => match attacking_side.get_active_immutable(attacking_slot_ref).id {
            PokemonName::CUBONE
            | PokemonName::MAROWAK
            | PokemonName::MAROWAKALOLA
            | PokemonName::MAROWAKALOLATOTEM => {
                attacking_choice.base_power *= 2.0;
            }
            _ => {}
        },
        Items::TWISTEDSPOON => {
            if attacking_choice.move_type == PokemonType::PSYCHIC {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::HARDSTONE => {
            if attacking_choice.move_type == PokemonType::ROCK {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::WAVEINCENSE => {
            if attacking_choice.move_type == PokemonType::WATER {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::MAGNET => {
            if attacking_choice.move_type == PokemonType::ELECTRIC {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::WISEGLASSES => {
            if attacking_choice.category == MoveCategory::Special {
                attacking_choice.base_power *= 1.1;
            }
        }
        Items::FISTPLATE => {
            if attacking_choice.move_type == PokemonType::FIGHTING {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::SKYPLATE => {
            if attacking_choice.move_type == PokemonType::FLYING {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::TOXICPLATE => {
            if attacking_choice.move_type == PokemonType::POISON {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::EARTHPLATE => {
            if attacking_choice.move_type == PokemonType::GROUND {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::STONEPLATE => {
            if attacking_choice.move_type == PokemonType::ROCK {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::INSECTPLATE => {
            if attacking_choice.move_type == PokemonType::BUG {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::SPOOKYPLATE => {
            if attacking_choice.move_type == PokemonType::GHOST {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::IRONPLATE => {
            if attacking_choice.move_type == PokemonType::STEEL {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::FLAMEPLATE => {
            if attacking_choice.move_type == PokemonType::FIRE {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::SPLASHPLATE => {
            if attacking_choice.move_type == PokemonType::WATER {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::MEADOWPLATE => {
            if attacking_choice.move_type == PokemonType::GRASS {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::ZAPPLATE => {
            if attacking_choice.move_type == PokemonType::ELECTRIC {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::MINDPLATE => {
            if attacking_choice.move_type == PokemonType::PSYCHIC {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::ICICLEPLATE => {
            if attacking_choice.move_type == PokemonType::ICE {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::DRACOPLATE => {
            if attacking_choice.move_type == PokemonType::DRAGON {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::DREADPLATE => {
            if attacking_choice.move_type == PokemonType::DARK {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::PIXIEPLATE => {
            if attacking_choice.move_type == PokemonType::FAIRY {
                attacking_choice.base_power *= 1.2;
            }
        }
        Items::LIGHTBALL => {
            if attacking_side
                .get_active_immutable(attacking_slot_ref)
                .id
                .is_pikachu_variant()
            {
                attacking_choice.base_power *= 2.0;
            }
        }
        _ => {}
    }
}
