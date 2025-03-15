use std::cell::{Ref, RefCell};
use std::cmp::max;
use std::fmt::Display;
use std::rc::Rc;
use crate::effects::Effect;
use crate::combat::{DamageType, Actor, Damage, Action, EntityPointer};
use crate::world::WorldContext;
use crate::mov::{BarehandedBlow, Maneuver, Reaction};
use crate::text::{BarStyle, InfoGrid, TextFormatting, text_util, InfoLine, MakesWords};
use crate::equipment::{Equipment, };

/// Fundamental stats that any game entity can provide.
/// These stats are 'dynamic' during gameplay and can change.
/// From these basic stats, a broader set of Character data can be generated, and is fully described
/// in the `CharacterStats` object.
pub struct Stats {
    /// Dexterity
    pub dex: i64,
    /// Strength
    pub str: i64,
    /// Grit
    pub grt: i64,
    /// Willpower
    pub wil: i64,
    /// Charisma
    pub cha: i64,
    /// Intelligence
    pub int: i64
}


/// Describes the complete game stats that inform how a character interacts with the world. They are
/// calculated from The base `Stats` and - during simulation - also from prevalent `Effect`s
struct GameStats {
    /// Max HP
    mhp: i64,
    /// Max MP
    mmp: i64,
    /// Maximum AP
    map: i64,
    /// Max Movement Distance per Turn
    mve: i64,
    /// Physical DEF
    pdf: i64,
    /// Magical DEF
    mdf: i64,
    /// Mobility
    mob: i64,
    /// Health REGEN
    hrg: i64,
    /// Magic REGEN
    mrg: i64,
    /// AP per Turn
    tap: i64,
}

impl Display for GameStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

/// Characters are the key actors in the game world and make 100% of the player controlled entities.
pub struct Character {
    /// Character name
    /// Character names are considered **unique for the entire world**, making Characters addressable
    /// This requirement is not entirely strict (e.g. an individual world context can have named
    /// characters that are not saved outside the context, unlike player characters)
    name: String,
    /// If this Character is owned and **controlled by a player**, their user name is stored here
    owner: Option<String>,
    /// Characters are friendly to those on their team and unfriendly towards those not on their
    /// team
    party: String,
    /// Basic Character Stats
    base_stats: Stats,
    /// Every Character can hold a certain amount of equipment, held in this list
    equipment: Vec<Equipment>,
    /// All passive effects from other sources affecting this character.
    /// Passive effect affect a character for a set amount of (remaining) turns,
    /// stored with each effect wihtin the tuples
    /// (does not include effects from equipment or other sources on the character, which affects
    /// the character exactly every turn they have it equipped)
    timed_effects: Vec<(Box<dyn Effect>, i64)>,
    /// Current Character Hit Points
    hp: i64,
    /// Current Character Mental Points
    /// This is implemented via Internal Mutability Pattern / `RefCell`, because **MP can be updated
    /// during action resolution** (for magic reactions)
    mp: RefCell<i64>,
    /// Current Action Points of this Character.
    /// Action Points are spent to **react to other actions**, allowing the Character more actions
    /// per turn effectively.
    /// This is implemented via Internal Mutability Pattern / `RefCell`, because **AP are updated
    /// during action resolution** (for reactions)
    ap: RefCell<i64>,
    /// Represents this Character's current body fitness.
    /// Depletes slowly and replenishes when resting
    vit: i64,
    /// A cached reference to the character's current `GameStats`. Since these are required often
    /// to calculate base game movement, they can be cached as a reference in each Character
    game_stats: Option<GameStats>,
}


/// Basic features of Stats
/// As stats define a rich amount of aspects, this block contains a good number of functions
/// covering the calculation of the *richer stats* of the game for each character.
impl Stats {
    /// Returns the entity's maximum HP
    /// This is based on:
    pub fn max_hp(&self) -> i64 {
        let stat_factor = (self.grt + self.wil) * 2 // HP Focus stats
            + self.str + self.cha; // Secondary HP stats

        100 + stat_factor * 12
    }

    /// Returns the entity's maximum MP.
    pub fn max_mp(&self) -> i64 {
        let stat_factor = (self.int + self.cha) * 2
            + self.wil + self.dex;

        stat_factor * 2
    }

