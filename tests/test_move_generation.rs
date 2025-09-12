use poke_engine::choices::{Choices, MOVES};
use poke_engine::engine::abilities::Abilities;
use poke_engine::engine::state::{MoveChoice, MoveOptions, PokemonVolatileStatus};
use poke_engine::state::{
    LastUsedMove, Move, PokemonIndex, PokemonMoveIndex, PokemonMoves, SideReference, SlotReference,
    State,
};

fn disable_all_moves(pokemon_moves: &mut PokemonMoves) {
    pokemon_moves.m0.disabled = true;
    pokemon_moves.m1.disabled = true;
    pokemon_moves.m2.disabled = true;
    pokemon_moves.m3.disabled = true;
}

#[test]
fn test_basic_move_generation() {
    let mut state = State::default();
    // remove tera and only have 4 alive pkmn
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;

    // only have 1 move per pkmn
    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::NONE,
        disabled: false,
        pp: 32,
        choice: Default::default(),
    };
    state.side_one.pokemon.pkmn[1].moves.m0 = Move {
        id: Choices::NONE,
        disabled: false,
        pp: 32,
        choice: Default::default(),
    };
    state.side_two.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::NONE,
        disabled: false,
        pp: 32,
        choice: Default::default(),
    };
    state.side_two.pokemon.pkmn[1].moves.m0 = Move {
        id: Choices::NONE,
        disabled: false,
        pp: 32,
        choice: Default::default(),
    };
    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);
    // 15 moves choices per side:
    // 3 targets for each move ^ 2 pokemon = 9 choices where both active use a move
    // 3 targets for a move with 1 switch  = 6 choices where one active uses a move and the other switches
    // 9 + 6 = 15 total choices per side
    assert_eq!(8, move_options.side_one_combined_options.len());
    assert_eq!(8, move_options.side_two_combined_options.len());
}

#[test]
fn test_spread_move_does_not_result_in_multiple_move_choices() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;

    // only have 1 move per pkmn
    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::SURF,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::SURF).unwrap().clone(),
    };
    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);
    assert_eq!(1, move_options.side_one_combined_options.len());
}

#[test]
fn test_spikes_only_has_one_target() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;

    // only have 1 move per pkmn
    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::SPIKES,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::SURF).unwrap().clone(),
    };
    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);
    assert_eq!(1, move_options.side_one_combined_options.len());
}

#[test]
fn test_cannot_target_fainted_pkmn() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[1].hp = 0;

    // only have 1 move per pkmn
    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::NONE,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::NONE).unwrap().clone(),
    };
    state.side_one.pokemon.pkmn[1].moves.m0 = Move {
        id: Choices::NONE,
        disabled: false,
        pp: 32,
        choice: Default::default(),
    };
    state.side_two.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::NONE,
        disabled: false,
        pp: 32,
        choice: Default::default(),
    };
    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);
    // 2 targets for each pkmn on side 1 with no switches is 4 total move choice pairs
    assert_eq!(1, move_options.side_one_combined_options.len());
}

#[test]
fn test_helping_hand_can_only_target_ally() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[1].hp = 0;

    // only have 1 move per pkmn
    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::HELPINGHAND,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::HELPINGHAND).unwrap().clone(),
    };
    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);
    // 2 targets for each pkmn on side 1 with no switches is 4 total move choice pairs

    // Helping Hand can only target the ally, so there should be only one option for s1 slot a
    let expected_s1_options = vec![(
        MoveChoice::Move(
            SlotReference::SlotB,
            SideReference::SideOne,
            PokemonMoveIndex::M0,
        ),
        MoveChoice::None,
    )];

    assert_eq!(expected_s1_options, move_options.side_one_combined_options);
}

#[test]
fn test_self_boosting_move_only_has_1_target() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[1].hp = 0;

    // only have 1 move per pkmn
    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::QUIVERDANCE,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::QUIVERDANCE).unwrap().clone(),
    };
    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);

    let expected_s1_options = vec![(
        MoveChoice::Move(
            SlotReference::SlotA,
            SideReference::SideOne,
            PokemonMoveIndex::M0,
        ),
        MoveChoice::None,
    )];

    assert_eq!(expected_s1_options, move_options.side_one_combined_options);
}

#[test]
fn test_disabled_pkmn_cannot_use_last_used_move() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[1].hp = 0;

    state
        .side_one
        .slot_a
        .volatile_statuses
        .insert(PokemonVolatileStatus::DISABLE);

    // only have 1 move per pkmn
    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);
    state.side_one.slot_a.last_used_move = LastUsedMove::Move(PokemonMoveIndex::M0);
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::QUIVERDANCE,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::QUIVERDANCE).unwrap().clone(),
    };
    state.side_one.pokemon.pkmn[0].moves.m1 = Move {
        id: Choices::TACKLE,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::QUIVERDANCE).unwrap().clone(),
    };
    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);

    let expected_s1_options = vec![(
        MoveChoice::Move(
            SlotReference::SlotA,
            SideReference::SideOne,
            PokemonMoveIndex::M1,
        ),
        MoveChoice::None,
    )];

    assert_eq!(expected_s1_options, move_options.side_one_combined_options);
}

