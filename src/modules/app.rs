use super::config;
use super::nvidia;
use super::util::NaiveDateTimeWrapper;
use chrono::{prelude::*, Duration};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde::{Deserialize, Serialize};
use std::cmp::*;
use std::io::{prelude::*, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::str;
use std::sync::Mutex;
use std::thread;
use std::time;

use lazy_static::lazy_static;

#[derive(Clone, Debug)] //记录每个user的申请时刻，作为排序依据
pub struct User {
    urg: bool,
    finish: bool,
    timestamp: i64,
    email: String,
    date_time: NaiveDateTime,
    // time: Option<NaiveTime>,
}

impl User {
    pub fn new(
        email: String,
        date: Option<NaiveDate>,
        time: Option<NaiveTime>,
        urg: bool,
        finish: bool,
    ) -> User {
        //不指定时刻默认采用当前时刻
        if date.is_none() || time.is_none() {
            User {
                urg,
                finish,
                timestamp: Local::now().timestamp(),
                email,
                date_time: NaiveDateTimeWrapper::from(Local::now()).into(),
            }
        } else {
            User {
                urg,
                finish,
                timestamp: Local::now().timestamp(),
                email,
                date_time: NaiveDateTime::new(date.unwrap(), time.unwrap()),
            }
        }
    }
    //向文件写
    fn send_by_tcp(&self) {
        let curr_info = UserWrapper::from(self.clone());
        let mut stream = TcpStream::connect("127.0.0.1:7630").expect("Tcp connect failed");
        let mut info = serde_json::to_string(&curr_info).unwrap();
        info.push_str("\n");
        stream
            .write(info.as_bytes())
            .expect("Failed to write to stream");
    }
    //user只修改条目，不删除
    pub fn run(&mut self) {
        self.send_by_tcp();
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Server {
    account: String,
    password: String,
}
lazy_static! {
    static ref RECV_DATA: Mutex<Vec<UserWrapper>> = Mutex::new(Vec::new());
    static ref THREAD_ALIVE: Mutex<bool> = Mutex::new(true);
}
impl Server {
    pub fn new(account: String, password: String) -> Server {
        Server { account, password }
    }
    fn is_server_existed(&self) -> bool {
        TcpStream::connect(config::TCP_ADDR).is_ok()
    }
    //从监听中抓取一个数据，监听持续运行
    fn receive_by_tcp(&self) -> Option<Vec<UserWrapper>> {
        let mut lck = RECV_DATA.lock().unwrap();
        let ret = Some(lck.clone());
        lck.clear();
        ret
    }
    fn server_stop(&self,user:UserWrapper) ->bool {
        if user.email==String::from("stop@stop.stop"){
            let mut lck=THREAD_ALIVE.lock().unwrap();
            *lck=false;
            return true;
        }
        false
    }
    pub fn run(&self) {
        if self.is_server_existed() {
            return;
        } else {
            App::tcp_runtime();
        }
        let mut gpu = nvidia::Nvidia::new();
        let mut app_info = AppInfo::load();
        app_info.server_info = self.clone();
        'first_loop: loop {
            let users = self.receive_by_tcp();
            if let Some(users) = users {
                for user in users {
                    if self.server_stop(user.clone()){
                        break 'first_loop;
                    }
                    //更新数据库
                    if app_info.user_info.contains_key(&user.email) {
                        if let Some(x) = app_info.user_info.get_mut(&user.email) {
                            *x = user.clone();
                        }
                    } else {
                        app_info.user_info.insert(user.email.clone(), user.clone());
                    }
                    //邮件通知
                    if user.finish {
                        app_info.send_email(
                            user.email.clone(),
                            "任务注销通知",
                            &format! {"用户{}注销成功！欢迎下次预约！",user.email},
                        );
                    } else {
                        app_info.send_email(
                            user.email.clone(),
                            "服务器预约通知",
                            &format! {"用户{}预约成功！服务器就绪后将自动通知您！",user.email},
                        );
                    }
                }
            }
            app_info.update_current_user();
            //设备诊断通知
            app_info.dialog(&mut gpu);
            thread::sleep(time::Duration::from_secs(1));
            //备份
            app_info.write();
        }
    }
}

#[derive(Debug)]
pub enum App {
    User(User),
    Server(Server),
}
impl App {
    pub fn run(&mut self) {
        match self {
            App::User(user) => user.run(),
            App::Server(server) => server.run(),
        }
    }
    fn tcp_runtime() {
        thread::spawn(move || {
            let listener = TcpListener::bind(config::TCP_ADDR).expect("Tcp listen failed");
            let alive = { *THREAD_ALIVE.lock().unwrap() };
            while alive {
                let mut thread_vec: Vec<thread::JoinHandle<()>> = Vec::new();
                for stream in listener.incoming() {
                    let stream = stream.expect("failed!");
                    let handle = thread::spawn(move || {
                        let mut reader = BufReader::new(&stream);
                        let mut buffer: Vec<u8> = Vec::new();
                        reader
                            .read_until(b'\n', &mut buffer)
                            .expect("Could not read into buffer");
                        let info =
                            str::from_utf8(&buffer).expect("Could not write buffer as string");
                        let res: UserWrapper = serde_json::from_str(info).unwrap();
                        let mut lck = RECV_DATA.lock().unwrap();
                        lck.push(res);
                    });
                    thread_vec.push(handle);
                }
                for handle in thread_vec {
                    handle.join().unwrap();
                }
            }
        });
    }
}
impl Drop for App {
    fn drop(&mut self) {
        let mut lck = THREAD_ALIVE.lock().unwrap();
        *lck = false;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct UserWrapper {
    urg: bool,
    finish: bool,
    timestamp: i64,
    email: String,
    date_time: String,
}
impl From<User> for UserWrapper {
    fn from(user: User) -> Self {
        let date_time: String = user.date_time.to_string();
        UserWrapper {
            urg: user.urg,
            finish: user.finish,
            timestamp: user.timestamp,
            email: user.email,
            date_time: date_time,
        }
    }
}
impl Into<User> for UserWrapper {
    fn into(self) -> User {
        let date_time: NaiveDateTime =
            NaiveDateTime::parse_from_str(self.date_time.as_str(), "%Y-%m-%d %H:%M:%S").unwrap();
        User {
            urg: self.urg,
            finish: self.finish,
            timestamp: self.timestamp,
            email: self.email,
            date_time: date_time,
        }
    }
}

use std::collections::BTreeMap;
#[derive(Serialize, Deserialize, Debug)]
struct AppInfo {
    server_info: Server,
    curr_user: Option<UserWrapper>,
    user_info: BTreeMap<String, UserWrapper>, //使用email到info到映射
}
lazy_static! {
    static ref LAST_TIME: Mutex<Option<NaiveTime>> = Mutex::new(None);
    static ref TIME_GAP: Mutex<Duration> = Mutex::new(Duration::seconds(config::TIME_GAP_SECONDS));
}
impl AppInfo {
    fn load() -> AppInfo {
        //文件不存在，构造对象，否则使用加载
        let info: AppInfo = AppInfo {
            server_info: Server::new(String::from(""), String::from("")),
            curr_user: None,
            user_info: BTreeMap::new(),
        };
        match std::fs::read_to_string(config::INFO_FILE) {
            Ok(data) => serde_json::from_str(&data).unwrap(),
            Err(_) => info,
        }
    }

    fn write(&self) {
        let mut writer = std::fs::File::create(config::INFO_FILE).unwrap();
        let info = serde_json::to_string(self).unwrap();
        writer.write_all(info.as_bytes()).unwrap();
    }

    fn update_current_user(&mut self) {
        //同步map内容到curr_user
        if self.curr_user.is_some() {
            self.curr_user = Some(
                self.user_info
                    .get(&self.curr_user.as_ref().unwrap().email)
                    .unwrap()
                    .clone(),
            );
        }
        //清除map中所有finish的对象
        let temp_users = self.user_info.clone();
        for user in temp_users.iter() {
            if user.1.finish {
                self.user_info.remove(user.0);
            }
        }
        //更新curr_user
        if self.curr_user.is_none() || self.curr_user.as_ref().unwrap().finish {
            self.curr_user = self.get_new_user();
            //重置诊断计时
            let mut gap = TIME_GAP.lock().unwrap();
            *gap = Duration::seconds(config::TIME_GAP_SECONDS);
        }
    }
    fn get_new_user(&self) -> Option<UserWrapper> {
        //基于urg、时间戳比较
        let mut users: Vec<(String, User)> = Vec::new();
        for info in self.user_info.clone() {
            users.push((info.0, info.1.into()));
        }

        users.sort_by(|a, b| {
            if a.1.urg > b.1.urg {
                return Ordering::Greater;
            } else if a.1.urg < b.1.urg {
                return Ordering::Less;
            } else {
                if a.1.timestamp < b.1.timestamp {
                    return Ordering::Greater;
                } else if a.1.timestamp > b.1.timestamp {
                    return Ordering::Less;
                } else {
                    return Ordering::Equal;
                }
            }
        });
        //取最大的点
        if users.len() == 0 {
            None
        } else if users.len() == 1 {
            Some(UserWrapper::from(users[0].1.clone()))
        } else {
            let now: NaiveDateTime = NaiveDateTimeWrapper::from(Local::now()).into();
            let mut ret = Some(UserWrapper::from(users[users.len() - 1].1.clone()));
            if users[users.len() - 1].1.date_time - now <= Duration::hours(10) {
                ret
            } else {
                for i in (0..users.len() - 1).rev() {
                    if users[i].1.date_time - now <= Duration::hours(10) {
                        ret = Some(UserWrapper::from(users[i].1.clone()));
                        break;
                    }
                }
                ret
            }
        }
    }

    fn send_email(&self, mut receiver: String, subject: &str, body: &String) {
        let creds = Credentials::new(
            self.server_info.account.clone(),
            self.server_info.password.clone(),
        );
        let mailer = SmtpTransport::relay(config::SERVER)
            .unwrap()
            .credentials(creds)
            .build();
        let mut account = self.server_info.account.clone();
        account.insert_str(0, "<");
        account.push_str(">");
        receiver.insert_str(0, "<");
        receiver.push_str(">");
        let msg = Message::builder()
            .from(account.parse().unwrap())
            .to(receiver.parse().unwrap())
            .subject(subject)
            .body(body.clone())
            .unwrap();
        while mailer.send(&msg).is_err() {}
    }
    fn dialog(&self, gpu: &mut nvidia::Nvidia) {
        let now = Local::now().time();
        let start_time = NaiveTime::parse_from_str("08:00:00", "%H:%M:%S").unwrap();
        let end_time = NaiveTime::parse_from_str("21:30:00", "%H:%M:%S").unwrap();
        let mut last_time = LAST_TIME.lock().unwrap();
        let mut gap = TIME_GAP.lock().unwrap();
        let time_diff = if last_time.is_none() || now - last_time.unwrap() > *gap {
            true
        } else {
            false
        };

        //任务结束通知，每段时间通知一次，指数增长
        let bound = now >= start_time && now <= end_time && time_diff && self.curr_user.is_some();
        gpu.read_from_terminal();
        if gpu.is_free() && bound {
            *last_time = Some(now);
            *gap = *gap * 2;
            if *gap > Duration::minutes(config::TIME_GAP_MAX_MINUTES) {
                *gap = Duration::minutes(config::TIME_GAP_MAX_MINUTES);
            }
            self.send_email(self.curr_user.as_ref().unwrap().email.clone(), "设备空闲通知",
            &format! {"用户{}设备空闲，请在服务器进行确认！",self.curr_user.as_ref().unwrap().email});
        }
        if gpu.is_low_efficiency() && bound {
            *last_time = Some(now);
            *gap = *gap * 2;
            if *gap > Duration::minutes(config::TIME_GAP_MAX_MINUTES) {
                *gap = Duration::minutes(config::TIME_GAP_MAX_MINUTES);
            }
            self.send_email(self.curr_user.as_ref().unwrap().email.clone(), "任务效率通知",
            &format! {"用户{}当前设备运行效率较低，请检查！",self.curr_user.as_ref().unwrap().email});
        }
    }
}
