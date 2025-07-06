//! Futhorc created by Harys Dalvi (<https://www.harysdalvi.com/futhorc/>)

use std::collections::HashMap;

/// # Javascript Package
///
/// In order to run the javascript pkg:
///
/// ```sh
/// cargo install wasm-pack
/// make js
/// ```
///
/// Then copy the pkg folder to your web browser's site folder. For example with Apache on Debian:
///
/// ```sh
/// sudo mkdir --parent /var/www/html/pkg
/// sudo cp -r pkg /var/www/html
/// ```
///
/// Or if you installed the package via npm (does not exist at the moment):
///
/// ```sh
/// sudo mkdir --parent /var/www/html/pkg
/// sudo cp ~/node_modules/hnefatafl-copenhagen/* /var/www/html/pkg
/// ```
///
/// Then load the javascript on a webpage:
///
/// ```sh
/// cat << EOF > /var/www/html/index.html
/// <!DOCTYPE html>
/// <html>
/// <head>
///     <title>Copenhagen Hnefatafl</title>
/// </head>
/// <body>
///     <h1>Copenhagen Hnefatafl</h1>
///     <script type="module">
///         import init, { Ipa, ipa_to_runes_js } from '../pkg/hnefatafl_copenhagen.js';
///
///         init().then(() => {
///             const word = "know";
///             console.log(word);
///
///             const ipa = new Ipa();
///             const ipa_word = ipa.translate_js(word)
///             console.log(ipa_word);
///
///             const rune_word = ipa_to_runes_js(ipa_word);
///             console.log(rune_word);
///         });
///     </script>
/// </body>
/// </html>
/// EOF
/// ```
#[must_use]
pub fn words_to_runes(words: String) -> String {
    let ipa = Ipa::default();

    let ipa_words = ipa.translate(words);
    ipa_to_runes(&ipa_words)
}

#[cfg(feature = "js")]
use wasm_bindgen::prelude::wasm_bindgen;

#[must_use]
pub fn ipa_to_runes(ipa_words: &str) -> String {
    let mut runes = translate_to_runic_2(ipa_words);
    runes = translate_to_runic(&runes);
    remove_stress_markers(&runes)
}

#[cfg(feature = "js")]
#[wasm_bindgen]
#[must_use]
pub fn ipa_to_runes_js(ipa_words: &str) -> String {
    ipa_to_runes(ipa_words)
}

#[cfg(not(feature = "js"))]
#[derive(Clone, Debug)]
pub struct Ipa {
    pub english_to_ipa: HashMap<String, String>,
}

#[cfg(feature = "js")]
#[wasm_bindgen]
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, Debug)]
pub struct Ipa {
    #[wasm_bindgen(skip)]
    pub english_to_ipa: HashMap<String, String>,
}

#[cfg(feature = "js")]
#[wasm_bindgen]
impl Ipa {
    #[must_use]
    #[wasm_bindgen(constructor)]
    pub fn new() -> Ipa {
        Ipa::default()
    }

    #[must_use]
    pub fn translate_js(&self, words: String) -> String {
        self.translate(words)
    }
}

impl Ipa {
    // 1.  X Apostrophes and punctuation are used just like in standard English.
    // 2.    /i/ at the end of a word or before an apostrophe is simply ᛁ. So you
    //       have we'll/ᚹᛁ'ᛚ, will/ᚹᛁᛚ, wheel/ᚹᛁᛁᛚ. This applies to morphemes too:
    //       you have any/ᛖᚾᛁ, anything/ᛖᚾᛁᚦᛁᛝ. Also note that ᛁᛁ may represent /iɪ/ as in being/ᛒᛁᛁᛝ.
    // 3.  X If there is ambiguity between /f/ and /v/, use ᚠᚠ for /f/ and ᚠ for /v/.
    //       So you have live/ᛚᛁᚠ, leave/ᛚᛁᛁᚠ, leaf/ᛚᛁᛁᚠᚠ, lives/ᛚᛁᚠᛋ, leaves/ᛚᛁᛁᚠᛋ.
    //       Note that rules apply in the order they are listed here in case of a conflict.
    // 4.  X There is similar ambiguity clarification as above for /s/ (ᛋᛋ) and /z/
    //       (ᛋ), So you have ones/ᚹᚢᚾᛋ, once/ᚹᚢᚾᛋᛋ
    // 5.  X "No" is spelled ᚾᚩ and "know" is spelled ᚾᚩᚹ.
    // 6.    Words which use "tr" for /tʃɹ/ in standard English are spelled with ᛏᚱ,
    //       not ᚳᚻᚱ. Similar for "dr"/ᛞᚱ and /dʒɹ/; "x"/ᛉ and /ks/. So you have
    //       truck/ᛏᚱᚢᚳ, draw/ᛞᚱᛟ, tax/ᛏᚫᛉ, racks/ᚱᚫᚳᛋ.
    // 7.    Word-final /ə/ is written ᚪ. So you have comma/ᚳᛟᛗᚪ (not ᚳᛟᛗᚢ),
    //       vanilla/ᚠᚢᚾᛁᛚᚪ. Exception: the/ᚦᛖ.
    // 8.    Syllabic consonants are spelled with ᚢ before the consonant. So you have bottle/ᛒᛟᛏᚢᛚ.
    // 9.    ᛋ and ᛏ are optionally written together as the ligature ᛥ as in stone/ᛥᚩᚾ.
    //       Likewise for ᚳᚹ becoming ᛢ.
    // 10. X The name of this alphabet is written ᚠᚢᚦᚩᚱᚳ, but pronounced /fuθork/
    //       like "FOO-thork" as if it were spelled ᚠᚣᚦᚩᚱᚳ.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn translate(&self, mut words: String) -> String {
        words.make_ascii_lowercase();
        let mut ipa_words = Vec::new();

