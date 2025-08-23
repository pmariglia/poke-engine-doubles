use crate::engine::evaluate::evaluate;
use crate::engine::generate_instructions::generate_instructions_from_move_pair;
use crate::engine::state::{MoveChoice, MoveOptions};
use crate::instruction::StateInstructions;
use crate::state::State;
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use rand::thread_rng;
use std::collections::HashMap;
use std::time::Duration;

const PRUNE_ITERVAL: u32 = 100_000_000; // How many iterations before pruning the tree
const PRUNE_KEEP_COUNT: usize = 15; // Threshold for pruning based on visit counts

const MIN_VISITS_BEFORE_SELECTION: usize = 50;

fn sigmoid(x: f32) -> f32 {
    // Tuned so that ~400 points is very close to 1.0
    1.0 / (1.0 + (-0.0062 * x).exp())
}

#[derive(Debug)]
pub struct Node {
    pub root: bool,
    pub depth: u8,
    pub parent: *mut Node,
    pub children: HashMap<(usize, usize), Vec<Node>>,
    pub times_visited: u32,

    // represents the instructions & s1/s2 moves that led to this node from the parent
    pub instructions: StateInstructions,
    pub s1_choice: u16,
    pub s2_choice: u16,

    // represents the total score and number of visits for this node
    // de-coupled for s1 and s2
    pub s1_options: Option<Vec<MoveNode>>,
    pub s2_options: Option<Vec<MoveNode>>,
}

impl Node {
    fn new(depth: u8) -> Node {
        Node {
            root: false,
            depth,
            parent: std::ptr::null_mut(),
            instructions: StateInstructions::default(),
            times_visited: 0,
            children: HashMap::new(),
            s1_choice: 0,
            s2_choice: 0,
            s1_options: None,
            s2_options: None,
        }
    }
    unsafe fn populate(
        &mut self,
        s1_options: &mut Vec<(MoveChoice, MoveChoice)>,
        s2_options: &mut Vec<(MoveChoice, MoveChoice)>,
    ) {
        let s1_options_vec = s1_options
            .drain(..)
            .map(|x| MoveNode {
                move_choice: x,
                total_score: 0.0,
                visits: 0,
            })
            .collect();
        let s2_options_vec = s2_options
            .drain(..)
            .map(|x| MoveNode {
                move_choice: x,
                total_score: 0.0,
                visits: 0,
            })
            .collect();

        self.s1_options = Some(s1_options_vec);
        self.s2_options = Some(s2_options_vec);
    }

    unsafe fn should_branch_on_damage(&self) -> bool {
        if self.root {
            return true;
        }

        // if there aren't many options left, branch on damage if we're one node below the root
        if (*self.parent).root && self.s1_options.as_ref().unwrap().len() < 20
            || self.s2_options.as_ref().unwrap().len() < 20
        {
            return true;
        }
        false
    }

    pub fn maximize_ucb_for_side(&self, side_map: &[MoveNode]) -> usize {
        let mut choice = 0;
        let mut best_ucb1 = f32::MIN;
        for (index, node) in side_map.iter().enumerate() {
            let this_ucb1 = node.ucb1(self.times_visited);
            if this_ucb1 > best_ucb1 {
                best_ucb1 = this_ucb1;
                choice = index;
            }
        }
        choice
    }

