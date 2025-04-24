//! Built (mostly) on top of the `text` module, this file contains the logic necessary to create a
//! **dynamically sized Text UI** using the game's various rendering versions outlined in
//! `TextFormatting`.
//!
//! Using basic game entity's capabilities to display themselves, this module builds **layout
//! options** that build upon this and build a flexible UI, ultimately still built upon the
//! `InfoGrid` interface.
//!
//!
//! # Layout Usage
//!
//! Since Layouts shouldn't puff
//!
//!
//!
//!

use crate::characters::{Character, Stats};
use crate::combat::{Combat, DamageType};
use crate::text::{FrameType, InfoGrid, InfoLine, JointType, TextFormatting};



pub enum LayoutDirection {
    Horizontal, Vertical,
}

/// Describes a .
pub enum LayoutWeight {
    /// Most straightforward sizing option:
    ///
    /// Each element receives `weight` number of lines / chars.
    Absolute(usize),
    /// Distribute horizontal/vertical space evenly across all elements below
    Distribute(usize)
}

impl LayoutWeight {
    pub fn amount(&self) -> usize {
        match self {
            LayoutWeight::Absolute(d) => *d,
            LayoutWeight::Distribute(d) => *d
        }
    }
}

/// A linear layout
pub struct LinearLayout<'a> {
    /// The direction that this layout projects its wrapped sub-elements on
    direction: LayoutDirection,
    /// If set, will consume additional available characters to render the frame around/between
    /// elements of this layout
    frame: Option<FrameType>,

    /// A list of all elements this layout wraps, each with their weight.
    wrapped: Vec<(&'a dyn InfoGrid, LayoutWeight)>,

}

struct CardLayout<'a> {
    header: &'a dyn InfoGrid,
    content: &'a dyn InfoGrid,
}

impl<'a> LinearLayout<'a> {

    pub fn empty() -> Self {
        LinearLayout {
            direction: LayoutDirection::Horizontal,
            frame: Some(FrameType::Single),
            wrapped: vec![],
        }
    }

    pub fn configure(direction: LayoutDirection, frame: Option<FrameType>) -> Self {
        LinearLayout {
            direction,
            frame,
            wrapped: vec![],
        }
    }

    pub fn from(wrapped: Vec<&'a dyn InfoGrid>) -> Self {
        let mut ret = Self::empty();
        for w in wrapped {
            ret.add(w, LayoutWeight::Distribute(1));
        }
        ret
    }

    pub fn add(&mut self, g: &'a dyn InfoGrid, weight: LayoutWeight) {
        self.wrapped.push((g, weight));
    }

    pub fn set_direction(&mut self, d: LayoutDirection) {
        self.direction = d;
    }

    pub fn set_frame(&mut self, f: Option<FrameType>) {
        self.frame = f;
    }


    // Redis Helper Functions

    /// Distributes exactly `size` among the given Sub-Elements based on their weights configuration
    ///
    /// # Returns
    ///
    /// The returned Vector contains references tuples of contained sub-element + available amount
    /// of the given `size` (which can be interpreted as width / height as needed). The numbers
    /// provided in the second tuple parameter are guaranteed to add up to `size` (unless
    /// absolute weight configuration exceeds available `size`).
    fn distribute(&self, size: usize) -> Vec<(&'a dyn InfoGrid, usize)> {
        // Identify the amount allocated by absolute weights
        let absolute_amount = self.wrapped.iter().filter(|(_, w)| match w {
            LayoutWeight::Absolute(_) => true,
            LayoutWeight::Distribute(_) => false
        }).fold(0, |acc, (_, abs_weight)| acc+abs_weight.amount());

        if absolute_amount > size {
            panic!("Provided only {} size but absolute weights add up to {}", size, absolute_amount);
        }

        // Determine available size to distribute among relative weights
        let available_for_distribution = size - absolute_amount;

        let total_relative_weights = self.wrapped.iter().filter(|(_, w)|
            matches!(w, LayoutWeight::Distribute(_)))
            .fold(0, |acc, (_, w)| acc+w.amount());

        // Calculated list of all elements in order with associated length each
        // Will be updated throughout the rest of this process.
        let mut calculated_lengths: Vec<(&'a dyn InfoGrid, usize, Option<usize>)> = self.wrapped.iter().map(|(e, w)| match w {
            // None signals no way for absolute weights to receive 'extra' from under-distribution
            LayoutWeight::Absolute(w) => (*e, *w, None),
            LayoutWeight::Distribute(w) => {
                let numerator = available_for_distribution * w;
                (*e, numerator/total_relative_weights, Some(numerator%total_relative_weights))
            }
        }).collect();


