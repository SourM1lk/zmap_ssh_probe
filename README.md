ZMAP SSH Probe

An efficient and user-friendly tool to scan and check SSH login credentials.

## Description

SSH Scanner is designed to help sysadmins and security professionals verify SSH login credentials across a range of IPs. It utilizes a multi-threaded approach for speedy checks, ensuring you get results in real-time.

Inspired by [Zmap-ProxyScanner](https://github.com/Yariya/Zmap-ProxyScanner).

## Features

- Multi-threaded scanning.
- Support for large IP ranges.
- Efficient error handling and reporting.
- Detailed logging of successful and failed login attempts.
- Integration with Zmap for IP scanning.

## Prerequisites

Before you begin, ensure you have met the following requirements:

- Rust installed on your machine.
- SSH library dependencies.
- Zmap installed (for IP scanning integration).
- (any other dependencies)

## Installation

1. Clone the repository:

\```bash
git clone https://github.com/your_username/ssh-scanner.git
\```

2. Navigate to the project directory:

\```bash
cd ssh-scanner
\```

3. Build the project:

\```bash
cargo build --release
\```

## Usage

To use SSH Scanner with Zmap:

\```bash
zmap -p 22 | ./target/release/ssh-scanner [OPTIONS]
\```

Options:

- `-i, --ip <IP>`: Specify the IP or IP range (can be piped from Zmap).
- `-p, --port <PORT>`: Set the SSH port (default: 22).
- `-c, --credentials <FILE>`: Provide a file with a list of username-password pairs.
- (any other options)

Example:

\```bash
zmap -p 22 | ./target/release/ssh-scanner -c credentials.txt
\```

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.
