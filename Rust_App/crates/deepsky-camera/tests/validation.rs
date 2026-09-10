use deepsky_camera::*;
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
