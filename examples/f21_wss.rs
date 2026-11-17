//! Real native TLS must reject an untrusted local certificate.
use flutter_network_viewer::vm::{VmClient, normalize_uri};
fn main() {
    let uri = std::env::args().nth(1).unwrap();
    assert!(normalize_uri(&uri).unwrap().starts_with("wss://"));
    let result = VmClient::connect(&uri, std::time::Duration::from_secs(3));
    assert!(result.is_err(), "untrusted TLS certificate accepted");
    println!("PASS F21 WSS: untrusted self-signed certificate rejected by native TLS");
}