    /// Returns the entity's maximum AP
    pub fn max_ap(&self) -> i64 {
        let stat_factor = (self.dex + self.int + self.wil) * 2
            + self.grt;

        (stat_factor as f64 * 0.2f64) as i64
    }

    /// Returns the entity's TAP (amount of AP per turn)
    pub fn action_points(&self) -> i64 {
        let stat_factor = (self.dex + self.int) * 2 // AP Focus Stats
            + self.grt + self.wil; // Secondary AP Stats

        // Running Counter of the entity's total AP
        let mut total_ap = 1;

        // Every X total stat_factor points, one additional AP is earned
        total_ap += (stat_factor as f64 * 0.05).floor() as i64;

        total_ap
    }

    /// Returns the entity's MVE (maximum movement distance for a single turn)
    pub fn move_speed(&self) -> i64 {
        let stat_factor: i64 = 0;
        stat_factor * 5
    }

    /// Calculates the PDF based on stats.
    pub fn phys_defense(&self) -> i64 {
        let stat_factor = self.grt + self.str;

        (stat_factor as f64 * 1.2).floor() as i64
    }

    /// Calculates the MDF based on stats.
    pub fn mag_defense(&self) -> i64 {
        let stat_factor = self.wil + self.cha;

        (stat_factor as f64 * 1.2).floor() as i64
    }

    /// Calculates the MOB of this character
    pub fn mobility(&self) -> i64 {
        let stat_factor = self.dex + self.int;

        (stat_factor as f64 * 1.2).floor() as i64
    }

    /// Calculates the HRG of this character
    pub fn health_regen(&self) -> i64 {
        let stat_factor = self.grt*2 // Main Health Regen Stat
            + self.wil + self.str; // Secondary Health Regen Stat

        // Running Counter of Regen, ensure there is at least 1
        let mut health_regen = 1;

        health_regen += (stat_factor as f64 * 0.60).floor() as i64;

        health_regen
    }

    /// Calculates the MRG of this character
    pub fn magic_regen(&self) -> i64 {
        let stat_factor = self.wil*2
            + self.cha + self.int;

        // Running counter of magic regen, starting with a minimum of 1
        let mut magic_regen = 1;

        magic_regen += (stat_factor as f64 * 0.33).floor() as i64;

        magic_regen
    }

    /// Convenience function runs all calculations that convert the base stats into all
    /// Character game stats.
    pub fn to_game_stats(&self) -> GameStats {
        GameStats {
            mhp: self.max_hp(),
            mmp: self.max_mp(),
            tap: self.action_points(),
            mve: self.move_speed(),
            pdf: self.phys_defense(),
            mdf: self.mag_defense(),
            mob: self.mobility(),
            hrg: self.health_regen(),
            mrg: self.magic_regen(),
            map: self.max_ap(),
        }
    }

