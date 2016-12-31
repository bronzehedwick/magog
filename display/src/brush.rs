use std::rc::Rc;
use std::fmt;
use euclid::{Point2D, Rect, Size2D};
use vitral;
use calx_color::Rgba;
use calx_color::color::*;
use cache;
use atlas_cache::SubImageSpec;

pub type Color = [f32; 4];

/// Monochrome layer in a single frame.
#[derive(Clone, PartialEq)]
pub struct Splat {
    pub image: vitral::ImageData<usize>,
    pub offset: Point2D<f32>,
    pub color: Color,
    pub back_color: Color,
}

impl fmt::Debug for Splat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "Splat {{ {:?}+{:?} {:?} {:?} }}",
               self.image.tex_coords,
               self.offset,
               self.color,
               self.back_color)
    }
}

impl Splat {
    pub fn new(geom: &Geom, sheet: String, color: [f32; 4], back_color: [f32; 4]) -> Splat {
        Splat {
            image: cache::get(&SubImageSpec {
                sheet_name: sheet,
                bounds: geom.bounds,
            }),
            offset: geom.offset,
            color: color,
            back_color: back_color,
        }
    }
}

/// Stack of monochrome splat layers making up a drawable colored image.
pub type Frame = Vec<Splat>;

/// Collection of drawable frames.
pub type Brush = Vec<Frame>;

pub struct Builder {
    color: Rgba,
    back_color: Rgba,
    sheet_name: String,
    // The inner splat vectors are the *columns* of the splat matrix, and the merge operation will
    // build frames from the *rows* of the matrix.
    //
    // This sort of structure is used so that it'll be easy to specify a brush that has both a
    // complex structure (wallform or blobform) and has multiple splats for each frame.
    splat_matrix: Vec<Vec<Splat>>,
    pub brush: Brush,
}


impl Builder {
    pub fn new(sheet_name: &str) -> Builder {
        Builder {
            color: WHITE,
            back_color: BLACK,
            sheet_name: sheet_name.to_string(),
            splat_matrix: Vec::new(),
            brush: Vec::new(),
        }
    }

    pub fn color<C: Into<Rgba>>(mut self, color: C) -> Builder {
        self.color = color.into();
        self
    }

    pub fn colors<C: Into<Rgba>, D: Into<Rgba>>(mut self, color: C, back_color: D) -> Builder {
        self.color = color.into();
        self.back_color = back_color.into();
        self
    }

    fn into_splat(&self, geom: &Geom) -> Splat {
        Splat::new(geom,
                   self.sheet_name.clone(),
                   self.color.into_array(),
                   self.back_color.into_array())
    }

    pub fn splat<I: IntoIterator<Item = Geom>>(mut self, geom: I) -> Builder {
        let matrix_column = geom.into_iter().map(|g| self.into_splat(&g)).collect::<Vec<Splat>>();
        assert!(self.splat_matrix.is_empty() || matrix_column.len() == self.splat_matrix[0].len(),
                "Splat frame count does not match previous parallel splats");
        self.splat_matrix.push(matrix_column);
        self
    }

    pub fn tile(self, x: u32, y: u32) -> Builder { self.splat(Geom::tile(x, y)) }

    pub fn blob(
        self,
        vert_x: u32,
        vert_y: u32,
        rear_x: u32,
        rear_y: u32,
        x: u32,
        y: u32
    ) -> Builder {
        self.splat(Geom::blob(vert_x, vert_y, rear_x, rear_y, x, y))
    }

    pub fn wall(self, center_x: u32, center_y: u32, sides_x: u32, sides_y: u32) -> Builder {
        self.splat(Geom::wall(center_x, center_y, sides_x, sides_y))
    }

    pub fn merge(mut self) -> Builder {
        assert!(!self.splat_matrix.is_empty(),
                "Merging without any splats specified");
        let n = self.splat_matrix[0].len();

        for _ in 0..n {
            let mut frame = Vec::new();
            for i in self.splat_matrix.iter_mut() {
                frame.push(i.remove(0));
            }
            self.brush.push(frame);
        }

        self.splat_matrix.clear();
        self
    }

    pub fn finish(mut self) -> Rc<Brush> {
        if !self.splat_matrix.is_empty() {
            // A merge is pending, do it now.
            self = self.merge();
        }
        Rc::new(self.brush)
    }
}

pub struct Geom {
    offset: Point2D<f32>,
    bounds: Rect<u32>,
}

