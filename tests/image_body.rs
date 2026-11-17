use flutter_network_viewer::image_body::{
    ImageAsset, decode_image_body, image_body_from_capture, may_contain_image_body,
};
use serde_json::json;
use std::{io::Cursor, sync::Arc};
const PNG: &[u8] = include_bytes!("fixtures/preview.png");
const JPEG: &[u8] = include_bytes!("fixtures/preview.jpg");
fn b64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;
        for i in 0..4 {
            out.push(if i > chunk.len() {
                '='
            } else {
                ALPHABET[((n >> (18 - i * 6)) & 63) as usize] as char
            });
        }
    }
    out
}
#[test]
fn missing_mime_detects_png_without_transport_marker() {
    let data = json!({"body": b64(PNG)});
    assert!(may_contain_image_body(&data));
    let result = decode_image_body(&data);
    assert!(result.image);
    assert_eq!(&*result.bytes.unwrap(), PNG);
}
#[test]
fn transport_base64_and_case_insensitive_mime() {
    for headers in [
        json!({}),
        json!({"CONTENT-TYPE":["IMAGE/JPEG; charset=binary"]}),
    ] {
        let data = json!({"headers":headers,"body":b64(JPEG),"bodyEncoding":"base64"});
        assert_eq!(&*image_body_from_capture(&data).bytes.unwrap(), JPEG);
    }
}
#[test]
fn wrapped_and_url_safe_unpadded_images() {
    for body in [
        b64(PNG)
            .as_bytes()
            .chunks(40)
            .map(|c| format!("{}\r\n", std::str::from_utf8(c).unwrap()))
            .collect::<String>(),
        b64(JPEG)
            .replace('+', "-")
            .replace('/', "_")
            .trim_end_matches('=')
            .to_owned(),
    ] {
        assert!(decode_image_body(&json!({"body":body})).image);
    }
}
#[test]
fn json_string_and_nested_transport_are_normalized() {
    let encoded = b64(PNG);
    for data in [
        json!({"body":serde_json::to_string(&encoded).unwrap()}),
        json!({"body":b64(encoded.as_bytes()),"base64":true}),
    ] {
        assert_eq!(&*decode_image_body(&data).bytes.unwrap(), PNG);
    }
}
#[test]
fn data_uri_supports_base64_percent_and_json_wrapping() {
    for body in [
        format!("data:image/png;base64,{}", b64(PNG)),
        "data:image/svg+xml,%3Csvg%3E%2B%3C/svg%3E".into(),
        serde_json::to_string(&format!("data:image/png;base64,{}", b64(PNG))).unwrap(),
    ] {
        let result = decode_image_body(&json!({"body":body}));
        assert!(result.image);
        assert!(result.bytes.is_some());
    }
    assert_eq!(
        &*decode_image_body(&json!({"body":"data:image/svg+xml,%3Csvg%3E+%3C/svg%3E"}))
            .bytes
            .unwrap(),
        b"<svg>+</svg>"
    );
}
#[test]
fn ordinary_json_text_and_binary_remain_raw() {
    for data in [
        json!({"body":"{\"ok\":true}"}),
        json!({"body":b64(b"ordinary text")}),
        json!({"body":b64(&[1,2,3,4]),"base64":true}),
        json!({"body":"iVBORw0KGgo%%%invalid"}),
    ] {
        let result = decode_image_body(&data);
        assert!(!result.image);
        assert!(result.bytes.is_none());
    }
}
#[test]
fn malformed_base64_and_data_uri_preserve_image_intent() {
    for body in [
        "%%%invalid",
        "data:image/png;base64,%%%",
        "data:image/png,%ZZ",
    ] {
        let result =
            decode_image_body(&json!({"body":body,"base64":body=="%%%invalid","image":true}));
        assert!(result.image);
        assert!(result.bytes.is_none());
    }
}
#[test]
fn marked_non_base64_body_and_invalid_utf8_keep_captured_bytes() {
    let result = decode_image_body(&json!({"body":" <svg/> ","image":true}));
    assert_eq!(&*result.bytes.unwrap(), b"<svg/>");
    let result = decode_image_body(&json!({"body":"/w==","image":true,"base64":true}));
    assert_eq!(&*result.bytes.unwrap(), &[255]);
}
#[test]
fn all_supported_signatures_are_recognized_without_mime() {
    for bytes in [
        b"\x89PNG\r\n\x1a\n".as_slice(),
        b"\xff\xd8\xff",
        b"GIF87a",
        b"GIF89a",
        b"BM",
        b"RIFFxxxxWEBP",
    ] {
        assert!(decode_image_body(&json!({"body":b64(bytes),"base64":true})).image);
    }
}
#[test]
fn null_wrong_types_and_bounded_sniffing() {
    for body in [json!(null), json!(1), json!([])] {
        assert!(!may_contain_image_body(&json!({"body":body,"image":true})));
        assert!(
            decode_image_body(&json!({"body":body,"image":true}))
                .bytes
                .is_none()
        );
    }
    assert!(!may_contain_image_body(
        &json!({"body":format!("{}{}"," ".repeat(129),b64(PNG))})
    ));
}
#[test]
fn capture_fallback_and_input_remain_immutable() {
    let data = json!({"headers":{"Content-Type":["image/png"]},"body":null,"bodyPreview":b64(PNG),"bodyEncoding":"base64"});
    let before = data.clone();
    assert_eq!(&*image_body_from_capture(&data).bytes.unwrap(), PNG);
    assert_eq!(data, before);
}
#[test]
fn real_pixels_dimensions_and_corrupt_signature_fallback() {
    for bytes in [PNG, JPEG] {
        let expected = image::load_from_memory(bytes).unwrap();
        let asset = ImageAsset::new(Arc::from(bytes));
        assert!(!asset.is_prepared());
        let bitmap = asset.pixels().as_ref().unwrap();
        assert_eq!(
            (bitmap.width, bitmap.height),
            (expected.width(), expected.height())
        );
        assert_eq!(
            bitmap.rgba.len(),
            (bitmap.width * bitmap.height * 4) as usize
        );
    }
    let asset = ImageAsset::new(Arc::from(b"\xff\xd8\xff".as_slice()));
    assert!(asset.pixels().is_err());
    assert!(std::ptr::eq(asset.pixels(), asset.pixels()));
}
#[test]
fn shared_asset_decodes_once_and_last_owner_releases() {
    let owner = Arc::new(ImageAsset::new(Arc::from(PNG)));
    let weak = Arc::downgrade(&owner);
    let inline = owner.clone();
    let enlarged = owner.clone();
    assert!(Arc::ptr_eq(inline.bytes(), enlarged.bytes()));
    assert!(std::ptr::eq(inline.pixels(), enlarged.pixels()));
    drop(owner);
    drop(inline);
    assert!(weak.upgrade().is_some());
    drop(enlarged);
    assert!(weak.upgrade().is_none());
}
#[test]
fn oversized_dimension_scales_to_4096_without_upscaling() {
    let image = image::RgbaImage::new(5000, 2);
    let mut encoded = Cursor::new(Vec::new());
    image
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let asset = ImageAsset::new(encoded.into_inner().into());
    let pixels = asset.pixels().as_ref().unwrap();
    assert_eq!((pixels.width, pixels.height), (4096, 1));
    assert!(pixels.rgba.len() <= 64 * 1024 * 1024);
}
#[test]
fn over_budget_source_has_bounded_thumbnail_and_new_resource_is_independent() {
    let image = image::GrayImage::new(9000, 9000);
    let mut encoded = Cursor::new(Vec::new());
    image
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let asset = ImageAsset::new(encoded.into_inner().into());
    #[cfg(target_os = "macos")]
    {
        let pixels = asset
            .pixels()
            .as_ref()
            .expect("native thumbnail of >64MiB source");
        assert!(pixels.width <= 4096 && pixels.height <= 4096);
        assert!(pixels.rgba.len() <= 64 * 1024 * 1024);
    }
    #[cfg(not(target_os = "macos"))]
    assert!(asset.pixels().is_err());
    let old = ImageAsset::new(Arc::from(PNG));
    let new = ImageAsset::new(Arc::from(JPEG));
    assert!(!Arc::ptr_eq(old.bytes(), new.bytes()));
    assert!(!new.is_prepared());
}

