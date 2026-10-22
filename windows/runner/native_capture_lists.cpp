#include "native_capture_lists.h"
#include "capture_list_layout.h"

#include <flutter/encodable_value.h>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
#include <windowsx.h>
#include <algorithm>
#include <cmath>
#include <cstdint>
#include <functional>
#include <limits>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>

namespace {
using Value = flutter::EncodableValue;
using Map = flutter::EncodableMap;
using Array = flutter::EncodableList;
using Channel = flutter::MethodChannel<Value>;
const Value& Field(const Map& map, const char* key) {
  static const Value empty;
  auto found = map.find(Value(key));
  return found == map.end() ? empty : found->second;
}
const Map& AsMap(const Value& value) {
  static const Map empty;
  auto map = std::get_if<Map>(&value);
  return map ? *map : empty;
}
const Array& AsArray(const Value& value) {
  static const Array empty;
  auto array = std::get_if<Array>(&value);
  return array ? *array : empty;
}
std::string String(const Value& value) {
  auto text = std::get_if<std::string>(&value);
  return text ? *text : "";
}
double Number(const Value& value, double fallback = 0) {
  if (auto n = std::get_if<double>(&value)) return std::isfinite(*n) ? *n : fallback;
  if (auto n = std::get_if<int32_t>(&value)) return *n;
  if (auto n = std::get_if<int64_t>(&value)) return static_cast<double>(*n);
  return fallback;
}
bool Boolean(const Value& value) {
  auto result = std::get_if<bool>(&value);
  return result && *result;
}
std::wstring Wide(const std::string& text) {
  if (text.empty()) return {};
  const int size = MultiByteToWideChar(CP_UTF8, 0, text.data(), static_cast<int>(text.size()), nullptr, 0);
  std::wstring result(size, L'\0');
  MultiByteToWideChar(CP_UTF8, 0, text.data(), static_cast<int>(text.size()), result.data(), size);
  return result;
}
COLORREF Color(const Value& value) {
  const auto n = static_cast<uint32_t>(Number(value, 0xff000000));
  return RGB((n >> 16) & 255, (n >> 8) & 255, n & 255);
}
RECT Rect(double x, double y, double w, double h) {
  return {static_cast<LONG>(std::floor(x)), static_cast<LONG>(std::floor(y)),
          static_cast<LONG>(std::ceil(x + w)), static_cast<LONG>(std::ceil(y + h))};
}
void Fill(HDC dc, RECT rect, COLORREF color) {
  HBRUSH brush = CreateSolidBrush(color);
  FillRect(dc, &rect, brush);
  DeleteObject(brush);
}
struct Summary {
  Map data;
  double measured_width = -1;
  double measured_height = 36;
  std::unordered_map<std::string, std::wstring> text;
  explicit Summary(Map fields) : data(std::move(fields)) {
    for (const auto& field : data) {
      if (auto key = std::get_if<std::string>(&field.first)) {
        if (auto value = std::get_if<std::string>(&field.second)) text[*key] = Wide(*value);
      }
    }
  }
  const std::wstring& Text(const char* key) const {
    static const std::wstring empty;
    auto found = text.find(key);
    return found == text.end() ? empty : found->second;
  }
  COLORREF GetColor(const char* key) const { return Color(Field(data, key)); }
};

class CaptureList {
 public:
  using Emit = std::function<void(const char*, Map)>;
  CaptureList(HWND parent, bool console, Emit emit) : console_(console), emit_(std::move(emit)) {
    static const bool registered = [] {
      WNDCLASSW wc{};
      wc.lpfnWndProc = WindowProc;
      wc.hInstance = GetModuleHandle(nullptr);
      wc.hCursor = LoadCursor(nullptr, IDC_ARROW);
      wc.lpszClassName = L"FlutterViewerNativeCaptureList";
      return RegisterClassW(&wc) != 0;
    }();
    if (!registered) return;
    body_ = Font(12, FW_NORMAL); small_ = Font(10, FW_NORMAL); badge_ = Font(9, FW_BOLD);
    CreateWindowExW(0, L"FlutterViewerNativeCaptureList", console ? L"Console logs" : L"Network requests",
      WS_CHILD | WS_VSCROLL | WS_HSCROLL | WS_CLIPSIBLINGS | WS_TABSTOP,
      0, 0, 1, 1, parent, nullptr, GetModuleHandle(nullptr), this);
    if (console_) ShowScrollBar(hwnd_, SB_HORZ, FALSE);
  }
  ~CaptureList() {
    if (hwnd_) DestroyWindow(hwnd_);
    DeleteObject(body_); DeleteObject(small_); DeleteObject(badge_);
  }
  bool valid() const { return hwnd_ != nullptr; }

