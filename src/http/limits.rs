use crate::errors::*;
use crate::http::{SENTINEL, DiscordError, DiscordErrorCode, HttpConfig};
use crate::model::types::{Snowflake, DiscordToken};
use crate::serde::*;
use fnv::FnvHashMap;
use futures::compat::*;
use parking_lot::Mutex;
use std::cmp::{max, min};
use std::fmt;
use std::hash::Hash;
use std::panic::{AssertUnwindSafe, resume_unwind};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, Duration, UNIX_EPOCH, Instant};
use reqwest::StatusCode;
use reqwest::r#async::{Response, RequestBuilder};
use reqwest::header::*;
use tokio::timer::Delay;
use futures::FutureExt;

/// A struct representing a rate limited API call.
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
struct RateLimited {
    message: String,
    #[serde(with = "utils::duration_millis")]
    retry_after: Duration,
    global: bool,
}

/// The estimated limits for a particular bucket, used to seed new rate limits.
///
/// We make conservative estimates, but reset every once in a while, in case the actual rate limits
/// at Discord have changed.
#[derive(Copy, Clone, Debug)]
struct EstimatedLimits {
    expires_at: Instant,
    first_seen: SystemTime,
    limit: u32,
    reset_period: Duration,
}
impl EstimatedLimits {
    /// Seeds this from a particular guild/user's estimated limits.
    fn seed_from(opt: &mut Option<Self>, other: EstimatedLimits) {
        if let Some(limits) = opt {
            if Instant::now() > limits.expires_at {
                *limits = other;
            } else {
                limits.limit = min(other.limit, limits.limit);
                limits.reset_period = max(other.reset_period, limits.reset_period);
            }
        } else {
            *opt = Some(other);
        }
    }
}

#[derive(Debug)]
enum RateLimitData {
    /// No rate limit information is available.
    NoLimitAvailable,
    /// An response with no rate limit information was received.
    ReceivedNoLimits,
    /// Rate limits are known.
    Known {
        remaining: u32,
        resets_at: Instant,
        seeded_from_global: bool,
        estimated: EstimatedLimits,
    },
}

/// Stores information about a particular rate limit.
#[derive(Debug)]
struct RateLimit {
    /// How many API calls are active on this rate limit.
    consumed: u32,
    /// When this rate limit should be deleted to save memory.
    expires: Instant,
    /// The actual data of the rate limit.
    data: RateLimitData,
}

/// Checks whether times are the same, allowing for some floating point error.
fn is_same_time(a: SystemTime, b: SystemTime) -> bool {
    if a == b { return true }
    let min = min(a, b);
    let max = max(a, b);
    max.duration_since(min).unwrap() < Duration::from_millis(10)
}

impl RateLimit {
    fn new(config: &HttpConfig) -> RateLimit {
        RateLimit {
            consumed: 0,
            expires: Instant::now() + config.max_rate_limit_expired_period,
            data: RateLimitData::NoLimitAvailable,
        }
    }

    /// Checks whether we should wait for rate limits. If we shouldn't, it decrements the rate
    /// limit, assuming an API call will be made on this route.
    fn check_limit(
        &mut self, bucket_estimated: &Option<EstimatedLimits>, config: &HttpConfig,
    ) -> Option<Instant> {
        match &mut self.data {
            RateLimitData::NoLimitAvailable => {
                // If we have an estimate of the rate limits for this bucket, seed it from those.
                if let Some(estimated) = bucket_estimated {
                    self.data = RateLimitData::Known {
                        remaining: estimated.limit,
                        resets_at: Instant::now() + estimated.reset_period,
                        seeded_from_global: true,
                        estimated: *estimated,
                    };
                }
                None
            },
            RateLimitData::ReceivedNoLimits => None,
            RateLimitData::Known { remaining, resets_at, estimated, .. } => {
                let now = Instant::now();
                if *resets_at < now {
                    // Estimate how long we have until we get rate limited again.
                    // This should only happen when we have many concurrent API calls at once.
                    *remaining = estimated.limit;
                    *resets_at = now + estimated.reset_period;
                }
                if *remaining > self.consumed {
                    self.consumed += 1;
                    None
                } else if *remaining == 0 {
                    Some(*resets_at)
                } else {
                    Some(min(*resets_at, now + config.max_wait_for_active))
                }
            }
        }
    }

