//! Text is a critical aspect of this game, so formatting it effectively and richly is critical
//! all the way up to the user. This file contains basic text features for effective display of
//! game information to the user.
//!
//!
//!

use std::thread::current;

/// Interactions and Game Instances support **display with (monospaced) text**
/// This enum lists different ways to format **monospaced text of the same length**:
///
/// * `Plain`: Display the text only
/// * `Html`: Display the text in a HTML format. HTML attributes can contain richer data without
/// increasing the lengths/size of the output.
/// * `Console`: Output formatted with color codes that work in console
#[derive(Copy, Clone)]
pub enum TextFormatting {
    /// Line Formatting as plain string, nothing else.
    Plain,
    /// Line Formatting with <span>'s covering the content with rich HTML attributes including
    /// more information
    Html,
    /// Line Formatting for console (including colors)
    Console
}

impl TextFormatting {



    pub fn format_html(plain_string: String, info_class: &str, more_info: Option<String>) -> String {
        // If more info is provided, we format it as a proper HTML input.
        let more_info = match more_info {
            None => "",
            Some(info) => &format!("data-info=\"{}\"", info)
        };
        format!("<span class=\"{info_class}\"{more_info}>{plain_string}</span>")
    }


    /// Returns an appropriate console color code.
    fn console_color_lookup(info_class: &str) -> &str {
        match info_class {
            "hp" => "\x1b[31m", // Red
            "mp" => "\x1b[34m", // Blue
            "ap" => "\x1b[32m", // Green
            "PHY" => "\x1b[34m",//
            &_ => "", // Unknown case -> do no color change / empty string
        }
    }

    pub fn format_console(plain_string: String, info_class: &str) -> String {
        format!("{}{plain_string}\x1b[0m", Self::console_color_lookup(info_class))
    }

    /// This function can resolve any formatting to it's 'resolved' `String`, which might have a
    /// longer length than is acceptable on-screen when printing plainly, but would **retain
    /// the original `plain_string` input length** when displayed in the appropriate context, e.g.
    /// in web browser.
    ///
    /// Not all paramters are used in all types (e.g. `Plain` uses nothing, just returning the
    /// original input string.
    ///
    /// `Html`: Will return a *span* that wraps the original `plain_string`.
    /// `Console`: Will add colors when appropriate
    /// `Plain`: Will do no changes
    ///
    /// # Paramaters
    ///
    /// * `plain_string` to enrich
    /// * `info_class`: A small (system known) tag that provides much information with only one
    ///     `&str` param
    /// * `more_info`: If
    pub fn enrich_text(&self, plain_string: String, info_class: &str, more_info: Option<String>) -> String {
        match self {
            // No formatting needed in Plain formatting
            TextFormatting::Plain => plain_string,
            // To enrich the plain string in HTML, cover it in a <span>
            TextFormatting::Html => TextFormatting::format_html(plain_string, info_class, more_info),
            // If the `info_class` is known, add a console color (code) to this information
            TextFormatting::Console => TextFormatting::format_console(plain_string, info_class),
        }
    }

    /// Utility function can be used to split a **plain** (i.e. unformatted) string into words,
    /// each with
    ///
    /// # Parameters
    ///
    /// * `sentence`: Can technically be more (or less) than one sentence. A set of **unformatted**
    /// words to describe what's happening.
    /// * `info_class`: The info class to apply to all words.
    /// * `more_info`: Additional info to include with the words (in HTML formatting)
    pub fn to_words(&self, sentence: String, info_class: &str, mut more_info: Option<String>) -> Vec<(String, usize)> {
        sentence.split_whitespace().map(|w|
            (self.enrich_text(w.to_string(), info_class, more_info.take()), w.len())).collect()
    }
}

pub trait InfoLine {
    /// Looking at its current internal state, builds all key info of this actor into one line,
    /// applying possible `formatting`.
    ///
    /// # Params
    ///
    /// * `len`: The target length of the output (exact).
    /// * `formatting`: The formatting to use for the output
    fn format_line(&self, len: usize, formatting: TextFormatting) -> String;
}

