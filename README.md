# DeepskyEyes

**Deep-sky astronomical imaging and remote camera-control system for Google Pixel devices**

> A precision-oriented astrophotography control platform that turns a Google Pixel into a remotely controlled astronomical imaging instrument, with Android/Camera2 providing direct camera access and a native Rust desktop application providing control, preview, acquisition, sequencing, storage, diagnostics and future image-processing capabilities.

---

## doc/

puts mds docs file here in /doc/"".md

## 1. Project identity

**Project name:** `DeepskyEyes`

**Primary target device:** Google Pixel 8 Pro

**Android side:** Kotlin

**Desktop side:** Rust

**Desktop UI:** GPUI

**Primary use case:**

* deep-sky astrophotography;
* long-exposure acquisition;
* hundreds of individual exposures;
* RAW acquisition;
* fully manual camera control;
* remote focus;
* controlled white balance;
* deterministic exposure;
* repeatable capture sequences;
* live preview;
* capture metadata;
* calibration-frame acquisition;
* robust file management;
* eventual integration with astronomical image-processing workflows.

Example target session:

```text
Exposure:          15 s
Frames:            300
Total integration: 4500 s
                   75 min
ISO:               manually selected
Focus:             manually selected
WB:                manually selected
Output:            RAW/DNG
Control:           desktop
Preview:           desktop
Storage:           desktop
```

---

# 2. Core philosophy

DeepskyEyes must be designed around one principle:

> **Never assume that a camera capability exists. Discover it, validate it, expose it, and record what the hardware actually did.**

The Android Camera2 API defines a large set of camera controls and metadata, but the concrete capabilities are device- and camera-dependent.

Therefore:

```text
Android Camera2 API
        ↓
CameraCharacteristics
        ↓
Capability discovery
        ↓
Device-specific capability model
        ↓
Rust desktop representation
        ↓
GUI controls generated/validated from capabilities
```

We must never hard-code assumptions such as:

```text
"Pixel 8 Pro = 15 s RAW"
"main camera = 4080×3072"
"ISO = 50..6400"
"focus distance = 0..∞"
```

until those facts have been measured and/or retrieved from the actual device.

`CameraCharacteristics` exposes the properties of a `CameraDevice`, while `StreamConfigurationMap` is the authoritative source for supported output formats and sizes.

---

# 3. High-level architecture

```text
                         USB
                          │
                          │
                ┌─────────▼─────────┐
                │    Pixel 8 Pro     │
                │                    │
                │ Android            │
                │                    │
                │ ┌────────────────┐ │
                │ │ Kotlin         │ │
                │ │                │ │
                │ │ Camera2        │ │
                │ │ CameraManager  │ │
                │ │ CameraDevice   │ │
                │ │ CaptureSession │ │
                │ │ ImageReader    │ │
                │ │ DngCreator     │ │
                │ └───────┬────────┘ │
                │         │          │
                │ ┌───────▼────────┐ │
                │ │ Transport      │ │
                │ │ Protocol       │ │
                │ └────────────────┘ │
                └─────────┬──────────┘
                          │
                          │
                          ▼
                ┌─────────────────────┐
                │       PC            │
                │                     │
                │ Rust                │
                │                     │
                │ ┌─────────────────┐ │
                │ │ Transport       │ │
                │ ├─────────────────┤ │
                │ │ Protocol        │ │
                │ ├─────────────────┤ │
                │ │ Camera Client   │ │
                │ ├─────────────────┤ │
                │ │ Acquisition     │ │
                │ ├─────────────────┤ │
                │ │ Sequencer       │ │
                │ ├─────────────────┤ │
                │ │ Preview         │ │
                │ ├─────────────────┤ │
                │ │ RAW/DNG Storage │ │
                │ ├─────────────────┤ │
                │ │ Metadata        │ │
                │ ├─────────────────┤ │
                │ │ Diagnostics     │ │
                │ └────────┬────────┘ │
                │          │          │
                │ ┌────────▼────────┐ │
                │ │ GPUI             │ │
                │ │ Desktop UI       │ │
                │ └─────────────────┘ │
                └─────────────────────┘
```

---

# 4. Separation of responsibilities

## Android

Android is the **camera adapter**.

It is responsible for:

* discovering cameras;
* querying camera capabilities;
* opening Camera2 devices;
* creating capture sessions;
* creating output surfaces;
* constructing capture requests;
* applying supported controls;
* receiving `CaptureResult`;
* receiving RAW/YUV/JPEG buffers;
* maintaining camera state;
* generating DNG where appropriate;
* communicating camera state and results to the PC.

Android should **not** contain:

* the main astrophotography UI;
* the main sequencing logic;
* the main session database;
* the main image-processing pipeline;
* desktop-specific storage logic.

---

# 5. Desktop

The desktop application is the **mission control system**.

Rust owns:

* device connection;
* capability representation;
* command generation;
* sequencing;
* acquisition state;
* preview;
* storage;
* metadata;
* logging;
* diagnostics;
* session management;
* calibration workflow;
* UI state;
* future processing pipeline.

The desktop application should remain useful even if the Android implementation is eventually replaced by another camera backend.

---

# 6. Repository structure

Recommended monorepo:

```text
DeepskyEyes/
│
├── README.md
├── LICENSE
├── CONTRIBUTING.md
├── SECURITY.md
├── CHANGELOG.md
│
├── docs/
│   ├── architecture/
│   │   ├── overview.md
│   │   ├── android.md
│   │   ├── desktop.md
│   │   ├── protocol.md
│   │   ├── acquisition.md
│   │   ├── raw-pipeline.md
│   │   └── failure-model.md
│   │
│   ├── camera/
│   │   ├── camera2-capabilities.md
│   │   ├── pixel-8-pro.md
│   │   ├── raw.md
│   │   ├── exposure.md
│   │   ├── focus.md
│   │   └── white-balance.md
│   │
│   ├── astrophotography/
│   │   ├── deep-sky.md
│   │   ├── calibration.md
│   │   ├── stacking.md
│   │   └── sequencing.md
│   │
│   ├── research/
│   │   ├── pixel-capability-matrix.md
│   │   ├── usb-options.md
│   │   ├── thermal-testing.md
│   │   └── long-exposure-testing.md
│   │
│   └── protocol/
│       ├── protocol-v1.md
│       ├── commands.md
│       ├── errors.md
│       └── framing.md
│
├── android/
│   ├── settings.gradle.kts
│   ├── build.gradle.kts
│   ├── gradle.properties
│   ├── app/
│   │   └── ...
│   │
│   └── camera-core/
│       └── ...
│
├── desktop/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   │
│   ├── crates/
│   │   ├── deepsky-app/
│   │   ├── deepsky-ui/
│   │   ├── deepsky-protocol/
│   │   ├── deepsky-transport/
│   │   ├── deepsky-camera/
│   │   ├── deepsky-acquisition/
│   │   ├── deepsky-sequencer/
│   │   ├── deepsky-preview/
│   │   ├── deepsky-raw/
│   │   ├── deepsky-metadata/
│   │   ├── deepsky-session/
│   │   ├── deepsky-diagnostics/
│   │   └── deepsky-testkit/
│   │
│   └── resources/
│
├── protocol/
│   ├── schema/
│   ├── examples/
│   └── README.md
│
├── tools/
│   ├── capability-dump/
│   ├── protocol-inspector/
│   └── session-inspector/
│
└── tests/
    ├── protocol/
    ├── capability/
    ├── acquisition/
    └── integration/
```

