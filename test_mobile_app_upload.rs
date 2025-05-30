use anyhow::Result;

// Include the sentry-cli library
extern crate sentry_cli;

use sentry_cli::api::{Api, ChunkUploadCapability, ChunkedPreprodArtifactRequest};
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
            

            // Check specifically for preprod artifacts capability
            if !options.supports(ChunkUploadCapability::PreprodArtifacts) {
                println!("  ⚠️  PreprodArtifacts capability not supported");
                return Err(anyhow::anyhow!("PreprodArtifacts capability not supported"));
            }
            
            Ok(())
        }
        None => {
            println!("❌ Chunk upload not supported by server");
            Err(anyhow::anyhow!("Chunk upload not supported"))
        }
    }
}

fn test_full_mobile_app_upload_flow(org: &str, project: &str, archive_path: &str) -> Result<()> {
    println!("🚀 Testing full mobile app archive upload flow...");
    
    // Read the archive file directly
    let archive_path = std::path::Path::new(archive_path);
    println!("  📦 Archive file: {}", archive_path.display());
    
    // Read the archive file
    let content = std::fs::read(&archive_path)
        .map_err(|e| anyhow::anyhow!("Failed to read archive file {}: {}", archive_path.display(), e))?;
    
    println!("  📄 File size: {} bytes ({:.2} MB)", content.len(), content.len() as f64 / 1024.0 / 1024.0);
    
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
    let data = &content;
    let chunk_size = chunk_upload_options.chunk_size as usize;
    let (total_checksum, chunk_checksums) = get_sha1_checksums(data, chunk_size)?;
    
    println!("  🔢 Checksum calculation:");
    println!("    • Total archive SHA1: {}", total_checksum);
    println!("    • Number of chunks: {}", chunk_checksums.len());
    
    // Show individual chunk details (limit for large files)
    let max_chunks_to_show = 5;
    for (i, checksum) in chunk_checksums.iter().enumerate().take(max_chunks_to_show) {
        let chunk_start = i * chunk_size;
        let chunk_end = std::cmp::min(chunk_start + chunk_size, data.len());
        let chunk_size_actual = chunk_end - chunk_start;
        println!("      Chunk {}: {} bytes (SHA1: {})", i + 1, chunk_size_actual, checksum);
    }
    if chunk_checksums.len() > max_chunks_to_show {
        println!("      ... and {} more chunks", chunk_checksums.len() - max_chunks_to_show);
    }
    
    // Step 2: Upload all chunks (preprod artifacts likely need all chunks)
    println!("  🚀 Uploading chunks...");
    let chunks_to_upload: Vec<_> = data.chunks(chunk_size)
        .zip(chunk_checksums.iter())
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
    
    // Step 3: Assemble using the preprod artifact endpoint
    println!("  🔧 Assembling mobile app artifact...");
    
    // Create the simple preprod artifact request (mirroring DIF request pattern)
    let assembly_request = ChunkedPreprodArtifactRequest::new(
        total_checksum,
        &chunk_checksums,
    );
    
    println!("    📡 Request JSON:");
    println!("{}", serde_json::to_string_pretty(&assembly_request)?);
    
    let assembly_response = authenticated_api.assemble_preprod_artifact(org, project, &assembly_request)?;
    
    println!("    📦 Mobile app assembly initiated!");
    println!("      State: {:?}", assembly_response.state);
    println!("      Missing chunks: {}", assembly_response.missing_chunks.len());
    
    if let Some(ref detail) = assembly_response.detail {
        println!("      Detail: {}", detail);
    }
    
    // Step 4: Check assembly status
    match assembly_response.state {
        sentry_cli::api::ChunkedFileState::Ok => {
            println!("    ✅ Assembly completed successfully!");
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
    
    if assembly_response.missing_chunks.is_empty() {
        println!("  🎉 Full mobile app upload flow completed successfully!");
        println!("    📈 Summary:");
        println!("      • Archive: {}", archive_path.display());
        println!("      • Total chunks: {}", chunk_checksums.len());
        println!("      • File checksum: {}", total_checksum);
        println!("      • Organization: {}", org);
        println!("      • Project: {}", project);
        println!("      • Endpoint: /projects/{}/{}/files/preprodartifacts/assemble/", org, project);
    } else {
        println!("  ⚠️  Some chunks are still missing after upload attempt");
        println!("      Missing: {:?}", assembly_response.missing_chunks);
    }
    
    Ok(())
}

fn main() -> Result<()> {
    // Configuration
    let auth_token = "";
    let base_url = "http://localhost:8000";
    let org = "sentry";
    let project = "internal";
    let test_archive = "./TestUploads/HackerNews.xcarchive.zip";
    
    println!("📱 Sentry CLI Mobile App Upload Test");
    println!("====================================");
    println!("Base URL: {}", base_url);
    println!("Organization: {}", org);
    println!("Project: {}", project);
    println!("Test archive: {}", test_archive);
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
    
    // Test 1: Check chunk upload support
    if let Err(e) = test_chunk_upload_options(org) {
        println!("❌ Chunk upload not supported, cannot proceed with test: {}", e);
        return Err(e);
    }
    println!();
    
    // Test 2: Full chunk upload + assembly flow
    if std::path::Path::new(test_archive).exists() {
        if let Err(e) = test_full_mobile_app_upload_flow(org, project, test_archive) {
            println!("❌ Full mobile app upload flow test failed: {}", e);
            return Err(e);
        }
    } else {
        println!("⚠️  Test archive {} not found, skipping upload test", test_archive);
    }
    
    println!();
    println!("🏁 Mobile app upload test completed!");
    println!();
    println!("📝 Next steps:");
    println!("   1. Test with different archive types and sizes");
    println!("   2. Verify server-side processing of the preprodartifact/assemble endpoint");
    println!("   3. Integration with the mobile_app upload command");
    
    Ok(())
} 