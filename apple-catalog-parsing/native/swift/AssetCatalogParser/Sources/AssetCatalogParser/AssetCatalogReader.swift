import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers
import ObjcSupport

@_cdecl("swift_inspect_asset_catalog")
// Insepects the asset catalog and writes the results to a JSON file
// in the xcarchive containing the asset catalog.
public func swift_inspect_asset_catalog(_ path: UnsafePointer<CChar>) {
    let pathString = String(cString: path)
    if #available(macOS 13.0, *) {
        let supportedVersions = [13, 14, 15]
        let version = ProcessInfo.processInfo.operatingSystemVersion
        if supportedVersions.contains(version.majorVersion) {
            AssetUtil.disect(file: URL(filePath: pathString))
        } else {
            print("Skipping asset catalog inspection on unsupported macOS version \(version)")
        }
    } else {
        print("Skipping asset catalog inspection on macOS earlier than 13.0")
    }
}

enum AssetType: Int, Encodable {
    case image
    case icon
    case imageSet
}

struct AssetCatalogEntry: Encodable {
    let imageId: String
    let size: UInt
    let name: String
    let vector: Bool
    let width: Int?
    let height: Int?
    let filename: String?
    let type: AssetType?
}

enum Error: Swift.Error {
  case pathError
}

typealias objectiveCMethodImp = @convention(c) (AnyObject, Selector, UnsafeRawPointer) -> Unmanaged<
    AnyObject
>?

enum AssetUtil {
    private static func createResultsPath(assetPath: URL) throws -> URL {
        var archiveURL = assetPath
        var tailComponents: [String] = []
        while archiveURL.pathExtension != "xcarchive" && archiveURL.pathComponents.count > 1 {
            tailComponents.insert(archiveURL.lastPathComponent, at: 0)
            archiveURL.deleteLastPathComponent()
        }
        if archiveURL.pathExtension != "xcarchive" {
            throw Error.pathError
        }
        let parsedRoot = archiveURL.appendingPathComponent("ParsedAssets",
                                                           isDirectory: true)
        let destDir = tailComponents
            .dropLast()
            .reduce(parsedRoot) { partial, next in
                partial.appendingPathComponent(next, isDirectory: true)
            }
        try! FileManager.default.createDirectory(at: destDir,
                                                 withIntermediateDirectories: true)
        return destDir
    }

    @discardableResult static func disect(file: URL) -> [AssetCatalogEntry] {
        var assets: [AssetCatalogEntry] = []
        var colorLength: UInt = 0
        var colorCount = 0

        let (structuredThemeStore, assetKeys) = initializeCatalog(from: file)

        var images: [String: CGImage] = [:]

        for key in assetKeys {
            let keyList = unsafeBitCast(
                key.perform(Selector(("keyList"))),
                to: UnsafeMutableRawPointer.self
            )
            let rendition = createRendition(from: structuredThemeStore, keyList)

            let data = rendition.value(forKey: "_srcData") as! Data
            let length = UInt(data.count)
            let className = rendition.perform(Selector(("className"))).takeUnretainedValue() as! String
            let renditionTypeName =
                rendition.perform(Selector(("name"))).takeUnretainedValue() as! String

            var packedAssetSize: UInt = 0
            if renditionTypeName.hasPrefix("ZZZZPacked") {
                packedAssetSize += length
                continue
            }

            if handleReferenceKey(
                rendition,
                structuredThemeStore,
                Selector(("renditionWithKey:")),
                &packedAssetSize,
                renditionTypeName,
                length
            ) {
                continue
            }

            if className == "_CUIThemeColorRendition" {
                colorCount += 1
                colorLength += length
                continue
            }

            let name = resolveRenditionName(
                structuredThemeStore,
                keyList,
                renditionTypeName
            )

            let type = rendition.getUInt(forKey: "type") ?? 0

            let isVector = type == 9
            let (width, height, unslicedImage) = resolveImageDimensions(rendition, isVector)
            let assetType = determineAssetType(key)
            let imageId = UUID().uuidString
            images[imageId] = unslicedImage

            let asset = AssetCatalogEntry(
                imageId: imageId,
                size: length,
                name: name,
                vector: isVector,
                width: width,
                height: height,
                filename: renditionTypeName,
                type: assetType
            )
            assets.append(asset)
        }

        assets.append(AssetCatalogEntry(
            imageId: "",
            size: colorLength,
            name: "\(colorCount) Color\(colorCount > 1 ? "s" : "")",
            vector: false,
            width: nil,
            height: nil,
            filename: nil,
            type: nil
        ))

        let data = try! JSONEncoder().encode(assets)
        let folder = try! createResultsPath(assetPath: file)
        let url = folder
            .appendingPathComponent("Assets")
            .appendingPathExtension("json")
        try! data.write(to: url, options: [])
        for (id, cgImage) in images {
            let fileURL = folder.appendingPathComponent(id)
                .appendingPathExtension("png")

            guard let dest = CGImageDestinationCreateWithURL(
                fileURL as CFURL,
                UTType.png.identifier as CFString,
                1,
                nil
            )
            else {
                print("⚠️  Could not create destination for \(fileURL.path)")
                continue
            }

            CGImageDestinationAddImage(dest, cgImage, nil)
            CGImageDestinationFinalize(dest)
        }
        return assets
    }

