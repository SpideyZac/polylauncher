use std::{env, fs, path::Path};

fn main() {
    let bin = env::var("CARGO_BIN_NAME").ok();
    if bin.as_deref() != Some("pl-cli") {
        tauri_build::build()
    }

    let out_dir = env::var("OUT_DIR").unwrap();

    let target_dir = Path::new(&out_dir).ancestors().nth(3).unwrap();

    let dest = target_dir.join("resources");

    fs::create_dir_all(&dest).unwrap();
    copy_dir_all("../resources", &dest);
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) {
    fs::create_dir_all(&dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let ty = entry.file_type().unwrap();
        let dst = dst.as_ref().join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst);
        } else {
            fs::copy(entry.path(), dst).unwrap();
        }
    }
}
