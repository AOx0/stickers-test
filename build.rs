use std::process::Command;

fn main() {
    if let Err(err) = Command::new("tailwindcss")
        .args(["-i", "./static/input.css", "-o", "./static/style.css"])
        .spawn() {
            println!("cargo:warning=Failed to run tailwindcss: {}", err);
        }
}