// Run once on macOS: swift tool/generate_update_keys.swift
// Private seed is compatible with Sparkle's sign_update --ed-key-file.
import CryptoKit
import Foundation

let directory = URL(fileURLWithPath: ".local/update-signing", isDirectory: true)
try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true,
    attributes: [.posixPermissions: 0o700])
let privateFile = directory.appendingPathComponent("macos_ed25519.key")
let key: Curve25519.Signing.PrivateKey
if FileManager.default.fileExists(atPath: privateFile.path) {
    let encoded = try String(contentsOf: privateFile, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)
    guard let data = Data(base64Encoded: encoded) else { fatalError("Invalid private seed") }
    key = try Curve25519.Signing.PrivateKey(rawRepresentation: data)
} else {
    key = Curve25519.Signing.PrivateKey()
    try key.rawRepresentation.base64EncodedString().write(to: privateFile, atomically: true, encoding: .utf8)
    try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: privateFile.path)
}
try FileManager.default.createDirectory(atPath: "updates", withIntermediateDirectories: true)
try (key.publicKey.rawRepresentation.base64EncodedString() + "\n")
    .write(toFile: "updates/macos_ed25519.pub", atomically: true, encoding: .utf8)
print("Sparkle public key saved; private seed kept in ignored .local/update-signing.")
