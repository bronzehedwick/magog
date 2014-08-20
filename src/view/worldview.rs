use calx::color::consts::*;
use calx::color::{RGB};
use calx::engine::{Engine};
use calx::rectutil::RectUtil;
use calx::timing;
use cgmath::aabb::{Aabb, Aabb2};
use cgmath::point::{Point, Point2};
use cgmath::vector::{Vector2};
use time;
use view::tilecache;
use view::tilecache::tile::*;
use view::drawable::{Drawable, Translated};
use world::area::Area;
use world::fov::{Fov, FovStatus, Seen, Remembered};
use world::mobs::{Mobs, Mob, MobType};
use world::mobs;
use world::terrain::TerrainType;
use world::terrain;
use world::spatial::{Location, ChartPos};
use world::system::{World, Entity};

pub static FLOOR_Z: f32 = 0.500f32;
pub static BLOCK_Z: f32 = 0.400f32;
pub static FX_Z: f32 = 0.375f32;
pub static FOG_Z: f32 = 0.350f32;
pub static CAPTION_Z: f32 = 0.300f32;

/// 3x3 grid of terrain cells. Use this as the input for terrain tile
/// computation, which will need to consider the immediate vicinity of cells.
pub struct Kernel<C> {
    n: C,
    ne: C,
    e: C,
    nw: C,
    center: C,
    se: C,
    w: C,
    sw: C,
    s: C,
}

pub struct CellDrawable {
    loc: Location,
    kernel: Kernel<TerrainType>,
    fov: Option<FovStatus>,
    world: World,
}

impl Drawable for CellDrawable {
    fn draw(&self, ctx: &mut Engine, offset: &Vector2<f32>) {

        fn classify(c: &CellDrawable) -> (bool, bool) {
            let mut front_of_wall = false;
            let mut is_door = false;
            let nw = c.loc + Vector2::new(-1, 0);
            let ne = c.loc + Vector2::new(0, -1);

            for &p in vec![nw, ne].iter() {
                let t = c.world.terrain_at(p);
                if t.is_wall() {
                    front_of_wall = true;
                    if t.is_walkable() { is_door = true; }
                }
            }
            (front_of_wall, is_door)
        }
    }
}

impl<C: Clone> Kernel<C> {
    pub fn new(get: |Location| -> C, loc: Location) -> Kernel<C> {
        Kernel {
            n: get(loc + Vector2::new(-1, -1)),
            ne: get(loc + Vector2::new(0, -1)),
            e: get(loc + Vector2::new(1, -1)),
            nw: get(loc + Vector2::new(-1, 0)),
            center: get(loc),
            se: get(loc + Vector2::new(1, 0)),
            w: get(loc + Vector2::new(-1, 1)),
            sw: get(loc + Vector2::new(0, 1)),
            s: get(loc + Vector2::new(1, 1)),
        }
    }

    pub fn new_default(center: C, edge: C) -> Kernel<C> {
        Kernel {
            n: edge.clone(),
            ne: edge.clone(),
            e: edge.clone(),
            nw: edge.clone(),
            center: center,
            se: edge.clone(),
            w: edge.clone(),
            sw: edge.clone(),
            s: edge.clone(),
        }
    }
}

pub trait WorldView {
    fn draw_entities_at<C: DrawContext>(
        &self, ctx: &mut C, loc: Location, pos: &Point2<f32>);

    fn draw_area(
        &self, ctx: &mut Engine, center: Location, fov: &Fov);
}

impl WorldView for World {
    fn draw_entities_at<C: DrawContext>(
        &self, ctx: &mut C, loc: Location, pos: &Point2<f32>) {
        let kernel = Kernel::new(|loc| self.terrain_at(loc), loc);
        terrain_sprites(ctx, &kernel, pos);

        if ctx.get_mode() != FogOfWar {
            for mob in self.mobs_at(loc).iter() {
                draw_mob(ctx, mob, pos);
            }
        }
    }

