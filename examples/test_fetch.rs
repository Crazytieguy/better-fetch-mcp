use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing fetch functionality...\n");

    let test_urls = vec![
        "https://docs.convex.dev/",
        "https://svelte.dev",
        "https://httpbin.org/html",
    ];

    let cache_dir = PathBuf::from("/tmp/llms-fetch-test");

    // Clean up any existing cache
    if cache_dir.exists() {
        std::fs::remove_dir_all(&cache_dir)?;
    }

    for url in test_urls {
        println!("Testing URL: {}", url);

        // Create a temporary test
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        // Try to fetch the URL
        match client.get(url).send().await {
            Ok(response) => {
                println!("  Status: {}", response.status());
                println!(
                    "  Content-Type: {:?}",
                    response.headers().get("content-type")
                );

                if response.status().is_success() {
                    let body = response.text().await?;
                    println!("  Body length: {} bytes", body.len());
                    println!("  ✓ Success\n");
                } else {
                    println!("  ✗ Failed: Status not successful\n");
                }
            }
            Err(e) => {
                println!("  ✗ Error: {}\n", e);
            }
        }
    }

    println!("\nAll fetch tests completed!");
    Ok(())
}
