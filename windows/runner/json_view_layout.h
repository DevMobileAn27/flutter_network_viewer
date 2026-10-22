#ifndef RUNNER_JSON_VIEW_LAYOUT_H_
#define RUNNER_JSON_VIEW_LAYOUT_H_
#include <algorithm>
#include <iterator>
#include <set>
#include <vector>
namespace json_view {
struct Row { int start = 0, end = 0, fold = 0, depth = 0; };
inline std::vector<int> Visible(const std::vector<Row>& rows, const std::set<int>& closed) {
  std::vector<int> result;
  for (int i = 0; i < static_cast<int>(rows.size());) {
    result.push_back(i);
    i = closed.count(i) && rows[i].fold > i ? rows[i].fold + 1 : i + 1;
  }
  return result;
}
// Open all enclosing branches and return the deepest row containing the match.
inline int Reveal(const std::vector<Row>& rows, std::set<int>& closed, int offset, bool tree) {
  if (rows.empty()) return 0;
  auto found = std::upper_bound(rows.begin(),rows.end(),offset,
    [](int position, const Row& row) { return position < row.start; });
  int target = std::max(0,static_cast<int>(std::distance(rows.begin(),found))-1);
  if (tree) while (target > 0 && rows[target].end <= offset) --target;
  for (auto it = closed.begin(); it != closed.end();) {
    const auto& row = rows[*it];
    const int end = tree ? row.end : rows[row.fold].end;
    if (row.start <= offset && end > offset) it = closed.erase(it); else ++it;
  }
  return target;
}
}
#endif