    fn draw_area(
        &self, ctx: &mut Engine, center: Location, fov: &Fov) {
        let mut chart_bounds = Aabb2::new(
            to_chart(&Point2::new(0f32, 0f32)).to_point(),
            to_chart(&Point2::new(640f32, 392f32)).to_point());
        chart_bounds = chart_bounds.grow(&to_chart(&Point2::new(640f32, 0f32)).to_point());
        chart_bounds = chart_bounds.grow(&to_chart(&Point2::new(0f32, 392f32)).to_point());

        for pt in chart_bounds.points() {
            let p = ChartPos::new(pt.x, pt.y);
            let offset = to_screen(p);
            let loc = Location::new(center.x + p.x as i8, center.y + p.y as i8);

            let mut draw = SpriteCollector::new(ctx);

            match fov.get(loc) {
                Some(Seen) => {
                    self.draw_entities_at(&mut draw, loc, &offset);
                }
                Some(Remembered) => {
                    draw.mode = FogOfWar;
                    self.draw_entities_at(&mut draw, loc, &offset);
                }
                None => {
                    let (front_of_wall, is_door) = classify(self, p, fov);
                    if front_of_wall && !is_door {
                        draw.draw(CUBE, &offset, BLOCK_Z, &BLACK);
                    } else if !front_of_wall {
                        draw.draw(BLOCK_DARK, &offset, BLOCK_Z, &BLACK);
                    }
                }
            }
        }

        fn classify(world: &World, pt: ChartPos, fov: &Fov) -> (bool, bool) {
            let mut front_of_wall = false;
            let mut is_door = false;
            let nw = ChartPos::new(pt.x - 1, pt.y);
            let ne = ChartPos::new(pt.x, pt.y - 1);

            /*
            // Uses obsolete FOV, will be replaced by terrain-drawables soon anyway.
            for &p in vec![nw, ne].iter() {
                match fov.get(p).loc().map(|loc| world.terrain_at(loc)) {
                    Some(t) => {
                        if t.is_wall() {
                            front_of_wall = true;
                            if t.is_walkable() { is_door = true; }
                        }
                    }
                    _ => ()
                }
            }
            */
            return (front_of_wall, is_door);
        }
    }
}


/// Interface for sprite-drawing.
pub trait DrawContext {
    fn draw(&mut self, idx: uint, pos: &Point2<f32>, z: f32, color: &RGB);

    fn get_mode(&self) -> ViewMode;
}

pub struct SpriteCollector<'a> {
    pub mode: ViewMode,
    engine: &'a mut Engine,
}

#[deriving(Eq, PartialEq)]
pub enum ViewMode {
    Normal,
    FogOfWar,
}

impl<'a> SpriteCollector<'a> {
    pub fn new<'a>(engine: &'a mut Engine) -> SpriteCollector<'a> {
        SpriteCollector {
            mode: Normal,
            engine: engine,
        }
    }
}

impl<'a> DrawContext for SpriteCollector<'a> {
    fn draw(
        &mut self, idx: uint, pos: &Point2<f32>, z: f32, color: &RGB) {
        let color = match self.mode {
            Normal => *color,
            FogOfWar => RGB::new(0x22u8, 0x22u8, 0x11u8),
        };

        self.engine.set_layer(z);
        self.engine.set_color(&color);
        self.engine.draw_image(&tilecache::get(idx), pos);
    }

    fn get_mode(&self) -> ViewMode { self.mode }
}