#[test]
fn test_disabled_with_one_moves_gives_no_move() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[1].hp = 0;

    state
        .side_one
        .slot_a
        .volatile_statuses
        .insert(PokemonVolatileStatus::DISABLE);

    // only have 1 move per pkmn
    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);
    state.side_one.slot_a.last_used_move = LastUsedMove::Move(PokemonMoveIndex::M0);
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::QUIVERDANCE,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::QUIVERDANCE).unwrap().clone(),
    };
    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);

    let expected_s1_options = vec![(MoveChoice::None, MoveChoice::None)];

    assert_eq!(expected_s1_options, move_options.side_one_combined_options);
}

#[test]
fn test_encored_and_disabled_means_no_moves() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[1].hp = 0;

    state
        .side_one
        .slot_a
        .volatile_statuses
        .insert(PokemonVolatileStatus::DISABLE);
    state
        .side_one
        .slot_a
        .volatile_statuses
        .insert(PokemonVolatileStatus::ENCORE);

    // only have 1 move per pkmn
    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);
    state.side_one.slot_a.last_used_move = LastUsedMove::Move(PokemonMoveIndex::M0);
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::QUIVERDANCE,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::QUIVERDANCE).unwrap().clone(),
    };
    state.side_one.pokemon.pkmn[0].moves.m1 = Move {
        id: Choices::LEER,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::LEER).unwrap().clone(),
    };
    state.side_one.pokemon.pkmn[0].moves.m2 = Move {
        id: Choices::PROTECT,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::PROTECT).unwrap().clone(),
    };
    state.side_one.pokemon.pkmn[0].moves.m3 = Move {
        id: Choices::SPLASH,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::SPLASH).unwrap().clone(),
    };
    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);

    let expected_s1_options = vec![(MoveChoice::None, MoveChoice::None)];

    assert_eq!(expected_s1_options, move_options.side_one_combined_options);
}

#[test]
fn test_pollenpuff_can_target_ally_and_opponents() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;

    // only have 1 move per pkmn
    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);
    state.side_one.slot_a.last_used_move = LastUsedMove::Move(PokemonMoveIndex::M0);
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::POLLENPUFF,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::POLLENPUFF).unwrap().clone(),
    };
    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);

    let expected_s1_options = vec![
        (
            MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
            MoveChoice::None,
        ),
        (
            MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
            MoveChoice::None,
        ),
        (
            MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
            MoveChoice::None,
        ),
    ];

    assert_eq!(expected_s1_options, move_options.side_one_combined_options);
}

#[test]
fn test_beatup_can_target_ally_with_ragefist_and_opponents() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;

    // only have 1 move per pkmn
    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);
    state.side_one.slot_a.last_used_move = LastUsedMove::Move(PokemonMoveIndex::M0);
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::BEATUP,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::BEATUP).unwrap().clone(),
    };
    state.side_one.pokemon.pkmn[1].moves.m0 = Move {
        id: Choices::RAGEFIST,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::RAGEFIST).unwrap().clone(),
    };
    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);

    let expected_s1_options = vec![
        (
            MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
            MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        ),
        (
            MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
            MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        ),
        (
            MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
            MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        ),
        (
            MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
            MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        ),
        (
            MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
            MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        ),
        (
            MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
            MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
        ),
    ];

    assert_eq!(expected_s1_options, move_options.side_one_combined_options);
}

#[test]
fn test_beatup_can_target_ally_with_stamina_and_opponents() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;

    // only have 1 move per pkmn
    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);
    state.side_one.slot_a.last_used_move = LastUsedMove::Move(PokemonMoveIndex::M0);
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::BEATUP,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::BEATUP).unwrap().clone(),
    };
    state.side_one.pokemon.pkmn[1].ability = Abilities::STAMINA;
    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);

    let expected_s1_options = vec![
        (
            MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideOne,
                PokemonMoveIndex::M0,
            ),
            MoveChoice::None,
        ),
        (
            MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
            MoveChoice::None,
        ),
        (
            MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
            MoveChoice::None,
        ),
    ];

    assert_eq!(expected_s1_options, move_options.side_one_combined_options);
}

#[test]
fn test_decorate_can_target_only_ally() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;

    // only have 1 move per pkmn
    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);
    state.side_one.slot_a.last_used_move = LastUsedMove::Move(PokemonMoveIndex::M0);
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::DECORATE,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::DECORATE).unwrap().clone(),
    };
    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);

    let expected_s1_options = vec![(
        MoveChoice::Move(
            SlotReference::SlotB,
            SideReference::SideOne,
            PokemonMoveIndex::M0,
        ),
        MoveChoice::None,
    )];

    assert_eq!(expected_s1_options, move_options.side_one_combined_options);
}

