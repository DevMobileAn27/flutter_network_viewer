import CryptoKit
import Foundation

// Private key arrives through stdin, never through process arguments or logs.
let encoded = String(data: FileHandle.standardInput.readDataToEndOfFile(), encoding: .utf8)!
    .trimmingCharacters(in: .whitespacesAndNewlines)
guard let seed = Data(base64Encoded: encoded) else { fatalError("Invalid Sparkle seed") }
let key = try Curve25519.Signing.PrivateKey(rawRepresentation: seed)
let expected = try String(contentsOfFile: "updates/macos_ed25519.pub", encoding: .utf8)
    .trimmingCharacters(in: .whitespacesAndNewlines)
guard key.publicKey.rawRepresentation.base64EncodedString() == expected else {
    fatalError("Signing key does not match the public key embedded in the app")
}
let archive = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]), options: .mappedIfSafe)
print(try key.signature(for: archive).base64EncodedString())