fn terrain_sprites<C: DrawContext>(
    ctx: &mut C, k: &Kernel<TerrainType>, pos: &Point2<f32>) {
    match k.center {
        terrain::Void => {
            ctx.draw(BLANK_FLOOR, pos, FLOOR_Z, &BLACK);
        },
        terrain::Water => {
            ctx.draw(WATER, pos, FLOOR_Z, &ROYALBLUE);
        },
        terrain::Shallows => {
            ctx.draw(SHALLOWS, pos, FLOOR_Z, &CORNFLOWERBLUE);
        },
        terrain::Magma => {
            ctx.draw(MAGMA, pos, FLOOR_Z, &DARKRED);
        },
        terrain::Tree => {
            // A two-toner, with floor, using two z-layers
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            ctx.draw(TREE_TRUNK, pos, BLOCK_Z, &SADDLEBROWN);
            ctx.draw(TREE_FOLIAGE, pos, BLOCK_Z, &GREEN);
        },
        terrain::Floor => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
        },
        terrain::Chasm => {
            ctx.draw(CHASM, pos, FLOOR_Z, &DARKSLATEGRAY);
        },
        terrain::Grass => {
            ctx.draw(GRASS, pos, FLOOR_Z, &DARKGREEN);
        },
        terrain::Downstairs => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            ctx.draw(DOWNSTAIRS, pos, BLOCK_Z, &SLATEGRAY);
        },
        terrain::Portal => {
            let glow = (127.0 *(1.0 + (time::precise_time_s()).sin())) as u8;
            let portal_col = RGB::new(glow, glow, 255);
            ctx.draw(PORTAL, pos, BLOCK_Z, &portal_col);
        },
        terrain::Rock => {
            blockform(ctx, k, pos, BLOCK, &DARKGOLDENROD);
        }
        terrain::Wall => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            wallform(ctx, k, pos, WALL, &LIGHTSLATEGRAY, true);
        },
        terrain::RockWall => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            wallform(ctx, k, pos, ROCKWALL, &LIGHTSLATEGRAY, true);
        },
        terrain::Fence => {
            // The floor type beneath the fence tile is visible, make it grass
            // if there's grass behind the fence. Otherwise make it regular
            // floor.
            if k.n == terrain::Grass || k.ne == terrain::Grass || k.nw == terrain::Grass {
                ctx.draw(GRASS, pos, FLOOR_Z, &DARKGREEN);
            } else {
                ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            }
            wallform(ctx, k, pos, FENCE, &DARKGOLDENROD, false);
        },
        terrain::Bars => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            wallform(ctx, k, pos, BARS, &GAINSBORO, false);
        },
        terrain::Stalagmite => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            ctx.draw(STALAGMITE, pos, BLOCK_Z, &DARKGOLDENROD);
        },
        terrain::Window => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            wallform(ctx, k, pos, WINDOW, &LIGHTSLATEGRAY, false);
        },
        terrain::Door => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            wallform(ctx, k, pos, DOOR, &LIGHTSLATEGRAY, true);
            wallform(ctx, k, pos, DOOR + 4, &SADDLEBROWN, false);
        },
        terrain::OpenDoor => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            wallform(ctx, k, pos, DOOR, &LIGHTSLATEGRAY, true);
        },
        terrain::Table => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            ctx.draw(TABLE, pos, BLOCK_Z, &DARKGOLDENROD);
        },
        terrain::Fountain => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            ctx.draw(FOUNTAIN, pos, BLOCK_Z, &GAINSBORO);
        },
        terrain::Altar => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            ctx.draw(ALTAR, pos, BLOCK_Z, &GAINSBORO);
        },
        terrain::Barrel => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            ctx.draw(BARREL, pos, BLOCK_Z, &DARKGOLDENROD);
        },
        terrain::Grave => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            ctx.draw(GRAVE, pos, BLOCK_Z, &SLATEGRAY);
        },
        terrain::Stone => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            ctx.draw(STONE, pos, BLOCK_Z, &SLATEGRAY);
        },
        terrain::Menhir => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            ctx.draw(MENHIR, pos, BLOCK_Z, &SLATEGRAY);
        },
        terrain::DeadTree => {
            ctx.draw(FLOOR, pos, FLOOR_Z, &SLATEGRAY);
            ctx.draw(TREE_TRUNK, pos, BLOCK_Z, &SADDLEBROWN);
        },
        terrain::TallGrass => {
            ctx.draw(TALLGRASS, pos, BLOCK_Z, &GOLD);
        },
    }

    fn blockform<C: DrawContext>(ctx: &mut C, k: &Kernel<TerrainType>, pos: &Point2<f32>, idx: uint, color: &RGB) {
        ctx.draw(idx, pos, BLOCK_Z, color);
        // Back lines for blocks with open floor behind them.
        if !k.nw.is_wall() {
            ctx.draw(BLOCK_NW, pos, BLOCK_Z, color);
        }
        if !k.n.is_wall() {
            ctx.draw(BLOCK_N, pos, BLOCK_Z, color);
        }
        if !k.ne.is_wall() {
            ctx.draw(BLOCK_NE, pos, BLOCK_Z, color);
        }
    }

    fn wallform<C: DrawContext>(ctx: &mut C, k: &Kernel<TerrainType>, pos: &Point2<f32>, idx: uint, color: &RGB, opaque: bool) {
        let (left_wall, right_wall, block) = wall_flags_lrb(k);
        if block {
            if opaque {
                ctx.draw(CUBE, pos, BLOCK_Z, color);
            } else {
                ctx.draw(idx + 2, pos, BLOCK_Z, color);
                return;
            }
        }
        if left_wall && right_wall {
            ctx.draw(idx + 2, pos, BLOCK_Z, color);
        } else if left_wall {
            ctx.draw(idx, pos, BLOCK_Z, color);
        } else if right_wall {
            ctx.draw(idx + 1, pos, BLOCK_Z, color);
        } else if !block || !k.s.is_wall() {
            // NB: This branch has some actual local kernel logic not
            // handled by wall_flags_lrb.
            ctx.draw(idx + 3, pos, BLOCK_Z, color);
        }
    }

    // Return code:
    // (there is a wall piece to the left front of the tile,
    //  there is a wall piece to the right front of the tile,
    //  there is a solid block in the tile)
    fn wall_flags_lrb(k: &Kernel<TerrainType>) -> (bool, bool, bool) {
        if k.nw.is_wall() && k.n.is_wall() && k.ne.is_wall() {
            // If there is open space to east or west, even if this block
            // has adjacent walls to the southeast or the southwest, those
            // will be using thin wall sprites, so this block needs to have
            // the corresponding wall bit to make the wall line not have
            // gaps.
            (!k.w.is_wall() || !k.sw.is_wall(), !k.e.is_wall() || !k.se.is_wall(), true)
        } else {
            (k.nw.is_wall(), k.ne.is_wall(), false)
        }
    }
}

