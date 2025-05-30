use anyhow::Result;

// Include the sentry-cli library
extern crate sentry_cli;

use sentry_cli::api::{Api, ChunkUploadCapability};
use sentry_cli::config::{Auth, Config};
use sentry_cli::utils::auth_token::AuthToken;
use sentry_cli::utils::fs::get_sha1_checksums;
use sentry_cli::utils::chunks::{upload_chunks, Chunk};
use sentry_cli::utils::progress::ProgressStyle;

fn create_test_config(auth_token: &str, base_url: &str) -> Result<Config> {
    let mut config = Config::from_cli_config()?;
    
    // Set the base URL to localhost
    config.set_base_url(base_url);
    
    // Set the auth token
    let token: AuthToken = auth_token.into();
    config.set_auth(Auth::Token(token));
    
    Ok(config)
}

fn test_chunk_upload_options(org: &str) -> Result<()> {
    println!("🔍 Testing chunk upload options endpoint...");
    
    let api = Api::current();
    let authenticated_api = api.authenticated()?;
    
    match authenticated_api.get_chunk_upload_options(org)? {
        Some(options) => {
            println!("✅ Chunk upload supported!");
            println!("  URL: {}", options.url);
            println!("  Max chunks per request: {}", options.max_chunks);
            println!("  Max request size: {}", options.max_size);
            println!("  Chunk size: {}", options.chunk_size);
            println!("  Concurrency: {}", options.concurrency);
            
            println!("  Supported capabilities:");
            let capabilities = [
                (ChunkUploadCapability::DebugFiles, "debug_files"),
                (ChunkUploadCapability::ReleaseFiles, "release_files"),
                (ChunkUploadCapability::ArtifactBundles, "artifact_bundles"),
                (ChunkUploadCapability::ArtifactBundlesV2, "artifact_bundles_v2"),
                (ChunkUploadCapability::Pdbs, "pdbs"),
                (ChunkUploadCapability::PortablePdbs, "portablepdbs"),
                (ChunkUploadCapability::Sources, "sources"),
                (ChunkUploadCapability::BcSymbolmap, "bcsymbolmaps"),
                (ChunkUploadCapability::Il2Cpp, "il2cpp"),
            ];
            
            for (capability, name) in capabilities {
                let supported = options.supports(capability);
                let status = if supported { "✅" } else { "❌" };
                println!("    {status} {name}");
            }
            
            Ok(())
        }
        None => {
            println!("❌ Chunk upload not supported by server");
            Err(anyhow::anyhow!("Chunk upload not supported"))
        }
    }
}

