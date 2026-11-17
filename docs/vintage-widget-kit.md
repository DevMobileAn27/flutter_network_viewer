# Vintage widget kit — audit 2026-11-16

Production uses `desktop_style.rs` and `vintage.rs` as its style/component kit.
`VintageDemo` is a consumer of that kit. A catalog entry by itself does not prove
that the demo and production share an implementation; executable UI tests below
cover the changes made during this audit.

| Family | Shared implementation | Consumers / notes |
| --- | --- | --- |
| Palette, font, borders | `desktop_style::Palette`, `font_size`, `input_border`; Vintage tokens | All desktop screens, configurable Cascadia Mono |
| Hover / opaque backing / pointing cursor | `vintage::hover_action` | Action wrappers; 1.1x rebuilt in the resting layout slot |
| Square icon and short-label actions | `desktop_style::square_action` via `icon_control`, `square_icon`, `square_button`, `square_toggle` | Toolbars, copy, JSON find, dialog dismiss, font +/-; both axes owned locally |
| Font-size stepper | `vintage::font_size_stepper` | Settings and gallery; same disabled limits 10–24, only +/- change the value |
| Read-only text | `desktop_support::display_text`, `display_text_with_copy` | MCP endpoint, font value in Settings/gallery; shared background, padding, border, width measurement; selection configurable |
| Single-line input | `desktop_support::input_with_hint`, `outlined_text_edit` | Settings port and gallery inputs; `vintage::input` delegates to this renderer |
| Switch | `vintage::toggle` | MCP via Settings layout wrapper and gallery use the same track/knob renderer |
| Text / icon+text actions | `desktop_style::button`, `vintage::button` and `hover_action` | Button variants have contextual content/minimum sizes; Settings/manual-copy/header are layout wrappers, not square icon actions |
| Dropdown / options | egui ComboBox configured with shared palette/font and zero menu margin; `desktop_style::option`, Vintage dropdown option | No hover scale; contextual width/selection and menu content |
| Tabs / project selection | `desktop_style::pill`, Vintage tabs / key-value and table primitives | Different screen compositions retain the same font/palette/hover rules |
| Tables and metadata | Network/Console renderers; `vintage::key_value_table` | Screen-specific columns, borders, selection and shared action cells |
| JSON / Tree | `desktop_json.rs` | Production and gallery share viewer; seven syntax roles, folds, short solid Tree branch lines with gaps before child content; Code omits guides |
| Body previews / empty states | `desktop_body.rs`, text empty-state helpers with `vintage::empty_text(ctx)` | Specialized content renderers retain Vintage palette/font |
| Loader | `vintage::loading` | Shared 3–10 square-cell animation |
| Notifications / dialogs | `desktop_support::Notifications`, Vintage feedback compositions | Contextual content; shared palette/hover/opaque controls |

Findings corrected in this audit:

- Settings +/- were **34×44**, despite the `square_button` name. Parent form
  interaction height overrode the requested side. Square actions now scope both
  interaction dimensions, shared with icon controls. +/- are **34×34** at rest,
  grow 1.1x on hover and remain square; disabled actions do not grow.
- Gallery font controls still used text buttons and an editable DragValue. Both
  screens now call the same stepper; the value uses the existing display field.
- Gallery single-line inputs and switch had their own renderers. They now call
  the production field and switch implementation.

UI verification: `font_stepper_actions_are_square_at_rest_hover_and_disabled`,
`gallery_font_stepper_matches_production_square_display_only_control`,
`gallery_switch_previews_the_production_track_and_square_knob`, existing icon
geometry/centering tests, display-only persistence tests and inventory navigation.
Screen-specific tables, tabs, dialogs and action compositions remain wrappers;
this audit does not equate matching styles with a single renderer for every
screen. No new native screenshot was taken.

Icon-label spacing uses `desktop_style::ICON_TEXT_GAP = 4.` pixels. Mixed-font button labels use LayoutJob leading_space rather than space characters.
