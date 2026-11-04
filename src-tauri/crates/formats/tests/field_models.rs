use formats::field;
use formats::mbs::quad::{
    Animation, Attach, Keyframe, KeyframeLayer, MixItem, Pad, Quad, Skeleton, TimelineEntry,
    Transform,
};
use formats::mbs::v55::FogQuad;

fn attach(kind: &str, id: i64) -> Attach {
    Attach {
        kind: kind.to_string(),
        id,
    }
}

fn layer(dst: [f32; 8], tex: i64) -> Pad<KeyframeLayer> {
    Pad::Filled(KeyframeLayer {
        debug: (String::new(), 0, 0, String::new()),
        dstquad: dst.to_vec(),
        blend_id: 0,
        fogquad: Some(FogQuad::One("#ffffffff".to_string())),
        tex_id: Some(tex),
        srcquad: Some(vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0]),
        attribute: None,
        colorize: None,
    })
}

fn rest() -> Transform {
    Transform {
        move_: [0.0; 3],
        rotate: [0.0; 3],
        scale: [1.0, 1.0],
    }
}

fn entry(time: i64, attach: Option<Attach>, transform: Transform) -> TimelineEntry {
    TimelineEntry {
        debug: String::new(),
        time,
        matrix_mix_id: 0,
        color_mix_id: 0,
        dstquad_mix_id: 0,
        fogquad_mix_id: 0,
        hitquad_mix_id: 0,
        in_rate: 0,
        attach,
        color: None,
        matrix: Vec::new(),
        transform,
    }
}

fn skeleton(name: &str, animation: i64) -> Pad<Skeleton> {
    Pad::Filled(Skeleton {
        name: name.to_string(),
        bone: vec![formats::mbs::quad::Bone {
            attach: attach("animation", animation),
        }],
    })
}

fn quad(timeline: Vec<TimelineEntry>, keyframes: Vec<Pad<Keyframe>>) -> Quad {
    Quad {
        tag: serde_json::Value::Null,
        blend: Vec::new(),
        mix: [MixItem::Index(0), MixItem::Index(0), MixItem::Index(0)],
        keyframe: keyframes,
        hitbox: Vec::new(),
        slot: vec![Pad::Filled([attach("keyframe", 0), attach("hitbox", 0)])],
        animation: vec![Pad::Filled(Animation {
            name: "animation 0".to_string(),
            timeline,
            loop_id: 0,
        })],
        skeleton: vec![skeleton("windmill", 0)],
    }
}

fn frame(layers: Vec<Pad<KeyframeLayer>>) -> Pad<Keyframe> {
    Pad::Filled(Keyframe {
        debug: String::new(),
        name: "keyframe 0".to_string(),
        layer: layers,
        rect: Vec::new(),
    })
}

fn plain_dst() -> [f32; 8] {
    [-50.0, -100.0, 50.0, -100.0, 50.0, 0.0, -50.0, 0.0]
}

fn layer_at(scale_x: u16, scale_y: u16) -> field::Layer {
    field::Layer {
        scale_x,
        scale_y,
        ..Default::default()
    }
}

#[test]
fn a_model_layer_names_a_prop_and_the_prop_is_drawn_in_its_rest_pose() {
    let quad = quad(
        vec![entry(0, Some(attach("slot", 0)), rest())],
        vec![frame(vec![layer(plain_dst(), 2)])],
    );
    let props = field::props(&quad);
    assert_eq!(props.len(), 1, "one prop per skeleton");
    assert_eq!(
        props[0].len(),
        1,
        "the slot resolves to one textured keyframe layer"
    );
    assert_eq!(props[0][0].page, 2);
    assert_eq!(props[0][0].uv[2], (1.0, 1.0));
}

#[test]
fn a_prop_whose_bones_reach_no_keyframe_draws_nothing() {
    let mut quad = quad(
        vec![entry(0, None, rest())],
        vec![frame(vec![layer(plain_dst(), 0)])],
    );
    assert!(field::props(&quad)[0].is_empty());

    quad.slot.clear();
    let quad = Quad {
        animation: vec![Pad::Filled(Animation {
            name: "animation 0".to_string(),
            timeline: vec![entry(0, Some(attach("slot", 0)), rest())],
            loop_id: 0,
        })],
        ..quad
    };
    assert!(
        field::props(&quad)[0].is_empty(),
        "a slot that is not filled resolves to nothing"
    );
}

