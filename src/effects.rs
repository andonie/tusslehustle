use std::fmt::{Display, Formatter};
use crate::characters::{CharStat, Character, Stats};
use crate::combat::DamageType;


/// Effect Trait flexibly describes functionality of (passive) effects affecting a character
pub trait Effect {

    /// Describe this effect briefly
    fn describe(&self) -> String;

    /// Called to develop the 'effective' current stats of this character
    fn apply_to_stats(&self, stats: &mut Stats) {
        // Default Implementation is to do nothing
    }

    /// This function is called for every effect once for every turn
    fn process_turn(&self, target: &mut Character) {
        // Default Implementation is to do nothing
    }

    /// This checker function is called once every turn (after effects and moves have been
    /// processed).
    /// This function is called on timed effects to allow checking for additional conditions to
    /// **end the effect early**.
    fn cancel_self(&self) -> bool {
        false
    }

    /// Allows effects to specify a priority number. The higher the priority number specified in
    /// `effect_order`, the **later in resolution order** will these effects applied.
    fn effect_order(&self) -> i64 {
        1
    }


}

/// A timed effect, described with a borrowed effect and a numer of turns this effect will remain
/// active
pub struct TimedEffect<'a>(&'a dyn Effect, i64);



/// ~~~~~~~~~~~~~~~~~~~~  implementations of effects ~~~~~~~~~~~~~~~~~~~~


/// Applies the value wrapped with `CharStat` additively
pub struct StatAdditive(pub CharStat);

impl Effect for StatAdditive {
    /// Quick Format: e.g. +5 DEX
    fn describe(&self) -> String {
        format!("{}{} {}", if self.0.get_value() > 0 { "+" } else { "-" }, self.0.get_value().abs(),
            self.0.get_stat_name())
    }

    /// Overwrite stat change function to add the delta value of this stat change to the input
    /// `stats` to build.
    fn apply_to_stats(&self, stats: &mut Stats) {
        match self.0 {
            CharStat::STR(d) => {
                stats.str += d;
            }
            CharStat::DEX(d) => {
                stats.dex += d;
            }
            CharStat::GRT(d) => {
                stats.grt += d;
            }
            CharStat::WIL(d) => {
                stats.wil += d;
            }
            CharStat::CHA(d) => {
                stats.cha += d;
            }
            CharStat::INT(d) => {
                stats.int += d;
            }
            // All remaining cases cover (specific) game stats
            _ => {}
        }
    }
}

/// The fundamental defense values are set by physical and magical defense. This struct represents
/// a resistance to a **damage subtype** defined by its String name and the resistance involved
/// A **negative resistance number** can be used as an additional **vulnerability** to that damage
/// type
struct DamageResistance(DamageType, f64);

impl Effect for DamageResistance {
    fn describe(&self) -> String {
        let in_percentpoints = (self.1.abs() * 100f64).floor() as i64;

        format!("{}% {} to {}", in_percentpoints,
                if self.1 > 0f64 {"RES"} else {"VUL"}, self.0)
    }



    /// Ensure this (multiplicative) effect is processed only after the more basic (additive)
    /// effects have been processed
    fn effect_order(&self) -> i64 {
        10
    }
}


#[cfg(test)]
mod tests {
    use crate::combat::{Actor};
    use super::*;

    /// Basic Testcharacter to use
    fn test_character() -> Character {
        Character::new(String::from("Lindtbert"), None, Stats {
            str: 3,
            dex: 4,
            grt: 6,
            wil: 2,
            int: 5,
            cha: 6,
        })
    }

    #[test]
    fn test_basic_statboost() {
        let mut character = test_character();

        let cha_pre = character.calculate_current_stats().cha;

        // Test Effect
        //character.receive_timed_effect(Box::new(StatChange::CHA(20)), 4, ActionContext::new(character));

        // assert_eq!(character.calculate_current_stats().cha, cha_pre+20);

        // After 4 turns, the effect should be cancelled
        for _ in 0..4 {
            // Call post turn to prgress duration
            character.post_turn()
        }

        assert_eq!(character.calculate_current_stats().cha, cha_pre);
    }
}
