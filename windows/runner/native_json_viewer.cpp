#include "native_json_viewer.h"
#include "json_view_layout.h"
#include <flutter/encodable_value.h>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
#include <windowsx.h>
#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <functional>
#include <map>
#include <set>
#include <string>
#include <utility>
#include <vector>

namespace {
using Value = flutter::EncodableValue;
using Map = flutter::EncodableMap;
using Array = flutter::EncodableList;
using Channel = flutter::MethodChannel<Value>;
const Value& Field(const Map& map, const char* key) {
  static const Value empty;
  auto found = map.find(Value(key)); return found == map.end() ? empty : found->second;
}
const Map& AsMap(const Value& value) {
  static const Map empty; auto p = std::get_if<Map>(&value); return p ? *p : empty;
}
const Array& AsArray(const Value& value) {
  static const Array empty; auto p = std::get_if<Array>(&value); return p ? *p : empty;
}
double Number(const Value& value, double fallback = 0) {
  if (auto p = std::get_if<double>(&value)) return std::isfinite(*p) ? *p : fallback;
  if (auto p = std::get_if<int32_t>(&value)) return *p;
  if (auto p = std::get_if<int64_t>(&value)) return static_cast<double>(*p);
  return fallback;
}
int Integer(const Value& value) { return static_cast<int>(Number(value)); }
bool Boolean(const Value& value) { auto p = std::get_if<bool>(&value); return p && *p; }
std::string String(const Value& value) { auto p = std::get_if<std::string>(&value); return p ? *p : ""; }
std::wstring Wide(const std::string& value) {
  if (value.empty()) return {};
  const int n = MultiByteToWideChar(CP_UTF8,0,value.data(),static_cast<int>(value.size()),nullptr,0);
  std::wstring result(n,L'\0'); MultiByteToWideChar(CP_UTF8,0,value.data(),static_cast<int>(value.size()),result.data(),n); return result;
}
COLORREF Color(const Value& value) {
  const auto n = static_cast<uint32_t>(Number(value)); return RGB((n>>16)&255,(n>>8)&255,n&255);
}
RECT Rect(double x, double y, double w, double h) {
  return {static_cast<LONG>(std::floor(x)),static_cast<LONG>(std::floor(y)),
    static_cast<LONG>(std::ceil(x+w)),static_cast<LONG>(std::ceil(y+h))};
}
void Fill(HDC dc, RECT r, COLORREF color) { auto brush = CreateSolidBrush(color); FillRect(dc,&r,brush); DeleteObject(brush); }
struct Span { int start, length; std::string kind; };
struct Mapping { int start, length, source; };
struct RowData {
  std::wstring text;
  size_t columns = 0;
  std::vector<Span> spans;
  std::vector<Mapping> mappings;
  std::vector<bool> branches;
  bool last = true;
  int count = 0;
  bool has_swatch = false;
  COLORREF swatch = 0;
};
class JsonHost {
 public:
  using Emit = std::function<void(const char*, Map)>;
  JsonHost(HWND parent, Emit emit) : emit_(std::move(emit)) {
    static const bool registered = [] {
      WNDCLASSW wc{}; wc.style = CS_DBLCLKS; wc.lpfnWndProc = WindowProc; wc.hInstance = GetModuleHandle(nullptr);
      wc.hCursor = LoadCursor(nullptr,IDC_IBEAM); wc.lpszClassName = L"FlutterViewerNativeJson";
      return RegisterClassW(&wc) != 0;
    }();
    if (!registered) return;
    SetFont(1);
    CreateWindowExW(0,L"FlutterViewerNativeJson",L"JSON viewer",
      WS_CHILD|WS_VSCROLL|WS_HSCROLL|WS_CLIPSIBLINGS|WS_TABSTOP,
      0,0,1,1,parent,nullptr,GetModuleHandle(nullptr),this);
  }
  ~JsonHost() { if (hwnd_) DestroyWindow(hwnd_); DeleteObject(font_); DeleteObject(bold_font_); }
  bool valid() const { return hwnd_ != nullptr; }
  void Clear() {
    code_.clear(); trees_.clear(); code_data_.clear(); tree_data_.clear(); document_.clear();
    code_closed_.clear(); tree_closed_.clear(); visible_.clear(); matches_.clear();
    active_ = anchor_ = end_ = -1; x_ = y_ = total_width_ = 0; Layout();
  }
  void Load(const Map& args) {
    Clear(); document_ = Wide(String(Field(args,"text")));
    Parse(AsArray(Field(args,"code")),code_,code_data_);
    Parse(AsArray(Field(args,"tree")),trees_,tree_data_);
    for (int i = 0; i < static_cast<int>(trees_.size()); ++i)
      if (trees_[i].fold > i && !Boolean(Field(args,"expanded"))) tree_closed_.insert(i);
    Rebuild();
  }
  void Configure(const Map& args) {
    bool next = Boolean(Field(args,"tree"));
    if (next != tree_) { tree_ = next; x_ = y_ = 0; anchor_ = end_ = -1; }
    count_pattern_ = Wide(String(Field(args,"countPattern")));
    if (count_pattern_.empty()) count_pattern_ = L"{count} items";
    palette_ = AsMap(Field(args,"palette")); SetFont(Number(Field(args,"fontScale"),1)); Rebuild(); Reveal();
    SetWindowTextW(hwnd_,tree_ ? L"JSON tree viewer" : L"JSON viewer");
  }
  void Frame(const Map& args) {
    scale_ = std::max(0.5,Number(Field(args,"scale"),1));
    SetWindowPos(hwnd_,HWND_TOP,static_cast<int>(std::round(Number(Field(args,"x"))*scale_)),
      static_cast<int>(std::round(Number(Field(args,"y"))*scale_)),
      static_cast<int>(std::round(Number(Field(args,"width"))*scale_)),
      static_cast<int>(std::round(Number(Field(args,"height"))*scale_)),SWP_NOACTIVATE);
    const auto clip = Rect(Number(Field(args,"clipX"))*scale_,Number(Field(args,"clipY"))*scale_,
      Number(Field(args,"clipWidth"))*scale_,Number(Field(args,"clipHeight"))*scale_);
    HRGN region = CreateRectRgn(clip.left,clip.top,clip.right,clip.bottom);
    if (!SetWindowRgn(hwnd_,region,TRUE)) DeleteObject(region);
    ShowWindow(hwnd_,Boolean(Field(args,"visible")) ? SW_SHOWNOACTIVATE : SW_HIDE); Layout();
  }
  void Matches(const Map& args) {
    matches_.clear(); active_ = -1;
    for (const auto& value : AsArray(Field(args,"ranges"))) {
      const auto& range = AsArray(value);
      if (range.size() == 2) {
        int a = Integer(range[0]), b = Integer(range[1]);
        if (a >= 0 && b > a && b <= static_cast<int>(document_.size())) matches_.emplace_back(a,b);
      }
    }
    InvalidateRect(hwnd_,nullptr,FALSE);
  }
  void Active(int value) { active_ = value >= 0 && value < static_cast<int>(matches_.size()) ? value : -1; Reveal(); InvalidateRect(hwnd_,nullptr,FALSE); }
 private:
  HWND hwnd_ = nullptr;
  HFONT font_ = nullptr, bold_font_ = nullptr;
  double character_ = 7.2;
  std::wstring count_pattern_ = L"{count} items";
  Emit emit_;
  Map palette_;
  std::wstring document_;
  std::vector<json_view::Row> code_, trees_;
  std::vector<RowData> code_data_, tree_data_;
  std::vector<int> visible_;
  std::set<int> code_closed_, tree_closed_;
  std::vector<std::pair<int,int>> matches_;
  int active_ = -1, anchor_ = -1, end_ = -1;
  bool tree_ = false, dragging_ = false;
  double x_ = 0, y_ = 0, width_ = 0, height_ = 0, total_width_ = 0, scale_ = 1, line_ = 22;
  const std::vector<json_view::Row>& Rows() const { return tree_ ? trees_ : code_; }
  const std::vector<RowData>& Data() const { return tree_ ? tree_data_ : code_data_; }
  std::set<int>& Closed() { return tree_ ? tree_closed_ : code_closed_; }
  double Gutter() const { return tree_ ? 12 : std::max(52.0,static_cast<double>(std::to_string(code_.size()).size())*8+28); }
  double TextX(const json_view::Row& row) const { return Gutter()+(tree_ ? (row.depth*4+2)*character_ : 0)-x_; }
  COLORREF Palette(const char* name) const { return Color(Field(palette_,name)); }
  void SetFont(double scale) {
    scale = std::clamp(scale,0.5,3.0); line_ = 22*scale;
    if (font_) DeleteObject(font_);
    if (bold_font_) DeleteObject(bold_font_);
    font_ = CreateFontW(-static_cast<int>(12*scale),0,0,0,FW_NORMAL,FALSE,FALSE,FALSE,
      DEFAULT_CHARSET,OUT_DEFAULT_PRECIS,CLIP_DEFAULT_PRECIS,CLEARTYPE_QUALITY,FIXED_PITCH|FF_MODERN,L"Consolas");
    LOGFONTW style{}; GetObjectW(font_,sizeof(style),&style); style.lfWeight = FW_SEMIBOLD;
    bold_font_ = CreateFontIndirectW(&style);
    HDC dc = GetDC(nullptr); const auto old = SelectObject(dc,font_); SIZE size{};
    GetTextExtentPoint32W(dc,L" ",1,&size); character_ = size.cx;
    SelectObject(dc,old); ReleaseDC(nullptr,dc);
  }
  static void Parse(const Array& values, std::vector<json_view::Row>& rows, std::vector<RowData>& data) {
    rows.reserve(values.size()); data.reserve(values.size());
    for (const auto& value : values) {
      const auto& map = AsMap(value);
      rows.push_back({Integer(Field(map,"start")),Integer(Field(map,"end")),Integer(Field(map,"fold")),Integer(Field(map,"depth"))});
      RowData row; row.text = Wide(String(Field(map,"text")));
      row.columns = static_cast<size_t>(Number(Field(map,"columns"),static_cast<double>(row.text.size())));
      for (const auto& item : AsArray(Field(map,"spans"))) {
        const auto& span = AsArray(item);
        if (span.size() == 3) row.spans.push_back({Integer(span[0]),Integer(span[1]),String(span[2])});
      }
      for (const auto& item : AsArray(Field(map,"maps"))) {
        const auto& m = AsArray(item);
        if (m.size() == 3) row.mappings.push_back({Integer(m[0]),Integer(m[1]),Integer(m[2])});
      }
      for (const auto& branch : AsArray(Field(map,"branches"))) row.branches.push_back(Boolean(branch));
      row.last = map.find(Value("last")) == map.end() || Boolean(Field(map,"last"));
      row.count = Integer(Field(map,"count"));
      row.has_swatch = map.find(Value("swatch")) != map.end(); row.swatch = Color(Field(map,"swatch"));
      data.push_back(std::move(row));
    }
  }
  void Rebuild() {
    visible_ = json_view::Visible(Rows(),Closed());
    size_t columns = 0;
    for (int i : visible_) columns = std::max(columns,Data()[i].columns+static_cast<size_t>(tree_ ? Rows()[i].depth*4+(Data()[i].count > 0 ? 18 : 0) : 0)+8);
    total_width_ = Gutter()+static_cast<double>(columns)*line_*0.6;
    Layout();
  }
  void Layout() {
    if (!hwnd_) return;
    RECT r{}; GetClientRect(hwnd_,&r); width_ = r.right/scale_; height_ = r.bottom/scale_;
    const double total = static_cast<double>(visible_.size())*line_+12;
    x_ = std::clamp(x_,0.0,std::max(0.0,total_width_-width_)); y_ = std::clamp(y_,0.0,std::max(0.0,total-height_));
    SCROLLINFO si{sizeof(SCROLLINFO),SIF_RANGE|SIF_PAGE|SIF_POS,0,static_cast<int>(total),static_cast<UINT>(height_),static_cast<int>(y_),0};
    SetScrollInfo(hwnd_,SB_VERT,&si,TRUE);
    si.nMax = static_cast<int>(total_width_); si.nPage = static_cast<UINT>(width_); si.nPos = static_cast<int>(x_);
    SetScrollInfo(hwnd_,SB_HORZ,&si,TRUE); InvalidateRect(hwnd_,nullptr,FALSE);
  }
  void Transform(HDC dc) const {
    SetGraphicsMode(dc,GM_ADVANCED);
    const XFORM t{static_cast<FLOAT>(scale_),0,0,static_cast<FLOAT>(scale_),0,0};
    SetWorldTransform(dc,&t); SetBkMode(dc,TRANSPARENT); SelectObject(dc,font_);
  }
  double Prefix(HDC dc, const std::wstring& text, int length) const {
    SIZE size{}; GetTextExtentPoint32W(dc,text.data(),std::clamp(length,0,static_cast<int>(text.size())),&size); return size.cx;
  }
  void Text(HDC dc, const std::wstring& text, double x, double y, COLORREF color) const {
    SetTextColor(dc,color); TextOutW(dc,static_cast<int>(x),static_cast<int>(y),text.data(),static_cast<int>(text.size()));
  }
  void Mark(HDC dc, const RowData& data, int start, int end, double left, double top, COLORREF background, COLORREF foreground) {
    for (const auto& m : data.mappings) {
      const int a = std::max(start,m.source), b = std::min(end,m.source+m.length);
      if (b <= a) continue;
      const int local = m.start+a-m.source, length = b-a;
      if (local < 0 || local+length > static_cast<int>(data.text.size())) continue;
      const double before = Prefix(dc,data.text,local), after = Prefix(dc,data.text,local+length);
      Fill(dc,Rect(left+before,top,after-before,line_),background);
      Text(dc,data.text.substr(local,length),left+before,top+2,foreground);
    }
  }
  void Branches(HDC dc, const json_view::Row& row, const RowData& data, double top) {
    if (!tree_ || row.depth == 0) return;
    const HPEN pen = CreatePen(PS_SOLID,1,Palette("muted")); const auto old = SelectObject(dc,pen);
    const auto column = [&](int i) { return Gutter()+(i*4+0.5)*character_-x_; };
    const auto line = [&](double x1,double y1,double x2,double y2) {
      MoveToEx(dc,static_cast<int>(std::round(x1)),static_cast<int>(std::round(y1)),nullptr);
      LineTo(dc,static_cast<int>(std::round(x2)),static_cast<int>(std::round(y2)));
    };
    for (size_t i = 0; i < data.branches.size(); ++i) if (data.branches[i])
      line(column(static_cast<int>(i)),top,column(static_cast<int>(i)),top+line_);
    const double left = column(row.depth-1), middle = top+line_/2;
    line(left,top,left,data.last ? middle : top+line_);
    line(left,middle,left+2.5*character_,middle);
    SelectObject(dc,old); DeleteObject(pen);
  }
  void Paint() {
    PAINTSTRUCT ps{}; HDC target = BeginPaint(hwnd_,&ps); RECT client{}; GetClientRect(hwnd_,&client);
    HDC dc = CreateCompatibleDC(target); HBITMAP bitmap = CreateCompatibleBitmap(target,std::max(1L,client.right),std::max(1L,client.bottom));
    auto previous = SelectObject(dc,bitmap); const int saved = SaveDC(dc); Transform(dc);
    Fill(dc,Rect(0,0,width_,height_),Palette("surface"));
    const int first = std::max(0,static_cast<int>((y_-6)/line_));
    const int last = std::min(static_cast<int>(visible_.size()),static_cast<int>(std::ceil((y_+height_)/line_))+1);
    for (int position = first; position < last; ++position) {
      const int i = visible_[position]; const auto& row = Rows()[i]; const auto& data = Data()[i];
      const double left = TextX(row), top = 6+position*line_-y_;
      Branches(dc,row,data,top);
      Text(dc,data.text,left,top+2,Palette("text"));
      for (const auto& span : data.spans) {
        if (span.start < 0 || span.length < 0 || span.start+span.length > static_cast<int>(data.text.size())) continue;
        const double before = Prefix(dc,data.text,span.start);
        if (tree_ && span.kind == "keyStrong") SelectObject(dc,bold_font_);
        Text(dc,data.text.substr(span.start,span.length),left+before,top+2,Palette(span.kind == "keyStrong" ? "key" : span.kind.c_str()));
        SelectObject(dc,font_);
      }
      for (const auto& mapping : data.mappings) {
        const int search_start = mapping.source, search_end = mapping.source+mapping.length;
        auto match = std::lower_bound(matches_.begin(),matches_.end(),search_start,[](const auto& m,int offset) { return m.second <= offset; });
        for (; match != matches_.end() && match->first < search_end; ++match) {
          const bool active = std::distance(matches_.begin(),match) == active_;
          Mark(dc,data,std::max(search_start,match->first),std::min(search_end,match->second),left,top,
            active ? RGB(251,146,60) : RGB(253,230,138),RGB(23,32,51));
        }
      }
      if (anchor_ >= 0 && end_ >= 0) Mark(dc,data,std::min(anchor_,end_),std::max(anchor_,end_),left,top,Palette("selected"),Palette("selectedText"));
      if (!tree_) {
        Fill(dc,Rect(0,top,std::max(0.0,Gutter()-x_),line_),Palette("surface"));
        Text(dc,std::to_wstring(i+1),8-x_,top+2,Palette("muted"));
      }
      const double end = left+Prefix(dc,data.text,static_cast<int>(data.text.size()));
      if (tree_ && data.count > 0) {
        std::wstring count = count_pattern_; const auto token = count.find(L"{count}");
        if (token != std::wstring::npos) count.replace(token,7,std::to_wstring(data.count));
        Text(dc,L"  "+count,end,top+2,Palette("muted"));
      }
      if (row.fold > i) {
        Text(dc,Closed().count(i) ? L"\u25b8" : L"\u25be",tree_ ? left-2*character_ : Gutter()-18-x_,top+2,Palette(tree_ ? "accent" : "muted"));
        if (!tree_ && Closed().count(i)) Text(dc,L" \u2026",end,top+2,Palette("muted"));
      }
      if (data.has_swatch) Fill(dc,Rect(end+8,top+4,12,12),data.swatch);
    }
    RestoreDC(dc,saved); BitBlt(target,0,0,client.right,client.bottom,dc,0,0,SRCCOPY);
    SelectObject(dc,previous); DeleteObject(bitmap); DeleteDC(dc); EndPaint(hwnd_,&ps);
  }
  void Reveal() {
    if (active_ < 0 || active_ >= static_cast<int>(matches_.size())) return;
    const int offset = matches_[active_].first;
    const auto previous = Closed().size();
    const int target = json_view::Reveal(Rows(),Closed(),offset,tree_);
    if (previous != Closed().size()) Rebuild();
    auto found = std::find(visible_.begin(),visible_.end(),target);
    if (found == visible_.end()) return;
    y_ = static_cast<double>(std::distance(visible_.begin(),found))*line_-height_*0.35;
    HDC dc = GetDC(hwnd_); const int saved = SaveDC(dc); Transform(dc);
    int local = 0;
    for (const auto& m : Data()[target].mappings) if (offset >= m.source && offset < m.source+m.length) local = m.start+offset-m.source;
    x_ = Gutter()+(tree_ ? (Rows()[target].depth*4+2)*character_ : 0)+Prefix(dc,Data()[target].text,local)-width_*0.35;
    RestoreDC(dc,saved); ReleaseDC(hwnd_,dc); Layout();
  }
  int Hit(double py) const {
    const int pos = static_cast<int>(std::floor((py+y_-6)/line_));
    return pos >= 0 && pos < static_cast<int>(visible_.size()) ? visible_[pos] : -1;
  }
  int Offset(int i, double px) {
    const auto& data = Data()[i]; const auto& row = Rows()[i];
    HDC dc = GetDC(hwnd_); const int saved = SaveDC(dc); Transform(dc);
    int low = 0, high = static_cast<int>(data.text.size()); const double local = std::max(0.0,px-TextX(row));
    while (low < high) { const int mid = (low+high)/2; if (Prefix(dc,data.text,mid) < local) low = mid+1; else high = mid; }
    RestoreDC(dc,saved); ReleaseDC(hwnd_,dc);
    if (low > 0 && low < static_cast<int>(data.text.size()) && data.text[low] >= 0xdc00 && data.text[low] <= 0xdfff) --low;
    for (const auto& m : data.mappings) if (low >= m.start && low <= m.start+m.length) return m.source+low-m.start;
    return low == 0 ? row.start : std::min(row.end,static_cast<int>(document_.size()));
  }
  void Down(double px, double py, bool twice) {
    SetFocus(hwnd_); const int i = Hit(py); if (i < 0) return;
    const auto& row = Rows()[i]; const double fold = tree_ ? TextX(row)-2*character_ : Gutter()-20-x_;
    HDC dc = GetDC(hwnd_); const int saved = SaveDC(dc); Transform(dc);
    const double end = tree_ ? TextX(row)+Prefix(dc,Data()[i].text,static_cast<int>(Data()[i].text.size())) : fold+20;
    RestoreDC(dc,saved); ReleaseDC(hwnd_,dc);
    if (row.fold > i && px >= fold && px <= end) {
      if (Closed().count(i)) Closed().erase(i); else Closed().insert(i); Rebuild(); return;
    }
    const int pos = Offset(i,px); if (!(GetKeyState(VK_SHIFT)&0x8000)) anchor_ = pos; end_ = pos;
    if (twice) { anchor_ = row.start; end_ = tree_ ? row.end : row.start+static_cast<int>(Data()[i].text.size()); }
    dragging_ = true; SetCapture(hwnd_); InvalidateRect(hwnd_,nullptr,FALSE);
  }
  void Copy() {
    if (anchor_ < 0 || end_ < 0 || anchor_ == end_) return;
    const int a = std::clamp(std::min(anchor_,end_),0,static_cast<int>(document_.size()));
    const int b = std::clamp(std::max(anchor_,end_),a,static_cast<int>(document_.size()));
    const auto text = document_.substr(a,b-a); const size_t bytes = (text.size()+1)*sizeof(wchar_t);
    HGLOBAL memory = GlobalAlloc(GMEM_MOVEABLE,bytes); if (!memory) return;
    auto dest = GlobalLock(memory); if (!dest) { GlobalFree(memory); return; }
    std::memcpy(dest,text.c_str(),bytes); GlobalUnlock(memory);
    if (OpenClipboard(hwnd_)) { EmptyClipboard(); if (!SetClipboardData(CF_UNICODETEXT,memory)) GlobalFree(memory); CloseClipboard(); }
    else GlobalFree(memory);
  }
  bool Key(WPARAM key) {
    const bool control = (GetKeyState(VK_CONTROL)&0x8000) != 0;
    if (control && key == 'A') { anchor_ = 0; end_ = static_cast<int>(document_.size()); InvalidateRect(hwnd_,nullptr,FALSE); return true; }
    if (control && key == 'C') { Copy(); return true; }
    if (control && key == 'F') { SetFocus(GetParent(hwnd_)); emit_("find",{}); return true; }
    if (key == VK_F3) { emit_("next",{{Value("delta"),Value((GetKeyState(VK_SHIFT)&0x8000) ? -1 : 1)}}); return true; }
    switch (key) {
      case VK_DOWN: y_ += line_; break;
      case VK_UP: y_ -= line_; break;
      case VK_NEXT: y_ += height_; break;
      case VK_PRIOR: y_ -= height_; break;
      case VK_HOME: y_ = 0; break;
      case VK_END: y_ = static_cast<double>(visible_.size())*line_; break;
      case VK_LEFT: x_ -= 40; break;
      case VK_RIGHT: x_ += 40; break;
      case VK_TAB: SetFocus(GetParent(hwnd_)); PostMessage(GetParent(hwnd_),WM_KEYDOWN,VK_TAB,0); return true;
      default: return false;
    }
    Layout(); return true;
  }
  void Scroll(bool vertical, WPARAM wparam) {
    double& pos = vertical ? y_ : x_; const double page = vertical ? height_ : width_;
    SCROLLINFO si{sizeof(SCROLLINFO),SIF_TRACKPOS,0,0,0,0,0}; GetScrollInfo(hwnd_,vertical ? SB_VERT : SB_HORZ,&si);
    switch (LOWORD(wparam)) {
      case SB_LINEUP: pos -= line_; break;
      case SB_LINEDOWN: pos += line_; break;
      case SB_PAGEUP: pos -= page; break;
      case SB_PAGEDOWN: pos += page; break;
      case SB_THUMBTRACK: case SB_THUMBPOSITION: pos = si.nTrackPos; break;
      case SB_TOP: pos = 0; break;
      case SB_BOTTOM: pos = vertical ? static_cast<double>(visible_.size())*line_ : total_width_; break;
    }
    Layout();
  }
  static LRESULT CALLBACK WindowProc(HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam) {
    auto self = reinterpret_cast<JsonHost*>(GetWindowLongPtr(hwnd,GWLP_USERDATA));
    if (message == WM_NCCREATE) {
      self = static_cast<JsonHost*>(reinterpret_cast<CREATESTRUCT*>(lparam)->lpCreateParams);
      self->hwnd_ = hwnd; SetWindowLongPtr(hwnd,GWLP_USERDATA,reinterpret_cast<LONG_PTR>(self));
    }
    if (!self) return DefWindowProc(hwnd,message,wparam,lparam);
    switch (message) {
      case WM_ERASEBKGND: return 1;
      case WM_PAINT: self->Paint(); return 0;
      case WM_SIZE: self->Layout(); return 0;
      case WM_GETDLGCODE: return DLGC_WANTARROWS|DLGC_WANTCHARS;
      case WM_KEYDOWN: if (self->Key(wparam)) return 0; break;
      case WM_LBUTTONDOWN: case WM_LBUTTONDBLCLK:
        self->Down(GET_X_LPARAM(lparam)/self->scale_,GET_Y_LPARAM(lparam)/self->scale_,message == WM_LBUTTONDBLCLK); return 0;
      case WM_MOUSEMOVE:
        if (self->dragging_) {
          const double py = GET_Y_LPARAM(lparam)/self->scale_;
          if (py < 0) self->y_ -= self->line_; else if (py > self->height_) self->y_ += self->line_;
          self->Layout(); const int i = self->Hit(std::clamp(py,6.0,std::max(6.0,self->height_-1)));
          if (i >= 0) self->end_ = self->Offset(i,GET_X_LPARAM(lparam)/self->scale_);
        }
        return 0;
      case WM_LBUTTONUP: self->dragging_ = false; ReleaseCapture(); return 0;
      case WM_CAPTURECHANGED: self->dragging_ = false; return 0;
      case WM_VSCROLL: self->Scroll(true,wparam); return 0;
      case WM_HSCROLL: self->Scroll(false,wparam); return 0;
      case WM_MOUSEWHEEL: {
        double delta = GET_WHEEL_DELTA_WPARAM(wparam)/static_cast<double>(WHEEL_DELTA)*self->line_*3;
        if (GET_KEYSTATE_WPARAM(wparam)&MK_SHIFT) self->x_ -= delta; else self->y_ -= delta;
        self->Layout(); return 0;
      }
      case WM_MOUSEHWHEEL: self->x_ += GET_WHEEL_DELTA_WPARAM(wparam)/static_cast<double>(WHEEL_DELTA)*self->line_*3; self->Layout(); return 0;
      case WM_NCDESTROY: self->hwnd_ = nullptr; SetWindowLongPtr(hwnd,GWLP_USERDATA,0); break;
    }
    return DefWindowProc(hwnd,message,wparam,lparam);
  }
};
}
class NativeJsonViewer::Impl {
 public:
  Impl(flutter::BinaryMessenger* messenger, HWND parent) : parent_(parent), channel_(messenger,"dev.quocan.flutterNetworkViewer/native_json",&flutter::StandardMethodCodec::GetInstance()) {
    channel_.SetMethodCallHandler([this](const auto& call, auto result) {
      if (call.method_name() == "capabilities") { result->Success(Value(1)); return; }
      if (!call.arguments()) { result->Error("arguments","Missing arguments"); return; }
      const auto& args = AsMap(*call.arguments()); const auto id = static_cast<int64_t>(Number(Field(args,"id")));
      const auto& method = call.method_name();
      if (method == "create") {
        auto host = std::make_unique<JsonHost>(parent_,[this,id](const char* name,Map fields) {
          fields[Value("id")] = Value(id); channel_.InvokeMethod(name,std::make_unique<Value>(std::move(fields)));
        });
        if (!host->valid()) { result->Error("create","Could not create native JSON window"); return; }
        hosts_[id] = std::move(host);
      } else if (method == "dispose") hosts_.erase(id);
      else if (auto found = hosts_.find(id); found != hosts_.end()) {
        auto& host = *found->second;
        if (method == "clear") host.Clear();
        else if (method == "document") host.Load(args);
        else if (method == "configure") host.Configure(args);
        else if (method == "frame") host.Frame(args);
        else if (method == "matches") host.Matches(args);
        else if (method == "active") host.Active(Integer(Field(args,"index")));
        else { result->NotImplemented(); return; }
      }
      result->Success();
    });
  }
  ~Impl() { channel_.SetMethodCallHandler(nullptr); }
 private:
  HWND parent_;
  Channel channel_;
  std::map<int64_t,std::unique_ptr<JsonHost>> hosts_;
};
NativeJsonViewer::NativeJsonViewer(flutter::BinaryMessenger* messenger, HWND parent) : impl_(std::make_unique<Impl>(messenger,parent)) {}
NativeJsonViewer::~NativeJsonViewer() = default;
