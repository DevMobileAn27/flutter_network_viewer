//! Shared desktop JSON interaction, backed by the F02/F03/F07 document indexes.
use crate::desktop_support::tr;
use crate::{
    desktop::UiProbe,
    json::JsonDocument,
    json_find::{JsonFindController, ViewMode},
};
use eframe::egui::{self, Color32, FontId, TextFormat, Vec2};
use std::{collections::HashSet, sync::Arc};

#[derive(Default)]
pub struct JsonUiProbe {
    pub total_rows: usize,
    pub draft: String,
    pub text: String,
    pub error: String,
    pub query: String,
    pub find_error: String,
    pub matches: usize,
    pub active: Option<usize>,
    pub highlighted: usize,
    pub tree: bool,
    pub rows: Vec<usize>,
    pub swatches: Vec<String>,
    pub scroll: Vec2,
}

#[derive(Default)]
pub struct JsonViewer {
    source: String,
    loaded: bool,
    find: Option<JsonFindController>,
    find_query: String,
    tree: bool,
    code_folds: HashSet<usize>,
    tree_folds: HashSet<usize>,
    reveal: bool,
    select_all: bool,
    selection: Option<((usize, usize), (usize, usize))>,
    row_cache: Vec<usize>,
    cache_tree: bool,
    cache_font_size: f32,
    cache_folds: HashSet<usize>,
    content_width: f32,
    row_colors: std::collections::HashMap<usize, usize>,
}
impl JsonViewer {
    pub fn set_tree(&mut self, tree: bool) {
        self.tree = tree;
    }
    pub fn fold_all(&mut self, folded: bool) {
        let Some(find) = &self.find else {
            return;
        };
        let p = find.document().presentation();
        let folds = if self.tree {
            &mut self.tree_folds
        } else {
            &mut self.code_folds
        };
        folds.clear();
        if folded {
            if self.tree {
                folds.extend(
                    p.tree
                        .iter()
                        .enumerate()
                        .filter(|(i, r)| r.fold > *i)
                        .map(|(i, _)| i),
                );
            } else {
                folds.extend(
                    p.code
                        .iter()
                        .enumerate()
                        .filter(|(i, r)| r.fold > *i)
                        .map(|(i, _)| i),
                );
            }
        }
    }

    pub fn load(&mut self, source: &str) -> bool {
        if self.loaded && self.source == source {
            return self.find.is_some();
        }
        self.loaded = true;
        self.source = source.into();
        match JsonDocument::parse(source) {
            Ok(document) => {
                self.source = source.into();
                self.set_document(Arc::new(document));
                true
            }
            Err(_) => {
                self.find = None;
                self.row_cache.clear();
                self.row_colors.clear();
                false
            }
        }
    }
    fn set_document(&mut self, document: Arc<JsonDocument>) {
        if let Some(find) = &mut self.find {
            find.set_document(document);
        } else {
            let mut find = JsonFindController::new(document);
            if !self.find_query.is_empty() {
                find.set_query(&self.find_query);
            }
            self.find = Some(find);
        }
        self.code_folds.clear();
        self.tree_folds.clear();
        self.select_all = false;
        self.selection = None;
        self.row_cache.clear();
        self.reveal = true;
    }
    pub fn show(&mut self, ui: &mut egui::Ui, probe: &mut UiProbe) {
        if self.find.is_none() {
            ui.label(tr(ui.ctx(), "Formatted JSON"));
            return;
        }
        let width = ui.available_width();
        let font_size = crate::desktop_style::font_size(ui.ctx());
        let toolbar = ui.scope(|ui| {
            ui.set_min_width(width);
            ui.spacing_mut().item_spacing = egui::vec2(4., 4.);
            ui.spacing_mut().button_padding = egui::vec2(2., 2.);
            ui.spacing_mut().interact_size = egui::vec2(24., 24.);
            ui.horizontal_wrapped(|ui| {
                let inline_width = 590. + (font_size - 14.).max(0.) * 8.;
                if width > inline_width {
                    ui.add_space(width - inline_width);
                }
                self.find_controls(
                    ui,
                    probe,
                    160.,
                    false,
                    (width < inline_width).then_some(width),
                );
            });
        });
        probe
            .widgets
            .insert("json-toolbar".into(), toolbar.response.rect);
        self.show_content(ui, probe);
    }