fn test_full_debug_file_upload_flow(org: &str, project: &str, binary_path: &str) -> Result<()> {
    println!("🚀 Testing full debug file chunk upload flow...");
    
    // Read the binary file directly
    let binary_path = std::path::Path::new(binary_path);
    println!("  🔧 Binary file: {}", binary_path.display());
    
    // Read the binary file
    let content = std::fs::read(&binary_path)
        .map_err(|e| anyhow::anyhow!("Failed to read binary file {}: {}", binary_path.display(), e))?;
    
    println!("  📄 File size: {} bytes", content.len());
    
    let api = Api::current();
    let authenticated_api = api.authenticated()?;
    
    // Get chunk upload options
    let chunk_upload_options = authenticated_api.get_chunk_upload_options(org)?
        .ok_or_else(|| anyhow::anyhow!("Chunk upload not supported by server"))?;
    
    println!("  📊 Chunk configuration:");
    println!("    • Chunk size: {} bytes", chunk_upload_options.chunk_size);
    println!("    • Max chunks per request: {}", chunk_upload_options.max_chunks);
    println!("    • Max request size: {} bytes", chunk_upload_options.max_size);
    println!("    • Concurrency: {}", chunk_upload_options.concurrency);
    
    // Step 1: Prepare data and calculate checksums
    let data = &content; // Binary data, not string
    let chunk_size = chunk_upload_options.chunk_size as usize;
    let (total_checksum, chunk_checksums) = get_sha1_checksums(data, chunk_size)?;
    
    println!("  🔢 Checksum calculation:");
    println!("    • Total file SHA1: {}", total_checksum);
    println!("    • Number of chunks: {}", chunk_checksums.len());
    
    // Show individual chunk details
    for (i, checksum) in chunk_checksums.iter().enumerate() {
        let chunk_start = i * chunk_size;
        let chunk_end = std::cmp::min(chunk_start + chunk_size, data.len());
        let chunk_size_actual = chunk_end - chunk_start;
        println!("      Chunk {}: {} bytes (SHA1: {})", i + 1, chunk_size_actual, checksum);
    }
    
    // Step 2: Check which chunks are missing (if any)
    println!("  🔍 Checking for missing chunks...");
    let missing_checksums = authenticated_api.find_missing_dif_checksums(
        org,
        project,
        chunk_checksums.iter().copied()
    )?;
    
    if missing_checksums.is_empty() {
        println!("    ✅ All chunks already exist on server");
    } else {
        println!("    📤 {} chunks need to be uploaded", missing_checksums.len());
        
        // Step 3: Upload missing chunks
        println!("  🚀 Uploading chunks...");
        let chunks_to_upload: Vec<_> = data.chunks(chunk_size)
            .zip(chunk_checksums.iter())
            .filter(|(_, checksum)| missing_checksums.contains(checksum))
            .map(|(chunk_data, checksum)| (*checksum, chunk_data))
            .collect();
        
        if !chunks_to_upload.is_empty() {
            // Create Chunk objects for upload
            let chunks: Vec<_> = chunks_to_upload.iter()
                .map(|(checksum, data)| Chunk((*checksum, *data)))
                .collect();
            
            println!("    🌐 Uploading {} chunks to: {}", chunks.len(), chunk_upload_options.url);
            upload_chunks(&chunks, &chunk_upload_options, ProgressStyle::default_bar())?;
            
            println!("    ✅ Chunks uploaded successfully!");
        }
    }
    
    // Step 4: Use debug file assembly (correct for dSYM files)
    println!("  🔧 Assembling debug file...");
    
    // Create a ChunkedDifRequest
    let filename = binary_path.file_name().unwrap().to_string_lossy();
    let dif_request = sentry_cli::api::ChunkedDifRequest::new(
        filename.into(),
        &chunk_checksums,
        total_checksum,
    );
    
    // Create the AssembleDifsRequest from the single request
    let assembly_request: sentry_cli::api::AssembleDifsRequest = 
        std::iter::once(dif_request).collect();
    
    let assembly_response = authenticated_api.assemble_difs(org, project, &assembly_request)?;
    
    println!("    📦 Debug file assembly initiated!");
    
    // The response is a HashMap<Digest, ChunkedDifResponse>
    if let Some(response) = assembly_response.get(&total_checksum) {
        println!("      State: {:?}", response.state);
        println!("      Missing chunks: {}", response.missing_chunks.len());
        
        if let Some(ref detail) = response.detail {
            println!("      Detail: {}", detail);
        }
        
        // Step 5: Check assembly status
        match response.state {
            sentry_cli::api::ChunkedFileState::Ok => {
                println!("    ✅ Assembly completed successfully!");
                if let Some(ref dif) = response.dif {
                    println!("      Debug info file created: {}", dif.object_name);
                }
            }
            sentry_cli::api::ChunkedFileState::NotFound => {
                println!("    ❌ Assembly failed - bundle not found");
            }
            sentry_cli::api::ChunkedFileState::Created => {
                println!("    ⏳ Assembly created, waiting for processing...");
            }
            sentry_cli::api::ChunkedFileState::Assembling => {
                println!("    ⚙️  Assembly in progress...");
            }
            sentry_cli::api::ChunkedFileState::Error => {
                println!("    ❌ Assembly failed with error");
                return Err(anyhow::anyhow!("Assembly failed"));
            }
        }
        
        if response.missing_chunks.is_empty() {
            println!("  🎉 Full debug file upload flow completed successfully!");
            println!("    📈 Summary:");
            println!("      • Binary: {}", binary_path.display());
            println!("      • Total chunks: {}", chunk_checksums.len());
            println!("      • Chunks uploaded: {}", missing_checksums.len());
            println!("      • File checksum: {}", total_checksum);
            println!("      • Organization: {}", org);
            println!("      • Project: {}", project);
        } else {
            println!("  ⚠️  Some chunks are still missing after upload attempt");
        }
    } else {
        println!("    ❌ No response found for file checksum {}", total_checksum);
        return Err(anyhow::anyhow!("Assembly response missing"));
    }
    
    Ok(())
}