    fn copy(&self) -> Stats {
        Stats {
            dex: self.dex,
            str: self.str,
            grt: self.grt,
            wil: self.wil,
            cha: self.cha,
            int: self.int,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~ Secondary Stats  ~~~~~~~~~~~~~~~~~~~
    // (not considered Game Stats but other useful units calculated during gameplay


    /// Calculates the Maximum VIT a Characeter with these stats starts out with after Resting
    pub fn max_vit(&self) -> i64 {
        let stat_factor = self.grt*4 + self.wil*3 + self.str + self.cha;

        stat_factor * 15
    }

    /// Formats this stat block as a requirement string.
    /// All **non-zero stats** of this object are considered requirements and are included in this
    /// string formatter
    pub fn format_as_req_string(&self) -> String {
        let mut requirements = String::new();

        fn append_req(base: &mut String, req: String ) {
            if !base.is_empty() {
                base.push_str(", ");
            }
            base.push_str(&req);
        }

        if self.dex > 0 {
            append_req(&mut requirements, format!("{} DEX", self.dex));
        }
        if self.str > 0 {
            append_req(&mut requirements, format!("{} STR", self.str));
        }
        if self.grt > 0 {
            append_req(&mut requirements, format!("{} GRT", self.grt));
        }
        if self.wil > 0 {
            append_req(&mut requirements, format!("{} WIL", self.wil));
        }
        if self.cha > 0 {
            append_req(&mut requirements, format!("{} CHA", self.cha));
        }
        if self.int > 0 {
            append_req(&mut requirements, format!("{} INT", self.int));
        }

        requirements
    }

    /// Compares all of this instance's stats against `req`'s and returns true if the requirements
    /// encoded in `req` are fully met, i.e. all stat numbers are higher or equal to `req`'s
    /// respective stats.
    pub fn meets_requirements(&self, req: &Stats) -> bool {
        self.dex >= req.dex || self.str >= req.str || self.grt >= req.grt || self.wil >= req.wil
            || self.cha >= req.cha || self.int >= req.int
    }
}

impl GameStats {
    pub fn mhp(&self) -> i64 {
        self.mhp
    }


    /// Max MP
    pub fn mmp(&self) -> i64 {
        self.mmp
    }
    /// AP per Turn
    pub fn tap(&self) -> i64 {
        self.tap
    }
    /// Max Movement Distance per Turn
    pub fn mve(&self) -> i64 {
        self.mve
    }
    /// Physical DEF
    pub fn pdf(&self) -> i64 {
        self.pdf
    }
    /// Magical DEF
    pub fn mdf(&self) -> i64 {
        self.mdf
    }
    /// Mobility
    pub fn mob(&self) -> i64 {
        self.mob
    }
    /// Health REGEN
    pub fn hrg(&self) -> i64 {
        self.hrg
    }
    /// Magic REGEN
    pub fn mrg(&self) -> i64 {
        self.mrg
    }
}


/// Describes the units every character manages during lifetime. Unlike their passive `Stats`,
/// these units all vary during gameplay (e.g. characters lose/heal HP, use up and regen MP and AP
/// etc.).
///
/// While these stats are not used directly on the `Character` struct (because they are never used
/// ambiguously), they are used to communicate effectively for things like variable healing effects.
///
///
pub enum CharUnit {
    /// Represents an amount of HP.
    HP(i64),
    /// Represents an amount of MP.
    MP(i64),
    /// Represents an amount of AP.
    AP(i64),
    /// Represents an amount of VIT.
    VIT(i64),
}

impl CharUnit {
    fn unit_name(&self) -> &'static str {
        match self {
            CharUnit::HP(_) => "HP",
            CharUnit::MP(_) => "MP",
            CharUnit::AP(_) => "AP",
            CharUnit::VIT(_) => "VIT"
        }
    }

    fn info_class(&self) -> &'static str {
        match self {
            CharUnit::HP(_) => "hp",
            CharUnit::MP(_) => "mp",
            CharUnit::AP(_) => "ap",
            CharUnit::VIT(_) => "vit"
        }
    }

    fn unit_value(&self) -> i64 {
        match self {
            CharUnit::HP(v) => *v,
            CharUnit::MP(v) => *v,
            CharUnit::AP(v) => *v,
            CharUnit::VIT(v) => *v,
        }
    }
}

impl MakesWords for CharUnit {
    fn format_words(&self, formatting: TextFormatting) -> Vec<(String, usize)> {
        let mut output = Vec::new();

        // Express amount
        output.extend(formatting.to_words(self.unit_value().format_line(5, formatting), "amount", None));

        // Express Unit
        output.extend(formatting.to_words(self.unit_name().to_string(), self.unit_name(), None));

        output
    }
}

/// Display implementation for `CharUnit` is used for expression in single-line strings, e.g.
///
/// * `15 HP`
/// * `-50 MP`
///
impl InfoLine for CharUnit {
    fn format_line(&self, len: usize, formatting: TextFormatting) -> String {
        let name = self.unit_name();
        formatting.enrich_text(
            format!("{} {}", self.unit_value().format_line(len-name.len()-1, formatting), name),
            self.info_class(),
            None
        )
    }
}


impl Character {

    // -------------- Constructor --------------

