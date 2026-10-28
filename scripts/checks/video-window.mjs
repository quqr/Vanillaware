import { Report } from "../support/assert.mjs";

const { mergeFrameBatch } = await import(
  new URL("../../src/lib/frameCache.ts", import.meta.url).href
);
const { gopIndexAt, gopRange, nextGopToLoad } = await import(
  new URL("../../src/lib/videoWindow.ts", import.meta.url).href
);

const report = new Report();

const keyframes = [0, 30, 60, 90];
const total = 120;
const cache = new Set();
for (let frame = 0; frame < 30; frame += 1) cache.add(frame);
const has = (frame) => cache.has(frame);
const ask = (current, loaded) =>
  nextGopToLoad({ keyframes, total, current, loaded, has, prefetchAhead: 12 });

const gop0 = gopRange(keyframes, total, 0);
const gop1 = gopRange(keyframes, total, 1);
const cases = [
  ["no refetch inside a segment", ask(0, gop0), null],
  ["before the prefetch line", ask(17, gop0), null],
  ["prefetch when 12 frames from the end", ask(18, gop0), 1],
  ["next segment already cached", ask(20, gop1), null],
  ["seek to a distant segment", ask(100, gop1), 3],
  ["last segment never prefetches", ask(119, gopRange(keyframes, total, 3)), null],
  ["frame index past the end", gopIndexAt(keyframes, 9999), 3],
];
for (const [label, actual, expected] of cases) {
  report.equal(actual, expected, `window: ${label}`);
}

for (let frame = 30; frame < 60; frame += 1) cache.add(frame);
report.check(
  ask(25, gop0) === null,
  "window: refetches a segment that is already cached",
);
console.log(`window: ${cases.length + 1} assertions`);

const fake = () => ({
  closed: false,
  close() {
    this.closed = true;
  },
});
const range = (from, count) =>
  Array.from({ length: count }, (_, i) => ({ index: from + i, bitmap: fake() }));

let merge = mergeFrameBatch(new Map(), range(0, 30), 0);
report.equal(merge.next.size, 30, "cache: first segment");
report.equal(merge.dropped.length, 0, "cache: first segment drops nothing");

merge = mergeFrameBatch(merge.next, range(30, 30), 0);
report.equal(merge.next.size, 60, "cache: prefetched segment stays");
report.equal(merge.dropped.length, 0, "cache: prefetch drops nothing");

merge = mergeFrameBatch(merge.next, range(60, 30), 30);
report.equal(merge.next.size, 60, "cache: third segment");
report.equal(merge.dropped.length, 30, "cache: third segment evicts one segment");
report.check(!merge.next.has(0), "cache: oldest segment still present");
report.check(merge.next.has(89), "cache: newest segment missing");

const stale = fake();
const previous = new Map([[30, stale]]);
const replacement = { index: 30, bitmap: fake() };
const replaced = mergeFrameBatch(previous, [replacement], 0);
report.check(replaced.next.get(30) === replacement.bitmap, "cache: same frame not replaced");
report.check(replaced.dropped.includes(stale), "cache: superseded bitmap not handed back");
report.check(previous.get(30) === stale, "cache: merge mutated its input");

let mutable = range(0, 30);
const snapshot = mutable;
const lazyUpdater = () => mergeFrameBatch(new Map(), mutable, 0);
mutable = [];
report.equal(lazyUpdater().next.size, 0, "cache: lazy updater should see an empty array");
report.equal(
  mergeFrameBatch(new Map(), snapshot, 0).next.size,
  30,
  "cache: immutable snapshot lost frames",
);
console.log("cache: merge assertions including the deferred-updater accident");

process.exit(report.print());
