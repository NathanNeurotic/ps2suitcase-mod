mod project;

use eframe::egui::{self, Color32};
use eframe::{egui::RichText, App, Frame, NativeOptions};
use project::ProjectState;
use psu_packer::TimestampPolicy;

pub fn main() -> eframe::Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([960.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "PS2 Suitcase",
        options,
        Box::new(|_cc| Ok(Box::new(SuitcaseApp::default()))),
    )
}

#[derive(Default)]
struct SuitcaseApp {
    project: Option<ProjectState>,
    override_policy: Option<TimestampPolicy>,
    verify_after_export: bool,
    status: Option<String>,
}

impl App for SuitcaseApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        if let Some(project) = &mut self.project {
            project.refresh(false);
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open folder").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.project = Some(ProjectState::open(path));
                        self.status = Some("Project opened".to_string());
                    }
                }

                if let Some(policy) = self.override_policy {
                    ui.label(format!("Policy override: {policy}"));
                } else {
                    ui.label("Policy override: manifest");
                }

                ui.checkbox(&mut self.verify_after_export, "Verify after export");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(project) = &self.project {
                ui.heading(
                    project
                        .root()
                        .to_str()
                        .unwrap_or("<invalid utf-8>")
                        .to_string(),
                );

                if let Some(status) = &self.status {
                    ui.colored_label(Color32::LIGHT_GREEN, status);
                }

                if let Some(error) = &project.last_error {
                    ui.colored_label(Color32::RED, format!("Error: {error}"));
                }

                if !project.validation.is_empty() {
                    ui.separator();
                    ui.heading("Validation");
                    for issue in &project.validation {
                        let mut text = RichText::new(format!("{}: {}", issue.field, issue.message))
                            .color(Color32::RED);
                        if let Some(help) = &issue.help {
                            text = text.italics();
                            ui.label(text);
                            ui.label(RichText::new(format!("  hint: {help}")).color(Color32::LIGHT_BLUE));
                        } else {
                            ui.label(text);
                        }
                    }
                }

                if let Some(manifest) = &project.manifest {
                    ui.separator();
                    ui.heading("Preview");
                    ui.label(format!(
                        "Memory card folder: {} (policy: {})",
                        manifest.name,
                        manifest.timestamp_policy
                    ));

                    if let Some(preview) = &project.preview {
                        if preview.files.is_empty() {
                            ui.label("No files selected by manifest.");
                        } else {
                            egui::Grid::new("preview_grid")
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label(RichText::new("File").strong());
                                    ui.label(RichText::new("Bytes").strong());
                                    ui.label(RichText::new("Timestamp").strong());
                                    ui.end_row();

                                    for file in &preview.files {
                                        ui.label(&file.name);
                                        ui.label(file.size.to_string());
                                        ui.label(file.timestamp.to_string());
                                        ui.end_row();
                                    }
                                });

                            if preview
                                .files
                                .iter()
                                .any(|file| file.name.eq_ignore_ascii_case("icon.sys"))
                            {
                                ui.add_space(8.0);
                                ui.label(RichText::new("icon.sys preview").underline());
                                ui.label("Binary preview not yet implemented – contents verified via packer.");
                            }
                        }
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Override policy:");
                        let mut current = self.override_policy;
                        egui::ComboBox::from_id_salt("policy_override")
                            .selected_text(match current {
                                Some(policy) => policy.to_string(),
                                None => "manifest".to_string(),
                            })
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(current.is_none(), "manifest").clicked() {
                                    current = None;
                                }
                                for policy in [
                                    TimestampPolicy::Fixed,
                                    TimestampPolicy::Source,
                                    TimestampPolicy::Now,
                                ] {
                                    if ui
                                        .selectable_label(current == Some(policy), policy.to_string())
                                        .clicked()
                                    {
                                        current = Some(policy);
                                    }
                                }
                            });
                        self.override_policy = current;

                        if ui.button("Export PSU").clicked() {
                            self.export_current();
                        }
                    });
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Open a folder with a psu.toml manifest to begin.");
                });
            }
        });
    }
}

impl SuitcaseApp {
    fn export_current(&mut self) {
        let Some(project) = &self.project else {
            return;
        };

        if !project.validation.is_empty() {
            self.status = Some("Cannot export: resolve validation errors first.".to_string());
            return;
        }

        let default_name = project
            .manifest
            .as_ref()
            .map(|manifest| format!("{}.psu", manifest.name.replace(' ', "_")))
            .unwrap_or_else(|| "output.psu".to_string());

        let save_path = rfd::FileDialog::new()
            .set_file_name(&default_name)
            .save_file();

        let Some(path) = save_path else {
            return;
        };

        match project.export(self.override_policy, self.verify_after_export, &path) {
            Ok(_) => {
                self.status = Some(format!("Exported to {}", path.display()));
            }
            Err(err) => {
                self.status = Some(format!("Export failed: {err}"));
            }
        }
    }
}