    fn find_controls(
        &mut self,
        ui: &mut egui::Ui,
        probe: &mut UiProbe,
        search_width: f32,
        workspace: bool,
        wrapped_width: Option<f32>,
    ) {
        let focus_find = ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)
                || i.consume_key(egui::Modifiers::CTRL, egui::Key::F)
        });
        let mut options = self
            .find
            .as_ref()
            .map_or_else(Default::default, |find| find.options());
        let active = self.find.as_ref().and_then(|find| find.index());
        let matches = self.find.as_ref().map_or(0, |find| find.matches().len());
        let side = json_text_control_side(ui.ctx());
        let group = ui.horizontal(|ui| {
            let key = if workspace {
                "json-workspace-find"
            } else {
                "json-find"
            };
            let hint = tr(
                ui.ctx(),
                if workspace {
                    "Find in JSON…"
                } else {
                    "Find in JSON"
                },
            );
            let (search, cleared) = crate::desktop_style::search_bar(
                ui,
                probe,
                key,
                "json-clear",
                &mut self.find_query,
                &hint,
                search_width,
            );
            if cleared {
                self.find_query.clear();
            }
            if workspace && self.find.is_some() {
                probe.widgets.insert("json-find".into(), search.rect);
            }
            if focus_find {
                search.request_focus();
            }
            ui.add_enabled_ui(self.find.is_some(), |ui| {
                ui.horizontal(|ui| {
                    for (key, label, value) in [
                        ("json-case", "Aa", &mut options.match_case),
                        ("json-word", "ab", &mut options.whole_word),
                        ("json-regex", ".*", &mut options.regex),
                    ] {
                        if probe
                            .record(
                                key,
                                crate::desktop_style::square_toggle(ui, label, *value, side),
                            )
                            .clicked()
                        {
                            *value = !*value;
                        }
                    }
                });
            });
        });
        probe
            .widgets
            .insert("json-find-group".into(), group.response.rect);
        if wrapped_width.is_some() {
            ui.end_row();
        }

        ui.add_sized(
            [68., side],
            egui::Label::new(format!("{}/{matches}", active.map_or(0, |i| i + 1))),
        );
        let mut move_by = 0;
        for (key, direction) in [("json-prev", -1), ("json-next", 1)] {
            if probe
                .record(key, arrow_button(ui, direction, matches > 0))
                .clicked()
            {
                move_by = direction;
            }
        }
        if wrapped_width.is_some_and(|width| width < 320.) {
            ui.end_row();
        } else {
            ui.add_space(8.);
        }
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(3, if workspace { 0 } else { 3 }))
            .show(ui, |ui| {
                let labels = [tr(ui.ctx(), "JSON"), tr(ui.ctx(), "Tree")];
                let [json, tree] = crate::desktop_style::segmented_pair(
                    ui,
                    [&labels[0], &labels[1]],
                    &mut self.tree,
                );
                probe.record("json-code", json);
                probe.record("json-tree", tree);
            });
        if let Some(find) = &mut self.find {
            if !workspace
                && probe
                    .record(
                        "json-copy",
                        find_icon_button(ui, 0xef7f, &tr(ui.ctx(), "Copy JSON"), true),
                    )
                    .clicked()
            {
                crate::desktop_support::copy(ui.ctx(), find.document().text().to_owned());
            }
            if find.options() != options {
                find.set_options(options);
                self.reveal = true;
            }
            if find.query() != self.find_query {
                find.set_query(&self.find_query);
                self.reveal = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::F3)) {
                move_by = ui.input(|i| {
                    if i.events.iter().any(|event| {
                        matches!(event, egui::Event::Key {
                        key: egui::Key::F3, pressed: true, modifiers, ..
                    } if modifiers.shift)
                    }) {
                        -1
                    } else {
                        1
                    }
                });
            }
            if move_by != 0 {
                find.move_by(move_by);
                self.reveal = true;
            }
        }
    }

    fn show_content(&mut self, ui: &mut egui::Ui, probe: &mut UiProbe) {
        let Some(find) = &mut self.find else { return };
        let colors = crate::desktop_style::Palette::of(ui.ctx());
        find.set_mode(if self.tree {
            ViewMode::Tree
        } else {
            ViewMode::Code
        });
        crate::desktop_support::notify_changed(
            ui.ctx(),
            ui.id().with("json-find-error"),
            crate::desktop_support::NoticeKind::Error,
            find.error().map(str::to_owned),
        );
        if self.reveal {
            if self.tree {
                find.reveal_tree(&mut self.tree_folds);
            } else {
                find.reveal_code(&mut self.code_folds);
            }
        }
        let document = Arc::clone(find.document());
        let p = document.presentation();
        let folds = if self.tree {
            &mut self.tree_folds
        } else {
            &mut self.code_folds
        };
        let gutter = if self.tree {
            12.
        } else {
            (p.code.len().to_string().len() as f32 * 8. + 28.).max(52.)
        };
        let font = FontId::monospace(crate::desktop_style::font_size(ui.ctx()));
        let cell = ui.fonts_mut(|f| f.glyph_width(&font, ' '));
        let syntax = JsonSyntax::new(ui.visuals().dark_mode, colors);
        let count = if self.tree {
            p.tree.len()
        } else {
            p.code.len()
        };
        if self.row_cache.is_empty()
            || self.cache_tree != self.tree
            || self.cache_folds != *folds
            || self.cache_font_size != crate::desktop_style::font_size(ui.ctx())
        {
            self.cache_font_size = crate::desktop_style::font_size(ui.ctx());
            self.row_cache.clear();
            self.selection = None;
            self.row_colors.clear();
            let colors = document.color_previews();
            if self.tree {
                let by_path: std::collections::HashMap<_, _> = colors
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (&s.path, i))
                    .collect();
                for (row, r) in p.tree.iter().enumerate() {
                    if let Some(&index) = by_path.get(&r.path) {
                        self.row_colors.insert(row, index);
                    }
                }
            } else {
                for (index, s) in colors.iter().enumerate() {
                    let row = p.code.partition_point(|r| r.end < s.start);
                    if p.code
                        .get(row)
                        .is_some_and(|r| s.start >= r.start && s.start <= r.end)
                    {
                        self.row_colors.entry(row).or_insert(index);
                    }
                }
            }
            self.content_width = 200.;
            let mut index = 0;
            while index < count {
                self.row_cache.push(index);
                let (columns, depth, end) = if self.tree {
                    let r = &p.tree[index];
                    (r.columns + 18, r.depth * 4 + 2, r.fold)
                } else {
                    let r = &p.code[index];
                    (r.columns, 0, r.fold)
                };
                self.content_width = self
                    .content_width
                    .max((columns + depth) as f32 * cell + gutter + 32.);
                index = if folds.contains(&index) {
                    end + 1
                } else {
                    index + 1
                };
            }
            self.cache_tree = self.tree;
            self.cache_folds = folds.clone();
        }
        let visible = &self.row_cache;
        let output_id = ui.make_persistent_id(("json-output-text", self.tree));
        if ui.memory(|m| m.has_focus(output_id)) {
            if ui.input_mut(|i| {
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::A)
                    || i.consume_key(egui::Modifiers::CTRL, egui::Key::A)
            }) {
                self.select_all = true;
            }
            if ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy))) {
                if self.select_all {
                    crate::desktop_support::copy(ui.ctx(), document.text().to_owned());
                } else if let Some((a, b)) = self.selection {
                    let (a, b) = if a <= b { (a, b) } else { (b, a) };
                    let mut copied = String::new();
                    for (n, &row) in visible.iter().enumerate().take(b.0 + 1).skip(a.0) {
                        let text = if self.tree {
                            p.tree[row].text.clone()
                        } else {
                            p.code[row].text.clone()
                        };
                        let start = if n == a.0 { a.1 } else { 0 };
                        let end = if n == b.0 { b.1 } else { text.chars().count() };
                        copied.extend(text.chars().skip(start).take(end.saturating_sub(start)));
                        if n < b.0 {
                            copied.push('\n');
                        }
                    }
                    crate::desktop_support::copy(ui.ctx(), copied);
                }
            }
        }
        let row_height = (crate::desktop_style::font_size(ui.ctx()) + 8.).max(22.);
        let mut toggle = None;
        let mut highlighted = 0;
        let active = find.index().and_then(|i| find.matches().get(i));
        let target = active.and_then(|m| {
            visible.iter().position(|&row| {
                if self.tree {
                    p.tree[row].maps.iter().any(|map| {
                        m.start >= map.document_start && m.start < map.document_start + map.len
                    })
                } else {
                    m.start >= p.code[row].start && m.start <= p.code[row].end
                }
            })
        });
        let mut scroll_area = egui::ScrollArea::both()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .id_salt(("json-output-scroll", self.tree))
            .auto_shrink([false, false]);
        if self.reveal
            && let Some(target) = target
        {
            scroll_area =
                scroll_area.vertical_scroll_offset((target as f32 * row_height - 100.).max(0.));
        }
        let scroll = scroll_area.show_viewport(ui, |ui, viewport| {
            ui.painter().rect_filled(ui.clip_rect(), 0, colors.surface);
            let origin = ui.cursor().min + egui::vec2(0., 6.);
            ui.set_min_size(egui::vec2(
                self.content_width,
                visible.len() as f32 * row_height + 12.,
            ));
            let full = egui::Rect::from_min_size(
                origin,
                egui::vec2(self.content_width, visible.len() as f32 * row_height),
            );
            let response = ui.interact(
                egui::Rect::from_min_max(
                    full.min + egui::vec2(if self.tree { 12. } else { gutter }, 0.),
                    full.max,
                )
                .intersect(ui.clip_rect()),
                output_id,
                egui::Sense::click_and_drag(),
            );
            if response.clicked() || response.drag_started() {
                response.request_focus();
                self.select_all = false;
            }
            let pointer = ui.input(|i| i.pointer.interact_pos());
            let start = (viewport.min.y / row_height).floor().max(0.) as usize;
            let end = ((viewport.max.y / row_height).ceil() as usize + 1).min(visible.len());
            let pointer_row =
                pointer.map(|pos| ((pos.y - origin.y) / row_height).floor().max(0.) as usize);
            let mut hit = None;
            for (n, &row) in visible.iter().enumerate().take(end).skip(start) {
                probe.json.rows.push(row);
                let (text, doc_start, depth, fold_end) = if self.tree {
                    let r = &p.tree[row];
                    (r.text.as_str(), r.start, r.depth, r.fold)
                } else {
                    let r = &p.code[row];
                    (
                        r.text.as_str(),
                        r.start,
                        r.text.bytes().take_while(|b| *b == b' ').count() / 2,
                        r.fold,
                    )
                };
                let pos = origin + egui::vec2(0., n as f32 * row_height);
                let painter = ui.painter();
                let offset = if self.tree {
                    gutter + (depth * 4 + 2) as f32 * cell
                } else {
                    gutter
                };
                let text_pos = pos + egui::vec2(offset, 0.);
                if self.tree {
                    let tree = &p.tree[row];
                    let column = |index: usize| pos.x + gutter + (index as f32 * 4. + 0.5) * cell;
                    for (index, &continues) in tree.branches.iter().enumerate() {
                        if continues {
                            paint_branch(
                                painter,
                                egui::pos2(column(index), pos.y),
                                egui::pos2(column(index), pos.y + row_height),
                                colors.muted,
                            );
                        }
                    }
                    if depth > 0 {
                        let x = column(depth - 1);
                        let middle = pos.y + row_height / 2.;
                        paint_branch(
                            painter,
                            egui::pos2(x, pos.y),
                            egui::pos2(
                                x,
                                if tree.last {
                                    middle
                                } else {
                                    pos.y + row_height
                                },
                            ),
                            colors.muted,
                        );
                        // Match the original Flutter tree: short 2.5-cell elbows,
                        // leaving space before the child arrow and text. Descendant
                        // stems begin on the next row, never at the parent's arrow.
                        let end = x + 2.5 * cell;
                        paint_branch(
                            painter,
                            egui::pos2(x, middle),
                            egui::pos2(end, middle),
                            colors.muted,
                        );
                    }
                } else {
                    painter.text(
                        pos + egui::vec2(8., row_height / 2.),
                        egui::Align2::LEFT_CENTER,
                        row + 1,
                        FontId::monospace(crate::desktop_style::font_size(ui.ctx())),
                        colors.muted,
                    );
                }
                let mut job = egui::text::LayoutJob::default();
                let tree_value = self.tree.then(|| {
                    let tree = &p.tree[row];
                    let node = &p.nodes[&tree.path];
                    tree.maps
                        .iter()
                        .find(|map| map.document_start == node.value_start)
                        .map_or(0, |m| m.local_start)
                });
                let value_text = tree_value.map_or(text, |start| {
                    let mut utf16 = 0;
                    text.char_indices()
                        .find_map(|(i, ch)| {
                            let at = utf16;
                            utf16 += ch.len_utf16();
                            (at == start).then_some(&text[i..])
                        })
                        .unwrap_or("")
                });
                let code_colors = (!self.tree).then(|| syntax.code_colors(text));
                let mut local = 0;
                let mut active_character = None;
                for (character, ch) in text.chars().enumerate() {
                    let offset = if self.tree {
                        p.tree[row].maps.iter().find_map(|m| {
                            (local >= m.local_start && local < m.local_start + m.len)
                                .then_some(m.document_start + local.saturating_sub(m.local_start))
                        })
                    } else {
                        Some(doc_start + local)
                    };
                    let matched = offset.and_then(|offset| {
                        let index = find.matches().partition_point(|m| m.end <= offset);
                        find.matches()
                            .get(index)
                            .filter(|m| offset >= m.start && offset < m.end)
                            .map(|_| index)
                    });
                    let mut bg = matched.map(|i| {
                        if Some(i) == find.index() {
                            crate::app_colors::FIND_ACTIVE
                        } else {
                            crate::app_colors::FIND_OTHER
                        }
                    });
                    if matched.is_some() {
                        highlighted += 1;
                    }
                    if matched.is_some() && matched == find.index() && active_character.is_none() {
                        active_character = Some(character);
                    }
                    if self.select_all
                        || self.selection.is_some_and(|(a, b)| {
                            let (a, b) = if a <= b { (a, b) } else { (b, a) };
                            (n, character) >= a && (n, character) < b
                        })
                    {
                        bg = Some(ui.visuals().selection.bg_fill);
                    }
                    let foreground = if let Some(start) = tree_value {
                        if local + 2 < start {
                            syntax.key
                        } else if local < start {
                            colors.text
                        } else {
                            syntax.value_color(value_text)
                        }
                    } else {
                        code_colors.as_ref().expect("code colors")[character]
                    };
                    let selected = bg == Some(ui.visuals().selection.bg_fill);
                    let mut styled = format(
                        ui.ctx(),
                        if selected {
                            Color32::WHITE
                        } else if matched.is_some() {
                            crate::app_colors::FIND_TEXT
                        } else {
                            foreground
                        },
                        bg,
                    );
                    if tree_value.is_some_and(|start| local + 2 < start)
                        && p.tree[row].count.is_some()
                    {
                        styled.font_id = crate::desktop_style::semibold(
                            crate::desktop_style::font_size(ui.ctx()),
                        );
                    }
                    let mut buffer = [0; 4];
                    job.append(ch.encode_utf8(&mut buffer), 0., styled);
                    local += ch.len_utf16();
                }
                if self.tree {
                    if let Some(count) = p.tree[row].count {
                        job.append(
                            &format!(
                                "  {count} {}",
                                tr(ui.ctx(), if count == 1 { "item" } else { "items" })
                            ),
                            0.,
                            format(ui.ctx(), colors.muted, None),
                        );
                    }
                } else if folds.contains(&row) {
                    job.append(" ... ", 0., format(ui.ctx(), colors.muted, None));
                    // The matching closing row also carries any trailing comma.
                    job.append(
                        p.code[fold_end].text.trim(),
                        0.,
                        format(ui.ctx(), colors.text, None),
                    );
                }
                job.wrap.max_width = f32::INFINITY;
                for section in &mut job.sections {
                    section.format.line_height = Some(row_height);
                }
                let galley = ui.fonts_mut(|f| f.layout_job(job));
                if fold_end > row {
                    let center = pos
                        + egui::vec2(
                            offset
                                - if self.tree {
                                    1.5 * cell
                                } else {
                                    12. - depth as f32 * 2. * cell
                                },
                            row_height / 2.,
                        );
                    let button = egui::Rect::from_center_size(
                        center,
                        egui::vec2(if self.tree { 2. * cell } else { 20. }, row_height),
                    );
                    let r = ui
                        .interact(
                            button,
                            ui.id().with(("json-fold", row)),
                            egui::Sense::click(),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    // The visible glyphs sit above the center of the line box.
                    // Ignore the search and selection backgrounds when aligning.
                    let text_row = &galley.rows[0];
                    let glyphs = &text_row.visuals.mesh.vertices
                        [text_row.visuals.glyph_vertex_range.clone()];
                    let glyph_bounds =
                        glyphs
                            .iter()
                            .fold(egui::Rect::NOTHING, |mut bounds, glyph| {
                                bounds.extend_with(glyph.pos);
                                bounds
                            });
                    let paint_center = if glyphs.is_empty() {
                        center
                    } else {
                        egui::pos2(
                            center.x,
                            text_pos.y + text_row.pos.y + glyph_bounds.center().y,
                        )
                    };
                    let points = if folds.contains(&row) {
                        [[-2.5, -4.], [3., 0.], [-2.5, 4.]]
                    } else {
                        [[-4., -2.75], [4., -2.75], [0., 2.75]]
                    };
                    ui.painter().add(egui::Shape::convex_polygon(
                        points
                            .into_iter()
                            .map(|[x, y]| paint_center + egui::vec2(x, y))
                            .collect(),
                        if self.tree {
                            colors.accent
                        } else {
                            colors.muted
                        },
                        egui::Stroke::NONE,
                    ));
                    if probe.record(format!("json-fold:{row}"), r).clicked() {
                        toggle = Some(row);
                    }
                }
                if let Some(s) = self
                    .row_colors
                    .get(&row)
                    .map(|&i| &document.color_previews()[i])
                {
                    let [a, r, g, b] = s.argb.to_be_bytes();
                    let rect = egui::Rect::from_min_size(
                        text_pos + egui::vec2(galley.size().x + 8., 4.),
                        egui::vec2(12., 12.),
                    );
                    ui.painter().rect_filled(rect, 0, colors.control);
                    ui.painter()
                        .rect_filled(rect, 0, Color32::from_rgba_unmultiplied(r, g, b, a));
                    let response =
                        ui.interact(rect, ui.id().with(("color", row)), egui::Sense::click());
                    let key = format!("json-color:{row}");
                    probe.json.swatches.push(key.clone());
                    if probe
                        .record(key, response.on_hover_text(&s.value))
                        .clicked()
                    {
                        crate::desktop_support::copy(ui.ctx(), s.value.clone());
                    }
                }
                if pointer_row == Some(n)
                    && let Some(pointer) = pointer
                {
                    hit = Some((
                        n,
                        galley
                            .cursor_from_pos(pointer - text_pos)
                            .index
                            .min(text.chars().count()),
                    ));
                }
                if self.reveal
                    && let Some(cursor) = active_character
                {
                    let rect = galley
                        .pos_from_cursor(egui::text::CCursor::new(cursor))
                        .translate(text_pos.to_vec2())
                        .expand(8.);
                    ui.scroll_to_rect(rect, Some(egui::Align::Center));
                }
                ui.painter()
                    .galley(text_pos, galley, ui.visuals().text_color());
            }
            if ui.input(|i| i.pointer.primary_pressed())
                && response.hovered()
                && let Some(hit) = hit
            {
                self.selection = Some((hit, hit));
                response.request_focus();
                self.select_all = false;
            }
            if response.dragged()
                && let Some(hit) = hit
            {
                let anchor = self.selection.map_or(hit, |s| s.0);
                self.selection = Some((anchor, hit));
            }
            if response.dragged()
                && let Some(pointer) = pointer
            {
                let clip = ui.clip_rect();
                if pointer.y > clip.bottom() - 16. {
                    ui.scroll_with_delta(egui::vec2(0., -20.));
                }
                if pointer.y < clip.top() + 16. {
                    ui.scroll_with_delta(egui::vec2(0., 20.));
                }
            }
            let mut response = response;
            response.rect = full.intersect(ui.clip_rect());
            probe.record("json-output", response);
        });
        self.reveal = false;
        if let Some(row) = toggle
            && !folds.remove(&row)
        {
            folds.insert(row);
        }
        probe.json.total_rows = visible.len();
        // Probes never duplicate a large document on every frame.
        probe.json.text = document.text().chars().take(65536).collect();
        probe.json.highlighted = highlighted;
        probe
            .widgets
            .insert("json-content".into(), scroll.inner_rect);
        probe.json.scroll = scroll.state.offset;
        probe.json.query = find.query().into();
        probe.json.find_error = find.error().unwrap_or_default().into();
        probe.json.matches = find.matches().len();
        probe.json.active = find.index();
        probe.json.tree = self.tree;
    }
}
struct JsonSyntax {
    text: Color32,
    key: Color32,
    string: Color32,
    number: Color32,
    boolean: Color32,
    accent: Color32,
    muted: Color32,
}
impl JsonSyntax {
    fn new(dark: bool, p: crate::desktop_style::Palette) -> Self {
        let c = if dark {
            crate::app_colors::JSON_DARK
        } else {
            crate::app_colors::JSON_LIGHT
        };
        Self {
            text: p.text,
            key: c[1],
            string: c[2],
            number: c[3],
            boolean: c[4],
            accent: c[5],
            muted: c[6],
        }
    }
    fn value_color(&self, value: &str) -> Color32 {
        match value.as_bytes().first() {
            Some(b'"') => self.string,
            Some(b't' | b'f') => self.boolean,
            Some(b'n') => self.muted,
            Some(b'-' | b'0'..=b'9') => self.number,
            Some(b'{' | b'[') => self.accent,
            _ => self.text,
        }
    }
    // The input is a line of valid pretty JSON. Token boundaries keep digits in
    // strings (including UUIDs) in the same color as the rest of the string.
    fn code_colors(&self, text: &str) -> Vec<Color32> {
        let mut colors = Vec::new();
        let mut chars = text.char_indices().peekable();
        while let Some((_, ch)) = chars.next() {
            match ch {
                '"' => {
                    let mut count = 1;
                    let mut escaped = false;
                    for (_, c) in chars.by_ref() {
                        count += 1;
                        if c == '"' && !escaped {
                            break;
                        }
                        escaped = c == '\\' && !escaped;
                    }
                    let tail = chars.peek().map_or("", |(i, _)| &text[*i..]);
                    colors.extend(std::iter::repeat_n(
                        if tail.trim_start().starts_with(':') {
                            self.key
                        } else {
                            self.string
                        },
                        count,
                    ));
                }
                '-' | '0'..='9' | 't' | 'f' | 'n' => {
                    let color = match ch {
                        't' | 'f' => self.boolean,
                        'n' => self.muted,
                        _ => self.number,
                    };
                    colors.push(color);
                    while chars
                        .next_if(|(_, c)| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
                        .is_some()
                    {
                        colors.push(color);
                    }
                }
                '{' | '}' | '[' | ']' => colors.push(self.accent),
                _ => colors.push(self.text),
            }
        }
        colors
    }
}

pub(crate) fn json_text_control_side(ctx: &egui::Context) -> f32 {
    (crate::desktop_style::font_size(ctx) * 1.2 + 6.)
        .ceil()
        .max(24.)
}

fn paint_branch(painter: &egui::Painter, start: egui::Pos2, end: egui::Pos2, color: Color32) {
    painter.line_segment([start, end], egui::Stroke::new(1.0_f32, color));
}

fn find_icon_button(ui: &mut egui::Ui, code: u32, tooltip: &str, enabled: bool) -> egui::Response {
    let response = ui
        .add_enabled_ui(enabled, |ui| crate::desktop_style::icon_control(ui, code))
        .inner;
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, tooltip));
    response.on_hover_text(tooltip)
}

