# CTF Exploit Development Toolkit

A reusable Python + Shell toolkit for CTF binary exploitation challenges.

## What's Inside

The toolkit provides four core capabilities:

| Capability | Description |
|------------|-------------|
| **Binary Analysis** | Load an ELF binary and inspect its security properties (canary, NX, PIE, RELRO), symbols, PLT/GOT entries, and ROP gadgets |
| **ret2win** | Exploit a buffer overflow by redirecting execution to a win/flag/shell function |
| **ROP ret2libc** | Two-stage exploit: leak libc address via PLT/GOT, then call `system("/bin/sh")` to get a shell |
| **Remote Automation** | Connect to remote CTF challenge servers, run exploits, and capture flags |

## Installation

Requires Python 3.6+ and pwntools:

```bash
# Install pwntools
pip3 install pwntools

# The run.sh script auto-installs pwntools if missing
```

Optional tools for enhanced functionality:
- `GDB` + `pwndbg` / `GEF` / `PEDA` — for debugging exploits
- `objdump` / `readelf` — for static binary analysis
- `ROPgadget` — for finding ROP gadgets
- `one_gadget` — for finding one-shot RCE addresses in libc

## Usage

```bash
# Analyze a binary
./run.sh analyze ./challenge_binary

# Find RIP overwrite offset
./run.sh offset ./challenge_binary

# ret2win exploit (local)
./run.sh ret2win ./challenge_binary

# ret2win exploit (remote)
./run.sh ret2win ./challenge_binary remote 10.0.0.2 1337

# ROP ret2libc exploit (local, with libc)
./run.sh rop ./challenge_binary ./libc.so.6

# Remote exploit (auto-mode)
./run.sh remote 10.0.0.2 1337 ./challenge_binary

# Download a challenge binary from a remote server
./run.sh download 10.0.0.2 1337 ./challenge_dir

# View binary protections (traditional checksec style)
./run.sh checksec ./challenge_binary

# Disassemble a function
./run.sh disassemble ./challenge_binary main
```

## Architecture

```
outputs/
├── README.md          # This file
├── exploits/          # Place your exploit scripts here
├── binaries/          # Downloaded challenge binaries
└── libc/              # libc shared objects for ret2libc

code/
├── main.py            # Python CTF toolkit
└── run.sh             # Shell automation wrapper
```

## Workflow for a New CTF Binary

1. **Analyze** — `./run.sh analyze challenge` → check protections, find useful symbols
2. **Offset** — `./run.sh offset challenge` → find RIP overwrite position
3. **Strategy decision:**
   - Has a `win` function → `./run.sh ret2win challenge`
   - No win function, NX enabled → `./run.sh rop challenge`
   - NX disabled → can use shellcode directly (modify exploit)
4. **Remote** — `./run.sh remote host port challenge`

## Examples

### ret2win on a local binary

```bash
$ ./run.sh analyze ./challenge
  Canary:  False
  NX:      True
  PIE:     False
  RELRO:   Partial

$ ./run.sh offset ./challenge
  RIP offset: 72

$ ./run.sh ret2win ./challenge
  [*] Sending payload...
  [*] Switching to interactive mode
  $ cat flag.txt
  FLAG{ret2win_success}
```

### ret2libc on a remote challenge

```bash
$ ./run.sh remote 10.0.0.2 1337 ./challenge rop ./libc.so.6
  [*] Connecting to 10.0.0.2:1337
  [+] Leaked puts@GOT: 0x7f1234567890
  [+] Libc base: 0x7f1234000000
  [*] system@libc:  0x7f1234556780
  [*] /bin/sh@libc: 0x7f12346789ab
  [*] Sending stage 2...
  [*] Switching to interactive mode
  $ cat flag
  FLAG{ret2libc_aslr_bypassed}
```

## Extending

Add new exploit techniques by subclassing or adding modules:

```python
from pwn import *

def format_string_exploit(binary, local=True):
    """Example: format string vulnerability exploit."""
    elf = ELF(binary)
    # ... exploit logic
```

Then add the command to `main.py` and `run.sh`.
