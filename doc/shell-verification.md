# Shell verification and PC capture behavior

No Windows vision or UI automation is required for these tests.

## Verified on 2026-09-12

Rust workspace tests (including desktop feature), Rust optimized release build,
Android release build, debug unit tests and release lint passed. The updated
release APK was installed over ADB. No Windows visual interaction was used for
this verification.

The physical Pixel test passed with these observations:

| Check | Observed result |
| --- | --- |
| Color preview, 10 ms / ISO 100 | RGB 640x480; 9,996,350 ns / ISO 100 |
| Color preview, 50 ms / ISO 800 | RGB 640x480; 49,999,495 ns / ISO 799 |
| Center autofocus | Locked, 883 millidiopters; retained for RAW capture |
| 1-second RAW | 4080x3072; 25,107,216 bytes; 999,971,248 ns; cycle 3.315 s |
| 16-second RAW | 4080x3072; 25,107,216 bytes; 15,999,539,968 ns; cycle 33.331 s |

Both RAW files passed saved-session checksum verification. Sessions are in
`Rust_App/target/release/captures/SHELL_VERIFICATION_2026-09-12T073237Z_026260600`
and `SHELL_VERIFICATION_2026-09-12T073240Z_739178500`.
The 33.331-second first-cycle latency remains; increasing the timeout prevents
failure but is not a cadence improvement. These two captures do not establish
steady-state sequence throughput. Stop/pause/resume and numeric-input parsing
passed automated regression tests, not interactive desktop UI testing.

## Automated checks

From `Rust_App`:

```powershell
cargo test --workspace --features deepsky-ui/desktop -j 2
cargo build -p deepsky-app --features desktop --release -j 2
```

From `Android_App`:

```powershell
.tools/gradle-9.6.0/bin/gradle.bat :app:assembleRelease :app:testDebugUnitTest :app:lintRelease --no-daemon
```

Unit tests use explicitly synthetic fixtures to exercise errors and timing.
They are not hardware measurements. The hardware test below uses only ADB and
requires `Origin::Device`; it never falls back to a simulator.

## Real camera regression test

Authorize USB debugging, unlock Android, and start the DeepskyEyes service.
Disconnect any other desktop camera client. In `Rust_App`:

```powershell
$env:PATH = 'C:\platform-tools;' + $env:PATH
$env:DEEPSKY_TEST_SERIAL = '44101FDJG003S3'
$env:DEEPSKY_TEST_OUT = 'E:\project-seri\DeepskyEyes\Rust_App\target\release\captures'
cargo test -p deepsky-app --release --lib hardware_pc_controls_autofocus_and_full_raw -- --ignored --nocapture --test-threads=1
```

This test takes actual photographs and keeps them in separate
`SHELL_VERIFICATION_*` sessions. It checks color preview, changes exposure and
ISO, runs center autofocus, verifies that its measured distance survives manual
lock, takes 1-second and 16-second RAW exposures, checks stream dimensions and
1x zoom, saves files on the PC, and verifies their checksums through the session
recovery scanner. These files are test images, not astronomical calibration data.

## Timing is not one number

Exposure is the requested integration time, with the actual value in Camera2
capture metadata. The displayed cycle includes camera startup/readout, USB and
processing. There is no implicit one-second pause between desktop sequence
frames. On the tested Pixel HAL, the first valid long exposure can span a
pipeline frame plus exposure; early startup images with impossible timing are
discarded, not saved as valid RAWs. The timeout permits this overhead but does
not sleep or manufacture frame cadence.

## Full-frame storage

Desktop default storage is `captures` next to the EXE, independent of its working
directory. Each session has `lights`, `darks`, `flats`, `bias`, `test`, and
`metadata` directories. RAW capture uses the selected sensor stream dimensions
and 1x camera zoom, independently of preview zoom. Pixel bytes are not resized
or cropped. RAW16 packing strips only row/pixel padding and rejects truncated
buffers. The default preview display is Fit; Fill affects display only.

Numeric fields accept keyboard input (decimal comma or point), Ctrl+A,
Ctrl+V and Backspace; Enter confirms, Escape cancels. Frame count must be a
positive integer. Hardware settings are still validated against discovery.

The local Android release is optimized and non-debuggable, signed with the
existing development key for in-place device updates, not a store signing key.
