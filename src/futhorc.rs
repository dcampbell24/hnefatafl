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
    ipa.translate(words)
}

#[cfg(feature = "js")]
use wasm_bindgen::prelude::wasm_bindgen;

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
    // 1.  Apostrophes and punctuation are used just like in standard English.
    // 2.  /i/ at the end of a word or before an apostrophe is simply ᛁ. So you
    //     have we'll/ᚹᛁ'ᛚ, will/ᚹᛁᛚ, wheel/ᚹᛁᛁᛚ. This applies to morphemes too:
    //     you have any/ᛖᚾᛁ, anything/ᛖᚾᛁᚦᛁᛝ. Also note that ᛁᛁ may represent /iɪ/ as in being/ᛒᛁᛁᛝ.
    // 3.  If there is ambiguity between /f/ and /v/, use ᚠᚠ for /f/ and ᚠ for /v/.
    //     So you have live/ᛚᛁᚠ, leave/ᛚᛁᛁᚠ, leaf/ᛚᛁᛁᚠᚠ, lives/ᛚᛁᚠᛋ, leaves/ᛚᛁᛁᚠᛋ.
    //     Note that rules apply in the order they are listed here in case of a conflict.
    // 4.  There is similar ambiguity clarification as above for /s/ (ᛋᛋ) and /z/
    //     (ᛋ), So you have ones/ᚹᚢᚾᛋ, once/ᚹᚢᚾᛋᛋ
    // 5.  "No" is spelled ᚾᚩ and "know" is spelled ᚾᚩᚹ.
    // 6.  Words which use "tr" for /tʃɹ/ in standard English are spelled with ᛏᚱ,
    //     not ᚳᚻᚱ. Similar for "dr"/ᛞᚱ and /dʒɹ/; "x"/ᛉ and /ks/. So you have
    //     truck/ᛏᚱᚢᚳ, draw/ᛞᚱᛟ, tax/ᛏᚫᛉ, racks/ᚱᚫᚳᛋ.
    // 7.  Word-final /ə/ (Added or ʌ or ɜ) is written ᚪ. So you have comma/ᚳᛟᛗᚪ (not ᚳᛟᛗᚢ),
    //     vanilla/ᚠᚢᚾᛁᛚᚪ. Exception: the/ᚦᛖ.
    // 8.  Syllabic consonants are spelled with ᚢ before the consonant. So you have bottle/ᛒᛟᛏᚢᛚ.
    // 9.  ᛋ and ᛏ are optionally written together as the ligature ᛥ as in stone/ᛥᚩᚾ.
    //     Likewise for ᚳᚹ becoming ᛢ (Optional, not doing...).
    // 10. The name of this alphabet is written ᚠᚢᚦᚩᚱᚳ, but pronounced /fuθork/
    //     like "FOO-thork" as if it were spelled ᚠᚣᚦᚩᚱᚳ.
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

            if let Some(ipa_word) = self.english_to_ipa.get(&word) {
                let mut ipa_word = remove_stress_markers(ipa_word);

                if ipa_word.ends_with('ə') || ipa_word.ends_with('ʌ') || ipa_word.ends_with('ɜ')
                {
                    let mut chars = ipa_word.chars();
                    chars.next_back().unwrap();
                    ipa_word = chars.as_str().to_string();
                    ipa_word.push('ɑ');
                }

                if ipa_word.ends_with('i') {
                    let mut chars = ipa_word.chars();
                    chars.next_back().unwrap();
                    ipa_word = chars.as_str().to_string();
                    ipa_word.push('I');
                }

                if word.ends_with("'s") {
                    let mut chars = ipa_word.chars();
                    let c = chars.next_back().unwrap();

                    if Some('i') == chars.clone().last() {
                        chars.next_back().unwrap();
                        ipa_word = chars.as_str().to_string();
                        ipa_word.push('I');
                    } else {
                        ipa_word = chars.as_str().to_string();
                    }

                    ipa_word.push('\'');
                    ipa_word.push(c);
                } else if word.ends_with("'ll") {
                    let mut chars = ipa_word.chars();

                    if ipa_word.ends_with("ʌl") {
                        chars.next_back().unwrap();
                        chars.next_back().unwrap();

                        if Some('i') == chars.clone().last() {
                            chars.next_back().unwrap();
                            ipa_word = chars.as_str().to_string();
                            ipa_word.push_str("I'ʌl");
                        } else {
                            ipa_word = chars.as_str().to_string();
                            ipa_word.push_str("'ʌl");
                        }
                    } else {
                        let c = chars.next_back().unwrap();

                        if Some('i') == chars.clone().last() {
                            chars.next_back().unwrap();
                            ipa_word = chars.as_str().to_string();
                            ipa_word.push_str("I'");
                            ipa_word.push(c);
                        } else {
                            ipa_word = chars.as_str().to_string();
                            ipa_word.push('\'');
                            ipa_word.push(c);
                        }
                    }
                }

                if ch != ' ' {
                    ipa_word.push(ch);
                }

                ipa_words.push((ipa_word, true));
            } else {
                ipa_words.push((word, false));
            }
        }

        let mut translated = String::new();
        translated.push_str(&whitespaces[0]);

        for (i, (word, translated_to_ipa)) in ipa_words.iter().enumerate() {
            if *translated_to_ipa {
                let rune_word = translate_to_runic(&translate_to_runic_2(word));

                #[cfg(feature = "debug")]
                println!("{word} {rune_word}");

                translated.push_str(&rune_word);
            } else {
                translated.push_str(word);
            }

            translated.push_str(&translate_to_runic(&whitespaces[i + 1]));
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
        english_to_ipa.insert("the".to_string(), "ðɛ".to_string());

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
            'ɪ' | 'I' => "ᛁ",       // s_i_t, w_e_'ll, an_y_
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
            // Added below this line.
            'ʤ' => "ᚷᚻ", // _j_og
            'ʧ' => "ᚳᚻ", // _ch_eese
            'ɚ' => "ᚢᚱ", // runn_er_
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
    use crate::futhorc::Ipa;

    #[test]
    fn know_no_etc() {
        let ipa = Ipa::default();

        let mut words = String::new();
        words.push_str("no");
        let output = ipa.translate(words);
        assert_eq!(output, "ᚾᚩ");

        let mut words = String::new();
        words.push_str("know");
        let output = ipa.translate(words);
        assert_eq!(output, "ᚾᚩᚹ");
    }

    #[test]
    fn newlines() {
        let ipa = Ipa::default();

        let mut words = String::new();
        words.push_str("apple banana\ncarrot\n\n");
        let output = ipa.translate(words);
        assert_eq!(output, "ᚫᛈᚢᛚ᛫ᛒᚢᚾᚫᚾᚪ\nᚳᚫᚱᚢᛏ\n\n");
    }

    #[test]
    fn apostrophes() {
        let ipa = Ipa::default();

        let mut words = String::new();
        words.push_str("abram's");
        let output = ipa.translate(words);
        assert_eq!(output, "ᛠᛒᚱᚢᛗ'ᛋ");

        let mut words = String::new();
        words.push_str("absolut's");
        let output = ipa.translate(words);
        assert_eq!(output, "ᚫᛒᛋᛋᚢᛚᚣᛏ'ᛋᛋ");

        let mut words = String::new();
        words.push_str("company'll");
        let output = ipa.translate(words);
        assert_eq!(output, "ᚳᚢᛗᛈᚢᚾᛁ'ᚢᛚ");

        let mut words = String::new();
        words.push_str("he'll");
        let output = ipa.translate(words);
        assert_eq!(output, "ᚻᛁ'ᛚ");
    }

    #[test]
    fn final_ə() {
        let ipa = Ipa::default();

        let mut words = String::new();
        words.push_str("ababa");
        let output = ipa.translate(words);
        assert_eq!(output, "ᚢᛒᚪᛒᚪ");

        let mut words = String::new();
        words.push_str("the");
        let output = ipa.translate(words);
        assert_eq!(output, "ᚦᛖ");
    }

    #[test]
    fn syllabic_consonants() {
        let ipa = Ipa::default();

        let mut words = String::new();
        words.push_str("bottle");
        let output = ipa.translate(words);
        assert_eq!(output, "ᛒᚪᛏᚢᛚ"); // Not ᛒᛟᛏᚢᛚ via CMU
    }

    #[test]
    fn ends_with_i() {
        let ipa = Ipa::default();

        let mut words = String::new();
        words.push_str("wheel");
        let output = ipa.translate(words);
        assert_eq!(output, "ᚹᛁᛁᛚ");

        let mut words = String::new();
        words.push_str("any");
        let output = ipa.translate(words);
        assert_eq!(output, "ᛖᚾᛁ");
    }

    #[test]
    fn i_apostrophe() {
        let ipa = Ipa::default();

        let mut words = String::new();
        words.push_str("renaissance's");
        let output = ipa.translate(words);
        assert_eq!(output, "ᚱᛖᚾᚢᛋᛋᚪᚾᛋᛋᛁ'ᛋ");

        let mut words = String::new();
        words.push_str("we'll");
        let output = ipa.translate(words);
        assert_eq!(output, "ᚹᛁ'ᛚ");
    }
}
