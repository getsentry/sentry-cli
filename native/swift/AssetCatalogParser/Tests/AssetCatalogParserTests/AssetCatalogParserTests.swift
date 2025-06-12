@testable import AssetCatalogParser
import Testing
import Foundation

struct AssetCatalogParserTests {
  @Test func testParseAssets() throws {
    let archivePath = try #require(Bundle.module.path(forResource: "test", ofType: "xcarchive"))
    let url = URL(filePath: "\(archivePath)/Products/Applications/DemoApp.app/Assets.car")
    let results = AssetUtil.disect(file: url)
    #expect(results.count == 2)
  }
}
