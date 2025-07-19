use pyo3::prelude::*;
use pyo3::{pyfunction, pymethods, pymodule, wrap_pyfunction, Bound, PyResult};
use std::collections::HashSet;

use poke_engine::choices::{Choices, MoveCategory, MOVES};
use poke_engine::engine::abilities::Abilities;
use poke_engine::engine::generate_instructions::{
    calculate_damage_rolls, generate_instructions_from_move_pair,
};
use poke_engine::engine::items::Items;
use poke_engine::engine::state::{MoveChoice, PokemonVolatileStatus, Terrain, Weather};
use poke_engine::instruction::{Instruction, StateInstructions};
use poke_engine::mcts::{perform_mcts, MctsResult, MctsSideResult};
use poke_engine::pokemon::PokemonName;
use poke_engine::state::{
    LastUsedMove, Move, Pokemon, PokemonIndex, PokemonMoves, PokemonNature, PokemonStatus,
    PokemonType, Side, SideConditions, SidePokemon, SideReference, SideSlot, SlotReference, State,
    StateTerrain, StateTrickRoom, StateWeather, VolatileStatusDurations,
};
use std::str::FromStr;
use std::time::Duration;

fn movechoice_to_string(
    side: &Side,
    move_choice: &MoveChoice,
    attacking_slot_ref: &SlotReference,
) -> String {
    match move_choice {
        MoveChoice::Switch(_) => {
            format!("switch {}", move_choice.to_string(side, attacking_slot_ref))
        }
        _ => move_choice.to_string(side, attacking_slot_ref),
    }
}

#[derive(Clone)]
#[pyclass(name = "State")]
pub struct PyState {
    pub state: State,
}

#[pymethods]
impl PyState {
    #[new]
    fn new(
        side_one: PySide,
        side_two: PySide,
        weather: String,
        weather_turns_remaining: i8,
        terrain: String,
        terrain_turns_remaining: i8,
        trick_room: bool,
        trick_room_turns_remaining: i8,
        team_preview: bool,
    ) -> Self {
        let mut state = State {
            side_one: side_one.create_side(),
            side_two: side_two.create_side(),
            weather: StateWeather {
                weather_type: Weather::from_str(&weather).unwrap(),
                turns_remaining: weather_turns_remaining,
            },
            terrain: StateTerrain {
                terrain_type: Terrain::from_str(&terrain).unwrap(),
                turns_remaining: terrain_turns_remaining,
            },
            trick_room: StateTrickRoom {
                active: trick_room,
                turns_remaining: trick_room_turns_remaining,
            },
            team_preview,
            use_damage_dealt: false,
            use_last_used_move: false,
        };
        state.set_conditional_mechanics();
        PyState { state }
    }

    fn apply_one_instruction(&mut self, instruction: PyInstruction) {
        self.state.apply_one_instruction(&instruction.instruction);
    }

    fn apply_instructions(&mut self, instructions: Vec<PyInstruction>) {
        for instruction in instructions {
            self.apply_one_instruction(instruction);
        }
    }

    fn to_string(&self) -> String {
        self.state.serialize()
    }
}

#[derive(Clone)]
#[pyclass(name = "Side")]
pub struct PySide {
    pub side: Side,
}

impl PySide {
    fn create_side(&self) -> Side {
        self.side.clone()
    }
}

#[pymethods]
impl PySide {
    #[new]
    fn new(
        mut pokemon: Vec<PyPokemon>,
        slot_a: PySideSlot,
        slot_b: PySideSlot,
        side_conditions: PySideConditions,
    ) -> Self {
        while pokemon.len() < 6 {
            pokemon.push(PyPokemon::create_fainted());
        }
        PySide {
            side: Side {
                pokemon: SidePokemon {
                    p0: pokemon[0].create_pokemon(),
                    p1: pokemon[1].create_pokemon(),
                    p2: pokemon[2].create_pokemon(),
                    p3: pokemon[3].create_pokemon(),
                    p4: pokemon[4].create_pokemon(),
                    p5: pokemon[5].create_pokemon(),
                },
                side_conditions: side_conditions.create_side_conditions(),
                slot_a: slot_a.create_side_slot(),
                slot_b: slot_b.create_side_slot(),
            },
        }
    }
}

