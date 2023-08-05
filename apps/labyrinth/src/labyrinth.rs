use super::*;
use gam::*;
use gam::menu::*;
use gam::menu::api::DrawStyle;

use std::sync::{Arc, Mutex};

const BALL_RADIUS: i16 = 6;
const MOMENTUM_LIMIT: i32 = 8;
pub(crate) struct Labyrinth {
    gam: gam::Gam,
    gid: Gid,
    screensize: Point,
    // our security token for making changes to our record on the GAM
    _token: [u32; 4],
    ball: Circle,
    maze: Vec<Vec<Arc<Mutex<Tile>>>>,
    momentum: Point,
    trng: trng::Trng,
    modals: modals::Modals,
    com: com::Com,
}

impl Labyrinth {
    pub(crate) fn new(sid: xous::SID) -> Self {
	let xns = xous_names::XousNames::new().expect("couldn't connect to Xous Namespace Server");
	let gam = gam::Gam::new(&xns).expect("can't connect to Graphical Abstraction Manager");

	let token = gam.register_ux(UxRegistration {
            app_name: xous_ipc::String::<128>::from_str("labyrinth"),
            ux_type: gam::UxType::Framebuffer,
            predictor: None,
            listener: sid.to_array(), // note disclosure of our SID to the GAM -- the secret is now shared with the GAM!
            redraw_id: AppOp::Redraw.to_u32().unwrap(),
            gotinput_id: None,
            audioframe_id: None,
            focuschange_id: Some(AppOp::FocusChange.to_u32().unwrap()),
            rawkeys_id: Some(AppOp::Rawkeys.to_u32().unwrap()),
        }).expect("couldn't register Ux context for labyrinth");

        let gid = gam.request_content_canvas(token.unwrap()).expect("couldn't get content canvas");
        let screensize = gam.get_canvas_bounds(gid).expect("couldn't get dimensions of content canvas");

        let trng = trng::Trng::new(&xns).unwrap();
        let mut ball = Circle::new(Point::new(56, 8), BALL_RADIUS);
        ball.style = DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 1);
        gam.draw_circle(gid, ball).expect("couldn't erase ball's previous position");
        let modals = modals::Modals::new(&xns).unwrap();
        let com = com::Com::new(&xns).unwrap();
	let maze = create_maze::<21, 31>(&trng);
        Labyrinth {
            gid,
            gam,
            screensize,
            _token: token.unwrap(),
            ball,
	    maze,
            momentum: Point::new(0, 0),
            trng,
            modals,
            com,
        }    
    }
    pub(crate) fn update(&mut self) {
        // send a list of objects to draw to the GAM, to avoid race conditions in between operations
        let mut draw_list = GamObjectList::new(self.gid);

        // clear the previous location of the ball
        self.ball.style = DrawStyle::new(PixelColor::Light, PixelColor::Light, 1);
        draw_list.push(GamObjectType::Circ(self.ball)).unwrap();
	// for wall in &self.walls {
	//     draw_list.push(GamObjectType::Line(*wall)).unwrap();
	// }
        let (x, y, _z, _id) = self.com.gyro_read_blocking().unwrap();
        let ix = x as i16;
        let iy = y as i16;
        log::debug!("x: {}, y: {}", ix, iy);
        // negative x => tilt to right
        // positive x => tilt to left
        // negative y => tilt toward top
        // positive y => tilt toward bottom
        self.momentum = Point::new(
            -(ix / 200),
            iy / 200
        );
        
        // update the ball position based on the momentum vector
        self.ball.translate(self.momentum);

	// snap to edges
        if self.ball.center.x + BALL_RADIUS >= self.screensize.x {
            self.ball.center.x = self.screensize.x - BALL_RADIUS;
        }
        if self.ball.center.x - BALL_RADIUS <= 0 {
            self.ball.center.x = BALL_RADIUS;
        }
        if self.ball.center.y + BALL_RADIUS >= self.screensize.y {
            self.ball.center.y = self.screensize.y - BALL_RADIUS;
        }
        if self.ball.center.y - BALL_RADIUS <= 0 {
            self.ball.center.y = BALL_RADIUS;
        }
	
	let left = ((self.ball.center.x - BALL_RADIUS)/16) as usize;
	let right = ((self.ball.center.x + BALL_RADIUS)/16) as usize;
	let top = ((self.ball.center.y - BALL_RADIUS)/16) as usize;
	let bottom = ((self.ball.center.y + BALL_RADIUS)/16) as usize;
        // check if the ball hits a wall, if so, snap its position to the wall
	if left != right && (self.maze[left][top].lock().unwrap().border_right || (self.maze[left][bottom].lock().unwrap().border_right && top != bottom)) {
	    self.ball.center.x = (left as i16)*16 + 9;
	}
	if top != bottom && (self.maze[left][top].lock().unwrap().border_bottom || (self.maze[right][top].lock().unwrap().border_bottom && left != right)) {
	    self.ball.center.y = (top as i16)*16 + 9;
	}

        // draw the new location for the ball
        self.ball.style = DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 1);
        draw_list.push(GamObjectType::Circ(self.ball)).unwrap();
        self.gam.draw_list(draw_list).expect("couldn't execute draw list");
        log::trace!("ball app redraw##");
        self.gam.redraw().unwrap();
    }
    pub(crate) fn focus(&mut self) {
	// draw maze
	let walls = create_walls::<16>(&self.maze);
	for wall in walls {
	    self.gam.draw_line(self.gid, wall).expect("couldn't draw maze wall");
	}
    }
    pub(crate) fn rawkeys(&mut self, keys: [char; 4]) {
	// placeholder for now
        log::info!("got rawkey {:?}", keys);
    }
}

