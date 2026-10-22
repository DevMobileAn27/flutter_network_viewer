#ifndef RUNNER_NATIVE_IMAGE_DECODER_H_
#define RUNNER_NATIVE_IMAGE_DECODER_H_
#include <windows.h>
#include <wincodec.h>
#include <wrl/client.h>
#include <algorithm>
#include <cstdint>
#include <memory>
#include <vector>
namespace native_image {
using Microsoft::WRL::ComPtr;
struct Bitmap {
  HBITMAP handle = nullptr;
  UINT width = 0, height = 0;
  ~Bitmap() { if (handle) DeleteObject(handle); }
};
// WIC decodes the first frame, capped at 4096px / 64 MiB, on the COM worker.
inline std::shared_ptr<Bitmap> Decode(const std::vector<uint8_t>& bytes) {
  if (bytes.empty() || bytes.size() > MAXDWORD) return {};
  ComPtr<IWICImagingFactory> factory;
  if (FAILED(CoCreateInstance(CLSID_WICImagingFactory,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&factory)))) return {};
  ComPtr<IWICStream> stream;
  if (FAILED(factory->CreateStream(&stream)) || FAILED(stream->InitializeFromMemory(
    const_cast<BYTE*>(bytes.data()),static_cast<DWORD>(bytes.size())))) return {};
  ComPtr<IWICBitmapDecoder> decoder;
  ComPtr<IWICBitmapFrameDecode> frame;
  if (FAILED(factory->CreateDecoderFromStream(stream.Get(),nullptr,WICDecodeMetadataCacheOnDemand,&decoder)) ||
      FAILED(decoder->GetFrame(0,&frame))) return {};
  UINT width = 0, height = 0;
  if (FAILED(frame->GetSize(&width,&height)) || width == 0 || height == 0) return {};
  const double ratio = std::min(1.0,4096.0/std::max(width,height));
  const UINT w = std::max(1u,static_cast<UINT>(width*ratio));
  const UINT h = std::max(1u,static_cast<UINT>(height*ratio));
  ComPtr<IWICBitmapScaler> scaler;
  if (FAILED(factory->CreateBitmapScaler(&scaler)) ||
      FAILED(scaler->Initialize(frame.Get(),w,h,WICBitmapInterpolationModeFant))) return {};
  ComPtr<IWICFormatConverter> converter;
  if (FAILED(factory->CreateFormatConverter(&converter)) ||
      FAILED(converter->Initialize(scaler.Get(),GUID_WICPixelFormat32bppPBGRA,
        WICBitmapDitherTypeNone,nullptr,0,WICBitmapPaletteTypeCustom))) return {};
  // Apply EXIF orientation after scaling, so rotation cannot allocate the
  // original full-resolution image.
  ComPtr<IWICMetadataQueryReader> metadata;
  WICBitmapTransformOptions transform = WICBitmapTransformRotate0;
  if (SUCCEEDED(frame->GetMetadataQueryReader(&metadata))) {
    PROPVARIANT orientation; PropVariantInit(&orientation);
    if (SUCCEEDED(metadata->GetMetadataByName(L"/app1/ifd/{ushort=274}",&orientation)) && orientation.vt == VT_UI2) {
      switch (orientation.uiVal) {
        case 2: transform = WICBitmapTransformFlipHorizontal; break;
        case 3: transform = WICBitmapTransformRotate180; break;
        case 4: transform = WICBitmapTransformFlipVertical; break;
        case 5: transform = static_cast<WICBitmapTransformOptions>(WICBitmapTransformRotate90|WICBitmapTransformFlipHorizontal); break;
        case 6: transform = WICBitmapTransformRotate90; break;
        case 7: transform = static_cast<WICBitmapTransformOptions>(WICBitmapTransformRotate270|WICBitmapTransformFlipHorizontal); break;
        case 8: transform = WICBitmapTransformRotate270; break;
      }
    }
    PropVariantClear(&orientation);
  }
  ComPtr<IWICBitmapFlipRotator> rotated;
  if (FAILED(factory->CreateBitmapFlipRotator(&rotated)) || FAILED(rotated->Initialize(converter.Get(),transform))) return {};
  auto bitmap = std::make_shared<Bitmap>();
  if (FAILED(rotated->GetSize(&bitmap->width,&bitmap->height))) return {};
  BITMAPINFO info{}; info.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
  info.bmiHeader.biWidth = static_cast<LONG>(bitmap->width);
  info.bmiHeader.biHeight = -static_cast<LONG>(bitmap->height);
  info.bmiHeader.biPlanes = 1; info.bmiHeader.biBitCount = 32; info.bmiHeader.biCompression = BI_RGB;
  void* pixels = nullptr;
  bitmap->handle = CreateDIBSection(nullptr,&info,DIB_RGB_COLORS,&pixels,nullptr,0);
  if (!bitmap->handle || !pixels || FAILED(rotated->CopyPixels(nullptr,bitmap->width*4,
      bitmap->width*bitmap->height*4,static_cast<BYTE*>(pixels)))) return {};
  return bitmap;
}
}  // namespace native_image
#endif
