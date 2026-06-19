/*
 * kernel.c — Minimal RISC-V kernel
 *
 * Communicates with the outside world through UART0,
 * a memory-mapped serial device at 0x10000000.
 */

/* UART0 base address on QEMU virt machine */
#define UART0_BASE  0x10000000UL

/* UART registers (offset from base) */
#define UART_THR    0x00    /* Transmit Holding Register (write) */
#define UART_LSR    0x05    /* Line Status Register (read) */
#define LSR_TX_READY 0x20   /* THR is empty, ready for next byte */

/*
 * Pointer to UART transmit register.
 * volatile: prevent compiler from optimizing away writes.
 * The hardware reads this address, not memory.
 */
static volatile unsigned char *uart_tx =
    (volatile unsigned char *)(UART0_BASE + UART_THR);

static volatile unsigned char *uart_lsr =
    (volatile unsigned char *)(UART0_BASE + UART_LSR);

/*
 * putchar — Send a single character through the UART.
 *
 * Waits until the UART's transmit buffer is empty,
 * then writes the character. The UART hardware serializes
 * the byte and sends it over the serial line.
 */
void putchar(char c)
{
    /* Wait for UART to be ready to accept a new character */
    while ((*uart_lsr & LSR_TX_READY) == 0)
        ;

    /* Write character to transmit register */
    *uart_tx = (unsigned char)c;
}

/*
 * puts — Print a null-terminated string followed by a newline.
 */
void puts(const char *s)
{
    while (*s) {
        putchar(*s);
        s++;
    }
    putchar('\n');
}

/*
 * kernel_main — Entry point called by boot.s
 *
 * This is the first C function to run.
 * There is no standard library, no heap, no OS.
 * Just your code and the hardware.
 */
void kernel_main(void)
{
    puts("Hello, Kernel!");
}
