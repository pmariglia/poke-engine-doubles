// progressive-widening heuristics: rank a side's (MoveChoice, MoveChoice) pairs
// by a cheap proxy so MCTS can prune the long tail.
//
// design notes:
//   - the heuristic only controls *which* pairs UCB1 sees, not the UCB1 score itself
//   - per-slot scores are computed once and cached in a stack-local [f32; SLOT_SCORE_TABLE_LEN]
//     keyed by MoveChoice::to_u8() so the per-pair work is two indexes + a
//     synergy fn. zero heap allocs in this module.
//   - synergy is a stub for now; see pair_synergy below

use crate::choices::{Choice, Choices, MoveCategory, MoveChoiceTarget, MoveTarget};
use crate::engine::damage_calc::type_effectiveness_modifier;
use crate::engine::state::{MoveChoice, PokemonVolatileStatus, Weather};
use crate::state::{
    LastUsedMove, Pokemon, PokemonType, Side, SideReference, SideSlot, SlotReference, State,
};

// MoveChoice::to_u8() returns values in 0..=90
// this needs to be kept in sync with the MoveChoice encoding; we could make it a const fn?
pub const SLOT_SCORE_TABLE_LEN: usize = 91;

// damaging-move scoring constants
const BASE_POWER_MULTIPLIER: f32 = 0.25;
const SPREAD_MULTIPLIER: f32 = 1.5;
const STAB_BONUS: f32 = 1.5;
const MEGA_BONUS: f32 = 50.0;
const PRIORITY_BONUS_PER_STEP: f32 = 5.0;

// status-move priors. these are picked to sit between a weak (~40 BP) and
// a strong (~90 BP) attack so they compete on rank.
const STATUS_PRIOR_BOOST: f32 = 30.0;
const STATUS_PRIOR_STATUS_INFLICT: f32 = 55.0;
const STATUS_PRIOR_VOLATILE: f32 = 25.0;
const STATUS_PRIOR_SIDE_CONDITION: f32 = 50.0;
const STATUS_PRIOR_HEAL_FULL_HEAL: f32 = 80.0; // scaled by hp missing
const CAN_BE_ENCORED_OR_DISABLED_BONUS: f32 = 100.0; // if a target can be encore-ed

// switch priors
const SWITCH_BASE: f32 = 20.0;
const SWITCH_MATCHUP_WEIGHT: f32 = 20.0; // scales the type-matchup term

// Fake Out is filtered upstream to only appear when usable (just switched in),
// so any FAKEOUT we see here is high-EV: priority +3, guaranteed flinch
// and damage. push it well above mid-range attacks so progressive widening
// keeps it in the top-K early.
const FAKEOUT_BONUS: f32 = 180.0;

// flat penalty applied to a protect-family move when the attacking slot's
// protect-volatile counter is non-zero. consecutive protect succeeds with
// exponentially decaying probability; ranking it lower than alternatives is
// almost always correct.
const PROTECT_SPAM_PENALTY: f32 = 80.0;

// fills `scores_out` (parallel to `combined_options`) with a heuristic score
// per pair. caller must clear scores_out before calling; we push.
pub fn rank_side_pairs(
    state: &State,
    side_ref: SideReference,
    slot_a_options: &[MoveChoice],
    slot_b_options: &[MoveChoice],
    combined_options: &[(MoveChoice, MoveChoice)],
    scores_out: &mut Vec<f32>,
) {
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

    for (a, b) in combined_options {
        let s = slot_a_scores[a.to_u8() as usize]
            + slot_b_scores[b.to_u8() as usize]
            + pair_synergy(state, side_ref, a, b);
        scores_out.push(s);
    }
}