---

# 7. Android project architecture

Recommended Android modules:

```text
android/
│
├── app/
│
├── camera-core/
│
├── camera-capabilities/
│
├── camera-control/
│
├── camera-acquisition/
│
├── raw-dng/
│
├── transport/
│
├── protocol/
│
├── diagnostics/
│
└── test-support/
```

## `app`

Only application wiring.

Responsibilities:

* Android lifecycle;
* permissions;
* foreground/background behavior;
* service startup;
* dependency injection;
* configuration;
* minimal diagnostic UI.

The actual camera implementation must live elsewhere.

---

# 8. `camera-core`

Responsible for:

* `CameraManager`;
* `CameraDevice`;
* `CameraCaptureSession`;
* `CaptureRequest`;
* `CaptureResult`;
* `ImageReader`;
* `Surface`;
* callbacks;
* camera lifecycle.

The module must expose a clean domain API rather than leaking Android classes throughout the project.

Bad:

```kotlin
desktopClient.send(cameraDevice)
```

Good:

```kotlin
cameraController.capture(request)
```

---

# 9. Capability discovery

At startup the Android side must enumerate:

```text
Camera IDs
Lens facing
Physical camera IDs
Logical camera status
Hardware level
Available capabilities
Sensor information
Exposure range
Sensitivity range
Frame duration range
Focus capabilities
AF modes
AE modes
AWB modes
Color correction
OIS
Flash
Zoom/crop
RAW support
RAW sizes
JPEG sizes
YUV sizes
Maximum-resolution configurations
Stream durations
Stall durations
Recommended configurations
```

Camera2 explicitly exposes RAW capability through `REQUEST_AVAILABLE_CAPABILITIES_RAW`. A device with that capability supports `RAW_SENSOR` and provides metadata needed to interpret the raw data.

---

# 10. Capability matrix

The application must generate a structured capability document.

Example:

```json
{
  "camera_id": "0",
  "lens_facing": "BACK",
  "hardware_level": "FULL",
  "capabilities": {
    "raw": true,
    "manual_sensor": true,
    "manual_post_processing": true,
    "logical_multi_camera": true
  },
  "sensor": {
    "active_array": "...",
    "pixel_array": "...",
    "orientation": 90
  },
  "exposure": {
    "supported": true,
    "min_ns": 1000,
    "max_ns": 15000000000
  },
  "sensitivity": {
    "supported": true,
    "min": 50,
    "max": 6400
  }
}
```

The actual values must come from the device.

---

# 11. Resolution discovery

Never hard-code resolution.

Query:

```text
SCALER_STREAM_CONFIGURATION_MAP
SCALER_STREAM_CONFIGURATION_MAP_MAXIMUM_RESOLUTION
```

and enumerate:

```text
format
width
height
min frame duration
stall duration
```

The `StreamConfigurationMap` is explicitly documented as the authoritative list of supported output formats and sizes.

For each candidate configuration we must test:

```text
RAW_SENSOR
YUV_420_888
JPEG
PRIVATE / preview-compatible format
```

where applicable.

---

# 12. RAW acquisition

RAW is a first-class requirement.

Preferred path:

```text
Camera2
   ↓
ImageFormat.RAW_SENSOR
   ↓
ImageReader
   ↓
CaptureResult
   ↓
DNG
```

Android provides `DngCreator` specifically for writing raw pixel data to DNG using RAW_SENSOR buffers and `CaptureResult` metadata.

However, DeepskyEyes must investigate whether:

1. DNG should be generated on Android;
2. RAW_SENSOR should instead be transported directly to the PC;
3. both should be supported;
4. the Android-produced DNG preserves all metadata needed for scientific/astronomical processing.

The preferred architecture is to preserve the **least processed representation possible**.

---

# 13. RAW transfer strategy

Three modes should be investigated.

## Mode A — Android generates DNG

```text
RAW_SENSOR
   ↓
DngCreator
   ↓
DNG
   ↓
USB
   ↓
PC
```

Advantages:

* easy interoperability;
* Android supplies DNG metadata;
* PC receives immediately usable files.

Disadvantages:

* larger transfer;
* DNG creation cost on phone;
* less control over exact raw representation.

---

## Mode B — transfer RAW_SENSOR directly

```text
RAW_SENSOR
   ↓
transport
   ↓
PC
   ↓
Rust RAW writer
```

Advantages:

* maximum control;
* potentially more efficient;
* scientific processing can happen entirely on PC.

Disadvantages:

* requires complete interpretation of RAW metadata;
* Android-side sensor metadata must be transported;
* DNG generation moves to desktop.

---

## Mode C — both

For diagnostics:

```text
RAW_SENSOR
 ├──► raw packet
 └──► DNG
```

This may be valuable during development.

---

# 14. Exposure control

The project must support, where advertised by the camera:

```text
SENSOR_EXPOSURE_TIME
SENSOR_SENSITIVITY
SENSOR_FRAME_DURATION
```

The desktop should distinguish:

```text
requested value
actual applied value
actual result value
```

Example:

```text
Requested:
exposure = 15.000000 s

Applied:
exposure = 15.000000 s

Result:
exposure = 15.000000 s
```

If the camera clamps the request:

```text
Requested: 20.000 s
Applied:   15.000 s
```

the application must report this as a **clamped request**, not silently pretend that 20 seconds happened.

---

# 15. ISO / sensitivity

The UI should display:

```text
ISO requested
ISO applied
ISO reported
```

Camera2 sensitivity is sensor sensitivity, not necessarily equivalent to the marketing ISO semantics of a traditional camera.

Therefore documentation must avoid saying:

> "ISO 800 means exactly the same thing as ISO 800 on a DSLR."

Instead:

> "Sensor sensitivity requested through Camera2."

---

# 16. Focus

Potential controls:

```text
LENS_FOCUS_DISTANCE
CONTROL_AF_MODE
CONTROL_AF_TRIGGER
CONTROL_AF_REGIONS
```

