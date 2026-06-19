/* calc.h — public interface of the calc module. */
#ifndef CALC_H
#define CALC_H

int add(int a, int b);
int sub(int a, int b);

/* Number of arithmetic calls performed since program start. */
int read_counter(void);

#endif /* CALC_H */