    pub unsafe fn selection(
        &mut self,
        state: &mut State,
        move_options: &mut MoveOptions,
    ) -> (*mut Node, usize, usize) {
        let return_node = self as *mut Node;
        if self.s1_options.is_none() {
            state.get_all_options(move_options);
            self.populate(
                &mut move_options.side_one_combined_options,
                &mut move_options.side_two_combined_options,
            );
        }

        let s1_options = self.s1_options.as_ref().unwrap();
        let s2_options = self.s2_options.as_ref().unwrap();
        let times_visited_usize = self.times_visited as usize;
        let s1_mc_index = if s1_options.len() * MIN_VISITS_BEFORE_SELECTION > times_visited_usize {
            times_visited_usize % s1_options.len()
        } else {
            self.maximize_ucb_for_side(&s1_options)
        };
        let s2_mc_index = if s2_options.len() * MIN_VISITS_BEFORE_SELECTION > times_visited_usize {
            times_visited_usize % s2_options.len()
        } else {
            self.maximize_ucb_for_side(&s2_options)
        };

        let child_vector = self.children.get_mut(&(s1_mc_index, s2_mc_index));
        match child_vector {
            Some(child_vector) => {
                let child_vec_ptr = child_vector as *mut Vec<Node>;
                let chosen_child = self.sample_node(child_vec_ptr);
                state.apply_instructions(&(*chosen_child).instructions.instruction_list);
                (*chosen_child).selection(state, move_options)
            }
            None => (return_node, s1_mc_index, s2_mc_index),
        }
    }

    unsafe fn sample_node(&self, move_vector: *mut Vec<Node>) -> *mut Node {
        let mut rng = thread_rng();
        let weights: Vec<f64> = (*move_vector)
            .iter()
            .map(|x| x.instructions.percentage as f64)
            .collect();
        let dist = WeightedIndex::new(weights).unwrap();
        let chosen_node = &mut (&mut *move_vector)[dist.sample(&mut rng)];
        let chosen_node_ptr = chosen_node as *mut Node;
        chosen_node_ptr
    }

    pub unsafe fn expand(
        &mut self,
        state: &mut State,
        s1_move_index: usize,
        s2_move_index: usize,
    ) -> *mut Node {
        if self.depth >= 4 {
            return self as *mut Node;
        }
        let s1_move = &self.s1_options.as_ref().unwrap()[s1_move_index].move_choice;
        let s2_move = &self.s2_options.as_ref().unwrap()[s2_move_index].move_choice;
        // if the battle is over or both moves are none there is no need to expand
        if (state.battle_is_over() != 0.0 && !self.root)
            || (s1_move == &(MoveChoice::None, MoveChoice::None)
                && s2_move == &(MoveChoice::None, MoveChoice::None))
        {
            return self as *mut Node;
        }
        let should_branch_on_damage = self.should_branch_on_damage();
        let mut new_instructions = generate_instructions_from_move_pair(
            state,
            &s1_move.0,
            &s1_move.1,
            &s2_move.0,
            &s2_move.1,
            should_branch_on_damage,
        );
        let mut this_pair_vec = Vec::with_capacity(new_instructions.len());
        for state_instructions in new_instructions.drain(..) {
            let new_depth = if state_instructions.end_of_turn_triggered {
                self.depth + 1
            } else {
                self.depth
            };

            let mut new_node = Node::new(new_depth);
            new_node.parent = self;
            new_node.instructions = state_instructions;
            new_node.s1_choice = s1_move_index as u16;
            new_node.s2_choice = s2_move_index as u16;

            this_pair_vec.push(new_node);
        }

        // sample a node from the new instruction list.
        // this is the node that the rollout will be done on
        let new_node_ptr = self.sample_node(&mut this_pair_vec);
        state.apply_instructions(&(*new_node_ptr).instructions.instruction_list);
        self.children
            .insert((s1_move_index, s2_move_index), this_pair_vec);
        new_node_ptr
    }

    pub unsafe fn backpropagate(&mut self, score: f32, state: &mut State) {
        self.times_visited += 1;
        if self.root {
            return;
        }

        let parent_s1_movenode =
            &mut (*self.parent).s1_options.as_mut().unwrap()[self.s1_choice as usize];
        parent_s1_movenode.total_score += score;
        parent_s1_movenode.visits += 1;

        let parent_s2_movenode =
            &mut (*self.parent).s2_options.as_mut().unwrap()[self.s2_choice as usize];
        parent_s2_movenode.total_score += 1.0 - score;
        parent_s2_movenode.visits += 1;

        state.reverse_instructions(&self.instructions.instruction_list);
        (*self.parent).backpropagate(score, state);
    }

