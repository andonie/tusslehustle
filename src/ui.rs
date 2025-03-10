use std::cell::RefCell;
use crate::characters::{Character, Stats};
use crate::combat::{ActionStack, Combat, DamageType};
use crate::layouts::{LayoutDirection, LayoutSizing, LinearLayout};
use crate::text::{FrameType, InfoGrid, MakesWords, TextFormatting};
use crate::world::{TurnLogger, WorldContext};


/// Describes UI capabilities for text-based display. While similar to `InfoGrid` in that its final
/// output 'rendering' is a rectangle of (possibly styled) characters, a `TextUI`:
///
/// * does not represent any particular game entity
/// * should be aware of its own size before rendering (i.e. as part of object state and not as a
///  function param)
/// *
///
///
/// # Requirement `TurnLogger`
///
/// Every `TextUI` must satisfy `TurnLogger` in order to be processed appropriately in alignment
/// with the game's UI rendering process:
///
/// 1. Prepare the associated context, possibly calculating many turns ahead silently
/// 2. Once the context is at the appropriate time, calculate any future turns, providing the
/// associated information to the `TurnLogger`
/// 3. Once any relevant turns have processed, the `TextUI`'s `render` function is called to
/// request a **UI that represents all logged turns**
pub trait TextUI: TurnLogger {
    /// Renders a UI that represents **all relevant information since turn logging started**,
    /// which includes information logged from within turn resolution as well as any information
    /// this UI can / wants to fetch from the current `context`.
    ///
    /// The output is formatted like `InfoGrid`s using a determined available `w` and `h` and a
    /// preferred `formatting` for the output.
    fn render(&self, context: &dyn WorldContext, w: usize, h: usize, formatting: TextFormatting) -> Vec<String>;
}

/// Wraps information on
pub struct CombatTurnDisplay {
    ///
    formatting: TextFormatting,
    /// As the turn gets processed and this will be called as a `TurnLogger`, will gradually
    /// extend to include all verbalized `ActionStacks` to display alongside turn results on chars.
    turn_description: Vec<(String, usize)>
}

impl CombatTurnDisplay {
    pub fn with(formatting: TextFormatting) -> Self {
        CombatTurnDisplay {
            formatting,
            turn_description: Vec::new()
        }
    }
}

/// Implements `TurnLogger` to verbalize every completed `ActionStack`'s effect(s) internally.
/// All logged stacks will be included when this combat round is visualized by the display.
impl TurnLogger for CombatTurnDisplay {

    fn maneuver_stack(&mut self, stack: &ActionStack) {
        // Print Stack words and add them to this word list.
        let new_words = stack.format_words(self.formatting);
        self.turn_description.extend(new_words)
    }
}


/// Baseline visualization function of a combat turn. Based on provided `w` and `h` (and
/// `formatting`), displays this turn
/// the
impl TextUI for CombatTurnDisplay {
    fn render(&self, context: &dyn WorldContext, w: usize, h: usize, formatting: TextFormatting) -> Vec<String> {
        let mut main_layout = LinearLayout::configure(LayoutDirection::Vertical, LayoutSizing::Distribute, None);
        let character_layout = LinearLayout::from(context.find_characters(&|c| true).iter().map(|c| *c as &dyn InfoGrid).collect());
        main_layout.add(&character_layout, 1);
        // let res = self.turn_description.display(30, 4, self.formatting);
        //
        // for l in res {
        //     println!("{}", l);
        // }

        main_layout.add(&self.turn_description, 1);

        // Forward render request to now configured layout
        main_layout.display(w, h, formatting)
    }
}



#[cfg(test)]
mod tests {
    use crate::combat::{ Actor};
    use crate::equipment::{Equipment, EquipmentType};
    use crate::mov::Counter;
    use crate::text::{InfoGrid, TextFormatting};
    use crate::world::WorldContext;
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
        let mut combat = Combat::from_participants(party);

        combat
    }

    #[test]
    fn test_combat_view() {
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
        }

        for _ in 0..8 {

            let mut ui = CombatTurnDisplay::with(TextFormatting::Console);
            combat.process_turn(Some(&mut ui)).unwrap();

            for line in ui.render(&mut combat, 60, 9, TextFormatting::Console) {
                println!("{}", line);
            }

        }



    }
}