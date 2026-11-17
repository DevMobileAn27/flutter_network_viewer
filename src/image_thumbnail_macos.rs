//! ImageIO creates the thumbnail before raster allocation. The Rust decoder's
//! full-source allocation guard remains in force on platforms without ImageIO.
use crate::image_body::ImagePixels;
use std::ffi::{c_char, c_void};
type Ref = *const c_void;
#[repr(C)]
struct Point {
    x: f64,
    y: f64,
}
#[repr(C)]
struct Size {
    width: f64,
    height: f64,
}
#[repr(C)]
struct Rect {
    origin: Point,
    size: Size,
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFDataCreate(allocator: Ref, bytes: *const u8, length: isize) -> Ref;
    fn CFStringCreateWithCString(allocator: Ref, text: *const c_char, encoding: u32) -> Ref;
    fn CFNumberCreate(allocator: Ref, kind: isize, value: Ref) -> Ref;
    fn CFDictionaryCreate(
        allocator: Ref,
        keys: *const Ref,
        values: *const Ref,
        count: isize,
        keys_cb: Ref,
        values_cb: Ref,
    ) -> Ref;
    fn CFRelease(value: Ref);
    static kCFBooleanTrue: Ref;
    static kCFBooleanFalse: Ref;
}
#[link(name = "ImageIO", kind = "framework")]
unsafe extern "C" {
    fn CGImageSourceCreateWithData(data: Ref, options: Ref) -> Ref;
    fn CGImageSourceCreateThumbnailAtIndex(source: Ref, index: usize, options: Ref) -> Ref;
}
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGImageGetWidth(image: Ref) -> usize;
    fn CGImageGetHeight(image: Ref) -> usize;
    fn CGColorSpaceCreateDeviceRGB() -> Ref;
    fn CGBitmapContextCreate(
        data: *mut c_void,
        width: usize,
        height: usize,
        bits: usize,
        stride: usize,
        space: Ref,
        bitmap: u32,
    ) -> Ref;
    fn CGContextTranslateCTM(context: Ref, x: f64, y: f64);
    fn CGContextScaleCTM(context: Ref, x: f64, y: f64);
    fn CGContextDrawImage(context: Ref, rect: Rect, image: Ref);
}
struct Owned(Ref);
impl Drop for Owned {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CFRelease(self.0);
            }
        }
    }
}
fn owned(value: Ref) -> Result<Owned, String> {
    if value.is_null() {
        Err("ImageIO thumbnail unavailable".into())
    } else {
        Ok(Owned(value))
    }
}
pub fn thumbnail(bytes: &[u8]) -> Result<ImagePixels, String> {
    // Every object stays alive until all consumers of its raw reference finish.
    unsafe {
        let data = owned(CFDataCreate(
            std::ptr::null(),
            bytes.as_ptr(),
            bytes.len() as isize,
        ))?;
        let names = [
            c"kCGImageSourceCreateThumbnailFromImageAlways",
            c"kCGImageSourceCreateThumbnailWithTransform",
            c"kCGImageSourceThumbnailMaxPixelSize",
            c"kCGImageSourceShouldCache",
        ];
        // These option constants' values are the public CFString names.
        let keys: Vec<_> = names
            .iter()
            .map(|s| {
                owned(CFStringCreateWithCString(
                    std::ptr::null(),
                    s.as_ptr(),
                    0x08000100,
                ))
            })
            .collect::<Result<_, _>>()?;
        let max: i32 = 4096;
        let number = owned(CFNumberCreate(
            std::ptr::null(),
            3,
            (&max as *const i32).cast(),
        ))?;
        let key_refs: Vec<_> = keys.iter().map(|s| s.0).collect();
        let values = [kCFBooleanTrue, kCFBooleanTrue, number.0, kCFBooleanFalse];
        let options = owned(CFDictionaryCreate(
            std::ptr::null(),
            key_refs.as_ptr(),
            values.as_ptr(),
            4,
            std::ptr::null(),
            std::ptr::null(),
        ))?;
        let source = owned(CGImageSourceCreateWithData(data.0, options.0))?;
        let image = owned(CGImageSourceCreateThumbnailAtIndex(source.0, 0, options.0))?;
        let width = CGImageGetWidth(image.0);
        let height = CGImageGetHeight(image.0);
        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Err("ImageIO thumbnail exceeds pixel budget".into());
        }
        let mut rgba = vec![0u8; width * height * 4];
        let space = owned(CGColorSpaceCreateDeviceRGB())?;
        let context = owned(CGBitmapContextCreate(
            rgba.as_mut_ptr().cast(),
            width,
            height,
            8,
            width * 4,
            space.0,
            1 | (4 << 12),
        ))?;
        CGContextTranslateCTM(context.0, 0., height as f64);
        CGContextScaleCTM(context.0, 1., -1.);
        CGContextDrawImage(
            context.0,
            Rect {
                origin: Point { x: 0., y: 0. },
                size: Size {
                    width: width as f64,
                    height: height as f64,
                },
            },
            image.0,
        );
        for pixel in rgba.as_chunks_mut::<4>().0 {
            let a = u32::from(pixel[3]);
            if a > 0 && a < 255 {
                for c in &mut pixel[..3] {
                    *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
        Ok(ImagePixels {
            width: width as u32,
            height: height as u32,
            rgba,
        })
    }
}
