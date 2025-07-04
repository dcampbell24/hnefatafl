#![cfg(feature = "zip")]

use std::{
    fs::{self, File},
    path::PathBuf,
};

#[cfg(feature = "zip")]
use ripunzip::{NullProgressReporter, UnzipEngine, UnzipOptions};

fn main() -> Result<(), anyhow::Error> {
    let mut path = PathBuf::new();
    path = path.join("src").join("CMU.in.IPA.txt");

    if !fs::exists(path.as_path())? {
        let ipa = File::open("CMU-IPA.zip")?;
        let options = UnzipOptions {
            output_directory: None,
            password: None,
            single_threaded: false,
            filename_filter: None,
            progress_reporter: Box::new(NullProgressReporter),
        };

        UnzipEngine::for_file(ipa)?.unzip(options)?;
        fs::rename("CMU.in.IPA.txt", path.as_path())?;
    }

    Ok(())
}
