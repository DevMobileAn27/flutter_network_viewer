# Minh họa cho README

[Bản PowerPoint](screenshots/readme-guide.pptx) chứa ảnh chụp giao diện, chú thích và các khung đánh dấu có thể chỉnh riêng. Các slide được xếp theo chức năng; mỗi nhóm xuất thành một GIF. Chỉ dùng dữ liệu mô phỏng trong ảnh.

## Chụp lại và dựng slide

Cần môi trường desktop, Rust, Python với `pillow`, `python-pptx`, `pypdfium2`, và LibreOffice để xuất slide sang PDF.

```sh
work=docs/artifacts/readme-guide
python3 tools/readme_guide.py product "$work/product.png"
cargo run --locked --example readme_capture -- "$work/screens" "$work/product.png"
python3 tools/readme_guide.py build "$work/screens" docs/screenshots
```

Ví dụ mở cửa sổ chạy renderer Rust và nạp dữ liệu Demo Shop vào capture store, không kết nối ứng dụng thật. Khi minh họa kết nối thủ công, nút **Find** nạp dữ liệu demo. Ảnh chụp thô và tọa độ widget nằm trong `$work/screens`. Tham số `--soffice` cho phép chỉ định đường dẫn LibreOffice ở máy khác.

## Xuất lại sau khi sửa PowerPoint

Mở `docs/screenshots/readme-guide.pptx` trong PowerPoint hoặc LibreOffice Impress, chỉnh nội dung, rồi lưu lại. Giữ nguyên thứ tự và số slide của từng nhóm; nếu thêm hoặc đổi thứ tự slide, cập nhật danh sách nhóm trong `tools/readme_guide.py` trước khi xuất.

```sh
python3 tools/readme_guide.py render docs/screenshots/readme-guide.pptx docs/screenshots docs/artifacts/readme-guide/rendered-slides
```

Lệnh render chính tệp PowerPoint đã lưu rồi ghép các slide thành GIF theo từng nhóm.

Thêm `--topic connection` vào lệnh `render` để chỉ xuất GIF kết nối.
