#[path = "../tests/support/replica.rs"]
mod replica;

use fieldset::{Changes, Fields, Index};

use replica::{
    AmpEnvelopePath, FilterEnvelopePath, FilterPath, LfoAPath, LfoBPath, MasterPath, MixerPath,
    OscAPath, OscBPath, PartPath, Preset, PresetMut, PresetPath,
};

// A write to one leaf, resolved from its path at setup; `Index` is the part
// for part leaves and ignored for master leaves.
type Write = fn(&mut PresetMut<'_>, Index<6>, f32);

const COUNT: usize = <Preset as Fields>::COUNT;

// Reference figures for an ideal reusable slot array on the reference machine.
const BASELINES: [(usize, u32); 4] = [(1, 9), (16, 50), (64, 191), (400, 1359)];

fn main() {
    for (k, nanos) in BASELINES {
        println!("reference slot array, k = {k}: {nanos} ns");
    }
    divan::main();
}

// `k` distinct leaves spread across the model: every `COUNT / k`-th one.
fn leaves(k: usize) -> Vec<(Write, Index<6>)> {
    assert!(k <= COUNT, "at most {COUNT} distinct leaves");
    if k == 0 {
        return Vec::new();
    }
    let step = COUNT / k;
    let paths: Vec<(Write, Index<6>)> = (0..k)
        .filter_map(|j| u16::try_from(j * step).ok())
        .filter_map(PresetPath::from_leaf)
        .map(resolve)
        .collect();
    assert_eq!(paths.len(), k, "every chosen leaf index is in range");
    paths
}

fn resolve(path: PresetPath) -> (Write, Index<6>) {
    match path {
        PresetPath::Master(path) => (resolve_master(path), Index::new(0).expect("six parts")),
        PresetPath::Parts(index, path) => (resolve_part(path), index),
    }
}

// One write table per replica struct: each row maps a leaf variant to the
// handle method that writes it.
macro_rules! writes {
    (@master $name:ident, $path:ident, $sub:ident; $($variant:ident => $field:ident),* $(,)?) => {
        fn $name(path: $path) -> Write {
            match path {
                $($path::$variant => |h, _, v| h.$sub().$field().set(v),)*
            }
        }
    };
    (@part $name:ident, $path:ident, $sub:ident; $($variant:ident => $field:ident),* $(,)?) => {
        fn $name(path: $path) -> Write {
            match path {
                $($path::$variant => |h, i, v| h.parts(i).$sub().$field().set(v),)*
            }
        }
    };
}

writes!(@master resolve_master, MasterPath, master; Volume => volume, Pan => pan, Tune => tune, Transpose => transpose, Glide => glide, BendRange => bend_range, Tempo => tempo, Swing => swing, ReverbSend => reverb_send, DelaySend => delay_send, Drive => drive);
writes!(@part resolve_osc_a, OscAPath, osc_a; Wave => wave, Octave => octave, Semitone => semitone, Detune => detune, PulseWidth => pulse_width, PwmDepth => pwm_depth, Sync => sync, FmDepth => fm_depth, Level => level, Pan => pan);
writes!(@part resolve_osc_b, OscBPath, osc_b; Wave => wave, Octave => octave, Semitone => semitone, Detune => detune, PulseWidth => pulse_width, PwmDepth => pwm_depth, Ring => ring, Noise => noise, Level => level, Pan => pan);
writes!(@part resolve_filter, FilterPath, filter; Mode => mode, Cutoff => cutoff, Resonance => resonance, Drive => drive, KeyTrack => key_track, EnvDepth => env_depth, LfoDepth => lfo_depth, Velocity => velocity, Slope => slope, Spread => spread, Feedback => feedback, Mix => mix);
writes!(@part resolve_filter_env, FilterEnvelopePath, filter_env; Delay => delay, Attack => attack, Hold => hold, Decay => decay, Sustain => sustain, Release => release, Curve => curve, Velocity => velocity);
writes!(@part resolve_amp_env, AmpEnvelopePath, amp_env; Delay => delay, Attack => attack, Hold => hold, Decay => decay, Sustain => sustain, Release => release, Curve => curve, Velocity => velocity);
writes!(@part resolve_lfo_a, LfoAPath, lfo_a; Shape => shape, Rate => rate, Depth => depth, Delay => delay, Fade => fade, Phase => phase);
writes!(@part resolve_lfo_b, LfoBPath, lfo_b; Shape => shape, Rate => rate, Depth => depth, Delay => delay, Fade => fade, Phase => phase);
writes!(@part resolve_mixer, MixerPath, mixer; Level => level, Pan => pan, ReverbSend => reverb_send, DelaySend => delay_send, Drive => drive);

fn resolve_part(path: PartPath) -> Write {
    match path {
        PartPath::OscA(path) => resolve_osc_a(path),
        PartPath::OscB(path) => resolve_osc_b(path),
        PartPath::Filter(path) => resolve_filter(path),
        PartPath::FilterEnv(path) => resolve_filter_env(path),
        PartPath::AmpEnv(path) => resolve_amp_env(path),
        PartPath::LfoA(path) => resolve_lfo_a(path),
        PartPath::LfoB(path) => resolve_lfo_b(path),
        PartPath::Mixer(path) => resolve_mixer(path),
    }
}

// One measured round: write every chosen leaf through its handle, then
// drain to completion. Returns how many paths the drain produced.
fn round(
    changes: &mut Changes<Preset>,
    preset: &mut Preset,
    writes: &[(Write, Index<6>)],
    value: f32,
) -> usize {
    {
        let mut handle = changes.track(preset);
        for &(write, index) in writes {
            write(&mut handle, index, value);
        }
    }
    changes.drain().map(divan::black_box).count()
}

#[divan::bench(args = [0, 1, 16, 64, 400])]
fn set_then_drain(bencher: divan::Bencher, k: usize) {
    let writes = leaves(k);
    let mut preset = replica::sample();
    let mut changes = Changes::<Preset>::new();
    assert_eq!(round(&mut changes, &mut preset, &writes, 0.0), k);
    assert!(changes.is_empty());

    let mut value = 0.0_f32;
    bencher.bench_local(|| {
        value += 1.0;
        round(&mut changes, &mut preset, &writes, value)
    });
}