/// Describes a game entity that can be flexibly printed across multiple lines
pub trait InfoGrid {
    /// Flexible display function renders the key information of entity into (one or many)
    /// individual lines of text.
    ///
    /// Entities of `FlexMultiLine` can be organized into flexible Layouts to visualize their
    /// information.
    ///
    /// # Params
    ///
    /// Sizing:
    ///
    /// * `len`: (Character) length of each line
    /// * `num_lines`: Number of lines to print.
    ///
    /// Each entity is expected to 'smartly rearrange' with available size changing.
    ///
    /// Formatting:
    ///
    /// * `formatting`: The formatting to apply.
    fn display(&self, w: usize, h: usize, formatting: TextFormatting) -> Vec<String>;
}

// ~~~~~~~~~~ Implementation of Shortening/Padding of known types ~~~~~~~~~~~

/// Not knowing anything about the information encoded in `self`, this implements a
/// conditional string shortening / padding to ensure the line length is met exactly.
impl InfoLine for String {
    /// Ignore text formatting (plain numbers don't have context)
    fn format_line(&self, len: usize, _: TextFormatting) -> String {
        // Always start of with a clone of the base data
        let ret = self.clone();
        if ret.len() == len {
            // Length Match! No operations necessary
            ret
        } else if len > ret.len() {
            // This text is too short. Pad with empty space
            format!("{}{}", ret, " ".repeat(len - ret.len()))
        } else {
            // This text is too long. Cut and add ".." to indicate we didn't fit it all.
            let cutoff = ret.chars().take(len).collect::<String>();
            format!("{}..", cutoff)
        }
    }
}

/// Implementation of `i64` display lines. This turns an `i64` (most commonly used number type in
/// game) into a nice, **effectively shortened** number, e.g.
///
/// * `4`
/// * `1.45 K`
/// * `23.2 B`
impl InfoLine for i64 {
    /// Ignore text formatting (plain numbers don't have context)
    fn format_line(&self, len: usize, f: TextFormatting) -> String {
        let abs_num = self.abs() as f64;
        if *self < 0 {
            // recursively solve this issue
            return format!("-{}", (-self).format_line(len-1, f));
        }


        let (value, suffix) = if abs_num >= 1_000_000_000.0 {
            (*self as f64 / 1_000_000_000.0, "B")
        } else if abs_num >= 1_000_000.0 {
            (*self as f64 / 1_000_000.0, "M")
        } else if abs_num >= 1_000.0 {
            (*self as f64 / 1_000.0, "K")
        } else {
            (*self as f64, "") // No shortening needed
        };

        // Precision of rounding is based on Removing the M/B/K and the 'size' of the
        // number of Ms/Bs/Ks before the comma
        let precision = if value >= 100.0 {
            if len < 5 {
                0
            } else {
                len - 5
            }
        } else if value >= 10.0 {
            if len < 4 {
                0
            } else {
                len - 4
            }
        } else {
            if len < 3 {
                0
            } else {
                len - 3
            }
        };

        // Format based on precision
        let formatted = format!("{:.*}", precision, value);

        //

        // Remove trailing ".0" when precision is 0
        let mut result = if let Some(_) = formatted.find('.') {
            formatted.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            formatted.clone()
        };

        while result.len() < len {
            // Result length is one short. This can happen because the "." character also requires
            // Space. In this case, pad with empty space
            result.push(' ');
        }

        result
    }
}

/// Describes **formatted game info** that is **encoded in human-readable words**.
/// This trait applies to things like an Action Stack that expresses 'rich' information that can't
/// be encoded in constructs like bars or (statically placed) numbers alone.
///
/// # Displaying words
///
/// Words are expected to be printed one after another, separated by " "
pub trait MakesWords {
    /// Formats a list of individiual words, each with their **visible charlength**.
    fn format_words(&self, formatting: TextFormatting) -> Vec<(String, usize)>;
}

