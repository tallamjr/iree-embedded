//! Golden-vector tests: the Rust front end must reproduce, byte for byte, the
//! features the vendored C reference produces for the committed recording
//! (which the micro_speech model was validated against).

use kws_frontend::{FEATURE_BYTES, Frontend};

static YES_AUDIO: &[u8] = include_bytes!("../../../examples/microbit-v2-kws/models/yes_audio.bin");
static YES_FEATURES: &[u8] =
    include_bytes!("../../../examples/microbit-v2-kws/models/yes_features.bin");

fn audio() -> Vec<i16> {
    YES_AUDIO
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect()
}

#[test]
fn oneshot_matches_c_reference_byte_exactly() {
    let samples = audio();
    let mut fe = Frontend::new();
    fe.init();
    let mut features = [0u8; FEATURE_BYTES];
    let frames = fe.features_oneshot(&samples, &mut features);
    assert_eq!(frames, 49);
    assert_eq!(YES_FEATURES.len(), FEATURE_BYTES);
    for (i, (got, want)) in features.iter().zip(YES_FEATURES.iter()).enumerate() {
        assert_eq!(
            got,
            want,
            "feature byte {} (frame {}, channel {}) differs",
            i,
            i / 40,
            i % 40
        );
    }
}

#[test]
fn streaming_chunks_match_oneshot() {
    let samples = audio();
    let mut fe = Frontend::new();
    fe.init();

    let mut oneshot = [0u8; FEATURE_BYTES];
    fe.features_oneshot(&samples, &mut oneshot);

    fe.reset();
    for chunk in samples.chunks(4000) {
        fe.push(chunk, 0, 1);
    }
    let mut streamed = [0u8; FEATURE_BYTES];
    let frames = fe.window(&mut streamed);
    assert_eq!(frames, 49);
    assert_eq!(oneshot, streamed);
}
