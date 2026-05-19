// debug entrypoint: deserializes a State, builds move options, and prints
// each side's heuristic scores in three sections:
//   1. slot A MoveChoices ranked individually
//   2. slot B MoveChoices ranked individually
//   3. (slot A, slot B) pairs ranked by combined score with a per-component
//      breakdown (slot A + slot B + synergy)
// used to sanity-check ranking behaviour without running a full MCTS.
//
// run:
//   cargo run --bin rank-debug --features=gen9 -- --state "<serialized state>"

use clap::Parser;
use poke_engine::engine::state::{MoveChoice, MoveOptions};
use poke_engine::heuristics::{pair_synergy, score_move_choice};
use poke_engine::state::{Side, SideReference, SlotReference, State};

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    state: String,
    // optionally cap the printed rows per pair table (default: print everything).
    // slot A/B tables are always printed in full since they're small.
    #[arg(short, long)]
    limit: Option<usize>,
}

// MoveChoice::to_u8() returns values in 0..=90; mirrors heuristics::SLOT_SCORE_TABLE_LEN
const SLOT_SCORE_TABLE_LEN: usize = 91;

fn main() {
    let args = Cli::parse();
    let state = State::deserialize(&args.state);

    let mut move_options = MoveOptions::new();
    state.get_all_options_keep_slot_buffers(&mut move_options);

    print_side(
        "Side One",
        &state,
        &state.sides[0],
        SideReference::SideOne,
        &move_options.side_one_slot_a_options,
        &move_options.side_one_slot_b_options,
        &move_options.side_one_combined_options,
        args.limit,
    );
    println!();
    print_side(
        "Side Two",
        &state,
        &state.sides[1],
        SideReference::SideTwo,
        &move_options.side_two_slot_a_options,
        &move_options.side_two_slot_b_options,
        &move_options.side_two_combined_options,
        args.limit,
    );
}

fn print_side(
    label: &str,
    state: &State,
    side: &Side,
    side_ref: SideReference,
    slot_a_options: &[MoveChoice],
    slot_b_options: &[MoveChoice],
    pairs: &[(MoveChoice, MoveChoice)],
    pair_limit: Option<usize>,
) {
    // fill per-slot score caches indexed by MoveChoice::to_u8() — same shape
    // as rank_side_pairs uses internally.
    let mut slot_a_scores = [0.0f32; SLOT_SCORE_TABLE_LEN];
    let mut slot_b_scores = [0.0f32; SLOT_SCORE_TABLE_LEN];
    for mc in slot_a_options {
        slot_a_scores[mc.to_u8() as usize] =
            score_move_choice(state, side_ref, SlotReference::SlotA, mc);
    }
    for mc in slot_b_options {
        slot_b_scores[mc.to_u8() as usize] =
            score_move_choice(state, side_ref, SlotReference::SlotB, mc);
    }

    println!("############ {} ############", label);
    print_slot_table(
        &format!("{} Slot A", label),
        side,
        SlotReference::SlotA,
        slot_a_options,
        &slot_a_scores,
    );
    println!();
    print_slot_table(
        &format!("{} Slot B", label),
        side,
        SlotReference::SlotB,
        slot_b_options,
        &slot_b_scores,
    );
    println!();
    print_pair_table(
        &format!("{} Combined", label),
        state,
        side,
        side_ref,
        pairs,
        &slot_a_scores,
        &slot_b_scores,
        pair_limit,
    );
}

fn print_slot_table(
    label: &str,
    side: &Side,
    slot_ref: SlotReference,
    options: &[MoveChoice],
    slot_scores: &[f32; SLOT_SCORE_TABLE_LEN],
) {
    let mut indices: Vec<usize> = (0..options.len()).collect();
    indices.sort_by(|&a, &b| {
        let sa = slot_scores[options[a].to_u8() as usize];
        let sb = slot_scores[options[b].to_u8() as usize];
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    println!("--- {} --- ({} options)", label, options.len());
    println!("{:>4}  {:>8}  {}", "rank", "score", "move");
    for (rank, &i) in indices.iter().enumerate() {
        let mc = &options[i];
        println!(
            "{:>4}  {:>8.2}  {}",
            rank,
            slot_scores[mc.to_u8() as usize],
            mc.to_string(side, &slot_ref),
        );
    }
}

fn print_pair_table(
    label: &str,
    state: &State,
    side: &Side,
    side_ref: SideReference,
    pairs: &[(MoveChoice, MoveChoice)],
    slot_a_scores: &[f32; SLOT_SCORE_TABLE_LEN],
    slot_b_scores: &[f32; SLOT_SCORE_TABLE_LEN],
    limit: Option<usize>,
) {
    // build (a_score, b_score, synergy, combined) per pair
    let breakdowns: Vec<(f32, f32, f32, f32)> = pairs
        .iter()
        .map(|(a, b)| {
            let a_s = slot_a_scores[a.to_u8() as usize];
            let b_s = slot_b_scores[b.to_u8() as usize];
            let syn = pair_synergy(state, side_ref, a, b);
            (a_s, b_s, syn, a_s + b_s + syn)
        })
        .collect();

    let mut indices: Vec<usize> = (0..pairs.len()).collect();
    indices.sort_by(|&a, &b| {
        breakdowns[b]
            .3
            .partial_cmp(&breakdowns[a].3)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let shown = limit.unwrap_or(indices.len()).min(indices.len());
    println!(
        "--- {} --- ({} pairs, showing top {})",
        label,
        pairs.len(),
        shown
    );
    println!(
        "{:>4}  {:>8}  {:>8}  {:>8}  {:>8}  {:<40}  {}",
        "rank", "combined", "slotA", "slotB", "synergy", "slotA-move", "slotB-move"
    );
    for (rank, &i) in indices.iter().take(shown).enumerate() {
        let (a, b) = &pairs[i];
        let (a_s, b_s, syn, combined) = breakdowns[i];
        println!(
            "{:>4}  {:>8.2}  {:>8.2}  {:>8.2}  {:>8.2}  {:<40}  {}",
            rank,
            combined,
            a_s,
            b_s,
            syn,
            a.to_string(side, &SlotReference::SlotA),
            b.to_string(side, &SlotReference::SlotB),
        );
    }
}