#[derive(Clone)]
#[pyclass(name = "SideSlot")]
pub struct PySideSlot {
    pub slot: SideSlot,
}

impl PySideSlot {
    fn create_side_slot(&self) -> SideSlot {
        self.slot.clone()
    }
}

#[pymethods]
impl PySideSlot {
    #[new]
    fn new(
        active_index: String,
        baton_passing: bool,
        shed_tailing: bool,
        volatile_status_durations: PyVolatileStatusDurations,
        wish: (i8, i16),
        future_sight: (i8, String),
        force_switch: bool,
        force_trapped: bool,
        slow_uturn_move: bool,
        volatile_statuses: Vec<String>,
        substitute_health: i16,
        attack_boost: i8,
        defense_boost: i8,
        special_attack_boost: i8,
        special_defense_boost: i8,
        speed_boost: i8,
        accuracy_boost: i8,
        evasion_boost: i8,
        last_used_move: String,
        switch_out_move_second_saved_move: String,
    ) -> Self {
        let mut vs_hashset = HashSet::new();
        for vs in volatile_statuses {
            vs_hashset.insert(PokemonVolatileStatus::from_str(&vs).unwrap());
        }
        PySideSlot {
            slot: SideSlot {
                active_index: PokemonIndex::deserialize(&active_index),
                baton_passing,
                shed_tailing,
                wish,
                future_sight: (future_sight.0, PokemonIndex::deserialize(&future_sight.1)),
                force_switch,
                force_trapped,
                slow_uturn_move,
                volatile_statuses: vs_hashset,
                volatile_status_durations: volatile_status_durations
                    .create_volatile_status_durations(),
                substitute_health,
                attack_boost,
                defense_boost,
                special_attack_boost,
                special_defense_boost,
                speed_boost,
                accuracy_boost,
                evasion_boost,
                last_used_move: LastUsedMove::deserialize(&last_used_move),
                damage_dealt: Default::default(),
                switch_out_move_second_saved_move: MoveChoice::deserialize(
                    &switch_out_move_second_saved_move,
                ),
            },
        }
    }
}

#[derive(Clone)]
#[pyclass(name = "VolatileStatusDurations")]
pub struct PyVolatileStatusDurations {
    pub volatile_status_durations: VolatileStatusDurations,
}

impl PyVolatileStatusDurations {
    fn create_volatile_status_durations(&self) -> VolatileStatusDurations {
        self.volatile_status_durations.clone()
    }
}

#[pymethods]
impl PyVolatileStatusDurations {
    #[new]
    fn new(
        confusion: i8,
        encore: i8,
        lockedmove: i8,
        protect: i8,
        slowstart: i8,
        taunt: i8,
        yawn: i8,
    ) -> PyVolatileStatusDurations {
        PyVolatileStatusDurations {
            volatile_status_durations: VolatileStatusDurations {
                confusion,
                encore,
                lockedmove,
                protect,
                slowstart,
                taunt,
                yawn,
            },
        }
    }
}

#[derive(Clone)]
#[pyclass(name = "SideConditions")]
pub struct PySideConditions {
    pub side_conditions: SideConditions,
}

impl PySideConditions {
    fn create_side_conditions(&self) -> SideConditions {
        self.side_conditions.clone()
    }
}

