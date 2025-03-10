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

/// Lists different sizing strategies layouts can use.
pub enum LayoutSizing {
    /// Most straightforward sizing option:
    ///
    /// Each element receives `weight` number of lines / chars.
    Absolute,
    /// Distribute horizontal/vertical space evenly across all elements below
    Distribute
}

/// A linear layout
pub struct LinearLayout<'a> {
    /// The direction that this layout projects its wrapped sub-elements on
    direction: LayoutDirection,
    /// The sizing strategy to use
    sizing: LayoutSizing,
    /// If set, will consume additional available characters to render the frame around/between
    /// elements of this layout
    frame: Option<FrameType>,

    /// A list of all elements this layout wraps, each with their weight.
    wrapped: Vec<(&'a dyn InfoGrid, usize)>,

}

struct CardLayout<'a> {
    header: &'a dyn InfoGrid,
    content: &'a dyn InfoGrid,
}

impl<'a> LinearLayout<'a> {

    pub fn empty() -> Self {
        LinearLayout {
            direction: LayoutDirection::Horizontal,
            sizing: LayoutSizing::Distribute,
            frame: Some(FrameType::Single),
            wrapped: vec![],
        }
    }

    pub fn configure(direction: LayoutDirection, sizing: LayoutSizing, frame: Option<FrameType>) -> Self {
        LinearLayout {
            direction,
            sizing,
            frame,
            wrapped: vec![],
        }
    }

    pub fn from(wrapped: Vec<&'a dyn InfoGrid>) -> Self {
        let mut ret = Self::empty();
        for w in wrapped {
            ret.add(w, 1);
        }
        ret
    }

    pub fn add(&mut self, g: &'a dyn InfoGrid, weight: usize) {
        self.wrapped.push((g, weight));
    }

    pub fn set_direction(&mut self, d: LayoutDirection) {
        self.direction = d;
    }

    pub fn set_frame(&mut self, f: Option<FrameType>) {
        self.frame = f;
    }

    pub fn set_sizing(&mut self, s: LayoutSizing) {
        self.sizing = s;
    }

}

/// Flexibly displays this Linear Layout based on configured sizing strategy.
impl<'a> InfoGrid for LinearLayout<'a> {
    fn display(&self, w: usize, h: usize, formatting: TextFormatting) -> Vec<String> {

        let mut output = Vec::new();

        // In case horizontal layout computes horizontal legths, save each column length in here
        let mut h_lengths = Vec::new();

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


                // Based on sizing strategy, build the content / lines for each wrapped
                match self.sizing {
                    LayoutSizing::Absolute => {
                        // Directly forward weights into w's of wrapped
                        self.wrapped.iter().map(|(g, weight) | g.display(*weight, available_line_num, formatting)).collect()
                    }
                    LayoutSizing::Distribute => {
                        // Calculate each subgrid's available `w` by distributing weights
                        // relative to each other / available space
                        let total_weight = self.wrapped.iter().fold(0, |mut acc, (g, w)| acc + *w );

                        // Review all grids, based on their weight and store them with remainder
                        let mut charnum_w_remainder = self.wrapped.iter().map(|(g, w)| {
                            let numerator = w * available_line_len;
                            (*g, numerator / total_weight, numerator % total_weight)
                        }).collect::<Vec<(&dyn InfoGrid, usize, usize)>>();

                        let distributed = charnum_w_remainder.iter().fold(0, |mut acc, (_, d, _)| acc+d);
                        let remaining = available_line_len - distributed;

                        assert!(remaining < self.wrapped.len());

                        // Distribute `remaining` chars among the `remaining` number of wrapped
                        // in order of highest remainder to lowest
                        let mut indices: Vec<usize> = (0..charnum_w_remainder.len()).collect();
                        // Sort indices by remainder
                        indices.sort_by_key(|i| charnum_w_remainder[*i].2);

                        for &i in indices.iter().take(remaining) {
                            charnum_w_remainder[i].1 += 1;
                        }

                        h_lengths = charnum_w_remainder.iter().map(|(g, w, _)| *w).collect();
                        charnum_w_remainder.iter().map(|(g, w, _) | g.display(*w, available_line_num, formatting)).collect()
                    }
                }
            }
            LayoutDirection::Vertical => {
                // Vertical Layout:
                // Each element receives 100% (available) width

                // Number of available lines shifts if a frame is involved by one line between (and
                // around) all wrapped elements
                let available_line_num = h - if let Some(_) = self.frame {self.wrapped.len()+1} else {0};

                // Calculate Available Line Width (Account for Frame elements ("| " per side)
                let available_line_width = w - if self.frame.is_some() {4} else {0};

                // Based on sizing strategy, distribute all available lines to each wrapped element
                match self.sizing {
                    LayoutSizing::Absolute => {
                        // Absolute Layout --> Every Subgrid receives exactly `weight` lines
                        // to work with.
                        self.wrapped.iter().map(|(g, weight)| g.display(available_line_width, *weight, formatting)).collect()
                    }
                    LayoutSizing::Distribute => {
                        let total_weight = self.wrapped.iter().fold(0, |mut acc, (_, weight)| acc + *weight);
                        let mut linenums_w_remainder: Vec<(&dyn InfoGrid, usize, usize)> = self.wrapped.iter().map(|(g, w)| {
                            let numerator = w * available_line_num;
                            (*g, numerator / total_weight, numerator % total_weight)
                        }).collect();

                        let distributed = linenums_w_remainder.iter().fold(0, |mut acc, (_, w, _)| acc+w);
                        let remaining = available_line_num - distributed;
                        assert!(remaining < self.wrapped.len());

                        let mut indices: Vec<usize> = (0..linenums_w_remainder.len()).collect();
                        // Sort indices by remainder
                        indices.sort_by_key(|i| linenums_w_remainder[*i].2);

                        for &i in indices.iter().take(remaining) {
                            linenums_w_remainder[i].1 += 1;
                        }

                        linenums_w_remainder.iter().map(|(g, he, _)| g.display(available_line_width, *he, formatting)).collect()
                    }
                }
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