  void Configure(const Map& args) {
    palette_ = AsMap(Field(args, "palette"));
    titles_.clear();
    for (const auto& title : AsArray(Field(args, "titles"))) titles_.push_back(Wide(String(title)));
    const auto& values = AsArray(Field(args, "widths"));
    if (values.size() == 3) {
      std::array<double, 3> next{Number(values[0],64), Number(values[1],56), Number(values[2],108)};
      if (next != configured_widths_) {
        configured_widths_ = next;
        widths_ = {std::clamp(next[0],64.0,1600.0), std::clamp(next[1],56.0,1600.0), std::clamp(next[2],96.0,1600.0)};
      }
    }
    Layout(true); InvalidateRect(hwnd_, nullptr, FALSE);
  }
  void Frame(const Map& args) {
    scale_ = std::max(0.5, Number(Field(args,"scale"),1));
    SetWindowPos(hwnd_, HWND_TOP, static_cast<int>(std::round(Number(Field(args,"x"))*scale_)),
      static_cast<int>(std::round(Number(Field(args,"y"))*scale_)),
      static_cast<int>(std::round(Number(Field(args,"width"))*scale_)),
      static_cast<int>(std::round(Number(Field(args,"height"))*scale_)), SWP_NOACTIVATE);
    if (!Boolean(Field(args,"visible"))) EmitScroll();
    ShowWindow(hwnd_, Boolean(Field(args,"visible")) ? SW_SHOWNOACTIVATE : SW_HIDE);
    Layout(false);
  }
  void Update(const Map& args) {
    const size_t first = RowAt(scroll_y_);
    const std::string anchor = first < order_.size() ? order_[first] : "";
    const double inset = first < order_.size() ? scroll_y_ - RowY(first) : 0;
    for (const auto& key : AsArray(Field(args,"removed"))) rows_.erase(String(key));
    for (const auto& fields : AsArray(Field(args,"rows"))) {
      Map row = AsMap(fields);
      auto key = String(Field(row,"key"));
      rows_.insert_or_assign(key, Summary(std::move(row)));
    }
    if (std::holds_alternative<Array>(Field(args,"order"))) {
      order_.clear(); indices_.clear();
      for (const auto& value : AsArray(Field(args,"order"))) {
        auto key = String(value);
        if (rows_.find(key) != rows_.end()) { indices_[key] = order_.size(); order_.push_back(key); }
      }
    }
    RebuildOffsets();
    if (scroll_y_ <= 1) scroll_y_ = 0;
    else if (indices_.find(anchor) != indices_.end()) scroll_y_ = RowY(indices_.at(anchor)) + inset;
    else scroll_y_ = 0;
    ClampScroll(); UpdateScrollbars(); InvalidateRect(hwnd_, nullptr, FALSE);
  }
  void Select(const std::string& key, bool notify) {
    const auto old = selected_;
    selected_ = key;
    InvalidateRow(old); InvalidateRow(key);
    if (notify) emit_("selected", {{Value("key"),Value(key)}});
  }

 private:
  HWND hwnd_ = nullptr;
  bool console_;
  Emit emit_;
  HFONT body_ = nullptr, small_ = nullptr, badge_ = nullptr;
  Map palette_;
  std::vector<std::wstring> titles_;
  std::unordered_map<std::string, Summary> rows_;
  std::vector<std::string> order_;
  std::unordered_map<std::string, size_t> indices_;
  std::vector<double> offsets_{0};
  std::array<double,3> widths_{64,56,108}, configured_widths_{0,0,0};
  std::string selected_;
  double scale_ = 1, width_ = 0, height_ = 0, total_width_ = 0;
  double scroll_x_ = 0, scroll_y_ = 0, last_drag_x_ = 0;
  int divider_ = -1;
  int wheel_remainder_ = 0;

