use serde::Deserialize;
use ureq;

#[derive(Deserialize)]
pub struct Client {
    client_id: String,
    client_secret: String,
    token: Option<Token>,
}

impl Client {
    pub fn new() -> Client {
	ureq::post("https://mstdn.party/api/v1/apps?client_name=xous social&redirect_uris=urn:ietf:wg:oauth:2.0:oob&scopes=read write follow push&website=https://mstdn.party")
	    .call()
	    .expect("couldn't get client id/secret")
	    .into_json()
	    .expect("couldn't convert to json!")
    }

    pub fn authenticate(&mut self) {
	let token: Token = ureq::post(&format!("https://mstdn.party/oauth/token?grant_type=password&username=vsj314@icloud.com&password=RS9JULretz3bytx&client_id={}&client_secret={}", self.client_id, self.client_secret))
	    .call()
	    .expect("couldn't get token")
	    .into_json()
	    .expect("couldn't turn token into string");
	self.token = Some(token);
    }

    pub fn check_creds(&self) -> String {
	if let Some(token) = &self.token {
	    ureq::get("https://mstdn.party/api/v1/accounts/verify_credentials")
		.set("Authorization", &format!("Bearer {}", token.access_token))
		.call()
		.expect("couln't verify!")
		.into_string()
		.expect("couldn't turn verification into string!")
	}
	else {
	    panic!("no token!");
	}
    }

    pub fn get_feed(&self) -> Vec<Status> {
	if let Some(token) = &self.token {
	    ureq::get("https://mstdn.party/api/v1/timelines/home")
		.set("Authorization", &format!("Bearer {}", token.access_token))
		.call()
		.expect("couldn't get inbox")
		.into_json()
		.expect("couldn't turn inbox into string")
	}
	else {
	    panic!("no token!");
	}
    }
}

#[derive(Deserialize)]
pub struct Token {
    access_token: String,
}

#[derive(Deserialize)]
pub struct Status {
    pub content: String,
    pub account: Account
}

#[derive(Deserialize)]
pub struct Account {
    pub username: String
}