The project must support:

```text
AF
AF lock
manual focus
focus distance
```

when supported.

For astrophotography, the important mode is:

```text
manual focus
+
focus locked
+
no AF activity during sequence
```

The system must prevent accidental autofocus during a 75-minute acquisition.

---

# 17. Focus workflow

Suggested workflow:

```text
LIVE PREVIEW
     ↓
AF / manual focus
     ↓
fine adjustment
     ↓
focus confirmation
     ↓
LOCK FOCUS
     ↓
START SEQUENCE
```

The sequencer should reject or warn about a sequence if focus is not locked.

---

# 18. White balance

White balance must be modeled carefully.

Camera2 can expose low-level color correction controls such as:

```text
COLOR_CORRECTION_GAINS
COLOR_CORRECTION_TRANSFORM
```

and newer APIs expose additional color-temperature/tint-related controls where supported.

The desktop abstraction should therefore distinguish:

```text
WB_AUTO
WB_PRESET
WB_TEMPERATURE
WB_TINT
WB_MANUAL_GAINS
```

depending on actual capability.

A Kelvin slider is a **user-facing abstraction**, not something we should assume maps directly to a native Camera2 integer.

---

# 19. Processing controls

Investigate and expose, where supported:

```text
EDGE_MODE
NOISE_REDUCTION_MODE
HOT_PIXEL_MODE
SHADING_MODE
TONEMAP_MODE
COLOR_CORRECTION_MODE
```

For astrophotography, the desired philosophy is generally:

```text
minimum computational manipulation
```

but the actual behavior must be measured.

We must determine whether:

* noise reduction can be completely disabled;
* hot-pixel correction can be disabled;
* sharpening can be disabled;
* lens shading correction can be disabled;
* tone mapping can be disabled;
* any hidden vendor processing remains.

---

# 20. Preview architecture

Preview and RAW acquisition are separate pipelines.

```text
                 Camera
                   │
             ┌─────┴─────┐
             │            │
           RAW          Preview
             │            │
             │          YUV/etc.
             │            │
             ▼            ▼
            Disk        Transport
                          │
                          ▼
                        Rust
                          │
                    Preview renderer
                          │
                          ▼
                         GPUI
```

The preview must never force RAW resolution or RAW timing to become unsuitable.

---

# 21. Preview requirements

The preview subsystem should eventually support:

* fit-to-window;
* 100% pixel view;
* zoom;
* pan;
* histogram;
* RGB histogram;
* luminance histogram;
* clipping indicators;
* focus peaking;
* false-color focus aid;
* exposure warning;
* center crop;
* crosshair;
* grid;
* star detection;
* statistics;
* frame number;
* exposure metadata;
* temperature;
* dropped-frame indicator.

---

# 22. GPUI

The desktop UI will use **GPUI**.

GPUI is a GPU-accelerated Rust UI framework and is used by Zed. It is currently pre-1.0, so the project must pin the GPUI version and isolate GPUI-specific code behind the `deepsky-ui` crate.

The UI layer must never own:

* USB logic;
* camera protocol;
* acquisition;
* RAW decoding;
* session persistence.

It should consume application state.

---

# 23. Important GPUI platform decision

GPUI support evolves quickly.

Current upstream code contains Windows-specific GPUI/platform components, and Zed itself is now supported on Windows.

However, GPUI remains pre-1.0 and platform APIs may change.

Therefore:

```text
deepsky-ui
    ↓
GPUI
```

must be treated as a replaceable presentation layer.

The rest of the application must not depend directly on GPUI types.

---

# 24. Rust workspace

Recommended:

```text
desktop/
├── Cargo.toml
└── crates/
    ├── deepsky-app
    ├── deepsky-ui
    ├── deepsky-protocol
    ├── deepsky-transport
    ├── deepsky-camera
    ├── deepsky-acquisition
    ├── deepsky-sequencer
    ├── deepsky-preview
    ├── deepsky-raw
    ├── deepsky-metadata
    ├── deepsky-session
    ├── deepsky-diagnostics
    └── deepsky-testkit
```

---

# 25. `deepsky-protocol`

Contains only:

* commands;
* responses;
* events;
* error codes;
* wire types;
* protocol version;
* serialization;
* framing definitions.

It must not know anything about GPUI or Camera2.

---

# 26. `deepsky-transport`

Transport abstraction:

```rust
trait Transport {
    async fn connect(&mut self) -> Result<()>;
    async fn send(&mut self, data: &[u8]) -> Result<()>;
    async fn receive(&mut self) -> Result<Vec<u8>>;
    async fn disconnect(&mut self) -> Result<()>;
}
```

Possible implementations:

```text
AdbTransport
UsbAccessoryTransport
TcpTransport
MockTransport
```

The exact production transport is a research item and must not be prematurely locked.

---

# 27. Transport strategy

Development:

```text
PC
 ↓
ADB
 ↓
Android socket/service
```

Production candidates:

1. ADB transport;
2. Android Open Accessory;
3. USB accessory/host protocol;
4. USB serial-like abstraction if hardware allows;
5. network transport as fallback.

Android officially supports USB host and accessory modes, but the actual behavior depends on hardware/device support.

The protocol must therefore be transport-independent.

---

# 28. Why transport independence matters

We do not want:

```text
Camera code
  ↓
ADB
```

We want:

```text
Camera code
  ↓
Protocol
  ↓
Transport
```

Then:

```text
ADB
USB
TCP
Mock
```

can all carry the same protocol.

---

# 29. Protocol design

The protocol should be:

* versioned;
* framed;
* length-delimited;
* checksummed where appropriate;
* request/response capable;
* event capable;
* extensible;
* backward-compatible where practical.

Logical message:

```text
HEADER
  protocol_version
  message_type
  flags
  request_id
  payload_length
  sequence_number

PAYLOAD

OPTIONAL CHECKSUM
```

---

# 30. Commands

Initial command families:

```text
SYSTEM
  HELLO
  GET_VERSION
  GET_STATUS
  PING
  GET_TIME

DEVICE
  GET_DEVICE_INFO
  GET_CAMERAS
  GET_CAMERA_CAPABILITIES

CAMERA
  OPEN_CAMERA
  CLOSE_CAMERA
  CONFIGURE_CAMERA
  GET_CAMERA_STATE

CONTROL
  SET_EXPOSURE
  SET_SENSITIVITY
  SET_FRAME_DURATION
  SET_FOCUS
  SET_AF_MODE
  SET_AWB
  SET_COLOR_GAINS
  SET_ZOOM
  SET_CROP
  SET_PROCESSING_MODE

PREVIEW
  START_PREVIEW
  STOP_PREVIEW
  SET_PREVIEW_CONFIGURATION

CAPTURE
  CAPTURE
  CANCEL_CAPTURE

SEQUENCE
  CREATE_SEQUENCE
  START_SEQUENCE
  PAUSE_SEQUENCE
  RESUME_SEQUENCE
  STOP_SEQUENCE

STORAGE
  GET_FRAME
  DELETE_FRAME
  GET_METADATA

DIAGNOSTICS
  GET_LOG
  GET_METRICS
  GET_THERMAL_STATUS
  GET_CAMERA_RESULT
```

