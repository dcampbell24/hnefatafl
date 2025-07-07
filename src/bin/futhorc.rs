//! Futhorc created by Harys Dalvi (<https://www.harysdalvi.com/futhorc/>)

use std::io;

use hnefatafl_copenhagen::futhorc::EnglishToRunes;

fn main() -> Result<(), anyhow::Error> {
    let dictionary = EnglishToRunes::default();

    loop {
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;

        print!("{}", dictionary.translate(line));
    }
}
