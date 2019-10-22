use crate::errors::*;
use crate::model::types::Snowflake;
use crate::serde::*;
use futures::compat::*;
use parking_lot::Mutex;
use std::cmp::min;
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::hash::Hash;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, Duration, UNIX_EPOCH, Instant};
use reqwest::StatusCode;
use reqwest::r#async::{Response, RequestBuilder};
use reqwest::header::*;
use tokio::timer::Delay;
use failure::_core::fmt::Debug;

// TODO: Add support for garbage collecting old channels and guilds.
// TODO: Add error contexts.

/// A struct representing a rate limited API call.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
struct RateLimited {
    message: String,
    #[serde(with = "utils::duration_millis")]
    retry_after: Duration,
    global: bool,
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
            *self = RawRateLimit::Known {
                limit: info.limit,
                remaining: info.remaining,
                resets_at: info.resets_at_instant,
                first_encountered_reset_period: info.resets_at,
                estimated_reset_period: info.resets_in,
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
        Delay::new(time).compat().await.unwrap();
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
        wait_until(time).await;
        true
    } else {
        false
    }
}
fn check_route_wait(lock: &mut RawRateLimit) -> impl Future<Output = bool> {
    let time = lock.check_wait_until();
    async move {
        if let Some(time) = time {
            debug!("Waiting for preexisting route rate limit until {:?}", time);
            wait_until(time).await;
            true
        } else {
            false
        }
    }
}

#[derive(Debug)]
struct RateLimitHeaders {
    limit: u32, remaining: u32,
    resets_at: SystemTime, resets_at_instant: Instant, resets_in: Duration,
    bucket: String,
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
    let now = Instant::now();

    let global      = parse_header::<bool>(headers, "X-RateLimit-Global")?.unwrap_or(false);
    let limit       = parse_header::<u32>(headers, "X-RateLimit-Limit")?;
    let remaining   = parse_header::<u32>(headers, "X-RateLimit-Remaining")?;
    let reset       = parse_header::<f64>(headers, "X-RateLimit-Reset")?;
    let reset_after = parse_header::<f64>(headers, "X-RateLimit-Reset-After")?;
    let bucket      = headers.get("X-RateLimit-Bucket");
    let any_limit   = limit.is_some() || remaining.is_some() || reset.is_some() ||
                      reset_after.is_some() || bucket.is_some();
    let all_limit   = limit.is_some() && remaining.is_some() && reset.is_some() &&
                      reset_after.is_some() && bucket.is_some();

    if global {
        if any_limit {
            bail!(DiscordBadResponse, "X-RateLimit-Global returned alongside other rate limits.");
        }
        Ok(None)
    } else if any_limit {
        if !all_limit {
            bail!(DiscordBadResponse, "Incomplete rate limit headers returned.");
        }
        let resets_in = Duration::from_secs_f64(reset_after.unwrap());
        Ok(Some(RateLimitHeaders {
            limit: limit.unwrap(),
            remaining: remaining.unwrap(),
            resets_at: UNIX_EPOCH + Duration::from_secs_f64(reset.unwrap()),
            resets_at_instant: now + resets_in,
            resets_in,
            bucket: bucket.unwrap().to_str()?.to_string(),
        }))
    } else {
        Ok(None)
    }
}

#[derive(Debug)]
enum ResponseStatus {
    Success(Option<RateLimitHeaders>, Response),
    RateLimited(Option<RateLimitHeaders>, Duration),
    GloballyRateLimited(Duration),
}
async fn check_response(
    request: RequestBuilder,
    reason: &Option<String>,
) -> Result<ResponseStatus> {
    let mut request = request.header("X-RateLimit-Precision", "millisecond");
    if let Some(reason) = &reason {
        request = request.header("X-Audit-Log-Reason", reason);
    }
    let mut response = request.send().compat().await?;
    if response.status().is_success() {
        let rate_info = parse_headers(&response)?;
        Ok(ResponseStatus::Success(rate_info, response))
    } else if response.status() == StatusCode::TOO_MANY_REQUESTS {
        let rate_info = response.json::<RateLimited>().compat().await?;
        debug!("Encountered rate limit: {:?}", rate_info);
        if rate_info.global {
            Ok(ResponseStatus::GloballyRateLimited(rate_info.retry_after))
        } else {
            Ok(ResponseStatus::RateLimited(parse_headers(&response)?, rate_info.retry_after))
        }
    } else {
        // TODO: Handle other HTTP errors.
        unimplemented!()
    }
}

