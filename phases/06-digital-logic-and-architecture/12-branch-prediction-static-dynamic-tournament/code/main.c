#include <stdio.h>

typedef enum { SN=0, WN=1, WT=2, ST=3 } state_t;

static state_t update(state_t s, int taken) {
  if (taken) return s == ST ? ST : (state_t)(s + 1);
  return s == SN ? SN : (state_t)(s - 1);
}

int main(void) {
  int trace[] = {1,1,0,1,0,0,1,1};
  state_t s = WT;
  for (int i = 0; i < (int)(sizeof(trace)/sizeof(trace[0])); i++) {
    int pred = (s >= WT);
    printf("step=%d pred=%d actual=%d\n", i, pred, trace[i]);
    s = update(s, trace[i]);
  }
  return 0;
}
