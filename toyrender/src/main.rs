#[macro_use]
extern crate log;
extern crate env_logger;
extern crate sdl2;
extern crate num;

extern crate toyrender;

use sdl2::rect::{ Rect };
use sdl2::pixels::{ PixelFormatEnum };
use sdl2::keyboard::Keycode;
use sdl2::render::Renderer;

use toyrender::vector3d::{ Vec3f, Vec3i };
use toyrender::linerasterizer::LineRasterizer;
use toyrender::pixmap::Pixmap;
use toyrender::model::{ Loader };

struct SdlCanvas
{
    renderer: Renderer<'static>,
    
    buffer: Pixmap,
    z_buffer: Pixmap,
    
    width: usize,
    height: usize
}

impl SdlCanvas {
    pub fn new(renderer: Renderer<'static>, w: usize, h: usize) -> SdlCanvas {
        SdlCanvas { 
            renderer: renderer, 
            z_buffer: Pixmap::new(w + 1, h + 1, std::i32::MIN),
            buffer: Pixmap::new(w + 1, h + 1, 0),
            width: w, height: h
        }
    }
    
    pub fn present(&mut self)
    {
        let mut texture = self.renderer.create_texture_streaming(PixelFormatEnum::RGB24, 
                                       (self.width as u32, self.height as u32)).unwrap();
        texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
            for x in 0..self.width {
                for y in 0..self.height {
                    let color = self.buffer[x][y];
                    
                    let offset = y*pitch + x*3;
                    buffer[offset + 0] = (color >> (8*2)) as u8;
                    buffer[offset + 1] = (color >> (8*1)) as u8;
                    buffer[offset + 2] = color as u8;
                }
            }
        }).unwrap();

        self.renderer.clear();
        self.renderer.copy(&texture, None, Some(Rect::new_unwrap(0, 0, 
                                                self.width as u32, self.height as u32)));

        self.renderer.present();
        
        self.z_buffer.fill(std::i32::MIN);
        self.buffer.fill(0);
    }
    
    pub fn line(&mut self, a: Vec3i, b: Vec3i, color: u32)
    {
        self.set_pixel(a, color);
        for p in LineRasterizer::new(a, b) {
            self.set_pixel(p, color);
        }
    }
    
    pub fn triangle(&mut self, mut a: Vec3i, mut b: Vec3i, mut c: Vec3i, color: u32)
    {
        if b.y() > a.y() { std::mem::swap(&mut a, &mut b); }
        if c.y() > a.y() { std::mem::swap(&mut a, &mut c); }
        if c.y() > b.y() { std::mem::swap(&mut c, &mut b); }       

        let mut fill_fn = |raster1 : &mut LineRasterizer, raster2: &mut LineRasterizer| {
            let mut y = raster1.point().y();

            while raster1.next_point() {
                if y != raster1.point().y() {
                    y = raster1.point().y();

                    while raster2.point().y() != y {
                        raster2.next_point();
                    }          
              
                    self.line(raster1.point(), raster2.point(), color);
                }
            }
        };
        
        // Fill top triangle part
        let mut raster1 = LineRasterizer::new(a, b);
        let mut raster2 = LineRasterizer::new(a, c);
        fill_fn(&mut raster1, &mut raster2);
        
        // Fill bottom triangle part
        raster1 = LineRasterizer::new(b, c);
        fill_fn(&mut raster1, &mut raster2);
    }

    pub fn textured_triangle(&mut self, mut a: Vec3i, mut b: Vec3i, mut c: Vec3i,
                             mut uv0: Vec3i, mut uv1: Vec3i, mut uv2: Vec3i,
                             intensity: f32, diffuse: &Pixmap)
    {
        let get_gray = |color: u32, intensity: f32| -> u32 {
            let mut result = ((color as u8) as f32*intensity) as u32;
            result += (((color >> 8) as u8) as f32*intensity) as u32*256;
            result += (((color >> 16) as u8) as f32*intensity) as u32*256*256;
            return result;
        };

        if b.y() > a.y() {
            std::mem::swap(&mut a, &mut b);
            std::mem::swap(&mut uv0, &mut uv1);
        }
        if c.y() > a.y() {
            std::mem::swap(&mut a, &mut c);
            std::mem::swap(&mut uv0, &mut uv2);
        }
        if c.y() > b.y() {
            std::mem::swap(&mut c, &mut b);
            std::mem::swap(&mut uv2, &mut uv1);
        }

        let alpha_step = 1.0 / (c.y - a.y) as f32;
        let mut alpha: f32 = 0.0;
        let mut fill_fn = |raster1 : &mut LineRasterizer, raster2: &mut LineRasterizer| {
            let mut y = raster1.point().y();

            let seg_height = raster2.end_point().y() - raster1.end_point().y();
            let beta_step = 1.0 / seg_height as f32;
            let mut beta = 0.0;
            while raster1.next_point() {
                if y != raster1.point().y() {
                    y = raster1.point().y();

                    while raster2.point().y() != y {
                        raster2.next_point();
                    }

                    self.line(raster1.point(), raster2.point(), get_gray(0xffffff, intensity));

                    alpha += alpha_step;
                    beta += beta_step;
                }
            }
        };

        // Fill top triangle part
        let mut raster1 = LineRasterizer::new(a, b);
        let mut raster2 = LineRasterizer::new(a, c);
        fill_fn(&mut raster1, &mut raster2);

        // Fill bottom triangle part
        raster1 = LineRasterizer::new(b, c);
        fill_fn(&mut raster1, &mut raster2);
    }
    
    pub fn set_pixel(&mut self, v: Vec3i, color: u32) {
        let x = v.x() as usize; let y = v.y() as usize;
        
        if self.z_buffer[x][y] < v.z() {
            self.z_buffer[x][y] = v.z();
            self.buffer[x][y] = color as i32;
        }
    }
}

