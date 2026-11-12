use fieldset::{Changes, Fields};

#[derive(Fields)]
struct Point {
    x: i16,
    y: i16,
}

type P = Point;

#[derive(Fields)]
struct Shape {
    #[fields(nested)]
    point: P,
}

fn main() {
    let expected: <P as Fields>::Path = PointPath::X;
    let mut model = Shape {
        point: Point { x: 0, y: 0 },
    };
    let mut changes = Changes::<Shape>::new();
    changes.track(&mut model).point().x().set(5);
    let mut drain = changes.drain();
    assert_eq!(drain.next(), Some(ShapePath::Point(expected)));
    assert_eq!(drain.next(), None);
    assert_eq!(model.point.x, 5);
    assert_eq!(model.point.y, 0);
}