pub(crate) fn labyrinth_pump_thread(cid_to_main: xous::CID, pump_sid: xous::SID) {
    let _ = std::thread::spawn({
        let cid_to_main = cid_to_main; // kind of redundant but I like making the closure captures explicit
        let sid = pump_sid;
        move || {
            let tt = ticktimer_server::Ticktimer::new().unwrap();
            let cid_to_self = xous::connect(sid).unwrap();
            let mut run = true;
            loop {
                // this blocks the process until a message is received, descheduling it from the run queue
                let msg = xous::receive_message(sid).unwrap();
                match FromPrimitive::from_usize(msg.body.id()) {
                    Some(PumpOp::Run) => {
                        run = true;
                        xous::send_message(
                            cid_to_self,
                            Message::new_scalar(PumpOp::Pump.to_usize().unwrap(), 0, 0, 0, 0)
                        ).expect("couldn't pump the main loop event thread");
                    },
                    Some(PumpOp::Stop) => run = false,
                    Some(PumpOp::Pump) => {
                        xous::send_message(
                            cid_to_main,
                            Message::new_blocking_scalar(AppOp::Pump.to_usize().unwrap(), 0, 0, 0, 0)
                        ).expect("couldn't pump the main loop event thread");
                        if run {
                            tt.sleep_ms(LABYRINTH_UPDATE_RATE_MS).unwrap();
                            xous::send_message(
                                cid_to_self,
                                Message::new_scalar(PumpOp::Pump.to_usize().unwrap(), 0, 0, 0, 0)
                            ).expect("couldn't pump the main loop event thread");
                        }
                    }
                    Some(PumpOp::Quit) => {
                        xous::return_scalar(msg.sender, 1).expect("couldn't ack the quit message");
                        break;
                    }
                    _ => log::error!("Got unrecognized message: {:?}", msg),
                }
            }
            xous::destroy_server(sid).ok();
        }
    });
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Tile {
    border_right: bool,
    border_bottom: bool,
    visited: bool,
}
impl Tile {
    fn new() -> Tile {
	Tile{border_right: true, border_bottom: true, visited: false}
    }
}
// create maze!
fn create_maze<const WIDTH: usize, const HEIGHT: usize>(trng: &trng::Trng) -> Vec<Vec<Arc<Mutex<Tile>>>> {
    // generate random maze
    let mut maze = Vec::with_capacity(WIDTH);
    for _ in 0..WIDTH {
	let mut row = Vec::with_capacity(HEIGHT);
	for _ in 0..HEIGHT {
	    row.push(Arc::new(Mutex::new(Tile::new())));
	}
	maze.push(row);
    }
    let (mut x, mut y) = (0, 0);
    let mut tile_stack = vec![(Arc::clone(&maze[x][y]), (0, 0))];
    log::info!("starting maze generation");
    loop {
	let old = &tile_stack[tile_stack.len()-1];
	let old_tile = Arc::clone(&old.0);
	x = old.1.0;
	y = old.1.1;
	old_tile.lock().unwrap().visited = true;
	let mut possible_moves = Vec::new();
	let mut possible_directions = Vec::new();
	if x > 0 && !maze[x-1][y].lock().unwrap().visited {
	    possible_moves.push(Arc::clone(&maze[x-1][y]));
	    possible_directions.push("l");
	}
	if y > 0 && !maze[x][y-1].lock().unwrap().visited {
	    possible_moves.push(Arc::clone(&maze[x][y-1]));
	    possible_directions.push("u");
	}
	if x < WIDTH-1 && !maze[x+1][y].lock().unwrap().visited {
	    possible_moves.push(Arc::clone(&maze[x+1][y]));
	    possible_directions.push("r");
	}
	if y < HEIGHT-1 && !maze[x][y+1].lock().unwrap().visited {
	    possible_moves.push(Arc::clone(&maze[x][y+1]));
	    possible_directions.push("d");
	}
	let num_possible_moves = possible_moves.len();
	if num_possible_moves == 0 {
	    tile_stack.pop();
	    if tile_stack.len() == 0 {
		break;
	    } else {
		continue;
	    }
	}
	let move_index = trng.get_u32().unwrap() as usize % num_possible_moves;
	let new_tile = Arc::clone(&possible_moves[move_index]);
	let dir = possible_directions[move_index];
	match dir {
	    "l" => {
		new_tile.lock().unwrap().border_right = false;
		x -= 1;
	    },
	    "u" => {
		new_tile.lock().unwrap().border_bottom = false;
		y -= 1;
	    },
	    "r" => {
		old_tile.lock().unwrap().border_right = false;
		x += 1;
	    },
	    "d" => {
		old_tile.lock().unwrap().border_bottom = false;
		y += 1;
	    },
	    _ => log::error!("Unexpected direction!")
	}
	tile_stack.push((new_tile, (x, y)));
    }
    return maze;
}
fn create_walls<const SIZE: usize>(maze: &Vec<Vec<Arc<Mutex<Tile>>>>) -> Vec<Line> {
    // get walls
    log::info!("generating walls");
    let mut walls = Vec::new();
    for (col_num, col) in maze.into_iter().enumerate() {
	for (tile_num, tile) in col.into_iter().enumerate() {
	    if tile.lock().unwrap().border_right {
		walls.push(
		    Line::new_with_style(
			Point::new(((col_num+1)*SIZE).try_into().unwrap(), (tile_num*SIZE).try_into().unwrap()),
			Point::new(((col_num+1)*SIZE).try_into().unwrap(), ((tile_num+1)*SIZE).try_into().unwrap()),
			DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 1)
		    )
		);
	    }
	    if tile.lock().unwrap().border_bottom {
		walls.push(
		    Line::new_with_style(
			Point::new((col_num*SIZE).try_into().unwrap(), ((tile_num+1)*SIZE).try_into().unwrap()),
			Point::new(((col_num+1)*SIZE).try_into().unwrap(), ((tile_num+1)*SIZE).try_into().unwrap()),
			DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 1)
		    )
		);
	    }
	}
    }
    return walls;
}
