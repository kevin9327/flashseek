use std::path::PathBuf;
use std::time::SystemTime;

use eframe::egui::{self, Color32, RichText};

use crate::engine::Engine;
use crate::types::Hit;

/// Map a parsed config into the native window's name/content path fields.
/// Multiple content roots are joined with `;`.
pub fn apply_config_to_fields(cfg: &crate::EngineConfig) -> (String, String) {
    let name_root = cfg.name_root.to_string_lossy().into_owned();
    let content_root = cfg
        .content_roots
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(";");
    (name_root, content_root)
}

pub fn run() -> eframe::Result<()> {
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_title("Flashseek"),
        ..Default::default()
    };
    eframe::run_native(
        "Flashseek",
        native,
        Box::new(|_cc| Ok(Box::new(FlashseekApp::default()))),
    )
}

struct FlashseekApp {
    query: String,
    name_root: String,
    content_root: String,
    config_path: String,
    status: String,
    engine: Option<Engine>,
    hits: Vec<Hit>,
    selected: Option<usize>,
    last_query: String,
}

impl Default for FlashseekApp {
    fn default() -> Self {
        let home = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".into());
        let docs = PathBuf::from(&home).join("Documents");
        Self {
            query: String::new(),
            name_root: home.clone(),
            content_root: docs.to_string_lossy().into_owned(),
            config_path: String::new(),
            status: "Index a folder to search. Name root is the complete catalog; content root is body search only.".into(),
            engine: None,
            hits: Vec::new(),
            selected: None,
            last_query: String::new(),
        }
    }
}

impl FlashseekApp {
    fn content_roots(&self) -> Vec<PathBuf> {
        self.content_root
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .collect()
    }

    fn reindex(&mut self) {
        let name = PathBuf::from(self.name_root.trim());
        let content_roots = self.content_roots();
        match Engine::from_roots(&name, &content_roots) {
            Ok(engine) => {
                let n = engine.catalog.len();
                let bodies = engine.content.bodies_len();
                self.status = format!("Indexed {n} names, {bodies} bodies from {}", name.display());
                self.engine = Some(engine);
                self.refresh();
            }
            Err(e) => {
                self.status = format!("Index failed: {e}");
                self.engine = None;
            }
        }
    }

    fn load_config(&mut self) {
        let path = self.config_path.trim();
        if path.is_empty() {
            self.status = "Config path is empty.".into();
            return;
        }
        match std::fs::read_to_string(path) {
            Ok(text) => match crate::parse_engine_config(&text) {
                Ok(cfg) => {
                    let (name, content) = apply_config_to_fields(&cfg);
                    self.name_root = name;
                    self.content_root = content;
                    self.reindex();
                }
                Err(e) => {
                    self.status = format!("Config parse failed: {e}");
                }
            },
            Err(e) => {
                self.status = format!("Config read failed: {e}");
            }
        }
    }

    fn refresh(&mut self) {
        let Some(engine) = self.engine.as_ref() else {
            self.hits.clear();
            return;
        };
        self.hits = engine.query(&self.query, SystemTime::now());
        self.last_query = self.query.clone();
        if self.selected.map(|i| i >= self.hits.len()).unwrap_or(false) {
            self.selected = None;
        }
    }
}

