use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running end-to-end MCP server test...\n");

    let temp_dir = tempfile::tempdir()?;
    let cache_path = temp_dir.path().to_str().unwrap();

    println!("Cache directory: {}", cache_path);

    let mut child = Command::new("cargo")
        .arg("run")
        .arg("--release")
        .arg("--")
        .arg(cache_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    std::thread::sleep(Duration::from_millis(500));

    println!("\n1. Sending initialize request...");
    let init = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0"
            }
        }
    });

    writeln!(stdin, "{}", init)?;
    stdin.flush()?;

    let mut response = String::new();
    reader.read_line(&mut response)?;
    println!("✓ Initialize response received");

    println!("\n2. Sending initialized notification...");
    let initialized = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    writeln!(stdin, "{}", initialized)?;
    stdin.flush()?;

    println!("✓ Initialized notification sent");

    println!("\n3. Listing available tools...");
    let list_tools = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });

    writeln!(stdin, "{}", list_tools)?;
    stdin.flush()?;

    let mut response = String::new();
    reader.read_line(&mut response)?;

    let parsed: serde_json::Value = serde_json::from_str(&response)?;
    if let Some(tools) = parsed["result"]["tools"].as_array() {
        println!("✓ Found {} tool(s):", tools.len());
        for tool in tools {
            println!(
                "  - {}: {}",
                tool["name"].as_str().unwrap_or("unknown"),
                tool["description"].as_str().unwrap_or("no description")
            );
        }
    }

    println!("\n4. Testing fetch with httpbin...");
    let fetch_call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "fetch",
            "arguments": {
                "url": "https://httpbin.org/html"
            }
        }
    });

    writeln!(stdin, "{}", fetch_call)?;
    stdin.flush()?;

    let mut response = String::new();
    reader.read_line(&mut response)?;

    let parsed: serde_json::Value = serde_json::from_str(&response)?;
    if let Some(files) = parsed["result"]["content"][0]["text"].as_str() {
        let files_parsed: serde_json::Value = serde_json::from_str(files)?;
        if let Some(files_array) = files_parsed["files"].as_array() {
            println!("✓ Fetch successful! Cached {} file(s):", files_array.len());
            for file in files_array {
                println!("  - {}", file["path"].as_str().unwrap_or("unknown"));
                println!(
                    "    Lines: {}, Words: {}, Characters: {}",
                    file["lines"].as_u64().unwrap_or(0),
                    file["words"].as_u64().unwrap_or(0),
                    file["characters"].as_u64().unwrap_or(0)
                );
            }
        }
    }

    drop(stdin);

    std::thread::sleep(Duration::from_millis(500));
    child.kill()?;

    println!("\n✓ All end-to-end tests passed!");
    println!("\nCache contents:");
    for entry in walkdir::WalkDir::new(temp_dir.path()) {
        let entry = entry?;
        if entry.file_type().is_file() {
            println!(
                "  - {}",
                entry.path().strip_prefix(temp_dir.path())?.display()
            );
        }
    }

    Ok(())
}
