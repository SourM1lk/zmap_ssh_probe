# ZMAP SSH Probe

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
- Zmap installed (for IP scanning integration).

## Installation

1. Clone the repository:

```bash
git clone https://github.com/SourM1lk/zmap_ssh_probe.git
```

2. Navigate to the project directory:

```bash
cd zmap_ssh_probe
```

3. Build the project:

```bash
cargo build --release
```

****Note**: Ensure that the `credentials.txt` file is in the same directory as the executable before running the scanner.
**

## Usage

To use SSH Scanner with Zmap:

```bash
# Warning: This zmap command scans the internet. Ensure your ZMAP command targets only authorized IPs.
zmap -p 22 | ./target/release/zmap_ssh_probe [OPTIONS]
```

Options:
- `-p, --port <PORT>`: Specify the SSH port to target. Default is `22`.
- `-o, --output_file <FILENAME>`: Name of the file to which successful logins will be written. Default is `results.txt`.
- `-w, --workers <WORKER_COUNT>`: Number of workers to use for scanning. Default is `1000`.

Example:
****Note**: The scanner currently spawns 10 threads. FI you want to change this find `#[tokio::main(flavor = "multi_thread", worker_threads = 10)]` and change to the ammount of threads you want. This also splits your workers. Example 1000 workers across 10 threads equals 100 workers per thread.
**
```bash
# Warning: This zmap command scans the internet. Ensure your ZMAP command targets only authorized IPs.
zmap -p 22 | ./target/release/zmap_ssh_probe -p 22 -o TestResults.txt -w 500

```

## Disclaimer
```
This software is provided "as is" without warranty of any kind, either expressed or implied, including, but not limited to, the implied warranties of merchantability and fitness for a particular purpose. The entire risk as to the quality and performance of the software is with the user.

In no event will the authors or copyright holders be liable for any damages, including lost profits, lost savings, or other incidental or consequential damages arising out of the use or inability to use the software, even if the authors or copyright holders have been advised of the possibility of such damages.

This software is intended for educational and research purposes only. The authors do not encourage or condone the use of this software for malicious activities or any actions that violate the law. It is the responsibility of the user to ensure that their use of this software complies with all applicable laws and regulations. The authors will not be held responsible for any misuse of the software.
```

## License

[![Beerware License](https://img.shields.io/badge/License-Beerware-yellow.svg)](https://en.wikipedia.org/wiki/Beerware)

This project is licensed under the Beerware License. If you found this project enjoyable or useful, and we meet someday, you can buy me a beer in appreciation. Cheers! 🍻