---

# 31. Events

Android must be able to asynchronously emit:

```text
DEVICE_CONNECTED
DEVICE_DISCONNECTED

CAMERA_OPENED
CAMERA_CLOSED
CAMERA_ERROR

CAPTURE_STARTED
CAPTURE_COMPLETED
CAPTURE_FAILED

FRAME_AVAILABLE
FRAME_TRANSFER_STARTED
FRAME_TRANSFER_COMPLETED

TEMPERATURE_CHANGED
THERMAL_WARNING

SEQUENCE_STARTED
SEQUENCE_PROGRESS
SEQUENCE_PAUSED
SEQUENCE_COMPLETED
SEQUENCE_FAILED
```

---

# 32. Request IDs

Every command requiring a response receives:

```text
request_id
```

Example:

```text
SET_EXPOSURE
request_id = 912
```

Response:

```text
request_id = 912
status = OK
requested = 15s
applied = 15s
```

---

# 33. Errors

Errors must be machine-readable.

Example:

```text
UNSUPPORTED_PARAMETER
INVALID_VALUE
VALUE_CLAMPED
CAMERA_BUSY
CAMERA_DISCONNECTED
SESSION_CONFIGURATION_FAILED
CAPTURE_FAILED
RAW_NOT_SUPPORTED
FORMAT_NOT_SUPPORTED
SIZE_NOT_SUPPORTED
TRANSPORT_ERROR
TIMEOUT
THERMAL_LIMIT
STORAGE_ERROR
PROTOCOL_ERROR
```

Human-readable messages are supplemental.

---

# 34. Capability-driven UI

The UI must not blindly show every possible control.

Instead:

```text
Device capabilities
        ↓
Capability model
        ↓
UI availability
```

Example:

```text
Manual focus:
[✓ supported]

Manual exposure:
[✓ supported]

RAW:
[✓ supported]

Manual white balance:
[✗ unsupported]
```

Unsupported controls should be unavailable rather than silently failing.

---

# 35. Sequencer

The sequencer is one of the most important components.

Example:

```text
Sequence:
    frames = 300
    exposure = 15s
    sensitivity = 800
    focus = locked
    WB = fixed
    delay = 1s
```

Sequence state machine:

```text
IDLE
 ↓
VALIDATING
 ↓
PREPARING
 ↓
RUNNING
 ↓
PAUSED
 ↓
RUNNING
 ↓
COMPLETING
 ↓
COMPLETED
```

Error transitions:

```text
RUNNING
  ↓
ERROR
  ↓
RECOVERING
  ↓
RUNNING
```

or:

```text
RUNNING
  ↓
FATAL_ERROR
```

---

# 36. Sequence validation

Before the first exposure:

```text
camera connected?
camera open?
RAW supported?
format supported?
resolution supported?
exposure supported?
ISO supported?
focus locked?
storage available?
thermal state acceptable?
disk space sufficient?
transport healthy?
clock valid?
```

Only then:

```text
START SEQUENCE
```

---

# 37. 300-frame acquisition

Target:

```text
300 × 15 seconds
```

The sequence must maintain:

```text
frame index
capture timestamp
requested exposure
actual exposure
requested ISO
actual ISO
focus state
WB state
camera temperature
device temperature
transfer status
file path
checksum
```

---

# 38. Timing

The system must distinguish:

```text
exposure time
frame duration
readout time
transfer time
processing time
inter-frame delay
```

Do not assume:

```text
15s exposure = 15s frame period
```

The actual camera timing must be measured.

`StreamConfigurationMap` provides minimum frame durations and stall durations that are relevant to understanding achievable timing for a particular format/size.

---

# 39. Storage architecture

Never write directly from the transport callback into a GUI-controlled file.

Use:

```text
Transport
   ↓
Acquisition queue
   ↓
Frame manager
   ↓
Disk writer
```

The disk writer must be asynchronous.

---

# 40. File naming

Example:

```text
M42/
├── lights/
│   ├── M42_2026-09-10T214512Z_L_0001.dng
│   ├── M42_2026-09-10T214528Z_L_0002.dng
│   └── ...
│
├── darks/
├── flats/
├── bias/
├── metadata/
└── session.json
```

Naming must be deterministic.

---

# 41. Session format

Every acquisition session gets a machine-readable manifest.

Example:

```json
{
  "project": "M42",
  "device": "Pixel 8 Pro",
  "camera_id": "0",
  "started_at": "...",
  "exposure_s": 15.0,
  "frames_requested": 300,
  "frames_completed": 300,
  "raw": true,
  "resolution": "DEVICE_REPORTED",
  "sensitivity": 800
}
```

The manifest must also contain:

* application version;
* protocol version;
* Android version;
* camera capability snapshot;
* capture configuration;
* actual results;
* file hashes;
* warnings;
* errors;
* thermal events.

---

# 42. Integrity

Every frame should have an integrity mechanism.

At minimum:

```text
filename
size
timestamp
hash
```

Potentially:

```text
SHA-256
```

The session manifest should record the checksum.

---

# 43. Crash recovery

If the PC crashes after frame 137:

```text
session state = RUNNING
expected = 300
completed = 137
```

On restart:

```text
scan session
verify existing files
reconstruct completed frames
mark missing frames
offer resume
```

The application must never overwrite existing frames accidentally.

---

# 44. Android-side crash recovery

If Android dies:

```text
PC detects transport loss
        ↓
sequence PAUSED/FAILED
        ↓
attempt reconnect
        ↓
query camera state
        ↓
verify configuration
        ↓
resume only if safe
```

Never blindly resume an astrophotography sequence after a camera reconnect.

---

# 45. Thermal management

Thermal behavior is a major research topic.

The application should monitor, where possible:

```text
battery temperature
device thermal status
CPU load
camera state
capture failures
transport failures
```

The UI should show:

```text
NORMAL
WARM
HOT
THROTTLING
CRITICAL
```

The sequence must have a thermal policy.

---

# 46. Power

Long sessions can last hours.

Investigate:

