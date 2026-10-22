#include "native_image_viewer.h"
#include "native_image_decoder.h"
#include <flutter/encodable_value.h>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
#include <wincodec.h>
#include <wrl/client.h>
#include <windowsx.h>
#include <algorithm>
#include <array>
#include <atomic>
#include <cmath>
#include <condition_variable>
#include <cstdint>
#include <cstring>
#include <deque>
#include <functional>
#include <map>
#include <mutex>
#include <string>
#include <thread>
#include <utility>
#include <vector>

namespace {
using Value = flutter::EncodableValue;
using Map = flutter::EncodableMap;
using Channel = flutter::MethodChannel<Value>;
using Microsoft::WRL::ComPtr;
const Value& Field(const Map& map, const char* key) {
  static const Value empty;
  const auto found = map.find(Value(key)); return found == map.end() ? empty : found->second;
}
const Map& AsMap(const Value& value) {
  static const Map empty; const auto p = std::get_if<Map>(&value); return p ? *p : empty;
}
double Number(const Value& value, double fallback = 0) {
  if (const auto p = std::get_if<double>(&value)) return std::isfinite(*p) ? *p : fallback;
  if (const auto p = std::get_if<int32_t>(&value)) return *p;
  if (const auto p = std::get_if<int64_t>(&value)) return static_cast<double>(*p);
  return fallback;
}
bool Boolean(const Value& value) { const auto p = std::get_if<bool>(&value); return p && *p; }
std::wstring Wide(const Value& value) {
  const auto p = std::get_if<std::string>(&value); if (!p || p->empty()) return {};
  const int n = MultiByteToWideChar(CP_UTF8,0,p->data(),static_cast<int>(p->size()),nullptr,0);
  std::wstring result(n,L'\0'); MultiByteToWideChar(CP_UTF8,0,p->data(),static_cast<int>(p->size()),result.data(),n); return result;
}
using native_image::Bitmap;
using native_image::Decode;
struct Asset {
  std::shared_ptr<Bitmap> bitmap;
  std::atomic<bool> complete{false};
};
class ImageHost {
 public:
  using Emit = std::function<void(const char*)>;
  ImageHost(HWND parent, Emit emit) : emit_(std::move(emit)) {
    static const bool registered = [] {
      WNDCLASSW wc{}; wc.style = CS_DBLCLKS; wc.lpfnWndProc = WindowProc;
      wc.hInstance = GetModuleHandle(nullptr); wc.hCursor = LoadCursor(nullptr,IDC_HAND);
      wc.lpszClassName = L"FlutterViewerNativeImage"; return RegisterClassW(&wc) != 0;
    }();
    if (registered) CreateWindowExW(0,L"FlutterViewerNativeImage",L"Image preview",
      WS_CHILD|WS_CLIPCHILDREN|WS_CLIPSIBLINGS|WS_TABSTOP,0,0,1,1,parent,nullptr,GetModuleHandle(nullptr),this);
    if (hwnd_) {
      for (int i = 0; i < 4; ++i) buttons_[i] = CreateWindowExW(0,L"BUTTON",L"",
        WS_CHILD|WS_TABSTOP|BS_OWNERDRAW,0,0,1,1,hwnd_,reinterpret_cast<HMENU>(static_cast<INT_PTR>(i+1)),GetModuleHandle(nullptr),nullptr);
    }
  }
  ~ImageHost() { if (hwnd_) DestroyWindow(hwnd_); }
  bool valid() const { return hwnd_ != nullptr; }
  void Bind(std::shared_ptr<Asset> asset) { asset_ = std::move(asset); zoom_ = 1; x_ = y_ = 0; error_ = false; Changed(); }
  void Changed() {
    if (asset_ && asset_->complete.load() && !asset_->bitmap && !error_) { error_ = true; emit_("error"); }
    InvalidateRect(hwnd_,nullptr,FALSE);
  }
  void Configure(const Map& args) {
    interactive_ = Boolean(Field(args,"interactive"));
    const auto n = static_cast<uint32_t>(Number(Field(args,"background")));
    background_ = RGB((n>>16)&255,(n>>8)&255,n&255);
    SetWindowTextW(hwnd_,Wide(Field(args,"label")).c_str());
    const auto surface = static_cast<uint32_t>(Number(Field(args,"surface"),0xfffafafa));
    surface_ = RGB((surface>>16)&255,(surface>>8)&255,surface&255);
    const auto text = static_cast<uint32_t>(Number(Field(args,"text"),0xff222222));
    text_ = RGB((text>>16)&255,(text>>8)&255,text&255);
    if (const auto labels = std::get_if<flutter::EncodableList>(&Field(args,"tools"))) {
      for (size_t i = 0; i < buttons_.size() && i < labels->size(); ++i)
        SetWindowTextW(buttons_[i],Wide((*labels)[i]).c_str());
    }
    LayoutTools(); Changed();
  }
  void Control(double factor) {
    if (factor <= 0 || !std::isfinite(factor)) { zoom_ = 1; x_ = y_ = 0; }
    else { const double next = std::clamp(zoom_*factor,0.1,10.0); x_ *= next/zoom_; y_ *= next/zoom_; zoom_ = next; }
    InvalidateRect(hwnd_,nullptr,FALSE);
  }
  void Frame(const Map& args) {
    scale_ = std::max(0.5,Number(Field(args,"scale"),1));
    const auto px = [this](double value) { return static_cast<int>(std::round(value*scale_)); };
    SetWindowPos(hwnd_,HWND_TOP,px(Number(Field(args,"x"))),px(Number(Field(args,"y"))),
      px(Number(Field(args,"width"))),px(Number(Field(args,"height"))),SWP_NOACTIVATE);
    const int x = px(Number(Field(args,"clipX"))), y = px(Number(Field(args,"clipY")));
    HRGN clip = CreateRectRgn(x,y,x+px(Number(Field(args,"clipWidth"))),y+px(Number(Field(args,"clipHeight"))));
    if (!SetWindowRgn(hwnd_,clip,TRUE)) DeleteObject(clip);
    ShowWindow(hwnd_,Boolean(Field(args,"visible")) ? SW_SHOWNOACTIVATE : SW_HIDE);
    LayoutTools();
  }
 private:
  HWND hwnd_ = nullptr;
  Emit emit_;
  std::shared_ptr<Asset> asset_;
  bool interactive_ = false, dragging_ = false, error_ = false;
  double scale_ = 1, zoom_ = 1, x_ = 0, y_ = 0;
  POINT drag_{};
  COLORREF background_ = RGB(245,245,245), surface_ = RGB(250,250,250), text_ = RGB(34,34,34);
  std::array<HWND,4> buttons_{};
  RECT toolbar_{};
  void LayoutTools() {
    RECT r; GetClientRect(hwnd_,&r);
    const int margin = static_cast<int>(16*scale_);
    const int width = std::min(static_cast<int>(184*scale_),std::max(0,r.right-margin*2));
    const int height = static_cast<int>(40*scale_);
    toolbar_ = {std::max(0,r.right-margin-width),std::max(0,r.bottom-margin-height),r.right-margin,r.bottom-margin};
    for (int i = 0; i < 4; ++i) {
      const int x = toolbar_.left+width*i/4, next = toolbar_.left+width*(i+1)/4;
      SetWindowPos(buttons_[i],HWND_TOP,x,toolbar_.top,next-x,height,SWP_NOACTIVATE);
      ShowWindow(buttons_[i],interactive_ ? SW_SHOWNOACTIVATE : SW_HIDE);
    }
  }
  void DrawTool(const DRAWITEMSTRUCT& item) {
    const HBRUSH brush = CreateSolidBrush(surface_); FillRect(item.hDC,&item.rcItem,brush); DeleteObject(brush);
    const int cx = (item.rcItem.right+item.rcItem.left)/2, cy = (item.rcItem.bottom+item.rcItem.top)/2;
    const int a = std::max(4,static_cast<int>(6*scale_));
    const HPEN pen = CreatePen(PS_SOLID,std::max(1,static_cast<int>(1.5*scale_)),text_);
    const auto old = SelectObject(item.hDC,pen);
    const auto line = [&](int x1,int y1,int x2,int y2) { MoveToEx(item.hDC,x1,y1,nullptr); LineTo(item.hDC,x2,y2); };
    if (item.CtlID == 1 || item.CtlID == 2) {
      line(cx-a,cy,cx+a,cy); if (item.CtlID == 2) line(cx,cy-a,cx,cy+a);
    } else if (item.CtlID == 3) {
      for (int x : {-1,1}) for (int y : {-1,1}) {
        line(cx+x*a,cy+y*a,cx+x*a,cy+y*a/2); line(cx+x*a,cy+y*a,cx+x*a/2,cy+y*a);
      }
    } else { line(cx-a,cy-a,cx+a,cy+a); line(cx-a,cy+a,cx+a,cy-a); }
    SelectObject(item.hDC,old); DeleteObject(pen);
    if (item.itemState&ODS_FOCUS) { RECT focus = item.rcItem; InflateRect(&focus,-3,-3); DrawFocusRect(item.hDC,&focus); }
  }
  void Paint(HDC target) {
    RECT r; GetClientRect(hwnd_,&r);
    const int w = r.right, h = r.bottom; if (w <= 0 || h <= 0) return;
    HDC dc = CreateCompatibleDC(target); HBITMAP back = CreateCompatibleBitmap(target,w,h);
    if (!dc || !back) { if (dc) DeleteDC(dc); if (back) DeleteObject(back); return; }
    const auto previous = SelectObject(dc,back);
    const auto brush = CreateSolidBrush(background_); FillRect(dc,&r,brush); DeleteObject(brush);
    if (asset_ && asset_->complete.load() && asset_->bitmap) {
      const auto& bitmap = *asset_->bitmap;
      const double fit = std::min(static_cast<double>(w)/bitmap.width,static_cast<double>(h)/bitmap.height)*zoom_;
      const int dw = std::max(1,static_cast<int>(std::round(bitmap.width*fit)));
      const int dh = std::max(1,static_cast<int>(std::round(bitmap.height*fit)));
      HDC source = CreateCompatibleDC(dc); const auto old = SelectObject(source,bitmap.handle);
      BLENDFUNCTION blend{AC_SRC_OVER,0,255,AC_SRC_ALPHA};
      AlphaBlend(dc,(w-dw)/2+static_cast<int>(x_),(h-dh)/2+static_cast<int>(y_),dw,dh,
        source,0,0,static_cast<int>(bitmap.width),static_cast<int>(bitmap.height),blend);
      SelectObject(source,old); DeleteDC(source);
    }
    if (interactive_) {
      const auto panelBrush = CreateSolidBrush(surface_); const auto oldBrush = SelectObject(dc,panelBrush);
      const auto oldPen = SelectObject(dc,GetStockObject(NULL_PEN));
      RoundRect(dc,toolbar_.left-2,toolbar_.top-2,toolbar_.right+2,toolbar_.bottom+2,
        static_cast<int>(12*scale_),static_cast<int>(12*scale_));
      SelectObject(dc,oldPen); SelectObject(dc,oldBrush); DeleteObject(panelBrush);
    }
    BitBlt(target,0,0,w,h,dc,0,0,SRCCOPY);
    SelectObject(dc,previous); DeleteObject(back); DeleteDC(dc);
  }
  static LRESULT CALLBACK WindowProc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp) {
    auto self = reinterpret_cast<ImageHost*>(GetWindowLongPtr(hwnd,GWLP_USERDATA));
    if (msg == WM_NCCREATE) {
      self = static_cast<ImageHost*>(reinterpret_cast<CREATESTRUCT*>(lp)->lpCreateParams);
      self->hwnd_ = hwnd; SetWindowLongPtr(hwnd,GWLP_USERDATA,reinterpret_cast<LONG_PTR>(self));
    }
    if (!self) return DefWindowProc(hwnd,msg,wp,lp);
    switch (msg) {
      case WM_ERASEBKGND: return 1;
      case WM_PAINT: { PAINTSTRUCT ps; HDC dc = BeginPaint(hwnd,&ps); self->Paint(dc); EndPaint(hwnd,&ps); return 0; }
      case WM_SIZE: self->LayoutTools(); InvalidateRect(hwnd,nullptr,FALSE); return 0;
      case WM_DRAWITEM: self->DrawTool(*reinterpret_cast<DRAWITEMSTRUCT*>(lp)); return TRUE;
      case WM_COMMAND:
        if (HIWORD(wp) == BN_CLICKED) {
          switch (LOWORD(wp)) {
            case 1: self->Control(0.8); break;
            case 2: self->Control(1.25); break;
            case 3: self->Control(0); break;
            case 4: self->emit_("close"); break;
          }
        } return 0;
      case WM_LBUTTONDOWN:
        SetFocus(hwnd); self->drag_ = {GET_X_LPARAM(lp),GET_Y_LPARAM(lp)};
        if (self->interactive_) { self->dragging_ = true; SetCapture(hwnd); } return 0;
      case WM_MOUSEMOVE:
        if (self->dragging_) {
          const POINT next{GET_X_LPARAM(lp),GET_Y_LPARAM(lp)};
          self->x_ += next.x-self->drag_.x; self->y_ += next.y-self->drag_.y;
          self->drag_ = next; InvalidateRect(hwnd,nullptr,FALSE);
        } return 0;
      case WM_LBUTTONUP:
        self->dragging_ = false; ReleaseCapture(); if (!self->interactive_) self->emit_("open"); return 0;
      case WM_CAPTURECHANGED: self->dragging_ = false; return 0;
      case WM_MOUSEWHEEL:
        if (self->interactive_) {
          const double delta = GET_WHEEL_DELTA_WPARAM(wp)/static_cast<double>(WHEEL_DELTA);
          if (GET_KEYSTATE_WPARAM(wp)&MK_CONTROL) self->Control(std::pow(1.25,delta));
          else { self->y_ += delta*40*self->scale_; InvalidateRect(hwnd,nullptr,FALSE); }
        } return 0;
      case WM_KEYDOWN:
        if (self->interactive_) {
          if (wp == VK_OEM_PLUS || wp == VK_ADD) self->Control(1.25);
          else if (wp == VK_OEM_MINUS || wp == VK_SUBTRACT) self->Control(0.8);
          else if (wp == '0') self->Control(0);
        } return 0;
      case WM_NCDESTROY: self->hwnd_ = nullptr; SetWindowLongPtr(hwnd,GWLP_USERDATA,0); break;
    }
    return DefWindowProc(hwnd,msg,wp,lp);
  }
};
}

