use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("embedded_tracks.rs");
    let mut f = fs::File::create(&dest_path).unwrap();

    let mp3_dir = Path::new("samples/mp3");
    println!("cargo:rerun-if-changed=samples/mp3");

    let mut tracks = Vec::new();

    if let Ok(entries) = fs::read_dir(mp3_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.to_string_lossy().eq_ignore_ascii_case("mp3") {
                        if let Some(name) = path.file_name() {
                            let name_str = name.to_string_lossy().into_owned();
                            if let Ok(abs_path) = fs::canonicalize(&path) {
                                tracks.push((name_str, abs_path.to_string_lossy().into_owned()));
                            }
                        }
                    }
                }
            }
        }
    }

    tracks.sort_by(|a, b| a.0.cmp(&b.0));

    writeln!(f, "pub static EMBEDDED_TRACKS: &[(&str, &[u8])] = &[").unwrap();
    for (name, abs_path) in &tracks {
        writeln!(f, "    ({:?}, include_bytes!(r#\"{}\"#)),", name, abs_path).unwrap();
    }
    writeln!(f, "];").unwrap();
}
