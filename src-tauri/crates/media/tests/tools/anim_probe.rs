fn main() {
    for path in std::env::args().skip(1) {
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let v55 = match formats::mbs::parse_bytes(&bytes, "swi_unic") {
            Ok(v55) => v55,
            Err(_) => continue,
        };
        let quad = match formats::mbs::convert(&v55) {
            Ok(quad) => quad,
            Err(_) => continue,
        };
        let mut moving = 0usize;
        let mut total = 0usize;
        let mut longest = 0i64;
        let mut mix = std::collections::HashMap::new();
        for cell in &quad.animation {
            let formats::mbs::quad::Pad::Filled(animation) = cell else { continue };
            total += 1;
            if let Some(last) = animation.timeline.last() {
                longest = longest.max(last.time);
            }
            for entry in &animation.timeline {
                *mix.entry(entry.matrix_mix_id).or_insert(0usize) += 1;
            }
            let rest = formats::field::props_at(&quad, 0);
            let later = formats::field::props_at(&quad, 1500);
            let _ = (&rest, &later);
        }
        // per-skeleton motion: compare the pose of every prop at two ticks
        let a = formats::field::props_at(&quad, 0);
        let b = formats::field::props_at(&quad, 1500);
        for (left, right) in a.iter().zip(b.iter()) {
            if left.len() != right.len() {
                moving += 1;
            } else if left
                .iter()
                .zip(right.iter())
                .any(|(l, r)| (l.dst[0].0 - r.dst[0].0).abs() > 0.01 || (l.dst[0].1 - r.dst[0].1).abs() > 0.01)
            {
                moving += 1;
            }
        }
        println!(
            "{}: {} animations, {} of {} props move by tick 1500, last key {longest}, mix {:?}",
            path.rsplit(['/', '\\']).next().unwrap(),
            total,
            moving,
            a.len(),
            mix
        );
    }
}
