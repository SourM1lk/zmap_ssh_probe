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

```bash
git clone https://github.com/your_username/ssh-scanner.git
```

2. Navigate to the project directory:

```bash
cd ssh-scanner
```

3. Build the project:

```bash
cargo build --release
```

## Usage

To use SSH Scanner with Zmap:

```bash
zmap -p 22 | ./target/release/ssh-scanner [OPTIONS]
```

Options:

- `-i, --ip <IP>`: Specify the IP or IP range (can be piped from Zmap).
- `-p, --port <PORT>`: Set the SSH port (default: 22).
- `-c, --credentials <FILE>`: Provide a file with a list of username-password pairs.
- (any other options)

Example:

```bash
zmap -p 22 | ./target/release/ssh-scanner -c credentials.txt
```

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## Disclaimer
```
This software is provided "as is" without warranty of any kind, either expressed or implied, including, but not limited to, the implied warranties of merchantability and fitness for a particular purpose. The entire risk as to the quality and performance of the software is with the user.

In no event will the authors or copyright holders be liable for any damages, including lost profits, lost savings, or other incidental or consequential damages arising out of the use or inability to use the software, even if the authors or copyright holders have been advised of the possibility of such damages.

This software is intended for educational and research purposes only. The authors do not encourage or condone the use of this software for malicious activities or any actions that violate the law. It is the responsibility of the user to ensure that their use of this software complies with all applicable laws and regulations. The authors will not be held responsible for any misuse of the software.
```

/*
 * ----------------------------------------------------------------------------
 * "THE BEER-WARE LICENSE" (Revision 42):
 * SourMilk wrote this file. As long as you retain this notice you
 * can do whatever you want with this stuff. If we meet some day, and you think
 * this stuff is worth it, you can buy me a beer in return. - Sour
 * ----------------------------------------------------------------------------
 */


## License

[![Beerware License](https://img.shields.io/badge/License-Beerware-yellow.svg)](https://en.wikipedia.org/wiki/Beerware)

This project is licensed under the Beerware License. If you found this project enjoyable or useful, and we meet someday, you can buy me a beer in appreciation. Cheers! 🍻
