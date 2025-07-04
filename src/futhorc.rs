//! Futhorc created by Harys Dalvi (<https://www.harysdalvi.com/futhorc/>)

// 1.    Apostrophes and punctuation are used just like in standard English.
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
// 10.   The name of this alphabet is written ᚠᚢᚦᚩᚱᚳ, but pronounced /fuθork/
//       like "FOO-thork" as if it were spelled ᚠᚣᚦᚩᚱᚳ.

use std::collections::HashMap;

#[cfg(feature = "js")]
use wasm_bindgen::prelude::wasm_bindgen;

#[must_use]
pub fn ipa_to_runes(ipa_words: &str) -> String {
    let mut ipa_words = remove_stress_markers(ipa_words);
    ipa_words.push('\n');

    let runes = translate_to_runic_2(&ipa_words);
    translate_to_runic(&runes)
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
    english_to_ipa: HashMap<String, String>,
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
    pub fn translate_js(&self, line: String) -> String {
        self.translate(line)
    }
}

impl Ipa {
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn translate(&self, mut line: String) -> String {
        line.make_ascii_lowercase();
        let mut ipa_words = Vec::new();

        for word in line.split_whitespace() {
            let mut word = word.to_string();
            let mut ch = ' ';
            if word.ends_with('.') || word.ends_with('!') || word.ends_with('?') {
                let mut chars = word.chars();
                ch = chars.next_back().unwrap();
                word = chars.as_str().to_string();
            }

            let mut ipa_word = self.english_to_ipa[&word].clone();

            if ch != ' ' {
                ipa_word.push(ch);
            }

            ipa_words.push(ipa_word);
        }

        ipa_words.join(" ")
    }
}

impl Default for Ipa {
    #[allow(clippy::missing_panics_doc)]
    fn default() -> Self {
        let ipa = include_bytes!("CMU.in.IPA.txt");
        let ipa = String::from_utf8(ipa.to_vec()).unwrap();

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

        Self { english_to_ipa }
    }
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
            'p' => "ᛈ",             // _p_ot
            'b' => "ᛒ",             // _b_oy
            't' => "ᛏ",             // _t_ime
            'd' => "ᛞ",             // _d_og
            'k' => "ᚳ",             // _k_ite
            'g' => "ᚷ",             // _g_ame
            'f' => "ᚠᚠ",            // _f_ear
            'v' => "ᚠ",             // _v_ine
            'θ' | 'ð' => "ᚦ",       // _th_ing, _th_is
            's' => "ᛋᛋ",            // _s_ee, lot_s_
            'z' => "ᛋ",             // _z_ebra, song_s_
            'ʃ' | 'ʒ' => "ᛋᚻ",      // _sh_are, mea_s_ure
            'h' => "ᚻ",             // _h_ole
            'm' => "ᛗ",             // _m_outh
            'n' => "ᚾ",             // _n_ow
            'ŋ' => "ᛝ",             // ri_ng_
            'j' => "ᛄ",             // _y_ou
            'w' => "ᚹ",             // _w_ind
            'ɹ' => "ᚱ",             // _r_ain
            'l' => "ᛚ",             // _l_ine
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
            ['e', 'ɪ'] => {
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
            ['a', 'ʊ'] => {
                skip = true;
                "ᚪᚹ" // f_ou_nd
            }
            // 2nd added.
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
            [one] | [one, _] => &one.to_string(),
            [] | [..] => "",
        };

        string.push_str(output);
    }

    string
}

#[cfg(test)]
mod tests {
    use crate::futhorc::{Ipa, ipa_to_runes};

    #[test]
    fn know_no_etc() {
        let ipa = Ipa::default();

        let mut line = String::new();
        line.push_str("no\n");
        let mut output = ipa.translate(line);
        output = ipa_to_runes(&output);
        assert_eq!(output, "ᚾᚩ");

        let mut line = String::new();
        line.push_str("know\n");
        let mut output = ipa.translate(line);
        output = ipa_to_runes(&output);
        assert_eq!(output, "ᚾᚩᚹ");
    }
}
