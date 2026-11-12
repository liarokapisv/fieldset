use fieldset::{Changes, Fields};

mod shapes {
    pub mod geometry {
        use fieldset::Fields;

        #[derive(Fields)]
        pub struct Point {
            pub x: i16,
            pub y: i16,
        }
    }
}

#[derive(Fields)]
struct Shape {
    id: u8,
    #[fields(nested)]
    point: shapes::geometry::Point,
}

fn main() {
    let mut model = Shape {
        id: 0,
        point: shapes::geometry::Point { x: 0, y: 0 },
    };
    let mut changes = Changes::<Shape>::new();
    changes.track(&mut model).point().y().set(-3);
    let mut drain = changes.drain();
    let path = drain.next();
    assert_eq!(path, Some(ShapePath::Point(shapes::geometry::PointPath::Y)));
    assert_eq!(drain.next(), None);
    assert_eq!(
        path.and_then(ShapePath::point),
        Some(shapes::geometry::PointPath::Y)
    );
    assert_eq!(ShapePath::Id.point(), None);
    assert_eq!(model.id, 0);
    assert_eq!(model.point.y, -3);
}
