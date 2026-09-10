# 02 — Camera2: capability discovery (ricerca online)

Copre: README §2, §9, §10, §11, §91, §119, §121.

## 1. Gerarchia ufficiale

```
CameraManager.getCameraIdList()
  → CameraCharacteristics (immutabile, proprietà del CameraDevice)
    → StreamConfigurationMap (SCALER_STREAM_CONFIGURATION_MAP [+ MAXIMUM_RESOLUTION])
      → formati / size / minFrameDuration / stallDuration
```

- `CameraCharacteristics` = sorgente delle proprietà del dispositivo. Da API 32 alcune chiavi possono cambiare dinamicamente; senza permesso CAMERA alcune chiavi sono omesse (`getKeysNeedingPermission()`).
- `StreamConfigurationMap` = **sorgente autorevole** di formati/size di output per creare `Surface` e `CameraCaptureSession`.
- Regola README §2: mai assumere "15 s RAW", "4080×3072", "ISO 50..6400". Tutto va scoperto.

## 2. Chiavi da enumerare al primo avvio (§9)

Camera IDs, lens facing, physical IDs, hardware level, available capabilities, sensor info,
`SENSOR_INFO_EXPOSURE_TIME_RANGE`, `SENSOR_INFO_SENSITIVITY_RANGE`, `SENSOR_INFO_MAX_FRAME_DURATION`,
focus/AF/AE/AWB, color correction, OIS, flash, zoom/crop, RAW support + RAW/JPEG/YUV sizes,
maximum-resolution configs, stream/min-frame/stall durations, recommended configs.

Chiavi AOSP rilevanti:
`REQUEST_AVAILABLE_CAPABILITIES`, `INFO_SUPPORTED_HARDWARE_LEVEL`,
`SENSOR_INFO_*`, `CONTROL_AE/AF/AWB_AVAILABLE_MODES`, `LENS_INFO_*`,
`SCALER_STREAM_CONFIGURATION_MAP`, `SCALER_STREAM_CONFIGURATION_MAP_MAXIMUM_RESOLUTION`,
`REQUEST_MAX_NUM_OUTPUT_*`, `CONTROL_AVAILABLE_HIGH_SPEED_VIDEO_CONFIGURATIONS`.

## 3. Capability che contano per DeepskyEyes

| Capability | Significato |
|---|---|
| `MANUAL_SENSOR` | Controllo manuale sensore: exposure time, sensitivity, frame duration garantiti. FULL lo richiede; LIMITED solo se dichiarato; LEGACY mai (AE OFF non supportato) |
| `MANUAL_POST_PROCESSING` | Controllo tonemap, color correction, denoise, sharpening, shading, aberration |
| `RAW` | Output `RAW_SENSOR` + metadati DNG necessari |
| `READ_SENSOR_SETTINGS` | Sottoinsieme di MANUAL_SENSOR: lettura settaggi sensore durante la cattura |
| `LOGICAL_MULTI_CAMERA` | Camera logica multi-fisica (vedi §08) |
| `BACKWARD_COMPATIBLE` | Baseline garantita |
| `REMOSAIC_REPROCESSING`, `MAX_RESOLUTION` | Modalità alta risoluzione / remosaic dove presente |

FULL garantisce AE OFF → controllo app di exposure/sensitivity/frameDuration.
Con AE ON, i valori manuali di exposure/sensitivity/frameDuration sono **ignorati**.

## 4. Resolution discovery (§11)

Per ogni formato candidato (`RAW_SENSOR`, `RAW10`, `RAW_OPAQUE`, `YUV_420_888`, `JPEG`, `PRIVATE`/preview):
`getOutputSizes()`, `getOutputMinFrameDuration()`, `getOutputStallDuration()`,
`isOutputSupportedFor()`, più mappa maximum-resolution (API 31+, ultra-high-res/DNG test in CTS).
Per FULL, max frame duration ≥ 100 ms. La durata minima/stallo determina il vero frame period
(mai assumere 15 s esposizione = 15 s periodo).

## 5. Capability matrix JSON (§10)

Il dump deve produrre un documento versionato tipo `capabilities.json` con camera_id, lens_facing,
hardware_level, capabilities{}, sensor{}, exposure{min,max_ns}, sensitivity{min,max},
frameDuration, focus, stream map. Va salvato come snapshot a inizio sessione (§73).

## 6. Primo esperimento hardware (§119) — checklist API

1–5: `getCameraIdList`, dump `CameraCharacteristics`, dump stream maps, RAW caps, manual sensor caps.
6–12: exposure/sensitivity/focus/AWB/processing/physical IDs.
13–20: 1×RAW, 1×DNG, ispezione metadati, 15 s, 10×15 s, 100×15 s, 300×15 s, misura tutto.
Solo dopo si congela il protocollo.

## Fonti

- https://developer.android.com/reference/android/hardware/camera2/CameraCharacteristics
- https://developer.android.com/reference/android/hardware/camera2/params/StreamConfigurationMap
- https://developer.android.com/media/camera/camera2
- https://developer.android.com/media/camera/camera2/multi-camera
- https://source.android.com/docs/core/camera/multi-camera
- https://source.android.com/docs/compatibility/cts/camera-its-tests
- https://android.googlesource.com/platform/frameworks/base/+/master/core/java/android/hardware/camera2/CameraCharacteristics.java
- https://android.googlesource.com/platform/frameworks/base/+/master/core/java/android/hardware/camera2/CameraMetadata.java
- https://android.googlesource.com/platform/frameworks/base/+/master/core/java/android/hardware/camera2/params/StreamConfigurationMap.java
