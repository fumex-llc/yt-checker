use actix_web::{web::Data, HttpResponse};
use futures_util::StreamExt;
use rand::{rng, RngExt};
use reqwest::{Client, Url};
use std::{process::Stdio, sync::atomic::AtomicUsize, io::ErrorKind};
use tokio::{process::Command, time::{Duration, Instant, timeout}, sync::Mutex};

use crate::args::Strategy;

type YtErr = std::io::Error;
type YtInvoke<T> = Result<T, YtErr>;

const YT_DLP_CMD: &str = "yt-dlp";
const RESULT_CACHE_TTL: Duration = Duration::from_secs(5);

async fn build_yt_player(video_id: String, height: u16, timeout_secs: u16,) -> YtInvoke<std::process::Output> {
    let height = format!("bestvideo[height<={height}]/bestvideo");
    let url = format!("https://www.youtube.com/watch?v={}", video_id);
    let socket_timeout = timeout_secs.to_string();
    let mut command = Command::new(YT_DLP_CMD);
    command
        .args(["--no-playlist", "-f", height.as_str(), "--get-url", "--no-warnings"])
        .arg("--socket-timeout")
        .arg(socket_timeout)
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    timeout(Duration::from_secs(timeout_secs as u64), command.output()).await.map_err(|_| {
        YtErr::new(ErrorKind::TimedOut, format!("yt-dlp timed out after {timeout_secs}s"))
    })?
}

struct Stats {
    bytes: u64,
    elapsed: Duration,
    url: String
}

impl Stats {
    fn mbps(&self) -> f64 {
        self.bytes as f64 * 8.0
            / self.elapsed.as_secs_f64()
            / 1_000_000.0
    }
}

pub struct YtChecker {
    urls: Vec<String>,
    quality: u16,
    timeout: u16,
    url_strategy: Strategy,
    with_v6: bool,
    last_h: AtomicUsize,
    check_lock: Mutex<()>,
    last_check: Mutex<Option<CachedCheck>>
}

#[derive(Clone)]
struct CachedCheck {
    checked_at: Instant,
    success: bool,
    error: Option<String>,
}

impl YtChecker {
    pub fn new(urls: Vec<String>, quality: u16, timeout: u16, with_v6: bool, url_strategy: Strategy) -> Self {
        Self {
            urls,
            quality,
            timeout,
            url_strategy,
            with_v6,
            last_h: AtomicUsize::new(0),
            check_lock: Mutex::new(()),
            last_check: Mutex::new(None)
        }
    }

    async fn check_cached(&self) -> Result<Option<Stats>, String> {
        let _guard = self.check_lock.lock().await;

        if let Some(cached) = self.last_check.lock().await.clone() {
            if cached.checked_at.elapsed() < RESULT_CACHE_TTL {
                return if cached.success {
                    Ok(None)
                } else {
                    Err(cached.error.unwrap_or_else(|| "recent YouTube check failed".to_owned()))
                };
            }
        }

        let result = self.check_youtube().await;
        let cached = CachedCheck {
            checked_at: Instant::now(),
            success: result.is_ok(),
            error: result.as_ref().err().map(ToString::to_string),
        };
        *self.last_check.lock().await = Some(cached);

        result.map(Some).map_err(|error| error.to_string())
    }

    async fn get_youtube_stream_url(&self, video_id: String) -> YtInvoke<String> {
        let out = build_yt_player(video_id, self.quality, self.timeout).await?;

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(YtErr::other(stderr.trim()))
        }

        let stdout = String::from_utf8(out.stdout).map_err(YtErr::other)?;

        let url = stdout.lines().find(|line| line.starts_with("https://"))
            .ok_or_else(|| YtErr::other("yt-dlp no URL"))?;

        if !url.contains("googlevideo.com") {
            return Err(YtErr::other("unexpected host: {url}"));
        }

        Ok(url.to_owned())
    }

    fn get_video_id(&self) -> YtInvoke<String> {
        match self.url_strategy {
            Strategy::Random => {
                let p = rng().random_range(0..self.urls.len());
                self.urls.get(p).cloned().ok_or(YtErr::other("index out of range"))
            }
            Strategy::RoundRobin => {
                let index = self.last_h.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % self.urls.len();
                self.urls.get(index).cloned().ok_or(YtErr::other("index out of range"))
            }
        }
    }

    async fn check_youtube(&self) -> YtInvoke<Stats> {
        let video_id = self.get_video_id()?;
        let video_url = self.get_youtube_stream_url(video_id.clone()).await?;

        let url = Url::parse(&video_url).map_err(YtErr::other)?;

        let host = url.host_str().ok_or(YtErr::other("googlevideo URL has no host"))?;

        let ipv6 = tokio::net::lookup_host((host, 443)).await?
            .find(|addr| addr.is_ipv6())
            .ok_or(YtErr::other("no IPv6 address for {host}"))?;

        let client = if self.with_v6 {
            Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(self.timeout as u64))
                .resolve(host, ipv6)
                .build()
                .map_err(YtErr::other)?
        } else {
            Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(self.timeout as u64))
                .build()
                .map_err(YtErr::other)?
        };

        let end = Duration::from_secs(self.timeout as u64);
        let start = Instant::now();

        let response = client
            .get(url)
            .send()
            .await
            .map_err(YtErr::other)?
            .error_for_status()
            .map_err(YtErr::other)?;

        let mut stream = response.bytes_stream();

        let mut bytes = 0u64;

        loop {
            let chunk = match timeout(end.saturating_sub(start.elapsed()), stream.next()).await {
                Ok(Some(chunk)) => chunk.map_err(YtErr::other)?,
                _ => break
            };

            bytes += chunk.len() as u64;
        }

        let elapsed = start.elapsed();

        if bytes == 0 {
            return Err(YtErr::other("YouTube returned zero bytes"))
        }

        Ok(Stats {bytes, elapsed, url: video_id})
    }
}

pub async fn check_youtube(yt: Data<YtChecker>) -> HttpResponse {
    match yt.check_cached().await {
        Ok(Some(stats)) => {
            log::info!(
                "YouTube video `https://www.youtube.com/watch?v={}` download OK: {:.2} MiB in {:.2}s ({:.2} Mbps)",
                stats.url,
                stats.bytes as f64 / 1024.0 / 1024.0,
                stats.elapsed.as_secs_f64(),
                stats.mbps(),
            );
            HttpResponse::NoContent().finish()
        }

        Ok(None) => HttpResponse::NoContent().finish(),

        Err(err) => {
            log::error!("YouTube video download FAILED: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
