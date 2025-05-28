use crate::choices::{Choice, Choices, MoveCategory, MOVES};
use crate::engine::evaluate::evaluate;
use crate::engine::generate_instructions::{
    calculate_both_damage_rolls, generate_instructions_from_move_pair,
};
use crate::engine::state::MoveChoice;
use crate::instruction::{Instruction, StateInstructions};
use crate::mcts::{perform_mcts, MctsResult};
use crate::state::{SlotReference, State};
use clap::Parser;
use std::io;
use std::io::Write;
use std::process::exit;
use std::str::FromStr;

struct IOData {
    state: State,
    instruction_list: Vec<Vec<Instruction>>,
    last_instructions_generated: Vec<StateInstructions>,
}

#[derive(Parser)]
struct Cli {
    #[clap(short, long, default_value = "")]
    state: String,

    #[clap(subcommand)]
    subcmd: Option<SubCommand>,
}

#[derive(Parser)]
enum SubCommand {
    MonteCarloTreeSearch(MonteCarloTreeSearch),
    CalculateDamage(CalculateDamage),
    GenerateInstructions(GenerateInstructions),
}

#[derive(Parser)]
struct MonteCarloTreeSearch {
    #[clap(short, long, required = true)]
    state: String,

    #[clap(short, long, default_value_t = 5000)]
    time_to_search_ms: u64,
}

#[derive(Parser)]
struct CalculateDamage {
    #[clap(short, long, required = true)]
    state: String,

    #[clap(short = 'o', long, required = true)]
    side_one_move: String,

    #[clap(short = 't', long, required = true)]
    side_two_move: String,

    #[clap(short = 'm', long, required = false, default_value_t = false)]
    side_one_moves_first: bool,
}

#[derive(Parser)]
struct GenerateInstructions {
    #[clap(short, long, required = true)]
    state: String,

    #[clap(short = 'a', long, required = true)]
    side_one_move_a: String,

    #[clap(short = 'b', long, required = true)]
    side_one_move_b: String,

    #[clap(short = 'c', long, required = true)]
    side_two_move_a: String,

    #[clap(short = 'd', long, required = true)]
    side_two_move_b: String,
}

impl Default for IOData {
    fn default() -> Self {
        IOData {
            state: State::default(),
            instruction_list: Vec::new(),
            last_instructions_generated: Vec::new(),
        }
    }
}

pub fn pprint_mcts_result(state: &State, result: MctsResult) {
    println!("\nTotal Iterations: {}\n", result.iteration_count);

    // Side One
    println!("Side One Options: {}", result.s1.len());
    println!("Side One (Top 10):");
    println!(
        "\t{:<40}{:>12}{:>12}{:>10}{:>10}",
        "Move", "Total Score", "Avg Score", "Visits", "% Visits"
    );

    let mut s1_sorted = result.s1.clone();
    s1_sorted.sort_by(|a, b| b.visits.cmp(&a.visits));

    for x in s1_sorted.iter().take(10) {
        println!(
            "\t{:<20}{:<20}{:>12.2}{:>12.2}{:>10}{:>10.2}",
            x.move_choice
                .0
                .to_string(&state.side_one, &SlotReference::SlotA),
            x.move_choice
                .1
                .to_string(&state.side_one, &SlotReference::SlotB),
            x.total_score,
            x.total_score / x.visits as f32,
            x.visits,
            (x.visits as f32 / result.iteration_count as f32) * 100.0
        );
    }

    if s1_sorted.len() > 5 {
        println!("\nSide One (Bottom 10):");
        println!(
            "\t{:<40}{:>12}{:>12}{:>10}{:>10}",
            "Move", "Total Score", "Avg Score", "Visits", "% Visits"
        );

        for x in s1_sorted.iter().rev().take(10) {
            println!(
                "\t{:<20}{:<20}{:>12.2}{:>12.2}{:>10}{:>10.2}",
                x.move_choice
                    .0
                    .to_string(&state.side_one, &SlotReference::SlotA),
                x.move_choice
                    .1
                    .to_string(&state.side_one, &SlotReference::SlotB),
                x.total_score,
                x.total_score / x.visits as f32,
                x.visits,
                (x.visits as f32 / result.iteration_count as f32) * 100.0
            );
        }
    }

    // Side Two
    println!("\nSide Two Options: {}", result.s2.len());
    println!("Side Two (Top 10):");
    println!(
        "\t{:<40}{:>12}{:>12}{:>10}{:>10}",
        "Move", "Total Score", "Avg Score", "Visits", "% Visits"
    );

    let mut s2_sorted = result.s2.clone();
    s2_sorted.sort_by(|a, b| b.visits.cmp(&a.visits));

    for x in s2_sorted.iter().take(10) {
        println!(
            "\t{:<20}{:<20}{:>12.2}{:>12.2}{:>10}{:>10.2}",
            x.move_choice
                .0
                .to_string(&state.side_two, &SlotReference::SlotA),
            x.move_choice
                .1
                .to_string(&state.side_two, &SlotReference::SlotB),
            x.total_score,
            x.total_score / x.visits as f32,
            x.visits,
            (x.visits as f32 / result.iteration_count as f32) * 100.0
        );
    }

    if s2_sorted.len() > 5 {
        println!("\nSide Two (Bottom 10):");
        println!(
            "\t{:<40}{:>12}{:>12}{:>10}{:>10}",
            "Move", "Total Score", "Avg Score", "Visits", "% Visits"
        );

        for x in s2_sorted.iter().rev().take(10) {
            println!(
                "\t{:<20}{:<20}{:>12.2}{:>12.2}{:>10}{:>10.2}",
                x.move_choice
                    .0
                    .to_string(&state.side_two, &SlotReference::SlotA),
                x.move_choice
                    .1
                    .to_string(&state.side_two, &SlotReference::SlotB),
                x.total_score,
                x.total_score / x.visits as f32,
                x.visits,
                (x.visits as f32 / result.iteration_count as f32) * 100.0
            );
        }
    }
}

