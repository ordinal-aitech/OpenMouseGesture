use std::fs;
use std::io::Write;
use std::process::Command;

fn main() {
    generate_license_file();
    tauri_build::build()
}

fn generate_license_file() {
    println!("cargo:rerun-if-changed=about.toml");
    println!("cargo:rerun-if-changed=about.hbs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    let temp_file = "license_temp.html";

    let status = Command::new("cargo")
        .args(&["about", "generate", "about.hbs", "-o", temp_file])
        .status()
        .expect("Failed to execute cargo about");

    if !status.success() {
        panic!("cargo about failed with exit code: {:?}", status.code());
    }

    let mut html = fs::read_to_string(temp_file).expect("Failed to read temporary license file");

    html = html.replace("<a href=", "<a target=\"_blank\" href=");

    let mut file = fs::File::create("license.html").expect("Failed to create license.html");
    file.write_all(html.as_bytes())
        .expect("Failed to write license.html");

    fs::remove_file(temp_file).expect("Failed to remove temporary license file");

    println!("Generated license.html with target=\"_blank\" attributes");
}
