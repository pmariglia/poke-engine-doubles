use crate::engine::evaluate::evaluate;
use crate::engine::generate_instructions::generate_instructions_from_move_pair;
use crate::engine::state::{MoveChoice, MoveOptions};
use crate::heuristics::rank_side_pairs;
use crate::instruction::StateInstructions;
use crate::mcts::{MctsResult, MctsSideResult};
use crate::state::{SideReference, State};
use dashmap::DashMap;
use rand::prelude::*;
use rand::rng;
use std::sync::atomic::{AtomicI8, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

const MCTS_DEADLINE_CHECK_INTERVAL: u32 = 1_000;
const MCTS_MAX_ITERATIONS_PER_TREE: u32 = 25_000_000;
const MCTS_MAX_DEPTH: u8 = 5;
const SCORE_SCALE: f32 = 400.0;

// progressive widening: at parent visit count N, only the top
// K = max(1, min(len, ceil(WIDEN_C * sqrt(N)))) options (by heuristic rank)
// are visible to UCB1. options past K are ignored by selection until N grows
// enough to admit them. with WIDEN_C = 2.0 the schedule is roughly:
//   N=1    -> K=2
//   N=10   -> K=7
//   N=100  -> K=20
//   N=1k   -> K=64
//   N=10k  -> K=200
// the root (which accrues visits fastest) opens fully well before the search
// ends; internal nodes stay pruned in proportion to how often they're visited.
const WIDEN_C: f32 = 2.0;

// added to a MoveNode's `visits` only, so while a
// thread is in flight through that move it reads like this many extra losing
// playouts: its ucb1 drops and other threads are steered onto different moves.
// this is purely a diversification deterrent, so the magnitude is a tuning knob
const VIRTUAL_LOSS_VISITS: u32 = 3;

// hard cap on the widened slice for *non-root* nodes. raising this means
// more selection work per internal node visit; lowering it means PW can't
// keep admitting more moves once parent_visits is huge. the root ignores
// this cap entirely and always sees every available option (see
// maximize_ucb_for_side) -- root selection happens every iteration and the
// user wants all options scored, not gated by PW's sqrt schedule.
const MAX_WIDEN: usize = 64;

// first-play urgency / smoothed exploration:
//
// the textbook UCB1 bonus is sqrt(c * ln(N) / n). at n = 0 that's +inf, which
// makes a freshly admitted move under progressive widening grab the next
// selection unconditionally, and after exactly one rollout its bonus collapses
// to sqrt(c * ln(N)) -- still huge, which is the "flood" the user was seeing.
//
// instead, replace `n` with `n + V_MIN` in both the unvisited branch and the
// visited branch. V_MIN is set per call to parent_visits / k -- the visit
// count an unvisited move would have if all k widened siblings were explored
// equally. this gives:
//   - n = 0:        bonus = sqrt(c * ln(N) / V_MIN)         (no singularity)
//   - n small:      bonus ~ sqrt(c * ln(N) / V_MIN)         (no transition flood)
//   - n >> V_MIN:   bonus ~ sqrt(c * ln(N) / n)             (standard UCB1)
//
// unvisited moves use the parent's per-side mean Q (sum_score / sum_visits
// over visited siblings in the widened slice, defaulting to 0.5 when no
// sibling is visited yet, with a tunable pessimism reduction). because every
// move shares the same V_MIN-smoothed denominator, an unvisited move can
// win selection without ever exploding -- it's compared against visited
// peers' actual averages plus the same bonus.
const FPU_REDUCTION: f32 = 0.0;
const FPU_DEFAULT_Q: f32 = 0.5;

// node map type alias for clarity.
// key: (parent node address, s1_move_index, s2_move_index)
// value: the branch (weighted list of outcome nodes for that move pair)
type ChildMap = DashMap<(usize, usize, usize), SharedBranch>;

fn sigmoid(x: f32) -> f32 {
    // Tuned so that ~400 points is very close to 1.0
    1.0 / (1.0 + (-0.0062 * x).exp())
}

// see WIDEN_C for the formula and schedule.
#[inline]
fn widen_k(parent_visits: u32, len: usize) -> usize {
    if len <= 1 {
        return len;
    }
    let n = parent_visits.max(1) as f32;
    let k = (WIDEN_C * n.sqrt()).ceil() as usize;
    k.max(1).min(len).min(MAX_WIDEN)
}

pub struct MoveNode {
    move_choice: (MoveChoice, MoveChoice),
    total_score: AtomicU64,
    visits: AtomicU32,
}

impl MoveNode {
    fn new(move_choice: (MoveChoice, MoveChoice)) -> Self {
        Self {
            move_choice,
            total_score: AtomicU64::new(0),
            visits: AtomicU32::new(0),
        }
    }

    fn add_virtual_loss(&self) {
        self.visits.fetch_add(VIRTUAL_LOSS_VISITS, Ordering::AcqRel);
    }

    fn remove_virtual_loss(&self) {
        self.visits.fetch_sub(VIRTUAL_LOSS_VISITS, Ordering::AcqRel);
    }

    fn add_result(&self, score: f32) {
        self.total_score
            .fetch_add((score * SCORE_SCALE).round() as u64, Ordering::AcqRel);
        self.visits.fetch_add(1, Ordering::AcqRel);
    }

    fn total_score_f32(&self) -> f32 {
        self.total_score.load(Ordering::Acquire) as f32 / SCORE_SCALE
    }
}

pub struct SharedNodeOptions {
    s1: Vec<MoveNode>,
    s2: Vec<MoveNode>,
}

impl SharedNodeOptions {
    fn new(
        s1_options: Vec<(MoveChoice, MoveChoice)>,
        s2_options: Vec<(MoveChoice, MoveChoice)>,
    ) -> Self {
        Self {
            s1: s1_options.into_iter().map(MoveNode::new).collect(),
            s2: s2_options.into_iter().map(MoveNode::new).collect(),
        }
    }

    // builds a SharedNodeOptions from a freshly-filled MoveOptions, ranking
    // each side's pairs by the heuristic and sorting the resulting MoveNode
    // vecs so index 0 is the highest-ranked pair. drains the reusable
    // MoveOptions buffers so allocations stay with the worker for the next node.
    fn from_ranked_move_options(state: &State, move_options: &mut MoveOptions) -> Self {
        move_options.side_one_pair_scores.clear();
        rank_side_pairs(
            state,
            SideReference::SideOne,
            &move_options.side_one_slot_a_options,
            &move_options.side_one_slot_b_options,
            &move_options.side_one_combined_options,
            &mut move_options.side_one_pair_scores,
        );
        move_options.side_two_pair_scores.clear();
        rank_side_pairs(
            state,
            SideReference::SideTwo,
            &move_options.side_two_slot_a_options,
            &move_options.side_two_slot_b_options,
            &move_options.side_two_combined_options,
            &mut move_options.side_two_pair_scores,
        );
        move_options.clear_slot_buffers();

        let s1 = build_sorted_movenodes(
            &mut move_options.side_one_combined_options,
            &mut move_options.side_one_pair_scores,
            &mut move_options.side_one_sort_indices,
        );
        let s2 = build_sorted_movenodes(
            &mut move_options.side_two_combined_options,
            &mut move_options.side_two_pair_scores,
            &mut move_options.side_two_sort_indices,
        );
        Self { s1, s2 }
    }
}

// drains `pairs` into a Vec<MoveNode> ordered by descending `scores`.
// uses `indices` as reusable scratch; both vecs are left empty on return.
fn build_sorted_movenodes(
    pairs: &mut Vec<(MoveChoice, MoveChoice)>,
    scores: &mut Vec<f32>,
    indices: &mut Vec<usize>,
) -> Vec<MoveNode> {
    let n = pairs.len();
    debug_assert_eq!(n, scores.len());
    indices.clear();
    indices.extend(0..n);
    indices.sort_by(|&a, &b| {
        scores[b]
            .partial_cmp(&scores[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut out = Vec::with_capacity(n);
    for &i in indices.iter() {
        out.push(MoveNode::new(pairs[i]));
    }
    pairs.clear();
    scores.clear();
    indices.clear();
    out
}

pub struct SharedBranch {
    nodes: Arc<[Node]>,
    total_weight: f32,
}

impl SharedBranch {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> *const Node {
        if self.nodes.len() <= 1 || self.total_weight <= 0.0 {
            return &self.nodes[0];
        }
        let mut threshold = rng.random_range(0.0..self.total_weight);
        for node in self.nodes.iter() {
            threshold -= node.instructions.percentage.max(0.0);
            if threshold <= 0.0 {
                return node;
            }
        }
        &self.nodes[self.nodes.len() - 1]
    }
}

struct PathStep {
    parent: *const Node,
    child: *const Node,
    s1_index: usize,
    s2_index: usize,
}

pub struct Node {
    root: bool,
    instructions: StateInstructions,
    depth: u8,
    times_visited: AtomicU32,

    // virtual_losses is the number of threads currently in flight through this node. added to
    // `times_visited` in select_move_pair to estimate the parent-visit count
    // for the ucb1 exploration term, which otherwise lags because
    // `times_visited` is only bumped at backprop. incremented by exactly 1 per
    // in-flight thread (unlike VIRTUAL_LOSS_VISITS) because it is a placeholder
    // for the real `times_visited += 1` that the thread will add when it
    // backpropagates
    // I8 effectively means you can't use more than 127 threads without risking overflow
    virtual_losses: AtomicI8,

    // boxed so an un-expanded node only carries a pointer slot inline
    // instead of the full SharedNodeOptions (two empty Vecs). leaves
    // outnumber internal nodes, so the inline reservation was almost always
    // dead weight. the heap alloc now only happens when a node is expanded
    options: OnceLock<Box<SharedNodeOptions>>,
}

impl Node {
    fn new_root(
        s1_options: Vec<(MoveChoice, MoveChoice)>,
        s2_options: Vec<(MoveChoice, MoveChoice)>,
    ) -> Arc<Self> {
        let node = Arc::new(Self {
            root: true,
            instructions: StateInstructions::default(),
            depth: 0,
            times_visited: AtomicU32::new(0),
            virtual_losses: AtomicI8::new(0),
            options: OnceLock::new(),
        });
        let _ = node
            .options
            .set(Box::new(SharedNodeOptions::new(s1_options, s2_options)));
        node
    }

    fn new_child(instructions: StateInstructions, depth: u8) -> Self {
        Self {
            root: false,
            instructions,
            depth,
            times_visited: AtomicU32::new(0),
            virtual_losses: AtomicI8::new(0),
            options: OnceLock::new(),
        }
    }

    fn as_key(&self) -> usize {
        self as *const Node as usize
    }

    fn ensure_options(&self, state: &State, move_options: &mut MoveOptions) -> &SharedNodeOptions {
        self.options.get_or_init(|| {
            state.get_all_options_keep_slot_buffers(move_options);
            Box::new(SharedNodeOptions::from_ranked_move_options(
                state,
                move_options,
            ))
        })
    }

    fn select_move_pair(&self, state: &State, move_options: &mut MoveOptions) -> (usize, usize) {
        let options = self.ensure_options(state, move_options);
        let parent_visits = self
            .times_visited
            .load(Ordering::Acquire)
            .saturating_add(self.virtual_losses.load(Ordering::Acquire).max(0) as u32)
            .max(1);
        (
            self.maximize_ucb_for_side(&options.s1, parent_visits),
            self.maximize_ucb_for_side(&options.s2, parent_visits),
        )
    }

    fn selection<R: Rng + ?Sized>(
        root: &Arc<Node>,
        state: &mut State,
        rng: &mut R,
        children: &ChildMap,
        path: &mut Vec<PathStep>,
        move_options: &mut MoveOptions,
    ) -> (*const Node, usize, usize) {
        // raw pointers walk both the root (a standalone Arc<Node>) and children
        // (Nodes living inside a branch's Arc<[Node]>) uniformly. every node is
        // owned by children/root for the whole search, so the pointers stay
        // valid
        let mut current: *const Node = Arc::as_ptr(root);
        loop {
            let node = unsafe { &*current };
            let (s1_index, s2_index) = node.select_move_pair(state, move_options);
            let options = node.options.get().expect("options set during selection");

            let key = (node.as_key(), s1_index, s2_index);
            match children.get(&key) {
                Some(branch) => {
                    let child = branch.sample(rng);

                    // drop the DashMap ref before mutating state to avoid
                    // holding the lock longer than necessary. the sampled node
                    // stays alive via the branch's Arc<[Node]> in the ChildMap
                    drop(branch);

                    let child_ref = unsafe { &*child };
                    options.s1[s1_index].add_virtual_loss();
                    options.s2[s2_index].add_virtual_loss();
                    child_ref.virtual_losses.fetch_add(1, Ordering::AcqRel);
                    state.apply_instructions(&child_ref.instructions.instruction_list);
                    path.push(PathStep {
                        parent: current,
                        child,
                        s1_index,
                        s2_index,
                    });
                    current = child;
                }
                None => {
                    // this is the leaf, stop selection
                    return (current, s1_index, s2_index);
                }
            }
        }
    }

    fn maximize_ucb_for_side(&self, side_options: &[MoveNode], parent_visits: u32) -> usize {
        // root bypasses progressive widening: every option is admitted from
        // the start, so the search distributes visits across the full move
        // list rather than only the heuristic top-K.
        let k = if self.root {
            side_options.len()
        } else {
            widen_k(parent_visits, side_options.len())
        };
        if k == 0 {
            return 0;
        }
        let slice = &side_options[..k];

        // pre-pass: parent's per-side mean Q over visited siblings. side-1
        // stores `score` and side-2 stores `1 - score` (see add_result), so
        // sum_score / sum_visits already lives in this side's perspective.
        let mut sum_score = 0.0f32;
        let mut sum_visits: u32 = 0;
        for node in slice {
            let v = node.visits.load(Ordering::Acquire);
            if v > 0 {
                sum_score += node.total_score_f32();
                sum_visits += v;
            }
        }
        let fpu_q = if sum_visits > 0 {
            (sum_score / sum_visits as f32 - FPU_REDUCTION).max(0.0)
        } else {
            FPU_DEFAULT_Q
        };
        let v_min = (parent_visits.max(1) as f32 / k as f32).max(1.0);
        let two_log_n = 2.0 * (parent_visits as f32).ln().max(0.0);

        // max pass; ties resolve to the earlier index (top heuristic rank)
        // because we use a strict `>` against best_ucb.
        let mut best_idx = 0usize;
        let mut best_ucb = f32::NEG_INFINITY;
        for (i, node) in slice.iter().enumerate() {
            let visits = node.visits.load(Ordering::Acquire);
            let avg = if visits > 0 {
                node.total_score_f32() / visits as f32
            } else {
                fpu_q
            };
            let denom = visits as f32 + v_min;
            let ucb = avg + (two_log_n / denom).sqrt();
            if ucb > best_ucb {
                best_idx = i;
                best_ucb = ucb;
            }
        }
        best_idx
    }

    /// looks up or creates the child branch for `(s1_index, s2_index)` and
    /// returns one sampled child, applying virtual loss bookkeeping.  Returns
    /// `None` when the node should not be expanded (depth cap reached, battle
    /// over, or both sides have no valid move).
    fn expand<R: Rng + ?Sized>(
        &self,
        state: &mut State,
        s1_index: usize,
        s2_index: usize,
        parent_is_root: bool,
        rng: &mut R,
        children: &ChildMap,
    ) -> Option<*const Node> {
        if self.depth >= MCTS_MAX_DEPTH {
            return None;
        }

        let options = self
            .options
            .get()
            .expect("options initialised before expand");
        let s1_move = &options.s1[s1_index].move_choice;
        let s2_move = &options.s2[s2_index].move_choice;

        if (state.battle_is_over() != 0.0 && !self.root)
            || (s1_move == &(MoveChoice::None, MoveChoice::None)
                && s2_move == &(MoveChoice::None, MoveChoice::None))
        {
            return None;
        }

        // faithful port of the single-threaded should_branch_on_damage:
        // the root always branches, and a node one level below the root
        // branches when either side has few combined options.
        let should_branch_on_damage = if self.root {
            true
        } else {
            (parent_is_root && options.s1.len() < 20) || options.s2.len() < 20
        };

        let instructions = generate_instructions_from_move_pair(
            state,
            &s1_move.0,
            &s1_move.1,
            &s2_move.0,
            &s2_move.1,
            should_branch_on_damage,
        );

        let mut total_weight = 0.0f32;
        let nodes = instructions
            .into_iter()
            .map(|instr| {
                total_weight += instr.percentage.max(0.0);
                // depth only increments when the end of the turn is reached,
                // matching the single-threaded engine
                let child_depth = if instr.end_of_turn_triggered {
                    self.depth.saturating_add(1)
                } else {
                    self.depth
                };
                Node::new_child(instr, child_depth)
            })
            .collect::<Arc<[Node]>>();
        let branch = SharedBranch {
            nodes,
            total_weight,
        };

        let key = (self.as_key(), s1_index, s2_index);
        // entry() on DashMap is atomic per-shard: only one thread will
        // construct the branch; all others get the winner's branch
        let branch_ref = children.entry(key).or_insert(branch);

        Some(branch_ref.sample(rng))
    }

    fn rollout(&self, state: &State, root_eval: f32) -> f32 {
        let battle_is_over = state.battle_is_over();
        if battle_is_over == 0.0 {
            sigmoid(evaluate(state) - root_eval)
        } else if battle_is_over == -1.0 {
            0.0
        } else {
            battle_is_over
        }
    }

    // walk `path` in reverse, updating visit counts and scores,
    // removes virtual losses, and reverse-applying instructions to restore `state` to how it
    // was in the root
    fn backpropagate(path: &[PathStep], leaf: &Node, score: f32, state: &mut State) {
        leaf.times_visited.fetch_add(1, Ordering::AcqRel);

        for step in path.iter().rev() {
            let (parent, child) = unsafe { (&*step.parent, &*step.child) };
            let options = parent.options.get().expect("path parent has options");
            options.s1[step.s1_index].add_result(score);
            options.s1[step.s1_index].remove_virtual_loss();
            options.s2[step.s2_index].add_result(1.0 - score);
            options.s2[step.s2_index].remove_virtual_loss();
            parent.times_visited.fetch_add(1, Ordering::AcqRel);
            child.virtual_losses.fetch_sub(1, Ordering::AcqRel);
            state.reverse_instructions(&child.instructions.instruction_list);
        }
    }
}

fn do_mcts<R: Rng + ?Sized>(
    root: &Arc<Node>,
    state: &mut State,
    root_eval: f32,
    rng: &mut R,
    children: &ChildMap,
    path: &mut Vec<PathStep>,
    move_options: &mut MoveOptions,
) {
    path.clear();

    let (leaf, s1_index, s2_index) =
        Node::selection(root, state, rng, children, path, move_options);
    let leaf = unsafe { &*leaf };

    // is the leaf's parent the root? required by the doubles
    // should_branch_on_damage heuristic. an empty path means the leaf
    // itself is the root (in which case parent_is_root is unused).
    let parent_is_root = path
        .last()
        .map(|step| unsafe { (*step.parent).root })
        .unwrap_or(false);

    let options = leaf.options.get().expect("options set during selection");
    options.s1[s1_index].add_virtual_loss();
    options.s2[s2_index].add_virtual_loss();
    let expanded = leaf.expand(state, s1_index, s2_index, parent_is_root, rng, children);
    match expanded {
        Some(child) => {
            let child = unsafe { &*child };
            child.virtual_losses.fetch_add(1, Ordering::AcqRel);
            state.apply_instructions(&child.instructions.instruction_list);
            path.push(PathStep {
                parent: leaf,
                child,
                s1_index,
                s2_index,
            });

            let score = child.rollout(state, root_eval);

            Node::backpropagate(path, child, score, state);
        }

        // if expansion returns None,
        // the battle is over, both sides have no valid moves, or the
        // depth cap was reached, so no child is added to the tree.
        // we do a rollout on the leaf and backpropagate without adding a child
        None => {
            // remove the virtual loss we added before expansion, since we're not actually expanding
            options.s1[s1_index].remove_virtual_loss();
            options.s2[s2_index].remove_virtual_loss();

            let score = leaf.rollout(state, root_eval);

            Node::backpropagate(path, leaf, score, state);
        }
    }
}

pub fn perform_mcts_shared_tree(
    state: &mut State,
    side_one_options: Vec<(MoveChoice, MoveChoice)>,
    side_two_options: Vec<(MoveChoice, MoveChoice)>,
    max_time: Duration,
    worker_count: usize,
) -> MctsResult {
    let root_eval = evaluate(state);
    let deadline = Instant::now() + max_time;
    let root = Node::new_root(side_one_options, side_two_options);
    let started_iterations = Arc::new(AtomicU32::new(0));

    // global map shared by all threads.
    let children: Arc<ChildMap> = Arc::new(DashMap::with_capacity(1 << 16));

    thread::scope(|scope| {
        for _ in 0..worker_count {
            let root = root.clone();
            let started_iterations = started_iterations.clone();
            let children = children.clone();
            let mut worker_state = state.clone();
            scope.spawn(move || {
                let mut rng = rng();
                let mut iterations_until_deadline_check = 0u32;
                let mut path = Vec::with_capacity(16);
                let mut move_options = MoveOptions::new();

                loop {
                    if iterations_until_deadline_check == 0 {
                        if Instant::now() >= deadline {
                            break;
                        }
                        iterations_until_deadline_check = MCTS_DEADLINE_CHECK_INTERVAL;
                    }
                    if started_iterations.fetch_add(1, Ordering::AcqRel)
                        >= MCTS_MAX_ITERATIONS_PER_TREE
                    {
                        break;
                    }

                    do_mcts(
                        &root,
                        &mut worker_state,
                        root_eval,
                        &mut rng,
                        &children,
                        &mut path,
                        &mut move_options,
                    );
                    iterations_until_deadline_check -= 1;
                }
            });
        }
    });

    print_tree_stats(&root, &children);

    let options = root.options.get().expect("root options initialized");
    MctsResult {
        s1: options
            .s1
            .iter()
            .map(|v| MctsSideResult {
                move_choice: v.move_choice,
                total_score: v.total_score_f32(),
                visits: v.visits.load(Ordering::Acquire),
            })
            .collect(),
        s2: options
            .s2
            .iter()
            .map(|v| MctsSideResult {
                move_choice: v.move_choice,
                total_score: v.total_score_f32(),
                visits: v.visits.load(Ordering::Acquire),
            })
            .collect(),
        iteration_count: root.times_visited.load(Ordering::Acquire),
    }
}

// debug helper: walks the built tree from `root` via the global children map
// and prints per-depth counts of nodes, leaves, and visits. a leaf is a node
// that has no entry in the children map (either depth-capped, terminal, or
// simply never re-selected after being created by its parent's expand).
//
// useful for sanity-checking how FPU / progressive-widening shape the search:
// a deep tree (visits concentrated at large depth values) suggests good
// exploitation; a shallow, wide tree (many leaves at depth 0-1) suggests the
// search is fanning out at the root without descending.
fn print_tree_stats(root: &Arc<Node>, children: &ChildMap) {
    use std::collections::{HashMap, HashSet};

    // pre-group children by parent address; walking the dashmap once and
    // bucketing avoids probing every (s1, s2) cell from each parent.
    let mut by_parent: HashMap<usize, Vec<*const Node>> = HashMap::new();
    for entry in children.iter() {
        let parent_addr = entry.key().0;
        let bucket = by_parent.entry(parent_addr).or_default();
        for child in entry.nodes.iter() {
            bucket.push(child as *const Node);
        }
    }

    let mut nodes_by_depth: HashMap<u8, u64> = HashMap::new();
    let mut leaves_by_depth: HashMap<u8, u64> = HashMap::new();
    let mut visits_by_depth: HashMap<u8, u64> = HashMap::new();
    let mut total_nodes = 0u64;

    let mut stack: Vec<*const Node> = vec![Arc::as_ptr(root)];
    let mut seen: HashSet<usize> = HashSet::new();
    seen.insert(root.as_key());

    while let Some(node_ptr) = stack.pop() {
        let node = unsafe { &*node_ptr };
        let depth = node.depth;
        let visits = node.times_visited.load(Ordering::Acquire) as u64;

        *nodes_by_depth.entry(depth).or_insert(0) += 1;
        *visits_by_depth.entry(depth).or_insert(0) += visits;
        total_nodes += 1;

        let mut had_children = false;
        if let Some(kids) = by_parent.get(&node.as_key()) {
            for &child in kids {
                had_children = true;
                let addr = child as usize;
                if seen.insert(addr) {
                    stack.push(child);
                }
            }
        }
        if !had_children {
            *leaves_by_depth.entry(depth).or_insert(0) += 1;
        }
    }

    let mut depths: Vec<u8> = nodes_by_depth.keys().copied().collect();
    depths.sort();
    let total_leaves: u64 = leaves_by_depth.values().sum();

    println!();
    println!("== Tree shape ==");
    println!("Distinct nodes: {}", total_nodes);
    println!("Leaf nodes:     {}", total_leaves);
    println!("Branch entries: {}", children.len());
    println!();
    println!(
        "{:>5}  {:>12}  {:>12}  {:>14}  {:>12}",
        "depth", "nodes", "leaves", "total_visits", "avg_visits"
    );
    for d in depths {
        let n = nodes_by_depth[&d];
        let l = leaves_by_depth.get(&d).copied().unwrap_or(0);
        let v = visits_by_depth[&d];
        let avg = if n > 0 { v as f64 / n as f64 } else { 0.0 };
        println!("{:>5}  {:>12}  {:>12}  {:>14}  {:>12.1}", d, n, l, v, avg);
    }
}
