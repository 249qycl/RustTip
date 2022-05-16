use chrono::prelude::*;
use regex::Regex;

#[derive(Debug)]
pub enum UtilError {
    TimeLateErr,
    ParseError,
    FormatError, //邮箱格式错误
}

pub struct Util;
impl Util {
    pub fn check_email(email: &str) -> Result<bool, UtilError> {
        let re = Regex::new(r"^[A-Za-z\d]+([-_.][A-Za-z\d]+)*@([A-Za-z\d]+[-.])+[A-Za-z\d]{2,4}$")
            .unwrap();
        if re.is_match(email) {
            return Ok(true);
        }
        Err(UtilError::FormatError)
    }

    pub fn check_date(date: &str) -> Result<NaiveDate, UtilError> {
        let now: NaiveDateTime = NaiveDateTimeWrapper::from(Local::now()).into();
        let dst = NaiveDate::parse_from_str(date, "%Y-%m-%d");
        if dst.unwrap() < now.date() {
            return Err(UtilError::TimeLateErr);
        }
        if dst.is_ok() {
            Ok(dst.unwrap())
        } else {
            Err(UtilError::ParseError)
        }
    }
    pub fn check_date_time(date: &str, time: &str) -> Result<NaiveDateTime, UtilError> {
        let now = NaiveDateTimeWrapper::from(Local::now()).into();
        let dst = NaiveDateTime::parse_from_str(
            format!("{} {}", date, time).as_str(),
            "%Y-%m-%d %H:%M:%S",
        );
        if dst.unwrap() < now {
            return Err(UtilError::TimeLateErr);
        }
        if dst.is_ok() {
            Ok(dst.unwrap())
        } else {
            Err(UtilError::ParseError)
        }
    }
}

pub struct NaiveDateTimeWrapper {
    naive_dt: NaiveDateTime,
}
impl From<DateTime<Local>> for NaiveDateTimeWrapper {
    fn from(datetime: DateTime<Local>) -> NaiveDateTimeWrapper {
        NaiveDateTimeWrapper {
            naive_dt: NaiveDateTime::parse_from_str(
                datetime.to_string().split(".").collect::<Vec<&str>>()[0],
                "%Y-%m-%d %H:%M:%S",
            )
            .unwrap(),
        }
    }
}
impl Into<NaiveDateTime> for NaiveDateTimeWrapper {
    fn into(self) -> NaiveDateTime {
        self.naive_dt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_naive_date_time_wrapper() {
        let now = Local::now();
        let n_dt_w: NaiveDateTime = NaiveDateTimeWrapper::from(now).into();
        let n_dt = NaiveDateTime::parse_from_str(
            now.to_string().split(".").collect::<Vec<&str>>()[0],
            "%Y-%m-%d %H:%M:%S",
        )
        .unwrap();
        assert_eq!(n_dt_w.date(), n_dt.date());
        assert_eq!(n_dt_w.time(), n_dt.time());
    }
}
