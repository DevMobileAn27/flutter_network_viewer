#!/usr/bin/env python3
"""Build editable README slides, then render those slides to one GIF per topic."""

import argparse
import io
import json
from pathlib import Path
import subprocess

from PIL import Image, ImageDraw, ImageFont
from pptx import Presentation
from pptx.dml.color import RGBColor
from pptx.enum.shapes import MSO_SHAPE
from pptx.enum.text import PP_ALIGN
from pptx.util import Inches, Pt
import pypdfium2 as pdfium


# slug, title, viewport crop (top/bottom in UI points), (caption, highlighted widgets)
GUIDES = [
    ("connection", "Kết nối qua URL", (0, 485), [
        ("Mở Manual URL trên thanh công cụ.", ("manual-url",)),
        ("Dán URL VM Service hoặc DevTools, rồi bấm Find.", ("vm-uri", "connect-submit")),
        ("Khi Demo Shop xuất hiện, mở Network để xem dữ liệu mới.", ("app:ios-session-1",)),
    ]),
    ("network", "Xem chi tiết request", (0, 550), [
        ("Trong Network, chọn request POST /orders.", ("request:ios-session-1:fetch-1",)),
        ("Overview tách riêng Method, Domain, Endpoint và Query params.", ("metadata:row:0", "metadata:row:3")),
        ("Chọn Response body để đọc JSON trả về.", ("tab:4", "json-output")),
    ]),
    ("image-preview", "Xem trước ảnh trong response", (0, 680), [
        ("Mở Response body của request ảnh, rồi bấm Enlarge.", ("image-open",)),
        ("Dùng các nút +, − và Fit để chỉnh chế độ xem.", ("image-zoom-in", "image-fit")),
    ]),
    ("console-tags", "Lọc Console theo tag", (0, 470), [
        ("Bấm nút tam giác trong cột Message.", ("console-message-filter",)),
        ("Nhập từ khóa rồi nhấn + hoặc Enter để thêm tag.", ("message-tag-input", "message-tag-add")),
        ("Thêm nhiều tag; bấm × trên thẻ để xóa tag.", ("message-tag-card:0", "message-tag-card:1")),
        ("Console giữ các dòng có Message khớp ít nhất một tag.", ("demo-log",)),
    ]),
    ("console-detail", "Xem chi tiết một dòng log", (0, 565), [
        ("Trong Console, chọn một dòng log.", ("demo-log",)),
        ("Đọc các trường trong bảng Detail, hoặc chuyển sang Raw.", ("console-detail", "console-raw")),
        ("Raw hỗ trợ JSON và Tree để xem bản ghi gốc.", ("json-code", "json-tree")),
    ]),
    ("json-beautify", "Định dạng JSON", (0, 550), [
        ("Dán JSON vào khung trái, rồi bấm Format.", ("json-format",)),
        ("Kết quả đã định dạng xuất hiện ở bên phải.", ("json-output",)),
        ("Chọn Tree để mở hoặc thu gọn từng nhánh.", ("json-tree",)),
    ]),
    ("json-search", "Tìm trong JSON", (0, 550), [
        ("Bấm ô tìm kiếm trên thanh công cụ JSON.", ("json-workspace-find",)),
        ("Nhập Coffee. Dùng các mũi tên để chuyển kết quả.", ("json-workspace-find", "json-next")),
        ("Khi chuyển sang Tree, từ khóa và kết quả vẫn được giữ.", ("json-tree",)),
    ]),
    ("pause-resume", "Tạm dừng và tiếp tục capture", (0, 415), [
        ("Bấm Pause để ngừng nhận request và log mới.", ("pause",)),
        ("Thanh trạng thái báo capture đang tạm dừng. Bấm Resume để tiếp tục.", ("paused-banner", "pause")),
        ("Các mục cũ vẫn còn; Viewer tiếp tục nhận dữ liệu mới.", ("pause",)),
    ]),
    ("export", "Xuất capture thành tệp JSON", (0, 495), [
        ("Bấm Export trên thanh công cụ.", ("export",)),
        ("Chọn đường dẫn, rồi bấm Save export để lưu toàn bộ capture.", ("export-path", "save-export")),
    ]),
    ("clear", "Xóa dữ liệu đã thu thập", (0, 495), [
        ("Bấm Clear để dọn các request và log hiện có.", ("clear",)),
        ("Bảng trống sau khi xóa. Viewer vẫn nhận dữ liệu mới từ app đã kết nối.", ("Network",)),
    ]),
    ("settings-mcp", "Kết nối MCP với Codex hoặc Claude", (324, 560), [
        ("Trong Settings → MCP, bật công tắc MCP server.", ("mcp-enabled",)),
        ("Bấm nút sao chép cấu hình dành cho Codex hoặc Claude.", ("mcp-copy-codex", "mcp-copy-claude")),
    ]),
]