// cheap-proxy score for a single MoveChoice in isolation. higher = better.
// scale: damaging moves land roughly in 0..200; status moves land in 25..100;
// switches land in 0..70. those ranges are intentionally overlapping so a
// good status move can outrank a bad attack.
pub fn score_move_choice(
    state: &State,
    side_ref: SideReference,
    slot_ref: SlotReference,
    mc: &MoveChoice,
) -> f32 {
    match mc {
        MoveChoice::Move(target_slot, target_side, move_index)
        | MoveChoice::MoveTera(target_slot, target_side, move_index)
        | MoveChoice::MoveMega(target_slot, target_side, move_index) => {
            let attacker_side = state.get_side_immutable(side_ref);
            let attacker = attacker_side.get_active_immutable(&slot_ref);
            let mv = &attacker.moves[move_index];
            let choice = &mv.choice;

            let target_side_obj = state.get_side_immutable(*target_side);
            let target_slot_obj = target_side_obj.get_slot_immutable(target_slot);
            let target = target_side_obj.get_active_immutable(target_slot);
            let base = if choice.category == MoveCategory::Status || choice.base_power <= 0.0 {
                score_status_move(state, attacker, attacker_side, target_slot_obj, choice)
            } else {
                score_damaging_move(attacker, target, choice)
            };

            let mega_bonus = if matches!(mc, MoveChoice::MoveMega(_, _, _)) {
                MEGA_BONUS
            } else {
                0.0
            };

            let fakeout_bonus = if mv.id == Choices::FAKEOUT {
                FAKEOUT_BONUS
            } else {
                0.0
            };

            let protect_spam_penalty = if is_protect_family(choice)
                && attacker_side
                    .get_slot_immutable(&slot_ref)
                    .volatile_status_durations
                    .protect
                    > 0
            {
                PROTECT_SPAM_PENALTY
            } else {
                0.0
            };

            base + mega_bonus + fakeout_bonus - protect_spam_penalty
        }
        MoveChoice::Switch(target_index) => {
            let attacker_side = state.get_side_immutable(side_ref);
            let current_active = attacker_side.get_active_immutable(&slot_ref);
            let incoming = &attacker_side.pokemon[target_index];
            let opp_side = state.get_side_immutable(side_ref.get_other_side());
            let opp_a = opp_side.get_active_immutable(&SlotReference::SlotA);
            let opp_b = opp_side.get_active_immutable(&SlotReference::SlotB);
            score_switch(current_active, incoming, opp_a, opp_b)
        }
        MoveChoice::None => 0.0,
        MoveChoice::TeamPreview(_, _) => 0.0,
    }
}

fn score_damaging_move(attacker: &Pokemon, target: &Pokemon, choice: &Choice) -> f32 {
    let type_eff = type_effectiveness_modifier(&choice.move_type, target);
    let is_stab = choice.move_type == attacker.types.0 || choice.move_type == attacker.types.1;
    let stab = if is_stab { STAB_BONUS } else { 1.0 };

    let accuracy_factor = (choice.accuracy / 100.0).clamp(0.0, 1.0);

    let spread = if choice.move_choice_target == MoveChoiceTarget::AllFoes
        || choice.move_choice_target == MoveChoiceTarget::AllOther
    {
        SPREAD_MULTIPLIER
    } else {
        1.0
    };

    let priority_bias = choice.priority as f32 * PRIORITY_BONUS_PER_STEP;

    (BASE_POWER_MULTIPLIER * choice.base_power * type_eff * stab * accuracy_factor * spread)
        + priority_bias
}

