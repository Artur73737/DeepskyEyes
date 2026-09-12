use deepsky_camera::*;

fn jpeg_caps() -> CameraCapabilities {
    CameraCapabilities {
        manual_sensor: Some(true),
        jpeg_thumbnail_sizes: vec![JpegSize { width: 0, height: 0 }, JpegSize { width: 320, height: 240 }],
        streams: vec![
            StreamConfiguration { width: 4032, height: 3024, format: PixelFormat::Jpeg, pixel_mode: SensorPixelMode::Default, binned: None, min_frame_duration_ns: None, stall_duration_ns: None },
            StreamConfiguration { width: 4080, height: 3072, format: PixelFormat::Raw16Le, pixel_mode: SensorPixelMode::Default, binned: None, min_frame_duration_ns: None, stall_duration_ns: None },
        ],
        ..Default::default()
    }
}
fn jpeg_request(caps: &CameraCapabilities, stream: usize, jpeg: JpegSettings) -> CaptureRequest {
    let mut settings = CaptureSettings::default();
    settings.stream = Some(caps.streams[stream].clone());
    settings.jpeg = Some(jpeg);
    CaptureRequest { request_id: 1, selection: CameraSelection { camera_id: caps.camera_id.clone(), physical_id: None }, settings }
}
#[test]
fn jpeg_quality_limits_reject() {
    let caps = jpeg_caps();
    for q in [1u8, 100] {
        let req = jpeg_request(&caps, 0, JpegSettings { quality: Some(q), ..Default::default() });
        caps.validate_request(&req, ValidationPolicy::Reject).unwrap();
    }
    for q in [0u8, 101, 255] {
        let req = jpeg_request(&caps, 0, JpegSettings { quality: Some(q), ..Default::default() });
        let err = caps.validate_request(&req, ValidationPolicy::Reject).unwrap_err();
        assert_eq!(err.code, ErrorCode::OutOfRange, "quality {q}");
    }
    let req = jpeg_request(&caps, 0, JpegSettings { thumbnail_quality: Some(0), ..Default::default() });
    assert_eq!(caps.validate_request(&req, ValidationPolicy::Reject).unwrap_err().code, ErrorCode::OutOfRange);
}
#[test]
fn jpeg_orientation_set_rejects() {
    let caps = jpeg_caps();
    for o in [0u16, 90, 180, 270] {
        let req = jpeg_request(&caps, 0, JpegSettings { orientation: Some(o), ..Default::default() });
        caps.validate_request(&req, ValidationPolicy::Reject).unwrap();
    }
    for o in [1u16, 45, 360] {
        let req = jpeg_request(&caps, 0, JpegSettings { orientation: Some(o), ..Default::default() });
        let err = caps.validate_request(&req, ValidationPolicy::Reject).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidRequest, "orientation {o}");
    }
}
#[test]
fn jpeg_thumbnail_size_must_be_announced() {
    let caps = jpeg_caps();
    let ok = jpeg_request(&caps, 0, JpegSettings { thumbnail_size: Some(JpegSize { width: 320, height: 240 }), ..Default::default() });
    caps.validate_request(&ok, ValidationPolicy::Reject).unwrap();
    let off = jpeg_request(&caps, 0, JpegSettings { thumbnail_size: Some(JpegSize { width: 0, height: 0 }), ..Default::default() });
    caps.validate_request(&off, ValidationPolicy::Reject).unwrap();
    let bad = jpeg_request(&caps, 0, JpegSettings { thumbnail_size: Some(JpegSize { width: 640, height: 480 }), ..Default::default() });
    assert_eq!(caps.validate_request(&bad, ValidationPolicy::Reject).unwrap_err().code, ErrorCode::Unsupported);
}
#[test]
fn jpeg_controls_refused_on_raw_stream() {
    let caps = jpeg_caps();
    let req = jpeg_request(&caps, 1, JpegSettings { quality: Some(90), ..Default::default() });
    let err = caps.validate_request(&req, ValidationPolicy::Reject).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    // Empty JPEG block on RAW is a no-op, not an error.
    let noop = jpeg_request(&caps, 1, JpegSettings::default());
    caps.validate_request(&noop, ValidationPolicy::Reject).unwrap();
}
#[test]
fn legacy_settings_without_jpeg_still_parse() {
    let old = r#"{"exposure_ns":100,"processing":{}}"#;
    let settings: CaptureSettings = serde_json::from_str(old).unwrap();
    assert_eq!(settings.jpeg, None);
    let mut v = serde_json::to_value(&jpeg_caps()).unwrap();
    v.as_object_mut().unwrap().remove("jpeg_thumbnail_sizes");
    let back: CameraCapabilities = serde_json::from_value(v).unwrap();
    assert!(back.jpeg_thumbnail_sizes.is_empty());
}
#[test]
fn missing_observation_survives_json() {
    let settings = CaptureSettings::default();
    let roundtrip: CaptureSettings = serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
    assert_eq!(roundtrip.exposure_ns, None); assert_eq!(roundtrip.focus, None); assert!(roundtrip.processing.is_empty());
}
#[test]
fn legacy_flags_do_not_infer_focus_wb_or_full_resolution() {
    let raw = deepsky_camera::capabilities::RawCapabilities { manual_sensor: true, manual_post_processing: true, raw_sizes: vec![(10,10),(20,20)], ..Default::default() };
    let model = deepsky_camera::capability_model::CapabilityModel::from_raw(&raw);
    assert!(!model.manual_focus); assert!(!model.manual_white_balance); assert!(!model.full_resolution_supported);
}
#[test]
fn malformed_capability_ranges_reject_even_without_requested_control() {
    let caps = CameraCapabilities { exposure_ns: Some(ValueRange { min: 0, max: 100 }), ..Default::default() };
    assert_eq!(caps.validate().unwrap_err().code, ErrorCode::InvalidCapabilities);
    assert!(ValueRange { min: 10, max: 1 }.validate().is_err());
}
