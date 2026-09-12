# PC CLI → Kotlin → Camera2

The PC owns requests, sequences, validation, files and verification. Kotlin owns
camera discovery, Camera2 requests/results, buffers and DNG conversion. No simulator
is selected implicitly: CLI defaults to ADB; `--source sim` explicitly opts into fixtures.

## Start and inspect

```powershell
$env:PATH = 'C:\platform-tools;' + $env:PATH
$cli = 'E:\project-seri\DeepskyEyes\Rust_App\target\release\deepsky-eyes.exe'
adb -s 44101FDJG003S3 shell am start -n com.deepskyeyes.android/.MainActivity -a com.deepskyeyes.android.START_BRIDGE
& $cli discover
& $cli capabilities --camera 0
& $cli capability_dump --out camera-inventory.json
& $cli ping
& $cli status
& $cli thermal
& $cli autofocus --camera 0
```

USB authorization and Android camera permission are required. The start action
brings the activity to the foreground; it does not bypass the lock screen or OS
restrictions. Do not connect two camera clients concurrently. Use `--serial ID`
when more than one device is present. Output diagnostic files are explicitly
chosen by `--out`; request-template/execute refuse existing destinations.

The extended dump contains all readable characteristics, all available request
keys with `adapter_route`/`exposed`, and available result keys. `exposed=false`
is an explicit implementation boundary, not a statement that hardware lacks it.
Even an exposed and advertised setting can be overridden by the HAL: inspect
the result, not merely the successful configure response.

## Parameter map

| CLI / JSON settings | Camera2 | Evidence |
| --- | --- | --- |
| `--camera ID` / selection.camera_id | CameraManager camera ID | discovery, metadata camera/physical ID |
| `--stream WxH:FORMAT[:MODE]` | ImageReader format/size, SENSOR_PIXEL_MODE | exact announced stream, actual Image dimensions |
| `--exposure-ns` | SENSOR_EXPOSURE_TIME, AE OFF | reported exposure ns |
| `--sensitivity` | SENSOR_SENSITIVITY | reported sensitivity |
| `--frame-duration-ns` | SENSOR_FRAME_DURATION | reported duration; must cover exposure/stream minimum |
| `--focus-mdiopt` | AF OFF + LENS_FOCUS_DISTANCE | 1000 units = 1 diopter, reported distance |
| `autofocus` / preview `--autofocus` | AF AUTO, central region, trigger, lock result | measured distance; reuse with --focus-mdiopt |
| `--wb-preset` | CONTROL_AWB_MODE | reported preset |
| `--wb-kelvin` / Temperature | conditional temperature contract | Unsupported on current Pixel adapter; no substitution |
| JSON white_balance.Manual | gains + transform matrix when advertised | rejected if not advertised; no inferred gain range |
| `--zoom` / `--zoom-x1000` | CONTROL_ZOOM_RATIO | requested and reported ratio |
| `--crop x,y,w,h` | SCALER_CROP_REGION | requested and reported rectangle; HAL may round |
| `--ois`, `--eis` | lens optical / video stabilization mode | reported OIS/EIS |
| `--processing` | edge, noise reduction, hot pixel, shading, aberration, distortion, tonemap | each requested/reported mode |
| `--frames`, `--delay-ns` | PC sequence, not sensor exposure | committed count and sensor timestamps |
| `--project`, `--kind`, `--out` | PC session storage | immutable manifests, filenames, checksums |
| `--strict-results` | PC observation gate | frame retained, remaining sequence stopped, nonzero exit |

## Scientific acquisition

```powershell
& $cli sequence --camera 0 --project TARGET --kind light --frames 30 --exposure-ns 1000000000 --sensitivity 100 --focus-mdiopt 0 --wb-preset daylight --delay-ns 0
& $cli capture --camera 0 --project RAW16 --stream 2032x1536:Raw16Le --exposure-ns 100000000 --sensitivity 100 --wb-preset daylight
& $cli capture --camera 0 --project CONTROL_TEST --exposure-ns 100000000 --sensitivity 100 --wb-preset daylight --frame-duration-ns 200000000 --ois on --eis on --processing edge=fast,noise_reduction=fast,tonemap=high_quality
& $cli session-inspect --path 'PATH_TO_SESSION'
```

Default output is `captures` beside the CLI EXE. Each new session has lights,
darks, flats, bias, test and metadata directories. `session.json` is revision zero:
use session-inspect for the latest committed state. The sidecar preserves exact
requested/applied/reported settings. A missing observation is not a successful match.

Normal mode records/prints differences; strict mode also stops after saving the
nonconforming frame for investigation. Numeric tolerances are 1% (ISO minimum 2),
focus 50 millidiopters; discrete modes and crop compare exactly. These are software
thresholds, not optical sharpness guarantees. The current Pixel forces RAW hot-pixel
and shading high_quality in the tested configuration, so strict off requests fail.

## Complete portable request, including JPEG

```powershell
& $cli request-template --camera 0 --stream 4032x3024:Jpeg --out request.json
# Edit explicit settings in request.json, using values announced by capabilities.
& $cli execute --request request.json --out capture.jpg
```

This path returns the original payload plus `capture.jpg.json` containing the
full camera metadata. It uses Reject validation and refuses overwrites. It is a
single diagnostic capture, not a scientific session/sequence; a write failure is
reported and any partial output must be inspected, not silently reused. AWB auto
is possible here via `{"Mode":"auto"}`; scientific RAW sequences still require
fixed WB and locked focus. AE/ISO auto and physical routing are not implemented
by the current adapter. Arbitrary native/vendor request keys are not accepted.

Exposure is integration time, not command wall time. Camera pipeline startup,
readout, RAW/DNG encoding, transfer, durability and the final preview all cost
additional time. No image is resized by the PC RAW writer; selecting a smaller
HAL stream or an explicit crop is a user request, not a silent fallback.

## Repeatable verification

`doc/cli-regression.ps1 -Long` runs real positive and negative camera tests, the
maximum exposure, 30x1s sequence and every resulting session's checksum scan.
It generates a new CLI_AUDIT directory and results.json. See root test.md/report.md
for dated observations and limitations; no claim of universal Camera2 support is made.
