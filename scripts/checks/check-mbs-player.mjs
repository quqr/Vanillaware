import { Report } from "../support/assert.mjs";

const {
  applyMatrix,
  boneT,
  engineMatrix,
  mixS7,
  newBoneStates,
  quadCorners,
  splineWeights,
  stepBone,
  TICK_UNITS,
} = await import(new URL("../../src/lib/sceneDraw.ts", import.meta.url).href);

const report = new Report();

function close(actual, expected, label, tolerance = 1e-6) {
  if (!(Math.abs(actual - expected) <= tolerance)) {
    report.add(`${label}: ${actual} != ${expected}`);
  }
}

function closeAll(actual, expected, label, tolerance = 1e-6) {
  if (actual.length !== expected.length) {
    report.add(`${label}: length ${actual.length} != ${expected.length}`);
    return;
  }
  for (let i = 0; i < actual.length; i += 1) {
    close(actual[i], expected[i], `${label}[${i}]`, tolerance);
  }
}

// --- Catmull-Rom weights reproduce the frame endpoints and stay normalised.
closeAll(splineWeights(0), [0, 1, 0, 0], "spline t=0", 1e-9);
closeAll(splineWeights(1), [0, 0, 1, 0], "spline t=1", 1e-9);
for (const t of [0.1, 0.25, 0.5, 0.75, 0.9]) {
  const sum = splineWeights(t).reduce((a, b) => a + b, 0);
  close(sum, 1, `spline sum t=${t}`, 1e-9);
}

// --- Catmull-Rom is exact on evenly spaced (linear) data.
{
  const frames = [0, 10, 20, 30].map((offset) => valuesOf([offset, offset, offset], [offset, offset]));
  const out = mixS7(frames[0], frames[1], frames[2], frames[3], 0.5, 2);
  close(out.move[0], 15, "catmull midpoint", 1e-9);
  const quarter = mixS7(frames[0], frames[1], frames[2], frames[3], 0.25, 2);
  close(quarter.move[0], 12.5, "catmull quarter", 1e-9);
}

// --- Linear mix and the "no mix" identity, all on the eight s7 floats.
{
  const cur = valuesOf([2, 2, 2], [2, 2]);
  const next = valuesOf([6, 6, 6], [6, 6]);
  closeAll(mixS7(cur, cur, next, next, 0.5, 1).move, [4, 4, 4], "linear move", 1e-9);
  closeAll(mixS7(cur, cur, next, next, 0.5, 1).scale, [4, 4], "linear scale", 1e-9);
  closeAll(mixS7(cur, cur, next, next, 0.5, 0).move, [2, 2, 2], "mix id 0 keeps cur", 1e-9);
}