class NativeImageViewer::Impl {
 public:
  Impl(flutter::BinaryMessenger* messenger, HWND parent) : parent_(parent),
    channel_(messenger,"dev.quocan.flutterNetworkViewer/native_image",&flutter::StandardMethodCodec::GetInstance()) {
    WNDCLASSW wc{}; wc.lpfnWndProc = Dispatch; wc.hInstance = GetModuleHandle(nullptr);
    wc.lpszClassName = L"FlutterViewerImageDecode"; RegisterClassW(&wc);
    dispatch_ = CreateWindowExW(0,wc.lpszClassName,L"",0,0,0,0,0,HWND_MESSAGE,nullptr,wc.hInstance,this);
    worker_ = std::thread([this] { Work(); });
    channel_.SetMethodCallHandler([this](const auto& call, auto result) {
      if (call.method_name() == "capabilities") { result->Success(Value(dispatch_ ? 1 : 0)); return; }
      if (!call.arguments()) { result->Error("arguments","Missing arguments"); return; }
      const auto& args = AsMap(*call.arguments()); const auto id = static_cast<int64_t>(Number(Field(args,"id")));
      const auto& method = call.method_name();
      if (method == "load") {
        const auto bytes = std::get_if<std::vector<uint8_t>>(&Field(args,"bytes"));
        if (!bytes) { result->Error("arguments","Missing image bytes"); return; }
        auto asset = std::make_shared<Asset>(); assets_[id] = asset;
        { std::lock_guard<std::mutex> lock(mutex_); jobs_.push_back({asset,*bytes}); }
        wake_.notify_one();
      } else if (method == "release") {
        const auto found = assets_.find(id);
        if (found != assets_.end()) {
          const auto asset = found->second;
          { std::lock_guard<std::mutex> lock(mutex_);
            jobs_.erase(std::remove_if(jobs_.begin(),jobs_.end(),[&asset](const Job& job) {
              return job.asset.lock() == asset;
            }),jobs_.end());
          }
          assets_.erase(found);
        }
      } else if (method == "create" || method == "bind") {
        const auto found = assets_.find(static_cast<int64_t>(Number(Field(args,"image"))));
        if (found == assets_.end()) { result->Error("image","Image resource has been released"); return; }
        if (method == "create") {
          auto host = std::make_unique<ImageHost>(parent_,[this,id](const char* name) {
            channel_.InvokeMethod(name,std::make_unique<Value>(Map{{Value("id"),Value(id)}}));
          });
          if (!host->valid()) { result->Error("create","Could not create native image window"); return; }
          hosts_[id] = std::move(host);
        }
        const auto host = hosts_.find(id); if (host != hosts_.end()) host->second->Bind(found->second);
      } else if (method == "dispose") hosts_.erase(id);
      else if (const auto found = hosts_.find(id); found != hosts_.end()) {
        if (method == "configure") found->second->Configure(args);
        else if (method == "control") found->second->Control(Number(Field(args,"factor")));
        else if (method == "frame") found->second->Frame(args);
        else { result->NotImplemented(); return; }
      }
      result->Success();
    });
  }
  ~Impl() {
    channel_.SetMethodCallHandler(nullptr);
    { std::lock_guard<std::mutex> lock(mutex_); stop_ = true; jobs_.clear(); }
    wake_.notify_one(); worker_.join();
    if (dispatch_) DestroyWindow(dispatch_);
  }
 private:
  struct Job { std::weak_ptr<Asset> asset; std::vector<uint8_t> bytes; };
  HWND parent_, dispatch_ = nullptr;
  Channel channel_;
  std::map<int64_t,std::shared_ptr<Asset>> assets_;
  std::map<int64_t,std::unique_ptr<ImageHost>> hosts_;
  std::mutex mutex_; std::condition_variable wake_; std::deque<Job> jobs_;
  bool stop_ = false; std::thread worker_;
  void Work() {
    const HRESULT com = CoInitializeEx(nullptr,COINIT_MULTITHREADED);
    for (;;) {
      Job job;
      { std::unique_lock<std::mutex> lock(mutex_);
        wake_.wait(lock,[this] { return stop_ || !jobs_.empty(); });
        if (stop_) break;
        job = std::move(jobs_.front()); jobs_.pop_front();
      }
      if (job.asset.expired()) continue;
      auto bitmap = SUCCEEDED(com) ? Decode(job.bytes) : nullptr;
      if (const auto asset = job.asset.lock()) {
        asset->bitmap = std::move(bitmap); asset->complete.store(true);
        PostMessage(dispatch_,WM_APP+17,0,0);
      }
    }
    if (SUCCEEDED(com)) CoUninitialize();
  }
  static LRESULT CALLBACK Dispatch(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp) {
    auto self = reinterpret_cast<Impl*>(GetWindowLongPtr(hwnd,GWLP_USERDATA));
    if (msg == WM_NCCREATE) {
      self = static_cast<Impl*>(reinterpret_cast<CREATESTRUCT*>(lp)->lpCreateParams);
      SetWindowLongPtr(hwnd,GWLP_USERDATA,reinterpret_cast<LONG_PTR>(self));
    }
    if (self && msg == WM_APP+17) {
      for (const auto& host : self->hosts_) host.second->Changed();
      return 0;
    }
    return DefWindowProc(hwnd,msg,wp,lp);
  }
};
NativeImageViewer::NativeImageViewer(flutter::BinaryMessenger* messenger, HWND parent) : impl_(std::make_unique<Impl>(messenger,parent)) {}
NativeImageViewer::~NativeImageViewer() = default;
