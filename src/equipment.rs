//! Contains implementation of Equipment for use by Characters.
//! Equipment is one of the few active gameplay selections for Players, so this equipment can hold
//! a variety of functionality, mostly implemented and abstracted through `Effect`s, `Move`s, and
//! `Reaction`s.

use std::fmt::Display;
use std::rc::Rc;
use crate::characters::{Stats, Character};
use crate::effects::Effect;
use crate::mov::{Maneuver, Reaction};
use crate::text::{InfoLine, TextFormatting};

/// Describes different types of equipment. Each character is limited by equipment types, e.g.
/// one person cannot wear more than one Helmet.
#[derive(PartialEq, Debug)]
pub enum EquipmentType {
    Weapon,
    Head,
    Chest,
    Arms,
    Hands,
    Feet,
    Ring,
    Accessory,
}

impl EquipmentType {

    /// Returns a number that represents the maximum number of equipment a character can
    /// equip at once.
    pub fn equipment_max(&self) -> usize {
        match self {
            EquipmentType::Head => 1,
            EquipmentType::Chest => 1,
            EquipmentType::Arms => 2,
            EquipmentType::Hands => 2,
            EquipmentType::Feet => 2,
            EquipmentType::Ring => 4,
            EquipmentType::Weapon => 2,
            EquipmentType::Accessory => 2,
        }
    }

    /// Returns this types shortcode as a 4-char string
    pub fn shortcode(&self) -> &'static str {
        match self {
            EquipmentType::Head => "HEAD",
            EquipmentType::Chest => "CHST",
            EquipmentType::Arms => "ARMS",
            EquipmentType::Hands => "HNDS",
            EquipmentType::Feet => "FEET",
            EquipmentType::Ring => "RING",
            EquipmentType::Weapon => "WPON",
            EquipmentType::Accessory => "ACCS"
        }
    }

    /// Convenience function performs a equipment type check on a given `character`:
    ///
    /// * If the character could equip another item of this type, returns `true`
    /// * If the character is maxed out on equipment of this type, returns `false`
    ///
    /// Does not make additional checks (e.g. stat requirements)
    pub fn can_equip(&self, character: &Character) -> bool {
        let currently_equipped = character.iter_equipment().filter(|e| e.eq_type == *self).count();
        currently_equipped < self.equipment_max()
    }
}

impl Display for EquipmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[{}]", self.shortcode())
    }
}

/// Describes all equipments in game.
pub struct Equipment {
    /// Name of the equipment
    name: String,
    /// Type of equipment (Headgear, Weapon etc.)
    eq_type: EquipmentType,
    /// Equipment has minimum STAT requirements needed to use it. Most stats would usually be set
    /// to 0, but any amount of requirements up to 6 for all stats are OK.
    stat_requirements: Stats,
    /// Equipment can provide passive effects that are valid as long as the equipment is held.
    ///
    /// # Box Type
    ///
    /// Since all optional functionality of equipment is implemented via traits (`dyn` requirement),
    /// effects are stored in the Box type, since single ownership is expected.
    /// While the `Box` abstraction incurs some general performance penalty, that penalty should be
    /// minimal in reality, as the all objects (`Box`, the `Effect` it wraps, and the `Equipment` it
    /// is attached to) are generated right after another, minimizing heap fragmentation.
    passive_effects: Vec<Box<dyn Effect>>,
    /// Equipment can make additional moves available
    moves: Vec<Box<dyn Maneuver>>,
    /// Equipment can make additional reactions available
    reactions: Vec<Box<dyn Reaction>>,
}

impl Equipment {

    // ~~~~~~~~~~~~~~ Constructor and Setup ~~~~~~~~~~~~~~

    /// Builds a
    pub fn new(name: String, eq_type: EquipmentType, stat_requirements: Stats) -> Equipment {

        Equipment {
            name,
            eq_type,
            stat_requirements,
            passive_effects: vec![],
            moves: vec![],
            reactions: vec![],
        }
    }

    pub fn add_passive_effect(&mut self, effect: Box<dyn Effect>) {
        self.passive_effects.push(effect);
    }

    pub fn add_move(&mut self, mov: Box<dyn Maneuver>) {
        self.moves.push(mov);
    }

    pub fn add_reaction(&mut self, reaction: Box<dyn Reaction>) {
        self.reactions.push(reaction);
    }

    // ~~~~~~~~~~~~~~ Getters ~~~~~~~~~~~~~~

    pub fn get_eq_type(&self) -> &EquipmentType {
        &self.eq_type
    }

    pub fn get_stat_requirements(&self) -> &Stats {
        &self.stat_requirements
    }

    pub fn get_passive_effects(&self) -> &Vec<Box<dyn Effect>> {
        &self.passive_effects
    }

    // ~~~ Listy Getters ~~~

    pub fn add_reactions<'a>(&'a self, reactions: &mut Vec<&'a dyn Reaction>) {
        self.reactions.iter().for_each(|r| reactions.push(r.as_ref()));
    }
}

impl InfoLine for Equipment {
    fn format_line(&self, len: usize, formatting: TextFormatting) -> String {
        // Number of characters allocated for EQ type (type + parenthesis + space)
        let total_type = 4 + 2 + 1;
        let total_name = len - total_type;
        let mut name = String::from(&self.name);
        if self.name.len() < total_name {
            // Pad as needed
            name.push_str(&" ".repeat(total_name - self.name.len()));
        } else {
            name.truncate(total_name-2);
            name.push_str("..");
        }

        format!("{} {}", self.eq_type, name)
    }
}