    /// Constructor function creates a basic character from the most critical paramters that define
    /// a character:
    ///
    /// * Their name
    /// * The (possible) owner
    /// * Their base stats that define basic capabilities
    pub fn new(name: String, owner: Option<String>, base_stats: Stats) -> Self {
        let mut character = Character {
            name: name,
            owner: owner,
            // By default, characters are part of no team
            party: "<no party>".to_string(),
            base_stats: base_stats,
            // By default, characters have no equipment
            equipment: vec![],
            // And no effects
            timed_effects: vec![],
            // These stats will be calculated right after building the base
            hp: 0,
            mp: RefCell::new(0),
            ap: RefCell::new(0),
            /// Vitality (secondary stat that declines gradually)
            vit: 0,
            // Empty cache at the beginning
            game_stats: None,
        };

        // Set the character's HP, MP, and secondary stats to max by default
        character.hp = character.base_stats.max_hp();
        *character.mp.get_mut() = character.base_stats.max_mp();
        *character.ap.get_mut() = character.base_stats.max_ap();
        character.vit = character.base_stats.max_vit();

        character
    }

    // -------------- Calculate Stats --------------

    /// Builds the current, actual base stats of this Character
    /// This takes into account base stats and also effects applied to this Character.
    ///
    /// Returns a newly created and calculated `Stats` object that describes the current
    pub fn calculate_current_stats(&self) -> Stats {
        // Copy the base stats
        let mut stats = self.base_stats.copy();

        // Forward all effects
        for effect in &self.all_current_effects() {
            effect.apply_to_stats(&mut stats);
        }

        stats
    }

    /// Using all current effects affecting this Character, calculates the basic game stats of this
    pub fn calculate_game_stats(&self) -> GameStats {
        // Build the final 'current' game stats
        let mut game_stats = self.calculate_current_stats().to_game_stats();

        for effect in &self.all_current_effects()
        {}

        game_stats
    }

    pub fn hp(&self) -> i64 {
        self.hp
    }

    pub fn mp(&self) -> i64 {
        *self.mp.borrow()
    }

    pub fn ap(&self) -> i64 {
        *self.ap.borrow()
    }

    /// Convenience function returns the percentage of HP this character has currently.
    pub fn hp_to_max_hp_ratio(&self) -> f64 {
        self.hp as f64 / self.calculate_current_stats().max_hp() as f64
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn party(&self) -> &String {
        &self.party
    }

    pub fn set_party(&mut self, party: String) {
        self.party = party;
    }

    /// Convenience function validates whether this character is part of the party named
    /// `party_name`
    pub fn party_check(&self, party_name: &String) -> bool {
        self.party == *party_name
    }

    pub fn as_target(&self) -> EntityPointer {
        EntityPointer::Character(vec![self.name.clone()])
    }

    // -------------- List all ... --------------

    /// Develops a complete list of all effects affecting this character, including:
    /// - Timed Effects (e.g. poisened, spell buffs/debuffs)
    fn all_current_effects(&self) -> Vec<&Box<dyn Effect>> {
        // Build a new vector to contain all effects to consider for this character at this time
        let mut effect_list = Vec::new();

        for (effect, _) in &self.timed_effects {
            effect_list.push(effect);
        }

        // In addition, add all permanent effects from Equipment
        for equipment in &self.equipment {
           for effect in equipment.get_passive_effects() {
               effect_list.push(effect);
           }
        }

        // Now that all effects are accounted for, sort this listing to ensure it's ordered in
        // resolution order (ascending by effect order number)
        // effect_list.sort_by_key(|e: &Box<&dyn Effect>| e.effect_order());

        effect_list
    }

    /// Computes all available moves for this Character at this time.
    fn all_current_moves(&self) -> Vec<&dyn Maneuver> {
        todo!()
    }

    /// Computes all available reactions for this Character at this time
    fn all_current_reactions(&self) -> Vec<&dyn Reaction> {
        let mut ret = Vec::new();
        for eq in self.equipment.iter() {
            eq.add_reactions(&mut ret)
        }

        ret
    }

    // -------------- Forward Iterators --------------

    /// Allows 'safe' mutable iteration of this character's equipment for checks.
    pub fn iter_equipment(&self) -> core::slice::Iter<Equipment> {
        self.equipment.iter()
    }

    // -------------- Modify --------------

    /// Equips an (equipment) item, unless something prevents it.
    pub fn equip(&mut self, equipment: Equipment) -> Result<(), String> {
        // Check 1: Max Equipment Number: For now, it's just hard set to 3
        if self.equipment.len() >= 3 {
            return Err("Cannot equip more than 3 items.".to_string())
        }

        // Check 2: Does this character meet the stat requirements?
        if ! self.calculate_current_stats().meets_requirements(equipment.get_stat_requirements()) {
            // collect
            return Err(format!("Not meeting the stat requirement."))
        }

        // Check 3: EquipmentType requirements
        if ! equipment.get_eq_type().can_equip(self) {
            return Err(format!("Cannot equip more {}", equipment.get_eq_type()))
        }

        // All Checks passed: Equipment should be added to character's equipment
        self.equipment.push(equipment);


        // Communicate that the equipment process was successful
        Ok(())
    }

    // -------------- (Text) Formatting helpers --------------

    /// Builds a string that represents this characters equipment best for the length provided
    fn build_equipment_description(&self, len: usize) -> String {
        if self.equipment.is_empty() {
            return "<no EQ>".to_string().format_line(len, TextFormatting::Plain);
        }
        // Split max len by number of EQ to display
        let eq_max_length = len / self.equipment.len();

        let mut ret = String::new();

        for equipment in self.equipment.iter() {
            ret.push_str(equipment.format_line(eq_max_length, TextFormatting::Plain).as_str());
        }

        ret
    }
}

impl Actor for Character {
    fn pre_turn(&mut self) {
        // REGEN: HP, MP, AP
        let stats = self.calculate_game_stats();
        self.hp = (self.hp + stats.hrg).min(stats.mhp);
        let mut mp_ptr = self.mp.get_mut();
        *mp_ptr = (*mp_ptr + stats.mrg).min(stats.mmp);
        let mut ap_ptr = self.ap.get_mut();
        *ap_ptr = (*ap_ptr + stats.tap).min(stats.map);
    }