fn arrow_button(ui: &mut egui::Ui, direction: isize, enabled: bool) -> egui::Response {
    find_icon_button(
        ui,
        if direction < 0 { 0xe0a0 } else { 0xe097 },
        &tr(ui.ctx(), if direction < 0 { "Previous" } else { "Next" }),
        enabled,
    )
}

fn format(ctx: &egui::Context, color: Color32, background: Option<Color32>) -> TextFormat {
    TextFormat {
        font_id: FontId::monospace(crate::desktop_style::font_size(ctx)),
        color,
        background: background.unwrap_or(Color32::TRANSPARENT),
        ..Default::default()
    }
}
#[derive(Default)]
pub struct JsonWorkspace {
    draft: String,
    error: String,
    viewer: JsonViewer,
}
impl JsonWorkspace {
    pub fn export_text(&self) -> String {
        self.viewer
            .find
            .as_ref()
            .map_or_else(String::new, |f| f.document().text().to_owned())
    }
    pub fn toolbar(&mut self, ui: &mut egui::Ui, probe: &mut UiProbe, search_width: f32) -> bool {
        ui.spacing_mut().button_padding = egui::vec2(6., 3.);
        ui.spacing_mut().interact_size.y = json_text_control_side(ui.ctx()) + 4.;
        let mut download = false;
        if probe
            .record(
                "json-format",
                crate::vintage::borderless_primary_button(ui, &tr(ui.ctx(), "Format")),
            )
            .clicked()
        {
            self.format_draft(ui.ctx());
        }
        self.viewer
            .find_controls(ui, probe, search_width, true, None);
        for (key, code, title) in [
            ("json-workspace-copy", 0xef7f, "Copy JSON"),
            ("json-download", 0xf05b, "Export"),
        ] {
            if probe
                .record(
                    key,
                    find_icon_button(ui, code, &tr(ui.ctx(), title), self.viewer.find.is_some()),
                )
                .clicked()
            {
                if key == "json-download" {
                    download = true;
                } else {
                    crate::desktop_support::copy(ui.ctx(), self.export_text());
                }
            }
        }
        if let Some(rect) = probe.widgets.get("json-workspace-copy").copied()
            && self.viewer.find.is_some()
        {
            probe.widgets.insert("json-copy".into(), rect);
        }
        download
    }
    fn format_draft(&mut self, ctx: &egui::Context) {
        match JsonDocument::parse(&self.draft) {
            Ok(document) => {
                self.viewer.source = self.draft.clone();
                self.viewer.set_document(Arc::new(document));
                self.error.clear();
            }
            Err(e) => {
                self.error = format!(
                    "Invalid JSON — line {}, column {}: {}",
                    e.line, e.column, e.message
                );
                crate::desktop_support::notify(
                    ctx,
                    crate::desktop_support::NoticeKind::Error,
                    self.error
                        .replace("Invalid JSON", &tr(ctx, "Invalid JSON"))
                        .replace("line", &tr(ctx, "line"))
                        .replace("column", &tr(ctx, "column")),
                );
            }
        }
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        probe: &mut UiProbe,
        settings: &mut crate::settings::AppSettings,
    ) {
        egui::Frame::NONE
            .inner_margin(8)
            .show(ui, |ui| self.show_content(ui, probe, settings));
    }
    fn show_content(
        &mut self,
        ui: &mut egui::Ui,
        probe: &mut UiProbe,
        settings: &mut crate::settings::AppSettings,
    ) {
        let shortcut = ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter)
                || i.consume_key(egui::Modifiers::CTRL, egui::Key::Enter)
        });
        const PANE_PADDING: i8 = 4;
        let inset = f32::from(PANE_PADDING);
        let available = ui.available_size();
        let width = available.x - 16.;
        let min = 240_f32.min(width / 2.);
        let input_width =
            (width * settings.values().json_input_fraction as f32).clamp(min, width - min);

        let origin = ui.cursor().left();
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.;
            ui.allocate_ui_with_layout(
                egui::vec2(input_width, available.y),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    let p = crate::desktop_style::Palette::of(ui.ctx());
                    let height = ui.available_height();
                    let frame = egui::Frame::NONE
                        .fill(p.control)
                        .stroke(egui::Stroke::new(1.0_f32, p.line))
                        .corner_radius(0)
                        // The frame stroke takes the first pixel of the inset.
                        .inner_margin(PANE_PADDING - 1);
                    let mut input_focused = false;
                    let response = frame
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(
                                (input_width - 2. * inset).max(1.),
                                (height - 2. * inset).max(1.),
                            ));
                            egui::ScrollArea::both()
                                .scroll_bar_visibility(
                                    egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                                )
                                .id_salt("json-input-scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    let mut layouter =
                                        |ui: &egui::Ui, text: &dyn egui::TextBuffer, width: f32| {
                                            let mut job = egui::text::LayoutJob::simple(
                                                text.as_str().to_owned(),
                                                egui::FontId::monospace(
                                                    crate::desktop_style::font_size(ui.ctx()),
                                                ),
                                                ui.visuals().text_color(),
                                                width,
                                            );
                                            job.sections[0].format.line_height = Some(
                                                crate::desktop_style::font_size(ui.ctx()) + 8.,
                                            );
                                            ui.fonts_mut(|fonts| fonts.layout_job(job))
                                        };
                                    input_focused = ui
                                        .add(
                                            egui::TextEdit::multiline(&mut self.draft)
                                                .vertical_align(egui::Align::Min)
                                                .layouter(&mut layouter)
                                                .id_salt("json-draft")
                                                .font(egui::FontId::monospace(
                                                    crate::desktop_style::font_size(ui.ctx()),
                                                ))
                                                .frame(false)
                                                .margin(egui::Margin::ZERO)
                                                .desired_width(ui.available_width())
                                                .desired_rows(
                                                    ((ui.available_height())
                                                        / (crate::desktop_style::font_size(
                                                            ui.ctx(),
                                                        ) + 8.))
                                                        .max(3.)
                                                        as usize,
                                                )
                                                .hint_text(format!(
                                                    "{} {{\"name\": \"value\"}}",
                                                    tr(
                                                        ui.ctx(),
                                                        "Enter your json input here. Example:"
                                                    )
                                                )),
                                        )
                                        .has_focus();
                                });
                        })
                        .response;
                    crate::desktop_style::input_border(ui, response.rect, input_focused);
                    probe.record("json-input", response);
                },
            );
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(16., available.y), egui::Sense::drag());
            let p = crate::desktop_style::Palette::of(ui.ctx());
            ui.painter().vline(
                rect.center().x,
                rect.y_range(),
                (
                    if response.hovered() || response.dragged() {
                        3.
                    } else {
                        1.
                    },
                    if response.hovered() || response.dragged() {
                        p.accent
                    } else {
                        p.line
                    },
                ),
            );
            let response = probe.record(
                "json-divider",
                response.on_hover_cursor(egui::CursorIcon::ResizeHorizontal),
            );
            if response.double_clicked() {
                let _ = settings.set_json_input_fraction(0.5);
            } else if response.dragged()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let next = ((pos.x - origin).clamp(min, width - min) / width) as f64;
                let _ = settings.set_json_input_fraction(next.clamp(0.1, 0.9));
            }
            ui.allocate_ui_with_layout(
                egui::vec2((width - input_width).max(40.), available.y),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    if shortcut {
                        self.format_draft(ui.ctx());
                    }
                    probe
                        .widgets
                        .insert("json-output-pane".into(), ui.available_rect_before_wrap());
                    if self.viewer.find.is_none() {
                        let p = crate::desktop_style::Palette::of(ui.ctx());
                        let rect = ui.available_rect_before_wrap();
                        let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(rect));
                        painter.rect_filled(rect, 0, p.surface);
                        painter.rect_stroke(
                            rect,
                            0,
                            egui::Stroke::new(1.0_f32, p.line),
                            egui::StrokeKind::Inside,
                        );
                        let hint = egui::text::LayoutJob::simple(
                            tr(ui.ctx(), "Your formatted json input will be display here"),
                            egui::FontId::proportional(crate::desktop_style::font_size(ui.ctx())),
                            p.muted,
                            (rect.width() - 2. * inset).max(1.),
                        );
                        let hint = painter.layout_job(hint);
                        painter.galley(rect.min + egui::Vec2::splat(inset), hint, p.muted);
                    } else {
                        egui::Frame::NONE.inner_margin(PANE_PADDING).show(ui, |ui| {
                            self.viewer.show_content(ui, probe);
                        });
                    }
                },
            );
        });
        probe.json.draft = self.draft.clone();
        probe.json.error = self.error.clone();
    }
}

/// Review gallery uses the production tokenizer with seven replacement colors.
pub fn vintage_syntax_job(
    ctx: &egui::Context,
    source: &str,
    colors: [Color32; 7],
) -> egui::text::LayoutJob {
    let syntax = JsonSyntax {
        text: colors[0],
        key: colors[1],
        string: colors[2],
        number: colors[3],
        boolean: colors[4],
        accent: colors[5],
        muted: colors[6],
    };
    let mut job = egui::text::LayoutJob::default();
    let font_id = FontId::monospace(crate::desktop_style::font_size(ctx));
    for (ch, color) in source.chars().zip(syntax.code_colors(source)) {
        let color = if color == syntax.text && matches!(ch, '{' | '}' | '[' | ']') {
            syntax.accent
        } else {
            color
        };
        job.append(
            ch.encode_utf8(&mut [0; 4]),
            0.,
            TextFormat {
                font_id: font_id.clone(),
                color,
                line_height: Some(font_id.size + 8.),
                ..Default::default()
            },
        );
    }
    job
}
