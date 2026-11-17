import CryptoKit
import Foundation
let input = FileHandle.standardInput.readDataToEndOfFile()
guard let encoded = String(data: input, encoding: .utf8),
      let seed = Data(base64Encoded: encoded.trimmingCharacters(in: .whitespacesAndNewlines)) else {
    throw NSError(domain: "Invalid signing seed", code: 1)
}
let key = try Curve25519.Signing.PrivateKey(rawRepresentation: seed)
guard key.publicKey.rawRepresentation.base64EncodedString() == CommandLine.arguments[2] else {
    throw NSError(domain: "Signing key does not match embedded trust anchor", code: 2)
}
let archive = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]), options: .mappedIfSafe)
print(try key.signature(for: archive).base64EncodedString())