impl Geom {
    pub fn new(
        offset_x: i32,
        offset_y: i32,
        orig_x: u32,
        orig_y: u32,
        width: u32,
        height: u32
    ) -> Geom {
        Geom {
            offset: Point2D::new(offset_x as f32, offset_y as f32),
            bounds: Rect::new(Point2D::new(orig_x, orig_y), Size2D::new(width, height)),
        }
    }

    pub fn tile(x: u32, y: u32) -> Option<Geom> {
        // The BrushBuilder API expects all geom stuff to be IntoIter, so this returns Option for
        // one-shot iterable instead of a naked value.
        Some(Geom::new(16, 16, x, y, 32, 32))
    }

    /// Helper for blob chunks.
    ///
    /// Blobs are built from three 96x32 strips. First one contains the vertical edges, the second
    /// contains the rear blob and the third contains the blob front. The vertical and rear
    /// frames are nondescript and will probably be reused extensively.
    ///
    /// Blob shaping is somewhat complicated and requires a large number of frames.
    pub fn blob(vert_x: u32, vert_y: u32, rear_x: u32, rear_y: u32, x: u32, y: u32) -> Vec<Geom> {
        vec![
            Geom::new(16, 16, vert_x, vert_y, 16, 32),       // 0: Top left    VERTICAL SIDES
            Geom::new(0, 16, vert_x + 16, vert_y, 16, 32),   // 1: Top right
            Geom::new(16, 16, vert_x + 32, vert_y, 16, 32),  // 2: Middle left
            Geom::new(0, 16, vert_x + 48, vert_y, 16, 32),   // 3: Middle right
            Geom::new(16, 16, vert_x + 64, vert_y, 16, 32),  // 4: Bottom left
            Geom::new(0, 16, vert_x + 80, vert_y, 16, 32),   // 5: Bottom right

            Geom::new(16, 16, rear_x, rear_y, 10, 32),       // 6: Left half       REAR PARTS

            Geom::new(6, 16, rear_x + 10, rear_y, 6, 32),    // 7: Front
            Geom::new(0, 16, rear_x + 16, rear_y, 6, 32),    // 8

            Geom::new(-6, 16, rear_x + 22, rear_y, 10, 32),  // 9: Right half

            Geom::new(16, 16, rear_x + 32, rear_y, 10, 32),  // 10: Y-axis slope
            Geom::new(6, 16, rear_x + 42, rear_y, 6, 32),    // 11
            Geom::new(0, 16, rear_x + 48, rear_y, 6, 32),    // 12
            Geom::new(-6, 16, rear_x + 54, rear_y, 10, 32),  // 13

            Geom::new(16, 16, rear_x + 64, rear_y, 10, 32),  // 14: X-axis slope
            Geom::new(6, 16, rear_x + 74, rear_y, 6, 32),    // 15
            Geom::new(0, 16, rear_x + 80, rear_y, 6, 32),    // 16
            Geom::new(-6, 16, rear_x + 86, rear_y, 10, 32),  // 17

            Geom::new(16, 16, x, y, 10, 32),                 // 18 Left half      FRONT PARTS

            Geom::new(6, 16, x + 10, y, 6, 32),              // 19: Front
            Geom::new(0, 16, x + 16, y, 6, 32),              // 20

            Geom::new(-6, 16, x + 22, y, 10, 32),            // 21: Right half

            Geom::new(16, 16, x + 32, y, 10, 32),            // 22: Y-axis slope
            Geom::new(6, 16, x + 42, y, 6, 32),              // 23
            Geom::new(0, 16, x + 48, y, 6, 32),              // 24
            Geom::new(-6, 16, x + 54, y, 10, 32),            // 25

            Geom::new(16, 16, x + 64, y, 10, 32),            // 26: X-axis slope
            Geom::new(6, 16, x + 74, y, 6, 32),              // 27
            Geom::new(0, 16, x + 80, y, 6, 32),              // 28
            Geom::new(-6, 16, x + 86, y, 10, 32),            // 29
        ]
    }

    /// Helper for wall tiles
    ///
    /// Wall tiles are chopped up from two 32x32 images. One contains the center pillar wallform
    /// and the other contains the two long sides wallform.
    pub fn wall(center_x: u32, center_y: u32, sides_x: u32, sides_y: u32) -> Vec<Geom> {
        vec![
            Geom::new(16, 16, center_x, center_y, 16, 32),       // 0
            Geom::new(0, 16, center_x + 16, center_y, 16, 32),   // 1
            Geom::new(16, 16, sides_x, sides_y, 16, 32),         // 2
            Geom::new(0, 16, sides_x + 16, sides_y, 16, 32),     // 3
        ]
    }
}
