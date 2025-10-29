import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers
import ObjcSupport

@_cdecl("swift_inspect_asset_catalog")
// Insepects the asset catalog and writes the results to a JSON file
// in the xcarchive containing the asset catalog.
public func swift_inspect_asset_catalog(_ path: UnsafePointer<CChar>, outputPath: UnsafePointer<CChar>) {
    let pathString = String(cString: path)
    let outputPathString = String(cString: outputPath)
    if #available(macOS 13.0, *) {
        let supportedVersions = [13, 14, 15]
        let version = ProcessInfo.processInfo.operatingSystemVersion
        if supportedVersions.contains(version.majorVersion) {
            AssetUtil.disect(file: URL(filePath: pathString), outputURL: URL(filePath: outputPathString))
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
    let idiom: String?
    let colorspace: String?
}

enum Error: Swift.Error {
  case pathError
}

typealias objectiveCMethodImp = @convention(c) (AnyObject, Selector, UnsafeRawPointer) -> Unmanaged<
    AnyObject
>?

private struct MultisizeSetInfo {
    let name: String
    let element: UInt
    let part: UInt
    let identifier: UInt
    let sizeIndexes: [(idiom: UInt, subtype: UInt)]
}

enum AssetUtil {
    private static func idiomToString(_ idiom: UInt?) -> String? {
        guard let idiom = idiom else { return nil }
        switch idiom {
        case 0: return "universal"
        case 1: return "phone"
        case 2: return "pad"
        case 3: return "tv"
        case 4: return "carplay"
        case 5: return "watch"
        case 6: return "marketing"
        default: return nil
        }
    }
    
    private static func colorSpaceIDToString(_ colorSpaceID: UInt?) -> String? {
        guard let colorSpaceID = colorSpaceID else { return nil }
        switch colorSpaceID {
        case 1: return "srgb"
        case 2: return "gray gamma 22"
        case 3: return "displayP3"
        case 4: return "extended srgb"
        default: return nil
        }
    }
    
    private static func createResultsPath(assetURL: URL, outputURL: URL) throws -> URL {
        var archiveURL = assetURL
        var tailComponents: [String] = []
        while archiveURL.pathExtension != "xcarchive" && archiveURL.pathComponents.count > 1 {
            tailComponents.insert(archiveURL.lastPathComponent, at: 0)
            archiveURL.deleteLastPathComponent()
        }
        if archiveURL.pathExtension != "xcarchive" {
            throw Error.pathError
        }

        let destDir = tailComponents
            .dropLast()
            .reduce(outputURL) { partial, next in
                partial.appendingPathComponent(next, isDirectory: true)
            }
        try! FileManager.default.createDirectory(at: destDir,
                                                 withIntermediateDirectories: true)
        return destDir
    }

    @discardableResult static func disect(file: URL, outputURL: URL) -> [AssetCatalogEntry] {
        var assets: [AssetCatalogEntry] = []
        var colorLength: UInt = 0
        var colorCount = 0

        let (structuredThemeStore, assetKeys) = initializeCatalog(from: file)

        var images: [String: (cgImage: CGImage, format: String)] = [:]
        
        // First pass: Build map of multisize sets and map element+idiom+subtype to the specific set
        var multisizeSets: [MultisizeSetInfo] = []
        
        for key in assetKeys {
            let keyList = unsafeBitCast(
                key.perform(Selector(("keyList"))),
                to: UnsafeMutableRawPointer.self
            )
            guard let rendition = createRendition(from: structuredThemeStore, keyList) else {
                continue
            }
            
            let type = rendition.getUInt(forKey: "type") ?? 0
            if type == 1010 {  // Multisize image set
                let renditionTypeName = rendition.perform(Selector(("name"))).takeUnretainedValue() as! String
                let keyElement = key.getUInt(forKey: "themeElement") ?? 0
                let keyPart = key.getUInt(forKey: "themePart") ?? 0
                let keyIdentifier = key.getUInt(forKey: "themeIdentifier") ?? 0
                
                // Extract size indexes to identify which images belong to this set
                var sizeIndexes: [(idiom: UInt, subtype: UInt)] = []
                if rendition.responds(to: Selector(("sizeIndexes"))),
                   let sizeIndexesResult = rendition.perform(Selector(("sizeIndexes"))),
                   let sizeIndexesArray = sizeIndexesResult.takeUnretainedValue() as? NSArray {
                    for sizeIndexObj in sizeIndexesArray {
                        if let obj = sizeIndexObj as? NSObject {
                            let idiom = obj.getUInt(forKey: "idiom") ?? 0
                            let subtype = obj.getUInt(forKey: "subtype") ?? 0
                            sizeIndexes.append((idiom: idiom, subtype: subtype))
                        }
                    }
                }
                
                multisizeSets.append(MultisizeSetInfo(
                    name: renditionTypeName,
                    element: keyElement,
                    part: keyPart,
                    identifier: keyIdentifier,
                    sizeIndexes: sizeIndexes
                ))
            }
        }

        // Second pass: Process all assets
        for key in assetKeys {
            let keyList = unsafeBitCast(
                key.perform(Selector(("keyList"))),
                to: UnsafeMutableRawPointer.self
            )
            guard let rendition = createRendition(from: structuredThemeStore, keyList) else {
                continue
            }

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
            let isMultisizeImageSet = type == 1010
            let assetType = determineAssetType(key)
            let imageId = UUID().uuidString
            let fileExtension = (renditionTypeName as NSString).pathExtension.lowercased()
            
            var width: Int?
            var height: Int?
            var unslicedImage: CGImage?
            
            if isMultisizeImageSet {
                continue
            } else {
                // Get image dimensions from regular rendition
                (width, height, unslicedImage) = resolveImageDimensions(rendition, isVector)
                
                // Skip SVGs, but save images even if they don't have an extension (default to png)
                if fileExtension != "svg", let unslicedImage = unslicedImage {
                    let format = fileExtension.isEmpty ? "png" : fileExtension
                    images[imageId] = (cgImage: unslicedImage, format: format)
                }
            }
            
            let idiomValue = key.getUInt(forKey: "themeIdiom")
            let colorSpaceID = rendition.getUInt(forKey: "colorSpaceID")
            
            // Include multisize set name in the name field if it exists
            let finalName: String
            if let setName = findMultisizeSetName(key, in: multisizeSets) {
                finalName = "\(setName)/\(name)"
            } else {
                finalName = name
            }

            let asset = AssetCatalogEntry(
                imageId: imageId,
                size: length,
                name: finalName,
                vector: isVector,
                width: width,
                height: height,
                filename: renditionTypeName,
                type: assetType,
                idiom: idiomToString(idiomValue),
                colorspace: colorSpaceIDToString(colorSpaceID)
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
            type: nil,
            idiom: nil,
            colorspace: nil
        ))

        let data = try! JSONEncoder().encode(assets)
        let folder = try! createResultsPath(assetURL: file, outputURL: outputURL)
        let url = folder
            .appendingPathComponent("Assets")
            .appendingPathExtension("json")
        try! data.write(to: url, options: [])
        for (id, imageInfo) in images {
            let format = imageInfo.format
            let cgImage = imageInfo.cgImage
            let fileURL = folder.appendingPathComponent(id).appendingPathExtension(format)
            
            guard let utType = utTypeForExtension(format) else {
                print("⚠️  Unsupported format '\(format)' for \(id), skipping")
                continue
            }
            
            guard let dest = CGImageDestinationCreateWithURL(fileURL as CFURL, utType as CFString, 1, nil) else {
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
        -> NSObject?
    {
        let renditionWithKeySelector = Selector(("renditionWithKey:"))
        let renditionWithKeyMethod = themeStore.method(for: renditionWithKeySelector)!
        let renditionWithKeyImp = unsafeBitCast(renditionWithKeyMethod, to: objectiveCMethodImp.self)
        return renditionWithKeyImp(themeStore, renditionWithKeySelector, keyList)?.takeUnretainedValue()
            as? NSObject
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
        guard let referenceRendition = createRendition(from: themeStore, referenceKeyList) else {
            return false
        }

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
    
    private static func findMultisizeSetName(
        _ key: NSObject,
        in multisizeSets: [MultisizeSetInfo]
    ) -> String? {
        let element = key.getUInt(forKey: "themeElement") ?? 0
        let identifier = key.getUInt(forKey: "themeIdentifier") ?? 0
        let idiom = key.getUInt(forKey: "themeIdiom") ?? 0
        let subtype = key.getUInt(forKey: "themeSubtype") ?? 0
        
        for setInfo in multisizeSets {
            guard setInfo.element == element,
                  setInfo.identifier == identifier else {
                continue
            }
            
            for sizeIndex in setInfo.sizeIndexes {
                if sizeIndex.idiom == idiom && sizeIndex.subtype == subtype {
                    return setInfo.name
                }
            }
        }
        return nil
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
    
    /// Maps a file extension to its corresponding UTType identifier
    /// Returns nil for unknown or unsupported formats
    private static func utTypeForExtension(_ ext: String) -> String? {
        switch ext {
        case "jpg", "jpeg":
            return UTType.jpeg.identifier
        case "png":
            return UTType.png.identifier
        case "heic", "heif":
            return UTType.heic.identifier
        case "gif":
            return UTType.gif.identifier
        case "webp":
            return UTType.webP.identifier
        case "pdf":
            return UTType.pdf.identifier
        case "svg":
            return UTType.svg.identifier
        case "tiff", "tif":
            return UTType.tiff.identifier
        case "bmp":
            return UTType.bmp.identifier
        default:
            return nil
        }
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