#[test]
fn test_coaching_can_target_only_ally() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;

    // only have 1 move per pkmn
    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);
    state.side_one.slot_a.last_used_move = LastUsedMove::Move(PokemonMoveIndex::M0);
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::COACHING,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::COACHING).unwrap().clone(),
    };
    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);

    let expected_s1_options = vec![(
        MoveChoice::Move(
            SlotReference::SlotB,
            SideReference::SideOne,
            PokemonMoveIndex::M0,
        ),
        MoveChoice::None,
    )];

    assert_eq!(expected_s1_options, move_options.side_one_combined_options);
}

#[test]
fn test_fakeout_is_an_option_when_just_switching_in() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;

    // only have 1 move per pkmn
    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);
    state.side_one.slot_a.last_used_move = LastUsedMove::Switch(PokemonIndex::P0);
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::FAKEOUT,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::FAKEOUT).unwrap().clone(),
    };
    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);

    let expected_s1_options = vec![
        (
            MoveChoice::Move(
                SlotReference::SlotA,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
            MoveChoice::None,
        ),
        (
            MoveChoice::Move(
                SlotReference::SlotB,
                SideReference::SideTwo,
                PokemonMoveIndex::M0,
            ),
            MoveChoice::None,
        ),
    ];

    assert_eq!(expected_s1_options, move_options.side_one_combined_options);
}

#[test]
fn test_fakeout_is_not_an_option_when_already_on_the_field() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;

    // only have 1 move per pkmn
    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);
    state.side_one.slot_a.last_used_move = LastUsedMove::Move(PokemonMoveIndex::M0);
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::FAKEOUT,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::FAKEOUT).unwrap().clone(),
    };
    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);

    let expected_s1_options = vec![(MoveChoice::None, MoveChoice::None)];

    assert_eq!(expected_s1_options, move_options.side_one_combined_options);
}

#[test]
fn test_commanding_pkmn_cannot_select_any_moves() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;

    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);

    // only have 1 move for side_one.pokemon.pkmn[0]
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::TACKLE,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::TACKLE).unwrap().clone(),
    };

    state
        .side_one
        .slot_a
        .volatile_statuses
        .insert(PokemonVolatileStatus::COMMANDING);

    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);

    // cannot select any moves when commanding
    let expected_s1_options = vec![(MoveChoice::None, MoveChoice::None)];
    assert_eq!(expected_s1_options, move_options.side_one_combined_options);
}

#[test]
fn test_commanding_pkmn_cannot_switch() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 100;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;

    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);

    // only have 1 move for side_one.pokemon.pkmn[0]
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::TACKLE,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::TACKLE).unwrap().clone(),
    };
    state.side_one.pokemon.pkmn[1].moves.m0 = Move {
        id: Choices::TACKLE,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::TACKLE).unwrap().clone(),
    };

    state
        .side_one
        .slot_a
        .volatile_statuses
        .insert(PokemonVolatileStatus::COMMANDING);

    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);

    // make sure none of the slot A options are anything but `none`
    for (move_choice, _) in move_options.side_one_combined_options.iter() {
        assert_eq!(MoveChoice::None, *move_choice);
    }
}

#[test]
fn test_fainted_commanding_pkmn_must_switch() {
    let mut state = State::default();
    state.side_one.pokemon.pkmn[5].terastallized = true;
    state.side_one.pokemon.pkmn[5].hp = 0;
    state.side_one.pokemon.pkmn[4].hp = 0;
    state.side_one.pokemon.pkmn[3].hp = 0;
    state.side_one.pokemon.pkmn[2].hp = 100;
    state.side_one.pokemon.pkmn[0].hp = 0;
    state.side_two.pokemon.pkmn[5].terastallized = true;
    state.side_two.pokemon.pkmn[5].hp = 0;
    state.side_two.pokemon.pkmn[4].hp = 0;
    state.side_two.pokemon.pkmn[3].hp = 0;
    state.side_two.pokemon.pkmn[2].hp = 0;

    disable_all_moves(&mut state.side_one.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_one.pokemon.pkmn[1].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[0].moves);
    disable_all_moves(&mut state.side_two.pokemon.pkmn[1].moves);

    // only have 1 move for side_one.pokemon.pkmn[0]
    state.side_one.pokemon.pkmn[0].moves.m0 = Move {
        id: Choices::TACKLE,
        disabled: false,
        pp: 32,
        choice: MOVES.get(&Choices::TACKLE).unwrap().clone(),
    };

    state
        .side_one
        .slot_a
        .volatile_statuses
        .insert(PokemonVolatileStatus::COMMANDING);

    let mut move_options = MoveOptions::new();
    state.get_all_options(&mut move_options);

    // fainted commanding pokemon must switch
    let expected_s1_options = vec![(MoveChoice::Switch(PokemonIndex::P2), MoveChoice::None)];
    assert_eq!(expected_s1_options, move_options.side_one_combined_options);
}