fn draw_mob<C: DrawContext>(
    ctx: &mut C, mob: &Entity, pos: &Point2<f32>) {
    let body_pos =
    if is_bobbing(mob) {
        pos.add_v(timing::cycle_anim(
            0.3f64,
            &[Vector2::new(0.0f32, 0.0f32), Vector2::new(0.0f32, -1.0f32)]))
    } else { *pos };

    let (icon, color) = visual(mob.mob_type());
    match mob.mob_type() {
        mobs::Serpent => {
            // Body
            ctx.draw(94, &body_pos, BLOCK_Z, &color);
            // Ground mound
            ctx.draw(95, pos, BLOCK_Z, &color);
        }
        _ => {
            ctx.draw(icon, &body_pos, BLOCK_Z, &color);
        }
    }

    fn visual(t: MobType) -> (uint, RGB) {
        match t {
            mobs::Player => (51, AZURE),
            mobs::Dreg => (72, OLIVE),
            mobs::GridBug => (76, MAGENTA),
            mobs::Serpent => (94, CORAL),
        }
    }

    fn is_bobbing(mob: &Entity) -> bool {
        // TODO: Sleeping mobs don't bob.
        mob.mob_type() != mobs::Player
    }
}

pub fn draw_mouse(ctx: &mut Engine) -> ChartPos {
    let mouse = ctx.get_mouse();
    let cursor_chart_pos = to_chart(&mouse.pos);

    ctx.set_color(&FIREBRICK);
    ctx.set_layer(FLOOR_Z);
    ctx.draw_image(&tilecache::get(CURSOR_BOTTOM), &to_screen(cursor_chart_pos));
    ctx.set_layer(BLOCK_Z);
    ctx.draw_image(&tilecache::get(CURSOR_TOP), &to_screen(cursor_chart_pos));

    cursor_chart_pos
}

static CENTER_X: f32 = 320.0;
static CENTER_Y: f32 = 180.0;

fn to_screen(pos: ChartPos) -> Point2<f32> {
    let x = (pos.x) as f32;
    let y = (pos.y) as f32;
    Point2::new(CENTER_X + 16.0 * x - 16.0 * y, CENTER_Y + 8.0 * x + 8.0 * y)
}

fn to_chart(pos: &Point2<f32>) -> ChartPos {
    let column = ((pos.x + 8.0 - CENTER_X) / 16.0).floor();
    let row = ((pos.y - CENTER_Y as f32 - column * 8.0) / 16.0).floor();
    ChartPos::new((column + row) as int, row as int)
}