  static HFONT Font(int size, int weight) {
    return CreateFontW(-size, 0, 0, 0, weight, FALSE, FALSE, FALSE, DEFAULT_CHARSET,
      OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, FIXED_PITCH | FF_MODERN, L"Consolas");
  }
  COLORREF Palette(const char* name) const { return Color(Field(palette_,name)); }
  double Header() const { return console_ ? 0 : 30; }
  double RowY(size_t row) const { return console_ ? offsets_[std::min(row,offsets_.size()-1)] : static_cast<double>(row)*52.0; }
  double RowHeight(size_t row) const { return console_ ? offsets_[row+1]-offsets_[row] : 52; }
  size_t RowAt(double y) const { return console_ ? capture_list::RowAt(offsets_,y) : std::min(order_.size(),static_cast<size_t>(std::max(0.0,y)/52)); }
  double ContentHeight() const { return console_ ? offsets_.back() : static_cast<double>(order_.size())*52.0; }
  std::array<double,4> Columns() const { return capture_list::Columns(widths_,total_width_); }
  std::array<double,3> Edges() const { auto c = Columns(); return {c[0],c[0]+c[1]+6,c[0]+c[1]+c[2]+12}; }
  int HitDivider(double x) const {
    if (!console_) { auto edges = Edges(); for (int i=0;i<3;++i) if (std::abs(x-edges[i]-3)<=4) return i; }
    return -1;
  }
  void Transform(HDC dc, double x=0, double y=0) const {
    SetGraphicsMode(dc, GM_ADVANCED);
    const XFORM transform{static_cast<FLOAT>(scale_),0,0,static_cast<FLOAT>(scale_),static_cast<FLOAT>(x*scale_),static_cast<FLOAT>(y*scale_)};
    SetWorldTransform(dc,&transform); SetBkMode(dc,TRANSPARENT);
  }
  double TextWidth(HDC dc, const std::wstring& text, HFONT font) const {
    auto old = SelectObject(dc,font); SIZE size{};
    GetTextExtentPoint32W(dc,text.data(),static_cast<int>(text.size()),&size); SelectObject(dc,old);
    return size.cx;
  }
  double BadgeWidth(HDC dc,const std::wstring& value) const { return TextWidth(dc,value,badge_)+12; }
  double MessageX(HDC dc,const Summary& row) const { return 8+BadgeWidth(dc,row.Text("level"))+6+TextWidth(dc,row.Text("time"),small_)+6; }
  void RebuildOffsets() {
    offsets_.assign(1,0);
    if (!console_) return;
    HDC dc = GetDC(hwnd_); int saved = SaveDC(dc); Transform(dc); SelectObject(dc,body_);
    for (const auto& key : order_) {
      auto& row = rows_.at(key);
      const double w = std::max(16.0,total_width_-MessageX(dc,row)-8);
      if (total_width_>1 && row.measured_width!=w) {
        RECT rect = Rect(0,0,w,0);
        DrawTextW(dc,row.Text("message").data(),static_cast<int>(row.Text("message").size()),&rect,DT_CALCRECT|DT_WORDBREAK|DT_NOPREFIX|DT_EDITCONTROL);
        row.measured_height = std::max(36.0,static_cast<double>(rect.bottom-rect.top)+12);
        row.measured_width = w;
      }
      offsets_.push_back(offsets_.back()+row.measured_height);
    }
    RestoreDC(dc,saved); ReleaseDC(hwnd_,dc);
  }
  void Layout(bool immediate) {
    if (!hwnd_) return;
    RECT client{}; GetClientRect(hwnd_,&client);
    width_ = (client.right-client.left)/scale_; height_ = (client.bottom-client.top)/scale_;
    double previous = total_width_;
    total_width_ = console_ ? width_ : std::max(width_,widths_[0]+widths_[1]+widths_[2]+158);
    if (console_ && previous!=total_width_) {
      if (immediate || previous<=1) RebuildOffsets();
      else SetTimer(hwnd_,1,120,nullptr);
    }
    ClampScroll(); UpdateScrollbars(); InvalidateRect(hwnd_,nullptr,FALSE);
  }
  void ClampScroll() {
    scroll_x_ = std::clamp(scroll_x_,0.0,std::max(0.0,total_width_-width_));
    scroll_y_ = std::clamp(scroll_y_,0.0,std::max(0.0,ContentHeight()-height_+Header()));
  }
  void UpdateScrollbars() {
    SCROLLINFO info{sizeof(SCROLLINFO),SIF_RANGE|SIF_PAGE|SIF_POS,0,0,0,0,0};
    info.nMax = static_cast<int>(std::min(static_cast<double>(std::numeric_limits<int>::max()-1),std::ceil(ContentHeight()*scale_)));
    info.nPage = static_cast<UINT>(std::max(0.0,(height_-Header())*scale_));
    info.nPos = static_cast<int>(scroll_y_*scale_); SetScrollInfo(hwnd_,SB_VERT,&info,TRUE);
    if (!console_) {
      info.nMax = static_cast<int>(std::ceil(total_width_*scale_)); info.nPage = static_cast<UINT>(std::max(0.0,width_*scale_));
      info.nPos = static_cast<int>(scroll_x_*scale_); SetScrollInfo(hwnd_,SB_HORZ,&info,TRUE);
    }
  }
  void EmitScroll() { emit_("scrolled",{{Value("x"),Value(scroll_x_)},{Value("y"),Value(scroll_y_)}}); }
  void Scroll(double x,double y) {
    scroll_x_=x; scroll_y_=y; ClampScroll(); UpdateScrollbars(); InvalidateRect(hwnd_,nullptr,FALSE); SetTimer(hwnd_,2,100,nullptr);
  }
  void ScrollBar(int bar, WPARAM command) {
    SCROLLINFO info{sizeof(SCROLLINFO),SIF_ALL,0,0,0,0,0}; GetScrollInfo(hwnd_,bar,&info);
    double pos = bar==SB_VERT ? scroll_y_ : scroll_x_;
    const double page = bar==SB_VERT ? std::max(36.0,height_-Header()) : width_;
    switch (LOWORD(command)) {
      case SB_LINEUP: pos-=36; break; case SB_LINEDOWN: pos+=36; break;
      case SB_PAGEUP: pos-=page; break; case SB_PAGEDOWN: pos+=page; break;
      case SB_TOP: pos=0; break; case SB_BOTTOM: pos=static_cast<double>(info.nMax)/scale_; break;
      case SB_THUMBTRACK: case SB_THUMBPOSITION: pos=info.nTrackPos/scale_; break;
      default: return;
    }
    Scroll(bar==SB_HORZ?pos:scroll_x_,bar==SB_VERT?pos:scroll_y_);
  }
  void Text(HDC dc,const std::wstring& value,RECT rect,HFONT font,COLORREF color,UINT flags=DT_SINGLELINE|DT_END_ELLIPSIS) const {
    if (rect.right<=rect.left || rect.bottom<=rect.top) return;
    auto old=SelectObject(dc,font); SetTextColor(dc,color);
    DrawTextW(dc,value.data(),static_cast<int>(value.size()),&rect,flags|DT_NOPREFIX);
    SelectObject(dc,old);
  }
  void Badge(HDC dc,const std::wstring& value,double x,double y,double available,COLORREF fill,bool selected) const {
    const double w=std::max(0.0,std::min(available,BadgeWidth(dc,value)));
    auto brush=CreateSolidBrush(fill); auto pen=CreatePen(PS_SOLID,1,selected?RGB(219,234,254):fill);
    auto old_b=SelectObject(dc,brush), old_p=SelectObject(dc,pen);
    RoundRect(dc,static_cast<int>(x),static_cast<int>(y),static_cast<int>(x+w),static_cast<int>(y+20),8,8);
    SelectObject(dc,old_b); SelectObject(dc,old_p); DeleteObject(brush); DeleteObject(pen);
    Text(dc,value,Rect(x+6,y+3,w-12,14),badge_,RGB(255,255,255));
  }
  void InvalidateRow(const std::string& key) {
    auto found=indices_.find(key); if (found==indices_.end()) return;
    RECT rect=Rect(0,(RowY(found->second)-scroll_y_+Header())*scale_,width_*scale_,RowHeight(found->second)*scale_);
    InvalidateRect(hwnd_,&rect,FALSE);
  }
  void Paint() {
    PAINTSTRUCT paint{}; HDC target=BeginPaint(hwnd_,&paint);
    RECT client{}; GetClientRect(hwnd_,&client);
    if (client.right<=0 || client.bottom<=0) { EndPaint(hwnd_,&paint); return; }
    HDC dc=CreateCompatibleDC(target); auto bitmap=CreateCompatibleBitmap(target,client.right,client.bottom);
    auto old_bitmap=SelectObject(dc,bitmap); Fill(dc,client,Palette("background"));
    const int saved=SaveDC(dc); Transform(dc);
    RECT clip=Rect(0,Header(),width_,std::max(0.0,height_-Header()));
    IntersectClipRect(dc,clip.left,clip.top,clip.right,clip.bottom);
    Transform(dc,-scroll_x_,Header()-scroll_y_);
    const size_t start=RowAt(scroll_y_+std::max(0.0,paint.rcPaint.top/scale_-Header()));
    const size_t end=std::min(order_.size(),RowAt(scroll_y_+paint.rcPaint.bottom/scale_-Header())+1);
    for (size_t i=start;i<end;++i) {
      const auto& row=rows_.at(order_[i]); const double y=RowY(i),h=RowHeight(i); const bool selected=order_[i]==selected_;
      const auto fg=Palette(selected?"selectedText":"text"), muted=Palette(selected?"selectedMuted":"muted");
      if (selected) { Fill(dc,Rect(0,y,total_width_,h),Palette("selected")); Fill(dc,Rect(0,y,4,h),Palette("marker")); }
      Fill(dc,Rect(0,y+h-1,total_width_,1),Palette("line"));
      if (!console_) {
        auto c=Columns(); const double sx=c[0]+c[1]+12, tx=sx+c[2]+6;
        Badge(dc,row.Text("method"),8,y+16,c[0]-16,row.GetColor("methodColor"),selected);
        Text(dc,row.Text("path"),Rect(c[0]+18,y+8,c[1]-24,17),body_,fg);
        Text(dc,row.Text("meta"),Rect(c[0]+18,y+29,c[1]-24,15),small_,muted);
        Badge(dc,row.Text("status"),sx+8,y+16,c[2]-16,row.GetColor("statusColor"),selected);
        Text(dc,row.Text("duration"),Rect(tx+12,y+18,c[3]-24,17),small_,muted,DT_SINGLELINE|DT_END_ELLIPSIS|DT_RIGHT);
      } else {
        const double bx=BadgeWidth(dc,row.Text("level")),mx=MessageX(dc,row);
        Badge(dc,row.Text("level"),8,y+6,bx,row.GetColor("levelColor"),selected);
        Text(dc,row.Text("time"),Rect(8+bx+6,y+8,mx-bx-20,16),small_,muted);
        Text(dc,row.Text("message"),Rect(mx,y+6,std::max(16.0,total_width_-mx-8),h-12),body_,selected?fg:row.GetColor("messageColor"),DT_WORDBREAK|DT_EDITCONTROL);
      }
    }
    if (!console_) for (auto x:Edges()) Fill(dc,Rect(x+2,scroll_y_,1,height_),Palette("line"));
    RestoreDC(dc,saved);
    if (!console_) {
      int header_saved=SaveDC(dc); Transform(dc,-scroll_x_,0);
      IntersectClipRect(dc,static_cast<int>(scroll_x_),0,static_cast<int>(scroll_x_+width_),30);
      Fill(dc,Rect(scroll_x_,0,width_,30),Palette("surface"));
      auto c=Columns(); double x=0;
      for (size_t i=0;i<std::min<size_t>(4,titles_.size());++i) {
        Text(dc,titles_[i],Rect(x+(i==0||i==2?8:12),8,c[i]-20,16),small_,Palette("muted"),DT_SINGLELINE|DT_END_ELLIPSIS|(i==3?DT_RIGHT:DT_LEFT));
        x+=c[i]+6;
      }
      for (auto edge:Edges()) Fill(dc,Rect(edge+2,0,1,30),Palette("line"));
      RestoreDC(dc,header_saved);
    }
    BitBlt(target,paint.rcPaint.left,paint.rcPaint.top,paint.rcPaint.right-paint.rcPaint.left,paint.rcPaint.bottom-paint.rcPaint.top,
      dc,paint.rcPaint.left,paint.rcPaint.top,SRCCOPY);
    SelectObject(dc,old_bitmap); DeleteObject(bitmap); DeleteDC(dc); EndPaint(hwnd_,&paint);
  }
  void FinishResize() {
    if (divider_<0) return;
    divider_=-1;
    emit_("widthsChanged",{{Value("widths"),Value(Array{Value(widths_[0]),Value(widths_[1]),Value(widths_[2])})}});
  }
  void Key(WPARAM key) {
    if (order_.empty()) return;
    auto found=indices_.find(selected_);
    int next=found==indices_.end()?-1:static_cast<int>(found->second);
    if (key==VK_DOWN) ++next; else if (key==VK_UP) --next;
    else if (key==VK_HOME) next=0; else if (key==VK_END) next=static_cast<int>(order_.size())-1;
    else if (key==VK_NEXT) next+=std::max(1,static_cast<int>((height_-Header())/36));
    else if (key==VK_PRIOR) next-=std::max(1,static_cast<int>((height_-Header())/36)); else return;
    next=std::clamp(next,0,static_cast<int>(order_.size())-1); Select(order_[next],true);
    double y=scroll_y_;
    if (RowY(next)<y) y=RowY(next);
    if (RowY(next)+RowHeight(next)>y+height_-Header()) y=RowY(next)+RowHeight(next)-height_+Header();
    Scroll(scroll_x_,y);
  }
  static LRESULT CALLBACK WindowProc(HWND hwnd,UINT message,WPARAM wp,LPARAM lp) {
    auto self=reinterpret_cast<CaptureList*>(GetWindowLongPtr(hwnd,GWLP_USERDATA));
    if (message==WM_NCCREATE) {
      self=static_cast<CaptureList*>(reinterpret_cast<CREATESTRUCT*>(lp)->lpCreateParams);
      self->hwnd_=hwnd; SetWindowLongPtr(hwnd,GWLP_USERDATA,reinterpret_cast<LONG_PTR>(self));
    }
    if (!self) return DefWindowProc(hwnd,message,wp,lp);
    switch (message) {
      case WM_ERASEBKGND: return 1;
      case WM_PAINT: self->Paint(); return 0;
      case WM_SIZE: self->Layout(false); return 0;
      case WM_TIMER:
        if (wp==2) { KillTimer(hwnd,2); self->EmitScroll(); return 0; }
        if (wp==1) { KillTimer(hwnd,1); self->RebuildOffsets(); self->ClampScroll(); self->UpdateScrollbars(); InvalidateRect(hwnd,nullptr,FALSE); }
        return 0;
      case WM_VSCROLL: self->ScrollBar(SB_VERT,wp); return 0;
      case WM_HSCROLL: self->ScrollBar(SB_HORZ,wp); return 0;
      case WM_MOUSEWHEEL: {
        self->wheel_remainder_+=GET_WHEEL_DELTA_WPARAM(wp);
        int steps=self->wheel_remainder_/WHEEL_DELTA; self->wheel_remainder_%=WHEEL_DELTA;
        if (GET_KEYSTATE_WPARAM(wp)&MK_SHIFT) self->Scroll(self->scroll_x_-steps*72,self->scroll_y_);
        else self->Scroll(self->scroll_x_,self->scroll_y_-steps*108);
        return 0;
      }
      case WM_MOUSEHWHEEL: self->Scroll(self->scroll_x_+GET_WHEEL_DELTA_WPARAM(wp)*72.0/WHEEL_DELTA,self->scroll_y_); return 0;
      case WM_LBUTTONDOWN: {
        SetFocus(hwnd);
        double x=GET_X_LPARAM(lp)/self->scale_+self->scroll_x_,y=GET_Y_LPARAM(lp)/self->scale_;
        self->divider_=self->HitDivider(x); self->last_drag_x_=x;
        if (self->divider_>=0) { SetCapture(hwnd); return 0; }
        if (y>=self->Header()) {
          size_t i=self->RowAt(y-self->Header()+self->scroll_y_);
          if (i<self->order_.size()) self->Select(self->order_[i],true);
        }
        return 0;
      }
      case WM_MOUSEMOVE:
        if (self->divider_>=0) {
          double x=GET_X_LPARAM(lp)/self->scale_+self->scroll_x_;
          capture_list::Resize(self->widths_,self->divider_,x-self->last_drag_x_,self->total_width_);
          self->last_drag_x_=x; self->Layout(true);
        }
        return 0;
      case WM_LBUTTONUP: self->FinishResize(); if (GetCapture()==hwnd) ReleaseCapture(); return 0;
      case WM_CAPTURECHANGED: self->FinishResize(); return 0;
      case WM_SETCURSOR: {
        if (LOWORD(lp)!=HTCLIENT) break;
        POINT p{}; GetCursorPos(&p); ScreenToClient(hwnd,&p);
        SetCursor(LoadCursor(nullptr,self->HitDivider(p.x/self->scale_+self->scroll_x_)>=0?IDC_SIZEWE:IDC_ARROW)); return TRUE;
      }
      case WM_GETDLGCODE: return DLGC_WANTARROWS;
      case WM_KEYDOWN:
        if (wp==VK_TAB) { SetFocus(GetParent(hwnd)); SendMessage(GetParent(hwnd),message,wp,lp); return 0; }
        self->Key(wp); return 0;
      case WM_NCDESTROY: SetWindowLongPtr(hwnd,GWLP_USERDATA,0); self->hwnd_=nullptr; break;
    }
    return DefWindowProc(hwnd,message,wp,lp);
  }
};
}  // namespace