W, H = 12.5, 6.8
BG, INK, MUTED, BLUE, BORDER, RED = (
    "#F0F4FA", "#111D34", "#40516D", "#245BCD", "#A8B7CB", "#C82342"
)


def rgb(value):
    return RGBColor.from_string(value.removeprefix("#"))


def label(slide, text, bounds, size, color=INK, bold=False, right=False):
    frame = slide.shapes.add_textbox(*(Inches(v) for v in bounds)).text_frame
    frame.margin_left = frame.margin_right = frame.margin_top = frame.margin_bottom = 0
    p = frame.paragraphs[0]
    p.alignment = PP_ALIGN.RIGHT if right else PP_ALIGN.LEFT
    run = p.add_run()
    run.text = text
    run.font.name = "Arial"
    run.font.size = Pt(size)
    run.font.bold = bold
    run.font.color.rgb = rgb(color)


def rectangle(slide, bounds, color, line=None, weight=0.8):
    shape = slide.shapes.add_shape(MSO_SHAPE.RECTANGLE, *(Inches(v) for v in bounds))
    if color:
        shape.fill.solid()
        shape.fill.fore_color.rgb = rgb(color)
    else:
        shape.fill.background()
    if line:
        shape.line.color.rgb = rgb(line)
        shape.line.width = Pt(weight)
    else:
        shape.line.fill.background()


def make_slide(deck, screenshots, slug, title, crop, index, total, caption, widgets):
    slide = deck.slides.add_slide(deck.slide_layouts[6])
    slide.background.fill.solid()
    slide.background.fill.fore_color.rgb = rgb(BG)
    label(slide, "FLUTTER NETWORK VIEWER  /  HƯỚNG DẪN NHANH", (.38, .19, 10.2, .23), 10, BLUE, True)
    label(slide, title, (.38, .49, 11, .61), 28, bold=True)
    label(slide, f"{index:02d} / {total:02d}", (10.9, .22, 1.22, .31), 13, MUTED, True, True)
    for i in range(total):
        rectangle(slide, (10.94 + i * (.28 if total < 4 else .33), .65, .22, .045),
                  BLUE if i == index - 1 else BORDER)

    stem = screenshots / f"{slug}-{index}"
    info = json.loads(stem.with_suffix(".json").read_text(encoding="utf-8"))
    view_w, view_h = info["size"]
    top, bottom = crop
    crop_left, crop_right = 0, view_w
    if slug == "connection" and index == 2:
        a, b, c, d = info["widgets"]["manual-url-frame"]
        crop_left, crop_right = max(0, a - 60), min(view_w, c + 60)
        top, bottom = max(0, b - 60), min(view_h, d + 60)
    view_h_crop = bottom - top
    with Image.open(stem.with_suffix(".png")) as source:
        px = source.height / view_h
        img = source.crop((round(crop_left * px), round(top * px), round(crop_right * px), round(bottom * px))).convert("RGB")
    img.thumbnail((1500, 830), Image.Resampling.LANCZOS)
    stream = io.BytesIO()
    img.save(stream, format="PNG", optimize=True)
    stream.seek(0)

    scale = min(11.74 / (crop_right - crop_left), 4.88 / view_h_crop)
    w, h = (crop_right - crop_left) * scale, view_h_crop * scale
    left, y = (W - w) / 2, 1.16 + (4.88 - h) / 2
    rectangle(slide, (left - .016, y - .016, w + .032, h + .032), None, BORDER)
    slide.shapes.add_picture(stream, Inches(left), Inches(y), Inches(w), Inches(h))
    for key in widgets:
        a, b, c, d = info["widgets"][key]
        a, c = max(1, a - 3), min(view_w - 1, c + 3)
        b, d = max(top + 1, b - 3), min(bottom - 1, d + 3)
        if a >= c or b >= d:
            raise ValueError(f"{stem}: highlight {key} outside the screenshot crop")
        rectangle(slide, (left + (a - crop_left) * scale, y + (b - top) * scale, (c - a) * scale, (d - b) * scale),
                  None, RED, 2.2)

    rectangle(slide, (.38, 6.09, 11.74, .50), "#FFFFFF")
    rectangle(slide, (.38, 6.09, .055, .50), BLUE)
    label(slide, caption, (.54, 6.21, 11.4, .28), 15)


