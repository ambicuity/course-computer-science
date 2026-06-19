#include <stdio.h>

typedef enum { CLOSED, SYN_SENT, ESTABLISHED } tcp_state_t;

int main(void) {
    tcp_state_t st = CLOSED;
    printf("state=%d\n", st);

    st = SYN_SENT;   // send SYN
    printf("send SYN -> state=%d\n", st);

    st = ESTABLISHED; // received SYN-ACK, sent ACK
    printf("recv SYN-ACK/send ACK -> state=%d\n", st);

    return 0;
}