        let mut indices: Vec<usize> = (0..calculated_lengths.len()).collect();
        // Sort indices by remainder
        indices.sort_by_key(|i| match calculated_lengths[*i].2 {
            // Super low priority for
            None => -200i32,
            Some(r) => r as i32,
        });

        let undistributed = size - calculated_lengths.iter().fold(0, |acc, (_, used_length, _ )| acc+used_length);

        for &i in indices.iter().take(undistributed) {
            calculated_lengths[i].1 += 1;
        }
        
        calculated_lengths.into_iter().map(|(el, len, _)|  (el, len)).collect()
    }

}

/// Flexibly displays this Linear Layout based on configured sizing strategy.
impl<'a> InfoGrid for LinearLayout<'a> {
    fn display(&self, w: usize, h: usize, formatting: TextFormatting) -> Vec<String> {

        let mut output = Vec::new();

        // In case horizontal layout computes horizontal legths, save each column length in here
        let mut h_lengths: Vec<usize> = Vec::new();

        let built_content_lines: Vec<Vec<String>> = match &self.direction {
            // Horizontal Layout:
            // -> Attach all individual lines created by the wrapped elements
            LayoutDirection::Horizontal => {
                // Number of available line only shifts by 2 for frames
                let available_line_num = h - if let Some(_) = self.frame {2} else {0};
                // Calculate available line length for wrapped,
                // taking out chars allocated for spacing between elements (such as " | ")
                let available_line_len = w - (self.wrapped.len() - 1) // minimum: empty spaces
                    - if self.frame.is_some() {
                    (self.wrapped.len() + 1) * 2 // Two additional chars (space+bar char) per wrapped + 1 (inside + around)
                } else {0};


                // Distribute the WIDTH across all elements as per weighing
                let distributed = self.distribute(available_line_len);
                
                // Build the lines from each distributed element as finished content lines
                distributed.into_iter().map(|(el, size)| el.display(size, available_line_num, formatting)).collect()

            }
            LayoutDirection::Vertical => {
                // Vertical Layout:
                // Each element receives 100% (available) width

                // Number of available lines shifts if a frame is involved by one line between (and
                // around) all wrapped elements
                let available_line_num = h - if let Some(_) = self.frame {self.wrapped.len()+1} else {0};

                // Calculate Available Line Width (Account for Frame elements ("| " per side)
                let available_line_width = w - if self.frame.is_some() {4} else {0};
                
                // Distribute the HEIGHT (lines) across all elements
                let distributed = self.distribute(available_line_num);
                
                // Build the lines from each element as finsihed content lines
                distributed.into_iter().map(|(el, size)| el.display(available_line_width, size, formatting)).collect()
            }
        };

        // With all subgrids rendered, build the layouted content by concatenating inputs
        // appropriately, and adding frame characters (if configured)
        match &self.direction {
            LayoutDirection::Horizontal => {
                // Number of available line only shifts by 2 for frames
                let available_line_num = h - if let Some(_) = self.frame {2} else {0};
                // If a Frametype is provided, start with a row of the frame
                if let Some(frametype) = &self.frame {
                    let mut top_row = String::with_capacity(w);
                    top_row.push(frametype.top_left());
                    for (n, g) in built_content_lines.iter().enumerate() {
                        // Fill horizontal bits for the whole grid + 2 spaces on the side
                        top_row.push_str(&frametype.hor().to_string().repeat(h_lengths[n] + 2));
                        // Push T Junction (unless this is the last, in which case we add a corner)
                        if n != built_content_lines.len() - 1 {
                            top_row.push(frametype.joint(JointType::TDown));
                        } else {
                            top_row.push(frametype.top_right());
                        }
                    }
                    output.push(top_row);
                }

                // Concatenate all individual grid lines
                for i in 0..available_line_num {
                    let mut line = String::with_capacity(w);

                    // If Applicable: Opening Frame Line
                    if let Some(f) = &self.frame {
                        line.push(f.ver());
                        line.push(' ');
                    }

                    // Zip Together Content
                    for (x, grid) in built_content_lines.iter().enumerate() {
                        if i >= grid.len() {
                            for line in grid {
                                println!("Line: {}", line);
                            }
                        }
                        line.push_str(&grid[i]);
                        line.push(' ');
                        // If frame type set, add frame after each grid
                        if let Some(f) = &self.frame {
                            line.push(f.ver());
                            if x != built_content_lines.len() - 1 {
                                line.push(' ');
                            }
                        }
                    }



                    output.push(line);
                }

                // If a Frametype is provided, finish with a row of the frame
                if let Some(frametype) = &self.frame {
                    let mut bottom_row = String::with_capacity(w);
                    bottom_row.push(frametype.bottom_left());
                    for (n, g) in built_content_lines.iter().enumerate() {
                        // Fill horizontal bits for the whole grid + 2 spaces on the side
                        bottom_row.push_str(&frametype.hor().to_string().repeat(h_lengths[n] + 2));
                        // Push T Junction (unless this is the last, in which case we add a corner)
                        if n != built_content_lines.len() - 1 {
                            bottom_row.push(frametype.joint(JointType::TUp));
                        } else {
                            bottom_row.push(frametype.bottom_right());
                        }
                    }
                    output.push(bottom_row);
                }

            }
            LayoutDirection::Vertical => {

                // If a frame is provided, the top row is just the frame
                if let Some(frametype) = &self.frame {
                    output.push(format!("{}{}{}", frametype.top_left(),
                                        frametype.hor().to_string().repeat(w-2),
                                        frametype.top_right()));
                }

                // Put all inputs together
                let last_line_index = built_content_lines.len() - 1;
                for (line_index, lines) in built_content_lines.into_iter().enumerate() {
                    let final_index = lines.len() - 1;
                    for (n, line) in lines.into_iter().enumerate() {
                        match &self.frame {
                            None => output.push(line),
                            Some(f) => {
                                output.push(format!("{} {} {}", f.ver(), line, f.ver()));
                                if n == final_index && line_index != last_line_index {
                                    output.push(format!("{}{}{}", f.joint(JointType::TRight), f.hor().to_string().repeat(w-2), f.joint(JointType::TLeft)));
                                }
                            }
                        }
                    }

                }

                // If a frame is provided, the bottom row is just the frame
                if let Some(frametype) = &self.frame {
                    output.push(format!("{}{}{}", frametype.bottom_left(),
                                        frametype.hor().to_string().repeat(w-2),
                                        frametype.bottom_right()));
                }
            }
        }


        output
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
    fn test_layout() {
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

            combat.process_turn(None).unwrap();

            {
                let lindtbert = combat.get_character(&"Lindtbert".to_string()).unwrap();
                let baddie = combat.get_character(&"Baddie".to_string()).unwrap();
                // Map each Character to their individual line-by-line output

                let mut view = LinearLayout::from(vec![lindtbert, baddie]);

                view.set_frame(Some(FrameType::Double));
                view.set_direction(LayoutDirection::Vertical);

                for line in view.display(50, 12, TextFormatting::Console) {
                    println!("{}", line);
                }


            }
        }



    }
}
