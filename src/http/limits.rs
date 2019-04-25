use crate::errors::*;
use crate::model::types::RateLimited;
use futures::compat::*;
use parking_lot::{Mutex, MutexGuard, MappedMutexGuard};
use serde::de::DeserializeOwned;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::str::FromStr;
use std::time::{SystemTime, Duration, UNIX_EPOCH, Instant};
use reqwest::StatusCode;
use reqwest::r#async::{Response, RequestBuilder};
use reqwest::header::*;
use tokio::timer::Delay;

// TODO: Add support for garbage collecting old channels and guilds.
// TODO: Add error contexts.

const SECOND: Duration = Duration::from_secs(1);
fn time_diff(resets_at: SystemTime) -> Duration {
    let now = SystemTime::now();
    match resets_at.duration_since(now) {
        Ok(diff) => max(diff, SECOND),
        Err(_) => SECOND,
    }
}

/// Stores information about a particular rate limit.
#[derive(Debug)]
enum RawRateLimit {
    NoLimitAvailable,
    Known {
        limit: u32,
        remaining: u32,
        resets_at: Instant,
        first_encountered_reset_period: SystemTime,
        estimated_reset_period: Duration,
    },
}
impl RawRateLimit {
    fn check_wait_until(&mut self) -> Option<Instant> {
        match self {
            RawRateLimit::NoLimitAvailable => None,
            RawRateLimit::Known { limit, remaining, resets_at, estimated_reset_period, .. } => {
                let now = Instant::now();
                if *resets_at < now {
                    // Estimate how long we have until we get rate limited again.
                    // This should only happen when we have many concurrent API calls at once.
                    *remaining = *limit;
                    *resets_at = now + *estimated_reset_period;
                }
                if *remaining > 0 {
                    *remaining -= 1;
                    None
                } else {
                    Some(*resets_at)
                }
            }
        }
    }
    fn push_rate_limit(&mut self, info: RateLimitHeaders) {
        let replace = match self {
            RawRateLimit::NoLimitAvailable => true,
            RawRateLimit::Known {
                limit, remaining, first_encountered_reset_period, ..
            } => if *first_encountered_reset_period == info.resets_at && *limit == info.limit {
                *remaining = min(*remaining, info.remaining);
                false
            } else {
                true
            },
        };
        if replace {
            let time_diff = time_diff(info.resets_at);
            let resets_at = Instant::now() + time_diff;
            *self = RawRateLimit::Known {
                limit: info.limit, remaining: info.remaining, resets_at,
                first_encountered_reset_period: info.resets_at,
                estimated_reset_period: time_diff
            }
        }
    }
}
impl Default for RawRateLimit {
    fn default() -> Self {
        RawRateLimit::NoLimitAvailable
    }
}

pub type GlobalLimit = Mutex<Option<Instant>>;

async fn wait_until(time: Instant) {
    if time > Instant::now() {
        await!(Delay::new(time).compat()).unwrap();
    }
}

async fn check_global_wait(global_limit: &GlobalLimit) -> bool {
    let time = {
        let mut lock = global_limit.lock();
        if let Some(time) = *lock {
            if time < Instant::now() {
                *lock = None;
            }
        }
        *lock
    };

    if let Some(time) = time {
        debug!("Waiting for preexisting global rate limit until {:?}", time);
        await!(wait_until(time));
        true
    } else {
        false
    }
}
fn check_route_wait(mut lock: MappedMutexGuard<RawRateLimit>) -> impl Future<Output = bool> {
    let time = lock.check_wait_until();
    drop(lock);
    async move {
        if let Some(time) = time {
            debug!("Waiting for preexisting route rate limit until {:?}", time);
            await!(wait_until(time));
            true
        } else {
            false
        }
    }
}

