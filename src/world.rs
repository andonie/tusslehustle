use crate::characters::Character;

use crate::combat::{Action, ActionStack};
use crate::player::PlayerInput;

/// Top-Level Game Structure, containing an arbitrary number of game contexts that are run in
/// a **turn-based simulation** based on all actors configuration, similarly to a
/// Physics Simulation.
///
/// The World class also makes the **main interface to interact with the game**, e.g. for players
/// re-equipping characters or giving items.
///
/// # Data Representation
/// A game world's overall state is managed within a **world directory**. That directory includes:
/// * `contexts/`: A directory containing the active contexts
/// * `players/`: A directory containing player data
struct World {

}


/// Describes time in the simulated world
struct WorldTime {

}

/// Describes a context within a simulated adventure world. World contexts can be things like
/// a turn-based battle, party rest activity, or traveling simulation.
///
/// Each world context:
///
/// * processes time based on the turn-based time interval
/// * processes user input (in between turns)
/// * has a **party**, i.e. player Characters it takes ownership over
///
/// World contexts all can **independently process turn progress** as the game simulation
/// progresses.
pub trait WorldContext {

    // ~~~~~~~~~~~~~~~~~~~ FUNDAMENTALS ~~~~~~~~~~~~~~~~~~~

    /// Processes a turn in this world context.
    fn process_turn(&mut self, logger: Option<&mut dyn TurnLogger>) -> Result<(),String>;

    /// Processes player input command (e.g. handing an item or exchanging characters / equipment)
    fn process_player_input(&mut self, input: &PlayerInput) -> Result<String,String>;


    // ~~~~~~~~~~~~~~~~~~~ CHARACTER ACCESS ~~~~~~~~~~~~~~~~~~~

    /// Returns the Characater if its' part of this context.
    fn get_character(&self, name: &str) -> Option<&Character> {
        let res = self.find_characters(&|c| c.name()==name);
        // If anything, the first match should be considered
        match res.first() {
            // Propagate None not found
            None => None,
            // Recast correctly and return character reference
            Some(c) => Some(*c),
        }
    }

    fn iter_characters(&self) -> core::slice::Iter<Character> ;

    fn iter_characters_mut(&mut self) -> core::slice::IterMut<Character> ;

    /// Allows to quickly filter for characters of any type, based on filter function
    fn find_characters(&self, filter: &dyn Fn(&Character) -> bool) -> Vec<&Character> {
        self.iter_characters().filter(|c|filter(c)).collect()
    }

    /// Similar to `find_characters`, but with *mutable* access
    fn find_characters_mut(&mut self, filter: &dyn Fn(&Character) -> bool) -> Vec<&mut Character> {
        self.iter_characters_mut().filter(|c|filter(c)).collect()
    }


    /// Returns a mutable reference to the character of `name` if it exists in this context
    fn get_character_mut(&mut self, name: &String) -> Option<&mut Character> {
        let mut res = self.find_characters_mut(&|c| c.name()==name);
        // If anything, the first match should be considered

        if res.len() == 0 {
            None
        } else {
            Some(res.remove(0))
        }
    }

    // ~~~~~~~~~~~~~~~~~~~ COMBAT ~~~~~~~~~~~~~~~~~~~
    // Functions called and used specifically during combat

    /// Called during action resolution.
    /// Every `action` that happens could offer reactions from (other) sources in the world.
    ///
    /// The returned vector represents all *reactions that have been made*. to the original `action`
    /// object.
    fn request_reactions(&mut self, action: &Action) -> Vec<Action>;
}


/// Turns can have a lot of interesting information that is often times ignored. E.g. when
/// calculating hundreds of turns rapidly, the individual maneuvers, reactions, and effects are
/// of each turn can be discarded after they are applied.
///
/// In order to 'preserve' the details from turn resolution, a `TurnLogger` can be provided that
/// gets access to these details before they are discarded.
///
/// This is how e.g. UI input from turn resolution can be caught and displayed.
pub trait TurnLogger {

    /// Called during combat after every action stack is built (and before it is resolved).
    /// Can be used to log the entire 'happening' of the one maneuver.
    fn maneuver_stack(&mut self, stack: &ActionStack);

}
