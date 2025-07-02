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
        'ɑ' => "ᚪ".to_string(),             // f_a_r
        'ɔ' => "ᛟ".to_string(),             // h_o_t
        'æ' => "ᚫ".to_string(),             // h_a_t
        'ɛ' => "ᛖ".to_string(),             // s_e_nd
        'ɪ' => "ᛁ".to_string(),             // s_i_t
        'i' => "ᛁᛁ".to_string(),            // s_ee_d
        'ʊ' | 'u' => "ᚣ".to_string(),       // b_oo_k, f_oo_d
        'ə' | 'ʌ' | 'ɜ' => "ᚢ".to_string(), // _a_bout, f_u_n, t_u_rn
        'p' => "ᛈ".to_string(),             // _p_ot
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
            [one] | [one, _] => &one.to_string(),
            [] | [..] => "",
        };

        string.push_str(output);
    }

    string
}