    fn post_turn(&mut self) {
        // Decrease the turn count of all timed effects on this Character
        for (_, remaining_time) in &mut self.timed_effects {
            *remaining_time -= 1;
        }
        // Filter out all effects that timed out
        self.timed_effects.retain(|(_, remaining_time)| *remaining_time > 0);

        // TODO: Check if 'stayalive requirements' are met (HP>0)
    }

    fn next_move(&self) -> &dyn Maneuver {
        &BarehandedBlow
    }

    fn apply_damage(&mut self, damage: &Damage) {

        // We calculate the effective damage in this running counter
        let mut effective_damage = damage.amount();

        // Before confirming the effective damage, process it through all effects
        let fx = self.all_current_effects();


        for effect in fx {
            // TODO the error that broke me effect.on_damage_receive(self, damage_type, effective_damage);
        }

        // First, Apply All Defenses to this damage
        let gamestats = self.calculate_game_stats();

        // Apply basic PHY / MAG defense
        let defense_adjust = match damage.dmg_type() {
            DamageType::PHY(_) => gamestats.pdf,
            DamageType::MAG(_) => gamestats.mdf,
            DamageType::ZAP(_) => gamestats.mdf/2,
            // ULT damage cannot be defended
            DamageType::ULT => 0,
        };

        // Adjust damange by PHY / MAG defense (ensure too small damage don't go into negative)
        effective_damage = (effective_damage - defense_adjust).max(0);


        // By now, the effective damage represents the actual damage we receive.
        // --> Apply directly to HP
        match damage.dmg_type() {
            // Most damage affects HP
            DamageType::PHY(_) | DamageType::MAG(_) | DamageType::ULT => {
                self.hp -= effective_damage;
            }
            // ZAP Damage zaps MP instead of HP
            DamageType::ZAP(_) => {
                *self.mp.get_mut() -= effective_damage;
            }
        }

        // Since this Check state

    }

    fn apply_directly(&mut self, val: &CharUnit) {
        match val {
            CharUnit::HP(v) => {
                self.hp += *v;
            }
            CharUnit::MP(v) => {
                *self.mp.get_mut() = *v;
            }
            CharUnit::AP(v) => {
                *self.ap.get_mut() = *v;
            }
            CharUnit::VIT(v) => {
                self.vit += *v;
            }
        }
    }

    /// Adds a new effect to this character for a certain `effect_duration` in turns
    fn apply_timed_effect(&mut self, effect: Box<dyn Effect>, effect_duration: i64) {
        self.timed_effects.push((effect, effect_duration));

    }

