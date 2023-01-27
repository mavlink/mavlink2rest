extern crate reqwest;
extern crate vergen;

use vergen::{vergen, Config};

fn main() {
    // Generate the 'cargo:' key output
    vergen(Config::default()).expect("Something is wrong!");

    for remote_file in [
        "https://unpkg.com/vue@3.0.5/dist/vue.global.js",
        "https://unpkg.com/highlight.js@10.6.0/styles/github.css",
        "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/10.6.0/highlight.min.js",
    ] {
        let mut resp = reqwest::blocking::get(remote_file)
            .expect(&format!("Failed to download vue file: {remote_file}"));

        let filename = remote_file.split('/').last().unwrap();
        let vue_file_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("src/html/{filename}"));
        let mut output_file = std::fs::File::create(&vue_file_path)
            .expect(&format!("Failed to create vue file: {vue_file_path:?}"));
        std::io::copy(&mut resp, &mut output_file).expect("Failed to copy content.");
    }
}