fn push_global_rate_limit(global_limit: &GlobalLimit, target: Instant) {
    let mut lock = global_limit.lock();
    if lock.is_none() || lock.unwrap() < target {
        *lock = Some(target)
    }
}

#[derive(Default, Debug)]
pub struct RateLimitStore(
    Mutex<HashMap<String, Arc<Mutex<HashMap<Snowflake, RawRateLimit>>>>>,
);
impl RateLimitStore {
    fn get_bucket(&self, bucket: String) -> Arc<Mutex<HashMap<Snowflake, RawRateLimit>>> {
        let mut buckets = self.0.lock();
        buckets.entry(bucket).or_insert_with(
            || Arc::new(Mutex::new(HashMap::new()))
        ).clone()
    }
}

struct RateLimitRouteData {
    bucket: String,
    limit: Arc<Mutex<HashMap<Snowflake, RawRateLimit>>>,
}

#[derive(Default)]
pub struct RateLimitRoute {
    data: Mutex<Option<RateLimitRouteData>>,
}
impl RateLimitRoute {
    async fn check_wait(&self, global_limit: &GlobalLimit, id: Snowflake) {
        loop {
            if check_global_wait(global_limit).await { continue }
            let fut = self.data.lock().as_ref().map(|routes| {
                check_route_wait(
                    routes.limit.lock().entry(id).or_insert(RawRateLimit::NoLimitAvailable),
                )
            });
            if let Some(fut) = fut {
                if fut.await { continue }
            }
            break
        }
    }
    fn push_rate_info(
        &self,
        limits: Option<RateLimitHeaders>,
        store: &RateLimitStore,
        id: Snowflake,
    ) {
        if let Some(limits) = limits {
            let mut data = self.data.lock();
            if data.as_ref().map_or(true, |x| x.bucket == limits.bucket) {
                *data = Some(RateLimitRouteData {
                    bucket: limits.bucket.clone(),
                    limit: store.get_bucket(limits.bucket.clone()),
                });
            }
            let mut routes = data.as_ref().unwrap().limit.lock();
            let limit = routes.entry(id).or_insert(RawRateLimit::NoLimitAvailable);
            limit.push_rate_limit(limits);
        } else {
            let mut data = self.data.lock();
            *data = None;
        }
    }

    pub async fn perform_rate_limited(
        &self,
        global_limit: &GlobalLimit,
        store: &RateLimitStore,
        make_request: impl Fn() -> Result<RequestBuilder>,
        reason: Option<String>,
        id: Snowflake,
    ) -> Result<Response> {
        loop {
            self.check_wait(global_limit, id).await;
            match check_response(make_request()?, &reason).await? {
                ResponseStatus::Success(rate_limit, response) => {
                    self.push_rate_info(rate_limit, store, id);
                    return Ok(response)
                }
                ResponseStatus::RateLimited(rate_limit, wait_duration) => {
                    self.push_rate_info(rate_limit, store, id);
                    wait_until(Instant::now() + wait_duration).await;
                }
                ResponseStatus::GloballyRateLimited(wait_duration) => {
                    let time = Instant::now() + wait_duration;
                    push_global_rate_limit(global_limit, time);
                    wait_until(time).await;
                }
            }
        }
    }
}
impl fmt::Debug for RateLimitRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RateLimit(")?;
        let lock = self.data.lock();
        if lock.is_none() {
            f.write_str("<none>")?;
        } else {
            lock.as_ref().unwrap().bucket.fmt(f)?;
        }
        drop(lock);
        f.write_str(")")
    }
}