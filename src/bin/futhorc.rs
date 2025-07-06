//! Futhorc created by Harys Dalvi (<https://www.harysdalvi.com/futhorc/>)

use std::io;

use hnefatafl_copenhagen::futhorc::{Ipa, ipa_to_runes};

fn main() -> Result<(), anyhow::Error>{
    let ipa = Ipa::default();

    loop {
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;

        let ipa_words = ipa.translate(line);
        print!("{ipa_words}");
        print!("{}", ipa_to_runes(&ipa_words));
    }
}