        let whitespaces = parse_whitespace(&words);

        for word in words.split_whitespace() {
            let mut word = word.to_string();

            let mut ch = ' ';
            if word.ends_with('.') || word.ends_with('!') || word.ends_with('?') {
                let mut chars = word.chars();
                ch = chars.next_back().unwrap();
                word = chars.as_str().to_string();
            }

            let mut ipa_word = self.english_to_ipa[&word].clone();

            if word.ends_with("'s") {
                let mut chars = ipa_word.chars();
                let c = chars.next_back().unwrap();
                ipa_word = chars.as_str().to_string();
                ipa_word.push('\'');
                ipa_word.push(c);
            } else if word.ends_with("'ll") {
                if ipa_word.ends_with("ʌl") {
                    let mut chars = ipa_word.chars();
                    chars.next_back().unwrap();
                    chars.next_back().unwrap();
                    ipa_word = chars.as_str().to_string();
                    ipa_word.push_str("'ʌl");
                } else {
                    let mut chars = ipa_word.chars();
                    let c = chars.next_back().unwrap();
                    ipa_word = chars.as_str().to_string();
                    ipa_word.push('\'');
                    ipa_word.push(c);
                }
            }

            if ch != ' ' {
                ipa_word.push(ch);
            }

            ipa_words.push(ipa_word);
        }

        let mut translated = String::new();
        translated.push_str(&whitespaces[0]);

        for (i, ipa_word) in ipa_words.iter().enumerate() {
            translated.push_str(ipa_word);
            translated.push_str(&whitespaces[i + 1]);
        }

        translated
    }
}

impl Default for Ipa {
    fn default() -> Self {
        let ipa = include_str!("../CMU.in.IPA.txt");

        let mut english_to_ipa = HashMap::new();
        for line in ipa.lines() {
            let words: Vec<_> = line.split_ascii_whitespace().collect();
            let mut words_0 = words[0].chars();
            words_0.next_back();
            let words_0 = words_0.as_str();

            if words[0] == "XXXXX" {
                continue;
            }

            english_to_ipa.insert(words_0.to_string(), words[1].to_string());
        }

        english_to_ipa.insert("know".to_string(), "noʊw".to_string());
        english_to_ipa.insert("futhorc".to_string(), "vʌθɑɹk".to_string());

        Self { english_to_ipa }
    }
}

fn parse_whitespace(words: &str) -> Vec<String> {
    let mut space = String::new();
    let mut spaces = Vec::new();
    let mut on_space = true;

    for ch in words.chars() {
        if on_space {
            if ch.is_whitespace() {
                space.push(ch);
            } else {
                on_space = false;
                spaces.push(space.clone());
                space.clear();
            }
        } else if ch.is_whitespace() {
            on_space = true;
            space.push(ch);
        }
    }

    if on_space {
        spaces.push(space);
    } else {
        spaces.push(String::new());
    }

    spaces
}

fn remove_stress_markers(string: &str) -> String {
    let mut output = String::new();

    for ch in string.chars() {
        match ch {
            'ˈ' | 'ˌ' => {}
            ch => output.push(ch),
        }
    }

    output
}

