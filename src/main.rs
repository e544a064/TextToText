#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod transform;

use eframe::egui;
use transform::Transform;

fn main() -> eframe::Result<()> {
	let options = eframe::NativeOptions {
		viewport: (
			egui::ViewportBuilder::default()
			. with_title("Text to Text")
			. with_inner_size([920.0, 800.0])
			. with_min_inner_size([560.0, 520.0])
		),
		..Default::default()
	};

	eframe::run_native(
		"Text to Text",
		options,
		Box::new(|cc| {
			configure_style(&cc.egui_ctx);
			Ok(Box::new(TextToTextApp::default()))
		}),
	)
}

fn configure_style(ctx: &egui::Context) {
	ctx.all_styles_mut(|style| {
		style.spacing.button_padding = egui::vec2(10.0, 4.0);
		style.spacing.item_spacing = egui::vec2(8.0, 6.0);
	});
}

struct TextToTextApp {
	input: String,
	output: String,
	status: Status,
}

struct Status {
	message: String,
	is_error: bool,
}

impl Default for TextToTextApp {
	fn default() -> Self {
		Self {
			input: String::new(),
			output: String::new(),
			status: Status {
				message: "Pick a transformation - the result appears in the bottom box.".to_owned(),
				is_error: false,
			},
		}
	}
}

impl TextToTextApp {
	fn run(&mut self, transform: Transform) {
		match transform.apply(&self.input) {
			Ok(result) => {
				self.output = result;
				self.status = Status {
					message: format!("{} applied.", transform.name()),
					is_error: false,
				};
			}
			Err(error) => {
				self.status = Status {
					message: format!("{} failed: {error}", transform.name()),
					is_error: true,
				};
			}
		}
	}

	fn set_status(&mut self, message: impl Into<String>) {
		self.status = Status {
			message: message.into(),
			is_error: false,
		};
	}

	fn status_panel(&mut self, ui: &mut egui::Ui) {
		egui::Panel::bottom("status_panel")
		. show_separator_line(false)
		. show(ui, |ui| {
			let color = if self.status.is_error {
				ui.visuals().error_fg_color
			} else {
				ui.visuals().weak_text_color()
			};
			ui.label(
				egui::RichText::new(&self.status.message)
				. size(16.0)
				. color(color),
			);
			ui.add_space(6.0);
		});
	}

	fn input_panel(&mut self, ui: &mut egui::Ui) {
		// Reserve space for the fixed controls row and a usable output box.
		let max_input_height = (ui.available_height() - 300.0).max(110.0);
		egui::Panel::top("input_panel")
		. resizable(true)
		. default_size(300.0)
		. min_size(110.0)
		. max_size(max_input_height)
		. show(ui, |ui| {
			ui.add_space(6.0);
			ui.horizontal(|ui| {
				ui.strong("Input");
				ui.weak(format!("{} characters", self.input.chars().count()));
				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
					if ui.button("Clear").clicked() {
						self.input.clear();
					}
					if ui.button("Swap input/output").clicked() {
						std::mem::swap(&mut self.input, &mut self.output);
						self.set_status("Swapped input and output.");
					}
				});
			});

			let rows = visible_rows(ui);
			egui::ScrollArea::vertical()
			. id_salt("input_scroll")
			. auto_shrink([false, false])
			. show(ui, |ui| {
				ui.add(
					egui::TextEdit::multiline(&mut self.input)
					. desired_width(f32::INFINITY)
					. desired_rows(rows)
					. font(egui::TextStyle::Monospace),
				);
			});
		});
	}

	fn output_panel(&mut self, ui: &mut egui::Ui) {
		egui::CentralPanel::default().show(ui, |ui| {
			ui.add_space(6.0);
			ui.horizontal(|ui| {
				ui.strong("Output");
				ui.weak(format!("{} characters", self.output.chars().count()));
				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
					if ui.button("Clear").clicked() {
						self.output.clear();
					}
					if ui.button("Copy").clicked() {
						ui.ctx().copy_text(self.output.clone());
						self.set_status("Copied output to the clipboard.");
					}
				});
			});

			let rows = visible_rows(ui);
			egui::ScrollArea::vertical()
			. id_salt("output_scroll")
			. auto_shrink([false, false])
			. show(ui, |ui| {
				ui.add(
					egui::TextEdit::multiline(&mut self.output)
					. desired_width(f32::INFINITY)
					. desired_rows(rows)
					. font(egui::TextStyle::Monospace),
				);
			});
		});
	}

	fn controls_panel(&mut self, ui: &mut egui::Ui) {
		// Fixed-size panel: it wraps its content, so resizing the input box never changes it.
		egui::Panel::top("controls_panel")
		. show_separator_line(false)
		. show(ui, |ui| {
			ui.add_space(6.0);
			ui.label(egui::RichText::new("Transformations").strong());
			ui.add_space(2.0);

			egui::Grid::new("transformations")
			. num_columns(Transform::GROUPS.len())
			. spacing([16.0, 4.0])
			. show(ui, |ui| {
				// One column per group, with the group name as its header.
				for &(group, _) in Transform::GROUPS {
					ui.label(egui::RichText::new(group).weak());
				}
				ui.end_row();

				let rows = (
					Transform::GROUPS
					. iter()
					. map(|(_, transforms)| transforms.len())
					. max()
					.unwrap_or(0)
				);

				for row in 0..rows {
					for &(_, transforms) in Transform::GROUPS {
						if let Some(&transform) = transforms.get(row) {
							if ui.button(transform.label()).clicked() {
								self.run(transform);
							}
						} else {
							// Keep the grid aligned when groups differ in length.
							ui.label("");
						}
					}
					ui.end_row();
				}
			});
		});
	}
}

impl eframe::App for TextToTextApp {
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		self.status_panel(ui);
		self.input_panel(ui);
		self.controls_panel(ui);
		self.output_panel(ui);
	}
}

/// How many monospace rows fit into the remaining height of the current panel.
fn visible_rows(ui: &egui::Ui) -> usize {
	let line_height = ui.text_style_height(&egui::TextStyle::Monospace);
	let available = ui.available_height() - ui.spacing().item_spacing.y;
	((available / line_height).floor() as usize).max(3)
}