    pub fn rollout(&mut self, state: &mut State, root_eval: &f32) -> f32 {
        let battle_is_over = state.battle_is_over();
        if battle_is_over == 0.0 {
            let eval = evaluate(state);
            sigmoid(eval - root_eval)
        } else {
            if battle_is_over == -1.0 {
                0.0
            } else {
                battle_is_over
            }
        }
    }
}

#[derive(Debug)]
pub struct MoveNode {
    pub move_choice: (MoveChoice, MoveChoice),
    pub total_score: f32,
    pub visits: u32,
}

impl MoveNode {
    pub fn ucb1(&self, parent_visits: u32) -> f32 {
        if self.visits == 0 {
            return f32::INFINITY;
        }
        let score = (self.total_score / self.visits as f32)
            + (2.0 * (parent_visits as f32).ln() / self.visits as f32).sqrt();
        score
    }
    pub fn average_score(&self) -> f32 {
        let score = self.total_score / self.visits as f32;
        score
    }
}

#[derive(Clone)]
pub struct MctsSideResult {
    pub move_choice: (MoveChoice, MoveChoice),
    pub total_score: f32,
    pub visits: u32,
}

impl MctsSideResult {
    pub fn average_score(&self) -> f32 {
        if self.visits == 0 {
            return 0.0;
        }
        let score = self.total_score / self.visits as f32;
        score
    }
}

pub struct MctsResult {
    pub s1: Vec<MctsSideResult>,
    pub s2: Vec<MctsSideResult>,
    pub iteration_count: u32,
}

unsafe fn prune_tree(root_node: &mut Node) {
    if root_node.s1_options.as_ref().unwrap().len() <= PRUNE_KEEP_COUNT
        || root_node.s2_options.as_ref().unwrap().len() <= PRUNE_KEEP_COUNT
    {
        return;
    }

    let mut s1_with_indices: Vec<(usize, u32)> = root_node
        .s1_options
        .as_ref()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(idx, move_node)| (idx, move_node.visits))
        .collect();
    s1_with_indices.sort_by_key(|&(_, visits)| visits);

    let mut s1_removed_set: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut s1_indices_to_remove: Vec<usize> = s1_with_indices
        .iter()
        .take(s1_with_indices.len() - PRUNE_KEEP_COUNT)
        .map(|&(idx, _)| {
            s1_removed_set.insert(idx);
            idx
        })
        .collect();

    s1_indices_to_remove.sort();
    for idx in s1_indices_to_remove.iter().rev() {
        root_node.s1_options.as_mut().unwrap().remove(*idx);
    }

    let mut s2_with_indices: Vec<(usize, u32)> = root_node
        .s2_options
        .as_ref()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(idx, move_node)| (idx, move_node.visits))
        .collect();
    s2_with_indices.sort_by_key(|&(_, visits)| visits);

    let mut s2_removed_set: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut s2_indices_to_remove: Vec<usize> = s2_with_indices
        .iter()
        .take(s2_with_indices.len() - PRUNE_KEEP_COUNT)
        .map(|&(idx, _)| {
            s2_removed_set.insert(idx);
            idx
        })
        .collect();
    s2_indices_to_remove.sort();
    for idx in s2_indices_to_remove.iter().rev() {
        root_node.s2_options.as_mut().unwrap().remove(*idx);
    }

    // Remove children from HashMap where either s1 or s2 index was pruned, and remap remaining keys
    let old_children = std::mem::take(&mut root_node.children);
    for ((old_s1_idx, old_s2_idx), mut children) in old_children {
        // Skip if either index was removed
        if s1_removed_set.contains(&old_s1_idx) || s2_removed_set.contains(&old_s2_idx) {
            continue;
        }

        // Calculate new indices by counting how many lower indices were removed
        let new_s1_idx = old_s1_idx
            - s1_removed_set
                .iter()
                .filter(|&&removed| removed < old_s1_idx)
                .count();
        let new_s2_idx = old_s2_idx
            - s2_removed_set
                .iter()
                .filter(|&&removed| removed < old_s2_idx)
                .count();

        // Update child node references
        for child in &mut children {
            child.s1_choice = new_s1_idx as u16;
            child.s2_choice = new_s2_idx as u16;
        }

        root_node
            .children
            .insert((new_s1_idx, new_s2_idx), children);
    }

    // Recalculate times visited for the root node
    // calculate it by summing the visits of all CHILDREN
    root_node.times_visited = root_node
        .children
        .values()
        .flatten()
        .map(|child| child.times_visited)
        .sum();
}