    /// Clears the limit (when no rate limit headers are received).
    fn clear_limit(&mut self, config: &HttpConfig) {
        self.data = RateLimitData::ReceivedNoLimits;
        self.expires = Instant::now() + config.max_rate_limit_expired_period;
    }

    /// Checks whether we should decrease the consumed count.
    fn call_completed(&mut self) {
        debug_assert!(self.consumed != 0);
        if self.consumed != 0 {
            self.consumed -= 1;
        }
    }

    /// Updates the rate limit from headers.
    fn update_limit(
        &mut self, config: &HttpConfig,
        bucket_estimated: &mut Option<EstimatedLimits>, info: RateLimitHeaders,
    ) {
        let replace = match &mut self.data {
            RateLimitData::NoLimitAvailable => true,
            RateLimitData::ReceivedNoLimits => true,
            RateLimitData::Known { estimated, remaining, seeded_from_global, .. } => {
                if *seeded_from_global ||
                    (is_same_time(estimated.first_seen, info.resets_at) &&
                     estimated.limit == info.limit)
                {
                    *remaining = min(*remaining, info.remaining);
                    false
                } else {
                    estimated.first_seen < info.resets_at
                }
            },
        };
        if replace {
            let estimated = EstimatedLimits {
                first_seen: info.resets_at,
                expires_at: Instant::now() + config.estimated_limits_expiry,
                limit: info.limit,
                reset_period: info.resets_in,
            };
            self.data = RateLimitData::Known {
                remaining: info.remaining,
                resets_at: info.resets_at_instant,
                seeded_from_global: false,
                estimated,
            };
            self.expires = info.resets_at_instant + config.max_rate_limit_expired_period;
            EstimatedLimits::seed_from(bucket_estimated, estimated);
        }
    }
}

/// The actual rate limits for a bucket.
#[derive(Debug)]
struct BucketLimits {
    /// The limit stored for routes with no parameters.
    only_limit: RateLimit,
    /// The main limit stores.
    limits: FnvHashMap<Snowflake, RateLimit>,
}
impl BucketLimits {
    fn get(&mut self, id: Snowflake, config: &HttpConfig) -> &mut RateLimit {
        if id == SENTINEL {
            &mut self.only_limit
        } else {
            if !self.limits.contains_key(&id) {
                self.limits.insert(id, RateLimit::new(config));
            }
            self.limits.get_mut(&id).unwrap()
        }
    }
}

/// Represents a particular bucket of the rate limits.
#[derive(Debug)]
struct Bucket {
    /// The next time expired rate limits will be purged.
    next_purge: Instant,
    /// The next time the rate limits hash will be reallocated.
    next_reallocate: Instant,
    /// A copy of the current http configuration.
    config: HttpConfig,
    /// The actual limits. Seperate for borrowck reasons.
    limits: BucketLimits,
    /// A cached set of estimated limits to seed new limits with.
    estimated_limits: Option<EstimatedLimits>,
}
impl Bucket {
    fn new(config: &HttpConfig) -> Self {
        let now = Instant::now();
        Bucket {
            next_purge: now + config.clear_rate_limits_period,
            next_reallocate: now + config.reallocate_caches_period,
            config: config.clone(),
            limits: BucketLimits {
                only_limit: RateLimit::new(config),
                limits: FnvHashMap::default(),
            },
            estimated_limits: None,
        }
    }

    fn do_checks(&mut self) {
        let now = Instant::now();
        if now > self.next_purge {
            self.limits.limits.retain(|_, v| now < v.expires);
            self.next_purge = now + self.config.clear_rate_limits_period;
            if now > self.next_reallocate {
                self.limits.limits.shrink_to_fit();
                self.next_reallocate = now + self.config.reallocate_caches_period;
            }
        }
    }

