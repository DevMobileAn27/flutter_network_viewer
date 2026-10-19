import unittest
import xml.etree.ElementTree as ET

from create_update_feed import SPARKLE, feed_xml, sign_archive


class UpdateFeedTest(unittest.TestCase):
    def test_macos_feed_uses_build_number_and_ed25519(self):
        result = ET.fromstring(feed_xml('macos-arm64', '0.2.0', '8', 'Viewer 0.2.0',
                                        '<script>alert(1)</script>', 'app.zip', 123,
                                        'edSignature', 'signed'))
        item = result.find('channel/item')
        self.assertEqual(item.find(f'{{{SPARKLE}}}version').text, '8')
        enclosure = item.find('enclosure')
        self.assertEqual(enclosure.attrib[f'{{{SPARKLE}}}edSignature'], 'signed')
        self.assertEqual(enclosure.attrib[f'{{{SPARKLE}}}os'], 'macos')
        self.assertIn('/releases/download/v0.2.0/', enclosure.attrib['url'])
        self.assertIn('&lt;script&gt;', item.find('description').text)

    def test_windows_feed_uses_installer_and_version(self):
        result = ET.fromstring(feed_xml('windows-x64', '0.2.0', '8', 'Viewer',
                                        'Changes', 'app-setup.exe', 456,
                                        'dsaSignature', 'signed'))
        enclosure = result.find('channel/item/enclosure')
        self.assertEqual(enclosure.attrib[f'{{{SPARKLE}}}version'], '0.2.0')
        self.assertTrue(enclosure.attrib['url'].endswith('-setup.exe'))
        self.assertEqual(enclosure.attrib[f'{{{SPARKLE}}}installerArguments'], '/SILENT /NORESTART /SP-')

    def test_unsigned_updates_are_rejected(self):
        with self.assertRaises(ValueError):
            sign_archive(None, 'macos-arm64', '')
        with self.assertRaises(ValueError):
            feed_xml('macos-arm64', '0.2.0', '8', 'Viewer', '', 'app.zip', 1, 'edSignature', '')


if __name__ == '__main__':
    unittest.main()
