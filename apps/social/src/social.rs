use super::*;
use client::{Client, Status};
use gam::UxRegistration;
use graphics_server::{Gid, Point, Rectangle, TextBounds, TextView, DrawStyle, PixelColor};
use graphics_server::api::GlyphStyle;
use core::fmt::Write;

pub(crate) struct SocialFeed {
    content: Gid,
    gam: gam::Gam,

    // variables that define our graphical attributes
    screensize: Point,
    margin: Point, // margin to edge of canvas
    // how much we've scrolled down
    scroll: i16,

    // client
    client: Client,
    feed: Vec<Status>,
    
    // our security token for making changes to our record on the GAM
    token: [u32; 4],
}

impl SocialFeed {
    pub(crate) fn new(xns: &xous_names::XousNames, sid: xous::SID) -> Self {
        let gam = gam::Gam::new(xns).expect("can't connect to GAM");

        let token = gam.register_ux(UxRegistration {
            app_name: xous_ipc::String::<128>::from_str(gam::APP_NAME_SOCIAL),
            ux_type: gam::UxType::Framebuffer,
            predictor: None,
            listener: sid.to_array(), // note disclosure of our SID to the GAM -- the secret is now shared with the GAM!
            redraw_id: SocialOp::Redraw.to_u32().unwrap(),
            gotinput_id: None,
            audioframe_id: None,
            rawkeys_id: Some(SocialOp::Key.to_u32().unwrap()),
            focuschange_id: None,
        }).expect("couldn't register Ux context for social");

        let content = gam.request_content_canvas(token.unwrap()).expect("couldn't get content canvas");
        let screensize = gam.get_canvas_bounds(content).expect("couldn't get dimensions of content canvas");
	let mut client = Client::new();
	client.authenticate();
	let _ = client.check_creds();
	let feed = client.get_feed();
        SocialFeed {
            content,
            gam,
            screensize,
	    scroll: 0,
            margin: Point::new(4, 4),
	    client,
	    feed,
            token: token.unwrap(),
        }
    }

    pub(crate) fn scroll_up(&mut self) -> Result<(), xous::Error> {
	if self.scroll < 0 {
	    self.scroll += 5;
	}
	self.redraw()
    }

    pub(crate) fn scroll_down(&mut self) -> Result<(), xous::Error> {
	self.scroll -= 5;
	self.redraw()
    }
    
    fn clear_area(&self) {
        self.gam.draw_rectangle(self.content,
            Rectangle::new_with_style(Point::new(0, 0), self.screensize,
            DrawStyle {
                fill_color: Some(PixelColor::Light),
                stroke_color: None,
                stroke_width: 0
            }
        )).expect("can't clear content area");
    }
    
    pub(crate) fn redraw(&mut self) -> Result<(), xous::Error> {
	self.clear_area();
	let mut y = self.scroll;
	for status in &self.feed {
	    let mut text_view = TextView::new(self.content,
					      TextBounds::GrowableFromTl(
						  Point::new(self.margin.x, y),
						  (self.screensize.x-2*self.margin.x).try_into().unwrap()));
	    text_view.border_width = 1;
            text_view.draw_border = true;
            text_view.clear_area = true;
            text_view.rounded_border = Some(3);
	    text_view.style = GlyphStyle::Regular;
	    let text = format!("{}\n{}", status.account.username, status.content);
	    write!(text_view.text, "{}", text).expect("Couldn't write to Social feed!");
	    self.gam.post_textview(&mut text_view).expect("couldn't render textview");
	    log::trace!("social app feed redraw##");
            self.gam.redraw().expect("couldn't redraw screen");

	    if let Some(bounds) = text_view.bounds_computed {
		y += bounds.br.y-bounds.tl.y;
	    } else {
		// we have gone off bottom of screen, no need to continue
		break;
	    }
	}
        Ok(())
    }
}
