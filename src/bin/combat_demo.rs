use std::thread::sleep;
use std::time::Duration;
use untitled::equipment::{Equipment, EquipmentType};
use untitled::mov::Counter;
use untitled::text::{InfoGrid, TextFormatting};
use untitled::world::WorldContext;
use untitled::characters::{Character, Stats};
use untitled::combat::{Combat, DamageType};
use untitled::effects::StatChange;
use untitled::ui::{CombatTurnDisplay, TextUI};

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
        eq.add_passive_effect(Box::new(StatChange::GRT(10)));

        lindtbert.equip(eq).unwrap();
    }

    for _ in 0..800 {

        let mut ui = CombatTurnDisplay::with(TextFormatting::Console);
        combat.process_turn(Some(&mut ui)).unwrap();

        for line in ui.render(&mut combat, 80, 5, TextFormatting::Console) {
            println!("{}", line);
        }

        sleep(Duration::from_secs(1));

        print!("\x1b[5A\x1b[80D");
    }



}

fn main() {
    test_combat_view();
}