#include "../windows/runner/native_image_decoder.h"
#include <cstring>
#include <fstream>
#include <iostream>
#include <iterator>
#include <stdexcept>

void Check(bool value, const char* message) { if (!value) throw std::runtime_error(message); }
std::vector<uint8_t> Read(const char* path) {
  std::ifstream file(path,std::ios::binary);
  Check(static_cast<bool>(file),"Missing image fixture");
  return {std::istreambuf_iterator<char>(file),std::istreambuf_iterator<char>()};
}
int main() {
  const HRESULT com = CoInitializeEx(nullptr,COINIT_MULTITHREADED);
  if (FAILED(com)) return 1;
  try {
    auto png = native_image::Decode(Read("test/fixtures/preview.png"));
    auto jpeg = native_image::Decode(Read("test/fixtures/preview.jpg"));
    Check(png && jpeg,"WIC must decode PNG and JPEG");
    Check(!native_image::Decode({0xff,0xd8,0xff}),"Corrupt images must fail safely");
    Check(!native_image::Decode({}),"Empty bytes must fail safely");
    // 6000 x 120 BMP; destination must be at most 4096px wide.
    BITMAPFILEHEADER file{}; BITMAPINFOHEADER header{};
    header.biSize = sizeof(header); header.biWidth = 6000; header.biHeight = 120;
    header.biPlanes = 1; header.biBitCount = 32; header.biCompression = BI_RGB;
    header.biSizeImage = 6000*120*4;
    file.bfType = 0x4d42; file.bfOffBits = sizeof(file)+sizeof(header);
    file.bfSize = file.bfOffBits+header.biSizeImage;
    std::vector<uint8_t> bmp(file.bfSize,255);
    std::memcpy(bmp.data(),&file,sizeof(file));
    std::memcpy(bmp.data()+sizeof(file),&header,sizeof(header));
    auto capped = native_image::Decode(bmp);
    Check(capped && capped->width == 4096 && capped->height < 120,"Large images must be downsampled");
    std::cout << "WIC native image tests passed\n";
  } catch (const std::exception& error) {
    std::cerr << error.what() << '\n'; CoUninitialize(); return 1;
  }
  CoUninitialize(); return 0;
}