fn do_mcts(
    root_node: &mut Node,
    state: &mut State,
    root_eval: &f32,
    move_options: &mut MoveOptions,
) {
    let (mut new_node, s1_move, s2_move) = unsafe { root_node.selection(state, move_options) };
    new_node = unsafe { (*new_node).expand(state, s1_move, s2_move) };
    let rollout_result = unsafe { (*new_node).rollout(state, root_eval) };
    unsafe { (*new_node).backpropagate(rollout_result, state) }
}

pub fn perform_mcts(
    state: &mut State,
    mut side_one_options: Vec<(MoveChoice, MoveChoice)>,
    mut side_two_options: Vec<(MoveChoice, MoveChoice)>,
    max_time: Duration,
) -> MctsResult {
    let mut root_node = Node::new(0);
    unsafe {
        root_node.populate(&mut side_one_options, &mut side_two_options);
    }
    root_node.root = true;

    let mut combined_options = MoveOptions::new();
    let root_eval = evaluate(state);
    let start_time = std::time::Instant::now();
    while start_time.elapsed() < max_time {
        for _ in 0..1000 {
            do_mcts(&mut root_node, state, &root_eval, &mut combined_options);
        }

        if root_node.times_visited == PRUNE_ITERVAL {
            unsafe {
                prune_tree(&mut root_node);
            }
        }

        /*
        Cut off after 25 million iterations

        Under normal circumstances the bot will only run for 2.5-3.5 million iterations
        however towards the end of a battle the bot may perform tens of millions of iterations

        Beyond about 30 million iterations some floating point nonsense happens where
        MoveNode.total_score stops updating because f32 does not have enough precision

        I can push the problem farther out by using f64 but if the bot is running for this long
        then it almost certainly sees a forced win
        */
        if root_node.times_visited == 25_000_000 {
            break;
        }
    }

    let result = MctsResult {
        s1: root_node
            .s1_options
            .as_ref()
            .unwrap()
            .iter()
            .map(|v| MctsSideResult {
                move_choice: v.move_choice.clone(),
                total_score: v.total_score,
                visits: v.visits,
            })
            .collect(),
        s2: root_node
            .s2_options
            .as_ref()
            .unwrap()
            .iter()
            .map(|v| MctsSideResult {
                move_choice: v.move_choice.clone(),
                total_score: v.total_score,
                visits: v.visits,
            })
            .collect(),
        iteration_count: root_node.times_visited,
    };

    result
}

pub fn _analyze_leaf_node_depths(root_node: &Node) {
    let mut depth_counts: HashMap<usize, usize> = HashMap::new();

    fn collect_leaf_depths(node: &Node, depth_counts: &mut HashMap<usize, usize>) {
        // A leaf node is one with no children
        if node.children.is_empty() {
            *depth_counts.entry(node.depth as usize).or_insert(0) += 1;
        } else {
            // Recursively traverse all children
            for child_vector in node.children.values() {
                for child in child_vector {
                    collect_leaf_depths(child, depth_counts);
                }
            }
        }
    }

    collect_leaf_depths(root_node, &mut depth_counts);

    // Print results sorted by depth
    let mut depths: Vec<_> = depth_counts.keys().collect();
    depths.sort();

    println!("Leaf node count by depth:");
    for &depth in depths {
        println!("Depth {}: {} leaf nodes", depth, depth_counts[&depth]);
    }

    let total_leaves: usize = depth_counts.values().sum();
    println!("Total leaf nodes: {}", total_leaves);
}
