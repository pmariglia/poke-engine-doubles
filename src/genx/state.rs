use super::abilities::Abilities;
use super::choice_effects::charge_volatile_to_choice;
use super::items::Items;
use crate::choices::{Choice, Choices, MoveCategory, MoveChoiceTarget, MoveTarget};
use crate::define_enum_with_from_str;
use crate::instruction::{
    ChangeSideConditionInstruction, ChangeStatInstruction, ChangeType,
    ChangeVolatileStatusDurationInstruction, Instruction, RemoveVolatileStatusInstruction,
    StateInstructions,
};
use crate::pokemon::PokemonName;
use crate::state::{
    LastUsedMove, Pokemon, PokemonBoostableStat, PokemonIndex, PokemonMoveIndex,
    PokemonSideCondition, PokemonStatus, PokemonType, Side, SideReference, SideSlot, SlotReference,
    State,
};
use core::panic;
use std::collections::HashSet;
use std::str::FromStr;

fn common_pkmn_stat_calc(stat: u16, ev: u16, level: u16) -> u16 {
    // 31 IV always used
    ((2 * stat + 31 + (ev / 4)) * level) / 100
}

fn multiply_boost(boost_num: i8, stat_value: i16) -> i16 {
    match boost_num {
        -6 => stat_value * 2 / 8,
        -5 => stat_value * 2 / 7,
        -4 => stat_value * 2 / 6,
        -3 => stat_value * 2 / 5,
        -2 => stat_value * 2 / 4,
        -1 => stat_value * 2 / 3,
        0 => stat_value,
        1 => stat_value * 3 / 2,
        2 => stat_value * 4 / 2,
        3 => stat_value * 5 / 2,
        4 => stat_value * 6 / 2,
        5 => stat_value * 7 / 2,
        6 => stat_value * 8 / 2,
        _ => panic!("Invalid boost number: {}", boost_num),
    }
}

pub struct MoveOptions {
    pub side_one_slot_a_options: Vec<MoveChoice>,
    pub side_one_slot_b_options: Vec<MoveChoice>,
    pub side_two_slot_a_options: Vec<MoveChoice>,
    pub side_two_slot_b_options: Vec<MoveChoice>,
    pub side_one_combined_options: Vec<(MoveChoice, MoveChoice)>,
    pub side_two_combined_options: Vec<(MoveChoice, MoveChoice)>,
}