    private static func initializeCatalog(from file: URL) -> (
        themeStore: NSObject, assetKeys: [NSObject]
    ) {
        let catalogClass: NSObject.Type = NSClassFromString("CUICatalog")! as! NSObject.Type
        var catalog: NSObject =
            catalogClass.perform(Selector(("alloc"))).takeRetainedValue() as! NSObject
        catalog =
            catalog.perform(Selector(("initWithURL:error:")), with: file as NSURL, with: nil)
                .takeUnretainedValue() as! NSObject
        let structuredThemeStore =
            catalog.perform(Selector(("_themeStore"))).takeUnretainedValue() as! NSObject
        let assetStorage = structuredThemeStore.perform(Selector(("themeStore"))).takeUnretainedValue()
        let assetKeys =
            assetStorage.perform(Selector(("allAssetKeys"))).takeUnretainedValue() as! [NSObject]
        return (structuredThemeStore, assetKeys)
    }

    private static func createRendition(from themeStore: NSObject, _ keyList: UnsafeMutableRawPointer)
        -> NSObject
    {
        let renditionWithKeySelector = Selector(("renditionWithKey:"))
        let renditionWithKeyMethod = themeStore.method(for: renditionWithKeySelector)!
        let renditionWithKeyImp = unsafeBitCast(renditionWithKeyMethod, to: objectiveCMethodImp.self)
        return renditionWithKeyImp(themeStore, renditionWithKeySelector, keyList)!.takeUnretainedValue()
            as! NSObject
    }

    private static func handleReferenceKey(
        _ rendition: NSObject,
        _ themeStore: NSObject,
        _: Selector,
        _ packedAssetSize: inout UInt,
        _: String,
        _ length: UInt
    ) -> Bool {
        let referenceKey = safeValueForKey(rendition, "_referenceKey")
        guard let referenceKey = referenceKey as? NSObject else { return false }

        let referenceKeyList = unsafeBitCast(
            referenceKey.perform(Selector(("keyList"))),
            to: UnsafeMutableRawPointer.self
        )
        let referenceRendition = createRendition(from: themeStore, referenceKeyList)

        if let result = referenceRendition.perform(Selector(("unslicedImage"))) {
            let image = result.takeUnretainedValue() as! CGImage
            if image.dataProvider?.data as? Data != nil {
                packedAssetSize += length
            }
        }
        return true
    }

    private static func determineAssetType(_ key: NSObject) -> AssetType {
        let themeElement = key.getUInt(forKey: "themeElement") ?? 0
        let themePart = key.getUInt(forKey: "themePart") ?? 0

        if themeElement == 85, themePart == 220 {
            return .icon
        } else if themeElement == 9 {
            return .imageSet
        }
        return .image
    }

    private static func resolveRenditionName(
        _ structuredThemeStore: NSObject,
        _ keyList: UnsafeMutableRawPointer,
        _ renditionTypeName: String
    ) -> String {
        let renditionNameForKeyListSelector = Selector(("renditionNameForKeyList:"))
        let renditionNameForKeyListMethod = structuredThemeStore.method(
            for: renditionNameForKeyListSelector
        )!
        let renditionNameForKeyList = unsafeBitCast(
            renditionNameForKeyListMethod,
            to: objectiveCMethodImp.self
        )

        var renditionName: String?
        if let result = renditionNameForKeyList(
            structuredThemeStore,
            renditionNameForKeyListSelector,
            keyList
        ) {
            renditionName = result.takeUnretainedValue() as? String
        }

        let name = renditionTypeName == "CoreStructuredImage" ? renditionName : renditionTypeName
        return name!
    }

    private static func resolveImageDimensions(_ rendition: NSObject, _ isVector: Bool) -> (
        width: Int?, height: Int?, image: CGImage?
    ) {
        var unslicedImage: CGImage?
        if let result = rendition.perform(Selector(("unslicedImage"))) {
            unslicedImage = (result.takeUnretainedValue() as! CGImage)
        }

        var width: Int?
        var height: Int?
        if !isVector {
            width = unslicedImage?.width
            height = unslicedImage?.height
        }

        return (width, height, unslicedImage)
    }
}

private extension NSObject {
    func getUInt(forKey key: String) -> UInt? {
        if let result = perform(Selector(key)) {
            return UInt(bitPattern: result.toOpaque())
        }
        return nil
    }
}
