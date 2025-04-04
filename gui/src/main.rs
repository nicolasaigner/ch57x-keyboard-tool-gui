// Atualização para evitar terminal ao chamar o CLI no Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use chrono::Local;
use eframe::{egui, App as EframeApp};
use serde::{Deserialize, Serialize};
use std::{fs, process::{Command, Stdio}};

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Default)]
pub struct GuiApp {
    yaml_path: String,
    output_log: Vec<String>,
    mapping: Option<MappingFile>,
}

impl GuiApp {
    fn log(&mut self, msg: &str) {
        let now = Local::now().format("[%H:%M:%S]").to_string();
        self.output_log.push(format!("{} {}", now, msg));
    }
}

impl EframeApp for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading("CH57x Keyboard Viewer");

            ui.horizontal(|ui| {
                ui.label("Arquivo YAML:");
                ui.add_sized(egui::vec2(600.0, 30.0), egui::TextEdit::singleline(&mut self.yaml_path));
                if ui.add_sized(egui::vec2(150.0, 30.0), egui::Button::new("Selecionar...")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("YAML", &["yaml"]).pick_file() {
                        self.yaml_path = path.display().to_string();
                        match fs::read_to_string(&self.yaml_path) {
                            Ok(content) => match serde_yaml::from_str::<MappingFile>(&content) {
                                Ok(mapping) => {
                                    self.mapping = Some(mapping);
                                    self.log(&format!("Carregou o arquivo {}", self.yaml_path));
                                }
                                Err(e) => self.log(&format!("Erro ao carregar YAML: {}", e)),
                            },
                            Err(e) => self.log(&format!("Erro ao ler YAML: {}", e)),
                        }
                    }
                }
            });
        });

        egui::TopBottomPanel::bottom("footer_panel").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.add(egui::Button::new("Validar YAML").min_size(egui::vec2(120.0, 40.0))).clicked() {
                    if let Some(mapping) = &self.mapping {
                        let yaml = serde_yaml::to_string(mapping).unwrap_or_default();
                        let output = run_tool_with_data("validate", &yaml);
                        if output.trim().is_empty() {
                            self.log("Validação: sucesso");
                        } else {
                            for line in output.lines().filter(|l| !l.trim().is_empty()) {
                                self.log(&format!("Validação: {}", line.trim()));
                            }
                        }
                    }
                }

                if ui.add(egui::Button::new("Upload YAML").min_size(egui::vec2(120.0, 40.0))).clicked() {
                    if let Some(mapping) = &self.mapping {
                        let yaml = serde_yaml::to_string(mapping).unwrap_or_default();
                        let output = run_tool_with_data("upload", &yaml);
                        if output.trim().is_empty() {
                            self.log("Upload: sucesso");
                        } else {
                            for line in output.lines().filter(|l| !l.trim().is_empty()) {
                                self.log(&format!("Upload: {}", line.trim()));
                            }
                        }
                    }
                }
            });
        });

        egui::SidePanel::left("left_panel")
            .resizable(false)
            .min_width(600.0)
            .default_width(600.0)
            .show(ctx, |ui| {
                if let Some(mapping) = &self.mapping {
                    draw_keyboard(ui, mapping);

                    ui.separator();
                    ui.label("Knobs:");
                    for (i, knob) in mapping.layers[0].knobs.iter().enumerate() {
                        ui.group(|ui| {
                            ui.label(format!("◉ Knob {}", i + 1));
                            ui.label(format!("↻ ccw (counter-clockwise / Sentido Anti Horário):  {}", knob.ccw));
                            ui.label(format!("press (Apertar):  {}", knob.press));
                            ui.label(format!("↺ cw (clockwise / Sentido Horário):  {}", knob.cw));
                        });
                    }
                }
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::BLACK))
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Log:").color(egui::Color32::WHITE));
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .auto_shrink([false; 2])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in &self.output_log {
                            ui.label(egui::RichText::new(line).monospace().color(egui::Color32::WHITE));
                        }
                    });
            });
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MappingFile {
    orientation: String,
    rows: usize,
    columns: usize,
    knobs: usize,
    layers: Vec<Layer>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Layer {
    buttons: Vec<Vec<String>>,
    knobs: Vec<Knob>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Knob {
    ccw: String,
    press: String,
    cw: String,
}

fn draw_keyboard(ui: &mut egui::Ui, mapping: &MappingFile) {
    let mut buttons = mapping.layers[0].buttons.clone();
    match mapping.orientation.as_str() {
        "upsidedown" => {
            for row in &mut buttons {
                row.reverse();
            }
        }
        "clockwise" => {
            buttons = transpose(&buttons);
            buttons.reverse();
        }
        "counterclockwise" => {
            buttons = transpose(&buttons);
            for row in &mut buttons {
                row.reverse();
            }
        }
        _ => {}
    }

    for row in buttons.iter() {
        ui.horizontal(|ui| {
            for label in row {
                ui.add(egui::Button::new(label).min_size(egui::vec2(100.0, 50.0)));
            }
        });
    }
}

fn transpose<T: Clone>(v: &Vec<Vec<T>>) -> Vec<Vec<T>> {
    if v.is_empty() || v[0].is_empty() {
        return vec![];
    }
    (0..v[0].len())
        .map(|i| v.iter().map(|row| row[i].clone()).collect())
        .collect()
}

fn run_tool_with_data(command: &str, yaml_content: &str) -> String {
    let mut cmd = Command::new("ch57x-keyboard-tool");
    cmd.arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = match cmd.spawn() {
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
    format!("{}\n{}", stdout.trim(), stderr.trim())
}

fn main() -> Result<(), eframe::Error> {
    let mut options = eframe::NativeOptions::default();
    options.viewport.inner_size = Some(egui::Vec2::new(1200.0, 600.0));
    eframe::run_native(
        "CH57x Keyboard GUI",
        options,
        Box::new(|_cc| Ok(Box::new(GuiApp::default()))),
    )
}