fn test_assembly_only(org: &str, projects: &[String]) -> Result<()> {
    println!("🔧 Testing artifact bundle assembly API (without chunk upload)...");
    
    let api = Api::current();
    let authenticated_api = api.authenticated()?;
    
    // Create dummy chunk data for testing assembly endpoint
    let test_data = b"test chunk data for assembly endpoint";
    let mut hasher = sha1_smol::Sha1::new();
    hasher.update(test_data);
    let checksum = hasher.digest();
    
    let chunks = vec![checksum];
    
    println!("  🧪 Testing with dummy data:");
    println!("    • Checksum: {}", checksum);
    println!("    • Chunks: {}", chunks.len());
    
    // Test artifact bundle assembly without uploading chunks first
    match authenticated_api.assemble_artifact_bundle(
        org,
        projects,
        checksum,
        &chunks,
        Some("test-release-v1.0.0"),
        None, // dist
    ) {
        Ok(response) => {
            println!("  ✅ Assembly API responded!");
            println!("    State: {:?}", response.state);
            println!("    Missing chunks: {}", response.missing_chunks.len());
            if let Some(detail) = response.detail {
                println!("    Detail: {}", detail);
            }
            
            match response.state {
                sentry_cli::api::ChunkedFileState::NotFound => {
                    println!("  ℹ️  'NotFound' is expected since we didn't upload chunks first");
                }
                _ => {
                    println!("  🎯 Unexpected state for non-uploaded chunks");
                }
            }
        }
        Err(e) => {
            println!("  ❌ Assembly API call failed: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}

fn main() -> Result<()> {
    // Configuration
    let auth_token = "";
    let base_url = "http://localhost:8000";
    let org = "sentry";
    let project = "internal";
    let test_file = "./TestUploads/HackerNews_arm64";
    
    println!("🧪 Sentry CLI Full Upload Test (Debug Files)");
    println!("=============================================");
    println!("Base URL: {}", base_url);
    println!("Organization: {}", org);
    println!("Project: {}", project);
    println!("Test binary: {}", test_file);
    println!();
    
    // Create and bind config
    let config = create_test_config(auth_token, base_url)?;
    config.bind_to_process();
    
    // Initialize API
    let api = Api::current();
    
    // Test authentication
    println!("🔐 Testing authentication...");
    match api.authenticated()?.get_auth_info() {
        Ok(auth_info) => {
            println!("✅ Authentication successful!");
            if let Some(user) = auth_info.user {
                println!("  User: {}", user.email);
            }
            if let Some(auth) = auth_info.auth {
                println!("  Scopes: {:?}", auth.scopes);
            }
        }
        Err(e) => {
            println!("❌ Authentication failed: {}", e);
            return Err(e.into());
        }
    }
    println!();
    
    // // Test 1: Check chunk upload support
    // if let Err(e) = test_chunk_upload_options(org) {
    //     println!("❌ Chunk upload not supported, cannot proceed with full test: {}", e);
    //     return Err(e);
    // }
    println!();
    
    // Test 2: Assembly API only (like the original test) - COMMENTED OUT FOR DSYM TEST
    // if let Err(e) = test_assembly_only(org, &[project.to_string()]) {
    //     println!("❌ Assembly-only test failed: {}", e);
    // }
    // println!();
    
    // Test 3: Full chunk upload + assembly flow
    if std::path::Path::new(test_file).exists() {
        if let Err(e) = test_full_debug_file_upload_flow(org, project, test_file) {
            println!("❌ Full upload flow test failed: {}", e);
            return Err(e);
        }
    } else {
        println!("⚠️  Test file {} not found, skipping full upload test", test_file);
    }
    
    println!();
    println!("🏁 All tests completed!");
    
    Ok(())
} 