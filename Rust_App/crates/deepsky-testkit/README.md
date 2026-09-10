# Synthetic backend

`SimulatorBackend::new(TimingMode::Accelerated)` implements every operation of
`deepsky_camera::backend::CameraBackend`. `Default` selects accelerated timing;
`with_seed(mode, seed)` selects a repeatable noise sequence. `default_request()`
provides an explicit complete configuration for the synthetic instrument.

Open its discovered selection, configure, then capture or preview. Configuration
is transactional: rejected requests preserve the previous configuration. Capture
requires explicit exposure, sensitivity, stream and focus. Missing settings are
not reported as measured values. Manual WB gains describe interpretation of Bayer
data and do not multiply the RAW payload. Preview is a simple Gray8 rendering.

Identity and timestamp domain explicitly say SYNTHETIC. The single 128×96 RGGB
RAW16 little-endian stream, ranges and temperatures are invented test parameters,
not Pixel capabilities. Output is not DNG and makes no scientific calibration claim.
Exposure and sensitivity scale stars, manual defocus spreads their light, zoom
changes their positions, and seed/frame index control background noise.

Accelerated timing advances a virtual clock without sleeping. Realtime sleeps for
the same frame period plus stream stall; both report virtual timestamps, not wall
clock measurements. Metadata start/end bound the simulated frame cycle including
stall, while reported exposure and frame duration remain separate. Preview does
not consume a capture ID or advance this clock.

`inject_failure` affects the next operation once. Disconnect/crash clear the open
camera and configuration; reopen and reconfigure to recover. Timeout and I/O
failure preserve configuration and do not emit or advance a frame. Thermal warning
sets a persistent synthetic warning available through `thermal()`.

Run `cargo test -j 1 -p deepsky-camera -p deepsky-testkit` from Rust_App.
