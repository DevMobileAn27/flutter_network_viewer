use flutter_network_viewer::form_body::{
    OmittedBodyReason, body_content_type, is_form_body, omitted_body_reason, parse_form_body,
};
use serde_json::{Value, json};

fn parse(body: &str, mime: &str) -> flutter_network_viewer::form_body::FormBody {
    parse_form_body(&json!({"body":body,"contentType":mime}))
}
fn multipart(headers: &str, value: &str) -> String {
    format!("--AbC\r\n{headers}\r\n\r\n{value}\r\n--AbC--\r\n")
}
const MIME: &str = "multipart/form-data; boundary=AbC";

#[test]
fn url_fields_preserve_order_duplicates_empty_names_and_unicode() {
    let f = parse(
        "name=C%C3%A0+ph%C3%AA&tag=a&tag=b&empty=&flag&=x&&a=b=c&plus=%2B&emoji=%F0%9F%98%80",
        "APPLICATION/X-WWW-FORM-URLENCODED; charset=UTF-8",
    );
    assert!(!f.incomplete);
    let fields: Vec<_> = f
        .parts
        .iter()
        .map(|p| (p.name.as_str(), p.value.as_deref().unwrap()))
        .collect();
    assert_eq!(
        fields,
        [
            ("name", "Cà phê"),
            ("tag", "a"),
            ("tag", "b"),
            ("empty", ""),
            ("flag", ""),
            ("", "x"),
            ("a", "b=c"),
            ("plus", "+"),
            ("emoji", "😀")
        ]
    );
    assert_eq!(f.parts[0].byte_length, "Cà phê".len());
    assert_eq!(f.parts[8].byte_length, 4);
    assert!(f.parts.iter().all(|p| !p.is_file()));
    assert!(!parse("&&", "application/x-www-form-urlencoded").incomplete);
}

#[test]
fn binary_multipart_keeps_metadata_without_attachment_text() {
    // 00 ff fe CR LF --AbC--not-a-boundary 7f, encoded as part of whole body.
    let raw = "LS1BYkNcclxu"; // deliberately invalid multipart but valid base64
    assert!(
        parse_form_body(&json!({"body":raw,"encoding":"base64","contentType":MIME})).incomplete
    );
    let bytes = [b"--AbC\r\nContent-Disposition: form-data; name=title\r\n\r\nC\xc3\xa0 ph\xc3\xaa \xf0\x9f\x98\x80\r\n--AbC\r\nContent-Disposition: form-data; name=attachment; filename=\"photo; test.jpg\"\r\nContent-Type: image/jpeg\r\n\r\n".as_slice(), b"\0\xff\xfe\r\n--AbC--not-a-boundary\x7f", b"\r\n--AbC--\r\n"].concat();
    let args = json!({"body":base64(&bytes),"encoding":"base64","contentType":MIME});
    let before = args.clone();
    let f = parse_form_body(&args);
    assert_eq!(args, before);
    assert!(!f.incomplete);
    assert_eq!(f.parts.len(), 2);
    assert_eq!(f.parts[0].value.as_deref(), Some("Cà phê 😀"));
    let p = &f.parts[1];
    assert!(p.is_file());
    assert_eq!(p.filename.as_deref(), Some("photo; test.jpg"));
    assert_eq!(p.content_type.as_deref(), Some("image/jpeg"));
    assert_eq!(
        p.byte_length,
        b"\0\xff\xfe\r\n--AbC--not-a-boundary\x7f".len()
    );
    assert_eq!(p.value, None);
}

#[test]
fn multipart_repeated_empty_fields_and_empty_filename() {
    let body = "--AbC\r\nContent-Disposition: form-data; name=tag\r\n\r\none\r\n--AbC\r\nContent-Disposition: form-data; name=tag\r\n\r\ntwo\r\n--AbC\r\nContent-Disposition: form-data; name=\"\"\r\n\r\n\r\n--AbC\r\nContent-Disposition: form-data; name=file; filename=\"\"\r\n\r\n\r\n--AbC--";
    let f = parse(body, MIME);
    assert!(!f.incomplete);
    assert_eq!(f.parts.len(), 4);
    assert_eq!(f.parts[0].name, f.parts[1].name);
    assert_eq!(f.parts[2].name, "");
    assert_eq!(f.parts[2].value.as_deref(), Some(""));
    assert!(f.parts[3].is_file());
    assert_eq!(f.parts[3].byte_length, 0);
}

