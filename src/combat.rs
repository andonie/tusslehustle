
use std::fmt::{Display, Formatter};
use crate::characters::{CharUnit, Character, Stats};
use crate::effects::Effect;
use crate::player::PlayerInput;
use crate::world::{TurnLogger, WorldContext};
use crate::mov::{Maneuver, Counter};
use crate::text::{InfoGrid, InfoLine, TextFormatting};

///
/// Simulates combat between Characters. Each character's **party** affiliation defines the Teams,
/// and Combat continues turn-wise until **only one party remains**.
pub struct Combat {
    /// All combat participants are owned by this context during its lifetime.
    /// Participants are of different **parties**, as defined by each character's `party` field.
    /// Combat continues until one party remains.
    participants: Vec<Character>,
}

impl Combat {

    pub fn from_participants(participants: Vec<Character>) -> Self {
        Combat { participants }
    }

    /// Builds a turn order, i.e. a Vector that orders all participants MOB stat
    fn build_turn_order(&self) -> Vec<String> {
        // Build a char list with Name - MOB
        let mut char_list: Vec<(&String, i64)>  = self.participants.iter()
            .map(|x| (x.name(), x.calculate_current_stats().mobility() ))
            .collect();

        // Sort list by Mobility, map to String list only.
        char_list.sort_by_key(|(n, m)| *m);
        char_list.iter().map(|(n, m)| n.clone().clone()).collect()
    }

}

/// Each combat is a world context, meaning it **independently processes turns**.
impl WorldContext for Combat {

    fn process_turn(&mut self, logger: Option<&dyn TurnLogger>) -> Result<(), String> {

        // Build Turn Order for this round

        let mut turn_order: Vec<String> = self.build_turn_order();

        // Before Maneuvers of the round are started, run `pre_turn`
        for char in &mut turn_order {
            let char = self.get_character_mut(char).unwrap();
            char.pre_turn();
        }

        // In turn order, process
        for char in &turn_order {
            // Build a new stack to process this maneuver
            let mut maneuver_stack = ActionStack::new();

            if let char = self.get_character(&char).unwrap() {
                let next_move = char.next_move();
                let actions = next_move.execute(char, self);

                // Initalizes the Move Stack using the action provided by the move
                maneuver_stack.build(actions, self);

            } else {
                // CouLdn't find character - shouldn't happen
                panic!("Shouldn't happen!");
            }

            if let Some(logger) = &logger {
                logger.maneuver_stack(&maneuver_stack);
            }
            println!("STACK:\n{}", maneuver_stack.format_line(0, TextFormatting::Console));

            // The move stack is now filled, describing the complete action
            // from Move to final response
            // Now, we resolve the stack.
            maneuver_stack.resolve(self);
        }

        // After Maneuvers of the round are finished, run `post_turn`
        for char in &mut turn_order {
            let char = self.get_character_mut(char).unwrap();
            char.post_turn();
        }
        
        
        // Finished turn
        Ok(())
    }

    fn process_player_input(&mut self, input: &PlayerInput) -> Result<String, String> {
        todo!()
    }

    fn iter_characters(&self) -> core::slice::Iter<Character> {
        self.participants.iter()
    }

    fn iter_characters_mut(&mut self) -> core::slice::IterMut<Character> {
        self.participants.iter_mut()
    }

    fn request_reactions(&mut self, action: &Action) -> Vec<Action> {
        // Return vector
        let mut reactions = Vec::new();
        let mut turn_order: Vec<String> = self.build_turn_order();
        // Reactions happen in reverse turn order (most agile char gets to decide last
        turn_order.reverse();


        // Mutable iteration (characters usually discount AP for reactions)
        // -> Fill up `reactions`, forwarding internal updates to each `Actor`s implementation

        for character in &turn_order {
            let character = self.get_character(character).unwrap();
            character.respond_to_action(self, action, &mut reactions);
        }

        reactions
    }
}


/// Describes an atomic action on the stack.
/// Key Data that defines an action:
/// 1. Actor(s) responsible for the action
/// 2. Action the 'protagonist's
/// 3. A Target for the action
pub struct Action {
    source: EntityPointer,
    effect: ActionEffect,
    target: EntityPointer,