pub fn main() {
    env_logger::init().unwrap();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let w = 900;
    let h = 900;
    let d = 255;

    let window = video_subsystem.window("rust-sdl2 demo: Video", w, h)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let renderer = window.renderer().build().unwrap();
    
    let mut canvas = SdlCanvas::new(renderer, w as usize, h as usize);
    
    let light_dir = Vec3f::new(0.0, 0.0, -1.0);
    let model = Loader::from_files("obj/african/african_head.obj",
                                   "obj/african/african_head_diffuse.tga").unwrap();

    for i in 0..model.faces.len() {
        let face = model.faces[i];

        let mut screen_coords = [Vec3f::new(0.0, 0.0, 0.0); 3];
        let mut world_coords = [Vec3f::new(0.0,0.0,0.0); 3];
    
        for i in 0..3 {
            let world = model.verticies[face[i][0] as usize];
            
            screen_coords[i] = Vec3f::new(
                ((world.x + 1.0) * w as f32 / 2.0), 
                h as f32 - ((world.y + 1.0) * h as f32 / 2.0), 
                world.z as f32 * d as f32
            );
            world_coords[i] = world;
        }
         
        let n: Vec3f = ((world_coords[2]-world_coords[0]) ^ (world_coords[1]-world_coords[0])).normalized();        
        let intensity = light_dir * n;
        
        if intensity > 0.0 {
            let l = (255.0 * intensity) as u32;
            let color = l | l << 8 | l << 16;

            //canvas.triangle(
            //    screen_coords[0].round(),
            //    screen_coords[1].round(),
            //    screen_coords[2].round(),
            //    color);

            canvas.textured_triangle(screen_coords[0].to::<i32>(),
                                     screen_coords[1].to::<i32>(),
                                     screen_coords[2].to::<i32>(),
                                     model.uv(i, 0),
                                     model.uv(i, 1),
                                     model.uv(i, 2),
                                     intensity,
                                     &model.diffuse
                                     );
        }
    }
    canvas.present();
    
    let mut running = true;
    let mut event_pump = sdl_context.event_pump().unwrap();

    while running {
        for event in event_pump.poll_iter() {
            use sdl2::event::Event;

            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    running = false
                },
                _ => {}
            }
        }
        // The rest of the game loop goes here...
    }
}