#[pymethods]
impl PySideConditions {
    #[new]
    fn new(
        spikes: i8,
        toxic_spikes: i8,
        stealth_rock: i8,
        sticky_web: i8,
        tailwind: i8,
        lucky_chant: i8,
        lunar_dance: i8,
        reflect: i8,
        light_screen: i8,
        aurora_veil: i8,
        crafty_shield: i8,
        safeguard: i8,
        mist: i8,
        protect: i8,
        healing_wish: i8,
        mat_block: i8,
        quick_guard: i8,
        toxic_count: i8,
        wide_guard: i8,
    ) -> Self {
        PySideConditions {
            side_conditions: SideConditions {
                spikes,
                toxic_spikes,
                stealth_rock,
                sticky_web,
                tailwind,
                lucky_chant,
                lunar_dance,
                reflect,
                light_screen,
                aurora_veil,
                crafty_shield,
                safeguard,
                mist,
                protect,
                healing_wish,
                mat_block,
                quick_guard,
                toxic_count,
                wide_guard,
            },
        }
    }
}

#[derive(Clone)]
#[pyclass(name = "Pokemon")]
pub struct PyPokemon {
    pub pokemon: Pokemon,
}

impl PyPokemon {
    fn create_pokemon(&self) -> Pokemon {
        self.pokemon.clone()
    }
    fn create_fainted() -> PyPokemon {
        let mut pkmn = Pokemon::default();
        pkmn.hp = 0;
        PyPokemon { pokemon: pkmn }
    }
}

#[pymethods]
impl PyPokemon {
    #[new]
    fn new(
        id: String,
        level: i8,
        types: [String; 2],
        base_types: [String; 2],
        hp: i16,
        maxhp: i16,
        ability: String,
        base_ability: String,
        item: String,
        nature: String,
        evs: (u8, u8, u8, u8, u8, u8),
        attack: i16,
        defense: i16,
        special_attack: i16,
        special_defense: i16,
        speed: i16,
        status: String,
        rest_turns: i8,
        sleep_turns: i8,
        weight_kg: f32,
        mut moves: Vec<PyMove>,
        terastallized: bool,
        tera_type: String,
    ) -> Self {
        while moves.len() < 4 {
            moves.push(PyMove::create_empty_move());
        }
        PyPokemon {
            pokemon: Pokemon {
                id: PokemonName::from_str(&id).unwrap(),
                level,
                types: (
                    PokemonType::from_str(&types[0]).unwrap(),
                    PokemonType::from_str(&types[1]).unwrap(),
                ),
                base_types: (
                    PokemonType::from_str(&base_types[0]).unwrap(),
                    PokemonType::from_str(&base_types[1]).unwrap(),
                ),
                hp,
                maxhp,
                ability: Abilities::from_str(&ability).unwrap(),
                base_ability: Abilities::from_str(&base_ability).unwrap(),
                item: Items::from_str(&item).unwrap(),
                nature: PokemonNature::from_str(&nature).unwrap(),
                evs: (evs.0, evs.1, evs.2, evs.3, evs.4, evs.5),
                attack,
                defense,
                special_attack,
                special_defense,
                speed,
                status: PokemonStatus::from_str(&status).unwrap(),
                rest_turns,
                sleep_turns,
                weight_kg,
                moves: PokemonMoves {
                    m0: moves[0].create_move(),
                    m1: moves[1].create_move(),
                    m2: moves[2].create_move(),
                    m3: moves[3].create_move(),
                },
                terastallized,
                tera_type: PokemonType::from_str(&tera_type).unwrap(),
            },
        }
    }
}

#[derive(Clone)]
#[pyclass(name = "Move")]
pub struct PyMove {
    pub mv: Move,
}

impl PyMove {
    fn create_move(&self) -> Move {
        self.mv.clone()
    }
    fn create_empty_move() -> PyMove {
        let mut mv = Move::default();
        mv.disabled = true;
        mv.pp = 0;
        PyMove { mv }
    }
}

#[pymethods]
impl PyMove {
    #[new]
    fn new(id: String, pp: i8, disabled: bool) -> Self {
        let choice = Choices::from_str(&id).unwrap();
        PyMove {
            mv: Move {
                id: choice,
                disabled,
                pp,
                choice: MOVES.get(&choice).unwrap().clone(),
            },
        }
    }
}

