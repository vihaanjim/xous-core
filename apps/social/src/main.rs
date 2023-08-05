#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

mod social;
use social::SocialFeed;
mod client;

use num_traits::*;
use xous_ipc::Buffer;

pub(crate) const SERVER_NAME_SOCIAL: &str = "_ActivityPub client_";

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
pub(crate) enum SocialOp {
    /// redraw our UI
    Redraw = 0,
    /// get a keystroke
    Key,
    /// exit the application
    Quit,
}

fn main() -> ! {
    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);
    log::info!("my PID is {}", xous::process::id());

    let xns = xous_names::XousNames::new().unwrap();
    // unlimited connections allowed, this is a user app and it's up to the app to decide its policy
    let sid = xns.register_name(SERVER_NAME_SOCIAL, None).expect("can't register server");

    let mut social_feed = SocialFeed::new(&xns, sid);
    loop {
        let msg = xous::receive_message(sid).unwrap();
        log::debug!("got message {:?}", msg);
        match FromPrimitive::from_usize(msg.body.id()) {
	    Some(SocialOp::Redraw) => {
                social_feed.redraw().expect("MTXCLI couldn't redraw");
	    },
	    Some(SocialOp::Key) => xous::msg_scalar_unpack!(msg, k1, _, _, _, {
		let c = core::char::from_u32(k1 as u32).unwrap_or('\u{0000}');
		if c == 'p' {
		    social_feed.scroll_up().expect("couldn't scroll up");
		}
		if c == 'n' {
		    social_feed.scroll_down().expect("couldn't scroll down");
		}
	    }),
	    Some(SocialOp::Quit) => {
		log::error!("got Quit");
                break;
	    },
	    _ => {
		log::error!("got unknown message");
	    }
	}
	log::trace!("reached bottom of main loop");
    }
    // clean up our program
    log::error!("main loop exit, destroying servers");
    xns.unregister_server(sid).unwrap();
    xous::destroy_server(sid).unwrap();
    log::trace!("quitting");
    xous::terminate_process(0)
}