fn pprint_state_instruction_vector(instructions: &Vec<StateInstructions>) {
    for (i, instruction) in instructions.iter().enumerate() {
        println!("Index: {}", i);
        println!("StateInstruction: {:?}", instruction);
    }
}

pub fn main() {
    let args = Cli::parse();
    let mut io_data = IOData::default();

    if args.state != "" {
        let state = State::deserialize(args.state.as_str());
        io_data.state = state;
    }

    let mut state;
    let side_one_options;
    let side_two_options;
    match args.subcmd {
        None => {
            command_loop(io_data);
            exit(0);
        }
        Some(subcmd) => match subcmd {
            SubCommand::MonteCarloTreeSearch(mcts) => {
                state = State::deserialize(mcts.state.as_str());
                (side_one_options, side_two_options) = state.root_get_all_options();
                let result = perform_mcts(
                    &mut state,
                    side_one_options.clone(),
                    side_two_options.clone(),
                    std::time::Duration::from_millis(mcts.time_to_search_ms),
                );
                pprint_mcts_result(&state, result);
            }
            SubCommand::CalculateDamage(_calculate_damage) => panic!("Not implemented yet"),
            //     state = State::deserialize(calculate_damage.state.as_str());
            //     let mut s1_choice = MOVES
            //         .get(&Choices::from_str(calculate_damage.side_one_move.as_str()).unwrap())
            //         .unwrap()
            //         .to_owned();
            //     let mut s2_choice = MOVES
            //         .get(&Choices::from_str(calculate_damage.side_two_move.as_str()).unwrap())
            //         .unwrap()
            //         .to_owned();
            //     let s1_moves_first = calculate_damage.side_one_moves_first;
            //     if calculate_damage.side_one_move == "switch" {
            //         s1_choice.category = MoveCategory::Switch
            //     }
            //     if calculate_damage.side_two_move == "switch" {
            //         s2_choice.category = MoveCategory::Switch
            //     }
            //     calculate_damage_io(&state, s1_choice, s2_choice, s1_moves_first);
            // }
            SubCommand::GenerateInstructions(generate_instructions) => {
                state = State::deserialize(generate_instructions.state.as_str());
                let (s1_a_movechoice, s1_b_movechoice, s2_a_movechoice, s2_b_movechoice);
                match MoveChoice::from_string(
                    generate_instructions.side_one_move_a.as_str(),
                    &state.side_one,
                    SlotReference::SlotA,
                ) {
                    None => {
                        println!(
                            "Invalid move choice for side one a: {}",
                            generate_instructions.side_one_move_a.as_str()
                        );
                        exit(1);
                    }
                    Some(v) => s1_a_movechoice = v,
                }
                match MoveChoice::from_string(
                    generate_instructions.side_one_move_b.as_str(),
                    &state.side_one,
                    SlotReference::SlotB,
                ) {
                    None => {
                        println!(
                            "Invalid move choice for side one a: {}",
                            generate_instructions.side_one_move_b.as_str()
                        );
                        exit(1);
                    }
                    Some(v) => s1_b_movechoice = v,
                }
                match MoveChoice::from_string(
                    generate_instructions.side_two_move_a.as_str(),
                    &state.side_two,
                    SlotReference::SlotA,
                ) {
                    None => {
                        println!(
                            "Invalid move choice for side one a: {}",
                            generate_instructions.side_two_move_a.as_str()
                        );
                        exit(1);
                    }
                    Some(v) => s2_a_movechoice = v,
                }
                match MoveChoice::from_string(
                    generate_instructions.side_two_move_b.as_str(),
                    &state.side_two,
                    SlotReference::SlotB,
                ) {
                    None => {
                        println!(
                            "Invalid move choice for side one a: {}",
                            generate_instructions.side_two_move_b.as_str()
                        );
                        exit(1);
                    }
                    Some(v) => s2_b_movechoice = v,
                }

                if matches!(s1_a_movechoice, MoveChoice::MoveTera(_, _, _))
                    && matches!(s1_b_movechoice, MoveChoice::MoveTera(_, _, _))
                {
                    println!("side_one can only terastallize one pokemon");
                    exit(1);
                }

                if matches!(s2_a_movechoice, MoveChoice::MoveTera(_, _, _))
                    && matches!(s2_b_movechoice, MoveChoice::MoveTera(_, _, _))
                {
                    println!("side_two can only terastallize one pokemon");
                    exit(1);
                }

                let instructions = generate_instructions_from_move_pair(
                    &mut state,
                    &s1_a_movechoice,
                    &s1_b_movechoice,
                    &s2_a_movechoice,
                    &s2_b_movechoice,
                    true,
                );
                pprint_state_instruction_vector(&instructions);
            }
        },
    }
    exit(0);
}