#[derive(Clone)]
#[pyclass(get_all)]
struct PyMctsSideResult {
    pub move_choice: (String, String),
    pub total_score: f32,
    pub visits: u32,
}

impl PyMctsSideResult {
    fn from_mcts_side_result(result: MctsSideResult, side: &Side) -> Self {
        PyMctsSideResult {
            move_choice: (
                movechoice_to_string(side, &result.move_choice.0, &SlotReference::SlotA),
                movechoice_to_string(side, &result.move_choice.1, &SlotReference::SlotB),
            ),
            total_score: result.total_score,
            visits: result.visits,
        }
    }
}

#[derive(Clone)]
#[pyclass(get_all)]
struct PyMctsResult {
    s1: Vec<PyMctsSideResult>,
    s2: Vec<PyMctsSideResult>,
    iteration_count: u32,
}

impl PyMctsResult {
    fn from_mcts_result(result: MctsResult, state: &State) -> Self {
        PyMctsResult {
            s1: result
                .s1
                .iter()
                .map(|r| PyMctsSideResult::from_mcts_side_result(r.clone(), &state.side_one))
                .collect(),
            s2: result
                .s2
                .iter()
                .map(|r| PyMctsSideResult::from_mcts_side_result(r.clone(), &state.side_two))
                .collect(),
            iteration_count: result.iteration_count,
        }
    }
}

#[pyfunction]
fn mcts(mut py_state: PyState, duration_ms: u64) -> PyResult<PyMctsResult> {
    let duration = Duration::from_millis(duration_ms);
    let (s1_options, s2_options) = py_state.state.root_get_all_options();
    let mcts_result = perform_mcts(&mut py_state.state, s1_options, s2_options, duration);

    let py_mcts_result = PyMctsResult::from_mcts_result(mcts_result, &py_state.state);
    Ok(py_mcts_result)
}

#[derive(Clone)]
#[pyclass(name = "Instruction")]
struct PyInstruction {
    pub instruction: Instruction,
}

#[pymethods]
impl PyInstruction {
    fn __str__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self.instruction))
    }
}

impl PyInstruction {
    fn from_instruction(instruction: Instruction) -> Self {
        PyInstruction { instruction }
    }
}

#[derive(Clone)]
#[pyclass(name = "StateInstructions")]
struct PyStateInstructions {
    #[pyo3(get)]
    pub percentage: f32,
    pub instruction_list: Vec<PyInstruction>,
}

#[pymethods]
impl PyStateInstructions {
    #[getter]
    fn get_instruction_list(&self) -> PyResult<Vec<PyInstruction>> {
        Ok(self.instruction_list.clone())
    }
}

impl PyStateInstructions {
    fn from_state_instructions(instructions: StateInstructions) -> Self {
        PyStateInstructions {
            percentage: instructions.percentage,
            instruction_list: instructions
                .instruction_list
                .into_iter()
                .map(|i| PyInstruction::from_instruction(i))
                .collect(),
        }
    }
}