impl eframe::App for FlashseekApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.query.clear();
            self.refresh();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
            self.reindex();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::C) && i.modifiers.ctrl) {
            if let Some(i) = self.selected.or(Some(0)) {
                if let Some(hit) = self.hits.get(i) {
                    ctx.copy_text(hit.record.path.to_string_lossy().into_owned());
                    self.status = format!("Copied {}", hit.record.path.display());
                }
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(i) = self.selected {
                if let Some(hit) = self.hits.get(i) {
                    open_path(&hit.record.path);
                }
            } else if let Some(hit) = self.hits.first() {
                open_path(&hit.record.path);
            }
        }
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Search");
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .desired_width(f32::INFINITY)
                        .hint_text("지난주 세금 pdf   or   invoice ext:pdf"),
                );
                if resp.changed() {
                    self.refresh();
                }
            });
            ui.horizontal(|ui| {
                ui.label("Names");
                ui.add(egui::TextEdit::singleline(&mut self.name_root).desired_width(280.0));
                ui.label("Content");
                ui.add(egui::TextEdit::singleline(&mut self.content_root).desired_width(280.0));
                if ui.button("Index").clicked() {
                    self.reindex();
                }
                ui.label(RichText::new(format!("{} hits", self.hits.len())).weak());
            });
            ui.horizontal(|ui| {
                ui.label("Config");
                ui.add(
                    egui::TextEdit::singleline(&mut self.config_path)
                        .desired_width(280.0)
                        .hint_text("name_root= / content_root= file"),
                );
                if ui.button("Load config").clicked() {
                    self.load_config();
                }
            });
            ui.label(RichText::new(&self.status).small().color(Color32::GRAY));
            ui.add_space(4.0);
        });

        egui::SidePanel::right("preview")
            .resizable(true)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.heading("Preview");
                ui.separator();
                if let Some(i) = self.selected {
                    if let Some(hit) = self.hits.get(i) {
                        ui.label(RichText::new(&hit.record.name).strong());
                        ui.label(RichText::new(hit.record.path.to_string_lossy()).small());
                        ui.label(format!(
                            "name={} path={} content={} score={:.1}",
                            hit.name_match, hit.path_match, hit.content_match, hit.score
                        ));
                        ui.separator();
                        if let Some(sn) = &hit.snippet {
                            show_snippet(ui, sn);
                        } else if is_image(&hit.record.path) {
                            ui.label("[image] double-click the row to open");
                        } else {
                            let handler = crate::preview::should_use_handler(&hit.record.path, false);
                            if handler {
                                match crate::preview::preview_handler_clsid(&hit.record.path) {
                                    Ok(Some(cls)) => ui.label(format!("Windows preview handler {cls}")),
                                    _ => ui.label("Windows preview handler registered lookup ran; none found."),
                                };
                            } else {
                                ui.label("No content snippet.");
                            }
                        }
                    }
                } else {
                    ui.label("Select a result.");
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, hit) in self.hits.iter().enumerate() {
                    let selected = self.selected == Some(i);
                    let label = format!("{}\n{}", hit.record.name, hit.record.path.display());
                    let resp = ui.selectable_label(selected, label);
                    if resp.clicked() {
                        self.selected = Some(i);
                    }
                    if resp.double_clicked() {
                        open_path(&hit.record.path);
                    }
                }
            });
        });
    }
}

fn show_snippet(ui: &mut egui::Ui, sn: &crate::types::Snippet) {
    let mut job = egui::text::LayoutJob::default();
    let mut cursor = 0usize;
    let mut spans: Vec<(usize, usize)> = sn.highlights.clone();
    spans.sort_unstable();
    for (a, b) in spans {
        if a > cursor && a <= sn.text.len() {
            job.append(
                &sn.text[cursor..a],
                0.0,
                egui::TextFormat::simple(egui::FontId::proportional(14.0), Color32::WHITE),
            );
        }
        let end = b.min(sn.text.len());
        if a < end {
            job.append(
                &sn.text[a..end],
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(14.0),
                    color: Color32::BLACK,
                    background: Color32::from_rgb(255, 220, 80),
                    ..Default::default()
                },
            );
            cursor = end;
        }
    }
    if cursor < sn.text.len() {
        job.append(
            &sn.text[cursor..],
            0.0,
            egui::TextFormat::simple(egui::FontId::proportional(14.0), Color32::WHITE),
        );
    }
    ui.label(job);
}

fn open_path(path: &std::path::Path) {
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", &path.to_string_lossy()])
        .spawn();
}

fn is_image(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()).map(|s| s.to_ascii_lowercase()).as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp")
    )
}

#[cfg(all(test, feature = "ui"))]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn apply_config_joins_content_roots() {
        let cfg = crate::EngineConfig {
            name_root: PathBuf::from(r"C:\"),
            content_roots: vec![
                PathBuf::from(r"C:\Users\a\Documents"),
                PathBuf::from(r"C:\Users\a\Downloads"),
            ],
        };
        let (name, content) = apply_config_to_fields(&cfg);
        assert_eq!(name, r"C:\");
        assert_eq!(content, r"C:\Users\a\Documents;C:\Users\a\Downloads");
    }

    #[test]
    fn apply_config_empty_content_roots() {
        let cfg = crate::EngineConfig {
            name_root: PathBuf::from(r"D:\data"),
            content_roots: vec![],
        };
        let (name, content) = apply_config_to_fields(&cfg);
        assert_eq!(name, r"D:\data");
        assert_eq!(content, "");
    }
}