def generate(screenshots, output, soffice, topic=None):
    output.mkdir(parents=True, exist_ok=True)
    deck = Presentation()
    deck.slide_width, deck.slide_height = Inches(W), Inches(H)
    deck.core_properties.title = "Flutter Network Viewer — Hướng dẫn sử dụng"
    for slug, title, crop, slides in GUIDES:
        for index, (caption, widgets) in enumerate(slides, 1):
            make_slide(deck, screenshots, slug, title, crop, index, len(slides), caption, widgets)
    pptx = output / "readme-guide.pptx"
    deck.save(pptx)
    render(pptx, output, screenshots.parent / "rendered-slides", soffice, topic)


def render(pptx, output, pdf_dir, soffice, topic=None):
    # Render the editable deck itself. Never assemble GIFs from raw screenshots.
    output.mkdir(parents=True, exist_ok=True)
    pdf_dir.mkdir(parents=True, exist_ok=True)
    profile = (pdf_dir / "lo-profile").resolve().as_uri()
    pdf = pdf_dir / f"{pptx.stem}.pdf"
    pdf.unlink(missing_ok=True)
    subprocess.run([soffice, f"-env:UserInstallation={profile}", "--headless", "--convert-to",
                    "pdf", "--outdir", str(pdf_dir), str(pptx.resolve())], check=True, timeout=120)
    document = pdfium.PdfDocument(pdf)
    assert len(document) == sum(len(g[3]) for g in GUIDES)
    offset = 0
    for slug, _, _, slides in GUIDES:
        if topic and slug != topic:
            offset += len(slides)
            continue
        frames = []
        for index in range(offset, offset + len(slides)):
            page = document[index]
            bitmap = page.render(scale=1.25)
            frames.append(bitmap.to_pil().convert("RGB"))
            bitmap.close()
            page.close()
        offset += len(frames)
        gif = output / f"{slug}.gif"
        frames[0].save(gif, save_all=True, append_images=frames[1:], optimize=True,
                       duration=[2600] * (len(frames) - 1) + [3400], loop=0, disposal=1)
        with Image.open(gif) as image:
            assert image.n_frames == len(frames)
        print(f"{gif.name}: {len(frames)} slides, {gif.stat().st_size:,} bytes")
    document.close()


def make_product(path):
    image = Image.new("RGB", (680, 440), "#f8ead6")
    draw = ImageDraw.Draw(image)
    draw.rectangle((0, 0, 680, 20), fill="#9c401d")
    draw.ellipse((369, 59, 650, 351), fill="#f1d3a5")
    draw.ellipse((384, 281, 622, 329), fill="#dba963")
    draw.rounded_rectangle((526, 144, 611, 231), 29, outline="#8f3d23", width=16)
    draw.rounded_rectangle((402, 115, 560, 292), 37, fill="#ad4a25")
    draw.ellipse((406, 102, 556, 166), fill="#713d26")
    draw.ellipse((414, 109, 548, 153), fill="#b6774f")
    draw.arc((449, 120, 513, 144), 8, 317, fill="#f3cc9f", width=4)
    for xy, text, size, fill in [
        ((44, 85), "DEMO SHOP", 24, "#743820"),
        ((38, 156), "Coffee", 68, "#381f16"),
        ((42, 246), "Product image", 27, "#743820"),
        ((42, 373), "SYNTHETIC SAMPLE  ·  PNG RESPONSE", 18, "#743820"),
    ]:
        try:
            font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", size)
        except OSError:
            font = ImageFont.load_default(size=size)
        draw.text(xy, text, fill=fill, font=font)
    path.parent.mkdir(parents=True, exist_ok=True)
    image.save(path)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(dest="mode", required=True)
    demo = modes.add_parser("product")
    demo.add_argument("path", type=Path)
    build = modes.add_parser("build")
    build.add_argument("screenshots", type=Path)
    build.add_argument("output", type=Path)
    build.add_argument("--soffice", default="/Applications/LibreOffice.app/Contents/MacOS/soffice")
    export = modes.add_parser("render")
    export.add_argument("pptx", type=Path)
    export.add_argument("output", type=Path)
    export.add_argument("work", type=Path)
    export.add_argument("--soffice", default="/Applications/LibreOffice.app/Contents/MacOS/soffice")
    for command in (build, export):
        command.add_argument("--topic", choices=[g[0] for g in GUIDES])
    args = parser.parse_args()
    if args.mode == "product":
        make_product(args.path)
    elif args.mode == "build":
        generate(args.screenshots, args.output, args.soffice, args.topic)
    else:
        render(args.pptx, args.output, args.work, args.soffice, args.topic)