class NativeCaptureLists::Impl {
 public:
  Impl(flutter::BinaryMessenger* messenger,HWND parent) : parent_(parent),
    channel_(messenger,"dev.quocan.flutterNetworkViewer/native_lists",&flutter::StandardMethodCodec::GetInstance()) {
    channel_.SetMethodCallHandler([this](const auto& call,auto result) {
      if (call.method_name()=="capabilities") { result->Success(Value(1)); return; }
      const Map& args=call.arguments()?AsMap(*call.arguments()):empty_;
      const int64_t id=static_cast<int64_t>(Number(Field(args,"id"),-1));
      if (id<0) { result->Error("arguments","Missing native list ID"); return; }
      if (call.method_name()=="create") {
        lists_.erase(id);
        auto list=std::make_unique<CaptureList>(parent_,String(Field(args,"kind"))=="console",[this,id](const char* method,Map fields) {
          fields[Value("id")]=Value(id); channel_.InvokeMethod(method,std::make_unique<Value>(std::move(fields)));
        });
        if (!list->valid()) { result->Error("native-window","Could not create native list window"); return; }
        lists_[id]=std::move(list);
      } else if (call.method_name()=="dispose") lists_.erase(id);
      else if (lists_.find(id)!=lists_.end()) {
        auto& list=*lists_.at(id);
        if (call.method_name()=="configure") list.Configure(args);
        else if (call.method_name()=="frame") list.Frame(args);
        else if (call.method_name()=="update") list.Update(args);
        else if (call.method_name()=="selection") list.Select(String(Field(args,"key")),false);
        else { result->NotImplemented(); return; }
      }
      result->Success();
    });
  }
  ~Impl() { channel_.SetMethodCallHandler(nullptr); lists_.clear(); }
 private:
  HWND parent_;
  Channel channel_;
  Map empty_;
  std::unordered_map<int64_t,std::unique_ptr<CaptureList>> lists_;
};
NativeCaptureLists::NativeCaptureLists(flutter::BinaryMessenger* messenger,HWND parent)
  : impl_(std::make_unique<Impl>(messenger,parent)) {}
NativeCaptureLists::~NativeCaptureLists() = default;