#[test]
fn extended_filename_case_sensitive_boundary_and_quoted_parameters() {
    let f = parse(
        &multipart(
            "CONTENT-DISPOSITION: form-data; NAME=\"f\\\"oo\"; filename=\"fallback\"; FILENAME*=UTF-8''B%E1%BA%A3n%20v%E1%BA%BD.png",
            "abc",
        ),
        "MULTIPART/FORM-DATA; BOUNDARY=\"AbC\"",
    );
    assert!(!f.incomplete);
    assert_eq!(f.parts[0].filename.as_deref(), Some("Bản vẽ.png"));
    assert_eq!(f.parts[0].name, "f\"oo");
    assert_eq!(f.parts[0].byte_length, 3);
    assert!(
        parse(
            &multipart("Content-Disposition: form-data; name=x", "abc"),
            "multipart/form-data; boundary=abc"
        )
        .incomplete
    );
}

#[test]
fn invalid_extended_filename_falls_back_without_plus_conversion() {
    for extended in ["UTF-8''%GG", "UTF-8''%FF", "latin1''abc", "garbage"] {
        let f = parse(
            &multipart(
                &format!(
                    "Content-Disposition: form-data; name=x; filename=fallback; filename*={extended}"
                ),
                "abc",
            ),
            MIME,
        );
        assert_eq!(f.parts[0].filename.as_deref(), Some("fallback"));
    }
    let f = parse(
        &multipart(
            "Content-Disposition: form-data; name=x; filename*=utf-8'en'a+b",
            "x",
        ),
        MIME,
    );
    assert_eq!(f.parts[0].filename.as_deref(), Some("a+b"));
}

#[test]
fn truncated_retains_complete_parts_and_remainder() {
    let raw = "--AbC\r\nContent-Disposition: form-data; name=a\r\n\r\ncomplete\r\n--AbC\r\nContent-Disposition: form-data; name=b\r\n\r\npartial\r\n--Ab";
    let f = parse(raw, MIME);
    assert!(f.incomplete);
    assert_eq!(f.parts[0].value.as_deref(), Some("complete"));
    assert_eq!(f.parts[1].value.as_deref(), Some("partial\r\n--Ab"));
    assert!(parse("--AbC\r\ncorrupt headers", MIME).incomplete);
}

#[test]
fn malformed_inputs_return_incomplete_without_partial_url_results() {
    for body in ["bad=%GG", "good=yes&bad=%", "x=%FF", "x=%C3", "x=%0"] {
        let f = parse(body, "application/x-www-form-urlencoded");
        assert!(f.incomplete, "{body}");
        assert!(f.parts.is_empty());
    }
    for boundary in ["", "\"\"", "\"a\r\nb\"", &"a".repeat(201), "\"unterminated"] {
        assert!(parse("raw", &format!("multipart/form-data; boundary={boundary}")).incomplete);
    }
    for body in ["%%%", "A", "====", "a=bc", "AA=A"] {
        assert!(
            parse_form_body(&json!({"body":body,"encoding":"base64","contentType":MIME}))
                .incomplete
        );
    }
    for args in [
        Value::Null,
        json!({}),
        json!({"body":12}),
        json!({"body":"x","contentType":4}),
    ] {
        assert!(parse_form_body(&args).incomplete);
    }
}

#[test]
fn base64_url_fields_whitespace_and_missing_padding() {
    for raw in ["eD0rJTJC", " eD0r\nJTJC\t", "eD0", "eD0="] {
        let f = parse_form_body(
            &json!({"body":raw,"encoding":"base64","contentType":"application/x-www-form-urlencoded"}),
        );
        assert!(!f.incomplete);
        assert_eq!(
            f.parts[0].value.as_deref(),
            Some(if raw.contains("JTJC") { " +" } else { "" })
        );
    }
}

