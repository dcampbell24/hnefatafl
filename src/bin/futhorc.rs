//! Futhorc created by Harys Dalvi (<https://www.harysdalvi.com/futhorc/>)
use std::io;

fn main() -> Result<(), anyhow::Error> {
    loop {
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;

        let output_1 = translate_to_runic_2(&line);

        let mut output_2 = String::new();
        for char in output_1.chars() {
            output_2.push_str(&translate_to_runic(char));
        }

        println!("{output_2}");
    }
}

fn translate_to_runic(c: char) -> String {
    match c {
        'ɑ' => 'ᚪ'.to_string(),             // f_a_r
        'ɔ' => 'ᛟ'.to_string(),             // h_o_t
        'æ' => 'ᚫ'.to_string(),             // h_a_t
        'ɛ' => 'ᛖ'.to_string(),             // s_e_nd
        'ɪ' => 'ᛁ'.to_string(),             // s_i_t
        'i' => "ᛁᛁ".to_string(),            // s_ee_d
        'ʊ' | 'u' => 'ᚣ'.to_string(),       // b_oo_k, f_oo_d
        'ə' | 'ʌ' | 'ɜ' => 'ᚢ'.to_string(), // _a_bout, f_u_n, t_u_rn
        'p' => 'ᛈ'.to_string(),             // _p_ot
        'b' => 'ᛒ'.to_string(),             // _b_oy
        't' => 'ᛏ'.to_string(),             // _t_ime
        'd' => 'ᛞ'.to_string(),             // _d_og
        'k' => 'ᚳ'.to_string(),             // _k_ite
        'g' => 'ᚷ'.to_string(),             // _g_ame
        'f' | 'v' => 'ᚠ'.to_string(),       // _f_ear, _v_ine
        'θ' | 'ð' => 'ᚦ'.to_string(),       // _th_ing, _th_is
        's' => "ᛋᛋ".to_string(),            // _s_ee, lot_s_
        'z' => 'ᛋ'.to_string(),             // _z_ebra, song_s_
        'ʃ' | 'ʒ' => "ᛋᚻ".to_string(),      // _sh_are, mea_s_ure
        'h' => 'ᚻ'.to_string(),             // _h_ole
        'm' => 'ᛗ'.to_string(),             // _m_outh
        'n' => 'ᚾ'.to_string(),             // _n_ow
        'ŋ' => 'ᛝ'.to_string(),             // ri_ng_
        'j' => 'ᛄ'.to_string(),             // _y_ou
        'w' => 'ᚹ'.to_string(),             // _w_ind
        'ɹ' => 'ᚱ'.to_string(),             // _r_ain
        'l' => 'ᛚ'.to_string(),             // _l_ine
        c => c.to_string(),
    }
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