* USB power behavior;
* whether the Pixel charges while controlled;
* charge throttling;
* battery temperature;
* screen state;
* Doze;
* background restrictions;
* foreground service requirements;
* camera lifetime;
* USB power negotiation.

The Android app must be designed so the operating system does not suspend the camera service during acquisition.

---

# 47. Screen independence

The camera service must not depend on the phone screen being:

```text
ON
UNLOCKED
VISIBLE
```

This must be tested explicitly.

---

# 48. Android service

Likely architecture:

```text
MainActivity
     │
     ▼
CameraService
     │
     ├── CameraManager
     ├── CameraController
     ├── Acquisition
     └── TransportServer
```

The service should own the long-running camera operation.

---

# 49. Camera lifecycle

Explicit state machine:

```text
CLOSED
 ↓
OPENING
 ↓
OPEN
 ↓
CONFIGURING
 ↓
READY
 ↓
CAPTURING
 ↓
READY
```

Error states:

```text
ERROR
DISCONNECTED
RECOVERING
```

---

# 50. Logical vs physical cameras

The Pixel may expose logical multi-camera devices.

Camera2 provides `getPhysicalCameraIds()` for logical camera devices.

DeepskyEyes must distinguish:

```text
logical camera
physical camera
lens
sensor
```

We must identify exactly which physical sensor corresponds to:

```text
main
ultrawide
telephoto
front
```

and determine whether RAW/manual control is available independently.

---

# 51. Astrophotography-specific concern: computational photography

This is one of the most important investigations.

We must determine whether the Pixel applies:

* temporal noise reduction;
* spatial noise reduction;
* sharpening;
* HDR;
* multi-frame fusion;
* denoising;
* lens correction;
* pixel binning;
* remosaicing;
* tone mapping;
* automatic black-level processing;
* hot-pixel correction.

RAW capability does not automatically mean that the entire camera pipeline behaves like a dedicated astronomy camera.

---

# 52. RAW scientific validity

We must determine:

* Bayer pattern;
* bit depth;
* black level;
* white level;
* saturation level;
* analog/digital gain behavior;
* read noise;
* dark current;
* fixed-pattern noise;
* hot pixels;
* defective pixels;
* temperature dependency;
* lens shading;
* optical black pixels;
* sensor crop;
* binning;
* pixel mode.

The RAW capability documentation specifies that RAW_SENSOR is available and that DNG-related optional metadata is provided by the camera device when RAW capability exists.

That does **not** eliminate the need for empirical validation.

---

# 53. Deep-sky calibration

Future processing must support:

```text
LIGHT
DARK
FLAT
BIAS / OFFSET
```

depending on what the Pixel's sensor characteristics make useful.

Calibration frames must preserve:

```text
exposure
gain
temperature
focus
resolution
orientation
camera
```

A dark frame taken at:

```text
ISO 800
15 s
30 °C
```

must not be blindly applied to:

```text
ISO 1600
15 s
15 °C
```

---

# 54. Dark-frame library

Potential future feature:

```text
Dark Library

ISO:
  100
  200
  400
  800
  1600

Exposure:
  1s
  5s
  10s
  15s
  30s

Temperature:
  10°C
  15°C
  20°C
  25°C
  30°C
```

This should be empirically generated.

---

# 55. Focus stability

Deep-sky imaging is highly sensitive to focus.

Investigate:

* focus repeatability;
* temperature drift;
* autofocus repeatability;
* manual focus resolution;
* lens movement during capture;
* focus-distance semantics;
* whether focus changes between sessions;
* whether optical stabilization moves the optical path.

---

# 56. Optical stabilization

Determine:

```text
OIS available?
OIS controllable?
OIS active during RAW?
OIS active during preview?
OIS movement during long exposure?
```

For astrophotography this may be extremely important.

If OIS cannot be disabled, its impact must be measured.

---

# 57. Electronic stabilization

Disable any:

```text
EIS
video stabilization
dynamic crop
```

unless explicitly desired.

---

# 58. Exposure consistency

Run:

```text
100 × 15s
```

and compare metadata and pixel statistics.

Measure:

```text
mean
median
variance
black level
saturation
histogram
hot pixels
```

Look for frame-to-frame changes.

---

# 59. Timing consistency

Measure:

```text
requested exposure
actual exposure
frame start
frame end
inter-frame interval
```

for:

```text
10 frames
100 frames
300 frames
```

---

# 60. Transfer performance

Measure:

```text
RAW size
transfer speed
capture-to-PC latency
queue depth
dropped frames
USB disconnects
```

The system must be able to acquire continuously without the transport becoming the bottleneck.

---

# 61. Backpressure

The acquisition system must handle:

```text
camera faster than disk
disk faster than camera
USB faster than camera
USB slower than camera
```

with explicit policies.

Never allow unbounded memory growth.

---

# 62. Ring buffers

Recommended architecture:

```text
Camera producer
      ↓
bounded ring buffer
      ↓
┌─────┴──────────┐
│                │
Disk writer    Preview
```

When the buffer is full:

```text
BLOCK
```

or:

```text
DROP PREVIEW
```

but **never silently drop RAW science frames**.

---

# 63. Preview frame dropping

Preview is disposable.

RAW frames are not.

Therefore:

```text
RAW queue:
lossless

Preview queue:
lossy
```

If preview falls behind:

```text
drop old preview frame
display newest
```

---

# 64. UI layout

Initial desktop UI:

```text
┌──────────────────────────────────────────────────────┐
│ DeepskyEyes       Pixel 8 Pro       CONNECTED       │
├──────────────────────────────────────┬───────────────┤
│                                      │ CAMERA        │
│                                      │               │
│                                      │ Sensor        │
│          LIVE PREVIEW                │ Lens          │
│                                      │ RAW           │
│                                      │ Resolution    │
│                                      │               │
│                                      │ Exposure      │
│                                      │ ISO           │
│                                      │ Focus         │
│                                      │ WB            │
│                                      │ Zoom          │
│                                      │               │
│                                      │ [LOCK]        │
├──────────────────────────────────────┴───────────────┤
│ SEQUENCE                                               │
│ Frames: 023 / 300   Exposure: 15.000s                 │
│ Progress: ███████░░░░░                                 │
│ [START] [PAUSE] [STOP]                                │
├──────────────────────────────────────────────────────┤
│ Histogram / Statistics / Diagnostics                    │
└──────────────────────────────────────────────────────┘
```

---

# 65. UI pages

Recommended:

```text
Dashboard
Camera
Preview
Sequence
Calibration
Sessions
Diagnostics
Settings
```

---

# 66. Camera page

Show:

```text
camera ID
physical camera ID
lens
sensor
RAW
resolution
hardware level
capabilities
```

---

# 67. Sequence page

Allow:

