# Tạo GitHub Release

1. Tăng version trong `pubspec.yaml`, ví dụ `0.1.2+3`.
2. Sửa [`github_release_info.md`](../github_release_info.md):

   ```markdown
   ---
   tag: v0.1.2
   title: Flutter Network Viewer v0.1.2
   ---

   ## What's new

   - Nội dung bạn muốn hiển thị trên release.
   ```

   Giữ hai dòng `---`. Điền tag và title không có dấu ngoặc kép; toàn bộ Markdown phía dưới là description.

3. Cấu hình [khóa ký cập nhật](app-updates.md) một lần. Commit và push lên `main`. Khi version `X.Y.Z` tăng, workflow **Build and draft release** tự kiểm tra, build macOS Apple Silicon, macOS Intel và Windows x64, rồi tạo **Draft Release** kèm ba ZIP, installer Windows, feed cập nhật đã ký và `SHA256SUMS.txt`.
4. Vào **Releases**, kiểm tra draft rồi bấm **Publish release**.

Chỉ tăng số build sau dấu `+` không tạo release mới. Không cần tự tạo hoặc push tag. Tag được tạo đúng commit khi publish draft.

Có thể chạy thủ công trong **Actions → Build and draft release → Run workflow** trên nhánh `main` để thử lại khi build thất bại. Chọn version/tag chưa có release; workflow không ghi đè release hoặc di chuyển tag đã tồn tại sang commit khác.

File ZIP cũng được lưu trong Artifacts trong 7 ngày. Các file đính kèm Draft Release được giữ riêng, không phụ thuộc thời hạn này.

Nếu repository đang chặn quyền ghi của `GITHUB_TOKEN`, quản trị viên cần cho phép workflow tạo release. Workflow chỉ dùng quyền `contents: write` ở bước tạo draft.
