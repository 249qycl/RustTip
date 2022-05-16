use super::app::{self, Server, User};
use super::util::{NaiveDateTimeWrapper, Util, UtilError};
use chrono::prelude::*;
use clap::{Arg, SubCommand};
use std::env;
use std::process::Command;

pub enum CliError {
    UtilError(UtilError),
    InputError,
    NoneError,
}

impl From<UtilError> for CliError {
    fn from(err: UtilError) -> CliError {
        CliError::UtilError(err)
    }
}
impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            CliError::UtilError(err) => match err {
                UtilError::TimeLateErr => write!(f, "穿越失败"),
                UtilError::ParseError => write!(f, "时间格式错误"),
                UtilError::FormatError => write!(f, "邮箱格式错误"),
            },
            CliError::InputError => write!(f, "输入格式错误"),
            CliError::NoneError => Ok(()),
        }
    }
}

pub fn read_command() -> Result<app::App, CliError> {
    let matches = clap::App::new("RuTip")
        .subcommand(
            SubCommand::with_name("urg")
                .arg(Arg::with_name("email").required(true))
                .help("Eg: RustTip urg 邮箱"),
        )
        .subcommand(
            SubCommand::with_name("finish")
                .arg(Arg::with_name("email").required(true))
                .help("Eg: RustTip finish 邮箱"),
        )
        .subcommand(
            SubCommand::with_name("user")
                .arg(Arg::with_name("email").required(true))
                .arg(Arg::with_name("date").help("Eg:2022-1-1"))
                .arg(Arg::with_name("time").help("Eg:14:30:00"))
                .help("Eg: RustTip user 邮箱 日期(可选) 时间(可选)"),
        )
        .subcommand(
            SubCommand::with_name("server")
                .arg(Arg::with_name("account").required(true))
                .arg(Arg::with_name("password").required(true))
                .help("Eg: RustTip server 邮箱 SMTP服务密码"),
        )
        .subcommand(
            SubCommand::with_name("subserver")
                .arg(Arg::with_name("account").required(true))
                .arg(Arg::with_name("password").required(true)),
        )
        .subcommand(
            SubCommand::with_name("stop").help("Eg: RustTip stop"),
        )
        .help("自动预约: RustTip user 邮箱 日期(可选) 时间(可选)\n取消预约: RustTip finish 邮箱\n紧急预约: RustTip urg 邮箱\n服务启动: RustTip server 邮箱 SMTP服务密码\n服务关闭: RustTip stop")
        .get_matches();

    match matches.subcommand() {
        ("user", Some(sub)) => {
            let naive_dt: NaiveDateTime = NaiveDateTimeWrapper::from(Local::now()).into();
            let mut info: (String, Option<NaiveDate>, Option<NaiveTime>) =
                (String::new(), Some(naive_dt.date()), Some(naive_dt.time()));

            if sub.value_of("email").is_some() {
                Util::check_email(sub.value_of("email").unwrap())?;
                info.0 = sub.value_of("email").unwrap().to_string();
            } else {
                Err(CliError::InputError)?;
            }
            if sub.value_of("date").is_some() && sub.value_of("time").is_some() {
                let datetime = Util::check_date_time(
                    sub.value_of("date").unwrap(),
                    sub.value_of("time").unwrap(),
                )?;
                info.1 = Some(datetime.date());
                info.2 = Some(datetime.time());
            } else if sub.value_of("date").is_some() {
                let date = Util::check_date(sub.value_of("date").unwrap())?;
                info.1 = Some(date);
            }
            return Ok(app::App::User(User::new(
                info.0, info.1, info.2, false, false,
            )));
        }
        ("server", Some(_)) => {
            //启动子进程，参数全部传递给子进程
            let mut args: Vec<String> = env::args().collect();
            args[1] = String::from("subserver");
            Command::new(&args[0])
                .args(&args[1..])
                .spawn()
                .expect("Child process failed to start.");
            Err(CliError::NoneError)?;
        }
        ("subserver", Some(sub)) => {
            if sub.value_of("account").is_some() && sub.value_of("password").is_some() {
                Util::check_email(sub.value_of("account").unwrap())?;
                return Ok(app::App::Server(Server::new(
                    sub.value_of("account").unwrap().to_string(),
                    sub.value_of("password").unwrap().to_string(),
                )));
            } else {
                Err(CliError::InputError)?;
            }
        }

        ("urg", Some(sub)) => {
            if sub.value_of("email").is_some() {
                Util::check_email(sub.value_of("email").unwrap())?;
                return Ok(app::App::User(User::new(
                    sub.value_of("email").unwrap().to_string(),
                    None,
                    None,
                    true,
                    false,
                )));
            } else {
                Err(CliError::InputError)?;
            }
        }
        ("finish", Some(sub)) => {
            if sub.value_of("email").is_some() {
                Util::check_email(sub.value_of("email").unwrap())?;
                return Ok(app::App::User(User::new(
                    sub.value_of("email").unwrap().to_string(),
                    None,
                    None,
                    false,
                    true,
                )));
            } else {
                Err(CliError::InputError)?;
            }
        }
        ("stop", Some(_))=> {
            return Ok(app::App::User(User::new(
                String::from("stop@stop.stop"),
                None,
                None,
                false,
                true,
            )));
        }
        _ => Err(CliError::InputError)?,
    }

    Err(CliError::InputError)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_cli_error_display() {
        assert_eq!(
            format!("{}", super::CliError::InputError),
            String::from("输入格式错误")
        );
    }
}
