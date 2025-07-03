#![cfg(feature = "zip")]

//! Futhorc created by Harys Dalvi (<https://www.harysdalvi.com/futhorc/>)
use std::{
    collections::HashMap,
    fs::{self, File},
    io,
    path::PathBuf,
};

#[cfg(feature = "zip")]
use ripunzip::{NullProgressReporter, UnzipEngine, UnzipOptions};

fn main() -> Result<(), anyhow::Error> {
    let words_hash = words_hash()?;
    println!("apple: {}", words_hash["apple"]);

    let mut line = String::new();

    loop {
        line.clear();
        io::stdin().read_line(&mut line)?;

        let output = translate(&line, &words_hash);
        println!("{output}");
    }
}

fn words_hash() -> Result<HashMap<String, String>, anyhow::Error> {
    let mut txt = PathBuf::new();
    txt.push("CMU.in.IPA.txt");

    if !fs::exists(&txt)? {
        let ipa = File::open("CMU-IPA.zip")?;
        let options = UnzipOptions {
            output_directory: None,
            password: None,
            single_threaded: false,
            filename_filter: None,
            progress_reporter: Box::new(NullProgressReporter),
        };
        UnzipEngine::for_file(ipa)?.unzip(options)?;
    }

    let mut words_hash = HashMap::new();
    let txt = fs::read_to_string(txt)?;
    for line in txt.lines() {
        let words: Vec<_> = line.split_ascii_whitespace().collect();
        let mut words_0 = words[0].chars();
        words_0.next_back();
        let words_0 = words_0.as_str();

        if words[0] == "XXXXX" {
            continue;
        }

        words_hash.insert(words_0.to_string(), words[1].to_string());
    }

    Ok(words_hash)
}

#[must_use]
fn translate(line: &str, words: &HashMap<String, String>) -> String {
    // let mut line = translate_no(&line);
    let mut ipa_words = Vec::new();

    for word in line.split_ascii_whitespace() {
        ipa_words.push(words[word].clone());
    }
    let mut ipa_words = ipa_words.join(" ");
    ipa_words.push('\n');

    let mut line = translate_to_runic_2(&ipa_words);
    line = translate_to_runic(&line);

    line
}

fn _translate_no(string: &str) -> String {
    let mut output = String::new();
    let mut letter = 0;
    let mut no = false;
    let mut space = false;

    'outer: for (i, ch) in string.chars().enumerate() {
        match letter {
            0 => {
                if (i == 0 || space) && ch == 'n' {
                    letter = 1;
                    no = true;
                    continue 'outer;
                }
            }
            1 => {
                if no && ch == 'n' {
                    letter = 2;
                    continue 'outer;
                } else if no {
                    output.push('k');
                    letter = 0;
                    no = false;
                }
            }
            2 => {
                if no && ch == 'ʊ' {
                    letter = 3;
                    continue 'outer;
                } else if no {
                    output.push_str("no");
                    letter = 0;
                    no = false;
                }
            }
            3 => {
                if no && (ch == ' ' || ch == '.' || ch == '!' || ch == '?') {
                    output.push_str("ᚾᚩ");
                } else {
                    output.push_str("noʊ");
                    letter = 0;
                    no = false;
                }
            }
            _ => unreachable!(),
        }

        space = ch == ' ';

        output.push(ch);
    }

    if no && letter == 3 {
        output.push_str("ᚾᚩ");
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
            'f' | 'v' => "ᚠ",       // _f_ear, _v_ine
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
            ['a', 'ʊ'] => {
                skip = true;
                "ᚪᚹ" // f_ou_nd
            }
            ['o', 'ʊ'] => {
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
    use crate::{translate, words_hash};

    #[test]
    fn know_no_etc() -> Result<(), anyhow::Error> {
        let words_hash = words_hash()?;
        let mut line = String::new();

        // no and know both are phonetically the same.
        line.push_str("no\n");
        let output = translate(&line, &words_hash);
        assert_eq!(output, "ᚾᚩ");
        // It would have to be "noʊw" to get the correct translation of ᚾᚩᚹ for know.

        Ok(())
    }
}
