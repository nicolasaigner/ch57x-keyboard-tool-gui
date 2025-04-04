// gui/build.rs
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Caminho onde o executável final do GUI será colocado
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);

    // Caminho para o binário do CLI compilado
    let cli_bin = Path::new("../cli/target/release/ch57x-keyboard-tool.exe");

    // Copia o binário para a pasta do GUI (pode ser ajustado conforme seu uso)
    if cli_bin.exists() {
        let _ = fs::copy(cli_bin, out_path.join("ch57x-keyboard-tool.exe"));
    }
}