```text
number of frames
exposure
sensitivity
focus
WB
delay
destination
file naming
dark/flat/light type
```

Advanced:

```text
settling time
retries
temperature threshold
storage threshold
resume behavior
```

---

# 68. Diagnostics page

Must expose raw information rather than hiding it.

Example:

```text
Camera state: READY

Exposure:
requested 15,000,000 ns
applied    15,000,000 ns

Sensitivity:
requested 800
applied    800

Frame duration:
...

RAW:
supported

Output:
RAW_SENSOR
4080×3072

Transport:
connected
RX 82 MB/s
TX 0.3 MB/s

Dropped RAW:
0

Dropped preview:
3
```

---

# 69. Logging

Use structured logs.

Levels:

```text
TRACE
DEBUG
INFO
WARN
ERROR
```

Every capture should be traceable by:

```text
session_id
sequence_id
frame_id
request_id
```

---

# 70. Deterministic logs

Example:

```text
2026-09-10T21:45:12.123Z
session=abc
sequence=42
frame=17
capture_started
exposure_ns=15000000000
sensitivity=800
```

---

# 71. Clock synchronization

The system has two clocks:

```text
Android clock
PC clock
```

Do not assume they are synchronized.

Protocol should support:

```text
PING
timestamp request
timestamp response
clock offset estimation
```

For astrophotography, eventually investigate:

```text
UTC
GPS
NTP
device clock accuracy
```

---

# 72. Metadata

Every frame should preserve:

```text
timestamp
camera ID
physical camera ID
resolution
format
exposure
sensitivity
frame duration
focus
WB
crop
zoom
OIS state
processing modes
temperature
software version
protocol version
```

where available.

---

# 73. Capability snapshots

At the start of every session:

```text
capabilities.json
```

should be stored.

This ensures that future analysis knows exactly what the device advertised at acquisition time.

---

# 74. Configuration versioning

Every saved configuration gets a schema version:

```text
config_version = 1
```

Never rely on unversioned JSON.

---

# 75. Rust module dependency rules

Dependency direction:

```text
deepsky-app
     │
     ├── deepsky-ui
     ├── deepsky-camera
     ├── deepsky-acquisition
     ├── deepsky-sequencer
     ├── deepsky-session
     └── diagnostics

deepsky-ui
     ↓
application interfaces

deepsky-camera
     ↓
deepsky-protocol
     ↓
deepsky-transport
```

No lower-level module may depend on GPUI.

---

# 76. Android dependency rules

```text
app
 ↓
service
 ↓
camera-core
 ↓
camera-control
 ↓
acquisition
 ↓
transport
```

Protocol models must be separated from Android camera objects.

---

# 77. Testing strategy

Four layers:

```text
Unit
Integration
Hardware
Long-duration
```

---

# 78. Unit tests

Test:

* protocol encoding;
* protocol decoding;
* framing;
* checksums;
* capability parsing;
* range validation;
* sequence state machine;
* file naming;
* metadata;
* configuration migration.

---

# 79. Integration tests

Use mock transport:

```text
Rust
 ↕
Mock Android
```

Test:

```text
connect
discover
configure
capture
transfer
sequence
failure
reconnect
```

---

# 80. Hardware tests

Real Pixel:

```text
camera discovery
RAW
manual exposure
ISO
focus
WB
resolution
preview
transfer
```

---

# 81. Long-duration tests

Mandatory:

```text
1 hour
2 hours
4 hours
```

Measure:

```text
memory
temperature
battery
USB stability
capture failures
frame drops
storage
timing drift
```

---

# 82. Stress test

Target:

```text
300 × 15s
```

without interruption.

Then:

```text
500 × 15s
1000 × 15s
```

if practical.

---

# 83. Failure injection

Simulate:

```text
USB disconnect
Android process crash
camera disconnect
disk full
disk failure
transport timeout
malformed packet
camera busy
invalid exposure
invalid ISO
unsupported resolution
thermal warning
PC sleep
Android screen lock
```

---

# 84. Security

The protocol should not blindly accept arbitrary commands from any network peer.

For USB-only operation this risk is smaller, but still define:

```text
authentication
session handshake
protocol version
device identity
```

Do not expose a production camera-control endpoint on the LAN without authentication.

---

# 85. Development protocol

Initial development should favor:

```text
ADB
```

because it simplifies debugging.

But ADB must not become a hard dependency of the camera architecture.

Production transport remains an explicit research decision.

---

# 86. CLI

Before GUI completion, provide:

```bash
deepsky-eyes discover
deepsky-eyes capabilities
deepsky-eyes connect
deepsky-eyes camera list
deepsky-eyes camera info 0
deepsky-eyes set exposure 15s
deepsky-eyes set iso 800
deepsky-eyes set focus ...
deepsky-eyes capture
deepsky-eyes sequence --frames 300 --exposure 15s
```

The CLI becomes the first integration test client.

---

# 87. GUI must not be required for automation

The application must support:

```text
CLI
GUI
future API
```

using the same core.

---

# 88. API abstraction

Core interface:

```text
CameraBackend
Transport
AcquisitionBackend
StorageBackend
PreviewBackend
```

The Pixel implementation is:

```text
PixelCameraBackend
```

Future possibilities:

```text
DSLRBackend
MirrorlessBackend
AstroCameraBackend
SimulatorBackend
```

---

# 89. Simulator

Create a simulated Android camera.

It can generate:

```text
fake RAW
fake metadata
fake timing
fake failures
```

This allows desktop development without the phone.

---

# 90. Pixel capability research

Before declaring the camera backend complete, determine exactly:

```text
camera IDs
physical IDs
logical IDs
sensor sizes
pixel array sizes
active array sizes
RAW sizes
YUV sizes
JPEG sizes
maximum-resolution mode
manual sensor support
manual post-processing
RAW support
exposure range
ISO range
frame duration
focus range
AF modes
AE modes
AWB modes
color correction
OIS
flash
zoom
crop
noise reduction
edge
hot pixel
shading
tonemap
```

---

# 91. Critical research checklist

## Camera2

* [ ] `MANUAL_SENSOR`
* [ ] `MANUAL_POST_PROCESSING`
* [ ] `RAW`
* [ ] `READ_SENSOR_SETTINGS`
* [ ] logical multi-camera
* [ ] physical camera IDs
* [ ] maximum resolution mode
* [ ] recommended RAW stream
* [ ] RAW stream dimensions
* [ ] minimum frame duration
* [ ] stall duration

---

# 92. Exposure research

* [ ] maximum exposure on main camera
* [ ] maximum exposure on ultrawide
* [ ] maximum exposure on telephoto
* [ ] minimum exposure
* [ ] sensitivity range
* [ ] sensitivity step behavior
* [ ] exposure precision
* [ ] actual-vs-requested exposure
* [ ] frame duration
* [ ] readout time
* [ ] exposure stability across 300 frames