    /// A possible reference to this action on the stack.
    /// Will be set when the action is added to the stack.
    stack_target: Option<EntityPointer>,



}

impl Action {

    /// Builds a new Action instance,
    pub fn from_source(source: EntityPointer, effect: ActionEffect, target: EntityPointer) -> Self {

        Action {
            source,
            effect,
            target,
            // The action starts out **without a position on the stack**.
            stack_target: None,
        }
    }

    pub fn get_source(&self) -> &EntityPointer {
        &self.source
    }

    pub fn get_effect(&self) -> &ActionEffect {
        &self.effect
    }

    pub fn set_effect(&mut self, effect: ActionEffect) {
        self.effect = effect;
    }

    pub fn get_target(&self) -> &EntityPointer {
        &self.target
    }

    /// Updates the `target` of this action
    pub fn set_target(&mut self, target: EntityPointer) {
        self.target = target;
    }

    /// Returns a `Target` of the `Action` type that points to this particular action
    /// on the stack. **Expects this action to be properly added to a stack**, which it should be.
    pub fn build_self_target(&self) -> EntityPointer {
        if let Some(target) = &self.stack_target {
            target.clone()
        } else {
            panic!("Bad Target Configuration")
        }
    }

    /// Convenience function checks action target and returns `true` when this action is targetting
    /// characters and the given `name` is a match.
    pub fn targets_character(&self, name: &String) -> bool {
        if let EntityPointer::Character(chars) = &self.target {
            chars.iter().any(|s| s == name)
        } else {
            false
        }
    }

    /// Sets / updates the location of this `Action` on the stack by defining it as a target on the
    /// stack.
    fn set_stack_location(&mut self, target: EntityPointer) {
        self.stack_target = Some(target);
    }

    /// Resolves this action on the provided world `context`. Called from the Action Stack during
    /// resolution after Action-Targeting effects have been resolved separately,
    fn resolve_on_chars(&self, context: &mut dyn WorldContext) -> Result<(), String> {
        match &self.target {
            EntityPointer::Character(c) => {
                for character in context.find_characters_mut(&|char: &Character| c.contains(char.name())) {
                    self.effect.apply_to_character(character)
                }
            }
            // Action's are expected
            EntityPointer::Action(i) => (),
            EntityPointer::Effect(_, _) => {}
            EntityPointer::Environment => {}
        }
        Ok(())
    }

    fn resolve_on_action(&self, action: &mut Action) -> Result<(), String> {
        self.effect.apply_to_action(action);
        Ok(())
    }
}

impl InfoLine for Action {
    fn format_line(&self, len: usize, formatting: TextFormatting) -> String {
        // First: Source Actor
        let source = self.source.format_line(0, formatting);
        let target = self.target.format_line(0, formatting);


        format!("{} {} {} {} {}", source, self.effect.verb(), target, self.effect.preposition(), self.effect.format_value(len, formatting))
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {} on {}", self.source, self.effect.short_name(), self.target)
    }
}

/// This struct is used to trail and document the resolution of an action. As multiple effects
/// can happen in parallel, for example:
///
/// 1. A character starts a move, dealing physical slashing damage to an opponent
/// 2. The opponent is under the effect of a special effect that returns half of each physical
/// damage as magical lightning damage.
/// 3. The character has a one-time magical shield, which deals back magical twice radiant damage
/// 4. Under a special effect, the opponent has been knocked out for 4 turns.
pub struct ActionStack {

    /// The stack with a running count of actions for resolution. As other Characters react to
    /// "what's happening", new actions are added on top of this stack. In turn actions are
    /// resolved in stack order, i.e. last in first out, giving the last reaction to what's
    /// happning the first right of resolution.
    stack: Vec<Action>,
}

impl ActionStack {
    /// Constructor function builds
    fn new() -> Self {
        ActionStack {
            stack: Vec::new(),
        }
    }

    /// "Narrates" the stack briefly by summarizing the each effect:
    fn brief_narration(&self) -> String {
        let mut narration = String::new();
        for action in self.stack.iter() {
            narration = format!("{}\n{}", narration, action);
        }

        narration
    }

