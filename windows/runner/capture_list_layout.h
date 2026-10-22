#ifndef RUNNER_CAPTURE_LIST_LAYOUT_H_
#define RUNNER_CAPTURE_LIST_LAYOUT_H_
#include <algorithm>
#include <array>
#include <vector>

namespace capture_list {
inline std::array<double, 4> Columns(const std::array<double, 3>& widths, double total) {
  return {widths[0], std::max(140.0, total - widths[0] - widths[1] - widths[2] - 18), widths[1], widths[2]};
}
inline void Resize(std::array<double, 3>& widths, int divider, double delta, double total) {
  const double spare = std::max(0.0, Columns(widths, total)[1] - 140);
  if (divider == 0) widths[0] = std::clamp(widths[0] + delta, 64.0, std::min(1600.0, widths[0] + spare));
  if (divider == 1) widths[1] = std::clamp(widths[1] - delta, 56.0, std::min(1600.0, widths[1] + spare));
  if (divider == 2) {
    const double sum = widths[1] + widths[2];
    widths[1] = std::clamp(widths[1] + delta, std::max(56.0, sum - 1600), std::min(1600.0, sum - 96));
    widths[2] = sum - widths[1];
  }
}
// Prefix offsets include a final sentinel; empty lists have exactly {0}.
inline size_t RowAt(const std::vector<double>& offsets, double y) {
  auto found = std::upper_bound(offsets.begin(), offsets.end(), y);
  return found == offsets.begin() ? 0 : std::min(offsets.size() - 1, static_cast<size_t>(found - offsets.begin() - 1));
}
}  // namespace capture_list
#endif