/// 'Trivial' Implementation of `MakesWords` from forwards content to formatting `enrich_text`
impl MakesWords for Vec<(String, &str, Option<String>)> {
    fn format_words(&self, formatting: TextFormatting) -> Vec<(String, usize)> {
        self.clone().into_iter().map(|(w, info_class, add_info)| {
            let len = w.len();
            (formatting.enrich_text(w, info_class, add_info), len)
        }).collect()
    }
}


/// Convenience function builds
fn fold_word_line_break(w: usize) -> Box<dyn Fn(Vec<Vec<(String, usize)>>, (String, usize)) -> Vec<Vec<(String, usize)>>> {
    let fun = move |mut acc: Vec<Vec<(String, usize)>>, (word, wordlength): (String, usize)| {
        let last_line = acc.last_mut().unwrap();

        if last_line.is_empty() {
            last_line.push((word, wordlength));
            return acc;
        }

        // Build total character length of this line from wrapped words + spaces in between
        let current_charlen = last_line.iter().fold(0, |mut acc, (_, l)| acc + l)
            + last_line.len() - 1; // One char for space allocated between all words


        if current_charlen + wordlength + 1 > w {
            // If empty spaces are needed to finish of `last_line`'s appropriate length, add
            // add them to the last word
            if current_charlen < w {
                let (word, l) = last_line.last_mut().unwrap();
                for _ in 0..(w - current_charlen) {
                    word.push(' ');
                }
                *l += w-current_charlen;
            }
            acc.push(vec![(word, wordlength)]);
        } else {
            // Enough Space -> Add to current line
            last_line.push((word, wordlength));
        }

        acc
    };
    Box::new(fun)
}

fn truncate_outlist(out_lines: &mut Vec<String>, h: usize) {
    if out_lines.len() > h {
        out_lines.truncate(h);
        let mut last_line = out_lines.last_mut().unwrap();
        if last_line.len() >= 3 {

        }
    }
}

fn expand_wordlists(linewords: Vec<Vec<(String, usize)>>, w: usize) -> Vec<String> {
    // Calculate the length of the last line
    let last_line = linewords.last().unwrap();
    let last_line_length = last_line.iter().map(|(word, l)| *l).sum::<usize>()
        + last_line.len() - 1; // Add one empty space in between every word
    let mut out: Vec<String> = linewords.into_iter()
        // Concatenate words
        .map(|words| words.iter().fold(String::new(), |mut acc, (w, _)| {
            if acc.is_empty() {
                acc.push_str(w);
                acc
            } else {
                acc.push(' ');
                acc.push_str(w);
                acc
            }
        }))
        .collect();

    if last_line_length < w {
        // Pad last line with empty spaces as needed
        let mut last_line = out.last_mut().unwrap();
        for _ in 0..(w-last_line_length) {
            last_line.push(' ');
        }
    }
    // Validate that laste line is expanded to correct size



    out
}

/// Anything that makes words can be displayed as an `InfoGrid`. This implementation takes all words
/// provided by the trait and **wraps words around lines** as necessary.
impl<T: MakesWords> InfoGrid for T {

    fn display(&self, w: usize, h: usize, formatting: TextFormatting) -> Vec<String> {
        /// Helper function uses up words until the line is filled, always returning
        /// lines properly filled with `w` visible characters

        let words = self.format_words(formatting);

        // Split words first into lines as needed
        // implementation is modelled as a single fold, consuming all words generated
        let line_split_words = words.into_iter().fold(vec![vec![]], fold_word_line_break(w));

        // Now expand all sorted lines of words into String lines
        let mut out_lines: Vec<String> = expand_wordlists(line_split_words, w);

        truncate_outlist(&mut out_lines, h);

        out_lines
    }
}