fn score_status_move(
    state: &State,
    attacker: &Pokemon,
    attacker_side: &Side,
    target_slot: &SideSlot,
    choice: &Choice,
) -> f32 {
    let mut s = 0.0;
    if choice.boost.is_some() {
        s += STATUS_PRIOR_BOOST;
    }
    if choice.status.is_some() {
        s += STATUS_PRIOR_STATUS_INFLICT;
    }
    if choice.volatile_status.is_some() {
        s += STATUS_PRIOR_VOLATILE;
    }
    if choice.side_condition.is_some() {
        s += STATUS_PRIOR_SIDE_CONDITION;
    }
    if let Some(heal) = &choice.heal {
        // only valuable when not at full HP. heal.amount is the fraction of
        // maxhp restored (negative for self-damage moves like substitute).
        let missing = if attacker.maxhp > 0 {
            1.0 - (attacker.hp as f32 / attacker.maxhp as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if heal.amount > 0.0 {
            s += STATUS_PRIOR_HEAL_FULL_HEAL * missing;
        }
    }
    // spread status moves are slightly more valuable
    if choice.move_choice_target == MoveChoiceTarget::AllFoes
        || choice.move_choice_target == MoveChoiceTarget::AllOther
    {
        s *= SPREAD_MULTIPLIER;
    }

    match choice.move_id {
        Choices::ENCORE | Choices::DISABLE => {
            if matches!(target_slot.last_used_move, LastUsedMove::Move(_)) {
                s += CAN_BE_ENCORED_OR_DISABLED_BONUS;
            }
        }
        Choices::TAILWIND => {
            if attacker_side.side_conditions.tailwind != 0 {
                s = 0.0;
            }
        }
        Choices::AURORAVEIL => {
            if attacker_side.side_conditions.aurora_veil != 0
                || state.get_weather() != Weather::HAIL
            {
                s = 0.0;
            }
        }
        Choices::LIGHTSCREEN => {
            if attacker_side.side_conditions.light_screen != 0 {
                s = 0.0;
            }
        }
        Choices::REFLECT => {
            if attacker_side.side_conditions.reflect != 0 {
                s = 0.0;
            }
        }
        _ => {}
    }

    let accuracy_factor = (choice.accuracy / 100.0).clamp(0.0, 1.0);
    s *= accuracy_factor;
    let _ = state; // reserved for future state-dependent priors (e.g. weather setup)
    s
}

// switch scoring: incoming pokemon's defensive type matchup vs both opposing
// actives. bonus when current active is low HP, penalty when it's healthy.
fn score_switch(
    current_active: &Pokemon,
    incoming: &Pokemon,
    opp_a: &Pokemon,
    opp_b: &Pokemon,
) -> f32 {
    // type matchup: lower defensive multiplier vs opponents' STAB types is better.
    // we compute (1 - avg_eff) so resistances (eff<1) score positive and
    // weaknesses (eff>1) score negative.
    let mut matchup_score = 0.0;
    for opp in [opp_a, opp_b] {
        if opp.hp <= 0 {
            continue;
        }
        let eff_t1_current = if opp.types.0 != PokemonType::TYPELESS {
            type_effectiveness_modifier(&opp.types.0, current_active)
        } else {
            1.0
        };
        let eff_t2_current = if opp.types.1 != PokemonType::TYPELESS {
            type_effectiveness_modifier(&opp.types.1, current_active)
        } else {
            1.0
        };
        let eff_t1_inc = if opp.types.0 != PokemonType::TYPELESS {
            type_effectiveness_modifier(&opp.types.0, incoming)
        } else {
            1.0
        };
        let eff_t2_inc = if opp.types.1 != PokemonType::TYPELESS {
            type_effectiveness_modifier(&opp.types.1, incoming)
        } else {
            1.0
        };
        let avg_eff_current = (eff_t1_current + eff_t2_current) * 0.5;
        let avg_eff_incoming = (eff_t1_inc + eff_t2_inc) * 0.5;
        matchup_score += avg_eff_current - avg_eff_incoming;
    }

    SWITCH_BASE + SWITCH_MATCHUP_WEIGHT * matchup_score
}

// matches PROTECT and its variants. detection is via the move's
// volatile_status field rather than a hardcoded move-id list so this stays
// correct as new protect-family moves are added to the engine. all of these
// moves increment SideSlot.volatile_status_durations.protect on use, which is
// the same counter we read for the spam penalty.
fn is_protect_family(choice: &Choice) -> bool {
    let Some(vs) = &choice.volatile_status else {
        return false;
    };
    if vs.target != MoveTarget::User {
        return false;
    }
    matches!(
        vs.volatile_status,
        PokemonVolatileStatus::PROTECT
            | PokemonVolatileStatus::BANEFULBUNKER
            | PokemonVolatileStatus::BURNINGBULWARK
            | PokemonVolatileStatus::KINGSSHIELD
            | PokemonVolatileStatus::SPIKYSHIELD
            | PokemonVolatileStatus::SILKTRAP
            | PokemonVolatileStatus::MAXGUARD
    )
}

// synergy / anti-synergy between two MoveChoices on the same side.
// stubbed at 0.0 to ship the framework. candidate rules to add later:
//   - Helping Hand: boost partner's damaging move score
//   - Follow Me / Rage Powder: discount partner's risky moves (they'll be redirected)
//   - Earthquake + Levitate/Flying-type ally: small bonus (no friendly damage)
//   - Earthquake + grounded ally: large penalty (friendly damage)
//   - Heat Wave / Surf + Flash Fire / Water Absorb ally: boost partner's ability
//   - Spread move + protect/wide guard ally interactions
pub fn pair_synergy(
    _state: &State,
    _side_ref: SideReference,
    _slot_a_mc: &MoveChoice,
    _slot_b_mc: &MoveChoice,
) -> f32 {
    0.0
}