    /// Starts the stack resolution process and 'builds' the stack from an original
    /// basic number of `action`s created by the originating `Move`.
    /// Uses the world `context` to solicit reactions.
    fn build(&mut self, actions: Vec<Action>, context: &mut dyn WorldContext) {
        // Iteratively add all actions
        for action in actions {
            self.add_action(action, context);
        }
        //

    }

    fn get_action(&mut self, i: usize) -> &mut Action {
        self.stack.get_mut(i).unwrap()
    }

    fn add_action(&mut self, mut action: Action, context: &mut dyn WorldContext) {
        // Create a reference in this action for its' own stack location
        // (used for possible actions that directly target other actions rather than characters)
        action.set_stack_location(EntityPointer::Action(self.stack.len()));

        // Solicit reactions
        let reactions = context.request_reactions(&action);

        // Confirm the action to stack (consumes action)
        self.stack.push(action);

        // Every reaction is assumed to have resources (like AP) paid for, so they all should
        // be added to the stack
        for reaction in reactions {
            self.add_action(reaction, context);
        }


    }



    /// Resolves the stack in its current state on the given World Context,
    /// changing player / character states as the resolution happens.
    ///
    /// This process **consumes this instance** with all contained actions as they are enacted
    /// on the world `context` provided..
    fn resolve(&mut self, context: &mut dyn WorldContext) {

        // Track Actions that need to be resolved on other actions in this listing
        let mut targeting_actions: Vec<(usize, Action)> = Vec::new();
        // A counter to keep track of the 'stack index' used to compare targets
        let mut current_stack_index = self.stack.len() - 1;

        // Consume Actions from this stack in LIFO order
        while let Some(mut action) = self.stack.pop() {
            // ~~ Pre-Resolution Check: Action-Targeting Stack ~~
            // Before further resolving this action, check any effects in the action-targetting
            // listing and enact them on this effect before using it.
            for (_, a) in targeting_actions.iter().filter(|(i, _)| *i==current_stack_index) {
                a.resolve_on_action(&mut action).unwrap();
            }

            // ~~ Main Resolution ~~
            // Fundamentally, handle each action based on what it's targeting.
            match action.get_target() {
                // If this action targets a character, resolve it directly
                EntityPointer::Character(chars) => action.resolve_on_chars(context).expect("Issue resolving Action"),
                // If this action targets another action, add it to the 'side stack' that tracks
                // actions to still enact on other actions
                EntityPointer::Action(i) => targeting_actions.push((*i, action)),

                // As of now, effects and the environment cannot / should not be selected as
                // targets for `ActionEffects`
                EntityPointer::Effect(_, _) => {}
                EntityPointer::Environment => {}
            }

            // Exclude the final iteration step 0 as the format remains usize for easy targeting
            // in the vector
            if current_stack_index != 0 {
                // Decrement stack index as current actions are processed
                current_stack_index -= 1;
            }
        }
    }

}

impl InfoLine for ActionStack {
    fn format_line(&self, len: usize, formatting: TextFormatting) -> String {
        let mut narration = String::new();
        for action in self.stack.iter() {
            if narration.is_empty() {
                narration = action.format_line(0, formatting);
            } else {
                narration = format!("{}\n{}", narration, action.format_line(0, formatting));
            }
        }

        narration
    }
}

/// Implementation prints the full action stack onto multiple lines.
impl InfoGrid for ActionStack {
    fn display(&self, max_len: usize, num_lines: usize, formatting: TextFormatting) -> Vec<String> {
        todo!()
    }
}


/// This describes the atomic effects that are resolved on the Action Stack during turn resolution.
///
pub enum ActionEffect {
    // ~~~~~~~~~~~~~~~~~~~ Targeting Characters ~~~~~~~~~~~~~~~~~~~
    /// Applies Damage to target character(s)
    Attack(Damage),

    /// Basic Healing Effect returns up to wrapped value (but no more than it's maximum value) to
    /// the target character(s). This allows this effect to wrap HP/MP/AP/VIT etc.
    ///
    /// ## Negative Values
    ///
    /// Negative values are allowed in `CharUnits` and all values should be expected to be processed
    /// appropriately. Negative values **reduce the respective unit (not taking into account
    /// resistances, e.g. like `ULT` damage). This can be used for very specific effects like a
    /// 1:1 life drain.
    Heal(CharUnit),

