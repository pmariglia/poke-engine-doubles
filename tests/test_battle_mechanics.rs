use poke_engine::choices::Choices;
use poke_engine::engine::abilities::{Abilities, WEATHER_ABILITY_TURNS};
use poke_engine::engine::generate_instructions::generate_instructions_from_move_pair;
use poke_engine::engine::items::Items;
use poke_engine::engine::state::{MoveChoice, PokemonVolatileStatus, Terrain, Weather};
use poke_engine::instruction::Instruction::{
    DecrementTerrainTurnsRemaining, DecrementTrickRoomTurnsRemaining,
};
use poke_engine::instruction::{
    ApplyVolatileStatusInstruction, BoostInstruction, ChangeAbilityInstruction,
    ChangeItemInstruction, ChangeSideConditionInstruction, ChangeStatInstruction,
    ChangeStatusInstruction, ChangeTerrain, ChangeType, ChangeVolatileStatusDurationInstruction,
    ChangeWeather, DamageInstruction, DisableMoveInstruction, FormeChangeInstruction,
    HealInstruction, IncrementTimesAttackedInstruction, Instruction,
    RemoveVolatileStatusInstruction, SetLastUsedMoveInstruction,
    SetSecondMoveSwitchOutMoveInstruction, SetSleepTurnsInstruction, StateInstructions,
    SwitchInstruction, ToggleForceSwitchInstruction, ToggleTerastallizedInstruction,
};
use poke_engine::pokemon::PokemonName;
use poke_engine::state::LastUsedMove;
use poke_engine::state::{
    PokemonBoostableStat, PokemonIndex, PokemonMoveIndex, PokemonSideCondition, PokemonStatus,
    PokemonType, SideReference, SlotReference, State,
};

struct TestMoveChoice {
    choice: Choices,
    move_choice: MoveChoice,
}
impl Default for TestMoveChoice {
    fn default() -> Self {
        TestMoveChoice {
            choice: Choices::NONE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        }
    }
}

pub fn generate_instructions_with_state_assertion(
    state: &mut State,
    side_one_a_move: &MoveChoice,
    side_one_b_move: &MoveChoice,
    side_two_a_move: &MoveChoice,
    side_two_b_move: &MoveChoice,
) -> Vec<StateInstructions> {
    let before_state_string = format!("{:?}", state);
    let before_serialized = state.serialize();
    let instructions = generate_instructions_from_move_pair(
        state,
        side_one_a_move,
        side_one_b_move,
        side_two_a_move,
        side_two_b_move,
        false,
    );
    let after_state_string = format!("{:?}", state);
    let after_serialized = state.serialize();
    assert_eq!(before_state_string, after_state_string);
    assert_eq!(before_serialized, after_serialized);
    instructions
}

fn set_moves_on_pkmn_and_call_generate_instructions(
    state: &mut State,
    move_one_a: TestMoveChoice,
    move_one_b: TestMoveChoice,
    move_two_a: TestMoveChoice,
    move_two_b: TestMoveChoice,
) -> Vec<StateInstructions> {
    state
        .side_one
        .get_active(&SlotReference::SlotA)
        .replace_move(PokemonMoveIndex::M0, move_one_a.choice);
    state
        .side_one
        .get_active(&SlotReference::SlotB)
        .replace_move(PokemonMoveIndex::M0, move_one_b.choice);
    state
        .side_two
        .get_active(&SlotReference::SlotA)
        .replace_move(PokemonMoveIndex::M0, move_two_a.choice);
    state
        .side_two
        .get_active(&SlotReference::SlotB)
        .replace_move(PokemonMoveIndex::M0, move_two_b.choice);

    let instructions = generate_instructions_with_state_assertion(
        state,
        &move_one_a.move_choice,
        &move_one_b.move_choice,
        &move_two_a.move_choice,
        &move_two_b.move_choice,
    );
    instructions
}