// --- The engine matrix is row major with translation in the last row.
{
  closeAll(
    engineMatrix(valuesOf([0, 0, 0], [1, 1]), 0),
    [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
    "identity matrix",
    1e-12,
  );
  const moved = engineMatrix(valuesOf([3, 4, 5], [1, 1]), 0);
  closeAll(applyMatrix(moved, 0, 0), [3, 4], "translation lands in the last row", 1e-12);
  close(moved[11], 0, "last row of the basis stays zero", 1e-12);
}

// --- Rotation is right-multiplied in the Z, X, Y order of f_700058e014.
{
  const rz = engineMatrix(valuesOf([0, 0, 0], [1, 1], [0, 0, Math.PI / 2]), 0);
  closeAll(applyMatrix(rz, 1, 0), [0, 1], "Z rotation", 1e-9);

  const rx = engineMatrix(valuesOf([0, 0, 0], [1, 1], [Math.PI / 2, 0, 0]), 0);
  closeAll(applyMatrix(rx, 0, 1), [0, 0], "X rotation flattens Y", 1e-9);

  const ry = engineMatrix(valuesOf([0, 0, 0], [1, 1], [0, Math.PI / 2, 0]), 0);
  closeAll(applyMatrix(ry, 1, 0), [0, 0], "Y rotation flattens X", 1e-9);

  // Applying X before Z would squash the point to the origin instead.
  const zx = engineMatrix(valuesOf([0, 0, 0], [1, 1], [Math.PI / 2, 0, Math.PI / 2]), 0);
  closeAll(applyMatrix(zx, 0, 1), [-1, 0], "Z runs before X", 1e-9);

  // Scale is applied before the rotation, not after.
  const scaled = engineMatrix(valuesOf([0, 0, 0], [2, 1], [0, 0, Math.PI / 2]), 0);
  closeAll(applyMatrix(scaled, 1, 0), [0, 2], "scale runs before rotation", 1e-9);
}

// --- Frame flags bit 0 and bit 1 mirror a whole column.
{
  const base = valuesOf([0, 0, 0], [1, 1]);
  closeAll(applyMatrix(engineMatrix(base, 0x1), 2, 3), [-2, 3], "flip X", 1e-12);
  closeAll(applyMatrix(engineMatrix(base, 0x2), 2, 3), [2, -3], "flip Y", 1e-12);
  const moved = valuesOf([5, 6, 0], [1, 1]);
  closeAll(applyMatrix(engineMatrix(moved, 0x3), 0, 0), [-5, -6], "flip mirrors translation", 1e-12);
}

// --- s2/s1 records are centre, four corners, first corner again.
{
  const record = [9, 9, 0, 0, 4, 0, 4, 4, 0, 4, 0, 0];
  closeAll(quadCorners(record), [0, 0, 4, 0, 4, 4, 0, 4], "corners are [2..10]", 0);
}

// --- Interpolation coefficient follows f_700058e80c.
function animationOf(frames) {
  return { id: 0, name: "", loopId: -1, frames };
}

function valuesOf(move, scale, rotate = [0, 0, 0]) {
  return { move, rotate, scale };
}

function frameOf(time, extra = {}) {
  return {
    time,
    flags: "0x20",
    attach: null,
    color: null,
    matrix: new Array(16).fill(0),
    move: [0, 0, 0],
    rotate: [0, 0, 0],
    scale: [1, 1],
    inRate: 1,
    matrixMixId: 0,
    dstQuadMixId: 0,
    fogQuadMixId: 0,
    hitQuadMixId: 0,
    ...extra,
  };
}

{
  const animation = animationOf([frameOf(8)]);
  const state = { frameIndex: 0, remaining: 8 * TICK_UNITS, done: false };
  close(boneT(animation, state), 0, "t at frame start");
  state.remaining = 4 * TICK_UNITS;
  close(boneT(animation, state), 0.5, "t at half");
  state.remaining = 0;
  close(boneT(animation, state), 1, "t at frame end");
}

// --- in_rate quantises the elapsed time into 1/rate steps.
{
  const animation = animationOf([frameOf(8, { inRate: 4 })]);
  const state = { frameIndex: 0, remaining: 5 * TICK_UNITS, done: false };
  // elapsed = 3 frames -> trunc(3/4)*4 = 0 -> t = 0
  close(boneT(animation, state), 0, "rate 4 quantises to 0");
  state.remaining = 1 * TICK_UNITS;
  // elapsed = 7 -> trunc(7/4)*4 = 4 -> t = 4/8
  close(boneT(animation, state), 0.5, "rate 4 quantises to 4/8");
}

// --- A single 60 Hz tick consumes exactly 1000 units.
{
  const animation = animationOf([frameOf(1), frameOf(1)]);
  const [state] = newBoneStates(1);
  const tickMs = 1000 / 60;
  report.check(stepBone(animation, state, tickMs * 0.5, 1) === false, "half tick keeps the frame");
  report.check(stepBone(animation, state, tickMs * 0.5, 1) === true, "second half advances");
  report.equal(state.frameIndex, 1, "frame index after one tick");
  report.check(!state.done, "track not finished");
}

// --- Long stalls skip the frames that would have been missed.
{
  const animation = animationOf([frameOf(2), frameOf(2), frameOf(2), frameOf(2)]);
  const [state] = newBoneStates(1);
  const skipped = stepBone(animation, state, (1000 / 60) * 2.5, 1);
  report.check(skipped, "long stall reports a change");
  report.equal(state.frameIndex, 1, "long stall advances one whole frame");
}

// --- A track that runs out of frames and has no loop target stops on the last one.
{
  const animation = animationOf([frameOf(1), frameOf(1)]);
  const [state] = newBoneStates(1);
  stepBone(animation, state, (1000 / 60) * 10, 1);
  report.equal(state.frameIndex, 1, "clamped to the last frame");
  report.check(state.done, "non looping track finishes");
}

// --- A loop target rewinds instead of stopping.
{
  const animation = { id: 0, name: "", loopId: 1, frames: [frameOf(1), frameOf(1), frameOf(1)] };
  const [state] = newBoneStates(1);
  stepBone(animation, state, (1000 / 60) * 5, 1);
  report.check(!state.done, "looping track keeps running");
  report.check(state.frameIndex >= 1 && state.frameIndex <= 2, "looping track stays inside the loop");
}

// --- resolveBones follows the engine: the quad comes from the current keyframe
// untouched, only the s7 record is tweened and the matrix rebuilt from it.
{
  const { resolveBones } = await import(new URL("../../src/lib/sceneDraw.ts", import.meta.url).href);
  const layerOf = (offset) => ({
    flags: "0x1",
    dst: [0, 0, offset, 0, offset + 2, 0, offset + 2, 2, offset, 2, offset, 0],
    src: [0, 0, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0],
    texId: 0,
    blendId: 0,
    fog: null,
    colorize: null,
    attribute: null,
  });
  const scene = {
    mbsPath: "",
    tag: "",
    atlas: [],
    keyframe: [
      { name: "k0", flags: "0x0", layers: [layerOf(0)] },
      { name: "k1", flags: "0x0", layers: [layerOf(10)] },
    ],
    hitbox: [],
    animations: [],
    skeletons: [],
    selfCheck: [],
  };
  const animation = {
    id: 0,
    name: "mix",
    loopId: -1,
    frames: [
      frameOf(2, {
        attach: { kind: "keyframe", id: 0 },
        matrixMixId: 1,
        dstQuadMixId: 1,
        move: [0, 0, 0],
      }),
      frameOf(2, {
        attach: { kind: "keyframe", id: 1 },
        matrixMixId: 1,
        dstQuadMixId: 1,
        move: [10, 0, 0],
      }),
    ],
  };
  const animations = new Map([[0, animation]]);
  const state = { frameIndex: 0, remaining: 1000, done: false };
  const resolved = resolveBones(scene, [{ kind: "animation", id: 0 }], [state], animations);
  report.equal(resolved.layers.length, 1, "one layer resolved");
  close(resolved.layers[0].layer.dst[2], 0, "quad stays on the current keyframe even with in_s6 on", 1e-9);
  close(resolved.layers[0].matrix[12], 5, "translation tweened at t=0.5", 1e-9);

  const noMix = { ...animation, frames: [frameOf(2, { attach: { kind: "keyframe", id: 0 } })] };
  const plain = resolveBones(
    scene,
    [{ kind: "animation", id: 0 }],
    [{ frameIndex: 0, remaining: 1000, done: false }],
    new Map([[0, noMix]]),
  );
  close(plain.layers[0].matrix[12], 0, "mix id 0 keeps the current transform", 1e-9);
}

// --- The layer panel hides a record by its own content, and a ticked
// attribute bit drops every layer that carries it, mirroring the engine's
// `(object_mask & layer_mask) != 0 -> skip` test.
{
  const { layerKey, layerVisible, NO_FILTER } = await import(
    new URL("../../src/lib/sceneDraw.ts", import.meta.url).href
  );
  const quad = (offset) => [0, 0, offset, 0, offset + 10, 0, offset + 10, 10, offset, 10, offset, 0];
  const base = {
    flags: "0x1",
    dst: quad(0),
    src: [0, 0, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0],
    texId: 3,
    blendId: 0,
    fog: null,
    colorize: null,
    attribute: null,
  };
  report.check(layerKey(base) !== layerKey({ ...base, dst: quad(20) }), "different quads split the key");
  report.check(
    layerKey(base) === layerKey({ ...base, colorize: "COLOR_1" }),
    "cosmetic fields stay out of the key",
  );
  report.check(layerVisible(base, NO_FILTER), "visible by default");
  report.check(
    !layerVisible(base, { hidden: new Set([layerKey(base)]), attributes: new Set() }),
    "manual hide wins",
  );

  const tagged = { ...base, attribute: ["keyattr_1"] };
  report.check(
    !layerVisible(tagged, { hidden: new Set(), attributes: new Set(["keyattr_1"]) }),
    "a ticked attribute bit hides the tagged layer",
  );
  report.check(
    layerVisible(tagged, { hidden: new Set(), attributes: new Set(["keyattr_2"]) }),
    "an unrelated bit keeps it",
  );
  report.check(
    layerVisible(base, { hidden: new Set(), attributes: new Set(["keyattr_1"]) }),
    "an untagged layer survives the mask",
  );
}

process.exit(report.print());
