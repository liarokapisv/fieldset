#[path = "support/replica.rs"]
mod replica;

use core::mem::size_of;

use fieldset::{Changes, Fields, Index, Path};

use replica::{Master, Part, Preset, PresetPath};

const COUNT: usize = 401;

static STATIC_CHANGES: Changes<Preset> = Changes::new();

fn leaf(count: usize) -> u16 {
    u16::try_from(count).unwrap_or(u16::MAX)
}

#[test]
fn counts() {
    assert_eq!(<Master as Fields>::COUNT, 11);
    assert_eq!(<Part as Fields>::COUNT, 65);
    assert_eq!(<Preset as Fields>::COUNT, COUNT);
}

#[test]
fn sizes() {
    assert_eq!(size_of::<Changes<Preset>>(), 52);
    assert!(size_of::<PresetPath>() <= 4);
    assert_eq!(size_of::<Option<PresetPath>>(), size_of::<PresetPath>());
    assert_eq!(size_of::<Preset>(), COUNT * 4);
}

#[test]
fn round_trip() {
    for l in 0..leaf(COUNT) {
        let path = <PresetPath as Path>::from_leaf(l);
        assert_eq!(path.map(Path::leaf), Some(l), "leaf {l}");
    }
    assert_eq!(<PresetPath as Path>::from_leaf(leaf(COUNT)), None);
    assert_eq!(<PresetPath as Path>::from_leaf(u16::MAX), None);
}

#[test]
fn drain_order() {
    let mut preset = replica::sample();
    let mut changes = Changes::<Preset>::new();
    changes.track(&mut preset).mark_all();
    assert_eq!(changes.len(), COUNT);
    let leaves: Vec<u16> = changes.drain().map(Path::leaf).collect();
    assert_eq!(leaves.len(), COUNT);
    assert_eq!(leaves, (0..leaf(COUNT)).collect::<Vec<_>>());
    assert!(changes.is_empty());
    assert_eq!(preset, replica::sample());
}

#[test]
fn sample_numbers_its_leaves() {
    let mut preset = replica::sample();
    let mut changes = Changes::<Preset>::new();
    let last = Index::new(5).unwrap_or_else(|| panic!("5 is in range for 6"));
    {
        let mut handle = changes.track(&mut preset);
        assert_eq!(handle.master().drive().get(), 10.0);
        assert_eq!(handle.parts(last).mixer().drive().get(), 400.0);
        handle.parts(last).mixer().drive().set(0.5);
    }
    assert_eq!(changes.drain().map(Path::leaf).collect::<Vec<_>>(), [400]);
    assert_eq!(preset.parts[5].mixer.drive, 0.5);
}

#[test]
fn static_changes() {
    assert!(STATIC_CHANGES.is_empty());
    assert_eq!(STATIC_CHANGES.len(), 0);
}