    /// Gives a timed effect for a given amount of turns to target character(s)
    GiveTimedEffect(Box<dyn Effect>, i64),


    // ~~~~~~~~~~~~~~~~~~~ Targeting Actions on the Stack  ~~~~~~~~~~~~~~~~~~~
    /// Cancels an Action below on this stack
    Cancel,
    /// A canceled action (see `Cancel` action), doing nothing
    Canceled,

    /// Adjust damage of target `Attack` additively
    AdjustDamageAbs(i64),
    /// Adjust damage of target `Attack` multiplicatively
    AdjustDamageMul(f64),
    /// Changes the target of the target action on the stack
    ChangeTarget(EntityPointer),

    

}

impl ActionEffect {

    // ~~~~~~~~~~~~~~~~ String Formatting ~~~~~~~~~~~~~~~~

    /// Returns a 3-char name code for each type (disregarding possible enum params)
    pub fn short_name(&self) -> &str {
        match self {
            ActionEffect::Attack(_) => "ATK",
            ActionEffect::GiveTimedEffect(_, _) => "EFF",
            ActionEffect::Cancel => "CCL",
            ActionEffect::Canceled => "XXX",
            ActionEffect::AdjustDamageAbs(_) => "ADA",
            ActionEffect::AdjustDamageMul(_) => "ADM",
            ActionEffect::ChangeTarget(_) => "CHT",
            ActionEffect::Heal(_) => "HEA"
        }
    }

    /// Assigns a (third person singular) verb that well describes this effect. Used to narrate
    /// what's happening during Turn Resolution.
    pub fn verb(&self) -> &str {
        match self {
            ActionEffect::Attack(d) => {
                d.0.verb()
            },
            ActionEffect::GiveTimedEffect(e, _) => "affects",
            ActionEffect::Cancel => "cancels",
            ActionEffect::Canceled => "--",
            ActionEffect::AdjustDamageAbs(d) => if *d > 0 { "increases the damage of" } else { "decreases the damage of" },
            ActionEffect::AdjustDamageMul(f) => if *f > 1f64 { "increases the damage of" } else { "decreases the damage of" },
            ActionEffect::ChangeTarget(_) => "change the target of",
            ActionEffect::Heal(_) => "heals",
        }
    }

    /// Returns a preposition for the effect, e.g.
    ///
    /// * "...attacks **for** XYZ Damage"
    /// * "...increases the damage **by** XYZ's difference"
    /// *
    pub fn preposition(&self) -> &str {
        match self {
            ActionEffect::Attack(_) => "for",
            ActionEffect::GiveTimedEffect(_, _) => "with",
            ActionEffect::Cancel => "",
            ActionEffect::Canceled => "",
            ActionEffect::AdjustDamageAbs(_) => "by",
            ActionEffect::AdjustDamageMul(_) => "by",
            ActionEffect::ChangeTarget(_) => "to",
            ActionEffect::Heal(_) => "for",
        }
    }

    /// Formats the value of this effect using a provided `formatting`
    pub fn format_value(&self, len: usize, formatting: TextFormatting) -> String {
        match self {
            ActionEffect::Attack(d) => format!("{}", d),
            ActionEffect::GiveTimedEffect(e, t) => {
                let effect = formatting.enrich_text(e.describe(), "effect", None);
                format!("{} for {} turns", effect, t)
            }
            ActionEffect::Cancel => "".to_string(),
            ActionEffect::Canceled => "".to_string(),
            ActionEffect::AdjustDamageAbs(a) => a.format_line(len, formatting),
            ActionEffect::AdjustDamageMul(m) => {
                // Calc percentages
                let percentage_points = ((1f64-m) * 100f64).floor() as i64;
                format!("{}{}%", if *m < 0f64 {"-"} else {""}, percentage_points)
            }
            ActionEffect::ChangeTarget(t) => t.format_line(len, formatting),
            ActionEffect::Heal(unit) => unit.format_line(len, formatting),
        }
    }

    // ~~~~~~~~~~~~~~~~ FUNCTIONALITY ~~~~~~~~~~~~~~~~