---

# 93. RAW research

* [ ] RAW_SENSOR available?
* [ ] exact dimensions
* [ ] bit depth
* [ ] Bayer pattern
* [ ] black level
* [ ] white level
* [ ] saturation level
* [ ] metadata completeness
* [ ] DNG correctness
* [ ] direct RAW transfer feasibility
* [ ] Android DNG vs desktop DNG comparison

---

# 94. Computational photography research

* [ ] RAW denoising?
* [ ] hot pixel correction?
* [ ] lens shading?
* [ ] sharpening?
* [ ] HDR?
* [ ] multi-frame fusion?
* [ ] hidden processing?
* [ ] black-level manipulation?
* [ ] automatic calibration?
* [ ] vendor-specific behavior?

---

# 95. Astrophotography research

* [ ] read noise
* [ ] dark current
* [ ] thermal noise
* [ ] fixed-pattern noise
* [ ] hot pixels
* [ ] amp glow
* [ ] sensor linearity
* [ ] gain linearity
* [ ] saturation
* [ ] dynamic range
* [ ] ISO invariance
* [ ] temperature dependence
* [ ] long-exposure behavior

---

# 96. Focus research

* [ ] manual focus resolution
* [ ] focus repeatability
* [ ] infinity focus position
* [ ] temperature drift
* [ ] autofocus reliability on stars
* [ ] focus lock behavior
* [ ] focus changes during long sequences
* [ ] lens movement
* [ ] OIS interaction

---

# 97. Thermal research

* [ ] temperature at idle
* [ ] temperature after 30 min
* [ ] temperature after 60 min
* [ ] temperature after 120 min
* [ ] temperature after 300 frames
* [ ] thermal throttling
* [ ] camera shutdown
* [ ] exposure instability
* [ ] noise increase with temperature

---

# 98. USB research

* [ ] charging while camera is active
* [ ] USB bandwidth
* [ ] sustained transfer speed
* [ ] cable stability
* [ ] USB mode
* [ ] ADB behavior
* [ ] Android USB accessory
* [ ] USB host/accessory limitations
* [ ] production transport
* [ ] disconnect/reconnect behavior

Android officially provides USB host and accessory APIs, but actual support depends on the hardware and mode being used.

---

# 99. Power research

* [ ] external power behavior
* [ ] battery drain
* [ ] charging temperature
* [ ] charging throttling
* [ ] screen-off behavior
* [ ] Doze behavior
* [ ] foreground service requirements
* [ ] background execution
* [ ] camera persistence

---

# 100. GPUI research

* [ ] supported OS targets
* [ ] Windows backend stability
* [ ] Linux backend
* [ ] macOS backend
* [ ] GPU requirements
* [ ] image rendering
* [ ] texture upload
* [ ] high-frequency preview
* [ ] large image display
* [ ] zoom/pan performance
* [ ] histogram rendering
* [ ] GPUI version pinning
* [ ] breaking API policy

GPUI is currently pre-1.0, so version pinning is mandatory.

---

# 101. Preview research

* [ ] YUV format
* [ ] preview resolution
* [ ] preview FPS
* [ ] preview bandwidth
* [ ] preview latency
* [ ] CPU conversion cost
* [ ] GPU conversion
* [ ] RAW preview possibility
* [ ] debayer performance
* [ ] histogram performance

---

# 102. Protocol research

* [ ] framing
* [ ] serialization
* [ ] message IDs
* [ ] request IDs
* [ ] event IDs
* [ ] checksums
* [ ] versioning
* [ ] compatibility
* [ ] reconnect
* [ ] resume
* [ ] authentication
* [ ] transport independence

---

# 103. Storage research

* [ ] average RAW size
* [ ] 300-frame session size
* [ ] sustained disk throughput
* [ ] filesystem behavior
* [ ] file naming
* [ ] checksum cost
* [ ] recovery after crash
* [ ] incomplete file detection

---

# 104. Example storage calculation

The actual RAW size must be measured.

Do not assume.

Once measured:

```text
RAW frame size × 300
```

gives minimum storage requirement.

Add:

```text
metadata
preview cache
logs
calibration frames
temporary files
```

and maintain a safety margin.

---

# 105. Data model

Core domain objects:

```text
Device
Camera
PhysicalCamera
CapabilitySet
CameraConfiguration
CaptureRequest
CaptureResult
Frame
FrameMetadata
Sequence
SequenceStep
Session
TransportStatus
ThermalStatus
StorageStatus
DiagnosticSnapshot
```

---

# 106. Requested vs actual

Every camera parameter must use this conceptual model:

```text
RequestedValue
AppliedValue
ReportedValue
```

This prevents false assumptions.

---

# 107. Example

```text
User:
Exposure = 15s

Command:
15s

Android:
accepted

CaptureResult:
15s

UI:
Exposure 15.000s ✓
```

If the device clamps:

```text
User:
Exposure = 30s

Android:
clamped to 15s

UI:
Requested: 30s
Applied:   15s
WARNING:   device limit
```

---

# 108. No silent fallback

Forbidden:

```text
RAW requested
JPEG silently returned
```

Forbidden:

```text
15s requested
10s captured
UI says 15s
```

Forbidden:

```text
manual focus requested
AF silently enabled
```

Every fallback must be explicit.

---

# 109. Astronomical session types

Initial:

```text
LIGHT
DARK
FLAT
BIAS
TEST
```

Future:

```text
FOCUS
PLATE_SOLVE
LIVE_STACK
```

---

# 110. Future plate solving

Potential architecture:

```text
Preview
 ↓
star detection
 ↓
plate solver
 ↓
RA / Dec
 ↓
mount integration
```

This is explicitly future scope.

---

# 111. Future mount integration

Potential protocols:

```text
INDI
ASCOM
Alpaca
```

Do not implement initially.

Design the architecture so it can be added later.

---

# 112. Future guiding

Potential:

```text
star centroid
 ↓
tracking error
 ↓
mount correction
```

Not part of MVP.

---

# 113. MVP

The first working milestone is deliberately narrow.

### MVP must do:

```text
Pixel connects
        ↓
discover cameras
        ↓
show capabilities
        ↓
select camera
        ↓
configure RAW
        ↓
configure exposure
        ↓
configure ISO
        ↓
configure focus
        ↓
capture one RAW
        ↓
transfer to PC
        ↓
save
        ↓
display preview
```

Only after this works:

```text
300 × 15s
```

---

# 114. MVP-2

Add:

```text
sequence engine
300-frame acquisition
metadata
session manifest
reconnect
crash recovery
thermal monitoring
```