#[test]
fn gif_bmp_webp_decode_real_pixels_and_gif_uses_first_frame() {
    let frame = image::RgbaImage::from_pixel(3, 2, image::Rgba([255, 0, 0, 255]));
    for format in [image::ImageFormat::Bmp, image::ImageFormat::WebP] {
        let mut encoded = Cursor::new(Vec::new());
        frame.write_to(&mut encoded, format).unwrap();
        let asset = ImageAsset::new(encoded.into_inner().into());
        let pixels = asset.pixels().as_ref().unwrap();
        assert_eq!((pixels.width, pixels.height), (3, 2));
        assert_eq!(&pixels.rgba[..4], &[255, 0, 0, 255]);
    }
    let mut encoded = Vec::new();
    {
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut encoded);
        encoder.encode_frame(image::Frame::new(frame)).unwrap();
        encoder
            .encode_frame(image::Frame::new(image::RgbaImage::from_pixel(
                3,
                2,
                image::Rgba([0, 255, 0, 255]),
            )))
            .unwrap();
    }
    let asset = ImageAsset::new(encoded.into());
    assert_eq!(
        &asset.pixels().as_ref().unwrap().rgba[..4],
        &[255, 0, 0, 255]
    );
}
#[test]
fn jpeg_exif_orientation_is_applied_after_decode() {
    // Minimal EXIF APP1: big-endian TIFF, one orientation tag (6 = rotate 90).
    let mut exif =
        b"Exif\0\0MM\0*\0\0\0\x08\0\x01\x01\x12\0\x03\0\0\0\x01\0\x06\0\0\0\0\0\0".to_vec();
    let source = image::RgbImage::from_fn(3, 2, |x, y| {
        image::Rgb([(x * 70) as u8, (y * 100) as u8, 0])
    });
    let mut encoded = Cursor::new(Vec::new());
    source
        .write_to(&mut encoded, image::ImageFormat::Jpeg)
        .unwrap();
    let source = encoded.into_inner();
    let mut jpeg = source[..2].to_vec();
    jpeg.extend([255, 225]);
    jpeg.extend(((exif.len() + 2) as u16).to_be_bytes());
    jpeg.append(&mut exif);
    jpeg.extend_from_slice(&source[2..]);
    let original = ImageAsset::new(source.into());
    let rotated = ImageAsset::new(jpeg.into());
    let a = original.pixels().as_ref().unwrap();
    let b = rotated.pixels().as_ref().unwrap();
    assert_eq!((b.width, b.height), (a.height, a.width));
    assert_eq!(
        &b.rgba[..4],
        &a.rgba[((a.height - 1) * a.width * 4) as usize..][..4]
    );
}
#[test]
fn concurrent_owners_share_one_decode_result() {
    let asset = Arc::new(ImageAsset::new(Arc::from(PNG)));
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let asset = &asset;
                scope.spawn(move || asset.pixels() as *const _ as usize)
            })
            .collect();
        let pointers: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert!(pointers.iter().all(|p| *p == pointers[0]));
    });
}