#[test]
fn a_prop_is_mirrored_into_the_upwards_world_and_scaled_by_the_layer() {
    let quad = quad(
        vec![entry(0, Some(attach("slot", 0)), rest())],
        vec![frame(vec![layer(plain_dst(), 0)])],
    );
    let props = field::props(&quad);

    let bounds = field::model_bounds(&layer_at(4096, 4096), &props[0]).expect("bounds");
    assert_eq!(
        bounds,
        [-50.0, 0.0, 50.0, 100.0],
        "model y runs down, world y runs up"
    );

    let doubled = field::model_bounds(&layer_at(8192, 4096), &props[0]).expect("bounds");
    assert_eq!(
        doubled,
        [-100.0, 0.0, 100.0, 100.0],
        "scale x only stretches x"
    );
}

#[test]
fn the_pose_transform_moves_the_prop_before_the_layer_places_it() {
    let transform = Transform {
        move_: [10.0, 20.0, 0.0],
        rotate: [0.0, 0.0, 0.0],
        scale: [2.0, 2.0],
    };
    let quad = quad(
        vec![entry(0, Some(attach("slot", 0)), transform)],
        vec![frame(vec![layer(plain_dst(), 0)])],
    );
    let props = field::props(&quad);

    let bounds = field::model_bounds(&layer_at(4096, 4096), &props[0]).expect("bounds");
    assert_eq!(
        bounds,
        [-90.0, -20.0, 110.0, 180.0],
        "s7 scales and moves the model in model space, so a +y move carries the prop down once mirrored"
    );
}

#[test]
fn a_layer_without_scale_or_panels_still_reports_a_pose() {
    let quad = quad(
        vec![entry(0, Some(attach("slot", 0)), rest())],
        vec![frame(vec![Pad::Empty(0)])],
    );
    assert!(
        field::props(&quad)[0].is_empty(),
        "a keyframe with no layer draws nothing"
    );
}

#[test]
fn the_model_unit_is_one_world_unit_per_model_unit() {
    // the authored layer box of every model layer is the prop's rest pose, so the
    // two units agree; half of it would put every prop inside half its box
    assert_eq!(field::MODEL_UNIT, 1.0);
}

fn keyed(
    time: i64,
    mix: i64,
    rate: i64,
    attach: Option<Attach>,
    transform: Transform,
) -> TimelineEntry {
    TimelineEntry {
        matrix_mix_id: mix,
        in_rate: rate,
        ..entry(time, attach, transform)
    }
}

fn moving(mix: i64, rate: i64) -> Quad {
    let far = Transform {
        move_: [10.0, 0.0, 0.0],
        rotate: [0.0; 3],
        scale: [1.0, 1.0],
    };
    quad(
        vec![
            keyed(0, mix, rate, Some(attach("slot", 0)), rest()),
            keyed(1, mix, rate, Some(attach("slot", 0)), far),
        ],
        vec![frame(vec![layer(plain_dst(), 0)])],
    )
}

#[test]
fn a_blended_key_reaches_the_next_key_one_frame_later() {
    let quad = moving(1, 0);
    let at =
        |clock: u64| field::model_bounds(&layer_at(4096, 4096), &field::props_at(&quad, clock)[0]);

    assert_eq!(
        at(0),
        Some([-50.0, 0.0, 50.0, 100.0]),
        "the first key holds"
    );
    assert_eq!(
        at(500),
        Some([-45.0, 0.0, 55.0, 100.0]),
        "one frame is 1000 units, so half of it is the midpoint"
    );
    assert_eq!(at(1000), Some([-40.0, 0.0, 60.0, 100.0]), "the second key");
    assert_eq!(
        at(60_000),
        Some([-40.0, 0.0, 60.0, 100.0]),
        "the last key holds forever"
    );
}

#[test]
fn a_stepped_key_holds_until_the_next_one() {
    let quad = moving(0, 0);
    let at =
        |clock: u64| field::model_bounds(&layer_at(4096, 4096), &field::props_at(&quad, clock)[0]);
    assert_eq!(at(500), Some([-50.0, 0.0, 50.0, 100.0]), "mix 0 holds");
    assert_eq!(at(1000), Some([-40.0, 0.0, 60.0, 100.0]), "and then snaps");
}

#[test]
fn a_rate_quantises_the_blend_into_steps() {
    let quad = moving(1, 4);
    let at =
        |clock: u64| field::model_bounds(&layer_at(4096, 4096), &field::props_at(&quad, clock)[0]);
    assert_eq!(
        at(200),
        Some([-50.0, 0.0, 50.0, 100.0]),
        "a fifth of the way in, the first of four steps still holds"
    );
    assert_eq!(
        at(750),
        Some([-42.5, 0.0, 57.5, 100.0]),
        "three quarters in, the third of four steps is exact"
    );
}
