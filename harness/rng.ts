// xorshift32: the same seed gives the same journals on every machine. Shared by fuzz.ts and bigjournal.ts.
export function rng(start: number): () => number {
  let s = start >>> 0 || 1;
  return () => {
    s ^= s << 13;
    s >>>= 0;
    s ^= s >>> 17;
    s ^= s << 5;
    s >>>= 0;
    return s / 4294967296;
  };
}