struct RateLimitHeaders {
    limit: u32, remaining: u32, resets_at: SystemTime,
}
fn parse_header<T: FromStr>(
    headers: &HeaderMap, name: &'static str,
) -> Result<Option<T>> where <T as FromStr>::Err: Into<Error> {
    match headers.get(name) {
        Some(header) => {
            let header_str = header.to_str()
                .context(ErrorKind::DiscordBadResponse("Invalid UTF-8 in header."))?;
            let header = header_str.parse::<T>()
                .context(ErrorKind::DiscordBadResponse("Could not parse header."))?;
            Ok(Some(header))
        }
        None => Ok(None),
    }
}
fn parse_headers(response: &Response) -> Result<Option<RateLimitHeaders>> {
    let headers = response.headers();

    let global    = parse_header::<bool>(headers, "X-RateLimit-Global")?.unwrap_or(false);
    let limit     = parse_header::<u32>(headers, "X-RateLimit-Limit")?;
    let remaining = parse_header::<u32>(headers, "X-RateLimit-Remaining")?;
    let reset     = parse_header::<u64>(headers, "X-RateLimit-Reset")?;
    let any_limit = limit.is_some() || remaining.is_some() || reset.is_some();
    let all_limit = limit.is_some() && remaining.is_some() && reset.is_some();

    if global {
        if any_limit {
            bail!(DiscordBadResponse, "X-RateLimit-Global returned alongside other rate limits.");
        }
        Ok(None)
    } else if any_limit {
        if !all_limit {
            bail!(DiscordBadResponse, "Incomplete rate limit headers returned.");
        }
        Ok(Some(RateLimitHeaders {
            limit: limit.unwrap(),
            remaining: remaining.unwrap(),
            resets_at: UNIX_EPOCH + Duration::from_secs(reset.unwrap()),
        }))
    } else {
        Ok(None)
    }
}

enum ResponseStatus {
    Success(Option<RateLimitHeaders>, Response),
    RateLimited(Option<RateLimitHeaders>, Duration),
    GloballyRateLimited(Duration),
}
async fn check_response(request: RequestBuilder) -> Result<ResponseStatus> {
    let mut response = await!(request.send().compat())?;
    if response.status().is_success() {
        let rate_info = parse_headers(&response)?;
        Ok(ResponseStatus::Success(rate_info, response))
    } else if response.status() == StatusCode::TOO_MANY_REQUESTS {
        let rate_info = await!(response.json::<RateLimited>().compat())?;
        debug!("Encountered rate limit: {:?}", rate_info);
        if rate_info.global {
            Ok(ResponseStatus::GloballyRateLimited(rate_info.retry_after))
        } else {
            Ok(ResponseStatus::RateLimited(parse_headers(&response)?, rate_info.retry_after))
        }
    } else {
        unimplemented!()
    }
}

fn push_rate_info(mut lock: MappedMutexGuard<RawRateLimit>, headers: Option<RateLimitHeaders>) {
    if let Some(headers) = headers {
        lock.push_rate_limit(headers)
    }
}
fn push_global_rate_limit(global_limit: &GlobalLimit, target: Instant) {
    let mut lock = global_limit.lock();
    if lock.is_none() || lock.unwrap() < target {
        *lock = Some(target)
    }
}

async fn perform_rate_limited<'a, T: DeserializeOwned>(
    global_limit: &'a GlobalLimit,
    lock_raw_limit: impl Fn() -> MappedMutexGuard<'a, RawRateLimit> + 'a,
    make_request: impl Fn() -> RequestBuilder + 'a,
) -> Result<T> {
    loop {
        loop {
            if await!(check_global_wait(global_limit)) { continue }
            if await!(check_route_wait(lock_raw_limit())) { continue }
            break
        }
        match await!(check_response(make_request()))? {
            ResponseStatus::Success(rate_limit, mut response) => {
                push_rate_info(lock_raw_limit(), rate_limit);
                return Ok(await!(response.json::<T>().compat())?)
            }
            ResponseStatus::RateLimited(rate_limit, wait_duration) => {
                push_rate_info(lock_raw_limit(), rate_limit);
                await!(wait_until(Instant::now() + wait_duration));
            }
            ResponseStatus::GloballyRateLimited(wait_duration) => {
                let time = Instant::now() + wait_duration;
                push_global_rate_limit(global_limit, time);
                await!(wait_until(time));
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct RateLimit(Mutex<RawRateLimit>);
impl RateLimit {
    pub fn perform_rate_limited<'a, T: DeserializeOwned + 'a>(
        &'a self, global_limit: &'a GlobalLimit,
        make_request: impl Fn() -> RequestBuilder + 'a,
    ) -> impl Future<Output = Result<T>> + 'a {
        perform_rate_limited(global_limit, move || {
            MutexGuard::map(self.0.lock(), |x| x)
        }, make_request)
    }
}

#[derive(Default, Debug)]
pub struct RateLimitSet<K: Eq + Hash + Copy>(Mutex<HashMap<K, RawRateLimit>>);
impl <K: Eq + Hash + Copy> RateLimitSet<K> {
    pub fn perform_rate_limited<'a, T: DeserializeOwned + 'a>(
        &'a self, global_limit: &'a GlobalLimit,
        make_request: impl Fn() -> RequestBuilder + 'a, k: K,
    ) -> impl Future<Output = Result<T>> + 'a {
        perform_rate_limited(global_limit, move || {
            MutexGuard::map(self.0.lock(), |x|
                x.entry(k).or_insert_with(|| RawRateLimit::default())
            )
        }, make_request)
    }
}