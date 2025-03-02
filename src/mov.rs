use crate::characters::Character;
use crate::combat::{DamageType, Actor, Damage, Action, EntityPointer, ActionEffect};
use crate::effects::Effect;
use crate::equipment::Equipment;
use crate::world::WorldContext;


/// Describes character abilities that affect the `ActionStack`, which come in two types:
///
/// * `Maneuver`s, which are 'standalone attacks'
/// * `Reaction`s, which are conditional moves that cost AP and are reactions to other moves.
///
/// This trait describes their shared behavior.
pub trait Move {
    /// Every move has a name
    fn name(&self) -> String;

    /// Renders a general description of this Move
    fn describe(&self) -> String;

    /// Returns how much MP executing this move costs. Default is 0 (e.g. for basic
    /// physical attacks)
    fn mp_cost(&self) -> i64 {
        0
    }
}

/// Describes an individual activty a character can initiate
/// during one turn-based combat round.
/// During one round, **every character gets one maneuver only**.
pub trait Maneuver: Move {


    /// Called during turn-resolution to execute the move on a given `character`, the provided
    /// `context` can be used during the move resolution, e.g. to identify targets.
    ///
    /// Each move returns exactly one "initial action" that starts the stack to resolve this move.
    fn execute(&self, character: &Character, context: &dyn WorldContext) -> Vec<Action>;

}

/// Describes a reaction. Reactions can be made **towards any character move/action and to other
/// reactions**. The original move that starts it plus any reactions for an `ActionStack` during
/// combat, allowing for sophisticated moves, like powerful attacks ping-ponging between Counters.
pub trait Reaction: Move {
    /// Every reaction has an associated AP Cost. AP are a unit to measure a character's ability
    /// to react to what's happening and regenerate passively.
    fn ap_cost(&self) -> i64;

    /// Calculates any actions this `Reaction` would do in response to an another `action` that
    /// has just been 'announced' on the `ActionStack`. During turn resolution, the ActionStack
    /// keeps requesting reactions until all Characters are satisfied.
    ///
    /// This function returns:
    ///
    /// * `None` if this reaction doesn't apply to the given `action`.
    /// * `Some(vector)` **filled with one or more `Action` objects** that represent this reaction
    /// applied to the given world `context`.
    fn react(&self, character: &Character, action: &Action, context: &dyn WorldContext) -> Option<Vec<Action>>;
}

/// A very basic move that is available to all characters
pub struct BarehandedBlow;

impl Move for BarehandedBlow {
    fn name(&self) -> String {
        "Barehanded Blow".to_string()
    }

    fn describe(&self) -> String {
        "When nothing else helps, your body is a weapon, too. A meager move, except in the hands \
        of the exceptionally physically capable.".to_string()
    }
}

impl Maneuver for BarehandedBlow {


    ///
    fn execute(&self, character: &Character, context: &dyn WorldContext) -> Vec<Action> {
        // Calculate Damage
        let stats = character.calculate_current_stats();
        let blow_damage = (stats.dex + stats.str)*3 // Main DMG stats
            + stats.grt + stats.int;
        // Adjust stat-based factors a multiplier, ensure minimum damage
        let blow_damage = ((blow_damage as f64 * 0.45) as i64).max(1);
        let blow_damage = ActionEffect::Attack(Damage(DamageType::PHY("Strike"), blow_damage));

        // This move can at maximum attack one target
        // -> Start of with all valid targets, i.e. non-party members
        let targets = context.find_characters(
            // Look for anyone who's not part of our party
            &|char: &Character| !char.party_check(character.party()));

        // Attack the target with the lowest HP to Max HP ratio
        let target = targets.iter().max_by_key(
            |c| (c.hp_to_max_hp_ratio() * 1000f64) as i64).expect("Couldn't find a target");

        vec![Action::from_source(character.as_target(), blow_damage, target.as_target())]
    }
}

struct WeaponAttack<'a>(&'a Equipment);


