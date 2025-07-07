//! Futhorc created by Harys Dalvi (<https://www.harysdalvi.com/futhorc/>)

use std::io;

use hnefatafl_copenhagen::futhorc::Ipa;

fn main() -> Result<(), anyhow::Error> {
    let ipa = Ipa::default();

    loop {
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;

        print!("{}", ipa.translate(line));
    }
}