fn calculate_damage_io(
    state: &State,
    s1_choice: Choice,
    s2_choice: Choice,
    side_one_moves_first: bool,
) {
    let (damages_dealt_s1, damages_dealt_s2) =
        calculate_both_damage_rolls(state, s1_choice, s2_choice, side_one_moves_first);

    for dmg in [damages_dealt_s1, damages_dealt_s2] {
        match dmg {
            Some(damages_vec) => {
                let joined = damages_vec
                    .iter()
                    .map(|x| format!("{:?}", x))
                    .collect::<Vec<String>>()
                    .join(",");
                println!("Damage Rolls: {}", joined);
            }
            None => {
                println!("Damage Rolls: 0");
            }
        }
    }
}

fn command_loop(mut io_data: IOData) {
    loop {
        print!("> ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {}
            Err(error) => {
                println!("Error reading input: {}", error);
                continue;
            }
        }
        let mut parts = input.trim().split_whitespace();
        let command = parts.next().unwrap_or("");
        let mut args = parts;

        match command {
            "state" | "s" => {
                let state_string;
                match args.next() {
                    Some(s) => {
                        state_string = s;
                        let state = State::deserialize(state_string);
                        io_data.state = state;
                        println!("state initialized");
                    }
                    None => {
                        println!("Expected state string");
                    }
                }
                println!("{:?}", io_data.state);
            }
            "serialize" | "ser" => {
                println!("{}", io_data.state.serialize());
            }
            "matchup" | "m" => {
                println!("{}", io_data.state.pprint());
            }
            "generate-instructions" | "g" => {
                let (s1_a_move, s1_b_move, s2_a_move, s2_b_move);
                match args.next() {
                    Some(s) => match MoveChoice::from_string(
                        s,
                        &io_data.state.side_one,
                        SlotReference::SlotA,
                    ) {
                        Some(m) => {
                            s1_a_move = m;
                        }
                        None => {
                            println!("Invalid move choice for side one a: {}", s);
                            continue;
                        }
                    },
                    None => {
                        println!("Usage: generate-instructions <side-1 move> <side-2 move>");
                        continue;
                    }
                }
                match args.next() {
                    Some(s) => match MoveChoice::from_string(
                        s,
                        &io_data.state.side_one,
                        SlotReference::SlotB,
                    ) {
                        Some(m) => {
                            s1_b_move = m;
                        }
                        None => {
                            println!("Invalid move choice for side one b: {}", s);
                            continue;
                        }
                    },
                    None => {
                        println!("Usage: generate-instructions <side-1 move> <side-2 move>");
                        continue;
                    }
                }
                match args.next() {
                    Some(s) => match MoveChoice::from_string(
                        s,
                        &io_data.state.side_two,
                        SlotReference::SlotA,
                    ) {
                        Some(m) => {
                            s2_a_move = m;
                        }
                        None => {
                            println!("Invalid move choice for side two a: {}", s);
                            continue;
                        }
                    },
                    None => {
                        println!("Usage: generate-instructions <side-1 move> <side-2 move>");
                        continue;
                    }
                }
                match args.next() {
                    Some(s) => match MoveChoice::from_string(
                        s,
                        &io_data.state.side_two,
                        SlotReference::SlotB,
                    ) {
                        Some(m) => {
                            s2_b_move = m;
                        }
                        None => {
                            println!("Invalid move choice for side two b: {}", s);
                            continue;
                        }
                    },
                    None => {
                        println!("Usage: generate-instructions <side-1 move> <side-2 move>");
                        continue;
                    }
                }

                if matches!(s1_a_move, MoveChoice::MoveTera(_, _, _))
                    && matches!(s1_b_move, MoveChoice::MoveTera(_, _, _))
                {
                    println!("side_one can only terastallize one pokemon");
                    continue;
                }

                if matches!(s2_a_move, MoveChoice::MoveTera(_, _, _))
                    && matches!(s2_b_move, MoveChoice::MoveTera(_, _, _))
                {
                    println!("side_two can only terastallize one pokemon");
                    continue;
                }

                let instructions = generate_instructions_from_move_pair(
                    &mut io_data.state,
                    &s1_a_move,
                    &s1_b_move,
                    &s2_a_move,
                    &s2_b_move,
                    true,
                );
                pprint_state_instruction_vector(&instructions);
                io_data.last_instructions_generated = instructions;
            }
            "calculate-damage" | "d" => {
                let (mut s1_choice, mut s2_choice);
                match args.next() {
                    Some(s) => {
                        s1_choice = MOVES
                            .get(&Choices::from_str(s).unwrap())
                            .unwrap()
                            .to_owned();
                        if s == "switch" {
                            s1_choice.category = MoveCategory::Switch
                        }
                    }
                    None => {
                        println!("Usage: calculate-damage <side-1 move> <side-2 move> <side-1-moves-first>");
                        continue;
                    }
                }
                match args.next() {
                    Some(s) => {
                        s2_choice = MOVES
                            .get(&Choices::from_str(s).unwrap())
                            .unwrap()
                            .to_owned();
                        if s == "switch" {
                            s2_choice.category = MoveCategory::Switch
                        }
                    }
                    None => {
                        println!("Usage: calculate-damage <side-1 move> <side-2 move> <side-1-moves-first>");
                        continue;
                    }
                }
                let s1_moves_first: bool;
                match args.next() {
                    Some(s) => {
                        s1_moves_first = s.parse::<bool>().unwrap();
                    }
                    None => {
                        println!("Usage: calculate-damage <side-1 move> <side-2 move> <side-1-moves-first>");
                        continue;
                    }
                }
                calculate_damage_io(&io_data.state, s1_choice, s2_choice, s1_moves_first);
            }
            "instructions" | "i" => {
                println!("{:?}", io_data.last_instructions_generated);
            }
            "evaluate" | "ev" => {
                println!("Evaluation: {}", evaluate(&io_data.state));
            }
            "monte-carlo-tree-search" | "mcts" => match args.next() {
                Some(s) => {
                    let max_time_ms = s.parse::<u64>().unwrap();
                    let (side_one_options, side_two_options) = io_data.state.root_get_all_options();

                    let start_time = std::time::Instant::now();
                    let result = perform_mcts(
                        &mut io_data.state,
                        side_one_options.clone(),
                        side_two_options.clone(),
                        std::time::Duration::from_millis(max_time_ms),
                    );
                    let elapsed = start_time.elapsed();
                    pprint_mcts_result(&io_data.state, result);

                    println!("\nTook: {:?}", elapsed);
                }
                None => {
                    println!("Usage: monte-carlo-tree-search <timeout_ms>");
                    continue;
                }
            },
            "apply" | "a" => match args.next() {
                Some(s) => {
                    let index = s.parse::<usize>().unwrap();
                    let instructions = io_data.last_instructions_generated.remove(index);
                    io_data
                        .state
                        .apply_instructions(&instructions.instruction_list);
                    io_data.instruction_list.push(instructions.instruction_list);
                    io_data.last_instructions_generated = Vec::new();
                    println!("Applied instructions at index {}", index)
                }
                None => {
                    println!("Usage: apply <instruction index>");
                    continue;
                }
            },
            "pop" | "p" => {
                if io_data.instruction_list.is_empty() {
                    println!("No instructions to pop");
                    continue;
                }
                let instructions = io_data.instruction_list.pop().unwrap();
                io_data.state.reverse_instructions(&instructions);
                println!("Popped last applied instructions");
            }
            "pop-all" | "pa" => {
                for i in io_data.instruction_list.iter().rev() {
                    io_data.state.reverse_instructions(i);
                }
                io_data.instruction_list.clear();
                println!("Popped all applied instructions");
            }
            "" => {
                continue;
            }
            "exit" | "quit" | "q" => {
                break;
            }
            command => {
                println!("Unknown command: {}", command);
            }
        }
    }
}
