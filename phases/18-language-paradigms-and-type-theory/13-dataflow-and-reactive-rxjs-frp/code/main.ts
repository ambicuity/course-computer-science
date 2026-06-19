type Stream<T> = T[];

function mapS<A, B>(xs: Stream<A>, f: (x: A) => B): Stream<B> {
  return xs.map(f);
}

function filterS<A>(xs: Stream<A>, p: (x: A) => boolean): Stream<A> {
  return xs.filter(p);
}

function scanS<A, B>(xs: Stream<A>, seed: B, f: (acc: B, x: A) => B): Stream<B> {
  const out: B[] = [];
  let acc = seed;
  for (const x of xs) {
    acc = f(acc, x);
    out.push(acc);
  }
  return out;
}

const events = [1, 2, 3, 4, 5, 6];
const doubled = mapS(events, (x) => x * 2);
const even = filterS(doubled, (x) => x % 4 === 0);
const running = scanS(even, 0, (a, x) => a + x);

console.log({ events, doubled, even, running });
