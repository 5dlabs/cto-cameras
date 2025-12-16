pub mod server;

use lazy_static::lazy_static;
use prometheus::{CounterVec, GaugeVec, HistogramOpts, HistogramVec, Opts, Registry};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    // Camera connection status (0 = disconnected, 1 = connected)
    pub static ref CAMERA_CONNECTED: GaugeVec = GaugeVec::new(
        Opts::new("camera_stream_connected", "Camera RTSP stream connection status"),
        &["camera_id"]
    ).unwrap();

    // Total segments recorded
    pub static ref SEGMENTS_RECORDED: CounterVec = CounterVec::new(
        Opts::new("camera_segments_recorded_total", "Total number of video segments recorded"),
        &["camera_id"]
    ).unwrap();

    // Total segments uploaded
    pub static ref SEGMENTS_UPLOADED: CounterVec = CounterVec::new(
        Opts::new("camera_segments_uploaded_total", "Total number of segments uploaded to storage"),
        &["camera_id"]
    ).unwrap();

    // Upload failures
    pub static ref UPLOAD_FAILURES: CounterVec = CounterVec::new(
        Opts::new("camera_upload_failures_total", "Total number of upload failures"),
        &["camera_id"]
    ).unwrap();

    // Upload duration
    pub static ref UPLOAD_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new("camera_segment_upload_duration_seconds", "Time taken to upload segments")
            .buckets(vec![1.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0]),
        &["camera_id"]
    ).unwrap();

    // FFmpeg restarts
    pub static ref FFMPEG_RESTARTS: CounterVec = CounterVec::new(
        Opts::new("camera_ffmpeg_restarts_total", "Total number of FFmpeg process restarts"),
        &["camera_id"]
    ).unwrap();

    // Total bytes recorded
    pub static ref RECORDING_BYTES: CounterVec = CounterVec::new(
        Opts::new("camera_recording_bytes_total", "Total bytes recorded"),
        &["camera_id"]
    ).unwrap();
}

/// Initialize metrics registry
pub fn init_metrics() -> Result<(), prometheus::Error> {
    REGISTRY.register(Box::new(CAMERA_CONNECTED.clone()))?;
    REGISTRY.register(Box::new(SEGMENTS_RECORDED.clone()))?;
    REGISTRY.register(Box::new(SEGMENTS_UPLOADED.clone()))?;
    REGISTRY.register(Box::new(UPLOAD_FAILURES.clone()))?;
    REGISTRY.register(Box::new(UPLOAD_DURATION.clone()))?;
    REGISTRY.register(Box::new(FFMPEG_RESTARTS.clone()))?;
    REGISTRY.register(Box::new(RECORDING_BYTES.clone()))?;
    Ok(())
}
