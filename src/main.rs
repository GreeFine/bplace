#![feature(map_try_insert)]
#[warn(unused_extern_crates)]
#[macro_use]
extern crate lazy_static;
mod database;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use actix::prelude::*;
use actix::{Actor, StreamHandler, WrapFuture};
use actix_cors::Cors;
use actix_web::{
    error, get, middleware::Logger, web, App, Error, HttpRequest, HttpResponse, HttpServer,
    Responder, Result,
};
use actix_web_actors::ws;
use chrono::{format::StrftimeItems, Duration, NaiveDateTime, Utc};
use database::{create_pixel, UserPixel};
use env_logger::Env;

use crate::database::get_canvas;

type User = Addr<MyWs>;
type Users = Arc<RwLock<HashMap<String, User>>>;
type UsersLimits = Arc<RwLock<HashMap<String, (u8, NaiveDateTime)>>>;

lazy_static! {
    pub static ref USERS: Users = Arc::new(RwLock::new(HashMap::new()));
    pub static ref USER_LIMITS: UsersLimits = Arc::new(RwLock::new(HashMap::new()));
}

const PIXEL_PER_USER: u8 = 30;
const USER_LIMIT_RESET_HOURS: i64 = 2;
const UTC_FR_DIFFERENCE: i64 = 2;

pub struct MyWs {
    username: String,
    ip: String,
}

impl Actor for MyWs {
    type Context = ws::WebsocketContext<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        println!("User connection: [{}] -> {}", self.ip, self.username);
        {
            let mut users_limits = USER_LIMITS
                .write()
                .expect("unable to get lock on users_limits");
            let _ = users_limits.try_insert(
                self.username.clone(),
                (
                    PIXEL_PER_USER,
                    Utc::now().naive_utc() + Duration::hours(USER_LIMIT_RESET_HOURS),
                ),
            );
        }
        let mut users = USERS.write().expect("unable to get lock on users");
        users.insert(self.username.clone(), ctx.address());
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        println!("User disconnecting: [{}] -> {}", self.ip, self.username);
        let mut users = USERS.write().expect("unable to get lock on users");
        users.remove(&self.username);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

impl MyWs {
    fn broadcast(&self, msg: String) {
        let users = USERS.write().expect("unable to get lock on users");
        for (_, user) in users.iter() {
            user.do_send(Message(msg.clone()));
        }
    }
}

impl actix::Handler<Message> for MyWs {
    type Result = ();
    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) {
        ctx.text(msg.0)
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        if let Ok(ws::Message::Text(text)) = msg {
            if let Ok(pixel) = serde_json::from_str::<UserPixel>(&text) {
                let username = self.username.clone();
                {
                    let mut users_limits = USER_LIMITS
                        .write()
                        .expect("unable to get lock on users_limits");
                    let user_limit = users_limits[&username];
                    let user_pixel_remaining;
                    if user_limit.1 < Utc::now().naive_utc() {
                        users_limits.insert(
                            username.clone(),
                            (
                                PIXEL_PER_USER - 1,
                                Utc::now().naive_utc()
                                    + Duration::hours(UTC_FR_DIFFERENCE)
                                    + Duration::hours(USER_LIMIT_RESET_HOURS),
                            ),
                        );
                        user_pixel_remaining = PIXEL_PER_USER - 1;
                    } else if user_limit.0 > 0 {
                        user_pixel_remaining = user_limit.0 - 1;
                        users_limits.insert(username.clone(), (user_pixel_remaining, user_limit.1));
                    } else {
                        let fmt = StrftimeItems::new("%Y-%m-%d %H:%M:%S").clone();
                        ctx.text(format!(
                            r#"{{"error":"Pixel limit, reset at: {}"}}"#,
                            user_limit.1.format_with_items(fmt)
                        ));
                        return;
                    }
                    ctx.text(format!(
                        r#"{{"pixel_limit":"{}/{}"}}"#,
                        user_pixel_remaining, PIXEL_PER_USER
                    ));
                }
                let future = async move {
                    create_pixel(pixel, username)
                        .await
                        .expect("Unable to create pixel in database");
                };
                future.into_actor(self).spawn(ctx);
                self.broadcast(text.to_string());
            } else {
                ctx.text(r#"{"error":"unable to parse pixel"}"#);
            };
        }
    }
}

#[get("/ws/{username}")]
async fn ws_start(
    req: HttpRequest,
    path: web::Path<(String,)>,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap()
        .to_string();
    ws::start(
        MyWs {
            username: path.into_inner().0,
            ip,
        },
        &req,
        stream,
    )
}

async fn index() -> Result<impl Responder> {
    let resp = get_canvas()
        .await
        .map_err(|_| error::ErrorInternalServerError("unable to get canvas"))?;
    Ok(web::Json(resp))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    HttpServer::new(move || {
        let cors = Cors::default().allowed_origin_fn(|_, _req_head| true);
        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .service(ws_start)
            .route("/", web::get().to(index))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