#[pyfunction]
fn gi(
    mut py_state: PyState,
    side_one_a_move: String,
    side_one_b_move: String,
    side_two_a_move: String,
    side_two_b_move: String,
) -> PyResult<Vec<PyStateInstructions>> {
    let (s1_a_move, s1_b_move, s2_a_move, s2_b_move);
    match MoveChoice::from_string(
        &side_one_a_move,
        &py_state.state.side_one,
        SlotReference::SlotA,
    ) {
        Some(m) => s1_a_move = m,
        None => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid move for s1-a: {}",
                side_one_a_move
            )))
        }
    }
    match MoveChoice::from_string(
        &side_one_b_move,
        &py_state.state.side_one,
        SlotReference::SlotB,
    ) {
        Some(m) => s1_b_move = m,
        None => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid move for s1-b: {}",
                side_one_b_move
            )))
        }
    }
    match MoveChoice::from_string(
        &side_two_a_move,
        &py_state.state.side_two,
        SlotReference::SlotA,
    ) {
        Some(m) => s2_a_move = m,
        None => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid move for s2-a: {}",
                side_two_a_move
            )))
        }
    }
    match MoveChoice::from_string(
        &side_two_b_move,
        &py_state.state.side_two,
        SlotReference::SlotB,
    ) {
        Some(m) => s2_b_move = m,
        None => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid move for s2-b: {}",
                side_two_b_move
            )))
        }
    }
    let instructions = generate_instructions_from_move_pair(
        &mut py_state.state,
        &s1_a_move,
        &s1_b_move,
        &s2_a_move,
        &s2_b_move,
        true,
    );
    let py_instructions = instructions
        .iter()
        .map(|i| PyStateInstructions::from_state_instructions(i.clone()))
        .collect();

    Ok(py_instructions)
}

#[pyfunction]
fn calculate_damage(
    mut py_state: PyState,
    attacking_side: String,
    attacking_slot: String,
    target_side: String,
    target_slot: String,
    attacker_move: String,
    target_move: String,
) -> PyResult<Vec<i16>> {
    let (mut s1_choice, mut s2_choice);

    let attacking_side_ref = SideReference::from_str(&attacking_side)
        .unwrap_or_else(|_| panic!("Invalid attacking side: {attacking_side}"));
    let attacking_slot_ref = SlotReference::from_str(&attacking_slot)
        .unwrap_or_else(|_| panic!("Invalid attacking slot: {attacking_slot}"));
    let target_side_ref = SideReference::from_str(&target_side)
        .unwrap_or_else(|_| panic!("Invalid target side: {target_side}"));
    let target_slot_ref = SlotReference::from_str(&target_slot)
        .unwrap_or_else(|_| panic!("Invalid target slot: {target_slot}"));

    match MOVES.get(&Choices::from_str(attacker_move.as_str()).unwrap()) {
        Some(m) => s1_choice = m.to_owned(),
        None => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid move for attacker: {}",
                attacker_move
            )))
        }
    }
    match MOVES.get(&Choices::from_str(target_move.as_str()).unwrap()) {
        Some(m) => s2_choice = m.to_owned(),
        None => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid move for s2: {}",
                target_move
            )))
        }
    }
    if attacker_move == "switch" {
        s1_choice.category = MoveCategory::Switch
    }
    if target_move == "switch" {
        s2_choice.category = MoveCategory::Switch
    }

    let damage_rolls = calculate_damage_rolls(
        &mut py_state.state,
        &attacking_side_ref,
        &attacking_slot_ref,
        &target_side_ref,
        &target_slot_ref,
        s1_choice,
        &s2_choice,
    );

    let py_rolls;
    match damage_rolls {
        Some(rolls) => py_rolls = rolls,
        None => py_rolls = vec![0, 0],
    }

    Ok(py_rolls)
}

#[pyfunction]
fn state_from_string(s: String) -> PyResult<PyState> {
    Ok(PyState {
        state: State::deserialize(&s),
    })
}

#[pymodule]
#[pyo3(name = "_poke_engine")]
fn py_poke_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(state_from_string, m)?)?;
    m.add_function(wrap_pyfunction!(calculate_damage, m)?)?;
    m.add_function(wrap_pyfunction!(gi, m)?)?;
    m.add_function(wrap_pyfunction!(mcts, m)?)?;
    m.add_class::<PyState>()?;
    m.add_class::<PySide>()?;
    m.add_class::<PySideSlot>()?;
    m.add_class::<PySideConditions>()?;
    m.add_class::<PyVolatileStatusDurations>()?;
    m.add_class::<PyPokemon>()?;
    m.add_class::<PyMove>()?;
    m.add_class::<PyStateInstructions>()?;
    m.add_class::<PyInstruction>()?;
    Ok(())
}