/// Implementation of `InfoGrid` for a list of interpreted words ignores the provided
/// `TextFormatting`, as each word is presumably formatted.
impl InfoGrid for Vec<(String, usize)> {
    fn display(&self, w: usize, h: usize, _: TextFormatting) -> Vec<String> {
        // Fold lines
        let line_split_words = self.clone().into_iter().fold(vec![vec![]], fold_word_line_break(w));

        // Now expand all sorted lines of words into String lines
        let mut out_lines: Vec<String> = expand_wordlists(line_split_words, w);

        // Truncate if too long
        truncate_outlist(&mut out_lines, h);

        // Fill with empty spaces if too short
        if out_lines.len() < h {
            for _ in 0..(h - out_lines.len()) {
                out_lines.push(" ".to_string().repeat(w));
            }
        }

        out_lines

    }
}




/// Implements a **line wrap** over a set

pub mod text_util {
    use crate::text::{text_util, BarStyle, TextFormatting, InfoLine};

    /// Renders a nice labeled bar.
    ///
    pub fn render_bar_with_num(label: &str, max_len: usize, num: i64, bar_max: i64,
                               bar_style: BarStyle, bar_wrappers: Option<(char, char)>,
                               formatting_info: Option<(&TextFormatting, &str, String)>) -> String {
        let mut result = String::with_capacity(max_len);
        result.push_str(label);
        let mut bar_size = max_len-label.len(); // Default calc for small render case
        if max_len < 12 {
            // Smallest Case: Render HP as bar only
        } else {
            // Ad an empty space
            result.push(' ');
            // Add 5 Characters for HP
            result.push_str(&num.format_line( 5, TextFormatting::Plain));

            // Update bar size to reflect additional characters
            bar_size = max_len - label.len() - 6;
        }

        // Based on whether or not the bar is surrounded by outside characters,
        // Calculate appropriate bar size and render, taking into account formatting

        // If we have bar wrappers, discount the two characters from the calculated bar size.
        if let Some(_) = bar_wrappers {
            bar_size -= 2;
        }

        // Based on Formatting Infos provided, develop and render the Bar characters
        let bar_string = match formatting_info {
            // No formatting infos provided. Render plainly
            None => &bar_style.render_bar(bar_size, num, bar_max),
            Some((f, i_class, more_i)) => &f.enrich_text(bar_style.render_bar(bar_size, num, bar_max), i_class, Some(more_i))
        };

        match bar_wrappers {
            None => {
                result.push_str(bar_string);
            }
            Some((a, b)) => {
                result.push(a);
                result.push_str(bar_string);
                result.push(b);
            }
        }

        result
    }
}

// Implement flexi display traits for basic types

/// Represents the different styles of bars that can be used to display quotients.
pub enum BarStyle {
    /// An especially helpful style for small spaces, this prints a double-lined health bar:
    ///
    /// `::::::...`
    /// `:::......`
    /// `...      `
    DoubleLines,

    ///
    /// e.g. `TwoChars()` to represent the classic console loading animatino style:
    ///
    /// `█████████▒▒▒▒`
    ///
    /// More examples:
    /// `++++++++-----`
    /// `!!!!!!!!!....`
    /// `>>>>---------`
    /// `[[[[[[_______`
    TwoChars(char, char),

    /// Using a single `character repeatedly to render the bar,
    /// displaying negative space with an **empty space** to show what's missing to the max
    SingleChar(char),
}

impl BarStyle {