#[test]
fn test_all_participants_using_tackle_on_a_separate_target() {
    let mut state = State::default();
    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
    );
    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 48,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 48,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_everybody_targetting_side_one_slot_a() {
    let mut state = State::default();
    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::NONE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
    );
    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 4,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_basic_switching() {
    let mut state = State::default();
    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::NONE,
            move_choice: MoveChoice::Switch(PokemonIndex::P2),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );
    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Switch(SwitchInstruction {
            side_ref: SideReference::SideOne,
            slot_ref: SlotReference::SlotA,
            previous_index: PokemonIndex::P0,
            next_index: PokemonIndex::P2,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_switching_with_volatile_status_durations() {
    let mut state = State::default();
    state.side_one.slot_a.volatile_status_durations.confusion = 1;
    state.side_one.slot_a.volatile_status_durations.protect = 1;
    state.side_one.slot_a.volatile_status_durations.encore = 2;
    state.side_one.slot_a.volatile_status_durations.taunt = 3;
    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::NONE,
            move_choice: MoveChoice::Switch(PokemonIndex::P2),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );
    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ChangeVolatileStatusDuration(ChangeVolatileStatusDurationInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::CONFUSION,
                amount: -1,
            }),
            Instruction::ChangeVolatileStatusDuration(ChangeVolatileStatusDurationInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::PROTECT,
                amount: -1,
            }),
            Instruction::ChangeVolatileStatusDuration(ChangeVolatileStatusDurationInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::TAUNT,
                amount: -3,
            }),
            Instruction::ChangeVolatileStatusDuration(ChangeVolatileStatusDurationInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::ENCORE,
                amount: -2,
            }),
            Instruction::Switch(SwitchInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                previous_index: PokemonIndex::P0,
                next_index: PokemonIndex::P2,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_switching_in_terrain_activates_seed() {
    let mut state = State::default();
    state.terrain.terrain_type = Terrain::ELECTRICTERRAIN;
    state.terrain.turns_remaining = 5;
    state.side_one.pokemon.pkmn[2].item = Items::ELECTRICSEED;
    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::NONE,
            move_choice: MoveChoice::Switch(PokemonIndex::P2),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );
    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Switch(SwitchInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                previous_index: PokemonIndex::P0,
                next_index: PokemonIndex::P2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::Defense,
                amount: 1,
            }),
            Instruction::ChangeItem(ChangeItemInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P2,
                current_item: Items::ELECTRICSEED,
                new_item: Items::NONE,
            }),
            Instruction::DecrementTerrainTurnsRemaining,
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_boost_berry() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].item = Items::SALACBERRY;
    state.side_two.pokemon.pkmn[0].speed = 1;
    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::SPLASH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::SPLASH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
    );
    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideTwo,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::Speed,
                amount: 1,
            }),
            Instruction::ChangeItem(ChangeItemInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                current_item: Items::SALACBERRY,
                new_item: Items::NONE,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_choice_item_locking_and_boost() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].item = Items::CHOICEBAND;
    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );
    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::DisableMove(DisableMoveInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                move_index: PokemonMoveIndex::M1,
            }),
            Instruction::DisableMove(DisableMoveInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                move_index: PokemonMoveIndex::M2,
            }),
            Instruction::DisableMove(DisableMoveInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                move_index: PokemonMoveIndex::M3,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 72,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_leftovers_heal() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].item = Items::LEFTOVERS;
    state.side_one.pokemon.pkmn[0].hp = 50;
    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );
    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Heal(HealInstruction {
            side_ref: SideReference::SideOne,
            pokemon_index: PokemonIndex::P0,
            heal_amount: 6,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_neutralizing_gas_blocks_abilities() {
    let mut state = State::default();
    // Set target to have Neutralizing Gas
    state.side_two.pokemon.pkmn[0].ability = Abilities::NEUTRALIZINGGAS;
    // Set attacker to have Ice Face (should be blocked)
    state.side_one.pokemon.pkmn[0].ability = Abilities::ICEFACE;
    state.side_one.pokemon.pkmn[0].id = PokemonName::EISCUE;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    // Should just do normal damage, no Ice Face activation
    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 48,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_ice_face_blocks_physical_move() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::ICEFACE;
    state.side_two.pokemon.pkmn[0].id = PokemonName::EISCUE;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::FormeChange(FormeChangeInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                name_change: PokemonName::EISCUENOICE as i16 - PokemonName::EISCUE as i16,
            }),
            Instruction::ChangeAttack(ChangeStatInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                amount: 117,
            }),
            Instruction::ChangeDefense(ChangeStatInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                amount: 97,
            }),
            Instruction::ChangeSpecialAttack(ChangeStatInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                amount: 87,
            }),
            Instruction::ChangeSpecialDefense(ChangeStatInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                amount: 57,
            }),
            Instruction::ChangeSpeed(ChangeStatInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                amount: 216,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_ice_face_does_not_block_special_move() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::ICEFACE;
    state.side_two.pokemon.pkmn[0].id = PokemonName::EISCUE;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::WATERGUN,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    // Should do normal damage, Ice Face doesn't activate for special moves
    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 32,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_disguise_blocks_damaging_move() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::DISGUISE;
    state.side_two.pokemon.pkmn[0].id = PokemonName::MIMIKYU;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::FormeChange(FormeChangeInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                name_change: PokemonName::MIMIKYUBUSTED as i16 - PokemonName::MIMIKYU as i16,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 12, // 1/8 of max HP (assuming 100 max HP)
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_disguise_totem_form() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::DISGUISE;
    state.side_two.pokemon.pkmn[0].id = PokemonName::MIMIKYUTOTEM;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::FormeChange(FormeChangeInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                name_change: PokemonName::MIMIKYUBUSTED as i16 - PokemonName::MIMIKYUTOTEM as i16,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 12, // 1/8 of max HP
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_gulp_missile_surf_high_hp() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::GULPMISSILE;
    state.side_one.pokemon.pkmn[0].id = PokemonName::CRAMORANT;
    state.side_one.pokemon.pkmn[0].hp = 80;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::SURF,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::FormeChange(FormeChangeInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                name_change: PokemonName::CRAMORANTGULPING as i16 - PokemonName::CRAMORANT as i16,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 71,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 71,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 71,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_gulp_missile_dive_low_hp() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::GULPMISSILE;
    state.side_one.pokemon.pkmn[0].id = PokemonName::CRAMORANT;
    state.side_one.pokemon.pkmn[0].hp = 40; // Less than half of max HP (100)

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::SURF,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::FormeChange(FormeChangeInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                name_change: PokemonName::CRAMORANTGORGING as i16 - PokemonName::CRAMORANT as i16,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 71,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 71,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 71,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_protean_changes_type() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::PROTEAN;
    state.side_one.pokemon.pkmn[0].types = (PokemonType::NORMAL, PokemonType::TYPELESS);

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::WATERGUN,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ChangeType(ChangeType {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                new_types: (PokemonType::WATER, PokemonType::TYPELESS),
                old_types: (PokemonType::NORMAL, PokemonType::TYPELESS),
            }),
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::TYPECHANGE,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_libero_changes_type() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::LIBERO;
    state.side_one.pokemon.pkmn[0].types = (PokemonType::NORMAL, PokemonType::TYPELESS);

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::FLAMETHROWER,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ChangeType(ChangeType {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                new_types: (PokemonType::FIRE, PokemonType::TYPELESS),
                old_types: (PokemonType::NORMAL, PokemonType::TYPELESS),
            }),
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::TYPECHANGE,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 100,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_protean_does_not_activate_if_already_same_type() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::PROTEAN;
    state.side_one.pokemon.pkmn[0].types = (PokemonType::WATER, PokemonType::TYPELESS);

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::WATERGUN,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    // Should just do damage, no type change since already Water type
    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 48,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_protean_does_not_activate_if_terastallized() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::PROTEAN;
    state.side_one.pokemon.pkmn[0].types = (PokemonType::NORMAL, PokemonType::TYPELESS);
    state.side_one.pokemon.pkmn[0].terastallized = true;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::WATERGUN,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 32,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_protean_does_not_activate_with_typechange_status() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::PROTEAN;
    state.side_one.pokemon.pkmn[0].types = (PokemonType::NORMAL, PokemonType::TYPELESS);
    state
        .side_one
        .get_slot(&SlotReference::SlotA)
        .volatile_statuses
        .insert(PokemonVolatileStatus::TYPECHANGE);

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::WATERGUN,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 32,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_gorilla_tactics_disables_other_moves() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::GORILLATACTICS;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::DisableMove(DisableMoveInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                move_index: PokemonMoveIndex::M1,
            }),
            Instruction::DisableMove(DisableMoveInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                move_index: PokemonMoveIndex::M2,
            }),
            Instruction::DisableMove(DisableMoveInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                move_index: PokemonMoveIndex::M3,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 72,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_mummy_spreads_on_contact() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::MUMMY;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE, // Contact move
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::ChangeAbility(ChangeAbilityInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                ability_change: Abilities::MUMMY as i16 - Abilities::NONE as i16,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_mummy_does_not_spread_on_non_contact() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::MUMMY;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::WATERGUN, // Non-contact move
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 32,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_lingering_aroma_spreads_on_contact() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::LINGERINGAROMA;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::ChangeAbility(ChangeAbilityInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                ability_change: Abilities::MUMMY as i16 - Abilities::NONE as i16,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_wandering_spirit_spreads_on_contact() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::WANDERINGSPIRIT;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::ChangeAbility(ChangeAbilityInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                ability_change: Abilities::MUMMY as i16 - Abilities::NONE as i16,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_gulp_missile_gorging_form_changes_and_paralyzes() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::GULPMISSILE;
    state.side_two.pokemon.pkmn[0].id = PokemonName::CRAMORANTGORGING;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::FormeChange(FormeChangeInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                name_change: PokemonName::CRAMORANT as i16 - PokemonName::CRAMORANTGORGING as i16,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 25, // 1/4 of max HP
            }),
            Instruction::ChangeStatus(ChangeStatusInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                old_status: PokemonStatus::NONE,
                new_status: PokemonStatus::PARALYZE,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_gulp_missile_gulping_form_changes_and_lowers_defense() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::GULPMISSILE;
    state.side_two.pokemon.pkmn[0].id = PokemonName::CRAMORANTGULPING;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::FormeChange(FormeChangeInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                name_change: PokemonName::CRAMORANT as i16 - PokemonName::CRAMORANTGULPING as i16,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::Defense,
                amount: -1,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 25,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_color_change_changes_type_to_move_type() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::COLORCHANGE;
    state.side_one.pokemon.pkmn[0].types = (PokemonType::NORMAL, PokemonType::TYPELESS);

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::WATERGUN,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 32,
            }),
            Instruction::ChangeType(ChangeType {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                new_types: (PokemonType::WATER, PokemonType::TYPELESS),
                old_types: (PokemonType::NORMAL, PokemonType::TYPELESS),
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_color_change_does_not_activate_if_already_same_type() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::COLORCHANGE;
    state.side_two.pokemon.pkmn[0].types = (PokemonType::WATER, PokemonType::TYPELESS);

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::WATERGUN,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 15,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_color_change_does_not_activate_if_pokemon_faints() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::COLORCHANGE;
    state.side_two.pokemon.pkmn[0].hp = 1; // Will faint from damage

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::WATERGUN,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 1,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_stamina_boosts_defense_when_hit() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::STAMINA;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideTwo,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::Defense,
                amount: 1,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_cotton_down_lowers_attacker_speed() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::COTTONDOWN;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::Speed,
                amount: -1,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_sand_spit_sets_sandstorm() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::SANDSPIT;
    state.weather.weather_type = Weather::NONE;
    // so no weather damage is applied
    state.side_one.pokemon.pkmn[0].types.0 = PokemonType::GROUND;
    state.side_one.pokemon.pkmn[1].types.0 = PokemonType::GROUND;
    state.side_two.pokemon.pkmn[0].types.0 = PokemonType::GROUND;
    state.side_two.pokemon.pkmn[1].types.0 = PokemonType::GROUND;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 32,
            }),
            Instruction::ChangeWeather(ChangeWeather {
                new_weather: Weather::SAND,
                new_weather_turns_remaining: WEATHER_ABILITY_TURNS,
                previous_weather: Weather::NONE,
                previous_weather_turns_remaining: -1,
            }),
            Instruction::DecrementWeatherTurnsRemaining,
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_sand_spit_does_not_activate_if_sandstorm_already_active() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::SANDSPIT;
    state.weather.weather_type = Weather::SAND;
    state.weather.turns_remaining = 3;
    // so no weather damage is applied
    state.side_one.pokemon.pkmn[0].types.0 = PokemonType::GROUND;
    state.side_one.pokemon.pkmn[1].types.0 = PokemonType::GROUND;
    state.side_two.pokemon.pkmn[0].types.0 = PokemonType::GROUND;
    state.side_two.pokemon.pkmn[1].types.0 = PokemonType::GROUND;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 32,
            }),
            Instruction::DecrementWeatherTurnsRemaining,
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_seed_sower_sets_grassy_terrain() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::SEEDSOWER;
    state.terrain.terrain_type = Terrain::NONE;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::ChangeTerrain(ChangeTerrain {
                new_terrain: Terrain::GRASSYTERRAIN,
                new_terrain_turns_remaining: 5,
                previous_terrain: Terrain::NONE,
                previous_terrain_turns_remaining: 0,
            }),
            Instruction::DecrementTerrainTurnsRemaining,
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_toxic_debris_sets_toxic_spikes_on_physical_hit() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::TOXICDEBRIS;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE, // Physical move
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::ChangeSideCondition(ChangeSideConditionInstruction {
                side_ref: SideReference::SideOne,
                side_condition: PokemonSideCondition::ToxicSpikes,
                amount: 1,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_toxic_debris_does_not_activate_on_special_move() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::TOXICDEBRIS;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::WATERGUN, // Special move
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 32,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_berserk_boosts_special_attack_when_crossing_half_hp() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::BERSERK;
    state.side_two.pokemon.pkmn[0].hp = 60; // Above half HP (maxhp is 100)

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideTwo,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::SpecialAttack,
                amount: 1,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_berserk_does_not_activate_if_already_below_half_hp() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::BERSERK;
    state.side_two.pokemon.pkmn[0].hp = 40; // Already below half HP

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 40, // Takes remaining HP
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_rough_skin_damages_attacker_on_contact() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::ROUGHSKIN;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 12, // 1/8 of max HP
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_iron_barbs_damages_attacker_on_contact() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::IRONBARBS;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 12,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_basic_spread_move_damages_both_targets() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::IRONBARBS;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 12,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_aftermath_damages_attacker_when_knocked_out_by_contact() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::AFTERMATH;
    state.side_two.pokemon.pkmn[0].hp = 1; // Will be knocked out

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 1,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 25, // 1/4 of max HP
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_innards_out_damages_attacker_when_knocked_out() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::INNARDSOUT;
    state.side_two.pokemon.pkmn[0].hp = 30; // Will be knocked out by 48 damage

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 30,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 30,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_surf_spread_move() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::SURF,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 71,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 71,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 71,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_spread_move_while_sleeping() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].status = PokemonStatus::SLEEP;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::DAZZLINGGLEAM,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::SetSleepTurns(SetSleepTurnsInstruction {
            side_ref: SideReference::SideOne,
            pokemon_index: PokemonIndex::P0,
            new_turns: 1,
            previous_turns: 0,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_being_slept_into_spread_move() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::DAZZLINGGLEAM,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::SPORE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ChangeStatus(ChangeStatusInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                old_status: PokemonStatus::NONE,
                new_status: PokemonStatus::SLEEP,
            }),
            Instruction::SetSleepTurns(SetSleepTurnsInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                new_turns: 1,
                previous_turns: 0,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_switching_out_while_slept() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].status = PokemonStatus::SLEEP;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::NONE,
            move_choice: MoveChoice::Switch(PokemonIndex::P3),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Switch(SwitchInstruction {
            side_ref: SideReference::SideOne,
            slot_ref: SlotReference::SlotA,
            previous_index: PokemonIndex::P0,
            next_index: PokemonIndex::P3,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_fainting_target_does_not_allow_them_to_move() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].hp = 1;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::SURF,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideOne,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 1,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_lifeorb_with_spread_move_only_damages_once() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].item = Items::LIFEORB;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::SURF,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 92,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 92,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 92,
            }),
            Instruction::Heal(HealInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                heal_amount: -10,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_direct_boost_with_spread_move_only_boosts_once() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::MAKEITRAIN,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 71,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 71,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::SpecialAttack,
                amount: -1,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_spread_move_into_wideguard_during_psychicterrain() {
    let mut state = State::default();
    state.terrain.turns_remaining = 2;
    state.terrain.terrain_type = Terrain::PSYCHICTERRAIN;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::ERUPTION,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::WIDEGUARD,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ChangeSideCondition(ChangeSideConditionInstruction {
                side_ref: SideReference::SideTwo,
                side_condition: PokemonSideCondition::WideGuard,
                amount: 1,
            }),
            Instruction::DecrementTerrainTurnsRemaining,
            Instruction::ChangeSideCondition(ChangeSideConditionInstruction {
                side_ref: SideReference::SideTwo,
                side_condition: PokemonSideCondition::WideGuard,
                amount: -1,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_trick_opposite_side() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].item = Items::CHARCOAL;
    state.side_two.pokemon.pkmn[0].item = Items::ABSORBBULB;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TRICK,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ChangeItem(ChangeItemInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                current_item: Items::ABSORBBULB,
                new_item: Items::CHARCOAL,
            }),
            Instruction::ChangeItem(ChangeItemInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                current_item: Items::CHARCOAL,
                new_item: Items::ABSORBBULB,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_trick_same_side() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].item = Items::CHARCOAL;
    state.side_one.pokemon.pkmn[1].item = Items::ABSORBBULB;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TRICK,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ChangeItem(ChangeItemInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P1,
                current_item: Items::ABSORBBULB,
                new_item: Items::CHARCOAL,
            }),
            Instruction::ChangeItem(ChangeItemInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                current_item: Items::CHARCOAL,
                new_item: Items::ABSORBBULB,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_single_target_move_is_redirected_if_target_faints() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].hp = 10;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 10,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 48,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_lightningrod_redirect_electric_move() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[1].ability = Abilities::LIGHTNINGROD;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::THUNDERSHOCK,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Boost(BoostInstruction {
            side_ref: SideReference::SideTwo,
            slot_ref: SlotReference::SlotB,
            stat: PokemonBoostableStat::SpecialAttack,
            amount: 1,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_lightningrod_redirects_galvanized_move() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[1].ability = Abilities::LIGHTNINGROD;
    state.side_one.pokemon.pkmn[0].ability = Abilities::GALVANIZE;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Boost(BoostInstruction {
            side_ref: SideReference::SideTwo,
            slot_ref: SlotReference::SlotB,
            stat: PokemonBoostableStat::SpecialAttack,
            amount: 1,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_stormdrain_redirects_liquidmoved_move() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[1].ability = Abilities::STORMDRAIN;
    state.side_one.pokemon.pkmn[0].ability = Abilities::LIQUIDVOICE;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::ECHOEDVOICE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Boost(BoostInstruction {
            side_ref: SideReference::SideTwo,
            slot_ref: SlotReference::SlotB,
            stat: PokemonBoostableStat::SpecialAttack,
            amount: 1,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_tailwind_as_prankster_dark_type() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::PRANKSTER;
    state.side_one.pokemon.pkmn[0].types.0 = PokemonType::DARK;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TAILWIND,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ChangeSideCondition(ChangeSideConditionInstruction {
                side_ref: SideReference::SideOne,
                side_condition: PokemonSideCondition::Tailwind,
                amount: 4,
            }),
            Instruction::ChangeSideCondition(ChangeSideConditionInstruction {
                side_ref: SideReference::SideOne,
                side_condition: PokemonSideCondition::Tailwind,
                amount: -1,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_armortail_stop_increased_priority_single_target() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::ARMORTAIL;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::QUICKATTACK,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::QUICKATTACK,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_prankster_status_move_into_armortail() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::PRANKSTER;
    state.side_one.pokemon.pkmn[1].ability = Abilities::PRANKSTER;
    state.side_two.pokemon.pkmn[0].ability = Abilities::ARMORTAIL;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::THUNDERWAVE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::THUNDERWAVE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_moldbreaker_ignores_armortail() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::MOLDBREAKER;
    state.side_two.pokemon.pkmn[0].ability = Abilities::ARMORTAIL;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::QUICKATTACK,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::QUICKATTACK,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 48,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_friendguard_reduces_damage() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[1].ability = Abilities::FRIENDGUARD;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 37,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_friendguard_does_not_reduce_damage_to_self() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::FRIENDGUARD;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 48,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_helping_hand_being_used_on_ally() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::HELPINGHAND,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                volatile_status: PokemonVolatileStatus::HELPINGHAND,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 72,
            }),
            Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                volatile_status: PokemonVolatileStatus::HELPINGHAND,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_aromatic_mist_being_used_on_ally() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::AROMATICMIST,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Boost(BoostInstruction {
            side_ref: SideReference::SideOne,
            slot_ref: SlotReference::SlotB,
            stat: PokemonBoostableStat::SpecialDefense,
            amount: 1,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_ragepowder_applied_and_removed_end_of_turn() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::RAGEPOWDER,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::RAGEPOWDER,
            }),
            Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::RAGEPOWDER,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_electroshot_charges() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::ELECTROSHOT,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::SpecialAttack,
                amount: 1,
            }),
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::ELECTROSHOT,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_electroshot_with_powerherb() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].item = Items::POWERHERB;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::ELECTROSHOT,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::SpecialAttack,
                amount: 1,
            }),
            Instruction::ChangeItem(ChangeItemInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                current_item: Items::POWERHERB,
                new_item: Items::NONE,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 100,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_electroshot_does_not_charge_in_rain() {
    let mut state = State::default();
    state.weather.weather_type = Weather::RAIN;
    state.weather.turns_remaining = 5;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::ELECTROSHOT,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::SpecialAttack,
                amount: 1,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 100,
            }),
            Instruction::DecrementWeatherTurnsRemaining,
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_electroshot_executes_from_charge() {
    let mut state = State::default();
    state
        .side_one
        .slot_a
        .volatile_statuses
        .insert(PokemonVolatileStatus::ELECTROSHOT);

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::ELECTROSHOT,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::ELECTROSHOT,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 100,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_direclaw() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::DIRECLAW,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![
        StateInstructions {
            end_of_turn_triggered: true,
            percentage: 49.998,
            instruction_list: vec![Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 63,
            })],
        },
        StateInstructions {
            end_of_turn_triggered: true,
            percentage: 16.666,
            instruction_list: vec![
                Instruction::Damage(DamageInstruction {
                    side_ref: SideReference::SideTwo,
                    pokemon_index: PokemonIndex::P0,
                    damage_amount: 63,
                }),
                Instruction::ChangeStatus(ChangeStatusInstruction {
                    side_ref: SideReference::SideTwo,
                    pokemon_index: PokemonIndex::P0,
                    old_status: PokemonStatus::NONE,
                    new_status: PokemonStatus::SLEEP,
                }),
            ],
        },
        StateInstructions {
            end_of_turn_triggered: true,
            percentage: 16.666,
            instruction_list: vec![
                Instruction::Damage(DamageInstruction {
                    side_ref: SideReference::SideTwo,
                    pokemon_index: PokemonIndex::P0,
                    damage_amount: 63,
                }),
                Instruction::ChangeStatus(ChangeStatusInstruction {
                    side_ref: SideReference::SideTwo,
                    pokemon_index: PokemonIndex::P0,
                    old_status: PokemonStatus::NONE,
                    new_status: PokemonStatus::PARALYZE,
                }),
            ],
        },
        StateInstructions {
            end_of_turn_triggered: true,
            percentage: 16.67,
            instruction_list: vec![
                Instruction::Damage(DamageInstruction {
                    side_ref: SideReference::SideTwo,
                    pokemon_index: PokemonIndex::P0,
                    damage_amount: 63,
                }),
                Instruction::ChangeStatus(ChangeStatusInstruction {
                    side_ref: SideReference::SideTwo,
                    pokemon_index: PokemonIndex::P0,
                    old_status: PokemonStatus::NONE,
                    new_status: PokemonStatus::POISON,
                }),
                Instruction::Damage(DamageInstruction {
                    side_ref: SideReference::SideTwo,
                    pokemon_index: PokemonIndex::P0,
                    damage_amount: 12,
                }),
            ],
        },
    ];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_ragepowder_causes_move_to_target_this_pkmn() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::RAGEPOWDER,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB, // Targeting the Pokemon not using Rage Powder
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::RAGEPOWDER,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0, // Damage to the Pokemon using Rage Powder
                damage_amount: 48,
            }),
            Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::RAGEPOWDER,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_followme_causes_move_to_target_this_pkmn() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::FOLLOWME,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB, // Targeting the Pokemon not using Rage Powder
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::FOLLOWME,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0, // Damage to the Pokemon using Rage Powder
                damage_amount: 48,
            }),
            Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::FOLLOWME,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_ragepowder_does_not_redirect_grass_pokemon_moves() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].types = (PokemonType::GRASS, PokemonType::NORMAL);

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::RAGEPOWDER,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB, // Targeting the Pokemon not using Rage Powder
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::RAGEPOWDER,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P1, // Rage powder did not redirect to P0
                damage_amount: 48,
            }),
            Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::RAGEPOWDER,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_ragepowder_does_not_redirect_safetygoggles() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].item = Items::SAFETYGOGGLES;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::RAGEPOWDER,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB, // Targeting the Pokemon not using Rage Powder
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::RAGEPOWDER,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P1, // Rage powder did not redirect to P0
                damage_amount: 48,
            }),
            Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::RAGEPOWDER,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_ragepowder_does_not_redirect_overcoat() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::OVERCOAT;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::RAGEPOWDER,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB, // Targeting the Pokemon not using Rage Powder
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::RAGEPOWDER,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P1, // Rage powder did not redirect to P0
                damage_amount: 48,
            }),
            Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::RAGEPOWDER,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_ragepowder_does_not_affect_spread_move() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::RAGEPOWDER,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::OVERDRIVE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::RAGEPOWDER,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 48,
            }),
            Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::RAGEPOWDER,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_knockoff() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].item = Items::LIFEORB;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::KNOCKOFF,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 76,
            }),
            Instruction::ChangeItem(ChangeItemInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                current_item: Items::LIFEORB,
                new_item: Items::NONE,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_protect_setting_volatile() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::SURF,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::PROTECT,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideTwo,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::PROTECT,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 71,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 71,
            }),
            Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                side_ref: SideReference::SideTwo,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::PROTECT,
            }),
            Instruction::ChangeVolatileStatusDuration(ChangeVolatileStatusDurationInstruction {
                side_ref: SideReference::SideTwo,
                slot_ref: SlotReference::SlotA,
                amount: 1,
                volatile_status: PokemonVolatileStatus::PROTECT,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_consequtive_protect_chance_to_fail() {
    let mut state = State::default();
    state.side_two.slot_a.volatile_status_durations.protect = 1;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::SURF,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::PROTECT,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![
        StateInstructions {
            end_of_turn_triggered: true,
            percentage: 66.666664,
            instruction_list: vec![
                Instruction::Damage(DamageInstruction {
                    side_ref: SideReference::SideTwo,
                    pokemon_index: PokemonIndex::P0,
                    damage_amount: 71,
                }),
                Instruction::Damage(DamageInstruction {
                    side_ref: SideReference::SideTwo,
                    pokemon_index: PokemonIndex::P1,
                    damage_amount: 71,
                }),
                Instruction::Damage(DamageInstruction {
                    side_ref: SideReference::SideOne,
                    pokemon_index: PokemonIndex::P1,
                    damage_amount: 71,
                }),
                Instruction::ChangeVolatileStatusDuration(
                    ChangeVolatileStatusDurationInstruction {
                        side_ref: SideReference::SideTwo,
                        slot_ref: SlotReference::SlotA,
                        amount: -1,
                        volatile_status: PokemonVolatileStatus::PROTECT,
                    },
                ),
            ],
        },
        StateInstructions {
            end_of_turn_triggered: true,
            percentage: 33.333336,
            instruction_list: vec![
                Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                    side_ref: SideReference::SideTwo,
                    slot_ref: SlotReference::SlotA,
                    volatile_status: PokemonVolatileStatus::PROTECT,
                }),
                Instruction::Damage(DamageInstruction {
                    side_ref: SideReference::SideTwo,
                    pokemon_index: PokemonIndex::P1,
                    damage_amount: 71,
                }),
                Instruction::Damage(DamageInstruction {
                    side_ref: SideReference::SideOne,
                    pokemon_index: PokemonIndex::P1,
                    damage_amount: 71,
                }),
                Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                    side_ref: SideReference::SideTwo,
                    slot_ref: SlotReference::SlotA,
                    volatile_status: PokemonVolatileStatus::PROTECT,
                }),
                Instruction::ChangeVolatileStatusDuration(
                    ChangeVolatileStatusDurationInstruction {
                        side_ref: SideReference::SideTwo,
                        slot_ref: SlotReference::SlotA,
                        amount: 1,
                        volatile_status: PokemonVolatileStatus::PROTECT,
                    },
                ),
            ],
        },
    ];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_consequtive_spikyshield_chance_to_fail() {
    let mut state = State::default();
    state.side_two.slot_a.volatile_status_durations.protect = 1;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::SURF,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::SPIKYSHIELD,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![
        StateInstructions {
            end_of_turn_triggered: true,
            percentage: 66.666664,
            instruction_list: vec![
                Instruction::Damage(DamageInstruction {
                    side_ref: SideReference::SideTwo,
                    pokemon_index: PokemonIndex::P0,
                    damage_amount: 71,
                }),
                Instruction::Damage(DamageInstruction {
                    side_ref: SideReference::SideTwo,
                    pokemon_index: PokemonIndex::P1,
                    damage_amount: 71,
                }),
                Instruction::Damage(DamageInstruction {
                    side_ref: SideReference::SideOne,
                    pokemon_index: PokemonIndex::P1,
                    damage_amount: 71,
                }),
                Instruction::ChangeVolatileStatusDuration(
                    ChangeVolatileStatusDurationInstruction {
                        side_ref: SideReference::SideTwo,
                        slot_ref: SlotReference::SlotA,
                        amount: -1,
                        volatile_status: PokemonVolatileStatus::PROTECT,
                    },
                ),
            ],
        },
        StateInstructions {
            end_of_turn_triggered: true,
            percentage: 33.333336,
            instruction_list: vec![
                Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                    side_ref: SideReference::SideTwo,
                    slot_ref: SlotReference::SlotA,
                    volatile_status: PokemonVolatileStatus::SPIKYSHIELD,
                }),
                Instruction::Damage(DamageInstruction {
                    side_ref: SideReference::SideTwo,
                    pokemon_index: PokemonIndex::P1,
                    damage_amount: 71,
                }),
                Instruction::Damage(DamageInstruction {
                    side_ref: SideReference::SideOne,
                    pokemon_index: PokemonIndex::P1,
                    damage_amount: 71,
                }),
                Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                    side_ref: SideReference::SideTwo,
                    slot_ref: SlotReference::SlotA,
                    volatile_status: PokemonVolatileStatus::SPIKYSHIELD,
                }),
                Instruction::ChangeVolatileStatusDuration(
                    ChangeVolatileStatusDurationInstruction {
                        side_ref: SideReference::SideTwo,
                        slot_ref: SlotReference::SlotA,
                        amount: 1,
                        volatile_status: PokemonVolatileStatus::PROTECT,
                    },
                ),
            ],
        },
    ];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_wideguard_protection() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::SURF,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::WIDEGUARD,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ChangeSideCondition(ChangeSideConditionInstruction {
                side_ref: SideReference::SideTwo,
                side_condition: PokemonSideCondition::WideGuard,
                amount: 1,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 71,
            }),
            Instruction::ChangeSideCondition(ChangeSideConditionInstruction {
                side_ref: SideReference::SideTwo,
                side_condition: PokemonSideCondition::WideGuard,
                amount: -1,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_wideguard_protection_own_side() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::SURF,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::WIDEGUARD,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::WIDEGUARD,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ChangeSideCondition(ChangeSideConditionInstruction {
                side_ref: SideReference::SideTwo,
                side_condition: PokemonSideCondition::WideGuard,
                amount: 1,
            }),
            Instruction::ChangeSideCondition(ChangeSideConditionInstruction {
                side_ref: SideReference::SideOne,
                side_condition: PokemonSideCondition::WideGuard,
                amount: 1,
            }),
            Instruction::ChangeSideCondition(ChangeSideConditionInstruction {
                side_ref: SideReference::SideOne,
                side_condition: PokemonSideCondition::WideGuard,
                amount: -1,
            }),
            Instruction::ChangeSideCondition(ChangeSideConditionInstruction {
                side_ref: SideReference::SideTwo,
                side_condition: PokemonSideCondition::WideGuard,
                amount: -1,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_pollen_puff_damaging_other_side() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::POLLENPUFF,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 71,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_pollen_puff_healing_own_side() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[1].hp = 25;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::POLLENPUFF,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Heal(HealInstruction {
            side_ref: SideReference::SideOne,
            pokemon_index: PokemonIndex::P1,
            heal_amount: 50,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_covertcloak_prevents_flinch() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[1].item = Items::COVERTCLOAK;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::IRONHEAD,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P1,
            damage_amount: 63,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_suckerpunch_works() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::SUCKERPUNCH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 55,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_suckerpunch_fails_if_target_not_using_attacking_move() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::SUCKERPUNCH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideOne,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 48,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_suckerpunch_fails_if_target_moves_before_user() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::SUCKERPUNCH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::SUCKERPUNCH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideOne,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 55,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_tera_starstorm_targets_all_foes_if_terastallized() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].tera_type = PokemonType::STELLAR;
    state.side_one.pokemon.pkmn[0].terastallized = true;
    state.side_two.pokemon.pkmn[0].maxhp = 500;
    state.side_two.pokemon.pkmn[0].hp = 500;
    state.side_two.pokemon.pkmn[1].maxhp = 500;
    state.side_two.pokemon.pkmn[1].hp = 500;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TERASTARSTORM,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 106,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 106,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_tera_starstorm_targets_one_pkmn_if_not_terastallized() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].tera_type = PokemonType::STELLAR;
    state.side_one.pokemon.pkmn[0].terastallized = false;
    state.side_two.pokemon.pkmn[0].maxhp = 500;
    state.side_two.pokemon.pkmn[0].hp = 500;
    state.side_two.pokemon.pkmn[1].maxhp = 500;
    state.side_two.pokemon.pkmn[1].hp = 500;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TERASTARSTORM,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 141, // more dmg because one target
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_tera_starstorm_uses_attack_if_it_is_higher() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].tera_type = PokemonType::STELLAR;
    state.side_one.pokemon.pkmn[0].terastallized = true;
    state.side_one.pokemon.pkmn[0].attack = 150;
    state.side_one.pokemon.pkmn[0].special_attack = 100;
    state.side_two.pokemon.pkmn[0].maxhp = 500;
    state.side_two.pokemon.pkmn[0].hp = 500;
    state.side_two.pokemon.pkmn[1].maxhp = 500;
    state.side_two.pokemon.pkmn[1].hp = 500;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TERASTARSTORM,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 159,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 159,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_tera_starstorm_without_terastallizing() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].tera_type = PokemonType::STELLAR;
    state.side_two.pokemon.pkmn[0].maxhp = 500;
    state.side_two.pokemon.pkmn[0].hp = 500;
    state.side_two.pokemon.pkmn[1].maxhp = 500;
    state.side_two.pokemon.pkmn[1].hp = 500;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TERASTARSTORM,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 141,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_tera_starstorm_while_terastallizing() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].tera_type = PokemonType::STELLAR;
    state.side_two.pokemon.pkmn[0].maxhp = 500;
    state.side_two.pokemon.pkmn[0].hp = 500;
    state.side_two.pokemon.pkmn[1].maxhp = 500;
    state.side_two.pokemon.pkmn[1].hp = 500;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TERASTARSTORM,
            move_choice: MoveChoice::MoveTera(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ToggleTerastallized(ToggleTerastallizedInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 106,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 106,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_mid_turn_priority_change() {
    /*
    A more complicated interaction:
    - grassy terrain is up, meaning there is a +1 prio boost to a grassy glide on side_one
    - side_two switches into a pokemon with psychicsurge, getting rid of grassy terrain
    - side_one uses grassy glide, which should now have a priority of 0
    - side_two has a pokemon with higher speed than the grassy glide user, so it should move first
    */
    let mut state = State::default();
    state.terrain.terrain_type = Terrain::GRASSYTERRAIN;
    state.terrain.turns_remaining = 3;
    state.side_two.pokemon.pkmn[2].ability = Abilities::PSYCHICSURGE;
    state.side_one.pokemon.pkmn[0].speed = 100;
    state.side_two.pokemon.pkmn[0].speed = 105;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::GRASSYGLIDE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::NONE,
            move_choice: MoveChoice::Switch(PokemonIndex::P2),
        },
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Switch(SwitchInstruction {
                side_ref: SideReference::SideTwo,
                slot_ref: SlotReference::SlotB,
                previous_index: PokemonIndex::P1,
                next_index: PokemonIndex::P2,
            }),
            Instruction::ChangeTerrain(ChangeTerrain {
                previous_terrain: Terrain::GRASSYTERRAIN,
                new_terrain: Terrain::PSYCHICTERRAIN,
                previous_terrain_turns_remaining: 3,
                new_terrain_turns_remaining: 5,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo, // grassy glide hits side_two last
                pokemon_index: PokemonIndex::P0,
                damage_amount: 44,
            }),
            DecrementTerrainTurnsRemaining,
        ],
    }];

    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_lifedew_heals_both_pkmn_on_your_side() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].hp = 50;
    state.side_one.pokemon.pkmn[1].hp = 50;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::LIFEDEW,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Heal(HealInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                heal_amount: 25,
            }),
            Instruction::Heal(HealInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P1,
                heal_amount: 25,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_lifedew_does_not_overheal() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].hp = 50;
    state.side_one.pokemon.pkmn[1].hp = 80;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::LIFEDEW,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Heal(HealInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                heal_amount: 25,
            }),
            Instruction::Heal(HealInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P1,
                heal_amount: 20,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_lifedew_heals_single_ally() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].hp = 50;
    state.side_one.pokemon.pkmn[1].hp = 100;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::LIFEDEW,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Heal(HealInstruction {
            side_ref: SideReference::SideOne,
            pokemon_index: PokemonIndex::P0,
            heal_amount: 25,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_junglehealing_heals_and_removes_status() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].hp = 50;
    state.side_one.pokemon.pkmn[0].status = PokemonStatus::BURN;
    state.side_one.pokemon.pkmn[1].hp = 50;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::JUNGLEHEALING,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ChangeStatus(ChangeStatusInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                old_status: PokemonStatus::BURN,
                new_status: PokemonStatus::NONE,
            }),
            Instruction::Heal(HealInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                heal_amount: 25,
            }),
            Instruction::Heal(HealInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P1,
                heal_amount: 25,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_teraform_zero_removes_weather() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].tera_type = PokemonType::STELLAR;
    state.side_one.pokemon.pkmn[0].id = PokemonName::TERAPAGOSTERASTAL;
    state.side_one.pokemon.pkmn[0].ability = Abilities::TERASHELL;
    state.weather.weather_type = Weather::SUN;
    state.weather.turns_remaining = 5;
    state.terrain.terrain_type = Terrain::ELECTRICTERRAIN;
    state.terrain.turns_remaining = 3;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::MoveTera(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ToggleTerastallized(ToggleTerastallizedInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
            }),
            Instruction::FormeChange(FormeChangeInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                name_change: PokemonName::TERAPAGOSSTELLAR as i16
                    - PokemonName::TERAPAGOSTERASTAL as i16,
            }),
            Instruction::ChangeAttack(ChangeStatInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                amount: 167,
            }),
            Instruction::ChangeDefense(ChangeStatInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                amount: 177,
            }),
            Instruction::ChangeSpecialAttack(ChangeStatInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                amount: 217,
            }),
            Instruction::ChangeSpecialDefense(ChangeStatInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                amount: 177,
            }),
            Instruction::ChangeSpeed(ChangeStatInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                amount: 127,
            }),
            Instruction::ChangeAbility(ChangeAbilityInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                ability_change: Abilities::TERAFORMZERO as i16 - Abilities::TERASHELL as i16,
            }),
            Instruction::ChangeWeather(ChangeWeather {
                new_weather: Weather::NONE,
                new_weather_turns_remaining: 0,
                previous_weather: Weather::SUN,
                previous_weather_turns_remaining: 5,
            }),
            Instruction::ChangeTerrain(ChangeTerrain {
                new_terrain: Terrain::NONE,
                new_terrain_turns_remaining: 0,
                previous_terrain: Terrain::ELECTRICTERRAIN,
                previous_terrain_turns_remaining: 3,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 100,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_terapagos_terastal_formechange_with_starstorm() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].id = PokemonName::TERAPAGOSTERASTAL;
    state.side_one.pokemon.pkmn[0].ability = Abilities::TERASHELL;
    state.side_one.pokemon.pkmn[0].tera_type = PokemonType::STELLAR;
    state.side_two.pokemon.pkmn[0].maxhp = 500;
    state.side_two.pokemon.pkmn[0].hp = 500;
    state.side_two.pokemon.pkmn[1].maxhp = 500;
    state.side_two.pokemon.pkmn[1].hp = 500;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TERASTARSTORM,
            move_choice: MoveChoice::MoveTera(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ToggleTerastallized(ToggleTerastallizedInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
            }),
            Instruction::FormeChange(FormeChangeInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                name_change: PokemonName::TERAPAGOSSTELLAR as i16
                    - PokemonName::TERAPAGOSTERASTAL as i16,
            }),
            Instruction::ChangeAttack(ChangeStatInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                amount: 167,
            }),
            Instruction::ChangeDefense(ChangeStatInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                amount: 177,
            }),
            Instruction::ChangeSpecialAttack(ChangeStatInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                amount: 217,
            }),
            Instruction::ChangeSpecialDefense(ChangeStatInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                amount: 177,
            }),
            Instruction::ChangeSpeed(ChangeStatInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                amount: 127,
            }),
            Instruction::ChangeAbility(ChangeAbilityInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                ability_change: Abilities::TERAFORMZERO as i16 - Abilities::TERASHELL as i16,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 333,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 333,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_tera_starstorm_stellar_does_more_damage_to_terastallized_target() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].id = PokemonName::TERAPAGOSTERASTAL;
    state.side_one.pokemon.pkmn[0].ability = Abilities::TERASHELL;
    state.side_one.pokemon.pkmn[0].tera_type = PokemonType::STELLAR;
    state.side_two.pokemon.pkmn[0].maxhp = 500;
    state.side_two.pokemon.pkmn[0].hp = 500;
    state.side_two.pokemon.pkmn[0].terastallized = true;
    state.side_two.pokemon.pkmn[1].maxhp = 500;
    state.side_two.pokemon.pkmn[1].hp = 500;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TERASTARSTORM,
            move_choice: MoveChoice::MoveTera(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ToggleTerastallized(ToggleTerastallizedInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
            }),
            Instruction::FormeChange(FormeChangeInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                name_change: PokemonName::TERAPAGOSSTELLAR as i16
                    - PokemonName::TERAPAGOSTERASTAL as i16,
            }),
            Instruction::ChangeAttack(ChangeStatInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                amount: 167,
            }),
            Instruction::ChangeDefense(ChangeStatInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                amount: 177,
            }),
            Instruction::ChangeSpecialAttack(ChangeStatInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                amount: 217,
            }),
            Instruction::ChangeSpecialDefense(ChangeStatInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                amount: 177,
            }),
            Instruction::ChangeSpeed(ChangeStatInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                amount: 127,
            }),
            Instruction::ChangeAbility(ChangeAbilityInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                ability_change: Abilities::TERAFORMZERO as i16 - Abilities::TERASHELL as i16,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 500,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 333,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_terashell_halves_normal_effectiveness_damage() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].ability = Abilities::TERASHELL;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 24,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_terashell_quarters_double_effectiveness_damage() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].types.0 = PokemonType::FIGHTING;
    state.side_two.pokemon.pkmn[0].ability = Abilities::TERASHELL;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::VACUUMWAVE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideTwo,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 24, // still does 24 even though its super effective and would do 98
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_clearamulet_blocks_intimidate() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[2].ability = Abilities::INTIMIDATE;
    state.side_two.pokemon.pkmn[0].item = Items::CLEARAMULET;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::NONE,
            move_choice: MoveChoice::Switch(PokemonIndex::P2),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Switch(SwitchInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                previous_index: PokemonIndex::P0,
                next_index: PokemonIndex::P2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideTwo,
                slot_ref: SlotReference::SlotB,
                stat: PokemonBoostableStat::Attack,
                amount: -1,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_trickroom_inverts_speed_order() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].speed = 100;
    state.side_one.pokemon.pkmn[0].hp = 5;
    state.side_two.pokemon.pkmn[0].speed = 50;
    state.trick_room.active = true;
    state.trick_room.turns_remaining = 5;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 5,
            }),
            DecrementTrickRoomTurnsRemaining,
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_priority_bypasses_trickroom() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].speed = 100;
    state.side_one.pokemon.pkmn[0].hp = 5;
    state.side_two.pokemon.pkmn[0].speed = 50;
    state.trick_room.active = true;
    state.trick_room.turns_remaining = 5;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::VACUUMWAVE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 64,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 5,
            }),
            DecrementTrickRoomTurnsRemaining,
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_faster_disable_blocking_move_being_reused() {
    let mut state = State::default();
    state.side_one.slot_a.last_used_move = LastUsedMove::Move(PokemonMoveIndex::M0);

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::DISABLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::ApplyVolatileStatus(
            ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::DISABLE,
            },
        )],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_actual_speed_tie() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].speed = 100;
    state.side_two.pokemon.pkmn[0].speed = 100;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_parting_shot_into_clear_amulet_interaction() {
    let mut state = State::default();
    state.side_two.pokemon.pkmn[0].item = Items::CLEARAMULET;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::PARTINGSHOT,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_fakeout_into_fakeout_sets_last_used_move() {
    let mut state = State::default();
    state.use_last_used_move = true;
    state.side_one.slot_a.last_used_move = LastUsedMove::Switch(PokemonIndex::P0);
    state.side_two.slot_a.last_used_move = LastUsedMove::Switch(PokemonIndex::P0);

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::FAKEOUT,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::FAKEOUT,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::SetLastUsedMove(SetLastUsedMoveInstruction {
                side_ref: SideReference::SideTwo,
                slot_ref: SlotReference::SlotA,
                last_used_move: LastUsedMove::Move(PokemonMoveIndex::M0),
                previous_last_used_move: LastUsedMove::Switch(PokemonIndex::P0),
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 48,
            }),
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::FLINCH,
            }),
            Instruction::SetLastUsedMove(SetLastUsedMoveInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                last_used_move: LastUsedMove::None,
                previous_last_used_move: LastUsedMove::Switch(PokemonIndex::P0),
            }),
            Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::FLINCH,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_being_hit_with_ragefist_increases_ragefist_damage() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].types.0 = PokemonType::FIGHTING;
    state.side_two.pokemon.pkmn[0].types.0 = PokemonType::NORMAL;
    state.side_one.pokemon.pkmn[0].speed = 150;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::RAGEFIST,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 32,
            }),
            Instruction::IncrementTimesAttacked(IncrementTimesAttackedInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 79,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_basic_end_of_turn_gets_triggered() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::PROTECT,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::SPLASH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::SPLASH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::SPLASH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::PROTECT,
            }),
            Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::PROTECT,
            }),
            Instruction::ChangeVolatileStatusDuration(ChangeVolatileStatusDurationInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::PROTECT,
                amount: 1,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_switches_to_replace_fainted_pkmn_do_not_trigger_end_of_turn() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].hp = 0;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::PROTECT,
            move_choice: MoveChoice::Switch(PokemonIndex::P2),
        },
        TestMoveChoice {
            choice: Choices::SPLASH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::SPLASH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::SPLASH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: false,
        percentage: 100.0,
        instruction_list: vec![Instruction::Switch(SwitchInstruction {
            side_ref: SideReference::SideOne,
            slot_ref: SlotReference::SlotA,
            previous_index: PokemonIndex::P0,
            next_index: PokemonIndex::P2,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_pivot_move_does_not_trigger_end_of_turn() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].speed = 150;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::UTURN,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::SPLASH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::SPLASH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::SPLASH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: false,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 55,
            }),
            Instruction::ToggleForceSwitch(ToggleForceSwitchInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
            }),
            Instruction::SetSwitchOutMove(SetSecondMoveSwitchOutMoveInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                new_choice: MoveChoice::Move(
                    SlotReference::SlotA,
                    SideReference::SideTwo,
                    PokemonMoveIndex::M0,
                ),
                previous_choice: MoveChoice::None,
            }),
            Instruction::SetSwitchOutMove(SetSecondMoveSwitchOutMoveInstruction {
                side_ref: SideReference::SideTwo,
                slot_ref: SlotReference::SlotA,
                new_choice: MoveChoice::Move(
                    SlotReference::SlotA,
                    SideReference::SideTwo,
                    PokemonMoveIndex::M0,
                ),
                previous_choice: MoveChoice::None,
            }),
            Instruction::SetSwitchOutMove(SetSecondMoveSwitchOutMoveInstruction {
                side_ref: SideReference::SideTwo,
                slot_ref: SlotReference::SlotB,
                new_choice: MoveChoice::Move(
                    SlotReference::SlotA,
                    SideReference::SideTwo,
                    PokemonMoveIndex::M0,
                ),
                previous_choice: MoveChoice::None,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_switching_from_pivot_sets_end_of_turn() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].speed = 150;
    state.side_one.slot_a.force_switch = true;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::NONE,
            move_choice: MoveChoice::Switch(PokemonIndex::P2),
        },
        TestMoveChoice {
            choice: Choices::SPLASH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::SPLASH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::SPLASH,
            move_choice: MoveChoice::None,
        },
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ToggleForceSwitch(ToggleForceSwitchInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
            }),
            Instruction::Switch(SwitchInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                previous_index: PokemonIndex::P0,
                next_index: PokemonIndex::P2,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_multiple_none_moves_does_not_set_end_of_turn() {
    let mut state = State::default();

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::PROTECT,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::NONE,
            move_choice: MoveChoice::None,
        },
        TestMoveChoice {
            choice: Choices::SPLASH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::NONE,
            move_choice: MoveChoice::None,
        },
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::PROTECT,
            }),
            Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::PROTECT,
            }),
            Instruction::ChangeVolatileStatusDuration(ChangeVolatileStatusDurationInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::PROTECT,
                amount: 1,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_fainted_followme_does_not_redirect() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].speed = 150;
    state.side_one.pokemon.pkmn[1].speed = 150;
    state.side_two.pokemon.pkmn[0].hp = 5;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::FOLLOWME,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice {
            choice: Choices::SPLASH,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideTwo,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::FOLLOWME,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P0,
                damage_amount: 5,
            }),
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 48,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_commander_switch_in_when_dondozo_is_on_the_field() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].id = PokemonName::DONDOZO;
    state.side_one.pokemon.pkmn[2].id = PokemonName::TATSUGIRI;
    state.side_one.pokemon.pkmn[2].ability = Abilities::COMMANDER;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::NONE,
            move_choice: MoveChoice::Switch(PokemonIndex::P2),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Switch(SwitchInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                previous_index: PokemonIndex::P1,
                next_index: PokemonIndex::P2,
            }),
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::COMMANDED,
            }),
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                volatile_status: PokemonVolatileStatus::COMMANDING,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::Attack,
                amount: 2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::Defense,
                amount: 2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::SpecialAttack,
                amount: 2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::SpecialDefense,
                amount: 2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::Speed,
                amount: 2,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_neutralizinggas_prevents_commander_for_tatsu_switchin() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].id = PokemonName::DONDOZO;
    state.side_one.pokemon.pkmn[2].id = PokemonName::TATSUGIRI;
    state.side_one.pokemon.pkmn[2].ability = Abilities::COMMANDER;
    state.side_two.pokemon.pkmn[0].ability = Abilities::NEUTRALIZINGGAS;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::NONE,
            move_choice: MoveChoice::Switch(PokemonIndex::P2),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Switch(SwitchInstruction {
            side_ref: SideReference::SideOne,
            slot_ref: SlotReference::SlotB,
            previous_index: PokemonIndex::P1,
            next_index: PokemonIndex::P2,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_commander_switch_in_when_one_stat_cannot_be_boosted_twice() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].id = PokemonName::DONDOZO;
    state.side_one.slot_a.attack_boost = 5;
    state.side_one.pokemon.pkmn[2].id = PokemonName::TATSUGIRI;
    state.side_one.pokemon.pkmn[2].ability = Abilities::COMMANDER;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Switch(PokemonIndex::P2),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Switch(SwitchInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                previous_index: PokemonIndex::P1,
                next_index: PokemonIndex::P2,
            }),
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::COMMANDED,
            }),
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                volatile_status: PokemonVolatileStatus::COMMANDING,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::Attack,
                amount: 1,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::Defense,
                amount: 2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::SpecialAttack,
                amount: 2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::SpecialDefense,
                amount: 2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                stat: PokemonBoostableStat::Speed,
                amount: 2,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_dondozo_switch_in_when_tatsugiri_is_on_the_field() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::COMMANDER;
    state.side_one.pokemon.pkmn[2].id = PokemonName::DONDOZO;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Switch(PokemonIndex::P2),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Switch(SwitchInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                previous_index: PokemonIndex::P1,
                next_index: PokemonIndex::P2,
            }),
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                volatile_status: PokemonVolatileStatus::COMMANDED,
            }),
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::COMMANDING,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                stat: PokemonBoostableStat::Attack,
                amount: 2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                stat: PokemonBoostableStat::Defense,
                amount: 2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                stat: PokemonBoostableStat::SpecialAttack,
                amount: 2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                stat: PokemonBoostableStat::SpecialDefense,
                amount: 2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                stat: PokemonBoostableStat::Speed,
                amount: 2,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_neutralizinggas_prevents_commander_when_dondozo_switches_in() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::COMMANDER;
    state.side_one.pokemon.pkmn[2].id = PokemonName::DONDOZO;
    state.side_two.pokemon.pkmn[0].ability = Abilities::NEUTRALIZINGGAS;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::NONE,
            move_choice: MoveChoice::Switch(PokemonIndex::P2),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Switch(SwitchInstruction {
            side_ref: SideReference::SideOne,
            slot_ref: SlotReference::SlotB,
            previous_index: PokemonIndex::P1,
            next_index: PokemonIndex::P2,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_neutralizinggas_nullified_by_abilityshield_for_commander() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::COMMANDER;
    state.side_one.pokemon.pkmn[0].item = Items::ABILITYSHIELD;
    state.side_one.pokemon.pkmn[2].id = PokemonName::DONDOZO;
    state.side_two.pokemon.pkmn[0].ability = Abilities::NEUTRALIZINGGAS;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::NONE,
            move_choice: MoveChoice::Switch(PokemonIndex::P2),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Switch(SwitchInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                previous_index: PokemonIndex::P1,
                next_index: PokemonIndex::P2,
            }),
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                volatile_status: PokemonVolatileStatus::COMMANDED,
            }),
            Instruction::ApplyVolatileStatus(ApplyVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::COMMANDING,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                stat: PokemonBoostableStat::Attack,
                amount: 2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                stat: PokemonBoostableStat::Defense,
                amount: 2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                stat: PokemonBoostableStat::SpecialAttack,
                amount: 2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                stat: PokemonBoostableStat::SpecialDefense,
                amount: 2,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                stat: PokemonBoostableStat::Speed,
                amount: 2,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_move_executed_on_commanding_fails() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::COMMANDER;
    state
        .side_one
        .slot_a
        .volatile_statuses
        .insert(PokemonVolatileStatus::COMMANDING);

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_commanding_pokemon_cannot_use_move() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::COMMANDER;
    state
        .side_one
        .slot_a
        .volatile_statuses
        .insert(PokemonVolatileStatus::COMMANDING);

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_fainting_commanded_pokemon_removes_commanding_from_ally() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::COMMANDER;
    state.side_one.pokemon.pkmn[1].id = PokemonName::DONDOZO;
    state.side_one.pokemon.pkmn[1].hp = 1;
    state
        .side_one
        .slot_a
        .volatile_statuses
        .insert(PokemonVolatileStatus::COMMANDING);
    state
        .side_one
        .slot_b
        .volatile_statuses
        .insert(PokemonVolatileStatus::COMMANDED);

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::TACKLE,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideOne,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 1,
            }),
            Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::COMMANDING,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_orderup_boost_with_commanded_pkmn() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::COMMANDER;
    state.side_one.pokemon.pkmn[0].id = PokemonName::TATSUGIRI;
    state.side_one.pokemon.pkmn[1].id = PokemonName::DONDOZO;
    state
        .side_one
        .slot_a
        .volatile_statuses
        .insert(PokemonVolatileStatus::COMMANDING);
    state
        .side_one
        .slot_b
        .volatile_statuses
        .insert(PokemonVolatileStatus::COMMANDED);

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice::default(),
        TestMoveChoice {
            choice: Choices::ORDERUP,
            move_choice: MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::Damage(DamageInstruction {
                side_ref: SideReference::SideTwo,
                pokemon_index: PokemonIndex::P1,
                damage_amount: 63,
            }),
            Instruction::Boost(BoostInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotB,
                stat: PokemonBoostableStat::Attack,
                amount: 1,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_commanding_pkmn_takes_damage_from_poison() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].ability = Abilities::COMMANDER;
    state.side_one.pokemon.pkmn[0].id = PokemonName::TATSUGIRI;
    state.side_one.pokemon.pkmn[0].status = PokemonStatus::POISON;
    state.side_one.pokemon.pkmn[1].id = PokemonName::DONDOZO;
    state
        .side_one
        .slot_a
        .volatile_statuses
        .insert(PokemonVolatileStatus::COMMANDING);
    state
        .side_one
        .slot_b
        .volatile_statuses
        .insert(PokemonVolatileStatus::COMMANDED);

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![Instruction::Damage(DamageInstruction {
            side_ref: SideReference::SideOne,
            pokemon_index: PokemonIndex::P0,
            damage_amount: 12,
        })],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_fainted_commanding_pokemon_switching_out() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].hp = 0;
    state.side_one.pokemon.pkmn[0].ability = Abilities::COMMANDER;
    state.side_one.pokemon.pkmn[0].base_ability = Abilities::COMMANDER;
    state.side_one.pokemon.pkmn[0].id = PokemonName::TATSUGIRI;
    state.side_one.pokemon.pkmn[0].status = PokemonStatus::POISON;
    state.side_one.pokemon.pkmn[1].id = PokemonName::DONDOZO;
    state
        .side_one
        .slot_a
        .volatile_statuses
        .insert(PokemonVolatileStatus::COMMANDING);
    state
        .side_one
        .slot_b
        .volatile_statuses
        .insert(PokemonVolatileStatus::COMMANDED);

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::NONE,
            move_choice: MoveChoice::Switch(PokemonIndex::P2),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: false,
        percentage: 100.0,
        instruction_list: vec![
            Instruction::RemoveVolatileStatus(RemoveVolatileStatusInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                volatile_status: PokemonVolatileStatus::COMMANDING,
            }),
            Instruction::Switch(SwitchInstruction {
                side_ref: SideReference::SideOne,
                slot_ref: SlotReference::SlotA,
                previous_index: PokemonIndex::P0,
                next_index: PokemonIndex::P2,
            }),
        ],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}

#[test]
fn test_abilityshield_prevents_neutralizinggas() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[0].item = Items::ABILITYSHIELD;
    state.side_one.pokemon.pkmn[0].ability = Abilities::GALVANIZE;
    state.side_one.pokemon.pkmn[1].ability = Abilities::NEUTRALIZINGGAS;
    state.side_two.pokemon.pkmn[0].types.0 = PokemonType::GROUND;

    let vec_of_instructions = set_moves_on_pkmn_and_call_generate_instructions(
        &mut state,
        TestMoveChoice {
            choice: Choices::TACKLE, // should be rendered ineffective against the ground type
            move_choice: MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        },
        TestMoveChoice::default(),
        TestMoveChoice::default(),
        TestMoveChoice::default(),
    );

    let expected_instructions = vec![StateInstructions {
        end_of_turn_triggered: true,
        percentage: 100.0,
        instruction_list: vec![],
    }];
    assert_eq!(expected_instructions, vec_of_instructions);
}