    /// Applies this action to a given `character`. For effects targeting actions, nothing happens.
    fn apply_to_character(&self, character: &mut Character) {
        match self {
            ActionEffect::Attack(d) => character.apply_damage(d),
            ActionEffect::GiveTimedEffect(e, t) => (),
            ActionEffect::Cancel => {}
            ActionEffect::Canceled => {}
            ActionEffect::AdjustDamageAbs(_) => {}
            ActionEffect::AdjustDamageMul(_) => {}
            ActionEffect::ChangeTarget(_) => {}
            ActionEffect::Heal(v) => character.apply_directly(v),
        }

    }

    fn apply_to_action(&self, action: &mut Action) {
        match self {
            ActionEffect::Attack(_) => {}
            ActionEffect::GiveTimedEffect(_, _) => {}
            ActionEffect::Cancel => action.set_effect(ActionEffect::Canceled),
            ActionEffect::Canceled => {}
            ActionEffect::AdjustDamageAbs(d) => {
                if let ActionEffect::Attack(Damage(dt, da)) = action.get_effect() {
                    action.set_effect(ActionEffect::Attack(Damage(*dt, da+d)))
                }
            }
            ActionEffect::AdjustDamageMul(f) => {
                if let ActionEffect::Attack(Damage(dt, da)) = action.get_effect() {
                    action.set_effect(ActionEffect::Attack(Damage(*dt, (*da as f64*f).floor() as i64)))
                }
            }
            ActionEffect::ChangeTarget(t) => {
                // To maintain both this target ownership and the (required) ownership
                // for the targeted action's newly set target, we must clone this value's Target
                action.set_target(t.clone());
            }
            ActionEffect::Heal(_) => {}
        }
    }
}



/// Efficient wrapper to describe all source / target scenarios on the action stack.
///
/// Contains variants with **symbolic in-game pointers** for their respective covered game entity.
///
// Can be cloned to enable quick propagation of targets
#[derive(Clone)]
pub enum EntityPointer {
    /// Specifies one or many character targets from the encounter as target(s)
    Character(Vec<String>),
    /// Specifies a single action on the action stack
    Action(usize),
    /// Specifies an effect by two key characteristics:
    /// 1. Effect Source, formatted as an `EntityPointer`. While all types are possible, only
    /// pointers for entities that (can) hold effects (like `Character` or `Environment`).
    /// This value must be wrapped in a `Box` to allow recursion.
    /// 2. Effect Name
    Effect(Box<EntityPointer>, String),
    /// Describes the general environment (e.g. as a source of e.g. Heat) as a unique entity
    Environment
}


impl EntityPointer {

    /// Is an action target
    fn is_action(&self) -> bool {
        match self {
            EntityPointer::Character(_) => false,
            _ => true,
        }
    }

    /// Returns the number of individual targets are contained in this target. Single-target
    /// returns `1` and multi-attacks return larger numbers.
    fn num_entities(&self) -> usize {
        match self {
            EntityPointer::Character(targets) => targets.len(),
            EntityPointer::Action(_) => 1,
            EntityPointer::Effect(_, _) => 1,
            EntityPointer::Environment => 1
        }
    }

    /// If possible, returns a reference Main Character that's targeted from `context`
    fn get_character<'a>(&self, context: &'a dyn WorldContext) -> Option<&'a Character> {
        match self {
            EntityPointer::Character(name) => {
                if let name = name.first().unwrap() {
                    context.get_character(name)
                } else {
                    None
                }
            }
            // Action Targets do not have Character objectives
            _ => None,
        }
    }
}

impl InfoLine for EntityPointer {
    fn format_line(&self, len: usize, formatting: TextFormatting) -> String {
        match self {
            EntityPointer::Character(c) => {
                if c.len() == 1 {
                    c.first().unwrap().to_string()
                } else {
                    let res = c.iter().fold(String::new(), |mut acc, c|
                        if acc.is_empty() { c.to_string() } else {acc + ", " + c});
                    format!("the group of {}", res)
                }
            },
            EntityPointer::Action(a) => "a previous action".to_string(),
            EntityPointer::Effect(_, name) => format!("an effect ({})", name),
            EntityPointer::Environment => "the environment".to_string(),
        }
    }
}