fn translate_to_runic(string: &str) -> String {
    let mut output = String::new();

    for char in string.chars() {
        let runes = match char {
            ' ' => "᛫",
            'ɑ' => "ᚪ",             // f_a_r
            'ɔ' => "ᛟ",             // h_o_t
            'æ' => "ᚫ",             // h_a_t
            'ɛ' => "ᛖ",             // s_e_nd
            'ɪ' => "ᛁ",             // s_i_t
            'i' => "ᛁᛁ",            // s_ee_d
            'ʊ' | 'u' => "ᚣ",       // b_oo_k, f_oo_d
            'ə' | 'ʌ' | 'ɜ' => "ᚢ", // _a_bout, f_u_n, t_u_rn
            'p' | 'P' => "ᛈ",       // _p_ot
            'b' => "ᛒ",             // _b_oy
            't' | 'T' => "ᛏ",       // _t_ime
            'd' | 'D' => "ᛞ",       // _d_og
            'k' | 'K' => "ᚳ",       // _k_ite
            'g' => "ᚷ",             // _g_ame
            'f' | 'F' => "ᚠᚠ",      // _f_ear
            'v' => "ᚠ",             // _v_ine
            'θ' | 'ð' => "ᚦ",       // _th_ing, _th_is
            's' => "ᛋᛋ",            // _s_ee, lot_s_
            'z' => "ᛋ",             // _z_ebra, song_s_
            'ʃ' | 'ʒ' => "ᛋᚻ",      // _sh_are, mea_s_ure
            'h' => "ᚻ",             // _h_ole
            'm' | 'M' => "ᛗ",       // _m_outh
            'n' | 'N' => "ᚾ",       // _n_ow
            'ŋ' => "ᛝ",             // ri_ng_
            'j' => "ᛄ",             // _y_ou
            'w' => "ᚹ",             // _w_ind
            'ɹ' | 'R' => "ᚱ",       // _r_ain
            'l' | 'L' => "ᛚ",       // _l_ine
            'ʤ' => "ᚷᚻ",            // _j_og
            'ʧ' => "ᚳᚻ",            // _ch_eese
            // Added.
            'ɚ' => "ᚢᚱ",
            c => &c.to_string(),
        };

        output.push_str(runes);
    }

    output
}

fn translate_to_runic_2(string: &str) -> String {
    let vec: Vec<_> = string.chars().collect();
    let mut string = String::new();

    let mut skip = false;

    for two in vec.windows(2) {
        if skip {
            skip = false;
            continue;
        }

        let output = match two {
            // 2nd 2nd added.
            ['e', 'ɪ' | 'j'] => {
                skip = true;
                "ᛠ" // st_ay_
            }
            ['a', 'ɪ'] => {
                skip = true;
                "ᛡ" // l_ie_
            }
            // Added
            ['a', 'j'] => {
                skip = true;
                "ᛁ" // l_i_ve
            }
            // 2nd 2nd added.
            ['a', 'ʊ' | 'w'] => {
                skip = true;
                "ᚪᚹ" // f_ou_nd
            }
            // 2nd 2nd added.
            ['o', 'ʊ' | 'w'] => {
                skip = true;
                "ᚩ" // n_o_
            }
            ['ɔ', 'ɪ'] => {
                skip = true;
                "ᚩᛁ" // p_oi_nt
            }
            ['t', 'ʃ'] => {
                skip = true;
                "ᚳᚻ" // _ch_eese
            }
            ['d', 'ʒ'] => {
                skip = true;
                "ᚷᚻ" // _j_og
            }
            ['ŋ', 'g'] => {
                skip = true;
                "ᛝ" // ri_ng_
            }
            // Added.
            ['s', 'S'] => {
                skip = true;
                "ᛋᛋᛋ" // ri_ng_
            }
            [one] | [one, _] => &one.to_string(),
            [] | [..] => "",
        };

        string.push_str(output);
    }

    if !skip {
        string.push(*vec.last().unwrap());
    }

    string
}

#[cfg(test)]
mod tests {
    use crate::futhorc::{Ipa, ipa_to_runes};

    #[test]
    fn know_no_etc() {
        let ipa = Ipa::default();

        let mut words = String::new();
        words.push_str("no");
        let mut output = ipa.translate(words);
        output = ipa_to_runes(&output);
        assert_eq!(output, "ᚾᚩ");

        let mut words = String::new();
        words.push_str("know");
        let mut output = ipa.translate(words);
        output = ipa_to_runes(&output);
        assert_eq!(output, "ᚾᚩᚹ");
    }

    #[test]
    fn newlines() {
        let ipa = Ipa::default();

        let mut words = String::new();
        words.push_str("apple banana\ncarrot\n\n");
        let mut output = ipa.translate(words);
        output = ipa_to_runes(&output);
        assert_eq!(output, "ᚫᛈᚢᛚ᛫ᛒᚢᚾᚫᚾᚢ\nᚳᚫᚱᚢᛏ\n\n");
    }

    #[test]
    fn apostrophes() {
        let ipa = Ipa::default();

        /* fixme!
        let mut words = String::new();
        words.push_str("abram's");
        let mut output = ipa.translate(words);
        output = ipa_to_runes(&output);
        assert_eq!(output, "eᛄᛒᚱᚢᛗ'ᛋ");
        */

        let mut words = String::new();
        words.push_str("absolut's");
        let mut output = ipa.translate(words);
        output = ipa_to_runes(&output);
        assert_eq!(output, "ᚫᛒᛋᛋᚢᛚᚣᛏ'ᛋᛋ");

        let mut words = String::new();
        words.push_str("company'll");
        let mut output = ipa.translate(words);
        output = ipa_to_runes(&output);
        assert_eq!(output, "ᚳᚢᛗᛈᚢᚾᛁᛁ'ᚢᛚ");

        let mut words = String::new();
        words.push_str("he'll");
        let mut output = ipa.translate(words);
        output = ipa_to_runes(&output);
        assert_eq!(output, "ᚻᛁᛁ'ᛚ");
    }
}
