use fieldset::Fields;

#[derive(Fields, Clone, Copy, Debug, PartialEq)]
pub struct Master {
    pub volume: f32,
    pub pan: f32,
    pub tune: f32,
    pub transpose: f32,
    pub glide: f32,
    pub bend_range: f32,
    pub tempo: f32,
    pub swing: f32,
    pub reverb_send: f32,
    pub delay_send: f32,
    pub drive: f32,
}

#[derive(Fields, Clone, Copy, Debug, PartialEq)]
pub struct OscA {
    pub wave: f32,
    pub octave: f32,
    pub semitone: f32,
    pub detune: f32,
    pub pulse_width: f32,
    pub pwm_depth: f32,
    pub sync: f32,
    pub fm_depth: f32,
    pub level: f32,
    pub pan: f32,
}

#[derive(Fields, Clone, Copy, Debug, PartialEq)]
pub struct OscB {
    pub wave: f32,
    pub octave: f32,
    pub semitone: f32,
    pub detune: f32,
    pub pulse_width: f32,
    pub pwm_depth: f32,
    pub ring: f32,
    pub noise: f32,
    pub level: f32,
    pub pan: f32,
}

#[derive(Fields, Clone, Copy, Debug, PartialEq)]
pub struct Filter {
    pub mode: f32,
    pub cutoff: f32,
    pub resonance: f32,
    pub drive: f32,
    pub key_track: f32,
    pub env_depth: f32,
    pub lfo_depth: f32,
    pub velocity: f32,
    pub slope: f32,
    pub spread: f32,
    pub feedback: f32,
    pub mix: f32,
}

#[derive(Fields, Clone, Copy, Debug, PartialEq)]
pub struct FilterEnvelope {
    pub delay: f32,
    pub attack: f32,
    pub hold: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub curve: f32,
    pub velocity: f32,
}

#[derive(Fields, Clone, Copy, Debug, PartialEq)]
pub struct AmpEnvelope {
    pub delay: f32,
    pub attack: f32,
    pub hold: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub curve: f32,
    pub velocity: f32,
}

#[derive(Fields, Clone, Copy, Debug, PartialEq)]
pub struct LfoA {
    pub shape: f32,
    pub rate: f32,
    pub depth: f32,
    pub delay: f32,
    pub fade: f32,
    pub phase: f32,
}

#[derive(Fields, Clone, Copy, Debug, PartialEq)]
pub struct LfoB {
    pub shape: f32,
    pub rate: f32,
    pub depth: f32,
    pub delay: f32,
    pub fade: f32,
    pub phase: f32,
}

#[derive(Fields, Clone, Copy, Debug, PartialEq)]
pub struct Mixer {
    pub level: f32,
    pub pan: f32,
    pub reverb_send: f32,
    pub delay_send: f32,
    pub drive: f32,
}

#[derive(Fields, Clone, Copy, Debug, PartialEq)]
pub struct Part {
    #[fields(nested)]
    pub osc_a: OscA,
    #[fields(nested)]
    pub osc_b: OscB,
    #[fields(nested)]
    pub filter: Filter,
    #[fields(nested)]
    pub filter_env: FilterEnvelope,
    #[fields(nested)]
    pub amp_env: AmpEnvelope,
    #[fields(nested)]
    pub lfo_a: LfoA,
    #[fields(nested)]
    pub lfo_b: LfoB,
    #[fields(nested)]
    pub mixer: Mixer,
}

#[derive(Fields, Clone, Copy, Debug, PartialEq)]
pub struct Preset {
    #[fields(nested)]
    pub master: Master,
    #[fields(indexed, nested)]
    pub parts: [Part; 6],
}

// A preset whose every leaf holds its own leaf index as an `f32`.
pub fn sample() -> Preset {
    let mut next = -1.0_f32;
    let mut leaf = || {
        next += 1.0;
        next
    };
    let master = Master {
        volume: leaf(),
        pan: leaf(),
        tune: leaf(),
        transpose: leaf(),
        glide: leaf(),
        bend_range: leaf(),
        tempo: leaf(),
        swing: leaf(),
        reverb_send: leaf(),
        delay_send: leaf(),
        drive: leaf(),
    };
    let parts = core::array::from_fn(|_| Part {
        osc_a: OscA {
            wave: leaf(),
            octave: leaf(),
            semitone: leaf(),
            detune: leaf(),
            pulse_width: leaf(),
            pwm_depth: leaf(),
            sync: leaf(),
            fm_depth: leaf(),
            level: leaf(),
            pan: leaf(),
        },
        osc_b: OscB {
            wave: leaf(),
            octave: leaf(),
            semitone: leaf(),
            detune: leaf(),
            pulse_width: leaf(),
            pwm_depth: leaf(),
            ring: leaf(),
            noise: leaf(),
            level: leaf(),
            pan: leaf(),
        },
        filter: Filter {
            mode: leaf(),
            cutoff: leaf(),
            resonance: leaf(),
            drive: leaf(),
            key_track: leaf(),
            env_depth: leaf(),
            lfo_depth: leaf(),
            velocity: leaf(),
            slope: leaf(),
            spread: leaf(),
            feedback: leaf(),
            mix: leaf(),
        },
        filter_env: FilterEnvelope {
            delay: leaf(),
            attack: leaf(),
            hold: leaf(),
            decay: leaf(),
            sustain: leaf(),
            release: leaf(),
            curve: leaf(),
            velocity: leaf(),
        },
        amp_env: AmpEnvelope {
            delay: leaf(),
            attack: leaf(),
            hold: leaf(),
            decay: leaf(),
            sustain: leaf(),
            release: leaf(),
            curve: leaf(),
            velocity: leaf(),
        },
        lfo_a: LfoA {
            shape: leaf(),
            rate: leaf(),
            depth: leaf(),
            delay: leaf(),
            fade: leaf(),
            phase: leaf(),
        },
        lfo_b: LfoB {
            shape: leaf(),
            rate: leaf(),
            depth: leaf(),
            delay: leaf(),
            fade: leaf(),
            phase: leaf(),
        },
        mixer: Mixer {
            level: leaf(),
            pan: leaf(),
            reverb_send: leaf(),
            delay_send: leaf(),
            drive: leaf(),
        },
    });
    Preset { master, parts }
}