impl Display for EntityPointer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityPointer::Character(c) => write!(f, "{}", c.iter().fold(String::new(), |a, b| a + " " + b)),
            EntityPointer::Action(i) => write!(f, "prev: {}", i),
            EntityPointer::Effect(source, name) => write!(f, "effect: {}", name),
            EntityPointer::Environment => write!(f, "the environment"),
        }
    }
}

pub trait Actor {

    /// Called every turn before this actor is called, only allowing (mutable) access to the
    /// instance itself. This can be used to set up the instance for turn resolution
    fn pre_turn(&mut self);

    fn post_turn(&mut self);

    /// Called during turn resolution when this Actor is asked to select the move they want to
    /// make during this turn
    fn next_move(&self) -> &dyn Maneuver;

    fn apply_damage(&mut self, damage: &Damage);

    /// Applies the given `val` directly on this `Actor`, disregarding any resistances or effects.
    /// This is used e.g. to resolve healing effects.
    fn apply_directly(&mut self, val: &CharUnit);

    /// Adds a new effect to this character for a certain `effect_duration` in turns
    fn apply_timed_effect(&mut self, effect: Box<dyn Effect>, effect_duration: i64);

    /// Called during action resolution. Any actor can react to any action placed in the context,
    /// the implementation of when/how to react is up to each individual `Actor`.
    ///
    /// Allows an actor to respond to actions before they are resolved.
    ///
    /// # Parameters
    ///
    /// * `self`: Mutable reference to allow updating internal state (e.g. discounting AP)
    /// * `context`: The World Context. Used for any checks and confirmations
    /// * `action`: The action that is currently being added to the stack.
    /// * `reactions`: A mutable reference to all Reactions to the given `action`. This can be
    /// non-empty and include previous actions which should be left as-is. Reactions can be added
    /// via `reactions.push`. Each entity is expected to do the required checks and self updates
    /// (e.g. a `Character` discounting AP),
    /// as **all actions added to `reactions` are expected/guaranteed to be included on the stack,
    /// but could still be reacted to by other participants, as each reaction will invoke its own
    /// `respond_to_action` opportunity
    fn respond_to_action(&self, context: &dyn WorldContext, action: &Action, reactions: &mut Vec<Action>);


}

/// A very simple struct, Damage is described by it's type and amount
#[derive(Copy, Clone)]
pub struct Damage(pub DamageType, pub i64);

impl Damage {

    pub fn dmg_type(&self) -> &DamageType {
        &self.0
    }

    pub fn amount(&self) -> i64 {
        self.1
    }
}

impl InfoLine for Damage {
    fn format_line(&self, len: usize, formatting: TextFormatting) -> String {
        let base = format!("{} {}", self.1, self.0);
        base
    }
}

impl Display for Damage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.1, self.0)
    }
}

/// Covers all damage types in the game (Physical & Magical). In addition, the type of damage can
/// be spelled out with a string defining the specific type (e.g. PHY("Slashing") or MAG("Fire"))
#[derive(Copy, Clone)]
pub enum DamageType {
    /// Physical damage with subtype
    PHY(&'static str),
    /// Magical damage with subtype
    MAG(&'static str),
    /// ZAP damage 'zaps' the character, **affecting MP instead of HP** when resolving
    ZAP(&'static str),
    /// Ultimate damage - cannot be defended / reduced - and therefore doesn't have subtypes
    ULT
}

impl DamageType {
    /// Returns the specific subtype of damage it this resistance protects from
    /// Can return "Any" when any resistance of the given Damage type (PHY or MAG) is affected
    fn get_subtype_name(&self) -> &'static str {
        match self {
            DamageType::PHY(t) => t,
            DamageType::MAG(t) => t,
            DamageType::ULT => "",
            DamageType::ZAP(t) => t,
        }
    }

    /// Returns the main damage type name
    fn get_damage_type_name(&self) -> &'static str {
        match self {
            DamageType::PHY(_) => "PHY",
            DamageType::MAG(_) => "MAG",
            DamageType::ULT => "ULT",
            DamageType::ZAP(_) => "ZAP",
        }
    }

    fn verb(&self) -> &str {
        match self {
            DamageType::PHY(t) => {
                match *t {
                    "Pierce" => "stabs",
                    "Slash" => "strikes",
                    "Blunt" => "pummels",
                    _ => "attacks"
                }
            }
            DamageType::MAG(t) => {
                match *t {
                    "Fire" => "burns",
                    "Ice" => "freezes",
                    _ => "casts a spell attack on"
                }
            },
            DamageType::ZAP(t) => {
                "zaps"
            }
            DamageType::ULT => "obliterates"
        }
    }
}

impl Display for DamageType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let DamageType::ULT = self {
            return write!(f, "ULTIMATE")
        }
        write!(f, "[{}] {}", self.get_damage_type_name(), self.get_subtype_name())
    }
}



