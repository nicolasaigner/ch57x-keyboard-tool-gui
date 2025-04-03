use eframe::{egui, App as EframeApp};
use std::{fs, process::{Command, Stdio}};

pub struct GuiApp {
    yaml_path: String,
    output_log: String,
}

impl Default for GuiApp {
    fn default() -> Self {
        Self {
            yaml_path: String::new(),
            output_log: String::new(),
        }
    }
}

impl EframeApp for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Uploader CH57x YAML");

            ui.horizontal(|ui| {
                ui.label("Arquivo YAML:");
                ui.text_edit_singleline(&mut self.yaml_path);
                if ui.button("Selecionar...").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("YAML", &["yaml"]).pick_file() {
                        self.yaml_path = path.display().to_string();
                    }
                }
            });

            if ui.button("Validar YAML").clicked() {
                self.output_log = run_tool("validate", &self.yaml_path);
            }

            if ui.button("Upload YAML").clicked() {
                self.output_log = run_tool("upload", &self.yaml_path);
            }

            ui.separator();
            ui.label("Log:");
            ui.text_edit_multiline(&mut self.output_log);
        });
    }
}

fn run_tool(command: &str, yaml_path: &str) -> String {
    match fs::read_to_string(yaml_path) {
        Ok(yaml_content) => {
            let mut child = match Command::new("ch57x-keyboard-tool")
                .arg(command)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => return format!("Erro ao executar o binário: {}", e),
            };

            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                if let Err(e) = stdin.write_all(yaml_content.as_bytes()) {
                    return format!("Erro ao escrever no stdin: {}", e);
                }
            }

            let output = match child.wait_with_output() {
                Ok(o) => o,
                Err(e) => return format!("Erro ao obter saída: {}", e),
            };

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            format!("stdout:\n{}\n\nstderr:\n{}", stdout, stderr)
        }
        Err(e) => format!("Erro ao ler YAML: {}", e),
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "CH57x Keyboard GUI",
        options,
        Box::new(|_cc| Ok(Box::new(GuiApp::default()))),
    )
}