/// Describes a general Counter Ability. A counter attack is a **reaction to a Damage Effect**,
/// that can **reduce incoming damage** and/or **counter-damage the attacker**.
pub struct Counter {
    /// Describes the Damage Types this counter applies to.
    /// If this type is set one of the main damage types with empty string, e.g.
    /// `DamageType::PHY("")`, any physical or any magical damage will be countered.
    /// If this type is set to `DamageType::ULT`, any incoming damage will apply for this effect.
    damage_type: DamageType,
    /// Received damage could be reduced by the counter, represented by this factor.
    /// If this is `1`, no damage reduction is applied
    incoming_factor: f64,
    /// If this value is larger than `0`, this value
    outgoing_factor: f64,
}

impl Counter {

    pub fn new(damage_type: DamageType, incoming_factor: f64, outgoing_factor: f64) -> Counter {
        Counter {
            damage_type,
            incoming_factor,
            outgoing_factor,
        }
    }

    /// Checks whether this instance would react to the `incoming` damage type.
    fn relevant_for(&self, incoming: &DamageType) -> bool {
        // ULT Damage is unblockable
        if let DamageType::ULT = incoming {
            return false;
        }

        match self.damage_type {
            DamageType::PHY(s) => {
                if s == "" {
                    // Empty string means any PHY subtype is defended against
                    matches!(incoming, DamageType::PHY(_))
                } else {
                    if let DamageType::PHY(s2) = incoming {
                        s == *s2
                    } else {
                        false
                    }
                }

            },
            DamageType::MAG(s) => {
                if s == "" {
                    // Empty string means any MAG subtype is defended against
                    matches!(incoming, DamageType::MAG(_))
                } else {
                    if let DamageType::MAG(s2) = incoming {
                        s == *s2
                    } else {
                        false
                    }
                }

            },
            DamageType::ZAP(s) => {
                if s == "" {
                    // Empty string means any ZAP subtype is defended against
                    matches!(incoming, DamageType::ZAP(_))
                } else {
                    if let DamageType::ZAP(s2) = incoming {
                        s == *s2
                    } else {
                        false
                    }
                }

            },
            // ULT -> Generic Counter against any damage
            DamageType::ULT => true,
        }
    }
}

impl Move for Counter {
    fn name(&self) -> String {
        "Counter".to_string()
    }

    fn describe(&self) -> String {
        "Counter".to_string()
    }

    /// Update MP Cost to reflect calculated cost based on value
    fn mp_cost(&self) -> i64 {
        let mut total_cost = 0;

        // Account for Counter Damage Reduction
        // Damage Reduction Up To / Until 30% is free
        let red_cutoff = 0.7f64;
        if self.incoming_factor < red_cutoff {
            let red_cost = ((red_cutoff - self.incoming_factor) * 10f64) as i64;
            total_cost += red_cost;
        }

        // Account for Counter Return Damage
        // Return Damage Up To / Until 30% is free
        let dmg_cutoff = 0.3f64;
        if self.outgoing_factor > dmg_cutoff {
            let dmg_cost = ((self.outgoing_factor - dmg_cutoff) * 15f64) as i64;
            total_cost += dmg_cost;
        }

        total_cost
    }
}

impl Reaction for Counter {

    fn ap_cost(&self) -> i64 {
        3
    }

    fn react(&self, character: &Character, action: &Action, context: &dyn WorldContext) -> Option<Vec<Action>> {
        // Requirement 1: Only affecting actions that target me as a character directly
        if !action.targets_character(character.name()) {
            return None
        }
        // Requirement 2: Only reacting to incoming `Attack`s
        if let ActionEffect::Attack(Damage(dt, damage)) = action.get_effect() {
            // Requirement 3: Only reacting if the dt matches.
            if self.relevant_for(dt) {
                // All Checks passed! Build the counter-action
                let mut res = Vec::new();

                // Possibly reduce incoming damage
                if self.incoming_factor != 1f64 {
                    res.push(Action::from_source(character.as_target(), ActionEffect::AdjustDamageMul(self.incoming_factor), action.build_self_target()))
                }

                // Possibly return a counter attack
                if self.outgoing_factor != 0f64 {
                    res.push(Action::from_source(character.as_target(), ActionEffect::Attack(Damage(dt.clone(), (self.outgoing_factor * *damage as f64) as i64)), action.get_source().clone()))
                }

                Some(res)
            } else {
                None
            }
        } else {
            // This action is not a valid response target of this reaction
            None
        }
    }
}