    /// Called during combat action resolution. Called for every action played during combat,
    /// allowing characters to respond to actions as per their ability.
    ///
    /// As responding to an action incurs a unique cost (AP), this function also includes logic
    /// to ensure in-turn cost is paid, making use of the Interal Mutability Pattern through
    /// the Character's special `RefCell` parameters.
    fn respond_to_action(&self, context: &dyn WorldContext, action: &Action, reactions: &mut Vec<Action>) {
        if *self.ap.borrow() < 0 {
            // Once AP is below 0, character can no longer react
            return;
        }
        for reaction in self.all_current_reactions() {
            let mp_cost = reaction.mp_cost();
            if mp_cost > 0 && *self.mp.borrow() < mp_cost {
                // This reaction costs is MP we cannot afford. Cancel this reaction
                continue;
            }
            if let Some(react) = reaction.react(&self, action, context) {
                // Reaction has yielded an actual response. We want to progress with this!
                // 1. Pay the AP cost for this reaction
                // This may put the character below 0 which will stop them from reacting until
                // they recovered
                *self.ap.borrow_mut() -= reaction.ap_cost();

                // 3. Pay the MP cost (if applicable)
                if mp_cost > 0 {
                    *self.mp.borrow_mut() -= mp_cost;
                }

                // 2. Add Reactions that have been built
                reactions.extend(react)
            }
        }
    }
}

impl InfoGrid for Character {

    fn display(&self, max_len: usize, num_lines: usize, formatting: TextFormatting) -> Vec<String> {
        // ~~~~~~~~~~~~ INDIVIDUAL STAT PRINTS ~~~~~~~~~~~~
        // HP Bar
        let print_hp = |c: &Character, f| {
            text_util::render_bar_with_num("HP:", max_len, c.hp(), c.calculate_current_stats().max_hp(), BarStyle::DoubleLines, Some(('<', '>')), Some((&f, "hp", "Hitpoint Infos".to_string())))
        };

        // Name
        let print_charname = |c: &Character, f| c.name().format_line(max_len, formatting);
        // MP Bar
        let print_mp = |c: &Character, f| text_util::render_bar_with_num("MP:", max_len, c.mp(), c.calculate_current_stats().max_mp(), BarStyle::TwoChars('>', '-'), None, Some((&f, "mp", "MP Infos".to_string())));
        // AP Bar
        let print_ap = |c: &Character, f| text_util::render_bar_with_num("AP:", max_len, c.ap(), c.calculate_current_stats().max_ap(), BarStyle::TwoChars('!', '.'), None, Some((&f, "ap", "AP Infos".to_string())));
        // Short Gear overview
        let print_eq = |c: &Character, f| format!("EQ: {}",
            c.build_equipment_description(max_len-4)); // Discount 4 characters for "EQ: "

        // A progressive list of strategies to use when displaying the character line/by/line
        let strategies: Vec<(&dyn Fn(&Self, TextFormatting) -> String, &str)> = vec![
            (&print_charname, "name"),
            (&print_hp, "hp"),
            (&print_mp, "mp"),
            (&print_ap, "ap"),
            (&print_eq, "eq"),
        ];

        // Build Vector Lines
        let mut lines = Vec::new();
        for i in 0..num_lines {
            let (strat, info_class) = strategies.get(i).unwrap();

            // As the actual input for the line(s), commit the text formatting strategy together
            // with the associated info_class to format accordingly
            lines.push(strat(self, formatting));
        }
        lines
    }
}



#[cfg(test)]
mod tests {
    use crate::characters::Stats;
    use super::*;

    /// Basic Testcharacter to use
    fn test_character() -> Character {
        Character {
            name: String::from("Lindtbert"),
            owner: None,
            party: "Superparty".to_string(),
            base_stats: Stats {
                str: 3,
                dex: 4,
                grt: 6,
                wil: 2,
                int: 5,
                cha: 6,
            },
            equipment: vec![],
            timed_effects: vec![],
            hp: 120,
            mp: RefCell::new(50),
            ap: RefCell::new(15),
            vit: 200,
            game_stats: None,
        }
    }

    #[test]
    fn it_works() {
        let character = test_character();



    }
}