---

# 115. MVP-3

Add:

```text
dark frames
flat frames
calibration workflow
histogram
statistics
focus assistance
```

---

# 116. Future

Potential:

```text
plate solving
mount control
autoguiding
live stacking
automatic focus
sky catalog integration
FITS output
Siril integration
PixInsight workflow
INDI
ASCOM Alpaca
```

---

# 117. Design rules

## Rule 1

Never hard-code camera capabilities.

## Rule 2

Never hide a clamped camera parameter.

## Rule 3

Never drop a RAW science frame silently.

## Rule 4

Preview may drop frames.

## Rule 5

Camera transport must be independent of GUI.

## Rule 6

GPUI must be isolated.

## Rule 7

Camera2 must be isolated behind an Android camera abstraction.

## Rule 8

Protocol must be versioned.

## Rule 9

Every acquisition must be reproducible from metadata.

## Rule 10

Every assumption about the Pixel must be experimentally validated.

---

# 118. Definition of success

DeepskyEyes is considered successful when:

```text
Pixel 8 Pro
     ↓
USB
     ↓
Rust
     ↓
GPUI
```

can reliably perform:

```text
RAW
+
manual exposure
+
manual sensitivity
+
manual focus
+
fixed WB
+
selected resolution
+
preview
+
300 exposures
+
15 seconds each
+
no silent parameter changes
+
no lost RAW frames
+
complete metadata
```

for a sustained astrophotography session.

---

# 119. First hardware experiment

Before writing the complete application:

1. Install minimal Android test application.
2. Enumerate camera IDs.
3. Dump all `CameraCharacteristics`.
4. Dump all stream configurations.
5. Dump RAW capabilities.
6. Dump manual sensor capabilities.
7. Dump exposure range.
8. Dump sensitivity range.
9. Dump focus range.
10. Dump AWB/color capabilities.
11. Dump processing modes.
12. Dump physical camera IDs.
13. Capture one RAW.
14. Capture one DNG.
15. Inspect RAW metadata.
16. Test 15-second exposure.
17. Capture 10 × 15 s.
18. Capture 100 × 15 s.
19. Capture 300 × 15 s.
20. Measure everything.

Only after this should the full camera-control protocol be frozen.

---

# 120. Golden principle

The project must always distinguish between:

```text
WHAT ANDROID API DEFINES
```

and:

```text
WHAT THIS PIXEL ACTUALLY SUPPORTS
```

and:

```text
WHAT THE SENSOR ACTUALLY DOES
```

These are three different things.

The first comes from documentation.

The second comes from capability discovery.

The third comes from measurement.

**DeepskyEyes must eventually trust the third.**

---

# 121. Current technical references

Primary Android references:

* `CameraCharacteristics`
* `StreamConfigurationMap`
* `CameraMetadata`
* `CaptureRequest`
* `CameraDevice`
* `CameraCaptureSession`
* `ImageReader`
* `DngCreator`
* USB Host/Accessory APIs

Android documents `StreamConfigurationMap` as the authoritative source for supported stream formats/sizes, and `CameraCharacteristics` as the camera-device property source.

Android's RAW capability explicitly defines `RAW_SENSOR` output and DNG-related metadata.

`DngCreator` exists specifically to create DNG files from `RAW_SENSOR` data and capture metadata.

GPUI documentation/source should be treated as a moving dependency because it remains pre-1.0.

---

# 122. Immediate TODO list

## Phase 0 — Research

* [ ] Obtain Pixel 8 Pro.
* [ ] Identify exact Android version.
* [ ] Dump all Camera2 characteristics.
* [ ] Dump all stream configurations.
* [ ] Identify all physical cameras.
* [ ] Verify RAW.
* [ ] Verify manual sensor.
* [ ] Verify maximum exposure.
* [ ] Verify ISO/sensitivity.
* [ ] Verify focus.
* [ ] Verify WB.
* [ ] Verify processing controls.
* [ ] Verify maximum RAW resolution.
* [ ] Verify long-exposure stability.
* [ ] Verify USB throughput.
* [ ] Verify thermal behavior.

## Phase 1 — Android probe

* [ ] Minimal Kotlin app.
* [ ] Capability dump.
* [ ] RAW capture.
* [ ] DNG capture.
* [ ] Metadata dump.
* [ ] 15-second capture.
* [ ] Long sequence test.

## Phase 2 — Protocol

* [ ] Wire format.
* [ ] Framing.
* [ ] Commands.
* [ ] Events.
* [ ] Errors.
* [ ] Versioning.
* [ ] Mock transport.

## Phase 3 — Rust CLI

* [ ] Connect.
* [ ] Discover.
* [ ] Capability dump.
* [ ] Set controls.
* [ ] Capture.
* [ ] Download.
* [ ] Sequence.

## Phase 4 — Rust acquisition engine

* [ ] queues.
* [ ] ring buffers.
* [ ] RAW writer.
* [ ] metadata.
* [ ] checksums.
* [ ] recovery.
* [ ] diagnostics.

## Phase 5 — GPUI

* [ ] window.
* [ ] device panel.
* [ ] camera controls.
* [ ] preview.
* [ ] histogram.
* [ ] sequence panel.
* [ ] diagnostics.

## Phase 6 — Astronomical workflow

* [ ] LIGHT.
* [ ] DARK.
* [ ] FLAT.
* [ ] calibration metadata.
* [ ] session management.
* [ ] stacking integration.

---

# 123. Final architectural decision

The initial technology stack is:

```text
ANDROID
──────────────
Kotlin
Camera2
ImageReader
DngCreator
Android Service
Transport adapter
Protocol implementation


DESKTOP
──────────────
Rust
GPUI
Tokio / async runtime as appropriate
Protocol implementation
Transport implementation
Acquisition engine
Sequencer
RAW/DNG handling
Session manager
Metadata
Diagnostics


PROTOCOL
──────────────
Transport-independent
Versioned
Framed
Request/response
Event capable
Capability driven


DATA
──────────────
RAW/DNG
Metadata
Session manifest
Checksums
Calibration data
```

---

# 124. The one thing we do NOT assume

We do **not** currently declare:

> "The Pixel 8 Pro definitely allows every manual control we want for every lens and every resolution."

That statement must be proven experimentally.

DeepskyEyes is therefore intentionally designed so that the **camera capability discovery layer is the authority**.

The software should adapt to:

```text
supported
unsupported
clamped
restricted
temporarily unavailable
```

instead of pretending every camera behaves like a DSLR.

That distinction is fundamental to the project's reliability.

---

# 125. Project motto

> **Discover. Validate. Capture. Record everything. Assume nothing.**

---
