use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use regex::Regex;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use url::Url;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Redis URL (e.g., redis://:password@127.0.0.1:6379/)
    #[arg(default_value = "redis://127.0.0.1:6379/")]
    url: String,

    /// Filter by key or command using regex
    #[arg(short, long)]
    filter: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let url = Url::parse(&args.url).context("Invalid Redis URL")?;
    let host = url.host_str().unwrap_or("127.0.0.1");
    let port = url.port().unwrap_or(6379);
    let addr = format!("{}:{}", host, port);

    let filter_re = match args.filter {
        Some(ref f) => Some(Regex::new(f).context("Invalid regex filter")?),
        None => None,
    };

    let mut stream = TcpStream::connect(&addr)
        .await
        .with_context(|| format!("Failed to connect to {}", addr))?;

    // Authenticate if password is provided
    if let Some(password) = url.password() {
        let auth_cmd = format!(
            "*2\r\n$4\r\nAUTH\r\n${}\r\n{}\r\n",
            password.len(),
            password
        );
        stream.write_all(auth_cmd.as_bytes()).await?;

        let mut resp = [0u8; 1024];
        let n = stream.try_read(&mut resp).unwrap_or(0);
        let resp_str = String::from_utf8_lossy(&resp[..n]);
        if !resp_str.starts_with("+OK") {
            anyhow::bail!("Authentication failed: {}", resp_str.trim());
        }
    }

    // Send MONITOR command
    let monitor_cmd = "*1\r\n$7\r\nMONITOR\r\n";
    stream.write_all(monitor_cmd.as_bytes()).await?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    // Read the +OK response from MONITOR
    reader.read_line(&mut line).await?;
    if !line.starts_with("+OK") {
        anyhow::bail!("Failed to start MONITOR: {}", line.trim());
    }

    println!("{}", "Started MONITOR stream...".green().bold());

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // RESP simple string response starting with '+'
        if let Some(msg) = trimmed.strip_prefix('+') {
            if let Some(ref re) = filter_re {
                if !re.is_match(msg) {
                    continue;
                }
            }

            // Simple coloring: time in dim, IP in blue, command in yellow, args in green
            // Example format: 17192837.123 [0 127.0.0.1:1234] "GET" "foo"

            if let Some(first_quote) = msg.find('"') {
                let prefix = &msg[..first_quote];
                let rest = &msg[first_quote..];

                // Colorize command parts (everything in quotes)
                let parts: Vec<&str> = rest.split('"').filter(|s| !s.trim().is_empty()).collect();

                print!("{}", prefix.dimmed());
                for (i, part) in parts.iter().enumerate() {
                    if i == 0 {
                        print!(" {}", part.yellow().bold());
                    } else {
                        print!(" {}", part.green());
                    }
                }
                println!();
            } else {
                println!("{}", msg);
            }
        }
    }

    Ok(())
}
