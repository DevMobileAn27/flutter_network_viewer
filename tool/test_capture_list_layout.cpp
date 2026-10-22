// clang++ -std=c++17 tool/test_capture_list_layout.cpp -o /tmp/test-capture-layout && /tmp/test-capture-layout
#include "../windows/runner/capture_list_layout.h"
#include <cassert>
#include <cmath>
int main() {
  using namespace capture_list;
  assert(RowAt({0},10)==0);
  assert(RowAt({0,36,96,132},-1)==0);
  assert(RowAt({0,36,96,132},35.9)==0);
  assert(RowAt({0,36,96,132},36)==1);
  assert(RowAt({0,36,96,132},1000)==3);
  std::array<double,3> widths{64,56,108};
  assert(Columns(widths,386)[1]==140);
  Resize(widths,0,-100,600); assert(widths[0]==64);
  Resize(widths,0,1000,600); assert(widths[0]==278);
  Resize(widths,1,1000,600); assert(widths[1]==56);
  widths={64,56,108};
  Resize(widths,2,1000,600); assert(widths[1]==68 && widths[2]==96);
  Resize(widths,2,-1000,600); assert(widths[1]==56 && widths[2]==108);
  widths={1600,1600,1600};
  Resize(widths,2,-10000,5000); assert(widths[1]==1600 && widths[2]==1600);
}