#[cfg(test)]
mod tests {
    use crate::combat::{ Actor};
    use crate::equipment::{Equipment, EquipmentType};
    use crate::mov::Counter;
    use crate::text::{InfoGrid, TextFormatting};
    use super::*;

    /// Basic Testcharacter to use
    fn test_character(name: String) -> Character {
        Character::new(String::from(name), None, Stats {
            str: 3,
            dex: 8,
            grt: 6,
            wil: 2,
            int: 5,
            cha: 6,
        })
    }

    fn build_combat() -> Combat {
        let mut party = vec![test_character("Lindtbert".to_string())];
        let mut baddies = vec![test_character("Baddie".to_string())];
        for char in party.iter_mut() {
            char.set_party("Best Friends".to_string());
        }
        for char in baddies.iter_mut() {
            char.set_party("Baddies!".to_string());
        }
        // conjoin both groups into one encounter list
        party.extend(baddies);
        let mut combat = Combat {
            participants: party,
        };

        combat
    }

    #[test]
    fn test_single_stack() {
        let mut combat = build_combat();



    }

    #[test]
    fn test_turn() {

        let mut combat = build_combat();

        print!("PRE\n\n\n------------------------\n\n\n");
        for char in combat.iter_characters() {
            println!("{}", char.display(20, 3, TextFormatting::Console).join("\n"));
        }

        let charname = "Lindtbert".to_string();
        let hp_pre = combat.get_character(&charname).unwrap().hp();

        combat.process_turn(None).unwrap();

        for char in combat.iter_characters() {
            println!("{}", char.display(20, 3, TextFormatting::Console).join("\n"));

        }


        assert!(hp_pre > combat.get_character(&charname).unwrap().hp());
    }

    #[test]
    fn test_reaction() {
        let mut combat = build_combat();

        {
            // This time, equip Lindtbert with a ring to give him a special counter ability
            let mut lindtbert = combat.get_character_mut(&"Lindtbert".to_string()).unwrap();

            let mut eq = Equipment::new("Counter Ring".to_string(), EquipmentType::Ring, Stats {
                dex: 5,
                str: 0,
                grt: 0,
                wil: 0,
                cha: 0,
                int: 0,
            });
            eq.add_reaction(Box::new(
                Counter::new(DamageType::PHY(""), 0f64, 1f64)));

            lindtbert.equip(eq).unwrap();

            println!("{}", lindtbert.display(20, 4, TextFormatting::Console).join("\n"));
        }

        for _ in 0..8 {

            combat.process_turn(None).unwrap();

            {
                let num_lines = 4;
                let max_length = 20 * 2 + 2;
                let len_char = max_length / 2;
                let lindtbert = combat.get_character(&"Lindtbert".to_string()).unwrap();
                let baddie = combat.get_character(&"Baddie".to_string()).unwrap();
                // Map each Character to their individual line-by-line output
                let chars: Vec<(&Character, Vec<String>)> = vec![lindtbert, baddie].iter().map(|c|
                    (*c, c.display(len_char, num_lines, TextFormatting::Console))).collect();

                for i in 0..num_lines {
                    let mut line = String::with_capacity(len_char);
                    for (c, lines) in &chars {
                        line.push_str(&lines.get(i).unwrap());
                        line.push(' ');
                    }
                    println!("{}", line);
                }


            }
        }



    }
}
