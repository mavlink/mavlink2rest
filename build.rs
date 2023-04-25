extern crate reqwest;
extern crate vergen;

use std::{fs::File, io, path::Path};

use vergen::{vergen, Config};

fn main() {
    // Generate the 'cargo:' key output
    vergen(Config::default()).expect("Something is wrong!");

    let artifacts_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/html/");

    std::fs::create_dir_all(&artifacts_dir).expect("failed to create a dir");

    for remote_file in [
        "https://unpkg.com/vue@3.0.5/dist/vue.global.js",
        "https://unpkg.com/highlight.js@10.6.0/styles/github.css",
        "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/10.6.0/highlight.min.js",
    ] {
        download_file(remote_file, &artifacts_dir);
    }
}

fn download_file(remote_file: &str, dir: &Path) {
    let mut resp = reqwest::blocking::get(remote_file)
        .unwrap_or_else(|_| panic!("Failed to download vue file: {}", remote_file));

    let filename = remote_file.split('/').last().unwrap();
    let file_path = dir.join(filename);
    let mut output_file = File::create(&file_path)
        .unwrap_or_else(|_| panic!("Failed to create artifact file: {:?}", file_path));

    io::copy(&mut resp, &mut output_file).expect("Failed to copy content.");
}
