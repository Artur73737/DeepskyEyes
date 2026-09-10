# 08 — Logical vs physical cameras (ricerca online)

Copre: README §50, §91.

## 1. Modello Android 9+

- Camera **logica** = 1 `CameraDevice`/`CaptureSession` che fonde 2+ **fisiche** stesso facing.
- Capability `LOGICAL_MULTI_CAMERA`; `getPhysicalCameraIds()` / `LOGICAL_MULTI_CAMERA_PHYSICAL_IDS`.
- Controllo fisico: `OutputConfiguration.setPhysicalCameraId()` + `CaptureRequest.setPhysicalCameraId()`.
  Solo request non-reprocessing, solo sensori mono/Bayer.
- Metadati di correlazione: `LENS_POSE_ROTATION/TRANSLATION`, `LENS_INTRINSIC_CALIBRATION`, `LENS_DISTORTION`, `LENS_POSE_REFERENCE`, `LOGICAL_MULTI_CAMERA_SENSOR_SYNC_TYPE`, `LOGICAL_MULTI_CAMERA_ACTIVE_PHYSICAL_ID` (HAL 3.5+).
- Best practice AOSP: OEM deve esporre una logica per ogni facing. Su Android 9 la logica deve supportare sostituzione 1×logico(YUV/RAW) → 2×fisici stessa size/formato (regole diverse su Android 10+).

## 2. Pixel

- Esempio documentato: Pixel 3 sceglie la fisica da focale/crop. Per Pixel 8 Pro (wide+ultrawide+tele 5x) va scoperto: quali ID logici/fisici, quale sensore è main/ultrawide/tele/front, se RAW/manuale sono indipendenti per fisica.
- Nota pratica: su alcuni device gli ID fisici non si aprono direttamente (`unknown camera id`); si apre la logica e si indirizzano le fisiche via session config. Segnalazioni Reddit su accesso tele Pixel 8 Pro confermano di testare esplicitamente.

## 3. Azioni DeepskyEyes

Dump `cameraIdList` + `physicalCameraIds` + `REQUEST_AVAILABLE_CAPABILITIES` per ID; tabella main/ultrawide/tele/front;
test RAW/manuale per fisica; decidere se la UI espone logiche, fisiche o entrambe.

## Fonti

- https://developer.android.com/media/camera/camera2/multi-camera
- https://source.android.com/docs/core/camera/multi-camera
- https://tech.gc.com/every-camera-every-angle-on-android
- https://stackoverflow.com/questions/55923506/camera2-replacing-one-logical-stream-with-two-physical-streams-in-android-api-29
- https://www.reddit.com/r/GooglePixel/comments/1f4a4sj/pixel_8_pro_help_request_can_i_access_the
