#include "../windows/runner/json_view_layout.h"
#include <cassert>
#include <iostream>
int main() {
  using json_view::Row;
  const std::vector<Row> tree{{0,100,4,0},{2,9,1,1},{12,90,4,1},{20,25,3,2},{30,80,4,2}};
  std::set<int> closed{2};
  assert((json_view::Visible(tree,closed) == std::vector<int>{0,1,2}));
  assert(json_view::Reveal(tree,closed,23,true) == 3);
  assert(closed.empty());
  closed = {0,2};
  assert(json_view::Reveal(tree,closed,50,true) == 4);
  assert((json_view::Visible(tree,closed) == std::vector<int>{0,1,2,3,4}));
  const std::vector<Row> code{{0,1,5,0},{2,9,1,0},{10,15,4,0},{16,27,3,0},{28,29,4,0},{30,31,5,0}};
  closed = {0,2};
  assert(json_view::Reveal(code,closed,18,false) == 3);
  assert(closed.empty());
  assert(json_view::Visible({},{}).empty());
  std::cout << "Native JSON layout tests passed\n";
}