#[test]
fn mime_detection_and_header_lists_are_case_insensitive() {
    let t = body_content_type(&json!({"CONTENT-TYPE":["Multipart/Form-Data", "boundary=AbC"]}));
    assert_eq!(t, "Multipart/Form-Data; boundary=AbC");
    assert!(is_form_body(&t));
    assert!(is_form_body(
        " APPLICATION/X-WWW-FORM-URLENCODED ; charset=utf-8"
    ));
    assert!(!is_form_body("application/json"));
    assert_eq!(body_content_type(&Value::Null), "");
    assert_eq!(body_content_type(&json!({})), "");
}

#[test]
fn latin1_and_invalid_utf8_text_are_presented_without_losing_byte_count() {
    for (charset, expected) in [("ISO-8859-1", "ÿé"), ("latin1", "ÿé"), ("utf-8", "��")] {
        let bytes = [format!("--AbC\r\nContent-Disposition: form-data; name=x\r\nContent-Type: text/plain; charset={charset}\r\n\r\n").as_bytes(), &[255,233], b"\r\n--AbC--"].concat();
        let f =
            parse_form_body(&json!({"body":base64(&bytes),"encoding":"base64","contentType":MIME}));
        assert_eq!(f.parts[0].value.as_deref(), Some(expected));
        assert_eq!(f.parts[0].byte_length, 2);
    }
}

#[test]
fn missing_disposition_skips_part_and_marks_incomplete() {
    let f = parse(
        "preamble\r\n--AbC\r\nX-Foo: bar\r\n\r\nignored\r\n--AbC\r\nContent-Disposition: form-data; name=x\r\n\r\nkept\r\n--AbC--\r\nepilogue",
        MIME,
    );
    assert!(f.incomplete);
    assert_eq!(f.parts.len(), 1);
    assert_eq!(f.parts[0].value.as_deref(), Some("kept"));
    assert!(!parse("--AbC--", MIME).incomplete);
}

#[test]
fn part_count_and_header_scan_are_bounded() {
    let url = std::iter::repeat_n("a=b", 1001)
        .collect::<Vec<_>>()
        .join("&");
    let f = parse(&url, "application/x-www-form-urlencoded");
    assert_eq!(f.parts.len(), 1000);
    assert!(f.incomplete);
    let item = "--AbC\r\nContent-Disposition: form-data; name=a\r\n\r\nb\r\n";
    let f = parse(&(item.repeat(1001) + "--AbC--"), MIME);
    assert_eq!(f.parts.len(), 1000);
    assert!(f.incomplete);
    assert!(
        parse(
            &format!("--AbC\r\n{}\r\n\r\nx\r\n--AbC--", "x".repeat(65536)),
            MIME
        )
        .incomplete
    );
}

#[test]
fn omitted_body_reason_is_available_without_a_body() {
    let data = json!({"bodyOmitted":"profile-size-limit", "bodyCaptureLimit":4194304,"bodyByteLength":59774016});
    assert_eq!(
        omitted_body_reason(&data),
        Some(OmittedBodyReason::SizeLimit {
            limit_bytes: 4194304
        })
    );
    for reason in ["read-failed", "unknown"] {
        assert_eq!(
            omitted_body_reason(&json!({"bodyOmitted":reason})),
            Some(OmittedBodyReason::ReadFailed)
        );
    }
    assert_eq!(omitted_body_reason(&json!({})), None);
}

fn base64(bytes: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in bytes.chunks(3) {
        let n = ((c[0] as u32) << 16)
            | ((c.get(1).copied().unwrap_or(0) as u32) << 8)
            | c.get(2).copied().unwrap_or(0) as u32;
        s.push(CHARS[(n >> 18) as usize] as char);
        s.push(CHARS[((n >> 12) & 63) as usize] as char);
        s.push(if c.len() > 1 {
            CHARS[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        s.push(if c.len() > 2 {
            CHARS[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    s
}