impl MoveOptions {
    pub fn new() -> MoveOptions {
        MoveOptions {
            side_one_slot_a_options: Vec::with_capacity(9),
            side_one_slot_b_options: Vec::with_capacity(9),
            side_two_slot_a_options: Vec::with_capacity(9),
            side_two_slot_b_options: Vec::with_capacity(9),
            side_one_combined_options: Vec::with_capacity(81),
            side_two_combined_options: Vec::with_capacity(81),
        }
    }
    pub fn combine_slot_options(&mut self) {
        MoveOptions::combine_side_slot_options(
            &mut self.side_one_slot_a_options,
            &mut self.side_one_slot_b_options,
            &mut self.side_one_combined_options,
        );
        MoveOptions::combine_side_slot_options(
            &mut self.side_two_slot_a_options,
            &mut self.side_two_slot_b_options,
            &mut self.side_two_combined_options,
        );
    }
    pub fn combine_side_slot_options(
        slot_a_options: &mut Vec<MoveChoice>,
        slot_b_options: &mut Vec<MoveChoice>,
        combined_options: &mut Vec<(MoveChoice, MoveChoice)>,
    ) {
        let capacity = slot_a_options.len() * slot_b_options.len();
        for slot_a_choice in slot_a_options.iter() {
            for slot_b_choice in slot_b_options.iter() {
                // Check if both slots are trying to switch to the same Pokémon
                if let (MoveChoice::Switch(pkmn_a), MoveChoice::Switch(pkmn_b)) =
                    (slot_a_choice, slot_b_choice)
                {
                    if pkmn_a == pkmn_b {
                        // Skip this combination - can't switch to the same Pokémon
                        continue;
                    }
                }

                // Check if both slots are trying to terastallize
                if matches!(slot_a_choice, MoveChoice::MoveTera(_, _, _))
                    && matches!(slot_b_choice, MoveChoice::MoveTera(_, _, _))
                {
                    // Skip this combination - both Pokémon cannot terastallize together
                    continue;
                }

                combined_options.push((*slot_a_choice, *slot_b_choice));
            }
        }

        // If no valid combined_options exist, add None as fallback
        if combined_options.is_empty() {
            if capacity == 1
                && slot_a_options[0] == slot_b_options[0]
                && matches!(slot_a_options[0], MoveChoice::Switch(_))
            {
                combined_options.push((slot_a_options[0], MoveChoice::None));
            } else {
                combined_options.push((MoveChoice::None, MoveChoice::None));
            }
        }
        slot_a_options.clear();
        slot_b_options.clear();
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum MoveChoice {
    MoveTera(SlotReference, SideReference, PokemonMoveIndex),
    Move(SlotReference, SideReference, PokemonMoveIndex),
    Switch(PokemonIndex),
    None,
}

impl MoveChoice {
    // needs to return a unique u8 for each possible MoveChoice
    pub fn to_u8(&self) -> u8 {
        match self {
            // 4 moves, 2 slots, 2 sides = 16 => 0-15
            MoveChoice::MoveTera(slot, side, mv) => {
                (*slot as u8) << 3 | (*side as u8) << 2 | (*mv as u8)
            }
            // 4 moves, 2 slots, 2 sides = 16 => 16-31
            MoveChoice::Move(slot, side, mv) => {
                0b00010000 | (*slot as u8) << 3 | (*side as u8) << 2 | (*mv as u8)
            }
            // 6 switches (0-5) = 6 => 32-37
            MoveChoice::Switch(index) => 0b00100000 | (*index as u8),
            MoveChoice::None => 0b00100110, // 38
        }
    }

    pub fn serialize(&self) -> String {
        match self {
            MoveChoice::MoveTera(target_slot, target_side, index) => {
                format!(
                    "{},{},{},true",
                    target_side.to_string(),
                    target_slot.to_string(),
                    index.serialize()
                )
            }
            MoveChoice::Move(target_slot, target_side, index) => {
                format!(
                    "{},{},{},false",
                    target_side.to_string(),
                    target_slot.to_string(),
                    index.serialize()
                )
            }
            MoveChoice::Switch(index) => format!("{}", index.serialize()),
            MoveChoice::None => "none".to_string(),
        }
    }
    pub fn deserialize(s: &str) -> MoveChoice {
        let parts = s.split(",").collect::<Vec<&str>>();
        if parts.len() == 4 {
            let target_side = SideReference::from_str(parts[0]).unwrap();
            let target_slot = SlotReference::from_str(parts[1]).unwrap();
            let index: PokemonMoveIndex = PokemonMoveIndex::deserialize(parts[2]);
            let is_tera: bool = parts[3].parse().unwrap();
            if is_tera {
                MoveChoice::MoveTera(target_slot, target_side, index)
            } else {
                MoveChoice::Move(target_slot, target_side, index)
            }
        } else if parts.len() == 1 && parts[0].to_lowercase() == "none" {
            MoveChoice::None
        } else if parts.len() == 1 {
            MoveChoice::Switch(PokemonIndex::deserialize(parts[0]))
        } else {
            panic!("Invalid MoveChoice serialization: {}", s);
        }
    }
    pub fn to_string(&self, side: &Side, attacking_slot_ref: &SlotReference) -> String {
        match self {
            MoveChoice::MoveTera(target_slot, target_side, index) => {
                let mv = &side.get_active_immutable(attacking_slot_ref).moves[&index];
                if mv.choice.move_choice_target == MoveChoiceTarget::Ally
                    || (mv.choice.move_choice_target == MoveChoiceTarget::Normal
                        && mv.choice.target == MoveTarget::Target)
                {
                    format!(
                        "{},{},{},tera",
                        mv.id,
                        target_side.to_string(),
                        target_slot.to_string()
                    )
                    .to_lowercase()
                } else {
                    format!("{},tera", mv.id).to_lowercase()
                }
            }
            MoveChoice::Move(target_slot, target_side, index) => {
                let mv = &side.get_active_immutable(attacking_slot_ref).moves[&index];
                if mv.choice.move_choice_target == MoveChoiceTarget::Ally
                    || (mv.choice.move_choice_target == MoveChoiceTarget::Normal
                        && mv.choice.target == MoveTarget::Target)
                {
                    format!(
                        "{},{},{}",
                        mv.id,
                        target_side.to_string(),
                        target_slot.to_string()
                    )
                    .to_lowercase()
                } else {
                    format!("{}", mv.id).to_lowercase()
                }
            }
            MoveChoice::Switch(index) => format!("{}", side.pokemon[*index].id).to_lowercase(),
            MoveChoice::None => "No Move".to_string(),
        }
    }
    pub fn from_string(
        s: &str,
        side: &Side,
        attacking_slot_ref: SlotReference,
    ) -> Option<MoveChoice> {
        let mut s = s.to_lowercase();
        if s.starts_with("none") {
            return Some(MoveChoice::None);
        }

        let mut pkmn_iter = side.pokemon.into_iter();
        while let Some(pkmn) = pkmn_iter.next() {
            if pkmn.id.to_string().to_lowercase() == s
                && pkmn_iter.pokemon_index
                    != side.get_slot_immutable(&attacking_slot_ref).active_index
            {
                return Some(MoveChoice::Switch(pkmn_iter.pokemon_index));
            }
        }

        let is_tera = if s.ends_with(",tera") {
            s = s.trim_end_matches(",tera").to_string();
            true
        } else {
            false
        };
        let parts: Vec<&str> = s.split(',').collect();
        let move_name_part;
        let side_ref_str;
        let slot_ref_str;
        if parts.len() == 3 {
            move_name_part = parts[0];
            side_ref_str = parts[1];
            slot_ref_str = parts[2];
        } else if parts.len() == 1 {
            // if they don't provide side/slot target, assume the first slot
            // for spread moves or moves that hit everyone, this will be overwritten later
            move_name_part = parts[0];
            side_ref_str = "1";
            slot_ref_str = "a";
        } else {
            return None;
        }

        let target_side_ref = match SideReference::from_str(side_ref_str) {
            Ok(side_ref) => side_ref,
            Err(_) => return None,
        };
        let target_slot_ref = match SlotReference::from_str(slot_ref_str) {
            Ok(slot_ref) => slot_ref,
            Err(_) => return None,
        };

        let mut move_iter = side
            .get_active_immutable(&attacking_slot_ref)
            .moves
            .into_iter();
        let move_name = move_name_part.to_string();
        while let Some(mv) = move_iter.next() {
            if format!("{:?}", mv.id).to_lowercase() == move_name {
                return if is_tera {
                    Some(MoveChoice::MoveTera(
                        target_slot_ref,
                        target_side_ref,
                        move_iter.pokemon_move_index,
                    ))
                } else {
                    Some(MoveChoice::Move(
                        target_slot_ref,
                        target_side_ref,
                        move_iter.pokemon_move_index,
                    ))
                };
            }
        }

        None
    }
}

define_enum_with_from_str! {
    #[repr(u8)]
    #[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
    PokemonVolatileStatus {
        NONE,
        AQUARING,
        ATTRACT,
        AUTOTOMIZE,
        BANEFULBUNKER,
        BIDE,
        BOUNCE,
        BURNINGBULWARK,
        CHARGE,
        COMMANDED,
        COMMANDING,
        CONFUSION,
        CURSE,
        DEFENSECURL,
        DESTINYBOND,
        DIG,
        DISABLE,
        DIVE,
        ELECTRIFY,
        ELECTROSHOT,
        EMBARGO,
        ENCORE,
        ENDURE,
        FLASHFIRE,
        FLINCH,
        FLY,
        FOCUSENERGY,
        FOLLOWME,
        FORESIGHT,
        FREEZESHOCK,
        GASTROACID,
        GEOMANCY,
        GLAIVERUSH,
        GRUDGE,
        HEALBLOCK,
        HELPINGHAND,
        ICEBURN,
        IMPRISON,
        INGRAIN,
        KINGSSHIELD,
        LASERFOCUS,
        LEECHSEED,
        LIGHTSCREEN,
        LOCKEDMOVE,
        MAGICCOAT,
        MAGNETRISE,
        MAXGUARD,
        METEORBEAM,
        MINIMIZE,
        MIRACLEEYE,
        MUSTRECHARGE,
        NIGHTMARE,
        NORETREAT,
        OCTOLOCK,
        PARTIALLYTRAPPED,
        PERISH4,
        PERISH3,
        PERISH2,
        PERISH1,
        PHANTOMFORCE,
        POWDER,
        POWERSHIFT,
        POWERTRICK,
        PROTECT,
        PROTOSYNTHESISATK,
        PROTOSYNTHESISDEF,
        PROTOSYNTHESISSPA,
        PROTOSYNTHESISSPD,
        PROTOSYNTHESISSPE,
        QUARKDRIVEATK,
        QUARKDRIVEDEF,
        QUARKDRIVESPA,
        QUARKDRIVESPD,
        QUARKDRIVESPE,
        RAGE,
        RAGEPOWDER,
        RAZORWIND,
        REFLECT,
        ROOST,
        SALTCURE,
        SHADOWFORCE,
        SKULLBASH,
        SKYATTACK,
        SKYDROP,
        SILKTRAP,
        SLOWSTART,
        SMACKDOWN,
        SNATCH,
        SOLARBEAM,
        SOLARBLADE,
        SPARKLINGARIA,
        SPIKYSHIELD,
        SPOTLIGHT,
        STOCKPILE,
        SUBSTITUTE,
        SYRUPBOMB,
        TARSHOT,
        TAUNT,
        TELEKINESIS,
        THROATCHOP,
        TRUANT,
        TORMENT,
        TYPECHANGE,
        UNBURDEN,
        UPROAR,
        YAWN,
    },
    default = NONE
}

define_enum_with_from_str! {
    #[repr(u8)]
    #[derive(Debug, PartialEq, Copy, Clone)]
    Weather {
        NONE,
        SUN,
        RAIN,
        SAND,
        HAIL,
        SNOW,
        HARSHSUN,
        HEAVYRAIN,
    }
}

define_enum_with_from_str! {
    #[repr(u8)]
    #[derive(Debug, PartialEq, Copy, Clone)]
    Terrain {
        NONE,
        ELECTRICTERRAIN,
        PSYCHICTERRAIN,
        MISTYTERRAIN,
        GRASSYTERRAIN,
    }
}

impl Pokemon {
    pub fn recalculate_stats(
        &mut self,
        side_ref: SideReference,
        pokemon_index: PokemonIndex,
        instructions: &mut StateInstructions,
    ) {
        // recalculate stats from base-stats and push any changes made to the StateInstructions
        let stats = self.calculate_stats_from_base_stats();
        if stats.1 != self.attack {
            let ins = Instruction::ChangeAttack(ChangeStatInstruction {
                side_ref,
                pokemon_index,
                amount: stats.1 - self.attack,
            });
            self.attack = stats.1;
            instructions.instruction_list.push(ins);
        }
        if stats.2 != self.defense {
            let ins = Instruction::ChangeDefense(ChangeStatInstruction {
                side_ref,
                pokemon_index,
                amount: stats.2 - self.defense,
            });
            self.defense = stats.2;
            instructions.instruction_list.push(ins);
        }
        if stats.3 != self.special_attack {
            let ins = Instruction::ChangeSpecialAttack(ChangeStatInstruction {
                side_ref,
                pokemon_index,
                amount: stats.3 - self.special_attack,
            });
            self.special_attack = stats.3;
            instructions.instruction_list.push(ins);
        }
        if stats.4 != self.special_defense {
            let ins = Instruction::ChangeSpecialDefense(ChangeStatInstruction {
                side_ref,
                pokemon_index,
                amount: stats.4 - self.special_defense,
            });
            self.special_defense = stats.4;
            instructions.instruction_list.push(ins);
        }
        if stats.5 != self.speed {
            let ins = Instruction::ChangeSpeed(ChangeStatInstruction {
                side_ref,
                pokemon_index,
                amount: stats.5 - self.speed,
            });
            self.speed = stats.5;
            instructions.instruction_list.push(ins);
        }
    }
    pub fn recalculate_stats_without_updating_stats(
        &self,
        side_ref: SideReference,
        pokemon_index: PokemonIndex,
        instructions: &mut StateInstructions,
    ) {
        // recalculate stats from base-stats and push any changes made to the StateInstructions
        let stats = self.calculate_stats_from_base_stats();
        if stats.1 != self.attack {
            let ins = Instruction::ChangeAttack(ChangeStatInstruction {
                side_ref,
                pokemon_index,
                amount: stats.1 - self.attack,
            });
            instructions.instruction_list.push(ins);
        }
        if stats.2 != self.defense {
            let ins = Instruction::ChangeDefense(ChangeStatInstruction {
                side_ref,
                pokemon_index,
                amount: stats.2 - self.defense,
            });
            instructions.instruction_list.push(ins);
        }
        if stats.3 != self.special_attack {
            let ins = Instruction::ChangeSpecialAttack(ChangeStatInstruction {
                side_ref,
                pokemon_index,
                amount: stats.3 - self.special_attack,
            });
            instructions.instruction_list.push(ins);
        }
        if stats.4 != self.special_defense {
            let ins = Instruction::ChangeSpecialDefense(ChangeStatInstruction {
                side_ref,
                pokemon_index,
                amount: stats.4 - self.special_defense,
            });
            instructions.instruction_list.push(ins);
        }
        if stats.5 != self.speed {
            let ins = Instruction::ChangeSpeed(ChangeStatInstruction {
                side_ref,
                pokemon_index,
                amount: stats.5 - self.speed,
            });
            instructions.instruction_list.push(ins);
        }
    }
    pub fn calculate_stats_from_base_stats(&self) -> (i16, i16, i16, i16, i16, i16) {
        let base_stats = self.id.base_stats();
        (
            (common_pkmn_stat_calc(base_stats.0 as u16, self.evs.0 as u16, self.level as u16)
                + self.level as u16
                + 10) as i16,
            (common_pkmn_stat_calc(base_stats.1 as u16, self.evs.1 as u16, self.level as u16) + 5)
                as i16,
            (common_pkmn_stat_calc(base_stats.2 as u16, self.evs.2 as u16, self.level as u16) + 5)
                as i16,
            (common_pkmn_stat_calc(base_stats.3 as u16, self.evs.3 as u16, self.level as u16) + 5)
                as i16,
            (common_pkmn_stat_calc(base_stats.4 as u16, self.evs.4 as u16, self.level as u16) + 5)
                as i16,
            (common_pkmn_stat_calc(base_stats.5 as u16, self.evs.5 as u16, self.level as u16) + 5)
                as i16,
        )
    }

    fn add_moves_from_opponent_targets(
        &self,
        vec: &mut Vec<MoveChoice>,
        opponent_side_ref: SideReference,
        opponent_slot_a_can_target: bool,
        opponent_slot_b_can_target: bool,
        pkmn_move_index: PokemonMoveIndex,
        can_tera: bool,
    ) {
        if opponent_slot_a_can_target {
            vec.push(MoveChoice::Move(
                SlotReference::SlotA,
                opponent_side_ref,
                pkmn_move_index,
            ));
            if can_tera {
                vec.push(MoveChoice::MoveTera(
                    SlotReference::SlotA,
                    opponent_side_ref,
                    pkmn_move_index,
                ));
            }
        }
        if opponent_slot_b_can_target {
            vec.push(MoveChoice::Move(
                SlotReference::SlotB,
                opponent_side_ref,
                pkmn_move_index,
            ));
            if can_tera {
                vec.push(MoveChoice::MoveTera(
                    SlotReference::SlotB,
                    opponent_side_ref,
                    pkmn_move_index,
                ));
            }
        }
    }

    fn add_single_target_move(
        &self,
        side_ref: SideReference,
        slot_ref: &SlotReference,
        last_used_move: &LastUsedMove,
        vec: &mut Vec<MoveChoice>,
        opponent_targets: (bool, bool),
        move_choice: &Choice,
        pokemon_move_index: PokemonMoveIndex,
        partner: &Pokemon,
        mut can_tera: bool,
    ) {
        // Conditionally add single target moves to the vec
        // checks if it makes sense to use this move

        // don't consider protect+tera together
        if move_choice.move_id == Choices::PROTECT {
            can_tera = false
        }
        let partner_alive = partner.hp > 0;
        match move_choice.move_id {
            // Fakeout: Only makes sense if you just switched in
            Choices::FAKEOUT | Choices::FIRSTIMPRESSION => match last_used_move {
                LastUsedMove::Switch(_) => self.add_moves_from_opponent_targets(
                    vec,
                    side_ref.get_other_side(),
                    opponent_targets.0,
                    opponent_targets.1,
                    pokemon_move_index,
                    can_tera,
                ),
                _ => {}
            },
            // Decorate: only targeting ally makes sense
            Choices::DECORATE if partner_alive => {
                vec.push(MoveChoice::Move(
                    slot_ref.get_other_slot(),
                    side_ref,
                    pokemon_move_index,
                ));
                if can_tera {
                    vec.push(MoveChoice::MoveTera(
                        slot_ref.get_other_slot(),
                        side_ref,
                        pokemon_move_index,
                    ));
                }
            }
            // targeting ally makes sense
            Choices::BEATUP
            | Choices::POPULATIONBOMB
            | Choices::SCALESHOT
            | Choices::ICICLESPEAR
            | Choices::POPULATIONBOMB
            | Choices::BULLETSEED
                if partner_alive =>
            {
                if partner.has_move(Choices::RAGEFIST) || partner.ability == Abilities::STAMINA {
                    vec.push(MoveChoice::Move(
                        slot_ref.get_other_slot(),
                        side_ref,
                        pokemon_move_index,
                    ));
                    if can_tera {
                        vec.push(MoveChoice::MoveTera(
                            slot_ref.get_other_slot(),
                            side_ref,
                            pokemon_move_index,
                        ));
                    }
                }
                self.add_moves_from_opponent_targets(
                    vec,
                    side_ref.get_other_side(),
                    opponent_targets.0,
                    opponent_targets.1,
                    pokemon_move_index,
                    can_tera,
                )
            }
            Choices::POLLENPUFF | Choices::FLORALHEALING if partner_alive => {
                vec.push(MoveChoice::Move(
                    slot_ref.get_other_slot(),
                    side_ref,
                    pokemon_move_index,
                ));
                if can_tera {
                    vec.push(MoveChoice::MoveTera(
                        slot_ref.get_other_slot(),
                        side_ref,
                        pokemon_move_index,
                    ));
                }
                self.add_moves_from_opponent_targets(
                    vec,
                    side_ref.get_other_side(),
                    opponent_targets.0,
                    opponent_targets.1,
                    pokemon_move_index,
                    can_tera,
                )
            }
            // default: only try to target opponents with single-target moves
            _ => self.add_moves_from_opponent_targets(
                vec,
                side_ref.get_other_side(),
                opponent_targets.0,
                opponent_targets.1,
                pokemon_move_index,
                can_tera,
            ),
        }
    }

    pub fn add_available_moves(
        &self,
        side_ref: SideReference,
        slot_ref: &SlotReference,
        vec: &mut Vec<MoveChoice>,
        last_used_move: &LastUsedMove,
        opponent_targets: (bool, bool),
        charging_mv_index: Option<PokemonMoveIndex>,
        partner: &Pokemon,
        encored: bool,
        disabled: bool,
        taunted: bool,
        can_tera: bool,
    ) {
        let cannot_use_status_moves = self.item == Items::ASSAULTVEST || taunted;

        let mut iter = self.moves.into_iter();
        while let Some(p) = iter.next() {
            if !p.disabled && p.pp > 0 {
                // disqualifying conditions for a move
                let should_skip = match last_used_move {
                    LastUsedMove::Move(last_used_move_index) => {
                        // encore: must use the last used move
                        (encored && last_used_move_index != &iter.pokemon_move_index) ||
                            // disable: cannot use the last used move
                            (disabled && last_used_move_index == &iter.pokemon_move_index) ||
                            // bloodmoon/gigatonhammer: cannot use consecutively
                            ((self.moves[last_used_move_index].id == Choices::BLOODMOON ||
                                self.moves[last_used_move_index].id == Choices::GIGATONHAMMER) &&
                                &iter.pokemon_move_index == last_used_move_index)
                    }
                    _ => false,
                } || (cannot_use_status_moves
                    && p.choice.category == MoveCategory::Status)
                    || charging_mv_index.map_or(false, |idx| idx != iter.pokemon_move_index);

                if should_skip {
                    continue;
                }

                let move_choice = &p.choice;
                // Handle move targeting based on MoveChoiceTarget
                match move_choice.move_choice_target {
                    MoveChoiceTarget::Normal if move_choice.target == MoveTarget::Target => {
                        self.add_single_target_move(
                            side_ref,
                            slot_ref,
                            last_used_move,
                            vec,
                            opponent_targets,
                            move_choice,
                            iter.pokemon_move_index,
                            partner,
                            can_tera,
                        );
                    }
                    MoveChoiceTarget::Ally => {
                        vec.push(MoveChoice::Move(
                            slot_ref.get_other_slot(),
                            side_ref,
                            iter.pokemon_move_index,
                        ));

                        if can_tera {
                            vec.push(MoveChoice::MoveTera(
                                slot_ref.get_other_slot(),
                                side_ref,
                                iter.pokemon_move_index,
                            ));
                        }
                    }
                    _ => {
                        // For other moves - use input slot_ref and side_ref
                        // since separate logic will handle the actual targeting
                        vec.push(MoveChoice::Move(
                            *slot_ref,
                            side_ref,
                            iter.pokemon_move_index,
                        ));

                        if can_tera {
                            vec.push(MoveChoice::MoveTera(
                                *slot_ref,
                                side_ref,
                                iter.pokemon_move_index,
                            ));
                        }
                    }
                }
            }
        }
    }

    #[cfg(feature = "terastallization")]
    pub fn has_type(&self, pkmn_type: &PokemonType) -> bool {
        if self.terastallized && self.tera_type != PokemonType::STELLAR {
            pkmn_type == &self.tera_type
        } else {
            pkmn_type == &self.types.0 || pkmn_type == &self.types.1
        }
    }

    #[cfg(not(feature = "terastallization"))]
    pub fn has_type(&self, pkmn_type: &PokemonType) -> bool {
        pkmn_type == &self.types.0 || pkmn_type == &self.types.1
    }

    pub fn has_move(&self, pkmn_move: Choices) -> bool {
        for mv in self.moves.into_iter() {
            if mv.id == pkmn_move {
                return true;
            }
        }
        false
    }

    pub fn immune_to_rage_powder_redirection(&self) -> bool {
        if self.ability == Abilities::OVERCOAT
            || self.item == Items::SAFETYGOGGLES
            || self.has_type(&PokemonType::GRASS)
        {
            return true;
        }
        false
    }

    pub fn redirects_move_to_self(&self, choice: &Choice) -> bool {
        if self.ability == Abilities::LIGHTNINGROD && choice.move_type == PokemonType::ELECTRIC {
            return true;
        } else if self.ability == Abilities::STORMDRAIN && choice.move_type == PokemonType::WATER {
            return true;
        }
        false
    }

    pub fn item_is_permanent(&self) -> bool {
        match self.item {
            Items::LUSTROUSGLOBE => self.id == PokemonName::PALKIAORIGIN,
            Items::GRISEOUSCORE => self.id == PokemonName::GIRATINAORIGIN,
            Items::ADAMANTCRYSTAL => self.id == PokemonName::DIALGAORIGIN,
            Items::RUSTEDSWORD => {
                self.id == PokemonName::ZACIANCROWNED || self.id == PokemonName::ZACIAN
            }
            Items::RUSTEDSHIELD => {
                self.id == PokemonName::ZAMAZENTACROWNED || self.id == PokemonName::ZAMAZENTA
            }
            Items::SPLASHPLATE => self.id == PokemonName::ARCEUSWATER,
            Items::TOXICPLATE => self.id == PokemonName::ARCEUSPOISON,
            Items::EARTHPLATE => self.id == PokemonName::ARCEUSGROUND,
            Items::STONEPLATE => self.id == PokemonName::ARCEUSROCK,
            Items::INSECTPLATE => self.id == PokemonName::ARCEUSBUG,
            Items::SPOOKYPLATE => self.id == PokemonName::ARCEUSGHOST,
            Items::IRONPLATE => self.id == PokemonName::ARCEUSSTEEL,
            Items::FLAMEPLATE => self.id == PokemonName::ARCEUSFIRE,
            Items::MEADOWPLATE => self.id == PokemonName::ARCEUSGRASS,
            Items::ZAPPLATE => self.id == PokemonName::ARCEUSELECTRIC,
            Items::MINDPLATE => self.id == PokemonName::ARCEUSPSYCHIC,
            Items::ICICLEPLATE => self.id == PokemonName::ARCEUSICE,
            Items::DRACOPLATE => self.id == PokemonName::ARCEUSDRAGON,
            Items::DREADPLATE => self.id == PokemonName::ARCEUSDARK,
            Items::FISTPLATE => self.id == PokemonName::ARCEUSFIGHTING,
            Items::BLANKPLATE => self.id == PokemonName::ARCEUS,
            Items::SKYPLATE => self.id == PokemonName::ARCEUSFLYING,
            Items::PIXIEPLATE => self.id == PokemonName::ARCEUSFAIRY,
            Items::BUGMEMORY => self.id == PokemonName::SILVALLYBUG,
            Items::FIGHTINGMEMORY => self.id == PokemonName::SILVALLYFIGHTING,
            Items::GHOSTMEMORY => self.id == PokemonName::SILVALLYGHOST,
            Items::PSYCHICMEMORY => self.id == PokemonName::SILVALLYPSYCHIC,
            Items::FLYINGMEMORY => self.id == PokemonName::SILVALLYFLYING,
            Items::STEELMEMORY => self.id == PokemonName::SILVALLYSTEEL,
            Items::ICEMEMORY => self.id == PokemonName::SILVALLYICE,
            Items::POISONMEMORY => self.id == PokemonName::SILVALLYPOISON,
            Items::FIREMEMORY => self.id == PokemonName::SILVALLYFIRE,
            Items::DRAGONMEMORY => self.id == PokemonName::SILVALLYDRAGON,
            Items::GROUNDMEMORY => self.id == PokemonName::SILVALLYGROUND,
            Items::WATERMEMORY => self.id == PokemonName::SILVALLYWATER,
            Items::DARKMEMORY => self.id == PokemonName::SILVALLYDARK,
            Items::ROCKMEMORY => self.id == PokemonName::SILVALLYROCK,
            Items::GRASSMEMORY => self.id == PokemonName::SILVALLYGRASS,
            Items::FAIRYMEMORY => self.id == PokemonName::SILVALLYFAIRY,
            Items::ELECTRICMEMORY => self.id == PokemonName::SILVALLYELECTRIC,
            Items::CORNERSTONEMASK => {
                self.id == PokemonName::OGERPONCORNERSTONE
                    || self.id == PokemonName::OGERPONCORNERSTONETERA
            }
            Items::HEARTHFLAMEMASK => {
                self.id == PokemonName::OGERPONHEARTHFLAME
                    || self.id == PokemonName::OGERPONHEARTHFLAMETERA
            }
            Items::WELLSPRINGMASK => {
                self.id == PokemonName::OGERPONWELLSPRING
                    || self.id == PokemonName::OGERPONWELLSPRINGTERA
            }
            _ => false,
        }
    }

    pub fn item_can_be_removed(&self) -> bool {
        if self.ability == Abilities::STICKYHOLD {
            return false;
        }
        !self.item_is_permanent()
    }

    pub fn is_grounded(&self) -> bool {
        if self.item == Items::IRONBALL {
            return true;
        }
        if self.has_type(&PokemonType::FLYING)
            || self.ability == Abilities::LEVITATE
            || self.item == Items::AIRBALLOON
        {
            return false;
        }
        true
    }

    pub fn volatile_status_can_be_applied(
        &self,
        volatile_status: &PokemonVolatileStatus,
        active_volatiles: &HashSet<PokemonVolatileStatus>,
        ally_pkmn: &Pokemon,
        first_move: bool,
    ) -> bool {
        if active_volatiles.contains(volatile_status) || self.hp == 0 {
            return false;
        }
        match volatile_status {
            PokemonVolatileStatus::LEECHSEED => {
                if self.has_type(&PokemonType::GRASS)
                    || active_volatiles.contains(&PokemonVolatileStatus::SUBSTITUTE)
                {
                    return false;
                }
                true
            }
            PokemonVolatileStatus::CONFUSION => {
                if active_volatiles.contains(&PokemonVolatileStatus::SUBSTITUTE) {
                    return false;
                }
                true
            }
            PokemonVolatileStatus::SUBSTITUTE => self.hp > self.maxhp / 4,
            PokemonVolatileStatus::FLINCH => {
                if !first_move || [Abilities::INNERFOCUS].contains(&self.ability) {
                    return false;
                }
                true
            }
            PokemonVolatileStatus::PROTECT => first_move,
            PokemonVolatileStatus::TAUNT
            | PokemonVolatileStatus::TORMENT
            | PokemonVolatileStatus::ENCORE
            | PokemonVolatileStatus::DISABLE
            | PokemonVolatileStatus::HEALBLOCK
            | PokemonVolatileStatus::ATTRACT => {
                self.ability != Abilities::AROMAVEIL && ally_pkmn.ability != Abilities::AROMAVEIL
            }
            _ => true,
        }
    }

    pub fn immune_to_stats_lowered_by_opponent(
        &self,
        stat: &PokemonBoostableStat,
        volatiles: &HashSet<PokemonVolatileStatus>,
    ) -> bool {
        if [
            Abilities::CLEARBODY,
            Abilities::WHITESMOKE,
            Abilities::FULLMETALBODY,
        ]
        .contains(&self.ability)
            || ([Items::CLEARAMULET].contains(&self.item))
        {
            return true;
        }

        if volatiles.contains(&PokemonVolatileStatus::SUBSTITUTE) {
            return true;
        }

        if stat == &PokemonBoostableStat::Attack && self.ability == Abilities::HYPERCUTTER {
            return true;
        } else if stat == &PokemonBoostableStat::Accuracy && self.ability == Abilities::KEENEYE {
            return true;
        }

        false
    }
}

impl Side {
    pub fn active_is_charging_move(&self, slot_ref: SlotReference) -> Option<PokemonMoveIndex> {
        for volatile in self.get_slot_immutable(&slot_ref).volatile_statuses.iter() {
            if let Some(choice) = charge_volatile_to_choice(volatile) {
                let mut iter = self.get_active_immutable(&slot_ref).moves.into_iter();
                while let Some(mv) = iter.next() {
                    if mv.id == choice {
                        return Some(iter.pokemon_move_index);
                    }
                }
            }
        }
        None
    }

    pub fn calculate_highest_stat(&self, slot_ref: &SlotReference) -> PokemonBoostableStat {
        let mut highest_stat = PokemonBoostableStat::Attack;
        let mut highest_stat_value =
            self.calculate_boosted_stat(slot_ref, PokemonBoostableStat::Attack);
        for stat in [
            PokemonBoostableStat::Defense,
            PokemonBoostableStat::SpecialAttack,
            PokemonBoostableStat::SpecialDefense,
            PokemonBoostableStat::Speed,
        ] {
            let stat_value = self.calculate_boosted_stat(slot_ref, stat);
            if stat_value > highest_stat_value {
                highest_stat = stat;
                highest_stat_value = stat_value;
            }
        }
        highest_stat
    }
    pub fn get_boost_from_boost_enum(
        &self,
        slot_reference: &SlotReference,
        boost_enum: &PokemonBoostableStat,
    ) -> i8 {
        match boost_enum {
            PokemonBoostableStat::Attack => self.get_slot_immutable(slot_reference).attack_boost,
            PokemonBoostableStat::Defense => self.get_slot_immutable(slot_reference).defense_boost,
            PokemonBoostableStat::SpecialAttack => {
                self.get_slot_immutable(slot_reference).special_attack_boost
            }
            PokemonBoostableStat::SpecialDefense => {
                self.get_slot_immutable(slot_reference)
                    .special_defense_boost
            }
            PokemonBoostableStat::Speed => self.get_slot_immutable(slot_reference).speed_boost,
            PokemonBoostableStat::Evasion => self.get_slot_immutable(slot_reference).evasion_boost,
            PokemonBoostableStat::Accuracy => {
                self.get_slot_immutable(slot_reference).accuracy_boost
            }
        }
    }

    pub fn calculate_boosted_stat(
        &self,
        slot_ref: &SlotReference,
        stat: PokemonBoostableStat,
    ) -> i16 {
        let active = self.get_active_immutable(slot_ref);
        match stat {
            PokemonBoostableStat::Attack => {
                let boost = self.get_slot_immutable(slot_ref).attack_boost;
                multiply_boost(boost, active.attack)
            }
            PokemonBoostableStat::Defense => {
                let boost = self.get_slot_immutable(slot_ref).defense_boost;
                multiply_boost(boost, active.defense)
            }
            PokemonBoostableStat::SpecialAttack => {
                let boost = self.get_slot_immutable(slot_ref).special_attack_boost;
                multiply_boost(boost, active.special_attack)
            }
            PokemonBoostableStat::SpecialDefense => {
                let boost = self.get_slot_immutable(slot_ref).special_defense_boost;
                multiply_boost(boost, active.special_defense)
            }
            PokemonBoostableStat::Speed => {
                let boost = self.get_slot_immutable(slot_ref).speed_boost;
                multiply_boost(boost, active.speed)
            }
            _ => {
                panic!("Not implemented")
            }
        }
    }

    pub fn has_alive_non_rested_sleeping_pkmn(&self) -> bool {
        for p in self.pokemon.into_iter() {
            if p.status == PokemonStatus::SLEEP && p.hp > 0 && p.rest_turns == 0 {
                return true;
            }
        }
        false
    }

    pub fn can_use_tera(&self) -> bool {
        for p in self.pokemon.into_iter() {
            if p.terastallized {
                return false;
            }
        }
        true
    }

    pub fn add_switches(&self, vec: &mut Vec<MoveChoice>) {
        let mut iter = self.pokemon.into_iter();
        while let Some(p) = iter.next() {
            if p.hp > 0
                && iter.pokemon_index != self.slot_a.active_index
                && iter.pokemon_index != self.slot_b.active_index
            {
                vec.push(MoveChoice::Switch(iter.pokemon_index));
            }
        }
        if vec.len() == 0 {
            vec.push(MoveChoice::None);
        }
    }

    pub fn trapped(
        &self,
        slot: &SideSlot,
        opponent_active_a: &Pokemon,
        opponent_active_b: &Pokemon,
    ) -> bool {
        let active_pkmn = &self.pokemon.pkmn[slot.active_index as usize];
        if slot
            .volatile_statuses
            .contains(&PokemonVolatileStatus::LOCKEDMOVE)
        {
            return true;
        }
        if active_pkmn.item == Items::SHEDSHELL || active_pkmn.has_type(&PokemonType::GHOST) {
            return false;
        } else if slot
            .volatile_statuses
            .contains(&PokemonVolatileStatus::PARTIALLYTRAPPED)
        {
            return true;
        } else if opponent_active_a.ability == Abilities::SHADOWTAG
            || opponent_active_b.ability == Abilities::SHADOWTAG
        {
            return true;
        } else if (opponent_active_a.ability == Abilities::ARENATRAP
            || opponent_active_b.ability == Abilities::ARENATRAP)
            && active_pkmn.is_grounded()
        {
            return true;
        } else if (opponent_active_a.ability == Abilities::MAGNETPULL
            || opponent_active_b.ability == Abilities::MAGNETPULL)
            && active_pkmn.has_type(&PokemonType::STEEL)
        {
            return true;
        }
        false
    }

    pub fn num_fainted_pkmn(&self) -> i8 {
        let mut count = 0;
        for p in self.pokemon.into_iter() {
            if p.hp == 0 && p.id != PokemonName::NONE {
                count += 1;
            }
        }
        count
    }
}

impl State {
    pub fn root_get_all_options(
        &self,
    ) -> (Vec<(MoveChoice, MoveChoice)>, Vec<(MoveChoice, MoveChoice)>) {
        if self.team_preview {
            let mut s1_options = Vec::new();
            let mut s2_options = Vec::new();

            // collect alive indices
            let mut s1_alive_indices = Vec::new();
            let mut pkmn_iter = self.sides[0].pokemon.into_iter();
            while let Some(_) = pkmn_iter.next() {
                if self.sides[0].pokemon[pkmn_iter.pokemon_index].hp > 0 {
                    s1_alive_indices.push(pkmn_iter.pokemon_index);
                }
            }
            let mut s2_alive_indices = Vec::new();
            let mut pkmn_iter = self.sides[1].pokemon.into_iter();
            while let Some(_) = pkmn_iter.next() {
                if self.sides[1].pokemon[pkmn_iter.pokemon_index].hp > 0 {
                    s2_alive_indices.push(pkmn_iter.pokemon_index);
                }
            }

            // Generate all valid pairs for initial switch
            for i in 0..s1_alive_indices.len() {
                for j in (i + 1)..s1_alive_indices.len() {
                    s1_options.push((
                        MoveChoice::Switch(s1_alive_indices[i]),
                        MoveChoice::Switch(s1_alive_indices[j]),
                    ));
                }
            }
            for i in 0..s2_alive_indices.len() {
                for j in (i + 1)..s2_alive_indices.len() {
                    s2_options.push((
                        MoveChoice::Switch(s2_alive_indices[i]),
                        MoveChoice::Switch(s2_alive_indices[j]),
                    ));
                }
            }
            return (s1_options, s2_options);
        }

        let mut move_options = MoveOptions::new();
        self.get_all_options(&mut move_options);
        if self.sides[0].slot_a.force_trapped || self.sides[0].slot_a.slow_uturn_move {
            move_options
                .side_one_combined_options
                .retain(|(x, _)| match x {
                    MoveChoice::Move(_, _, _) | MoveChoice::MoveTera(_, _, _) => true,
                    MoveChoice::Switch(_) => false,
                    MoveChoice::None => true,
                });
        }
        if self.sides[0].slot_b.force_trapped || self.sides[0].slot_b.slow_uturn_move {
            move_options
                .side_one_combined_options
                .retain(|(_, x)| match x {
                    MoveChoice::Move(_, _, _) | MoveChoice::MoveTera(_, _, _) => true,
                    MoveChoice::Switch(_) => false,
                    MoveChoice::None => true,
                });
        }

        if self.sides[1].slot_a.force_trapped || self.sides[1].slot_a.slow_uturn_move {
            move_options
                .side_two_combined_options
                .retain(|(x, _)| match x {
                    MoveChoice::Move(_, _, _) | MoveChoice::MoveTera(_, _, _) => true,
                    MoveChoice::Switch(_) => false,
                    MoveChoice::None => true,
                });
        }
        if self.sides[1].slot_b.force_trapped || self.sides[1].slot_b.slow_uturn_move {
            move_options
                .side_two_combined_options
                .retain(|(_, x)| match x {
                    MoveChoice::Move(_, _, _) | MoveChoice::MoveTera(_, _, _) => true,
                    MoveChoice::Switch(_) => false,
                    MoveChoice::None => true,
                });
        }
        if move_options.side_one_combined_options.len() == 0 {
            move_options
                .side_one_combined_options
                .push((MoveChoice::None, MoveChoice::None));
        }
        if move_options.side_two_combined_options.len() == 0 {
            move_options
                .side_two_combined_options
                .push((MoveChoice::None, MoveChoice::None));
        }
        (
            move_options.side_one_combined_options,
            move_options.side_two_combined_options,
        )
    }

    fn handle_force_switch_side(
        &self,
        slot_a_options: &mut Vec<MoveChoice>,
        slot_b_options: &mut Vec<MoveChoice>,
        slot_a_force_switch: bool,
        slot_b_force_switch: bool,
        side: &Side,
    ) {
        if slot_a_force_switch {
            side.add_switches(slot_a_options);
        } else if side.slot_a.switch_out_move_second_saved_move != MoveChoice::None {
            slot_a_options.push(side.slot_a.switch_out_move_second_saved_move);
        } else {
            slot_a_options.push(MoveChoice::None);
        }

        if slot_b_force_switch {
            side.add_switches(slot_b_options);
        } else if side.slot_b.switch_out_move_second_saved_move != MoveChoice::None {
            slot_b_options.push(side.slot_b.switch_out_move_second_saved_move);
        } else {
            slot_b_options.push(MoveChoice::None);
        }
    }

    fn handle_opponent_during_force_switch(
        &self,
        slot_a_options: &mut Vec<MoveChoice>,
        slot_b_options: &mut Vec<MoveChoice>,
        side: &Side,
    ) {
        // Handle slot A
        if side.slot_a.switch_out_move_second_saved_move == MoveChoice::None {
            slot_a_options.push(MoveChoice::None);
        } else {
            slot_a_options.push(side.slot_a.switch_out_move_second_saved_move);
        }

        // Handle slot B
        if side.slot_b.switch_out_move_second_saved_move == MoveChoice::None {
            slot_b_options.push(MoveChoice::None);
        } else {
            slot_b_options.push(side.slot_b.switch_out_move_second_saved_move);
        }
    }

    fn handle_fainted_switches(
        &self,
        slot_a_options: &mut Vec<MoveChoice>,
        slot_b_options: &mut Vec<MoveChoice>,
        slot_a_fainted: bool,
        slot_b_fainted: bool,
        side: &Side,
    ) {
        if slot_a_fainted {
            side.add_switches(slot_a_options);
        } else {
            slot_a_options.push(MoveChoice::None);
        }

        if slot_b_fainted {
            side.add_switches(slot_b_options);
        } else {
            slot_b_options.push(MoveChoice::None);
        }
    }

    fn handle_slot_normal_options(
        &self,
        slot_options: &mut Vec<MoveChoice>,
        slot_ref: SlotReference,
        side: &Side,
        side_ref: SideReference,
        _partner_active: &Pokemon,
        opponent_active_a: &Pokemon,
        opponent_active_b: &Pokemon,
    ) {
        let slot = match slot_ref {
            SlotReference::SlotA => &side.slot_a,
            SlotReference::SlotB => &side.slot_b,
        };

        if slot
            .volatile_statuses
            .contains(&PokemonVolatileStatus::COMMANDING)
            || slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::MUSTRECHARGE)
        {
            slot_options.push(MoveChoice::None);
        } else {
            let active_charging_mv_index = side.active_is_charging_move(slot_ref);
            let encored = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::ENCORE);
            let disabled = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::DISABLE);
            let taunted = slot
                .volatile_statuses
                .contains(&PokemonVolatileStatus::TAUNT);

            let mut targets_opponent_slot_a = false;
            let mut targets_opponent_slot_b = false;

            let partner = side.get_active_immutable(&slot_ref.get_other_slot());
            if opponent_active_a.hp > 0 {
                targets_opponent_slot_a = true;
            }
            if opponent_active_b.hp > 0 {
                targets_opponent_slot_b = true;
            }

            side.get_active_immutable(&slot_ref).add_available_moves(
                side_ref,
                &slot_ref,
                slot_options,
                &slot.last_used_move,
                (targets_opponent_slot_a, targets_opponent_slot_b),
                active_charging_mv_index,
                partner,
                encored,
                disabled,
                taunted,
                side.can_use_tera(),
            );

            if !side.trapped(
                side.get_slot_immutable(&slot_ref),
                opponent_active_a,
                opponent_active_b,
            ) {
                side.add_switches(slot_options);
            }
        }
    }

    pub fn get_all_options(&self, move_options: &mut MoveOptions) {
        // Get active pokemon references
        let side_one_active_a = self.sides[0].get_active_immutable(&SlotReference::SlotA);
        let side_one_active_b = self.sides[0].get_active_immutable(&SlotReference::SlotB);
        let side_two_active_a = self.sides[1].get_active_immutable(&SlotReference::SlotA);
        let side_two_active_b = self.sides[1].get_active_immutable(&SlotReference::SlotB);

        // Check for external force switches
        let side_one_slot_a_force_switch = self.sides[0].slot_a.force_switch;
        let side_one_slot_b_force_switch = self.sides[0].slot_b.force_switch;
        let side_two_slot_a_force_switch = self.sides[1].slot_a.force_switch;
        let side_two_slot_b_force_switch = self.sides[1].slot_b.force_switch;

        // Handle external force switches first
        if side_one_slot_a_force_switch || side_one_slot_b_force_switch {
            self.handle_force_switch_side(
                &mut move_options.side_one_slot_a_options,
                &mut move_options.side_one_slot_b_options,
                side_one_slot_a_force_switch,
                side_one_slot_b_force_switch,
                &self.sides[0],
            );
            // Handle side two's saved moves or None
            self.handle_opponent_during_force_switch(
                &mut move_options.side_two_slot_a_options,
                &mut move_options.side_two_slot_b_options,
                &self.sides[1],
            );
            move_options.combine_slot_options();
            return;
        }

        if side_two_slot_a_force_switch || side_two_slot_b_force_switch {
            self.handle_force_switch_side(
                &mut move_options.side_two_slot_a_options,
                &mut move_options.side_two_slot_b_options,
                side_two_slot_a_force_switch,
                side_two_slot_b_force_switch,
                &self.sides[1],
            );
            // Handle side one's saved moves or None
            self.handle_opponent_during_force_switch(
                &mut move_options.side_one_slot_a_options,
                &mut move_options.side_one_slot_b_options,
                &self.sides[0],
            );
            move_options.combine_slot_options();
            return;
        }

        let side_one_has_alive_reserve = self.sides[0].has_alive_reserve_pkmn();
        let side_two_has_alive_reserve = self.sides[1].has_alive_reserve_pkmn();

        // Check for fainting force switches
        let side_one_slot_a_fainted = side_one_active_a.hp <= 0;
        let side_one_slot_b_fainted = side_one_active_b.hp <= 0;
        let side_two_slot_a_fainted = side_two_active_a.hp <= 0;
        let side_two_slot_b_fainted = side_two_active_b.hp <= 0;

        // Handle forced fainting switches where there is something
        // to switch to in the back
        if ((side_one_slot_a_fainted || side_one_slot_b_fainted) && side_one_has_alive_reserve)
            || ((side_two_slot_a_fainted || side_two_slot_b_fainted) && side_two_has_alive_reserve)
        {
            if side_one_slot_a_fainted || side_one_slot_b_fainted {
                self.handle_fainted_switches(
                    &mut move_options.side_one_slot_a_options,
                    &mut move_options.side_one_slot_b_options,
                    side_one_slot_a_fainted,
                    side_one_slot_b_fainted,
                    &self.sides[0],
                );
            } else {
                move_options.side_one_slot_a_options.push(MoveChoice::None);
                move_options.side_one_slot_b_options.push(MoveChoice::None);
            }

            if side_two_slot_a_fainted || side_two_slot_b_fainted {
                self.handle_fainted_switches(
                    &mut move_options.side_two_slot_a_options,
                    &mut move_options.side_two_slot_b_options,
                    side_two_slot_a_fainted,
                    side_two_slot_b_fainted,
                    &self.sides[1],
                );
            } else {
                move_options.side_two_slot_a_options.push(MoveChoice::None);
                move_options.side_two_slot_b_options.push(MoveChoice::None);
            }

            move_options.combine_slot_options();
            return;
        }

        // Handle normal turn options for side one
        if !side_one_slot_a_fainted {
            self.handle_slot_normal_options(
                &mut move_options.side_one_slot_a_options,
                SlotReference::SlotA,
                &self.sides[0],
                SideReference::SideOne,
                side_one_active_b,
                side_two_active_a,
                side_two_active_b,
            );
        }

        if !side_one_slot_b_fainted {
            self.handle_slot_normal_options(
                &mut move_options.side_one_slot_b_options,
                SlotReference::SlotB,
                &self.sides[0],
                SideReference::SideOne,
                side_one_active_a,
                side_two_active_a,
                side_two_active_b,
            );
        }

        // Handle normal turn options for side two
        if !side_two_slot_a_fainted {
            self.handle_slot_normal_options(
                &mut move_options.side_two_slot_a_options,
                SlotReference::SlotA,
                &self.sides[1],
                SideReference::SideTwo,
                side_two_active_b,
                side_one_active_a,
                side_one_active_b,
            );
        }

        if !side_two_slot_b_fainted {
            self.handle_slot_normal_options(
                &mut move_options.side_two_slot_b_options,
                SlotReference::SlotB,
                &self.sides[1],
                SideReference::SideTwo,
                side_two_active_a,
                side_one_active_a,
                side_one_active_b,
            );
        }

        // Ensure each slot has at least one option
        if move_options.side_one_slot_a_options.is_empty() {
            move_options.side_one_slot_a_options.push(MoveChoice::None);
        }
        if move_options.side_one_slot_b_options.is_empty() {
            move_options.side_one_slot_b_options.push(MoveChoice::None);
        }
        if move_options.side_two_slot_a_options.is_empty() {
            move_options.side_two_slot_a_options.push(MoveChoice::None);
        }
        if move_options.side_two_slot_b_options.is_empty() {
            move_options.side_two_slot_b_options.push(MoveChoice::None);
        }

        move_options.combine_slot_options();
    }

    pub fn reset_toxic_count(
        &mut self,
        side_ref: SideReference,
        vec_to_add_to: &mut Vec<Instruction>,
    ) {
        let side = self.get_side(side_ref);
        if side.side_conditions.toxic_count > 0 {
            vec_to_add_to.push(Instruction::ChangeSideCondition(
                ChangeSideConditionInstruction {
                    side_ref,
                    side_condition: PokemonSideCondition::ToxicCount,
                    amount: -1 * side.side_conditions.toxic_count,
                },
            ));
            side.side_conditions.toxic_count = 0;
        }
    }

    pub fn remove_volatile_statuses_on_switch(
        &mut self,
        side_ref: SideReference,
        slot_ref: &SlotReference,
        instructions: &mut Vec<Instruction>,
        baton_passing: bool,
        shed_tailing: bool,
    ) {
        let side = self.get_side(side_ref);

        // Take ownership of the current set to avoid borrow conflicts
        // since we may need to modify the side in the loop
        let mut volatile_statuses = std::mem::take(&mut side.get_slot(slot_ref).volatile_statuses);

        volatile_statuses.retain(|pkmn_volatile_status| {
            let should_retain = match pkmn_volatile_status {
                PokemonVolatileStatus::SUBSTITUTE => baton_passing || shed_tailing,
                PokemonVolatileStatus::LEECHSEED => baton_passing,
                PokemonVolatileStatus::TYPECHANGE => {
                    let active_index = side.get_slot(slot_ref).active_index;
                    let active = side.get_active(slot_ref);
                    if active.base_types != active.types {
                        instructions.push(Instruction::ChangeType(ChangeType {
                            side_ref,
                            pokemon_index: active_index,
                            new_types: active.base_types,
                            old_types: active.types,
                        }));
                        active.types = active.base_types;
                    }
                    false
                }
                // While you can't switch out of a locked move you can be forced out in other ways
                PokemonVolatileStatus::LOCKEDMOVE => {
                    let slot = side.get_slot(slot_ref);
                    instructions.push(Instruction::ChangeVolatileStatusDuration(
                        ChangeVolatileStatusDurationInstruction {
                            side_ref,
                            slot_ref: *slot_ref,
                            volatile_status: *pkmn_volatile_status,
                            amount: -1 * slot.volatile_status_durations.lockedmove,
                        },
                    ));
                    slot.volatile_status_durations.lockedmove = 0;
                    false
                }
                PokemonVolatileStatus::YAWN => {
                    let slot = side.get_slot(slot_ref);
                    instructions.push(Instruction::ChangeVolatileStatusDuration(
                        ChangeVolatileStatusDurationInstruction {
                            side_ref,
                            slot_ref: *slot_ref,
                            volatile_status: *pkmn_volatile_status,
                            amount: -1 * slot.volatile_status_durations.yawn,
                        },
                    ));
                    slot.volatile_status_durations.yawn = 0;
                    false
                }
                PokemonVolatileStatus::TAUNT => {
                    let slot = side.get_slot(slot_ref);
                    instructions.push(Instruction::ChangeVolatileStatusDuration(
                        ChangeVolatileStatusDurationInstruction {
                            side_ref,
                            slot_ref: *slot_ref,
                            volatile_status: *pkmn_volatile_status,
                            amount: -1 * slot.volatile_status_durations.taunt,
                        },
                    ));
                    slot.volatile_status_durations.taunt = 0;
                    false
                }
                _ => false,
            };

            if !should_retain {
                instructions.push(Instruction::RemoveVolatileStatus(
                    RemoveVolatileStatusInstruction {
                        side_ref,
                        slot_ref: *slot_ref,
                        volatile_status: *pkmn_volatile_status,
                    },
                ));
            }
            should_retain
        });

        // Clean up by re-setting the volatile statuses
        let slot = side.get_slot(slot_ref);
        slot.volatile_statuses = volatile_statuses;

        // reset volatile status durations to 0
        if slot.volatile_status_durations.confusion > 0 {
            instructions.push(Instruction::ChangeVolatileStatusDuration(
                ChangeVolatileStatusDurationInstruction {
                    side_ref,
                    slot_ref: *slot_ref,
                    volatile_status: PokemonVolatileStatus::CONFUSION,
                    amount: -1 * slot.volatile_status_durations.confusion,
                },
            ));
            slot.volatile_status_durations.confusion = 0;
        }
        if slot.volatile_status_durations.protect > 0 {
            instructions.push(Instruction::ChangeVolatileStatusDuration(
                ChangeVolatileStatusDurationInstruction {
                    side_ref,
                    slot_ref: *slot_ref,
                    volatile_status: PokemonVolatileStatus::PROTECT,
                    amount: -1 * slot.volatile_status_durations.protect,
                },
            ));
            slot.volatile_status_durations.protect = 0;
        }
        if slot.volatile_status_durations.taunt > 0 {
            instructions.push(Instruction::ChangeVolatileStatusDuration(
                ChangeVolatileStatusDurationInstruction {
                    side_ref,
                    slot_ref: *slot_ref,
                    volatile_status: PokemonVolatileStatus::TAUNT,
                    amount: -1 * slot.volatile_status_durations.taunt,
                },
            ));
            slot.volatile_status_durations.taunt = 0;
        }
        if slot.volatile_status_durations.encore > 0 {
            instructions.push(Instruction::ChangeVolatileStatusDuration(
                ChangeVolatileStatusDurationInstruction {
                    side_ref,
                    slot_ref: *slot_ref,
                    volatile_status: PokemonVolatileStatus::ENCORE,
                    amount: -1 * slot.volatile_status_durations.encore,
                },
            ));
            slot.volatile_status_durations.encore = 0;
        }
    }

    pub fn terrain_is_active(&self, terrain: &Terrain) -> bool {
        &self.terrain.terrain_type == terrain && self.terrain.turns_remaining > 0
    }

    pub fn get_terrain(&self) -> Terrain {
        if self.terrain.turns_remaining > 0 {
            self.terrain.terrain_type
        } else {
            Terrain::NONE
        }
    }

    pub fn get_weather(&self) -> Weather {
        if self.weather.turns_remaining == 0 {
            return Weather::NONE;
        }
        let s1_active_a = self.sides[0].get_active_immutable(&SlotReference::SlotA);
        let s1_active_b = self.sides[0].get_active_immutable(&SlotReference::SlotB);
        let s2_active_a = self.sides[1].get_active_immutable(&SlotReference::SlotA);
        let s2_active_b = self.sides[1].get_active_immutable(&SlotReference::SlotB);
        if s1_active_a.ability == Abilities::AIRLOCK
            || s1_active_a.ability == Abilities::CLOUDNINE
            || s2_active_a.ability == Abilities::AIRLOCK
            || s2_active_a.ability == Abilities::CLOUDNINE
            || s1_active_b.ability == Abilities::AIRLOCK
            || s1_active_b.ability == Abilities::CLOUDNINE
            || s2_active_b.ability == Abilities::AIRLOCK
            || s2_active_b.ability == Abilities::CLOUDNINE
        {
            Weather::NONE
        } else {
            self.weather.weather_type
        }
    }

    pub fn weather_is_active(&self, weather: &Weather) -> bool {
        let s1_active_a = self.sides[0].get_active_immutable(&SlotReference::SlotA);
        let s1_active_b = self.sides[0].get_active_immutable(&SlotReference::SlotB);
        let s2_active_a = self.sides[1].get_active_immutable(&SlotReference::SlotA);
        let s2_active_b = self.sides[1].get_active_immutable(&SlotReference::SlotB);
        &self.weather.weather_type == weather
            && s1_active_a.ability != Abilities::AIRLOCK
            && s1_active_a.ability != Abilities::CLOUDNINE
            && s2_active_a.ability != Abilities::AIRLOCK
            && s2_active_a.ability != Abilities::CLOUDNINE
            && s1_active_b.ability != Abilities::AIRLOCK
            && s1_active_b.ability != Abilities::CLOUDNINE
            && s2_active_b.ability != Abilities::AIRLOCK
            && s2_active_b.ability != Abilities::CLOUDNINE
    }

    fn _state_contains_any_move(&self, moves: &[Choices]) -> bool {
        for s in self.sides.iter() {
            for pkmn in s.pokemon.into_iter() {
                for mv in pkmn.moves.into_iter() {
                    if moves.contains(&mv.id) {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn set_damage_dealt_flag(&mut self) {
        if self._state_contains_any_move(&[
            Choices::COUNTER,
            Choices::MIRRORCOAT,
            Choices::METALBURST,
            Choices::COMEUPPANCE,
            Choices::FOCUSPUNCH,
        ]) {
            self.use_damage_dealt = true
        }
    }

    pub fn set_last_used_move_flag(&mut self) {
        if self._state_contains_any_move(&[
            Choices::ENCORE,
            Choices::FAKEOUT,
            Choices::FIRSTIMPRESSION,
            Choices::BLOODMOON,
            Choices::GIGATONHAMMER,
        ]) {
            self.use_last_used_move = true
        }
    }

    pub fn set_conditional_mechanics(&mut self) {
        /*
        These mechanics are not always relevant but when they are it
        is important that they are enabled. Enabling them all the time would
        suffer about a 20% performance hit.
        */
        self.set_damage_dealt_flag();
        self.set_last_used_move_flag();
    }
}
