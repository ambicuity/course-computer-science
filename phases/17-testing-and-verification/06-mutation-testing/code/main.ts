function fee(amount: number): number {
  if (amount <= 0) return 0;
  if (amount < 100) return 5;
  return Math.floor(amount / 20);
}

function mutantLtToLe(amount: number): number {
  if (amount <= 0) return 0;
  if (amount <= 100) return 5;
  return Math.floor(amount / 20);
}

function mutantPlus(amount: number): number {
  if (amount <= 0) return 0;
  if (amount < 100) return 5;
  return Math.floor(amount / 20) + 1;
}

function tests(fn: (x: number) => number): boolean {
  return fn(-1) === 0 && fn(0) === 0 && fn(50) === 5 && fn(100) === 5 && fn(200) === 10;
}

for (const [name, fn] of Object.entries({ fee, mutantLtToLe, mutantPlus })) {
  console.log(`${name}: ${tests(fn) ? "PASS" : "FAIL"}`);
}
