//! Icelandic Runic created by Alexander R. (<https://www.omniglot.com/conscripts/icelandicrunic.htm>)

// This file is part of hnefatafl-copenhagen.
//
// hnefatafl-copenhagen is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// hnefatafl-copenhagen is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::io;

fn main() {
    loop {
        let mut input = String::new();
        let mut output = String::new();

        match io::stdin().read_line(&mut input) {
            Ok(_characters_read) => {
                for c in input.chars() {
                    output.push(translate_to_runic(c));
                }
            }
            Err(error) => println!("error: {error}"),
        }

        print!("{output}");
    }
}

fn translate_to_runic(c: char) -> char {
    match c {
        'A' | 'a' => 'ᛆ',
        'Á' | 'á' => 'ᚨ',
        'B' | 'b' => 'ᛒ',
        'D' | 'd' => 'ᛑ',
        'Ð' | 'ð' => 'ᚧ',
        'E' | 'e' => 'ᛂ',
        'É' | 'é' => 'ᛖ',
        'F' | 'f' => 'ᚠ',
        'G' | 'g' => 'ᚵ',
        'H' | 'h' => 'ᚼ',
        'I' | 'i' => 'ᛁ',
        'Í' | 'í' => 'ᛇ',
        'J' | 'j' => 'ᛃ',
        'K' | 'k' => 'ᚴ',
        'L' | 'l' => 'ᛚ',
        'M' | 'm' => 'ᛘ',
        'N' | 'n' => 'ᚿ',
        'O' | 'o' => 'ᚮ',
        'Ó' | 'ó' => 'ᛟ',
        'P' | 'p' => 'ᛔ',
        'R' | 'r' => 'ᚱ',
        'S' | 's' => 'ᛋ',
        'T' | 't' => 'ᛐ',
        'U' | 'u' => 'ᚢ',
        'Ú' | 'ú' => 'ᚤ',
        'V' | 'v' => 'ᚡ',
        'X' | 'x' => 'ᛪ',
        'Y' | 'y' => 'ᛣ',
        'Ý' | 'ý' => 'ᛨ',
        'Þ' | 'þ' => 'ᚦ',
        'Æ' | 'æ' => 'ᛅ',
        'Ö' | 'ö' => 'ᚯ',
        ch => ch,
    }
}