    /// Checks whether we should wait for a rate limit. If we shouldn't, it decrements the cached
    /// rate limit.
    fn check_limit(&mut self, id: Snowflake) -> Option<Instant> {
        let limit = self.limits.get(id, &self.config);
        let limit = limit.check_limit(&self.estimated_limits, &self.config);
        self.do_checks();
        limit
    }

    /// Updates the rate limit with new information.
    fn update_limit(&mut self, id: Snowflake, headers: Option<RateLimitHeaders>) {
        let limit = self.limits.get(id, &self.config);
        if let Some(headers) = headers {
            limit.update_limit(&self.config, &mut self.estimated_limits, headers);
        } else {
            limit.clear_limit(&self.config);
        }
        self.do_checks();
    }

    /// Removes the consumed flag after we're done.
    fn call_completed(&mut self, id: Snowflake) {
        let limit = self.limits.get(id, &self.config);
        limit.call_completed();
        self.do_checks();
    }
}

// Code to actually do the waiting
pub type GlobalLimit = Mutex<Option<Instant>>;
async fn wait_until(time: Instant) {
    if time > Instant::now() {
        Delay::new(time).compat().await.unwrap();
    }
}
fn push_global_rate_limit(global_limit: &GlobalLimit, target: Instant) {
    let mut lock = global_limit.lock();
    if lock.is_none() || lock.unwrap() < target {
        *lock = Some(target)
    }
}
async fn check_wait(
    id: Snowflake, bucket: Option<Arc<Mutex<Bucket>>>, global_limit: &GlobalLimit,
) {
    let mut waiting = false;
    let mut report_waiting = || {
        if !waiting {
            waiting = true;
            trace!("Waiting for rate limit...");
        }
    };
    loop {
        // Check global rate limit
        let global_result = {
            let mut lock = global_limit.lock();
            if let Some(time) = *lock {
                if time < Instant::now() {
                    *lock = None;
                }
            }
            *lock
        };
        if let Some(time) = global_result {
            report_waiting();
            wait_until(time).await;
            continue;
        }

        // Check per-route rate limit.
        if let Some(bucket) = &bucket {
            let local_result = bucket.lock().check_limit(id);
            if let Some(time) = local_result {
                report_waiting();
                wait_until(time).await;
            } else {
                return;
            }
        } else {
            return;
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
) -> Result<Option<T>> where <T as FromStr>::Err: Into<LibError> {
    match headers.get(name) {
        Some(header) => {
            let header_str = header.to_str().bad_response("Invalid UTF-8 in header.")?;
            let header = header_str.parse::<T>().bad_response("Could not parse header.")?;
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
    let bucket      = parse_header::<String>(headers, "X-RateLimit-Bucket")?;
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
            bucket: bucket.unwrap(),
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
async fn check_response<'a>(
    request: RequestBuilder,
    reason: &'a Option<String>,
    client_token: &'a Option<DiscordToken>,
    call_name: &'static str,
) -> Result<ResponseStatus> {
    let mut request = request.header("X-RateLimit-Precision", "millisecond");
    if let Some(reason) = &reason {
        request = request.header("X-Audit-Log-Reason", reason);
    }
    if let Some(client_token) = &client_token {
        request = request.header("Authorization", client_token.to_header_value());
    }
    let mut response = request.send().compat().await.io_err("Failed to make API request.")?;
    if response.status().is_success() {
        let rate_info = parse_headers(&response)?;
        Ok(ResponseStatus::Success(rate_info, response))
    } else if response.status() == StatusCode::TOO_MANY_REQUESTS {
        let rate_info = response.json::<RateLimited>().compat().await
            .context(ErrorKind::DiscordBadResponse("Could not parse rate limit information."))?;
        debug!("Encountered rate limit: {:?}", rate_info);
        if rate_info.global {
            Ok(ResponseStatus::GloballyRateLimited(rate_info.retry_after))
        } else {
            Ok(ResponseStatus::RateLimited(parse_headers(&response)?, rate_info.retry_after))
        }
    } else {
        let status = response.status();
        let discord_error = match response.json::<DiscordError>().compat().await {
            Ok(v) => v,
            Err(_) => DiscordError { code: DiscordErrorCode::NoStatusSent, message: None },
        };
        Err(Error::new_with_backtrace(ErrorKind::RequestFailed(call_name, status, discord_error)))
    }
}

#[derive(Debug)]
pub struct RateLimitStore {
    config: HttpConfig,
    buckets: FnvHashMap<String, Arc<Mutex<Bucket>>>,
}
impl RateLimitStore {
    pub fn new(config: HttpConfig) -> Self {
        RateLimitStore {
            config,
            buckets: FnvHashMap::default(),
        }
    }

    fn get_bucket(&mut self, bucket: String) -> Arc<Mutex<Bucket>> {
        if !self.buckets.contains_key(&bucket) {
            let new = Arc::new(Mutex::new(Bucket::new(&self.config)));
            self.buckets.insert(bucket.clone(), new);
        }
        self.buckets.get_mut(&bucket).unwrap().clone()
    }
}

struct RateLimitRouteData {
    bucket: String,
    limit: Arc<Mutex<Bucket>>,
}

#[derive(Default)]
pub struct RateLimitRoute {
    data: Mutex<Option<RateLimitRouteData>>,
}
impl RateLimitRoute {
    async fn check_wait(
        &self, id: Snowflake, global_limit: &GlobalLimit,
    ) -> Option<Arc<Mutex<Bucket>>> {
        let bucket = {
            let data = self.data.lock();
            data.as_ref().map(|x| x.limit.clone())
        };
        check_wait(id, bucket.clone(), global_limit).await;
        bucket
    }
    fn update_limits(
        &self,
        id: Snowflake,
        headers: Option<RateLimitHeaders>,
        store: &Mutex<RateLimitStore>,
    ) {
        let mut data = self.data.lock();
        if let Some(headers) = &headers {
            if data.as_ref().map_or(true, |x| x.bucket == headers.bucket) {
                let mut store = store.lock();
                *data = Some(RateLimitRouteData {
                    bucket: headers.bucket.clone(),
                    limit: store.get_bucket(headers.bucket.clone()),
                });
            }
        }
        if let Some(data) = data.as_ref() {
            data.limit.lock().update_limit(id, headers);
        }
    }

    pub async fn perform_rate_limited<'a>(
        &'a self,
        global_limit: &'a GlobalLimit,
        store: &'a Mutex<RateLimitStore>,
        use_rate_limits: bool,
        make_request: &'a (dyn Fn() -> Result<RequestBuilder> + Send + Sync),
        reason: Option<String>,
        client_token: Option<DiscordToken>,
        id: Snowflake,
        call_name: &'static str,
    ) -> Result<Response> {
        loop {
            let mut stored_bucket = None;
            if use_rate_limits {
                stored_bucket = self.check_wait(id, global_limit).await;
            }
            let panic_result: StdResult<Result<_>, _> = AssertUnwindSafe(async {
                trace!("Sending request...");
                match check_response(make_request()?, &reason, &client_token, call_name).await? {
                    ResponseStatus::Success(rate_limit, response) => {
                        if use_rate_limits {
                            self.update_limits(id, rate_limit, store);
                        }
                        Ok(Some(response))
                    }
                    ResponseStatus::RateLimited(rate_limit, wait_duration) => {
                        if use_rate_limits {
                            self.update_limits(id, rate_limit, store);
                        }
                        wait_until(Instant::now() + wait_duration).await;
                        Ok(None)
                    }
                    ResponseStatus::GloballyRateLimited(wait_duration) => {
                        let time = Instant::now() + wait_duration;
                        if use_rate_limits {
                            push_global_rate_limit(global_limit, time);
                        }
                        wait_until(time).await;
                        Ok(None)
                    }
                }
            }).catch_unwind().await;
            if let Some(bucket) = stored_bucket {
                bucket.lock().call_completed(id);
            }
            match panic_result {
                Ok(Ok(Some(v))) => return Ok(v),
                Ok(Ok(None)) => { }
                Ok(Err(e)) => return Err(e),
                Err(e) => resume_unwind(e),
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