    /// Builds a string that represents the respective `BarStyle`, rendered for a given number
    /// of total characters `out_len`
    pub fn render_bar(&self, out_len: usize, bar_value: i64, bar_max: i64) -> String {
        // Build Output Variable, designed to hold exactly the amount specified for the output
        let mut out = String::with_capacity(out_len);
        // Do some of the math we'll need in all / most scenarios
        let ratio = bar_value as f64 / bar_max as f64;
        match self {
            BarStyle::DoubleLines => {
                // If the ratio is better than half, take the sec
                if ratio >= 0.5 {
                    let char_a = ":";
                    let char_b = ".";
                    let num_a = ((ratio - 0.5)  * 2f64 * out_len as f64).floor() as usize;
                    let num_b = if num_a > out_len {0} else {out_len - num_a};

                    out.push_str(format!("{}{}",
                                         char_a.repeat(num_a),
                                         char_b.repeat(num_b)).as_str());
                } else {
                    let char_a = ".";
                    let char_b = " ";
                    let num_a = (ratio  * 2f64 * out_len as f64).floor() as usize;
                    let num_b = out_len - num_a;
                    out.push_str(format!("{}{}",
                                         char_a.repeat(num_a),
                                         char_b.repeat(num_b)).as_str());
                }
            }
            BarStyle::TwoChars(a, b) => {
                // Build the number of 'on' and 'off' characters based on ratio
                let num_a = (ratio * out_len as f64).floor() as usize;
                let num_b = if num_a > out_len {0} else {out_len - num_a};

                out.push_str(format!("{}{}",
                                     a.to_string().repeat(num_a),
                                     b.to_string().repeat(num_b)).as_str());

            }
            BarStyle::SingleChar(c) => {
                // Build the number of 'on' and 'off' characters based on ratio
                let num_a = (ratio * out_len as f64).floor() as usize;
                let num_b = out_len - num_a;

                out.push_str(format!("{}{}",
                                     c.to_string().repeat(num_a),
                                     " ".repeat(num_b)).as_str());
            }
        }
        out
    }
}


// Frames

pub enum JointType {
    // T Joints
    TUp, TDown, TLeft, TRight,

    Cross
}

pub enum FrameType {
    Single, Double
}

impl FrameType {
    /// Horizontal frame element
    pub fn hor(&self) -> char {
        match self {
            FrameType::Single => '─',
            FrameType::Double => '━',
        }
    }

    pub fn ver(&self) -> char {
        match self {
            FrameType::Single => '│',
            FrameType::Double => '┃',
        }
    }

    pub fn top_left(&self) -> char {
        match self {
            FrameType::Single => '┌',
            FrameType::Double => '╔',
        }
    }

    pub fn top_right(&self) -> char {
        let x = " ";

        match self {
            FrameType::Single => '┐',
            FrameType::Double => '╗',
        }
    }

    pub fn bottom_left(&self) -> char {
        match self {
            FrameType::Single => '└',
            FrameType::Double => '╚',
        }
    }

    pub fn bottom_right(&self) -> char {
        match self {
            FrameType::Single => '┘',
            FrameType::Double => '╝',
        }
    }

    pub fn joint(&self, j: JointType) -> char {
        match self {
            FrameType::Single => {
                let x = "";
                match j {
                    JointType::TUp => '┴',
                    JointType::TDown => '┬',
                    JointType::TLeft => '┤',
                    JointType::TRight => '├',
                    JointType::Cross => '┼',
                }
            }
            FrameType::Double => {
                match j {
                    JointType::TUp => '╩',
                    JointType::TDown => '╦',
                    JointType::TLeft => '╣',
                    JointType::TRight => '╠',
                    JointType::Cross => '╬',
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::combat::{ Actor};
    use super::*;



    #[test]
    fn test_number_shortening() {
        let numbers = vec![14324343, -123123123, 534_543, 425_431_242, 2342342334, 1];

        for n in numbers {
            let res = n.format_line(5, TextFormatting::Plain);
            print!("{}\n", res);
            assert_eq!(res.len(), 5);
        }
    }

    #[test]
    fn test_bars() {

        let styles =
            vec![BarStyle::DoubleLines, BarStyle::TwoChars('█', '▒'), BarStyle::SingleChar('~')];

        for i in 0..4 {
            let val =100-20*i;
            print!("val:{val}:\n");
            for style in styles.iter() {
                print!("{}\n", style.render_bar(16, val, 100))
            }

        }


    }


    #[test]
    fn test_wordwrap() {

        let words = TextFormatting::Console.to_words("Mary had a super awesome lamb full of funny moments".to_string(), "test", None);

        for line in words.display(10, 4, TextFormatting::Console) {
            println!("{}", line);
        }


    }
}