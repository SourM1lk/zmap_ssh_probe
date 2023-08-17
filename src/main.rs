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

const MAX_PARALLEL_JOBS: usize = 1;
static IMPORTED: AtomicUsize = AtomicUsize::new(0);
static CHECKED: AtomicUsize = AtomicUsize::new(0);
static COMBOS_CHECKED: AtomicUsize = AtomicUsize::new(0);
static SUCCESS: AtomicUsize = AtomicUsize::new(0);
static FAILED: AtomicUsize = AtomicUsize::new(0);
static TIMEOUTS: AtomicUsize = AtomicUsize::new(0);

struct LoginAttempt {
    ip: String,
    username: String,
    password: String,
    success: bool,
}

fn check_ssh_login(ip: String, credentials: Vec<(String, String)>, results: mpsc::Sender<LoginAttempt>) {
    let timeout = 5_000; // milliseconds

    for (username, password) in &credentials {
        let mut session = Session::new().unwrap();
        session.set_timeout(timeout);

        if let Ok(_) = net::TcpStream::connect(&ip) {
            match session.handshake() {
                Ok(_) => {
                    let auth_success = session.userauth_password(username, password).is_ok();
                    results.send(LoginAttempt {
                        ip: ip.clone(),
                        username: username.clone(),
                        password: password.clone(),
                        success: auth_success,
                    }).unwrap();
                    session.disconnect(None, "Closing session", None).unwrap();

                    COMBOS_CHECKED.fetch_add(1, Ordering::Relaxed);

                    if auth_success {
                        SUCCESS.fetch_add(1, Ordering::Relaxed);
                        break;  // exit loop if a successful login was found for the IP
                    } else {
                        FAILED.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Err(e) => {
                    if e.to_string().contains("timeout") {
                        TIMEOUTS.fetch_add(1, Ordering::Relaxed);
                    } else {
                        println!("Error during handshake for {}: {:?}", ip, e);
                        FAILED.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        } else {
            CHECKED.fetch_add(1, Ordering::Relaxed);
            FAILED.fetch_add(1, Ordering::Relaxed);
            break; // If you can't connect to the IP, no point in trying other username/password combinations
        }
    }
}


fn main() {
    let (ip_tx, ip_rx) = mpsc::channel();
    let (results_tx, results_rx) = mpsc::channel();


    thread::spawn(|| {
        println!("+-------------------------------------+");
        println!("|             SSH SCANNER             |");
        println!("+-------------------------------------+");
        
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
            print!("\rIPs Imported: {} | IPs Checked: {} | Combos Checked: {} | Successful: {} | Failed: {} | Timeouts: {} ", 
                imported_count.to_string().blue(),
                checked_count.to_string().blue(),
                combos_checked_count.to_string().yellow(), 
                success_count.to_string().green(),
                failed_count.to_string().red(),
                timeouts_count.to_string().magenta()
            );

            io::stdout().flush().unwrap(); // Ensure that the print! macro actually writes its output immediately
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

    let credentials = vec![
        ("username1".to_string(), "password1".to_string()),
        ("username2".to_string(), "password2".to_string()),
    ];

    let pool = ThreadPool::new(MAX_PARALLEL_JOBS);

    loop {
        match ip_rx.recv() {
            Ok(ip) => {
                let results_tx = results_tx.clone();
                let creds = credentials.clone();
                pool.execute(move || {
                    println!("Checking {}", ip);
                    check_ssh_login(ip, creds, results_tx);
                });
            }
            Err(_) => {
                // Channel is closed, no more IPs
                break;
            }
        }
    }

    //drop(results_tx);  // close the sender, so we can iterate until receiver is empty

    let file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("results.txt")
        .unwrap();
    let mut file = BufWriter::new(file);

}
